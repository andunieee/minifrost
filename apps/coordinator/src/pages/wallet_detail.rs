use std::collections::{
    BTreeMap,
    BTreeSet,
};

use minifrost_coordinator::api::{
    BackupShare,
    KeySigner,
    KeyView,
    SendPreview,
    Update,
    WalletSummary,
};
use minifrost_ui::prelude::*;

use crate::nav::{
    App,
    Nav,
    Page,
    off_thread,
};

fn open_session(nav: Nav, title: String, started: Result<Update, String>) {
    match started {
        Ok(update) => nav.push(Page::Ceremony {
            title,
            update,
            key_id: None,
        }),
        Err(e) => nav.overlay.toast(e),
    }
}

/// "alice", "alice and bob", "alice, bob and carol".
fn names(signers: &[KeySigner]) -> String {
    match signers {
        [] => String::new(),
        [one] => one.name.clone(),
        [rest @ .., last] => format!(
            "{} and {}",
            rest.iter().map(|s| s.name.as_str()).collect::<Vec<_>>().join(", "),
            last.name
        ),
    }
}

/// A titled card of the wallet page.
fn section(p: &Palette, title_text: &str, icon_bytes: Bytes, icon_color: Color) -> Rect {
    card(p).padding(20.).spacing(10.).child(
        rect()
            .horizontal()
            .width(Size::fill())
            .content(Content::flex())
            .spacing(10.)
            .cross_align(Alignment::center())
            .child(icon(icon_bytes, 22., icon_color))
            .child(title(title_text).width(Size::flex(1.))),
    )
}

#[derive(PartialEq)]
pub struct WalletDetailPage {
    pub key_id: String,
}

