//! The Minifrost coordinator app (Freya): the desktop window and the Android activity's native side.

mod nav;
mod pages;

use std::sync::Arc;

use minifrost_coordinator::api::open_coordinator;
use minifrost_ui::{
    platform::{
        self,
        DataDir,
    },
    prelude::*,
};

use crate::{
    nav::{
        App,
        Nav,
        Page,
        Tab,
    },
    pages::*,
};

/// The whole app, over the data folder (provided as root context).
pub fn app() -> impl IntoElement {
    let dir = use_consume::<DataDir>();
    platform::use_init_platform();
    let overlay = use_init_overlay();
    let p = use_init_palette(|_| INDIGO);
    let insets = use_insets();
    let opened = use_hook(|| open_coordinator(dir.0.to_string_lossy().into_owned()).map(|c| App(Arc::new(c))));

    let body: Element = match opened {
        Ok(app) => Shell { app, overlay }.into(),
        Err(e) => rect()
            .expanded()
            .center()
            .padding(24.)
            .child(text(format!("Could not open the coordinator in {}: {e}", dir.0.display())).text_align(TextAlign::Center))
            .into(),
    };

    rect()
        .expanded()
        .background(p.bg)
        .color(p.text)
        .padding((insets.top, insets.right, insets.bottom, insets.left))
        .child(body)
        .child(OverlayHost)
}

#[derive(PartialEq)]
struct Shell {
    app: App,
    overlay: Overlay,
}

impl Component for Shell {
    fn render(&self) -> impl IntoElement {
        let p = use_palette();
        let app = use_provide_context(|| self.app.clone());
        let nav = use_provide_context(|| Nav::new(self.overlay));
        let width = Platform::get().current_window().root_size.read().width;
        let wide = width >= 700.;
        let on_key = {
            let app = app.clone();
            move |e: Event<KeyboardEventData>| {
                if platform::is_back(&e.key) && !nav.back(&app) {
                    platform::leave();
                }
            }
        };

        let content: Element = match nav.top() {
            Some(page) => match page {
                Page::WalletDetail { key_id } => WalletDetailPage { key_id }.into(),
                Page::Ceremony { title, update, key_id } => CeremonyPage { title, update, key_id }.into(),
                Page::NewWallet => NewWalletPage.into(),
                Page::Enroll => EnrollPage.into(),
                Page::NameWallet(naming) => NameWalletPage { naming }.into(),
                Page::ConfirmSend { key_id, preview } => ConfirmSendPage { key_id, preview }.into(),
                Page::VerifyAddress { wallet, label, address, code } => {
                    VerifyAddressPage { wallet, label, address, code }.into()
                }
            },
            None => {
                let tab: Element = match *nav.tab.read() {
                    Tab::Wallets => WalletsPage.into(),
                    Tab::Restore => RestorePage.into(),
                    Tab::Signers => SignersPage.into(),
                    Tab::Settings => SettingsPage.into(),
                };
                let tabs = [
                    (Tab::Wallets, "Wallets", lucide::wallet()),
                    (Tab::Restore, "Restore", lucide::history()),
                    (Tab::Signers, "Signers", lucide::shield()),
                    (Tab::Settings, "Settings", lucide::settings()),
                ]
                .into_iter()
                .map(|(tab, name, i)| TabButton { tab, name, icon: i, wide }.into_element());
                if wide {
                    rect()
                        .horizontal()
                        .expanded()
                        .content(Content::flex())
                        .child(rect().width(Size::px(88.)).height(Size::fill()).padding((12., 0.)).spacing(8.).children(tabs))
                        .child(rect().width(Size::px(1.)).height(Size::fill()).background(p.border))
                        .child(rect().width(Size::flex(1.)).height(Size::fill()).child(tab))
                        .into()
                } else {
                    rect()
                        .expanded()
                        .content(Content::flex())
                        .child(rect().width(Size::fill()).height(Size::flex(1.)).child(tab))
                        .child(rect().width(Size::fill()).height(Size::px(1.)).background(p.border))
                        .child(rect().horizontal().width(Size::fill()).content(Content::flex()).padding((6., 0.)).children(tabs))
                        .into()
                }
            }
        };

        rect().expanded().on_global_key_down(on_key).child(content)
    }
}

#[derive(PartialEq)]
struct TabButton {
    tab: Tab,
    name: &'static str,
    icon: Bytes,
    wide: bool,
}

impl Component for TabButton {
    fn render(&self) -> impl IntoElement {
        let p = use_palette();
        let nav = use_consume::<Nav>();
        let active = *nav.tab.read() == self.tab;
        let tab = self.tab;
        let color = if active { p.on_accent_soft } else { p.text_muted };
        rect()
            .maybe(!self.wide, |el| el.width(Size::flex(1.)))
            .maybe(self.wide, |el| el.width(Size::fill()))
            .cross_align(Alignment::center())
            .spacing(4.)
            .a11y_role(AccessibilityRole::Tab)
            .a11y_focusable(true)
            .on_press(move |_| nav.set_tab(tab))
            .child(
                rect()
                    .padding((4., 18.))
                    .corner_radius(16.)
                    .background(if active { p.accent_soft } else { Color::TRANSPARENT })
                    .child(icon(self.icon.clone(), 22., color)),
            )
            .child(
                label()
                    .font_size(12.)
                    .font_weight(if active { FontWeight::BOLD } else { FontWeight::NORMAL })
                    .color(if active { p.text } else { p.text_muted })
                    .text(self.name),
            )
    }
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(droid_app: winit::platform::android::activity::AndroidApp) {
    use freya_winit::renderer::NativeEvent;
    use minifrost_ui::freya::android::AndroidPlugin;
    use winit::{
        event_loop::EventLoop,
        platform::android::EventLoopBuilderExtAndroid,
    };

    android_logger::init_once(android_logger::Config::default().with_max_level(log::LevelFilter::Info));
    platform::android::init(droid_app.clone());
    let dir = DataDir(platform::android::files_dir());
    let event_loop = EventLoop::<NativeEvent>::with_user_event()
        .with_android_app(droid_app.clone())
        .build()
        .expect("the event loop");
    launch(
        LaunchConfig::new()
            .with_plugin(AndroidPlugin::new(droid_app))
            .with_window(WindowConfig::new(app).with_root_context(dir))
            .with_event_loop(event_loop),
    )
}
