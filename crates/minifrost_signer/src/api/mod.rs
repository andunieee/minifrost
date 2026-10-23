//! The API Flutter sees. Everything here is generated into Dart by flutter_rust_bridge.

use std::sync::Mutex;

use crate::engine::Signer;
pub use crate::engine::{
    KeyInfo, OutputSummary, PendingBackup, Prompt, QuizAnswer, QuizQuestion, Step, StepResult, TxSummary,
};

pub mod camera;
pub mod sound;

/// The signer. One per app process.
pub struct SignerApp {
    inner: Mutex<Signer>,
}

/// Create a brand-new signer (first launch). The caller must store `snapshot()` right away.
pub fn new_signer(name: String) -> SignerApp {
    SignerApp {
        inner: Mutex::new(Signer::create(name)),
    }
}

/// Restore the signer from the bytes the caller stored in secure storage.
pub fn load_signer(snapshot: Vec<u8>) -> Result<SignerApp, String> {
    Ok(SignerApp {
        inner: Mutex::new(Signer::from_snapshot(&snapshot)?),
    })
}

impl SignerApp {
    pub fn snapshot(&self) -> Vec<u8> {
        self.inner.lock().unwrap().snapshot()
    }
    pub fn name(&self) -> String {
        self.inner.lock().unwrap().name().to_string()
    }
    pub fn device_id(&self) -> String {
        self.inner.lock().unwrap().device_id().to_string()
    }
    /// QR frames that enrol this signer with a coordinator.
    pub fn hello_frames(&self) -> Vec<String> {
        self.inner.lock().unwrap().hello_frames()
    }
    pub fn keys(&self) -> Vec<KeyInfo> {
        self.inner.lock().unwrap().keys()
    }
    /// Feed a fully scanned QR payload (see `QrScanner`).
    pub fn receive(&self, payload: Vec<u8>) -> Result<StepResult, String> {
        self.inner.lock().unwrap().receive(&payload)
    }
    pub fn approve(&self) -> Result<StepResult, String> {
        self.inner.lock().unwrap().approve()
    }
    pub fn reject(&self) {
        self.inner.lock().unwrap().reject()
    }

    // ---- backups
    /// The user typed in a physical backup after `Step::EnterBackup`: key number and 25 words.
    /// Errors (mistyped word, bad checksum) leave the signer waiting for a corrected attempt.
    pub fn submit_backup(&self, share_index: u32, words: Vec<String>) -> Result<StepResult, String> {
        self.inner.lock().unwrap().submit_backup(share_index, words)
    }
    /// The check-backup quiz question currently on screen (after `Step::CheckBackup`).
    pub fn quiz_question(&self) -> Result<QuizQuestion, String> {
        self.inner.lock().unwrap().quiz_question()
    }
    pub fn quiz_answer(&self, choice: u32) -> Result<QuizAnswer, String> {
        self.inner.lock().unwrap().quiz_answer(choice)
    }
    /// Backups typed into this signer that are still waiting for the coordinator.
    pub fn pending_backups(&self) -> Vec<PendingBackup> {
        self.inner.lock().unwrap().pending_backups()
    }
}

/// BIP39 words that start with `prefix`, for the backup entry autocomplete.
#[flutter_rust_bridge::frb(sync)]
pub fn backup_word_suggestions(prefix: String) -> Vec<String> {
    frost_backup::bip39_words::words_with_prefix(&prefix.trim().to_uppercase())
        .iter()
        .map(|w| w.to_string())
        .collect()
}

// ------------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ScanProgress {
    pub received: u32,
    pub total: u32,
    /// Set once every frame of the code arrived and its checksum matched.
    pub payload: Option<Vec<u8>>,
    /// Set when the scanned QR is not ours or is damaged; safe to show briefly or ignore.
    pub problem: Option<String>,
}

/// Collects the frames of an animated QR code.
pub struct QrScanner {
    inner: Mutex<qr_transport::Assembler>,
}

pub fn new_qr_scanner() -> QrScanner {
    QrScanner {
        inner: Mutex::new(qr_transport::Assembler::new()),
    }
}

impl QrScanner {
    pub fn push(&self, text: String) -> ScanProgress {
        match self.inner.lock().unwrap().push(&text) {
            Ok(p) => ScanProgress {
                received: p.received as u32,
                total: p.total as u32,
                payload: p.complete,
                problem: None,
            },
            Err(e) => ScanProgress {
                received: 0,
                total: 0,
                payload: None,
                problem: Some(e.to_string()),
            },
        }
    }
    pub fn reset(&self) {
        self.inner.lock().unwrap().reset()
    }
}

/// Decode QR codes from one camera frame's luma (Y) plane. `row_stride` is the plane's bytes per
/// row, which can be wider than `width`.
pub fn decode_qr_luma(width: u32, height: u32, row_stride: u32, luma: Vec<u8>) -> Vec<String> {
    let (w, h, stride) = (width as usize, height as usize, row_stride as usize);
    if w == 0 || h == 0 || stride < w || luma.len() < stride * (h - 1) + w {
        return vec![];
    }
    let mut img = rqrr::PreparedImage::prepare_from_greyscale(w, h, |x, y| luma[y * stride + x]);
    img.detect_grids()
        .into_iter()
        .filter_map(|g| g.decode().ok().map(|(_, text)| text))
        .collect()
}