impl Component for WalletDetailPage {
    fn render(&self) -> impl IntoElement {
        let p = use_palette();
        let app = use_consume::<App>();
        let nav = use_consume::<Nav>();
        nav.watch();
        let mut summary = use_state(|| None::<WalletSummary>);
        let mut error = use_state(|| None::<String>);
        let mut loading = use_state(|| false);
        let to = use_state(String::new);
        let amount = use_state(String::new);
        let mut fee_rate = use_state(|| "2".to_string());
        let mut send_max = use_state(|| false);

        let refresh = {
            let (app, key_id) = (app.clone(), self.key_id.clone());
            move || {
                let (app, key_id) = (app.clone(), key_id.clone());
                loading.set(true);
                error.set(None);
                spawn(async move {
                    match off_thread(&app, move |a| a.refresh_wallet(key_id)).await {
                        Ok(s) => summary.set(Some(s)),
                        Err(e) => error.set(Some(e)),
                    }
                    loading.set(false);
                });
            }
        };
        use_hook({
            let mut refresh = refresh.clone();
            move || refresh()
        });

        let Some(w) = app.keys().into_iter().find(|k| k.key_id == self.key_id) else {
            return rect()
                .expanded()
                .child(app_bar(&p, "Wallet", back(move |_| nav.pop())))
                .child(page(640.).child(text("This wallet is not on this coordinator any more.")))
                .into_element();
        };
        let backups = app.backup_shares(w.key_id.clone()).unwrap_or_default();
        let pending: Vec<_> = app.pending_consolidations().into_iter().filter(|pc| pc.key_id == w.key_id).collect();
        let missing = backups.iter().filter(|b| !b.complete).count();
        let s = summary.read().clone();
        let wide = Platform::get().current_window().root_size.read().width >= 560.;

        let menu = {
            let (app, w) = (app.clone(), w.clone());
            move |_| {
                let (app, w) = (app.clone(), w.clone());
                let balance = summary.peek().as_ref().map(|s| s.balance_sats).unwrap_or(0);
                run(async move {
                    let choice = Choice::new(lucide::trash_2(), "Delete wallet").danger();
                    if nav.overlay.choose(w.name.clone(), "", vec![choice]).await == Some(0) {
                        delete_wallet(app, nav, w, balance).await;
                    }
                });
            }
        };

        // ---- backups
        let mut backup_card = section(
            &p,
            "Backups",
            if missing > 0 { lucide::triangle_alert() } else { lucide::shield_check() },
            if missing > 0 { p.error } else { p.success },
        )
        .child(
            text(if missing > 0 {
                format!(
                    "Write down every signer’s 25 words. Without backups, losing {} signer(s) loses the wallet.",
                    (w.n_keys + 1).saturating_sub(w.threshold)
                )
            } else {
                "Every key has a recorded backup. Check them from time to time.".to_string()
            })
            .color(p.text_muted),
        );
        for b in &backups {
            backup_card = backup_card.child(divider(&p)).child(backup_row(&p, &app, nav, &w, b));
        }
        for pc in &pending {
            let (app, key_id, name) = (app.clone(), pc.key_id.clone(), w.name.clone());
            backup_card = backup_card.child(divider(&p)).child(
                rect()
                    .width(Size::fill())
                    .spacing(8.)
                    .child(note(
                        lucide::hourglass(),
                        p.text,
                        format!("{} · key #{} is waiting to be finished", pc.device_name, pc.share_index),
                    ))
                    .child(small("Its typed-in backup becomes a normal share once it is finished.", p.text_muted))
                    .child(rect().child(Btn::new("Finish").on_press(move |_| {
                        open_session(nav, format!("Finish restoring “{name}”"), app.start_finish_restoring(key_id.clone()))
                    }))),
            );
        }
        let backup_card = backup_card.child(divider(&p)).child(small(
            "To add a replaced signer, type its backup into it and read its code on the Restore page.",
            p.text_muted,
        ));

        // ---- signers, by key number: two of them can hold the same key, when its backup went into both
        let mut holders: BTreeMap<u32, Vec<KeySigner>> = BTreeMap::new();
        for s in &w.signers {
            holders.entry(s.share_index).or_default().push(s.clone());
        }
        let mut signers_card = section(&p, "Signers", lucide::users(), p.text)
            .child(text(format!("{} of these {} keys sign a payment.", w.threshold, w.n_keys)).color(p.text_muted));
        for (index, signers) in holders {
            signers_card = signers_card.child(divider(&p)).child(label().font_weight(FontWeight::SEMI_BOLD).text(format!("Key #{index}")));
            if signers.len() > 1 {
                signers_card = signers_card.child(small(
                    format!(
                        "{} hold the same key. Together they give one signature, not {}.",
                        names(&signers),
                        signers.len()
                    ),
                    p.warn,
                ));
            }
            for s in &signers {
                let others = signers.len() - 1;
                let forget = {
                    let (app, w, s) = (app.clone(), w.clone(), s.clone());
                    move |_| run(forget_share(app.clone(), nav, w.clone(), s.clone(), others))
                };
                signers_card = signers_card.child(
                    rect()
                        .horizontal()
                        .width(Size::fill())
                        .content(Content::flex())
                        .spacing(12.)
                        .cross_align(Alignment::center())
                        .child(icon(lucide::smartphone(), 20., signer_color(&s.device_id, p.dark)))
                        .child(rect().width(Size::flex(1.)).child(signer_name(&p, &s.device_id, s.name.clone())))
                        .child(IconBtn::new(lucide::unlink(), forget)),
                );
            }
        }

        // ---- send
        let send = {
            let (app, key_id) = (app.clone(), w.key_id.clone());
            move |_| {
                let Ok(rate) = fee_rate.peek().trim().parse::<f64>() else {
                    nav.overlay.toast("Enter a fee rate.");
                    return;
                };
                let max = *send_max.peek();
                let sats = amount.peek().trim().parse::<i64>().ok();
                if !max && sats.is_none() {
                    nav.overlay.toast("Enter the amount in sats.");
                    return;
                }
                match app.prepare_send(key_id.clone(), to.peek().trim().to_string(), if max { None } else { sats }, rate) {
                    Ok(preview) => nav.push(Page::ConfirmSend {
                        key_id: key_id.clone(),
                        preview,
                    }),
                    Err(e) => nav.overlay.toast(e),
                }
            }
        };
        let suggest = {
            let (app, key_id) = (app.clone(), w.key_id.clone());
            move |_| {
                let (app, key_id) = (app.clone(), key_id.clone());
                spawn(async move {
                    match off_thread(&app, move |a| a.suggested_fee_rate(key_id, 6)).await {
                        Ok(r) => fee_rate.set(format!("{r:.1}")),
                        Err(e) => nav.overlay.toast(e),
                    }
                });
            }
        };
        let send_card = section(&p, "Send", lucide::arrow_up_right(), p.text)
            .child(field(&p, "Recipient address", Input::new(to).width(Size::fill())))
            .child(field(
                &p,
                "Amount (sats)",
                Input::new(amount).width(Size::fill()).enabled(!send_max()),
            ))
            .child(check_row(&p, send_max(), label().text("Send everything"), move |_| send_max.toggle()))
            .child(
                rect()
                    .horizontal()
                    .width(Size::fill())
                    .content(Content::flex())
                    .spacing(12.)
                    .cross_align(Alignment::end())
                    .child(rect().width(Size::flex(1.)).child(field(&p, "Fee rate (sat/vB)", Input::new(fee_rate).width(Size::fill()))))
                    .child(Btn::new("Suggest").text().icon(lucide::wand_sparkles()).on_press(suggest)),
            )
            .child(gap(4.))
            .child(Btn::new("Review & sign").icon(lucide::send()).fill().enabled(s.is_some()).on_press(send));

        let mut refresh = refresh;
        let refresh_press = move |_| {
            if !*loading.peek() {
                refresh()
            }
        };

        rect()
            .expanded()
            .child(
                app_bar(&p, w.name.clone(), back(move |_| nav.pop()))
                    .child(IconBtn::new(lucide::refresh_cw(), refresh_press))
                    .child(IconBtn::new(lucide::ellipsis_vertical(), menu)),
            )
            .child(
                page(640.)
                    .child(
                        rect()
                            .horizontal()
                            .width(Size::fill())
                            .content(Content::wrap_spacing(6.))
                            .spacing(8.)
                            .cross_align(Alignment::center())
                            .child(chip(&p, format!("{}-of-{}", w.threshold, w.n_keys)))
                            .child(chip(&p, network_label(&w.network)))
                            .children(w.signers.iter().map(|s| signer_name(&p, &s.device_id, s.name.clone()).font_size(13.).into_element())),
                    )
                    .child(if loading() { progress_line(&p, 1.) } else { gap(4.) })
                    .map(error.read().clone(), |el, e| el.child(text(e).color(p.error)))
                    .map(s.clone(), |el, s| el.child(balance_card(&p, &s)))
                    .maybe(missing > 0, |el| el.child(backup_card.clone()))
                    .map(s.clone(), |el, s| el.child(receive_card(&p, nav, &s, wide)))
                    .child(send_card)
                    .maybe(missing == 0, |el| el.child(backup_card))
                    .child(signers_card)
                    .child(gap(16.)),
            )
            .into_element()
    }
}

