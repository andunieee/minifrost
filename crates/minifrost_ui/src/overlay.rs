//! What shows above the screens: a toast at the bottom and dialogs the code can wait on, so a
//! flow reads top to bottom (`if !overlay.confirm(..).await { return }`).

use std::{
    cell::RefCell,
    future::Future,
    rc::Rc,
    time::Duration,
};

use freya::{
    icons::lucide,
    prelude::*,
};
use futures_channel::oneshot;

use crate::{
    theme::{
        readable_on,
        use_palette,
    },
    widgets::*,
};

/// Runs a flow to its end, even if the screen that started it goes away meanwhile.
pub fn run(future: impl Future<Output = ()> + 'static) {
    spawn_in_window(future);
}

#[derive(Clone, PartialEq)]
struct Toast {
    id: u64,
    message: String,
    background: Option<Color>,
    icon: Option<Bytes>,
}

#[derive(Clone, PartialEq)]
pub struct Choice {
    pub icon: Bytes,
    pub label: String,
    pub danger: bool,
}

impl Choice {
    pub fn new(icon: Bytes, label: impl Into<String>) -> Self {
        Self {
            icon,
            label: label.into(),
            danger: false,
        }
    }
    pub fn danger(mut self) -> Self {
        self.danger = true;
        self
    }
}

#[derive(Clone, PartialEq)]
enum Kind {
    Message,
    Confirm { action: String, cancel: String, danger: bool },
    Typing { name: String, action: String },
    Choose { choices: Vec<Choice> },
}

type Reply = Rc<RefCell<Option<oneshot::Sender<Option<usize>>>>>;

#[derive(Clone)]
struct Dialog {
    id: u64,
    title: String,
    body: String,
    kind: Kind,
    reply: Reply,
}

impl PartialEq for Dialog {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

/// Toasts and dialogs. Provided at the root by [`use_init_overlay`]; it is `Copy`, so tasks keep it.
#[derive(Clone, Copy, PartialEq)]
pub struct Overlay {
    toast: State<Option<Toast>>,
    dialogs: State<Vec<Dialog>>,
    next_id: State<u64>,
}

pub fn use_init_overlay() -> Overlay {
    use_provide_context(|| Overlay {
        toast: State::create(None),
        dialogs: State::create(vec![]),
        next_id: State::create(0),
    })
}

pub fn use_overlay() -> Overlay {
    use_consume::<Overlay>()
}

impl Overlay {
    fn id(mut self) -> u64 {
        let id = *self.next_id.peek() + 1;
        self.next_id.set(id);
        id
    }

    fn show(mut self, message: String, background: Option<Color>, icon: Option<Bytes>, duration: Duration) {
        let id = self.id();
        self.toast.set(Some(Toast {
            id,
            message,
            background,
            icon,
        }));
        let mut toast = self.toast;
        run(async move {
            timer(duration).await;
            if toast.peek().as_ref().is_some_and(|t| t.id == id) {
                toast.set(None);
            }
        });
    }

    /// A plain message, mostly what went wrong.
    pub fn toast(self, message: impl ToString) {
        self.show(message.to_string(), None, None, Duration::from_secs(4));
    }

    /// "It worked", in green or in `color` (the colour of the signer it is about).
    pub fn success(self, message: impl Into<String>, color: Option<Color>) {
        self.show(
            message.into(),
            Some(color.unwrap_or(Color::from_rgb(46, 125, 50))),
            Some(lucide::circle_check()),
            Duration::from_secs(4),
        );
    }

    /// "Nothing new", briefly, so repeats never hold up the next real result.
    pub fn info(self, message: impl Into<String>, color: Option<Color>) {
        self.show(
            message.into(),
            Some(color.unwrap_or(Color::from_rgb(69, 90, 100))),
            Some(lucide::info()),
            Duration::from_secs(2),
        );
    }

    fn open(mut self, title: String, body: String, kind: Kind) -> impl Future<Output = Option<usize>> {
        let (tx, rx) = oneshot::channel();
        let id = self.id();
        self.dialogs.write().push(Dialog {
            id,
            title,
            body,
            kind,
            reply: Rc::new(RefCell::new(Some(tx))),
        });
        async move { rx.await.ok().flatten() }
    }

