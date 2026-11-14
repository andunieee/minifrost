//! Small building blocks every screen uses: text styles, buttons, cards, the app bar, list rows.

use freya::{
    icons::lucide,
    prelude::*,
};

use crate::theme::{
    Palette,
    mix,
    signer_color,
    use_palette,
};

pub const MONO: &str = "monospace";

pub fn icon(bytes: Bytes, size: f32, color: Color) -> SvgViewer {
    SvgViewer::new(bytes)
        .stroke(color)
        .width(Size::px(size))
        .height(Size::px(size))
}

/// Body text that wraps to the width it is given.
pub fn text(s: impl Into<String>) -> Label {
    label().width(Size::fill()).text(s.into())
}

pub fn heading(s: impl Into<String>) -> Label {
    label()
        .width(Size::fill())
        .font_size(20.)
        .font_weight(FontWeight::BOLD)
        .text(s.into())
}

pub fn title(s: impl Into<String>) -> Label {
    label()
        .width(Size::fill())
        .font_size(16.)
        .font_weight(FontWeight::SEMI_BOLD)
        .text(s.into())
}

pub fn small(s: impl Into<String>, color: Color) -> Label {
    label().width(Size::fill()).font_size(12.).color(color).text(s.into())
}

pub fn mono(s: impl Into<String>, size: f32) -> Label {
    label().font_family(MONO).font_size(size).text(s.into())
}

pub fn gap(px: f32) -> Rect {
    rect().width(Size::fill()).height(Size::px(px))
}

pub fn divider(p: &Palette) -> Rect {
    rect().width(Size::fill()).height(Size::px(1.)).background(p.border)
}

/// A rounded surface holding a group of content.
pub fn card(p: &Palette) -> Rect {
    rect()
        .width(Size::fill())
        .background(p.surface)
        .corner_radius(12.)
        .padding(16.)
        .spacing(8.)
}

/// A card tinted with a colour and its readable text colour.
pub fn tinted_card(background: Color, color: Color) -> Rect {
    rect()
        .width(Size::fill())
        .background(background)
        .color(color)
        .corner_radius(12.)
        .padding(16.)
        .spacing(8.)
}

/// An icon and wrapped text side by side, for warnings and notes.
pub fn note(icon_bytes: Bytes, color: Color, s: impl Into<String>) -> Rect {
    rect()
        .horizontal()
        .width(Size::fill())
        .spacing(8.)
        .child(icon(icon_bytes, 18., color))
        .child(label().width(Size::flex(1.)).color(color).text(s.into()))
        .content(Content::flex())
}

/// The scrolling body of a screen: a centered column no wider than `max_width`.
pub fn page(max_width: f32) -> PageBuilder {
    PageBuilder {
        max_width,
        children: vec![],
        padding: 16.,
    }
}

pub struct PageBuilder {
    max_width: f32,
    padding: f32,
    children: Vec<Element>,
}

impl PageBuilder {
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }
}

impl MaybeExt for PageBuilder {}

impl ChildrenExt for PageBuilder {
    fn children_mut(&mut self) -> &mut Vec<Element> {
        &mut self.children
    }
}

impl IntoElement for PageBuilder {
    fn into_element(self) -> Element {
        ScrollView::new()
            .width(Size::fill())
            .height(Size::flex(1.))
            .child(
                rect().width(Size::fill()).cross_align(Alignment::center()).child(
                    rect()
                        .width(Size::fill())
                        .max_width(Size::px(self.max_width))
                        .padding(self.padding)
                        .spacing(12.)
                        .children(self.children),
                ),
            )
            .into_element()
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BtnKind {
    Filled,
    Tonal,
    Outline,
    Text,
    Danger,
}

/// A button. Filled for the main action of a screen, tonal and outline for the others, text for
/// small ones, danger for what can't be undone.
#[derive(PartialEq)]
pub struct Btn {
    label: String,
    kind: BtnKind,
    icon: Option<Bytes>,
    enabled: bool,
    busy: bool,
    color: Option<Color>,
    fill: bool,
    on_press: Option<EventHandler<()>>,
    key: DiffKey,
}

impl Btn {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            kind: BtnKind::Filled,
            icon: None,
            enabled: true,
            busy: false,
            color: None,
            fill: false,
            on_press: None,
            key: DiffKey::None,
        }
    }
    pub fn kind(mut self, kind: BtnKind) -> Self {
        self.kind = kind;
        self
    }
    pub fn tonal(self) -> Self {
        self.kind(BtnKind::Tonal)
    }
    pub fn outline(self) -> Self {
        self.kind(BtnKind::Outline)
    }
    pub fn text(self) -> Self {
        self.kind(BtnKind::Text)
    }
    pub fn danger(self) -> Self {
        self.kind(BtnKind::Danger)
    }
    pub fn icon(mut self, icon: Bytes) -> Self {
        self.icon = Some(icon);
        self
    }
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    /// Shows a spinner in place of the icon and ignores presses.
    pub fn busy(mut self, busy: bool) -> Self {
        self.busy = busy;
        self
    }
    /// Draws the button in this colour instead of the accent.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
    /// As wide as its parent.
    pub fn fill(mut self) -> Self {
        self.fill = true;
        self
    }
    pub fn on_press(mut self, on_press: impl Into<EventHandler<()>>) -> Self {
        self.on_press = Some(on_press.into());
        self
    }
}

