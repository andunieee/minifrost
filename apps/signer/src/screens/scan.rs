use minifrost_ui::prelude::*;

use crate::{
    ctl::Ctl,
    nav::{
        self,
        Nav,
    },
};

/// Reads a coordinator code (camera, sound, or pasted text) and follows wherever it leads.
#[derive(PartialEq)]
pub struct ScanScreen {
    pub expecting: String,
}

impl Component for ScanScreen {
    fn render(&self) -> impl IntoElement {
        let p = use_palette();
        let ctl = use_consume::<Ctl>();
        let nav = use_consume::<Nav>();
        // the camera can read the same code again before this screen is gone: only the first counts
        let mut delivered = use_state(|| false);
        let on_payload = move |payload: Vec<u8>| {
            if *delivered.peek() {
                return;
            }
            delivered.set(true);
            run(nav::receive(ctl.clone(), nav, payload));
        };

        rect()
            .expanded()
            .child(app_bar(&p, "Scan", back(move |_| nav.pop())))
            .child(
                page(640.).padding(12.).child(
                    ScanPanel::new(on_payload)
                        .hint("Point the camera at the coordinator’s QR code")
                        .expecting(self.expecting.clone()),
                ),
            )
    }
}
