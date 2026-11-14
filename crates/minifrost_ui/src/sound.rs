//! Data over sound (ggwave), the alternative to showing and scanning QR codes: one device plays
//! the frames of a message on a loop, the other listens and gets the same `MF1:` frame texts a
//! camera would have produced.

use std::sync::Mutex;

use futures_channel::mpsc;

static PLAYER: Mutex<Option<sound_transport::Player>> = Mutex::new(None);
static LISTENER: Mutex<Option<sound_transport::Listener>> = Mutex::new(None);

/// Plays the frames of one message on a loop through the speaker, replacing any message
/// already playing.
pub fn play(frames: &[String]) -> Result<(), String> {
    stop_playing();
    let player = sound_transport::Player::start(frames)?;
    *PLAYER.lock().unwrap() = Some(player);
    Ok(())
}

pub fn stop_playing() {
    // dropped outside the lock: stopping joins the audio thread
    let old = PLAYER.lock().unwrap().take();
    drop(old);
}

/// Listens on the microphone (asking for it first on Android); every frame heard arrives as
/// `MF1:…` text. Replaces any previous listener.
pub async fn listen() -> Result<mpsc::UnboundedReceiver<String>, String> {
    if !crate::platform::ensure_permission(crate::platform::Permission::Microphone).await {
        return Err("Microphone permission is needed to listen.".into());
    }
    stop_listening();
    let (tx, rx) = mpsc::unbounded();
    let listener = sound_transport::Listener::start(move |text| {
        let _ = tx.unbounded_send(text);
    })?;
    *LISTENER.lock().unwrap() = Some(listener);
    Ok(rx)
}

pub fn stop_listening() {
    let old = LISTENER.lock().unwrap().take();
    drop(old);
}