impl KeyExt for Btn {
    fn key_mut(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl Component for Btn {
    fn render(&self) -> impl IntoElement {
        let p = use_palette();
        let mut hovered = use_state(|| false);
        let accent = self.color.unwrap_or(p.accent);
        let (background, color, border) = match self.kind {
            BtnKind::Filled => (accent, crate::theme::readable_on(accent), None),
            BtnKind::Tonal => {
                let soft = mix(accent, p.bg, if p.dark { 0.72 } else { 0.84 });
                (soft, crate::theme::readable_on(soft), None)
            }
            BtnKind::Outline => (Color::TRANSPARENT, accent, Some(accent)),
            BtnKind::Text => (Color::TRANSPARENT, accent, None),
            BtnKind::Danger => (p.error, p.on_error, None),
        };
        let active = self.enabled && !self.busy;
        let background = if active && hovered() {
            if background == Color::TRANSPARENT {
                p.hover
            } else {
                mix(background, color, 0.1)
            }
        } else {
            background
        };
        let on_press = self.on_press.clone();
        let compact = self.kind == BtnKind::Text;

        rect()
            .horizontal()
            .cross_align(Alignment::center())
            .main_align(Alignment::center())
            .spacing(8.)
            .min_height(Size::px(if compact { 36. } else { 42. }))
            .padding(if compact { (8., 12.) } else { (10., 20.) })
            .corner_radius(21.)
            .background(background)
            .color(color)
            .map(border, |el, b| el.border(Border::new().fill(b).width(1.5)))
            .maybe(self.fill, |el| el.width(Size::fill()))
            .maybe(!self.enabled, |el| el.opacity(0.45))
            .a11y_role(AccessibilityRole::Button)
            .a11y_focusable(active)
            .on_pointer_enter(move |_| hovered.set(true))
            .on_pointer_leave(move |_| hovered.set(false))
            .on_press(move |_| {
                if active && let Some(h) = &on_press {
                    h.call(());
                }
            })
            .maybe(self.busy, |el| el.child(CircularLoader::new().size(18.)))
            .map(self.icon.clone().filter(|_| !self.busy), |el, i| {
                el.child(icon(i, 18., color))
            })
            .child(
                label()
                    .font_weight(FontWeight::SEMI_BOLD)
                    .max_lines(1)
                    .text(self.label.clone()),
            )
    }

    fn render_key(&self) -> DiffKey {
        self.key.clone().or(self.default_key())
    }
}

/// A round button that is only an icon.
#[derive(PartialEq)]
pub struct IconBtn {
    icon: Bytes,
    color: Option<Color>,
    on_press: EventHandler<()>,
    key: DiffKey,
}

impl IconBtn {
    pub fn new(icon: Bytes, on_press: impl Into<EventHandler<()>>) -> Self {
        Self {
            icon,
            color: None,
            on_press: on_press.into(),
            key: DiffKey::None,
        }
    }
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

impl KeyExt for IconBtn {
    fn key_mut(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl Component for IconBtn {
    fn render(&self) -> impl IntoElement {
        let p = use_palette();
        let mut hovered = use_state(|| false);
        let on_press = self.on_press.clone();
        rect()
            .padding(9.)
            .corner_radius(20.)
            .background(if hovered() { p.hover } else { Color::TRANSPARENT })
            .a11y_role(AccessibilityRole::Button)
            .a11y_focusable(true)
            .on_pointer_enter(move |_| hovered.set(true))
            .on_pointer_leave(move |_| hovered.set(false))
            .on_press(move |_| on_press.call(()))
            .child(icon(self.icon.clone(), 22., self.color.unwrap_or(p.text)))
    }

    fn render_key(&self) -> DiffKey {
        self.key.clone().or(self.default_key())
    }
}

/// The bar at the top of a screen: an optional back (or close) button, the title, and actions.
pub fn app_bar(p: &Palette, title: impl Into<String>, back: Option<(Bytes, EventHandler<()>)>) -> Rect {
    rect()
        .horizontal()
        .width(Size::fill())
        .height(Size::px(56.))
        .padding((0., 8.))
        .spacing(4.)
        .cross_align(Alignment::center())
        .content(Content::flex())
        .background(p.bg)
        .map(back, |el, (i, h)| el.child(IconBtn::new(i, h)))
        .child(
            label()
                .width(Size::flex(1.))
                .margin((0., 8.))
                .font_size(20.)
                .font_weight(FontWeight::SEMI_BOLD)
                .max_lines(1)
                .text_overflow(TextOverflow::Ellipsis)
                .text(title.into()),
        )
}

/// A back arrow for [`app_bar`].
pub fn back(on_back: impl Into<EventHandler<()>>) -> Option<(Bytes, EventHandler<()>)> {
    Some((lucide::arrow_left(), on_back.into()))
}

/// A close cross for [`app_bar`].
pub fn close(on_close: impl Into<EventHandler<()>>) -> Option<(Bytes, EventHandler<()>)> {
    Some((lucide::x(), on_close.into()))
}

/// A row in a list: leading icon, title and subtitle, then whatever trails it.
#[derive(PartialEq)]
pub struct ListRow {
    icon: Option<(Bytes, Color)>,
    title: Element,
    subtitle: Option<String>,
    trailing: Vec<Element>,
    on_press: Option<EventHandler<()>>,
    border: Option<Color>,
    key: DiffKey,
}

impl ListRow {
    pub fn new(title: impl IntoElement) -> Self {
        Self {
            icon: None,
            title: title.into_element(),
            subtitle: None,
            trailing: vec![],
            on_press: None,
            border: None,
            key: DiffKey::None,
        }
    }
    pub fn icon(mut self, icon: Bytes, color: Color) -> Self {
        self.icon = Some((icon, color));
        self
    }
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }
    pub fn trailing(mut self, trailing: impl IntoElement) -> Self {
        self.trailing.push(trailing.into_element());
        self
    }
    pub fn on_press(mut self, on_press: impl Into<EventHandler<()>>) -> Self {
        self.on_press = Some(on_press.into());
        self
    }
    pub fn border(mut self, color: Color) -> Self {
        self.border = Some(color);
        self
    }
}

impl KeyExt for ListRow {
    fn key_mut(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl Component for ListRow {
    fn render(&self) -> impl IntoElement {
        let p = use_palette();
        let mut hovered = use_state(|| false);
        let on_press = self.on_press.clone();
        let pressable = on_press.is_some();
        rect()
            .horizontal()
            .width(Size::fill())
            .content(Content::flex())
            .cross_align(Alignment::center())
            .spacing(14.)
            .padding((12., 16.))
            .corner_radius(12.)
            .background(if pressable && hovered() { mix(p.surface, p.text, 0.05) } else { p.surface })
            .map(self.border, |el, b| el.border(Border::new().fill(b).width(2.)))
            .maybe(pressable, |el| {
                el.a11y_role(AccessibilityRole::Button)
                    .a11y_focusable(true)
                    .on_pointer_enter(move |_| hovered.set(true))
                    .on_pointer_leave(move |_| hovered.set(false))
                    .on_press(move |_| {
                        if let Some(h) = &on_press {
                            h.call(());
                        }
                    })
            })
            .map(self.icon.clone(), |el, (i, c)| el.child(icon(i, 24., c)))
            .child(
                rect()
                    .width(Size::flex(1.))
                    .spacing(2.)
                    .child(self.title.clone())
                    .map(self.subtitle.clone(), |el, s| el.child(small(s, p.text_muted).font_size(13.))),
            )
            .children(self.trailing.clone())
    }

    fn render_key(&self) -> DiffKey {
        self.key.clone().or(self.default_key())
    }
}

/// A small rounded label.
pub fn chip(p: &Palette, s: impl Into<String>) -> Rect {
    rect()
        .padding((5., 10.))
        .corner_radius(8.)
        .background(p.surface_alt)
        .child(label().font_size(13.).text(s.into()))
}

/// A pill naming the kind of message a QR code carries or is expected to carry.
pub fn kind_label(p: &Palette, icon_bytes: Bytes, s: impl Into<String>) -> Rect {
    rect().width(Size::fill()).cross_align(Alignment::center()).child(
        rect()
            .horizontal()
            .cross_align(Alignment::center())
            .spacing(6.)
            .padding((6., 12.))
            .corner_radius(16.)
            .background(p.accent_soft)
            .color(p.on_accent_soft)
            .child(icon(icon_bytes, 18., p.on_accent_soft))
            .child(label().font_weight(FontWeight::SEMI_BOLD).text(s.into())),
    )
}

/// A signer's name drawn in its own colour.
pub fn signer_name(p: &Palette, device_id: &str, name: impl Into<String>) -> Label {
    label()
        .color(signer_color(device_id, p.dark))
        .font_weight(FontWeight::SEMI_BOLD)
        .text(name.into())
}

/// A coloured pill for a signer, filled when it has replied and outlined while it is awaited.
pub fn signer_chip(p: &Palette, device_id: &str, name: impl Into<String>, done: bool) -> Rect {
    let color = signer_color(device_id, p.dark);
    rect()
        .horizontal()
        .cross_align(Alignment::center())
        .spacing(6.)
        .padding((5., 10.))
        .corner_radius(8.)
        .border(Border::new().fill(color).width(if done { 1. } else { 1.5 }))
        .maybe(done, |el| el.background(color.with_a(46)))
        .child(icon(
            if done { lucide::circle_check() } else { lucide::hourglass() },
            16.,
            color,
        ))
        .child(
            label()
                .color(color)
                .font_weight(FontWeight::SEMI_BOLD)
                .text(name.into()),
        )
}

/// A thin bar filled to `value` (0 to 1).
pub fn progress_line(p: &Palette, value: f32) -> Rect {
    rect()
        .width(Size::fill())
        .height(Size::px(4.))
        .corner_radius(2.)
        .background(p.surface_alt)
        .child(
            rect()
                .width(Size::percent(value.clamp(0., 1.) * 100.))
                .height(Size::fill())
                .corner_radius(2.)
                .background(p.accent),
        )
}

/// A checkbox with its label, all of it pressable.
pub fn check_row(p: &Palette, checked: bool, title: impl IntoElement, on_toggle: impl Into<EventHandler<()>>) -> Rect {
    let on_toggle: EventHandler<()> = on_toggle.into();
    rect()
        .horizontal()
        .width(Size::fill())
        .content(Content::flex())
        .cross_align(Alignment::center())
        .spacing(12.)
        .padding((8., 4.))
        .corner_radius(8.)
        .a11y_role(AccessibilityRole::CheckBox)
        .a11y_focusable(true)
        .on_press(move |_| on_toggle.call(()))
        .child(Checkbox::new().selected(checked))
        .child(rect().width(Size::flex(1.)).color(p.text).child(title))
}

/// One option of a group of which exactly one is chosen.
pub fn radio_row(p: &Palette, selected: bool, title: impl Into<String>, on_select: impl Into<EventHandler<()>>) -> Rect {
    let on_select: EventHandler<()> = on_select.into();
    rect()
        .horizontal()
        .width(Size::fill())
        .content(Content::flex())
        .cross_align(Alignment::center())
        .spacing(12.)
        .padding((8., 4.))
        .corner_radius(8.)
        .a11y_role(AccessibilityRole::RadioButton)
        .a11y_focusable(true)
        .on_press(move |_| on_select.call(()))
        .child(icon(
            if selected { lucide::circle_dot() } else { lucide::circle() },
            20.,
            if selected { p.accent } else { p.text_muted },
        ))
        .child(label().width(Size::flex(1.)).text(title.into()))
}

/// A labelled text field.
pub fn field(p: &Palette, name: impl Into<String>, input: impl IntoElement) -> Rect {
    rect()
        .width(Size::fill())
        .spacing(4.)
        .child(small(name, p.text_muted))
        .child(input)
}

/// Buttons in a row that wraps on narrow screens.
pub fn button_row() -> Rect {
    rect()
        .horizontal()
        .width(Size::fill())
        .content(Content::wrap_spacing(8.))
        .spacing(8.)
        .cross_align(Alignment::center())
}

/// A button floating over the bottom right corner of its parent, for a screen's main action.
pub fn floating(button: impl IntoElement) -> Rect {
    rect()
        .position(Position::new_absolute().right(20.).bottom(20.))
        .layer(Layer::Relative(1))
        .child(button)
}

/// A centered spinner filling its parent.
pub fn loading() -> Rect {
    rect().expanded().center().child(CircularLoader::new())
}
