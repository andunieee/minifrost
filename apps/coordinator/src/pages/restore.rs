//! Rebuild a wallet from at least a threshold of its shares. Restoring starts on the signers: each
//! shows a restore code, for a backup typed into it or a key it holds, and this page reads them
//! one after another. The coordinator sorts them out and saves the wallet once it has enough.

use std::collections::BTreeMap;

use minifrost_coordinator::api::{
    PendingConsolidationView,
    Update,
};
use minifrost_ui::prelude::*;

use crate::nav::{
    App,
    Nav,
    Naming,
    Page,
};

/// What a restore code or step led to: naming the wallet it rebuilt, then the code for the
/// signers. `replace`: in place of the page on top (the naming form).
fn follow(nav: Nav, u: Update, replace: bool) -> bool {
    let page = if let Some(w) = u.unnamed_wallet {
        Page::NameWallet(Naming::Unnamed(w))
    } else if u.broadcast.is_some() {
        Page::Ceremony {
            title: "Finish restoring".into(),
            update: u,
            key_id: None,
        }
    } else {
        if replace {
            nav.pop();
        }
        return false;
    };
    if replace { nav.replace(page) } else { nav.push(page) }
    true
}

#[derive(PartialEq)]
pub struct RestorePage;

impl Component for RestorePage {
    fn render(&self) -> impl IntoElement {
        let p = use_palette();
        let app = use_consume::<App>();
        let nav = use_consume::<Nav>();
        nav.watch();
        let mut scanning = use_state(|| false);
        // the signer the toast on screen is about, so rereading its code doesn't toast again
        let mut toast_from = use_state(|| None::<String>);
        let shares = app.restoring_shares();
        let unnamed = app.unnamed_wallets();
        // the same wallet can have several signers waiting; one code serves them all
        let mut waiting: BTreeMap<String, Vec<PendingConsolidationView>> = BTreeMap::new();
        for pc in app.pending_consolidations() {
            waiting.entry(pc.key_id.clone()).or_default().push(pc);
        }

        let on_payload = {
            let app = app.clone();
            move |payload: Vec<u8>| {
                match app.read_restore_code(payload) {
                    Ok(u) => {
                        let from = u.from.as_ref().map(|s| s.device_id.clone());
                        let color = from.as_deref().map(|id| signer_color(id, p.dark));
                        if !u.repeat {
                            nav.overlay.success(u.message.clone(), color);
                        } else if *toast_from.peek() != from {
                            nav.overlay.info(u.message.clone(), color);
                        }
                        toast_from.set(from);
                        let finished = u.finished.is_some();
                        // the page behind doesn't need the camera, and the wallet this was for is done
                        if follow(nav, u, false) || finished {
                            scanning.set(false);
                        }
                    }
                    Err(e) => nav.overlay.toast(e),
                }
                nav.changed();
            }
        };

        rect().expanded().child(app_bar(&p, "Restore", None)).child(
            page(720.)
                .child(text(
                    "Rebuild a wallet from at least a threshold of its shares. On each signer, type in a backup (25 \
                     words) or tap a key and choose “Restore wallet on coordinator”, then read the code it shows here. \
                     The wallet is saved as soon as there are enough.",
                ))
                .child(
                    rect().child(
                        Btn::new(if scanning() { "Stop reading" } else { "Read a signer’s code" })
                            .icon(if scanning() { lucide::x() } else { lucide::scan_qr_code() })
                            .on_press(move |_| scanning.toggle()),
                    ),
                )
                .maybe(scanning(), |el| {
                    el.child(
                        rect().width(Size::fill()).max_width(Size::px(480.)).child(
                            ScanPanel::new(on_payload)
                                .hint("Show the signer’s restore code to the camera")
                                .expecting("signer restore code"),
                        ),
                    )
                })
                .children(unnamed.into_iter().map(|w| {
                    let subtitle = format!("{} signatures needed, read from {}", w.threshold, w.signers.join(", "));
                    let key = w.key_id.clone();
                    ListRow::new(label().text("Enough codes to rebuild a wallet"))
                        .icon(lucide::circle_check(), p.success)
                        .subtitle(subtitle)
                        .trailing(Btn::new("Name it").on_press(move |_| nav.push(Page::NameWallet(Naming::Unnamed(w.clone())))))
                        .key(key)
                        .into_element()
                }))
                .maybe(!shares.is_empty(), |el| {
                    el.child(gap(8.)).child(title("Read so far")).children(shares.into_iter().map(|s| {
                        let subtitle = match &s.key_name {
                            None => "From a backup: its wallet shows once more codes are read.".to_string(),
                            Some(name) => {
                                let mut parts = vec![format!("“{name}”")];
                                if let Some(n) = &s.network {
                                    parts.push(network_label(n).to_string());
                                }
                                if let Some(t) = s.threshold {
                                    parts.push(format!("{t} signatures needed"));
                                }
                                parts.join(" · ")
                            }
                        };
                        let remove = {
                            let (app, id) = (app.clone(), s.id.clone());
                            move |_| {
                                if let Err(e) = app.remove_restoring_share(id.clone()) {
                                    nav.overlay.toast(e);
                                }
                                nav.changed();
                            }
                        };
                        let one = s.clone();
                        ListRow::new(label().text(format!("Key #{} on {}", s.share_index, s.device_name)))
                            .icon(lucide::key_round(), signer_color(&s.device_id, p.dark))
                            .subtitle(subtitle)
                            .maybe(s.threshold.is_none(), |row| {
                                row.trailing(Btn::new("Only one signature?").text().on_press(move |_| {
                                    nav.push(Page::NameWallet(Naming::OneSignature(one.clone())))
                                }))
                            })
                            .trailing(IconBtn::new(lucide::x(), remove))
                            .key(&s.id)
                            .into_element()
                    }))
                })
                .maybe(!waiting.is_empty(), |el| {
                    el.child(gap(8.))
                        .child(title("Signers waiting for their key"))
                        .child(text("These signers took a backup. Until they read the wallet’s code they cannot sign."))
                        .children(waiting.into_iter().map(|(key_id, ps)| {
                            let lines: Vec<String> =
                                ps.iter().map(|pc| format!("{} · key #{}", pc.device_name, pc.share_index)).collect();
                            let show = {
                                let (app, key_id) = (app.clone(), key_id.clone());
                                move |_| match app.start_finish_restoring(key_id.clone()) {
                                    Ok(u) => nav.push(Page::Ceremony {
                                        title: "Finish restoring".into(),
                                        update: u,
                                        key_id: None,
                                    }),
                                    Err(e) => nav.overlay.toast(e),
                                }
                            };
                            ListRow::new(label().text(format!("Wallet “{}”", ps[0].key_name)))
                                .icon(lucide::hourglass(), p.text_muted)
                                .subtitle(lines.join("\n"))
                                .trailing(Btn::new("Show code").on_press(show))
                                .key(key_id)
                                .into_element()
                        }))
                }),
        )
    }
}

