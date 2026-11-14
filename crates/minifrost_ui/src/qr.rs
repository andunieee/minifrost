//! Showing data as QR codes: one still code, or the frames of a message on a loop.

use std::time::Duration;

use freya::{
    elements::image::ImageHandle,
    engine::prelude::AlphaType,
    icons::lucide,
    prelude::*,
};
use qrcode::{
    EcLevel,
    QrCode,
};

use crate::{
    overlay::use_overlay,
    sound,
    theme::{
        qr_ink,
        use_palette,
    },
    widgets::*,
};

/// White modules around the code, so it reads on a dark screen too.
const QUIET: usize = 2;

/// `data` as a QR code image, one pixel per module (draw it with nearest sampling), in `ink` on
/// white.
pub fn qr_image(data: &str, ink: Color) -> Option<ImageHandle> {
    let code = QrCode::with_error_correction_level(data.as_bytes(), EcLevel::M).ok()?;
    let modules = code.to_colors();
    let width = code.width();
    let side = width + 2 * QUIET;
    let mut rgba = vec![255u8; side * side * 4];
    for (i, module) in modules.iter().enumerate() {
        if *module == qrcode::Color::Dark {
            let (x, y) = (i % width + QUIET, i / width + QUIET);
            let at = (y * side + x) * 4;
            rgba[at..at + 3].copy_from_slice(&[ink.r(), ink.g(), ink.b()]);
        }
    }
    ImageHandle::from_rgba(side as u32, side as u32, rgba.into(), AlphaType::Opaque)
}

/// A QR code `size` pixels wide on a white card.
pub fn qr_view(data: &str, size: f32, ink: Color) -> Rect {
    rect()
        .padding(12.)
        .corner_radius(12.)
        .background(Color::WHITE)
        .maybe_child(qr_image(data, ink).map(|handle| {
            image(handle)
                .width(Size::px(size))
                .height(Size::px(size))
                .sampling_mode(SamplingMode::Nearest)
        }))
}

/// Shows a list of text frames as a looping QR code; a single frame is shown still. It can also
/// play the same frames as sound for a listening device, copy them as text, or show them raw.
#[derive(PartialEq)]
pub struct AnimatedQr {
    pub frames: Vec<String>,
    pub size: f32,
    /// What kind of message this code carries, shown above it.
    pub label: Option<String>,
    /// Whose code this is: frames the code and tints its modules, so the scanning side can tell
    /// at a glance which device it is pointed at.
    pub color: Option<Color>,
}

impl AnimatedQr {
    pub fn new(frames: Vec<String>, size: f32) -> Self {
        Self {
            frames,
            size,
            label: None,
            color: None,
        }
    }
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

const FPS: u64 = 4;

impl Component for AnimatedQr {
    fn render(&self) -> impl IntoElement {
        let p = use_palette();
        let overlay = use_overlay();
        let mut index = use_state(|| 0usize);
        let mut show_raw = use_state(|| false);
        let mut playing = use_state(|| false);
        let mut sound_error = use_state(|| None::<String>);

        use_hook(move || {
            spawn(async move {
                loop {
                    timer(Duration::from_millis(1000 / FPS)).await;
                    // the raw text lists every part, so the code holds still while it is read
                    if !*show_raw.peek() {
                        *index.write() += 1;
                    }
                }
            })
        });
        // new frames while playing: play those instead
        use_side_effect_with_deps(&self.frames, move |frames| {
            if *playing.peek() && let Err(e) = sound::play(frames) {
                playing.set(false);
                sound_error.set(Some(e));
            }
        });
        use_drop(move || {
            if *playing.peek() {
                sound::stop_playing();
            }
        });

        if self.frames.is_empty() {
            return rect().width(Size::px(self.size)).height(Size::px(self.size)).into_element();
        }
        let i = *index.read() % self.frames.len();
        let ink = self.color.map(qr_ink).unwrap_or(Color::BLACK);
        let frames = self.frames.clone();
        let copied = frames.join("\n");

        rect()
            .width(Size::fill())
            .cross_align(Alignment::center())
            .spacing(8.)
            .map(self.label.clone(), |el, l| el.child(kind_label(&p, lucide::qr_code(), l)))
            .child(
                rect()
                    .corner_radius(16.)
                    .map(self.color, |el, c| el.background(c).padding(8.))
                    .child(qr_view(&self.frames[i], self.size, ink)),
            )
            .maybe(self.frames.len() > 1, |el| {
                el.child(small(format!("part {} of {}", i + 1, self.frames.len()), p.text_muted).text_align(TextAlign::Center))
            })
            .child(
                rect()
                    .horizontal()
                    .content(Content::wrap_spacing(4.))
                    .main_align(Alignment::center())
                    .width(Size::fill())
                    .child(Btn::new("Copy contents").text().icon(lucide::copy()).on_press(move |_| {
                        match Clipboard::set(copied.clone()) {
                            Ok(()) => overlay.toast("Copied QR contents"),
                            Err(e) => overlay.toast(format!("Could not copy: {e:?}")),
                        }
                    }))
                    .child(
                        Btn::new(if playing() { "Stop sound" } else { "Play sound" })
                            .text()
                            .icon(if playing() { lucide::square() } else { lucide::volume_2() })
                            .on_press(move |_| {
                                if playing() {
                                    sound::stop_playing();
                                    playing.set(false);
                                    return;
                                }
                                sound_error.set(None);
                                match sound::play(&frames) {
                                    Ok(()) => playing.set(true),
                                    Err(e) => sound_error.set(Some(e)),
                                }
                            }),
                    )
                    .child(
                        Btn::new(if show_raw() { "Hide raw" } else { "Show raw" })
                            .text()
                            .icon(if show_raw() { lucide::eye_off() } else { lucide::eye() })
                            .on_press(move |_| show_raw.toggle()),
                    ),
            )
            .maybe(playing(), |el| {
                el.child(text("Playing… the other device should be listening, close by.").text_align(TextAlign::Center))
            })
            .map(sound_error.read().clone(), |el, e| el.child(text(e).color(p.error).text_align(TextAlign::Center)))
            .maybe(show_raw(), |el| {
                el.child(
                    ScrollView::new()
                        .width(Size::px(self.size + 24.))
                        .height(Size::auto())
                        .max_height(Size::px(200.))
                        .child(
                            SelectableText::new()
                                .width(Size::fill())
                                .font_family(MONO)
                                .font_size(12.)
                                .span(self.frames.join("\n")),
                        ),
                )
            })
            .into_element()
    }
}
