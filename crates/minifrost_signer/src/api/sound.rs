//! Data over sound (ggwave), the alternative to showing and scanning QR codes: one device plays
//! the frames of a message on a loop, the other listens and gets the same `MF1:` frame texts a
//! camera would have produced, to feed into a `QrScanner`.
//!
//! NOTE: the pub functions must stay at module top level — flutter_rust_bridge bakes their
//! full path into the generated bindings.

use std::sync::Mutex;

use crate::frb_generated::StreamSink;

static PLAYER: Mutex<Option<sound_transport::Player>> = Mutex::new(None);
static LISTENER: Mutex<Option<sound_transport::Listener>> = Mutex::new(None);

/// Play the frames of one message (what an animated QR would show) on a loop through the
/// speaker. Replaces any message already playing.
pub fn start_sound_playback(frames: Vec<String>) -> Result<(), String> {
    stop_sound_playback();
    let player = sound_transport::Player::start(&frames)?;
    *PLAYER.lock().unwrap() = Some(player);
    Ok(())
}

pub fn stop_sound_playback() {
    // dropped outside the lock: stopping joins the audio thread
    let old = PLAYER.lock().unwrap().take();
    drop(old);
}

/// Listen on the microphone; every heard frame arrives as `MF1:…` text. Replaces any previous
/// listener.
pub fn start_sound_listening(frames: StreamSink<String>) -> Result<(), String> {
    stop_sound_listening();
    let listener = sound_transport::Listener::start(move |text| {
        let _ = frames.add(text);
    })?;
    *LISTENER.lock().unwrap() = Some(listener);
    Ok(())
}

pub fn stop_sound_listening() {
    let old = LISTENER.lock().unwrap().take();
    drop(old);
}
