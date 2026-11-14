use minifrost_signer::api::new_signer;
use minifrost_ui::prelude::*;

use crate::ctl::{
    Ctl,
    Vault,
};

const MAX_NAME: usize = 40;

/// First run: name this signer and create it.
#[derive(PartialEq)]
pub struct Setup {
    pub vault: Vault,
    pub on_created: EventHandler<Ctl>,
}

impl Component for Setup {
    fn render(&self) -> impl IntoElement {
        let p = use_palette();
        let mut name = use_state(String::new);
        let mut error = use_state(|| None::<String>);
        let trimmed = name.read().trim().to_string();
        let create = {
            let (vault, on_created) = (self.vault.clone(), self.on_created.clone());
            move |_: ()| {
                let n = name.peek().trim().to_string();
                if n.is_empty() {
                    return;
                }
                let signer = new_signer(n);
                // stored before anything else happens
                match vault.save(&signer.snapshot()) {
                    Ok(()) => on_created.call(Ctl::new(signer, vault.clone())),
                    Err(e) => error.set(Some(e)),
                }
            }
        };

        rect().expanded().child(app_bar(&p, "Set up signer", None)).child(
            page(640.)
                .padding(24.)
                .child(text(
                    "This device becomes one signer of a threshold wallet. It never connects to the internet: it talks \
                     to the coordinator only through QR codes. Its key share is stored unencrypted on this device.",
                ))
                .child(gap(8.))
                .child(field(
                    &p,
                    "Name of this signer",
                    Input::new(WritableSetter::new(name, move |v: String| {
                        name.set(v.chars().take(MAX_NAME).collect())
                    }))
                    .placeholder("e.g. Alice")
                    .width(Size::fill())
                    .on_submit({
                        let mut create = create.clone();
                        move |_| create(())
                    }),
                ))
                .child(small(format!("{} / {MAX_NAME}", trimmed.chars().count()), p.text_muted).text_align(TextAlign::End))
                .map(error.read().clone(), |el, e| el.child(text(e).color(p.error)))
                .child(Btn::new("Create signer").enabled(!trimmed.is_empty()).fill().on_press(create)),
        )
    }
}