fn balance_card(p: &Palette, s: &WalletSummary) -> Rect {
    let pending = s.balance_sats - s.confirmed_sats;
    tinted_card(p.accent_soft, p.on_accent_soft)
        .padding((28., 24.))
        .child(label().font_size(13.).font_weight(FontWeight::SEMI_BOLD).text("Balance"))
        .child(label().font_size(32.).font_weight(FontWeight::SEMI_BOLD).text(format_btc(s.balance_sats)))
        .child(label().font_size(16.).text(format_sats(s.balance_sats)))
        .child(gap(8.))
        .child(
            rect()
                .horizontal()
                .content(Content::wrap_spacing(8.))
                .spacing(8.)
                .child(chip(p, format!("{} coin(s)", s.n_coins)))
                .child(chip(p, format!("{} confirmed", format_sats(s.confirmed_sats))))
                .maybe(pending > 0, |el| el.child(chip(p, format!("{} pending", format_sats(pending))))),
        )
}

fn receive_card(p: &Palette, nav: Nav, s: &WalletSummary, wide: bool) -> Rect {
    let address = s.receive_address.clone();
    let details = rect()
        .width(if wide { Size::flex(1.) } else { Size::fill() })
        .spacing(12.)
        .child(
            SelectableText::new()
                .width(Size::fill())
                .font_family(MONO)
                .font_size(16.)
                .span(group_address(&s.receive_address)),
        )
        .child(rect().child(Btn::new("Copy address").outline().icon(lucide::copy()).on_press(move |_| {
            match Clipboard::set(address.clone()) {
                Ok(()) => nav.overlay.toast("Address copied"),
                Err(e) => nav.overlay.toast(format!("Could not copy: {e:?}")),
            }
        })));
    let code = qr_view(&s.receive_address.to_uppercase(), 200., Color::BLACK);
    section(p, "Receive", lucide::arrow_down_left(), p.text).child(if wide {
        rect()
            .horizontal()
            .width(Size::fill())
            .content(Content::flex())
            .spacing(24.)
            .child(code)
            .child(details)
    } else {
        rect()
            .width(Size::fill())
            .spacing(16.)
            .child(rect().width(Size::fill()).cross_align(Alignment::center()).child(code))
            .child(details)
    })
}

