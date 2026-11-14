//! What differs between the desktop and Android: where data lives, the system bars, permissions,
//! the Back key.

use std::path::PathBuf;

use freya::prelude::*;

#[cfg(target_os = "android")]
pub mod android;

/// Room taken by the system bars and the keyboard, in logical pixels. Zero on the desktop.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Insets {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

/// Sets up what the platform reports (the insets, on Android). Call once, at the root.
pub fn use_init_platform() {
    let insets = use_provide_context(|| State::create(Insets::default()));
    #[cfg(target_os = "android")]
    use_hook(move || android::watch_insets(insets));
    #[cfg(not(target_os = "android"))]
    let _ = insets;
}

pub fn use_insets() -> Insets {
    use_try_consume::<State<Insets>>().map(|s| *s.read()).unwrap_or_default()
}

/// The folder a desktop app keeps its data in, where the Flutter builds kept it: the data
/// directory under the application id (Linux), or under the bundle id, inside the app sandbox
/// container if an earlier build left its data there (macOS).
#[cfg(not(target_os = "android"))]
pub fn desktop_data_dir(app_id: &str, mac_bundle_id: &str) -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    if cfg!(target_os = "macos") {
        let plain = base.join(mac_bundle_id);
        let sandboxed = dirs::home_dir().map(|home| {
            home.join("Library/Containers")
                .join(mac_bundle_id)
                .join("Data/Library/Application Support")
                .join(mac_bundle_id)
        });
        return match sandboxed {
            Some(old) if !plain.exists() && old.exists() => old,
            _ => plain,
        };
    }
    base.join(app_id)
}

/// Where the app keeps its data. Provided at the root by each app's `main` / `android_main`.
#[derive(Clone, PartialEq, Debug)]
pub struct DataDir(pub PathBuf);

/// What a runtime permission is for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Permission {
    Camera,
    Microphone,
}

/// Asks for a runtime permission where the platform has them (Android); true when granted.
pub async fn ensure_permission(permission: Permission) -> bool {
    #[cfg(target_os = "android")]
    {
        let name = match permission {
            Permission::Camera => "android.permission.CAMERA",
            Permission::Microphone => "android.permission.RECORD_AUDIO",
        };
        matches!(android::request("permission", name.as_bytes()).await.as_deref(), Ok([1]))
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = permission;
        true
    }
}

/// Whether a key is the system's Back (the Android back gesture or button).
pub fn is_back(key: &Key) -> bool {
    *key == Key::Named(NamedKey::BrowserBack)
}

/// What Back does when there is nowhere left to go back to: on Android, leave the app running
/// behind the others, like the system does.
pub fn leave() {
    #[cfg(target_os = "android")]
    android::send("move_to_back", &[]);
}
