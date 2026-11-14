//! Reading the other device's animated QR code: the camera, the microphone (data over sound),
//! or its text typed or pasted in.

use std::{
    cell::RefCell,
    rc::Rc,
};

use freya::{
    elements::image::ImageHandle,
    engine::prelude::AlphaType,
    icons::lucide,
    prelude::*,
};
use futures_util::StreamExt;

use crate::{
    camera::{
        self,
        Camera,
        CameraEvent,
        CameraInfo,
    },
    overlay::use_overlay,
    sound,
    theme::use_palette,
    widgets::*,
};

/// Every `MF1:` frame in pasted text. Frames are upper case, but text that was lower-cased or
/// run together still counts.
pub fn frames_in(text: &str) -> Vec<String> {
    let upper = text.to_ascii_uppercase();
    let mut out = vec![];
    let mut rest = upper.as_str();
    while let Some(at) = rest.find("MF1:") {
        let tail = &rest[at + 4..];
        let len = tail
            .find(|c: char| !(c.is_ascii_uppercase() || c.is_ascii_digit() || c == ':'))
            .unwrap_or(tail.len());
        out.push(format!("MF1:{}", &tail[..len]));
        rest = &tail[len..];
    }
    out
}

/// Collects the frames of animated codes and tells what came of each one read.
#[derive(Clone)]
struct Collector {
    assembler: Rc<RefCell<qr_transport::Assembler>>,
    progress: State<(usize, usize)>,
    problem: State<Option<String>>,
    on_payload: EventHandler<Vec<u8>>,
}

impl Collector {
    fn push(&self, text: &str) {
        let (mut progress, mut problem) = (self.progress, self.problem);
        let result = self.assembler.borrow_mut().push(text);
        match result {
            Ok(p) => {
                problem.set_if_modified(None);
                match p.complete {
                    Some(payload) => {
                        progress.set_if_modified((0, 0));
                        self.on_payload.call(payload);
                    }
                    None => progress.set_if_modified((p.received, p.total)),
                }
            }
            Err(e) => problem.set_if_modified(Some(e.to_string())),
        }
    }
}

/// Camera preview and QR frame assembly, with "Listen" (the same frames as sound from a device
/// playing them), "Input" (type QR text) and "Paste". Calls `on_payload` once per completed code;
/// the camera keeps reading, so a code still in view completes again.
#[derive(PartialEq)]
pub struct ScanPanel {
    pub on_payload: EventHandler<Vec<u8>>,
    pub hint: String,
    /// The kind of message this scan is waiting for, shown above the camera.
    pub expecting: Option<String>,
}

impl ScanPanel {
    pub fn new(on_payload: impl Into<EventHandler<Vec<u8>>>) -> Self {
        Self {
            on_payload: on_payload.into(),
            hint: "Hold the signer’s screen up to the camera".into(),
            expecting: None,
        }
    }
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = hint.into();
        self
    }
    pub fn expecting(mut self, expecting: impl Into<String>) -> Self {
        self.expecting = Some(expecting.into());
        self
    }
}