/// A backup doesn't say what its wallet was called or which network it is on: the user does.
#[derive(PartialEq)]
pub struct NameWalletPage {
    pub naming: Naming,
}

impl Component for NameWalletPage {
    fn render(&self) -> impl IntoElement {
        let p = use_palette();
        let app = use_consume::<App>();
        let nav = use_consume::<Nav>();
        let name = use_state(String::new);
        let mut network = use_state(|| "signet".to_string());

        let (screen_title, intro) = match &self.naming {
            Naming::Unnamed(w) => (
                "Name the rebuilt wallet",
                format!(
                    "{} signatures needed, read from {}. A backup doesn’t say what the wallet was called or which \
                     network it is on: use your backup sheet.",
                    w.threshold,
                    w.signers.join(", ")
                ),
            ),
            Naming::OneSignature(_) => (
                "Restore a one-signature wallet",
                "Do this only if your backup sheet says one signature is enough. Nothing can check that from a single \
                 backup, and a wallet of a higher threshold rebuilds by itself once you read enough codes."
                    .to_string(),
            ),
        };
        let valid = !name.read().trim().is_empty();
        let restore = {
            let naming = self.naming.clone();
            move |_| {
                let (n, net) = (name.peek().trim().to_string(), network.peek().clone());
                let result = match &naming {
                    Naming::Unnamed(w) => app.name_restored_wallet(w.key_id.clone(), n, net),
                    Naming::OneSignature(s) => app.restore_one_signature_wallet(s.id.clone(), n, net),
                };
                match result {
                    Ok(u) => {
                        nav.overlay.success(u.message.clone(), None);
                        follow(nav, u, true);
                    }
                    Err(e) => nav.overlay.toast(e),
                }
                nav.changed();
            }
        };

        rect().expanded().child(app_bar(&p, screen_title, back(move |_| nav.pop()))).child(
            page(560.)
                .child(text(intro))
                .child(field(&p, "Wallet name", Input::new(name).width(Size::fill()).auto_focus(true)))
                .child(small("Network", p.text_muted))
                .children(NETWORKS.iter().map(|(id, label)| {
                    let id = id.to_string();
                    radio_row(&p, *network.read() == id, *label, {
                        let id = id.clone();
                        move |_| network.set(id.clone())
                    })
                    .key(id)
                    .into_element()
                }))
                .child(
                    button_row()
                        .child(Btn::new("Not now").text().on_press(move |_| nav.pop()))
                        .child(Btn::new("Restore wallet").enabled(valid).on_press(restore)),
                ),
        )
    }
}
