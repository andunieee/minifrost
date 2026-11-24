//! One keygen, signing, backup or restoring session: show the current broadcast QR to the
//! signers, scan their replies, and move on when the coordinator has everything it needs for the
//! next round.

use minifrost_coordinator::api::{
    Broadcast,
    Finished,
    Update,
};
use minifrost_ui::prelude::*;

use crate::nav::{
    App,
    Nav,
    off_thread,
};

/// Leaves the session; one that isn't finished is cancelled, once the user agrees.
pub async fn close(app: App, nav: Nav, finished: bool) {
    if !finished {
        let ok = nav
            .overlay
            .ask(
                "Cancel this session?",
                "Progress with the signers will be discarded.",
                "Cancel session",
                "Keep going",
                false,
            )
            .await;
        if !ok {
            return;
        }
        if let Err(e) = app.cancel_session() {
            nav.overlay.toast(e);
        }
    }
    nav.pop();
}

#[derive(PartialEq)]
pub struct CeremonyPage {
    pub title: String,
    pub update: Update,
    pub key_id: Option<String>,
}

impl Component for CeremonyPage {
    fn render(&self) -> impl IntoElement {
        let p = use_palette();
        let app = use_consume::<App>();
        let nav = use_consume::<Nav>();
        let mut update = use_state(|| self.update.clone());
        let mut broadcast = use_state(|| self.update.broadcast.clone());
        let mut message = use_state(|| self.update.message.clone());
        let mut confirming = use_state(|| false);
        // the signer the toast on screen is about, so rescanning its reply doesn't toast again
        let mut toast_from = use_state(|| None::<String>);

        let u = update.read().clone();
        let finished = u.finished.clone();
        let width = Platform::get().current_window().root_size.read().width;
        let narrow = width < 900.;
        let qr_size = if narrow { (width - 80.).clamp(200., 380.) } else { 380. };

        let mut apply = move |next: Update| {
            // a wallet that was just created or rebuilt: go to it
            if let Some(Finished::KeyCreated { key_id, .. } | Finished::WalletRestored { key_id, .. }) = &next.finished {
                nav.open_wallet(key_id.clone());
                return;
            }
            message.set(next.message.clone());
            if next.broadcast.is_some() {
                broadcast.set(next.broadcast.clone());
            }
            update.set(next);
        };

        let on_payload = {
            let app = app.clone();
            move |payload: Vec<u8>| match app.receive(payload) {
                Ok(next) => {
                    let step_done = next.finished.is_some() || next.broadcast.is_some() || next.awaiting.is_empty();
                    let from = next.from.as_ref().map(|s| s.device_id.clone());
                    let color = from.as_deref().map(|id| signer_color(id, p.dark));
                    if next.repeat {
                        // the camera keeps reading the same reply while it's in view: say so once
                        if *toast_from.peek() != from {
                            nav.overlay.info(next.message.clone(), color);
                        }
                    } else if step_done {
                        nav.overlay
                            .success(format!("{} All replies received: step complete.", next.message), color);
                    } else {
                        nav.overlay.success(next.message.clone(), color);
                    }
                    toast_from.set(from);
                    // a repeat carries nothing new (no check code, no broadcast): keep what is on screen
                    if !next.repeat {
                        apply(next);
                    }
                }
                Err(e) => {
                    toast_from.set(None);
                    nav.overlay.toast(e);
                }
            }
        };

        // a step with no signer reply (a signer that approved the check code, a displayed,
        // checked or saved backup): the user confirms it here
        let mut confirm = move |result: Result<Update, String>| {
            confirming.set(false);
            match result {
                Ok(next) => {
                    if next.finished.is_some() {
                        nav.overlay.success(next.message.clone(), None);
                    }
                    apply(next);
                }
                Err(e) => nav.overlay.toast(e),
            }
        };

        let mut shown = rect().width(Size::fill()).spacing(12.);
        if let (Some(b), None) = (broadcast.read().clone(), &finished) {
            shown = shown.child(shown_code(&b, &message.read(), qr_size));
        }
        if let Some(code) = u.check_code.clone() {
            shown = shown.child(
                tinted_card(p.accent_soft, p.on_accent_soft)
                    .cross_align(Alignment::center())
                    .child(text("Check code: must be identical on every signer").text_align(TextAlign::Center))
                    .child(
                        SelectableText::new()
                            .font_family(MONO)
                            .font_size(36.)
                            .font_weight(FontWeight::BOLD)
                            .span(code),
                    ),
            );
        }
        if let Some(f) = finished.clone() {
            shown = shown.child(match f {
                Finished::Signed { txid, tx_hex } => SignedCard {
                    txid,
                    tx_hex,
                    key_id: self.key_id.clone(),
                }
                .into_element(),
                other => finished_card(&p, nav, &other).into_element(),
            });
        }

        let confirmation: Option<Element> = if u.confirm_each_signer {
            Some(signer_confirmations(&p, &u, confirming(), {
                let app = app.clone();
                move |id: String| {
                    confirming.set(true);
                    confirm(app.confirm_signer(id));
                }
            }))
        } else {
            u.confirm.clone().map(|label| {
                let app = app.clone();
                tinted_card(p.accent_soft, p.on_accent_soft)
                    .child(text(
                        "When you are done on the signer side, confirm it here. Only confirm if you really did it: this \
                         is the coordinator’s only record.",
                    ))
                    .child(
                        Btn::new(label)
                            .icon(lucide::check())
                            .fill()
                            .busy(confirming())
                            .on_press(move |_| {
                                confirming.set(true);
                                confirm(app.confirm());
                            }),
                    )
                    .into_element()
            })
        };

        let scan = rect()
            .width(Size::fill())
            .spacing(8.)
            .child(title("Signers"))
            .child(text(if u.awaiting.is_empty() {
                "All replies received.".to_string()
            } else {
                format!("Waiting for {}:", u.awaiting.len())
            }))
            .child(
                rect()
                    .horizontal()
                    .width(Size::fill())
                    .content(Content::wrap_spacing(6.))
                    .spacing(6.)
                    .children(u.awaiting.iter().map(|s| signer_chip(&p, &s.device_id, s.name.clone(), false).into_element()))
                    .children(u.received.iter().map(|s| signer_chip(&p, &s.device_id, s.name.clone(), true).into_element())),
            )
            .child(gap(8.))
            .child(ScanPanel::new(on_payload).expecting(match broadcast.read().as_ref() {
                Some(b) => format!("signer reply to “{}”", b.title),
                None => "signer reply".to_string(),
            }));
        let side: Option<Element> = match finished {
            Some(_) => None,
            None => Some(confirmation.unwrap_or_else(|| scan.into_element())),
        };

        let done = finished.is_some();
        let close_app = app.clone();
        let bar = app_bar(&p, self.title.clone(), minifrost_ui::widgets::close(move |_| run(close(close_app.clone(), nav, done))));

        let body: Element = if narrow {
            // a phone has no room for two columns: the code first, the reply area below it
            page(720.)
                .child(shown)
                .map(side, |el, side| el.child(divider(&p)).child(side))
                .into_element()
        } else {
            rect()
                .horizontal()
                .width(Size::fill())
                .height(Size::flex(1.))
                .content(Content::flex())
                .child(rect().width(Size::flex(1.)).height(Size::fill()).child(page(900.).padding(24.).child(shown)))
                .map(side, |el, side| {
                    el.child(rect().width(Size::px(1.)).height(Size::fill()).background(p.border))
                        .child(rect().width(Size::px(480.)).height(Size::fill()).child(page(480.).padding(24.).child(side)))
                })
                .into_element()
        };

        rect().expanded().content(Content::flex()).child(bar).child(body)
    }
}

