use minifrost_ui::prelude::*;

use crate::{
    ctl::Ctl,
    nav::{
        Nav,
        Screen,
    },
};

/// Shows QR frames for the coordinator to scan. With `next` set, it offers to scan the
/// coordinator's following code from here.
#[derive(PartialEq)]
pub struct ReplyScreen {
    pub frames: Vec<String>,
    pub title: String,
    pub what: String,
    pub next: Option<String>,
}

impl Component for ReplyScreen {
    fn render(&self) -> impl IntoElement {
        let p = use_palette();
        let ctl = use_consume::<Ctl>();
        let nav = use_consume::<Nav>();
        let window = Platform::get().current_window();
        let area = *window.root_size.read();
        let size = (area.width.min(area.height) - 88.).clamp(200., 420.);
        let next = self.next.clone();

        rect()
            .expanded()
            .child(app_bar(&p, self.title.clone(), back(move |_| nav.pop())))
            .child(
                page(640.)
                    .child(
                        AnimatedQr::new(self.frames.clone(), size)
                            .label(format!("Showing: {}", self.what))
                            .color(signer_color(&ctl.device_id, p.dark)),
                    )
                    .child(text(self.what.clone()).text_align(TextAlign::Center))
                    .child(text("Hold the device steady until the coordinator has all parts.").text_align(TextAlign::Center))
                    .child(gap(8.))
                    .child(match next {
                        Some(next) => rect()
                            .width(Size::fill())
                            .spacing(8.)
                            .child(
                                text(format!("Once the coordinator has every code it needs it shows “{next}”."))
                                    .text_align(TextAlign::Center),
                            )
                            .child(
                                Btn::new("Scan next code")
                                    .icon(lucide::scan_qr_code())
                                    .fill()
                                    // on top of this screen, so backing out of the scanner shows the reply again
                                    .on_press(move |_| nav.push(Screen::Scan { expecting: next.clone() })),
                            )
                            .child(Btn::new("Back to home").outline().fill().on_press(move |_| nav.home()))
                            .into_element(),
                        None => Btn::new("Done").fill().on_press(move |_| nav.home()).into_element(),
                    }),
            )
    }
}
