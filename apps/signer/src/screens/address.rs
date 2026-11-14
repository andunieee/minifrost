use minifrost_ui::prelude::*;

use crate::nav::Nav;

/// A receive address this signer derived from its own key, to compare with the coordinator's.
#[derive(PartialEq)]
pub struct AddressScreen {
    pub key_name: String,
    pub label: String,
    pub address: String,
}

impl Component for AddressScreen {
    fn render(&self) -> impl IntoElement {
        let p = use_palette();
        let nav = use_consume::<Nav>();

        rect()
            .expanded()
            .child(app_bar(&p, "Verify address", back(move |_| nav.pop())))
            .child(
                page(720.)
                    .child(text(format!("Wallet “{}”", self.key_name)).font_weight(FontWeight::BOLD))
                    .child(label().font_size(28.).text(self.label.clone()))
                    .child(mono(group_address(&self.address), 22.).width(Size::fill()))
                    .child(tinted_card(p.accent_soft, p.on_accent_soft).child(text(
                        "This signer worked the address out from its own key. Compare it, character by character, with \
                         the address on the coordinator. Give it to a sender only if they are identical.",
                    )))
                    .child(gap(8.))
                    .child(Btn::new("Done").fill().on_press(move |_| nav.home())),
            )
    }
}
