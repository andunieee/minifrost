//! The Android key store, through `ShareKeystore.java`. Sealing and opening ask for the screen
//! lock when it wasn't confirmed in the last 30 seconds. Not available on the desktop.

pub const SUPPORTED: bool = cfg!(target_os = "android");

#[cfg(target_os = "android")]
async fn call(op: &str, bytes: &[u8]) -> Result<Vec<u8>, String> {
    use minifrost_ui::platform::android::request;
    request(op, bytes).await.map_err(|e| {
        let (code, message) = e.split_once('\n').unwrap_or(("", e.as_str()));
        match code {
            "key_gone" => "The device key store no longer has the key this share was stored with. \
                Android deletes it when the screen lock is removed. Restore this share from its backup words."
                .to_string(),
            _ => message.to_string(),
        }
    })
}

#[cfg(not(target_os = "android"))]
async fn call(_op: &str, _bytes: &[u8]) -> Result<Vec<u8>, String> {
    Err("The device key store is only available on Android.".into())
}

pub async fn seal(share: &[u8]) -> Result<Vec<u8>, String> {
    call("keystore_seal", share).await
}

pub async fn open(sealed: &[u8]) -> Result<Vec<u8>, String> {
    call("keystore_open", sealed).await
}

/// Whether the phone has a screen lock, which the key store needs.
pub async fn screen_lock_set() -> bool {
    SUPPORTED && matches!(call("keystore_screen_lock", &[]).await.as_deref(), Ok([1]))
}