    /// Shows a message until it is dismissed.
    pub async fn alert(self, title: impl Into<String>, body: impl Into<String>) {
        self.open(title.into(), body.into(), Kind::Message).await;
    }

    /// A yes/no question; true only when `action` was pressed. `danger` draws it in the error
    /// colour, for what can't be undone.
    pub async fn confirm(
        self,
        title: impl Into<String>,
        body: impl Into<String>,
        action: impl Into<String>,
        danger: bool,
    ) -> bool {
        self.ask(title, body, action, "Cancel", danger).await
    }

    /// Like [`Overlay::confirm`], with the label of the button that says no.
    pub async fn ask(
        self,
        title: impl Into<String>,
        body: impl Into<String>,
        action: impl Into<String>,
        cancel: impl Into<String>,
        danger: bool,
    ) -> bool {
        let kind = Kind::Confirm {
            action: action.into(),
            cancel: cancel.into(),
            danger,
        };
        self.open(title.into(), body.into(), kind).await == Some(0)
    }

    /// The last step before something that can't be undone: `action` only unlocks once `name` is
    /// typed in exactly.
    pub async fn confirm_typing(
        self,
        title: impl Into<String>,
        body: impl Into<String>,
        name: impl Into<String>,
        action: impl Into<String>,
    ) -> bool {
        let kind = Kind::Typing {
            name: name.into(),
            action: action.into(),
        };
        self.open(title.into(), body.into(), kind).await == Some(0)
    }

    /// A list of actions to pick one from; None when dismissed.
    pub async fn choose(self, title: impl Into<String>, subtitle: impl Into<String>, choices: Vec<Choice>) -> Option<usize> {
        self.open(title.into(), subtitle.into(), Kind::Choose { choices }).await
    }

    fn answer(mut self, id: u64, answer: Option<usize>) {
        let mut dialogs = self.dialogs.write();
        if let Some(i) = dialogs.iter().position(|d| d.id == id) {
            let d = dialogs.remove(i);
            if let Some(tx) = d.reply.borrow_mut().take() {
                let _ = tx.send(answer);
            }
        }
    }