/// The code to show every signer, and what it is for.
fn shown_code(b: &Broadcast, message: &str, size: f32) -> Rect {
    rect()
        .width(Size::fill())
        .spacing(8.)
        .child(heading(b.title.clone()))
        .child(text(message.to_string()))
        .child(gap(8.))
        .child(AnimatedQr::new(b.frames.clone(), size).label(format!("Showing: {}", b.title)))
}

/// Keygen check round: one button per signer, pressed once that signer shows the code and the
/// user approved on it. Restoration: pressed once that signer read the code and saved its key.
fn signer_confirmations(p: &Palette, u: &Update, busy: bool, confirm: impl FnMut(String) + Clone + 'static) -> Element {
    let keygen = u.check_code.is_some();
    let mut signers: Vec<_> = u
        .received
        .iter()
        .map(|s| (s.clone(), true))
        .chain(u.awaiting.iter().map(|s| (s.clone(), false)))
        .collect();
    signers.sort_by(|a, b| a.0.name.cmp(&b.0.name));
    tinted_card(p.accent_soft, p.on_accent_soft)
        .child(text(if keygen {
            "Confirm each signer here once it shows the code above and you approved on it. Only confirm if you really \
             did: a signer you confirm without approving holds no share."
        } else {
            "Confirm each signer here once it read the code above and says its key is stored. Until it does it cannot sign."
        }))
        .children(signers.into_iter().map(|(s, done)| {
            if done {
                return rect().child(signer_chip(p, &s.device_id, s.name.clone(), true)).key(&s.device_id).into_element();
            }
            let mut confirm = confirm.clone();
            let id = s.device_id.clone();
            Btn::new(if keygen {
                format!("{} shows this code and I approved", s.name)
            } else {
                format!("{} saved its key", s.name)
            })
            .outline()
            .color(signer_color(&s.device_id, p.dark))
            .icon(lucide::check())
            .fill()
            .enabled(!busy)
            .on_press(move |_| confirm(id.clone()))
            .key(&s.device_id)
            .into_element()
        }))
        .into_element()
}

