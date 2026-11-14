//! The Minifrost signer app (Freya): the desktop window and the Android activity's native side.

mod ctl;
mod keystore;
mod nav;
mod screens;

use minifrost_signer::api::load_signer;
use minifrost_ui::{
    platform::{
        self,
        DataDir,
    },
    prelude::*,
};

use crate::{
    ctl::{
        Ctl,
        Vault,
    },
    nav::{
        Nav,
        Screen,
    },
    screens::*,
};

/// What the app found on start: a signer, nothing yet (first run), or a vault it can't open.
#[derive(Clone, PartialEq)]
enum Boot {
    Ready(Ctl),
    Setup(Vault),
    Failed(String),
}

fn boot(dir: &DataDir) -> Boot {
    let vault = Vault::new(&dir.0);
    if !vault.exists() {
        return Boot::Setup(vault);
    }
    match vault.load().and_then(load_signer) {
        Ok(signer) => Boot::Ready(Ctl::new(signer, vault)),
        Err(e) => Boot::Failed(e),
    }
}

/// The whole app, over the data folder (provided as root context).
pub fn app() -> impl IntoElement {
    let dir = use_consume::<DataDir>();
    platform::use_init_platform();
    let overlay = use_init_overlay();
    let mut state = use_state(|| boot(&dir));
    let ready = match &*state.read() {
        Boot::Ready(ctl) => Some(ctl.clone()),
        _ => None,
    };
    // once there is a signer, the whole app wears its colour
    let p = use_init_palette(|dark| match &ready {
        Some(ctl) => signer_color(&ctl.device_id, dark),
        None => INDIGO,
    });
    let insets = use_insets();

    let body: Element = match &*state.read() {
        Boot::Ready(ctl) => SignerRoot { ctl: ctl.clone(), overlay }.into(),
        Boot::Setup(vault) => Setup {
            vault: vault.clone(),
            on_created: EventHandler::new(move |ctl| state.set(Boot::Ready(ctl))),
        }
        .into(),
        Boot::Failed(e) => rect()
            .expanded()
            .center()
            .padding(24.)
            .spacing(16.)
            .child(icon(lucide::lock(), 48., p.text))
            .child(text(e.clone()).text_align(TextAlign::Center))
            .child(Btn::new("Try again").on_press(move |_| state.set(boot(&dir))))
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

/// The app once there is a signer: the screen on top of the stack.
#[derive(PartialEq)]
struct SignerRoot {
    ctl: Ctl,
    overlay: Overlay,
}

impl Component for SignerRoot {
    fn render(&self) -> impl IntoElement {
        let ctl = use_provide_context(|| self.ctl.clone());
        let nav = use_provide_context(|| Nav::new(self.overlay));
        let on_key = move |e: Event<KeyboardEventData>| {
            if platform::is_back(&e.key) && !nav.back(&ctl) {
                platform::leave();
            }
        };
        rect()
            .expanded()
            .on_global_key_down(on_key)
            .child(match nav.top() {
                Screen::Home => Home.into_element(),
                Screen::Scan { expecting } => ScanScreen { expecting }.into_element(),
                Screen::Reply {
                    frames,
                    title,
                    what,
                    next,
                } => ReplyScreen {
                    frames,
                    title,
                    what,
                    next,
                }
                .into_element(),
                Screen::Prompt(prompt) => PromptScreen { prompt }.into_element(),
                Screen::Quiz { key_name } => QuizScreen { key_name }.into_element(),
                Screen::EnterBackup => EnterBackupScreen.into_element(),
            })
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
