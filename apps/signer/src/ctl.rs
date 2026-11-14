//! Glue between the signer engine and storage. The one rule it enforces: whenever the engine says
//! its state changed, the new snapshot is durably stored BEFORE the caller gets the result (and
//! therefore before any QR code is shown). After a partial signature the snapshot contains the
//! burned nonce; showing the signature first would risk nonce reuse after a crash.

use std::{
    path::{
        Path,
        PathBuf,
    },
    sync::Arc,
};

use minifrost_signer::api::{
    KeyInfo,
    PendingBackup,
    QuizAnswer,
    QuizQuestion,
    SignerApp,
    Step,
    StepResult,
};
use minifrost_ui::prelude::*;

use crate::keystore;

/// The signer's state, in one plain, unencrypted file in the app's data folder, written
/// atomically (temporary file, then rename). On Android that is the app-private folder, which is
/// excluded from backups.
#[derive(Clone, PartialEq, Debug)]
pub struct Vault(PathBuf);

impl Vault {
    pub fn new(dir: &Path) -> Self {
        Self(dir.join("minifrost.vault"))
    }

    pub fn exists(&self) -> bool {
        self.0.exists()
    }

    pub fn load(&self) -> Result<Vec<u8>, String> {
        std::fs::read(&self.0).map_err(|e| format!("Could not read {}: {e}", self.0.display()))
    }

    pub fn save(&self, bytes: &[u8]) -> Result<(), String> {
        use std::io::Write;
        let write = || -> std::io::Result<()> {
            if let Some(dir) = self.0.parent() {
                std::fs::create_dir_all(dir)?;
            }
            let tmp = self.0.with_extension("vault.tmp");
            let mut file = std::fs::File::create(&tmp)?;
            file.write_all(bytes)?;
            file.sync_all()?;
            std::fs::rename(&tmp, &self.0)
        };
        write().map_err(|e| format!("Could not save the signer to {}: {e}", self.0.display()))
    }
}

/// What the screens show of the signer; refreshed after every change.
#[derive(Clone, Default, PartialEq)]
pub struct Data {
    pub keys: Vec<KeyInfo>,
    pub pending: Vec<PendingBackup>,
    pub new_on_keystore: bool,
}

/// The signer and its storage. Cheap to clone; every clone is the same signer.
#[derive(Clone)]
pub struct Ctl {
    signer: Arc<SignerApp>,
    vault: Vault,
    pub device_id: String,
    pub name: String,
    pub data: State<Data>,
}

impl PartialEq for Ctl {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.signer, &other.signer)
    }
}

impl Ctl {
    pub fn new(signer: SignerApp, vault: Vault) -> Self {
        let ctl = Self {
            device_id: signer.device_id(),
            name: signer.name(),
            signer: Arc::new(signer),
            vault,
            // owned by the root: the screen that creates the signer goes away right after
            data: State::create_in_scope(Data::default(), ScopeId::ROOT),
        };
        ctl.refresh();
        ctl
    }

    pub fn refresh(&self) {
        let mut data = self.data;
        data.set(Data {
            keys: self.signer.keys(),
            pending: self.signer.pending_backups(),
            new_on_keystore: self.signer.new_shares_on_keystore(),
        });
    }

    pub fn hello_frames(&self) -> Vec<String> {
        self.signer.hello_frames()
    }

    fn save(&self, snapshot: &[u8]) -> Result<(), String> {
        self.vault.save(snapshot)?;
        self.refresh();
        Ok(())
    }

    async fn persist(&self, r: StepResult) -> Result<StepResult, String> {
        if let Some(snapshot) = &r.snapshot {
            self.vault.save(snapshot)?;
        }
        self.refresh();
        match &r.new_share {
            Some(key) if self.data.peek().new_on_keystore && keystore::SUPPORTED => {
                let key = key.clone();
                Ok(self.move_new_share(r, &key).await)
            }
            _ => Ok(r),
        }
    }