impl Component for ScanPanel {
    fn render(&self) -> impl IntoElement {
        let p = use_palette();
        let overlay = use_overlay();
        let progress = use_state(|| (0usize, 0usize));
        let problem = use_state(|| None::<String>);
        let mut cameras = use_state(|| None::<Vec<CameraInfo>>);
        let mut selected = use_state(|| None::<u32>);
        let mut camera_error = use_state(|| None::<String>);
        let mut preview = use_state(|| None::<ImageHandle>);
        let mut listening = use_state(|| false);
        let mut sound_error = use_state(|| None::<String>);
        let mut show_input = use_state(|| false);
        let mut input = use_state(String::new);
        let camera = use_hook(|| Rc::new(RefCell::new(None::<Camera>)));
        let camera_task = use_hook(|| Rc::new(RefCell::new(None::<TaskHandle>)));
        let listener = use_hook(|| Rc::new(RefCell::new(None::<TaskHandle>)));

        // the handler may change from one render to the next; the collector always calls the latest
        let mut on_payload = use_state(|| self.on_payload.clone());
        on_payload.set_if_modified(self.on_payload.clone());
        let collector = use_hook(|| Collector {
            assembler: Rc::new(RefCell::new(qr_transport::Assembler::new())),
            progress,
            problem,
            on_payload: EventHandler::new(move |payload| on_payload.peek().call(payload)),
        });

        let start_camera = {
            let (camera, camera_task, collector) = (camera.clone(), camera_task.clone(), collector.clone());
            move |index: u32| {
                // the old camera stops before the new one opens
                if let Some(task) = camera_task.borrow_mut().take() {
                    task.cancel();
                }
                drop(camera.borrow_mut().take());
                selected.set(Some(index));
                camera_error.set(None);
                let (slot, collector) = (camera.clone(), collector.clone());
                let task = spawn(async move {
                    let (cam, mut events) = match camera::open(index).await {
                        Ok(opened) => opened,
                        Err(e) => {
                            camera_error.set(Some(e));
                            return;
                        }
                    };
                    *slot.borrow_mut() = Some(cam);
                    while let Some(event) = events.next().await {
                        match event {
                            CameraEvent::Preview { width, height, rgba } => {
                                preview.set(ImageHandle::from_rgba(width, height, rgba.into(), AlphaType::Opaque));
                            }
                            CameraEvent::Qr(text) => collector.push(&text),
                            CameraEvent::Error(e) => camera_error.set(Some(e)),
                        }
                    }
                });
                *camera_task.borrow_mut() = Some(task);
            }
        };

        use_hook({
            let mut start_camera = start_camera.clone();
            move || match camera::list_cameras() {
                Ok(list) => {
                    if let Some(first) = list.first() {
                        start_camera(first.index);
                    }
                    cameras.set(Some(list));
                }
                Err(e) => {
                    cameras.set(Some(vec![]));
                    camera_error.set(Some(e));
                }
            }
        });

        use_drop({
            let (camera, camera_task, listener) = (camera.clone(), camera_task.clone(), listener.clone());
            move || {
                if let Some(task) = camera_task.borrow_mut().take() {
                    task.cancel();
                }
                drop(camera.borrow_mut().take());
                if let Some(task) = listener.borrow_mut().take() {
                    task.cancel();
                    sound::stop_listening();
                }
            }
        });

        let toggle_listening = {
            let (listener, collector) = (listener.clone(), collector.clone());
            move |_| {
                if let Some(task) = listener.borrow_mut().take() {
                    task.cancel();
                    sound::stop_listening();
                    listening.set(false);
                    return;
                }
                sound_error.set(None);
                let (slot, collector) = (listener.clone(), collector.clone());
                let task = spawn(async move {
                    match sound::listen().await {
                        Ok(mut frames) => {
                            listening.set(true);
                            while let Some(text) = frames.next().await {
                                collector.push(&text);
                            }
                        }
                        Err(e) => sound_error.set(Some(e)),
                    }
                    listening.set(false);
                    slot.borrow_mut().take();
                });
                *listener.borrow_mut() = Some(task);
            }
        };

        let add_text = {
            let collector = collector.clone();
            move |text: &str| -> bool {
                let frames = frames_in(text);
                if frames.is_empty() {
                    overlay.toast("No QR frames (MF1:…) found in that text.");
                    return false;
                }
                for f in frames {
                    collector.push(&f);
                }
                true
            }
        };
        let paste = {
            let add_text = add_text.clone();
            move |_| match Clipboard::get() {
                Ok(text) => {
                    add_text(&text);
                }
                Err(e) => overlay.toast(format!("Could not read the clipboard: {e:?}")),
            }
        };
        let submit = move |_: ()| {
            let typed = input.peek().clone();
            if add_text(&typed) {
                input.set(String::new());
            }
        };

        let (received, total) = *progress.read();
        let cams = cameras.read().clone();
        let no_camera = if cfg!(target_os = "android") {
            "No camera available. You can input or paste QR text instead."
        } else {
            "No camera found (/dev/video*). You can input or paste QR text instead."
        };

        rect()
            .width(Size::fill())
            .spacing(8.)
            .map(self.expecting.clone(), |el, e| {
                el.child(kind_label(&p, lucide::scan_qr_code(), format!("Expecting: {e}")))
            })
            .map(cams.clone(), |el, cams| {
                el.maybe(cams.is_empty(), |el| el.child(card(&p).child(text(no_camera))))
                    .maybe(cams.len() > 1, |el| {
                        el.child(button_row().children(cams.iter().map(|c| {
                            let mut start = start_camera.clone();
                            let index = c.index;
                            Btn::new(c.name.clone())
                                .kind(if selected() == Some(index) { BtnKind::Tonal } else { BtnKind::Outline })
                                .on_press(move |_| start(index))
                                .key(index)
                                .into_element()
                        })))
                    })
            })
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::px(300.))
                    .corner_radius(12.)
                    .overflow(Overflow::Clip)
                    .background(Color::BLACK)
                    .center()
                    .child(match preview.read().clone() {
                        Some(handle) => image(handle)
                            .width(Size::fill())
                            .height(Size::fill())
                            .aspect_ratio(AspectRatio::Min)
                            .image_cover(ImageCover::Center)
                            .into_element(),
                        None if cams.is_none() => CircularLoader::new().into_element(),
                        None => icon(lucide::video_off(), 48., Color::from_af32rgb(0.5, 255, 255, 255)).into_element(),
                    }),
            )
            .map(camera_error.read().clone(), |el, e| el.child(text(e).color(p.error)))
            .child(
                text(problem.read().clone().unwrap_or_else(|| self.hint.clone()))
                    .text_align(TextAlign::Center),
            )
            .maybe(total > 1, |el| {
                el.child(progress_line(&p, received as f32 / total as f32)).child(
                    small(format!("{received} of {total} parts"), p.text_muted).text_align(TextAlign::Center),
                )
            })
            .child(
                rect()
                    .horizontal()
                    .width(Size::fill())
                    .content(Content::wrap_spacing(4.))
                    .main_align(Alignment::center())
                    .child(
                        Btn::new(if listening() { "Stop listening" } else { "Listen" })
                            .text()
                            .icon(if listening() { lucide::ear_off() } else { lucide::ear() })
                            .on_press(toggle_listening),
                    )
                    .child(Btn::new("Input").text().icon(lucide::keyboard()).on_press(move |_| show_input.toggle()))
                    .child(Btn::new("Paste").text().icon(lucide::clipboard_paste()).on_press(paste)),
            )
            .maybe(listening(), |el| {
                el.child(
                    text("Listening… press “Play sound” on the other device and keep it close, in a quiet room.")
                        .text_align(TextAlign::Center),
                )
            })
            .map(sound_error.read().clone(), |el, e| el.child(text(e).color(p.error).text_align(TextAlign::Center)))
            .maybe(show_input(), |el| {
                el.child(field(
                    &p,
                    "QR text",
                    Input::new(input)
                        .placeholder("MF1:…")
                        .width(Size::fill())
                        .auto_focus(true)
                        .on_submit({
                            let mut submit = submit.clone();
                            move |_| submit(())
                        }),
                ))
                .child(rect().width(Size::fill()).cross_align(Alignment::end()).child(Btn::new("Submit").text().on_press(submit.clone())))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_frames_in_text() {
        assert_eq!(frames_in("MF1:AB:12\nmf1:cd:34"), vec!["MF1:AB:12", "MF1:CD:34"]);
        assert_eq!(frames_in(" xx MF1:A1 yy"), vec!["MF1:A1"]);
        assert!(frames_in("nothing here").is_empty());
    }
}
