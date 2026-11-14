use minifrost_coordinator::api::{
    KeyView,
    SignerInfo,
};
use minifrost_ui::prelude::*;

use crate::nav::{
    App,
    Nav,
    Page,
};

#[derive(PartialEq)]
pub struct SignersPage;

impl Component for SignersPage {
    fn render(&self) -> impl IntoElement {
        let p = use_palette();
        let app = use_consume::<App>();
        let nav = use_consume::<Nav>();
        nav.watch();
        let signers = app.signers();

        rect()
            .expanded()
            .child(app_bar(&p, "Signers", None))
            .child(
                page(720.)
                    .maybe(signers.is_empty(), |el| {
                        el.child(text(
                            "No signers yet. On each signer phone choose “Show enrolment code”, then add it here.",
                        ))
                    })
                    .children(signers.into_iter().map(|s| {
                        let forget = {
                            let (app, s) = (app.clone(), s.clone());
                            move |_| run(forget(app.clone(), nav, s.clone()))
                        };
                        ListRow::new(signer_name(&p, &s.device_id, s.name.clone()))
                            .icon(lucide::smartphone(), signer_color(&s.device_id, p.dark))
                            .subtitle(short_id(&s.device_id, 24))
                            .trailing(IconBtn::new(lucide::trash_2(), forget))
                            .key(&s.device_id)
                            .into_element()
                    }))
                    .child(gap(72.)),
            )
            .child(floating(
                Btn::new("Add signer")
                    .icon(lucide::scan_qr_code())
                    .on_press(move |_| nav.push(Page::Enroll)),
            ))
    }
}

fn last_holder(k: &KeyView, share_index: u32) -> bool {
    k.signers.iter().filter(|h| h.share_index == share_index).count() == 1
}

/// Forget a signer, and with it the keys this coordinator knows it holds. The signer keeps them.
async fn forget(app: App, nav: Nav, s: SignerInfo) {
    let held: Vec<(KeyView, u32)> = app
        .keys()
        .into_iter()
        .flat_map(|k| {
            k.signers
                .iter()
                .filter(|h| h.device_id == s.device_id)
                .map(|h| (k.clone(), h.share_index))
                .collect::<Vec<_>>()
        })
        .collect();
    let effect = if held.is_empty() {
        format!("{} holds no key of a wallet here.", s.name)
    } else {
        let lines: Vec<String> = held
            .iter()
            .map(|(k, i)| {
                let alone = if last_holder(k, *i) { " (no other signer holds it)" } else { "" };
                format!("• “{}”, key #{i}{alone}", k.name)
            })
            .collect();
        format!("{} stops counting for:\n{}", s.name, lines.join("\n"))
    };
    let short = held
        .iter()
        .any(|(k, i)| last_holder(k, *i) && k.n_keys.saturating_sub(1) < k.threshold);
    let short = if short {
        "A wallet is then left with fewer keys than it needs to sign, until a signer with that key joins again.\n\n"
    } else {
        ""
    };
    let ok = nav
        .overlay
        .confirm(
            format!("Forget {}?", s.name),
            format!(
                "{effect}\n\n{short}Nothing changes on the signer: it keeps its keys. To add it back, enrol it again, \
                 and show the restore code of each key (tap the key on the signer) on the Restore page."
            ),
            "Forget",
            !held.is_empty(),
        )
        .await;
    if !ok {
        return;
    }
    if let Err(e) = app.forget_signer(s.device_id.clone()) {
        nav.overlay.toast(e);
    }
    nav.changed();
}

/// Reads a signer's enrolment code.
#[derive(PartialEq)]
pub struct EnrollPage;

impl Component for EnrollPage {
    fn render(&self) -> impl IntoElement {
        let p = use_palette();
        let app = use_consume::<App>();
        let nav = use_consume::<Nav>();
        let mut error = use_state(|| None::<String>);
        let on_payload = move |payload: Vec<u8>| match app.enroll(payload) {
            Ok(s) => {
                nav.overlay
                    .success(format!("Read enrolment code: added “{}”.", s.name), Some(signer_color(&s.device_id, p.dark)));
                nav.pop();
            }
            Err(e) => error.set(Some(e)),
        };

        rect().expanded().child(app_bar(&p, "Add signer", back(move |_| nav.pop()))).child(
            page(560.)
                .child(
                    ScanPanel::new(on_payload)
                        .hint("Show the signer’s enrolment code to the camera")
                        .expecting("signer enrolment code")
                        .into_element(),
                )
                .map(error.read().clone(), |el, e| el.child(text(e).color(p.error))),
        )
    }
}
