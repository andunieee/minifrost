//! The signer as the app uses it: the engine behind a lock, so it can be shared with other threads.

use std::sync::Mutex;

use crate::engine::Signer;
pub use crate::engine::{
    KeyInfo, OutputSummary, PendingBackup, Prompt, QuizAnswer, QuizQuestion, Step, StepResult,
    TxSummary,
};

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
    /// The user typed in a physical backup: key number and 25 words. Errors (mistyped word, bad
    /// checksum) change nothing. Otherwise it is saved and the step is its restore code.
    pub fn submit_backup(
        &self,
        share_index: u32,
        words: Vec<String>,
    ) -> Result<StepResult, String> {
        self.inner.lock().unwrap().submit_backup(share_index, words)
    }
    /// The restore code of a key this signer holds, or `Step::UnlockShare` while it is on the key
    /// store and not opened.
    pub fn restore_code(&self, key_id: String) -> Result<Step, String> {
        self.inner.lock().unwrap().restore_code(&key_id)
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

    // ---- deleting
    /// Delete this signer's share of a key for good. Returns the snapshot to store.
    pub fn delete_key(&self, key_id: String) -> Result<Vec<u8>, String> {
        self.inner.lock().unwrap().delete_key(&key_id)
    }
    /// Delete a typed-in backup still waiting for the coordinator. Returns the snapshot to store.
    pub fn delete_pending_backup(&self, id: String) -> Result<Vec<u8>, String> {
        self.inner.lock().unwrap().delete_pending_backup(&id)
    }

    // ---- device key store (the app seals and opens, see `Signer::share_for_keystore`)
    pub fn new_shares_on_keystore(&self) -> bool {
        self.inner.lock().unwrap().new_shares_on_keystore()
    }
    /// Returns the snapshot to store.
    pub fn set_new_shares_on_keystore(&self, enabled: bool) -> Vec<u8> {
        self.inner
            .lock()
            .unwrap()
            .set_new_shares_on_keystore(enabled)
    }
    /// The bytes to seal with the key store before `moved_to_keystore`.
    pub fn share_for_keystore(&self, key_id: String) -> Result<Vec<u8>, String> {
        self.inner.lock().unwrap().share_for_keystore(&key_id)
    }
    /// Returns the snapshot to store.
    pub fn moved_to_keystore(&self, key_id: String, sealed: Vec<u8>) -> Result<Vec<u8>, String> {
        self.inner
            .lock()
            .unwrap()
            .moved_to_keystore(&key_id, sealed)
    }
    /// The sealed share, for the key store to open before `open_keystore_share`.
    pub fn keystore_share(&self, key_id: String) -> Result<Vec<u8>, String> {
        self.inner.lock().unwrap().keystore_share(&key_id)
    }
    pub fn open_keystore_share(&self, key_id: String, share: Vec<u8>) -> Result<(), String> {
        self.inner
            .lock()
            .unwrap()
            .open_keystore_share(&key_id, share)
    }
    /// The share must be opened first. Returns the snapshot to store.
    pub fn move_to_disk(&self, key_id: String) -> Result<Vec<u8>, String> {
        self.inner.lock().unwrap().move_to_disk(&key_id)
    }
}

/// BIP39 words that start with `prefix`, for the backup entry autocomplete.
pub fn backup_word_suggestions(prefix: String) -> Vec<String> {
    frost_backup::bip39_words::words_with_prefix(&prefix.trim().to_uppercase())
        .iter()
        .map(|w| w.to_string())
        .collect()
}
