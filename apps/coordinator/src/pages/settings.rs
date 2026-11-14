use minifrost_ui::prelude::*;

use crate::nav::{
    App,
    Nav,
};

#[derive(PartialEq)]
pub struct SettingsPage;

impl Component for SettingsPage {
    fn render(&self) -> impl IntoElement {
        let p = use_palette();
        let app = use_consume::<App>();
        let nav = use_consume::<Nav>();
        let mut url = use_state({
            let app = app.clone();
            move || app.esplora_url("signet".into()).unwrap_or_default()
        });
        let mut saved = use_state(|| None::<&'static str>);
        let save = {
            let app = app.clone();
            move |_| match app.set_esplora_url(Some(url.peek().trim().to_string())) {
                Ok(()) => saved.set(Some("Saved")),
                Err(e) => nav.overlay.toast(e),
            }
        };
        let reset = move |_| match app.set_esplora_url(None) {
            Ok(()) => {
                url.set(app.esplora_url("signet".into()).unwrap_or_default());
                saved.set(Some("Reset to default"));
            }
            Err(e) => nav.overlay.toast(e),
        };

        rect().expanded().child(app_bar(&p, "Settings", None)).child(
            page(720.)
                .padding(24.)
                .child(title("Esplora backend"))
                .child(text(
                    "The coordinator is the only part of the system that goes online. It reads balances and publishes \
                     signed transactions through this Esplora server. Signers never see it.",
                ))
                .child(field(
                    &p,
                    "Base URL",
                    Input::new(url).placeholder("https://mempool.space/api").width(Size::fill()),
                ))
                .child(
                    button_row()
                        .child(Btn::new("Save").on_press(save))
                        .child(Btn::new("Use default").text().on_press(reset))
                        .map(saved(), |el, s| el.child(label().text(s))),
                )
                .child(small(
                    "The URL applies to all wallets. Defaults per network: mempool.space (mainnet, testnet), mutinynet.com \
                     (mutinynet).",
                    p.text_muted,
                )),
        )
    }
}