/// Result cards for the keygen, backup and restoration sessions.
fn finished_card(p: &Palette, nav: Nav, f: &Finished) -> Rect {
    let (icon_bytes, title_text, detail, button) = match f {
        Finished::KeyCreated { name, .. } => (
            lucide::circle_check(),
            format!("Wallet “{name}” created"),
            "Every signer stored its share when it approved the check code. Now back up every signer from the wallet \
             page: until a backup is written down, losing a signer can lose the wallet."
                .to_string(),
            "Done",
        ),
        Finished::BackupRecorded { key_name, share_index } => (
            lucide::circle_check(),
            format!("Backup of key #{share_index} recorded"),
            format!("You confirmed writing down the 25 words for “{key_name}”. Keep the paper safe and secret."),
            "Close",
        ),
        Finished::BackupChecked { key_name, share_index } => (
            lucide::badge_check(),
            format!("Backup of key #{share_index} checked"),
            format!("Every answer matched the signer’s share of “{key_name}”."),
            "Close",
        ),
        Finished::WalletRestored { name, .. } => (
            lucide::circle_check(),
            format!("Wallet “{name}” is restored"),
            "Every signer that took part holds its key and can sign.".to_string(),
            "Close",
        ),
        Finished::Signed { .. } => (lucide::circle_check(), "Done".to_string(), String::new(), "Close"),
    };
    rect()
        .width(Size::fill())
        .spacing(12.)
        .child(
            ListRow::new(label().font_weight(FontWeight::SEMI_BOLD).text(title_text))
                .icon(icon_bytes, p.success)
                .subtitle(detail),
        )
        .child(Btn::new(button).fill().on_press(move |_| nav.pop()))
}

/// A signed transaction: publish it through Esplora, or copy it.
#[derive(PartialEq)]
struct SignedCard {
    txid: String,
    tx_hex: String,
    key_id: Option<String>,
}

impl Component for SignedCard {
    fn render(&self) -> impl IntoElement {
        let p = use_palette();
        let app = use_consume::<App>();
        let nav = use_consume::<Nav>();
        let mut busy = use_state(|| false);
        let mut published = use_state(|| None::<String>);
        let mut error = use_state(|| None::<String>);
        let publish = {
            let (key_id, hex) = (self.key_id.clone(), self.tx_hex.clone());
            move |_| {
                let Some(key_id) = key_id.clone() else { return };
                let (app, hex) = (app.clone(), hex.clone());
                busy.set(true);
                error.set(None);
                run(async move {
                    match off_thread(&app, move |a| a.broadcast(key_id, hex)).await {
                        Ok(txid) => published.set(Some(txid)),
                        Err(e) => error.set(Some(e)),
                    }
                    busy.set(false);
                });
            }
        };
        let hex = self.tx_hex.clone();
        let copy = move |_| match Clipboard::set(hex.clone()) {
            Ok(()) => nav.overlay.toast("Raw transaction copied"),
            Err(e) => nav.overlay.toast(format!("Could not copy: {e:?}")),
        };

        rect()
            .width(Size::fill())
            .spacing(12.)
            .child(
                card(&p)
                    .child(
                        ListRow::new(label().font_weight(FontWeight::SEMI_BOLD).text("Signed by the threshold of signers"))
                            .icon(lucide::circle_check(), p.success)
                            .subtitle("The signature is valid. Nothing has been published yet."),
                    )
                    .child(SelectableText::new().width(Size::fill()).font_family(MONO).span(format!("txid {}", self.txid)))
                    .child(
                        button_row()
                            .child(
                                Btn::new("Publish via Esplora")
                                    .icon(lucide::globe())
                                    .busy(busy())
                                    .enabled(published.read().is_none() && self.key_id.is_some())
                                    .on_press(publish),
                            )
                            .child(Btn::new("Copy raw transaction").outline().icon(lucide::copy()).on_press(copy)),
                    )
                    .map(published.read().clone(), |el, txid| {
                        el.child(
                            SelectableText::new()
                                .width(Size::fill())
                                .font_family(MONO)
                                .color(p.success)
                                .span(format!("Published. txid {txid}")),
                        )
                    })
                    .map(error.read().clone(), |el, e| el.child(text(e).color(p.error))),
            )
            .child(Btn::new("Close").outline().fill().on_press(move |_| nav.pop()))
    }
}