    /// Dismisses the dialog on top (the Back key); false when there was none.
    pub fn dismiss(self) -> bool {
        let top = self.dialogs.peek().last().map(|d| d.id);
        match top {
            Some(id) => {
                self.answer(id, None);
                true
            }
            None => false,
        }
    }
}

/// Draws the toast and the dialog on top. Put it last in the root, over the screens.
#[derive(PartialEq)]
pub struct OverlayHost;

impl Component for OverlayHost {
    fn render(&self) -> impl IntoElement {
        let overlay = use_overlay();
        let p = use_palette();
        let insets = crate::platform::use_insets();
        let toast = overlay.toast.read().clone();
        let dialog = overlay.dialogs.read().last().cloned();

        rect()
            .layer(Layer::Overlay)
            .position(Position::new_global())
            .width(Size::window_percent(100.))
            .height(Size::window_percent(100.))
            .map(dialog, |el, d| el.child(DialogView { dialog: d, overlay }))
            .map(toast, |el, t| {
                let background = t.background.unwrap_or(if p.dark {
                    Color::from_rgb(225, 225, 232)
                } else {
                    Color::from_rgb(48, 48, 56)
                });
                let color = readable_on(background);
                el.child(
                    rect()
                        .position(Position::new_absolute().bottom(insets.bottom + 24.).left(0.))
                        .width(Size::fill())
                        .padding((0., 16.))
                        .cross_align(Alignment::center())
                        .child(
                            rect()
                                .horizontal()
                                .max_width(Size::px(560.))
                                .width(Size::fill())
                                .content(Content::flex())
                                .cross_align(Alignment::center())
                                .spacing(12.)
                                .padding((14., 16.))
                                .corner_radius(10.)
                                .background(background)
                                .color(color)
                                .shadow(Shadow::new().y(4.).blur(12.).color((0, 0, 0, 60)))
                                .on_press(move |_| {
                                    let mut toast = overlay.toast;
                                    toast.set(None);
                                })
                                .map(t.icon, |el, i| el.child(icon(i, 20., color)))
                                .child(label().width(Size::flex(1.)).text(t.message)),
                        ),
                )
            })
    }
}

#[derive(PartialEq)]
struct DialogView {
    dialog: Dialog,
    overlay: Overlay,
}

impl Component for DialogView {
    fn render(&self) -> impl IntoElement {
        let p = use_palette();
        let insets = crate::platform::use_insets();
        let typed = use_state(String::new);
        let overlay = self.overlay;
        let id = self.dialog.id;
        let sheet = matches!(self.dialog.kind, Kind::Choose { .. });
        let danger_icon = matches!(
            &self.dialog.kind,
            Kind::Typing { .. } | Kind::Confirm { danger: true, .. }
        );

        let body: Element = match &self.dialog.kind {
            Kind::Choose { choices } => rect()
                .width(Size::fill())
                .children(choices.iter().enumerate().map(|(i, c)| {
                    let color = if c.danger { p.error } else { p.text };
                    ListRow::new(label().color(color).text(c.label.clone()))
                        .icon(c.icon.clone(), color)
                        .on_press(move |_| overlay.answer(id, Some(i)))
                        .key(i)
                        .into_element()
                }))
                .into(),
            Kind::Typing { name, .. } => rect()
                .width(Size::fill())
                .spacing(12.)
                .child(text(self.dialog.body.clone()))
                .child(text(format!("Type “{name}” to confirm.")))
                .child(Input::new(typed).width(Size::fill()).auto_focus(true))
                .into(),
            _ => text(self.dialog.body.clone()).into(),
        };

        let buttons: Option<Element> = match &self.dialog.kind {
            Kind::Message => Some(dialog_buttons().child(Btn::new("OK").text().on_press(move |_| overlay.answer(id, None))).into()),
            Kind::Confirm { action, cancel, danger } => Some(
                dialog_buttons()
                    .child(Btn::new(cancel.clone()).text().on_press(move |_| overlay.answer(id, None)))
                    .child(
                        Btn::new(action.clone())
                            .kind(if *danger { BtnKind::Danger } else { BtnKind::Filled })
                            .on_press(move |_| overlay.answer(id, Some(0))),
                    )
                    .into(),
            ),
            Kind::Typing { name, action } => {
                let ok = typed.read().trim() == name.trim();
                Some(
                    dialog_buttons()
                        .child(Btn::new("Cancel").text().on_press(move |_| overlay.answer(id, None)))
                        .child(
                            Btn::new(action.clone())
                                .danger()
                                .enabled(ok)
                                .on_press(move |_| overlay.answer(id, Some(0))),
                        )
                        .into(),
                )
            }
            Kind::Choose { .. } => None,
        };

        rect()
            .expanded()
            .background((0, 0, 0, 140))
            .main_align(if sheet { Alignment::end() } else { Alignment::center() })
            .cross_align(Alignment::center())
            .padding(if sheet { 0. } else { 20. })
            .on_press(move |_| overlay.answer(id, None))
            .child(
                rect()
                    .width(Size::fill())
                    .max_width(Size::px(if sheet { 640. } else { 520. }))
                    .max_height(Size::percent(90.))
                    .background(p.surface)
                    .color(p.text)
                    .corner_radius(if sheet { CornerRadius::new_symmetric(20., 0.) } else { CornerRadius::new_all(20.) })
                    .padding((20., 20., 12., 20.))
                    .spacing(12.)
                    // presses inside the dialog don't close it
                    .on_press(|e: Event<PressEventData>| e.stop_propagation())
                    .maybe(danger_icon, |el| {
                        el.child(rect().width(Size::fill()).center().child(icon(lucide::triangle_alert(), 28., p.error)))
                    })
                    .child(heading(self.dialog.title.clone()))
                    .maybe(sheet && !self.dialog.body.is_empty(), |el| {
                        el.child(small(self.dialog.body.clone(), p.text_muted))
                    })
                    .child(ScrollView::new().width(Size::fill()).height(Size::auto()).max_height(Size::px(480.)).child(body))
                    .map(buttons, |el, b| el.child(b))
                    .child(gap(if sheet { insets.bottom } else { 0. })),
            )
    }
}

fn dialog_buttons() -> Rect {
    rect()
        .horizontal()
        .width(Size::fill())
        .main_align(Alignment::end())
        .spacing(8.)
}