fn backup_row(p: &Palette, app: &App, nav: Nav, w: &KeyView, b: &BackupShare) -> Rect {
    let color = if b.complete { p.success } else { p.error };
    let show = {
        let (app, key_id, device_id, name) = (app.clone(), w.key_id.clone(), b.device_id.clone(), b.name.clone());
        move |_| open_session(nav, format!("Back up {name}"), app.start_display_backup(key_id.clone(), device_id.clone()))
    };
    let check = {
        let (app, key_id, device_id, name) = (app.clone(), w.key_id.clone(), b.device_id.clone(), b.name.clone());
        move |_| open_session(nav, format!("Check {name}’s backup"), app.start_check_backup(key_id.clone(), device_id.clone()))
    };
    let mark = {
        let (app, key_id, index) = (app.clone(), w.key_id.clone(), b.share_index);
        move |_| {
            let (app, key_id) = (app.clone(), key_id.clone());
            run(async move {
                let ok = nav
                    .overlay
                    .confirm(
                        "Mark as backed up?",
                        format!(
                            "Only do this if you have already recorded the backup for key #{index} somewhere safe.\n\n\
                             If you mark a key as backed up without actually having the backup, you risk permanently \
                             losing access to this wallet."
                        ),
                        "I have the backup",
                        false,
                    )
                    .await;
                if ok {
                    if let Err(e) = app.mark_backup_complete(key_id, index) {
                        nav.overlay.toast(e);
                    }
                    nav.changed();
                }
            });
        }
    };
    rect()
        .width(Size::fill())
        .spacing(8.)
        .child(
            rect()
                .horizontal()
                .width(Size::fill())
                .content(Content::flex())
                .spacing(12.)
                .cross_align(Alignment::center())
                .child(icon(if b.complete { lucide::circle_check() } else { lucide::circle() }, 22., color))
                .child(
                    rect()
                        .width(Size::flex(1.))
                        .child(label().text(format!("{} · key #{}", b.name, b.share_index)))
                        .child(small(if b.complete { "Backed up" } else { "Not backed up" }, color)),
                ),
        )
        .child(
            button_row()
                .margin((0., 0., 0., 34.))
                .child(Btn::new(if b.complete { "Show again" } else { "Show backup" }).tonal().on_press(show))
                .child(if b.complete {
                    Btn::new("Check").outline().on_press(check)
                } else {
                    Btn::new("Mark as backed up").text().on_press(mark)
                }),
        )
}

async fn forget_share(app: App, nav: Nav, w: KeyView, s: KeySigner, others_with_key: usize) {
    let left = w.n_keys - if others_with_key > 0 { 0 } else { 1 };
    let index = s.share_index;
    let effect = if others_with_key > 0 {
        format!("Another signer still holds key #{index}, so the wallet still has {} keys.", w.n_keys)
    } else if left < w.threshold {
        format!(
            "No other signer holds key #{index}. The wallet is left with {left} key(s) and needs {} signatures: it \
             can’t sign until a signer with that key joins again.",
            w.threshold
        )
    } else {
        format!(
            "No other signer holds key #{index}. The wallet is left with {left} keys, {} of which still sign.",
            w.threshold
        )
    };
    let ok = nav
        .overlay
        .confirm(
            format!("Forget that {} holds key #{index}?", s.name),
            format!(
                "{effect}\n\nThis only changes this coordinator: {} keeps the key. To add it back, show its restore \
                 code (tap the key on the signer) on the Restore page.",
                s.name
            ),
            "Forget",
            others_with_key == 0,
        )
        .await;
    if !ok {
        return;
    }
    if let Err(e) = app.forget_share(w.key_id.clone(), s.device_id.clone()) {
        nav.overlay.toast(e);
    }
    nav.changed();
}