    /// "Store new shares on the device key store": the share was just stored on disk, move it now.
    /// When that fails the share simply stays on disk, and the step's message says so.
    async fn move_new_share(&self, r: StepResult, key_id: &str) -> StepResult {
        let note = match self.move_to_keystore(key_id).await {
            Ok(()) => "The share is on the device key store.".to_string(),
            Err(e) => format!(
                "The share stayed on disk: {e} You can move it to the device key store from its menu on the home screen."
            ),
        };
        match r.step {
            Step::Done { message } => StepResult {
                step: Step::Done {
                    message: format!("{message}\n\n{note}"),
                },
                ..r
            },
            _ => r,
        }
    }

    pub async fn receive(&self, payload: &[u8]) -> Result<StepResult, String> {
        let r = self.persist(self.signer.receive(payload.to_vec())?).await?;
        if let Step::UnlockShare { key_id, .. } = &r.step {
            // the message needs a share on the key store: open it (screen lock), then read it again
            self.open_share(key_id).await?;
            return self.persist(self.signer.receive(payload.to_vec())?).await;
        }
        Ok(r)
    }

    pub async fn approve(&self) -> Result<StepResult, String> {
        self.persist(self.signer.approve()?).await
    }

    pub fn reject(&self) {
        self.signer.reject()
    }

    /// The user typed a physical backup (key number and 25 words). Errors with a readable message
    /// when the words don't form a valid backup, so they can be corrected. Otherwise the backup is
    /// saved and the step is its restore code.
    pub async fn submit_backup(&self, share_index: u32, words: Vec<String>) -> Result<StepResult, String> {
        self.persist(self.signer.submit_backup(share_index, words)?).await
    }

    /// The restore code of a key this signer holds. A share on the key store is opened first
    /// (screen lock), since the code carries the share's public image.
    pub async fn restore_code(&self, key_id: &str) -> Result<Step, String> {
        let step = self.signer.restore_code(key_id.to_string())?;
        if !matches!(step, Step::UnlockShare { .. }) {
            return Ok(step);
        }
        self.open_share(key_id).await?;
        self.signer.restore_code(key_id.to_string())
    }

    pub fn quiz_question(&self) -> Result<QuizQuestion, String> {
        self.signer.quiz_question()
    }

    pub async fn quiz_answer(&self, choice: u32) -> Result<QuizAnswer, String> {
        let answer = self.signer.quiz_answer(choice)?;
        if let Some(done) = &answer.done {
            self.persist(done.clone()).await?;
        }
        Ok(answer)
    }

    // ---- deleting

    /// Deletes this signer's share of a key for good. A share on the key store goes with its
    /// sealed copy; nothing needs to be opened for that.
    pub fn delete_key(&self, key_id: &str) -> Result<(), String> {
        self.save(&self.signer.delete_key(key_id.to_string())?)
    }

    pub fn delete_pending_backup(&self, id: &str) -> Result<(), String> {
        self.save(&self.signer.delete_pending_backup(id.to_string())?)
    }

    // ---- device key store

    pub fn set_new_on_keystore(&self, enabled: bool) -> Result<(), String> {
        self.save(&self.signer.set_new_shares_on_keystore(enabled))
    }

    pub async fn move_to_keystore(&self, key_id: &str) -> Result<(), String> {
        let sealed = keystore::seal(&self.signer.share_for_keystore(key_id.to_string())?).await?;
        self.save(&self.signer.moved_to_keystore(key_id.to_string(), sealed)?)
    }

    /// Opens a share on the key store for the rest of this session.
    pub async fn open_share(&self, key_id: &str) -> Result<(), String> {
        let share = keystore::open(&self.signer.keystore_share(key_id.to_string())?).await?;
        self.signer.open_keystore_share(key_id.to_string(), share)
    }

    pub async fn move_to_disk(&self, key_id: &str) -> Result<(), String> {
        self.open_share(key_id).await?;
        self.save(&self.signer.move_to_disk(key_id.to_string())?)
    }
}