async fn delete_wallet(app: App, nav: Nav, w: KeyView, balance: i64) {
    let holds = if balance > 0 { format!("This wallet holds {}. ", format_btc(balance)) } else { String::new() };
    let t = w.threshold;
    let ok = nav
        .overlay
        .confirm(
            format!("Delete “{}”?", w.name),
            format!(
                "{holds}This coordinator forgets the wallet. The money stays on the chain and the signers keep their \
                 keys: to use the wallet again, read the restore codes of {t} signers (or of their typed-in backups) \
                 on the Restore page.\n\nWithout {t} of its signers or backups, deleting it here loses the money for good."
            ),
            "Continue",
            true,
        )
        .await;
    if !ok {
        return;
    }
    let sure = nav
        .overlay
        .confirm_typing(
            format!("Delete “{}”", w.name),
            format!("Make sure you have {t} of its signers or backups at hand."),
            w.name.clone(),
            "Delete wallet",
        )
        .await;
    if !sure {
        return;
    }
    match app.delete_wallet(w.key_id.clone()) {
        Ok(()) => {
            nav.overlay.success(format!("Deleted “{}” from this coordinator.", w.name), None);
            nav.pop();
        }
        Err(e) => nav.overlay.toast(e),
    }
}

/// The payment as it will be signed, and which signers sign it.
#[derive(PartialEq)]
pub struct ConfirmSendPage {
    pub key_id: String,
    pub preview: SendPreview,
}

impl Component for ConfirmSendPage {
    fn render(&self) -> impl IntoElement {
        let p = use_palette();
        let app = use_consume::<App>();
        let nav = use_consume::<Nav>();
        let wallet = use_hook({
            let (app, key_id) = (app.clone(), self.key_id.clone());
            move || app.keys().into_iter().find(|k| k.key_id == key_id)
        });
        // one signer per key until there are enough: two holding the same key give one signature
        let mut chosen = use_state({
            let wallet = wallet.clone();
            move || {
                let mut keys = BTreeSet::new();
                let mut out = BTreeSet::new();
                if let Some(w) = &wallet {
                    for s in &w.signers {
                        if keys.len() < w.threshold as usize && keys.insert(s.share_index) {
                            out.insert(s.device_id.clone());
                        }
                    }
                }
                out
            }
        });
        let Some(w) = wallet else {
            return rect().into_element();
        };
        let keys: BTreeSet<u32> = w.signers.iter().filter(|s| chosen.read().contains(&s.device_id)).map(|s| s.share_index).collect();
        let same_key = keys.len() < chosen.read().len();
        let ok = !same_key && keys.len() >= w.threshold as usize;
        let pv = &self.preview;
        let mut outputs = vec![TxOutput {
            address: pv.recipient.clone(),
            sats: pv.amount_sats,
            own_label: None,
        }];
        if let Some(change) = pv.change_sats {
            outputs.push(TxOutput {
                address: String::new(),
                sats: change,
                own_label: Some("Change".into()),
            });
        }
        let start = {
            let key_id = w.key_id.clone();
            move |_| {
                let ids: Vec<String> = chosen.peek().iter().cloned().collect();
                match app.begin_signing(key_id.clone(), ids) {
                    Ok(update) => nav.replace(Page::Ceremony {
                        title: "Sign payment".into(),
                        update,
                        key_id: Some(key_id.clone()),
                    }),
                    Err(e) => nav.overlay.toast(e),
                }
            }
        };

        rect()
            .expanded()
            .child(app_bar(&p, "Confirm payment", back(move |_| nav.pop())))
            .child(
                page(560.)
                    .child(tx_card(
                        &p,
                        Tx {
                            outputs,
                            fee_sats: pv.fee_sats,
                            fee_rate: Some(pv.fee_rate),
                            n_inputs: Some(pv.n_inputs),
                            txid: None,
                            warnings: vec![],
                            verified: true,
                        },
                    ))
                    .child(title(format!("Which signers will sign? (need {})", w.threshold)))
                    .children(w.signers.iter().map(|s| {
                        let id = s.device_id.clone();
                        check_row(
                            &p,
                            chosen.read().contains(&id),
                            rect()
                                .child(signer_name(&p, &id, s.name.clone()))
                                .child(small(format!("key #{}", s.share_index), p.text_muted)),
                            {
                                let id = id.clone();
                                move |_| {
                                    let mut set = chosen.write();
                                    if !set.remove(&id) {
                                        set.insert(id.clone());
                                    }
                                }
                            },
                        )
                        .key(id)
                        .into_element()
                    }))
                    .maybe(same_key, |el| {
                        el.child(
                            text(
                                "Two of the chosen signers hold the same key, which gives one signature. Choose only \
                                 one of them.",
                            )
                            .color(p.error),
                        )
                    })
                    .child(
                        button_row()
                            .child(Btn::new("Cancel").text().on_press(move |_| nav.pop()))
                            .child(Btn::new("Start signing").enabled(ok).on_press(start)),
                    ),
            )
            .into_element()
    }
}
