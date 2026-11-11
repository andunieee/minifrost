//! The coordinator state machine. Pure Rust apart from the snapshot file: QR payloads in,
//! [`Update`]s out. The signing flow is the one from the spec:
//!
//! 1. broadcast `SignPrepare` (transaction to review + nonce request)  -> signers show nonces
//! 2. scan every signer's nonces, broadcast the bundle (aggregated nonces, per-signer requests)
//! 3. scan every signer's partial signature -> a valid signed transaction

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use bitcoin::Network;
use frostsnap_core::{
    AccessStructureRef, DeviceId, KeyId, KeygenId, SessionHash, SignSessionId, SymmetricKey,
    WireSignTask,
    coordinator::{
        BeginKeygen, CoordinatorSend, CoordinatorToUserKeyGenMessage, CoordinatorToUserMessage,
        CoordinatorToUserSigningMessage, FrostCoordinator, Mutation, restoration::RecoverShare,
    },
    device::KeyPurpose,
    message::{
        CoordinatorToDeviceMessage, DeviceToCoordinatorMessage, KeyGenAck, keygen::DeviceKeygen,
        signing::OpenNonceStreams,
    },
};
use minifrost_wire::{Packet, SignPrepare, Step as WireStep};
use rand::{RngCore, rngs::OsRng};

use crate::wallet::{self, Amt, BuiltSend, HttpEsplora, WalletView};

pub mod restore;
use restore::BackupSession;
pub use restore::{BackupShare, PendingConsolidationView, RestoringShareView, UnnamedWalletView};

/// 3: shares being restored are gathered here (`restoring`) instead of in `frostsnap_core`
/// restorations.
const SNAPSHOT_VERSION: u8 = 3;
const FILE: &str = "coordinator.bin";

type Res<T> = Result<T, String>;

#[derive(bincode::Encode, bincode::Decode)]
struct Snapshot {
    /// Coordinator-held half of the share-decryption secret. Compromising only the coordinator
    /// (this file) gives an attacker nothing to sign with: the shares live on the signers.
    encryption_key: [u8; 32],
    signers: BTreeMap<DeviceId, String>,
    history: Vec<Mutation>,
    esplora_url: Option<String>,
    /// Backup run: (wallet, share index, backup recorded). Wallets that were restored have no
    /// entry and count as backed up, like in frostsnap.
    backups: Vec<(AccessStructureRef, u32, bool)>,
    /// Shares read from signers whose wallet isn't rebuilt yet.
    restoring: Vec<RecoverShare>,
}

#[derive(bincode::Decode)]
struct SnapshotV2 {
    encryption_key: [u8; 32],
    signers: BTreeMap<DeviceId, String>,
    history: Vec<Mutation>,
    esplora_url: Option<String>,
    backups: Vec<(AccessStructureRef, u32, bool)>,
}

// ---------------------------------------------------------------------------------------------
// What the UI sees
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct SignerInfo {
    pub device_id: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct KeyView {
    pub key_id: String,
    pub name: String,
    pub network: String,
    pub threshold: u16,
    pub signers: Vec<SignerInfo>,
}

#[derive(Debug, Clone)]
pub struct Broadcast {
    pub title: String,
    pub frames: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum Finished {
    KeyCreated {
        key_id: String,
        name: String,
    },
    Signed {
        txid: String,
        tx_hex: String,
    },
    /// A backup was displayed and the user confirmed writing it down.
    BackupRecorded {
        key_name: String,
        share_index: u32,
    },
    /// A signer's check-backup quiz was passed.
    BackupChecked {
        key_name: String,
        share_index: u32,
    },
    /// A wallet is rebuilt and no signer is waiting for anything, or a signer rejoined it.
    WalletRestored {
        key_id: String,
        name: String,
    },
}

#[derive(Debug, Clone, Default)]
pub struct Update {
    pub message: String,
    /// The signer whose reply produced this update, when there is one.
    pub from: Option<SignerInfo>,
    /// The reply had already been received this round; nothing changed.
    pub repeat: bool,
    /// QR to show to *all* signers next. `None` while there is nothing new to show.
    pub broadcast: Option<Broadcast>,
    /// The signers whose reply this round is still missing.
    pub awaiting: Vec<SignerInfo>,
    /// The signers whose reply this round has already been received.
    pub received: Vec<SignerInfo>,
    /// Keygen only: signers show the same code; the humans compare.
    pub check_code: Option<String>,
    /// This step needs no signer reply: the user confirms it on the coordinator with this button.
    pub confirm: Option<String>,
    /// Keygen check round: no signer reply either, but the user confirms each signer in
    /// `awaiting` separately, with `confirm_signer`, once it shows the check code and approved.
    pub confirm_each_signer: bool,
    /// Restoration: the shares read so far rebuild this wallet, but none knows its name and
    /// network. Ask the user, then call `name_restored_wallet`.
    pub unnamed_wallet: Option<UnnamedWalletView>,
    pub finished: Option<Finished>,
    pub snapshot_saved: bool,
}

// ---------------------------------------------------------------------------------------------

struct KeygenSession {
    id: KeygenId,
    devices: Vec<DeviceId>,
    round: KeygenRound,
    /// What every signer must show as its check code, once the certificates are in.
    session_hash: Option<SessionHash>,
}

/// The keygen reply the coordinator is waiting for.
#[derive(Clone, Copy, PartialEq, Eq)]
enum KeygenRound {
    Responses,
    Certificates,
    Acks,
}

impl KeygenRound {
    /// Which round a signer's keygen reply answers, and for which keygen.
    fn of(message: &DeviceKeygen) -> (Self, KeygenId) {
        match message {
            DeviceKeygen::Response(r) => (Self::Responses, r.keygen_id),
            DeviceKeygen::Certify { keygen_id, .. } => (Self::Certificates, *keygen_id),
            DeviceKeygen::Ack(a) => (Self::Acks, a.keygen_id),
        }
    }
}

struct SigningSession {
    key_id: KeyId,
    access: AccessStructureRef,
    built: BuiltSend,
    devices: BTreeSet<DeviceId>,
    session_id: Option<SignSessionId>,
}

enum Session {
    Idle,
    Keygen(KeygenSession),
    Signing(SigningSession),
    Backup(BackupSession),
}

pub struct Coordinator {
    inner: FrostCoordinator,
    encryption_key: SymmetricKey,
    signers: BTreeMap<DeviceId, String>,
    history: Vec<Mutation>,
    esplora_url: Option<String>,
    session: Session,
    /// signers that already answered in the current round
    replied: BTreeSet<DeviceId>,
    dir: Option<PathBuf>,
    wallets: BTreeMap<KeyId, WalletView>,
    prepared: Option<(KeyId, BuiltSend)>,
    backups: BTreeMap<(AccessStructureRef, u32), bool>,
    restoring: Vec<RecoverShare>,
}

impl Coordinator {
    /// Open (or create) the coordinator stored in `dir`.
    pub fn open(dir: &Path) -> Res<Self> {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let path = dir.join(FILE);
        let mut me = match std::fs::read(&path) {
            Ok(bytes) => Self::from_snapshot(&bytes)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let mut key = [0u8; 32];
                OsRng.fill_bytes(&mut key);
                Self::from_parts(Snapshot {
                    encryption_key: key,
                    signers: Default::default(),
                    history: vec![],
                    esplora_url: None,
                    backups: vec![],
                    restoring: vec![],
                })
            }
            Err(e) => return Err(format!("could not read {}: {e}", path.display())),
        };
        me.dir = Some(dir.to_path_buf());
        me.save()?;
        Ok(me)
    }

    pub fn in_memory() -> Self {
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);
        Self::from_parts(Snapshot {
            encryption_key: key,
            signers: Default::default(),
            history: vec![],
            esplora_url: None,
            backups: vec![],
            restoring: vec![],
        })
    }

    fn from_parts(snap: Snapshot) -> Self {
        let mut inner = FrostCoordinator::new();
        for m in &snap.history {
            inner.apply_mutation(m.clone());
        }
        Self {
            inner,
            encryption_key: SymmetricKey(snap.encryption_key),
            signers: snap.signers,
            history: snap.history,
            esplora_url: snap.esplora_url,
            session: Session::Idle,
            replied: Default::default(),
            dir: None,
            wallets: Default::default(),
            prepared: None,
            backups: snap
                .backups
                .into_iter()
                .map(|(a, i, done)| ((a, i), done))
                .collect(),
            restoring: snap.restoring,
        }
    }

    fn from_snapshot(bytes: &[u8]) -> Res<Self> {
        let (&v, rest) = bytes.split_first().ok_or("empty coordinator file")?;
        let corrupt = |e: bincode::error::DecodeError| format!("corrupt coordinator file: {e}");
        match v {
            2 => {
                let (v2, _): (SnapshotV2, _) =
                    bincode::decode_from_slice(rest, bincode::config::standard())
                        .map_err(corrupt)?;
                let mut me = Self::from_parts(Snapshot {
                    encryption_key: v2.encryption_key,
                    signers: v2.signers,
                    history: v2.history,
                    esplora_url: v2.esplora_url,
                    backups: v2.backups,
                    restoring: vec![],
                });
                me.migrate_restorations();
                Ok(me)
            }
            SNAPSHOT_VERSION => {
                let (snap, _): (Snapshot, _) =
                    bincode::decode_from_slice(rest, bincode::config::standard())
                        .map_err(corrupt)?;
                Ok(Self::from_parts(snap))
            }
            _ => Err(format!("unknown coordinator file version {v}")),
        }
    }

    fn snapshot_bytes(&self) -> Vec<u8> {
        let snap = Snapshot {
            encryption_key: self.encryption_key.0,
            signers: self.signers.clone(),
            history: self.history.clone(),
            esplora_url: self.esplora_url.clone(),
            backups: self
                .backups
                .iter()
                .map(|((a, i), done)| (*a, *i, *done))
                .collect(),
            restoring: self.restoring.clone(),
        };
        let mut out = vec![SNAPSHOT_VERSION];
        out.extend(bincode::encode_to_vec(&snap, bincode::config::standard()).expect("encodes"));
        out
    }

    /// Atomic write: temp file, fsync, rename. Owner-only permissions.
    fn save(&mut self) -> Res<()> {
        let staged: Vec<Mutation> = self.inner.take_staged_mutations().into_iter().collect();
        self.history.extend(staged);
        let Some(dir) = &self.dir else { return Ok(()) };
        let tmp = dir.join(format!("{FILE}.tmp"));
        {
            use std::io::Write;
            #[cfg(unix)]
            use std::os::unix::fs::OpenOptionsExt;
            let mut opts = std::fs::OpenOptions::new();
            opts.write(true).create(true).truncate(true);
            #[cfg(unix)]
            opts.mode(0o600);
            let mut f = opts.open(&tmp).map_err(|e| e.to_string())?;
            f.write_all(&self.snapshot_bytes())
                .map_err(|e| e.to_string())?;
            f.sync_all().map_err(|e| e.to_string())?;
        }
        std::fs::rename(&tmp, dir.join(FILE)).map_err(|e| e.to_string())
    }

    // ------------------------------------------------------------------ signers & keys ------

    pub fn signers(&self) -> Vec<SignerInfo> {
        self.signers
            .iter()
            .map(|(id, name)| SignerInfo {
                device_id: id.to_string(),
                name: name.clone(),
            })
            .collect()
    }

    fn name_of(&self, id: &DeviceId) -> String {
        self.signers
            .get(id)
            .cloned()
            .unwrap_or_else(|| id.to_string()[..8].to_string())
    }

    pub fn keys(&self) -> Vec<KeyView> {
        self.inner
            .iter_keys()
            .map(|k| {
                let acc = k.master_access_structure();
                KeyView {
                    key_id: k.key_id.to_string(),
                    name: k.key_name.clone(),
                    network: purpose_network(&k.purpose).to_string(),
                    threshold: acc.threshold(),
                    signers: acc
                        .devices_by_share_index()
                        .into_iter()
                        .map(|d| SignerInfo {
                            device_id: d.to_string(),
                            name: self.name_of(&d),
                        })
                        .collect(),
                }
            })
            .collect()
    }

    /// A signer's "hello" QR. Returns its name.
    pub fn enroll(&mut self, payload: &[u8]) -> Res<SignerInfo> {
        match Packet::from_bytes(payload).map_err(|e| e.to_string())? {
            Packet::Hello { device_id, name } => {
                let info = self.register_signer(device_id, &name);
                self.save()?;
                Ok(info)
            }
            _ => Err("That is not a signer enrolment code.".into()),
        }
    }

    fn register_signer(&mut self, device_id: DeviceId, name: &str) -> SignerInfo {
        let name = name.trim().chars().take(40).collect::<String>();
        self.signers.insert(device_id, name.clone());
        SignerInfo {
            device_id: device_id.to_string(),
            name,
        }
    }

    pub fn forget_signer(&mut self, device_id: &str) -> Res<()> {
        let id: DeviceId = device_id.parse().map_err(|_| "bad device id")?;
        if self
            .inner
            .iter_keys()
            .any(|k| k.master_access_structure().contains_device(id))
        {
            return Err("This signer holds a share of an existing key.".into());
        }
        if self.restoring.iter().any(|s| s.held_by == id) {
            return Err("This signer holds a share being restored.".into());
        }
        self.signers.remove(&id);
        self.save()
    }

    pub fn set_esplora_url(&mut self, url: Option<String>) -> Res<()> {
        self.esplora_url = url.filter(|u| !u.trim().is_empty());
        self.save()
    }

    pub fn esplora_url(&self) -> Option<String> {
        self.esplora_url.clone()
    }

    // ------------------------------------------------------------------ keygen --------------

    pub fn start_keygen(
        &mut self,
        key_name: String,
        threshold: u16,
        device_ids: Vec<String>,
        network: Network,
    ) -> Res<Update> {
        if !matches!(self.session, Session::Idle) {
            return Err(
                "Another keygen or signing session is in progress. Cancel it first.".into(),
            );
        }
        let devices: Vec<DeviceId> = device_ids
            .iter()
            .map(|d| d.parse().map_err(|_| "bad device id".to_string()))
            .collect::<Res<_>>()?;
        let unique: BTreeSet<_> = devices.iter().collect();
        if unique.len() != devices.len() || devices.is_empty() {
            return Err("Pick each signer once.".into());
        }
        if threshold == 0 || threshold as usize > devices.len() {
            return Err("Threshold must be between 1 and the number of signers.".into());
        }
        if key_name.trim().is_empty() {
            return Err("Give the wallet a name.".into());
        }
        let begin = BeginKeygen::new(
            devices.clone(),
            threshold,
            key_name.trim().to_string(),
            KeyPurpose::Bitcoin(network),
            &mut OsRng,
        );
        let keygen_id = begin.keygen_id;
        let sends = self
            .inner
            .begin_keygen(begin, &mut OsRng)
            .map_err(|e| e.to_string())?;
        self.session = Session::Keygen(KeygenSession {
            id: keygen_id,
            devices: devices.clone(),
            round: KeygenRound::Responses,
            session_hash: None,
        });
        self.replied.clear();
        let mut update = Update {
            message: format!(
                "Show this to all {} signers and have each approve it.",
                devices.len()
            ),
            broadcast: Some(self.broadcast(sends.into_iter().collect(), WireStep::KeygenJoin)),
            ..Default::default()
        };
        self.fill_awaiting(&mut update, &devices.iter().copied().collect());
        Ok(update)
    }

    // ------------------------------------------------------------------ signing -------------

    pub fn begin_signing(&mut self, key_id: &str, device_ids: Vec<String>) -> Res<Update> {
        if !matches!(self.session, Session::Idle) {
            return Err(
                "Another keygen or signing session is in progress. Cancel it first.".into(),
            );
        }
        let key = self
            .inner
            .iter_keys()
            .find(|k| k.key_id.to_string() == key_id)
            .ok_or("unknown key")?;
        let key_id = key.key_id;
        let acc = key.master_access_structure();
        let built = match self.prepared.take() {
            Some((k, b)) if k == key_id => b,
            _ => return Err("Prepare a payment first.".into()),
        };
        let devices: BTreeSet<DeviceId> = device_ids
            .iter()
            .map(|d| d.parse().map_err(|_| "bad device id".to_string()))
            .collect::<Res<_>>()?;
        if devices.len() < acc.threshold() as usize {
            return Err(format!("Choose at least {} signers.", acc.threshold()));
        }
        if let Some(d) = devices.iter().find(|d| !acc.contains_device(**d)) {
            return Err(format!("{} holds no share of this key.", self.name_of(d)));
        }
        let request = self
            .inner
            .maybe_request_nonce_replenishment(&devices, 1, &mut OsRng);
        let open: Vec<(DeviceId, OpenNonceStreams)> = request.into_open_nonce_streams().collect();
        let packet = Packet::SignPrepare(SignPrepare {
            step: WireStep::SignReview,
            key_id,
            sign_task: WireSignTask::BitcoinTransaction(built.template.clone()),
            open_nonce_streams: open,
        });
        self.save()?;
        self.session = Session::Signing(SigningSession {
            key_id,
            access: acc.access_structure_ref(),
            built,
            devices: devices.clone(),
            session_id: None,
        });
        self.replied.clear();
        let mut update = Update {
            message: "Show this to the chosen signers. Each reviews the transaction and shows nonces back.".into(),
            broadcast: Some(Broadcast {
                title: WireStep::SignReview.title().into(),
                frames: qr_transport::split(&packet.to_bytes(), qr_transport::DEFAULT_FRAME_CHARS),
            }),
            ..Default::default()
        };
        self.fill_awaiting(&mut update, &devices);
        Ok(update)
    }

    pub fn cancel(&mut self) -> Res<()> {
        match std::mem::replace(&mut self.session, Session::Idle) {
            Session::Keygen(k) => self.inner.cancel_keygen(k.id),
            Session::Signing(s) => {
                if let Some(id) = s.session_id {
                    self.inner.cancel_sign_session(id);
                }
            }
            Session::Backup(_) | Session::Idle => {}
        }
        self.replied.clear();
        self.save()
    }

    // ------------------------------------------------------------------ receiving -----------

    fn fill_awaiting(&self, update: &mut Update, expected: &BTreeSet<DeviceId>) {
        let (received, awaiting): (Vec<_>, Vec<_>) =
            expected.iter().partition(|d| self.replied.contains(d));
        let info = |d: &DeviceId| SignerInfo {
            device_id: d.to_string(),
            name: self.name_of(d),
        };
        update.awaiting = awaiting.into_iter().map(info).collect();
        update.received = received.into_iter().map(info).collect();
    }

    fn expected(&self) -> BTreeSet<DeviceId> {
        match &self.session {
            Session::Idle => Default::default(),
            Session::Keygen(k) => k.devices.iter().copied().collect(),
            Session::Signing(s) => s.devices.clone(),
            Session::Backup(b) => b.devices(),
        }
    }

    /// Keygen check round: the user approved the check code on this signer, which stored its
    /// share right then. A signer's ack says only that (the keygen id and the session hash,
    /// unsigned), so the coordinator feeds frostsnap the ack itself instead of scanning a QR. The
    /// wallet is created once every signer is confirmed.
    pub fn confirm_signer(&mut self, device_id: &str) -> Res<Update> {
        let device = self.parse_device(device_id)?;
        if matches!(
            self.session,
            Session::Backup(BackupSession::FinishRestoring { .. })
        ) {
            return self.confirm_restored_signer(device);
        }
        let (keygen_id, ack_session_hash) = match &self.session {
            Session::Keygen(KeygenSession {
                id,
                devices,
                round: KeygenRound::Acks,
                session_hash: Some(hash),
            }) if devices.contains(&device) => (*id, *hash),
            Session::Keygen(KeygenSession {
                round: KeygenRound::Acks,
                ..
            }) => {
                return Err(format!(
                    "{} is not part of this session.",
                    self.name_of(&device)
                ));
            }
            _ => return Err("No signer is waiting for a confirmation.".into()),
        };
        let name = self.name_of(&device);
        if !self.replied.insert(device) {
            return Err(format!("{name} was already confirmed."));
        }
        let result = self.handle_device_message(
            device,
            DeviceToCoordinatorMessage::KeyGen(DeviceKeygen::Ack(KeyGenAck {
                keygen_id,
                ack_session_hash,
            })),
        );
        if result.is_err() {
            self.replied.remove(&device);
        }
        let mut update = result?;
        update.from = Some(SignerInfo {
            device_id: device.to_string(),
            name: name.clone(),
        });
        update.snapshot_saved = {
            self.save()?;
            self.dir.is_some()
        };
        if update.finished.is_none() {
            update.message = format!("{name} confirmed.");
            update.check_code = Some(minifrost_wire::format_check_code(&ack_session_hash.0));
            update.confirm_each_signer = true;
            let expected = self.expected();
            self.fill_awaiting(&mut update, &expected);
        }
        Ok(update)
    }

    /// A reassembled QR payload from a signer.
    pub fn receive(&mut self, payload: &[u8]) -> Res<Update> {
        let packet = Packet::from_bytes(payload).map_err(|e| e.to_string())?;
        let (from, message) = match packet {
            Packet::FromDevice { device_id, message } => (device_id, message),
            Packet::Hello { .. } => return Err("That is an enrolment code, not a reply.".into()),
            Packet::RestoreShare { .. } => {
                return Err("That is a restore code: read it on the Restore page.".into());
            }
            _ => return Err("That code is meant for signers.".into()),
        };
        if matches!(self.session, Session::Idle) {
            return Err("No keygen, signing or backup session is running.".into());
        }
        if matches!(self.session, Session::Backup(_)) {
            return Err(
                "Nothing to scan for this step: confirm it on this screen once the signer is done."
                    .into(),
            );
        }
        let expected = self.expected();
        if !expected.contains(&from) {
            return Err(format!(
                "{} is not part of this session.",
                self.name_of(&from)
            ));
        }
        // frostsnap_core drops the whole pending keygen when it rejects a keygen message, so a
        // signer's reply to an earlier round (its old QR still on screen) must not reach it.
        if let (Session::Keygen(k), DeviceToCoordinatorMessage::KeyGen(m)) =
            (&self.session, &message)
            && KeygenRound::of(m) != (k.round, k.id)
        {
            let mut u = Update {
                message: format!(
                    "That is {}'s reply to an earlier step. Show it the current code.",
                    self.name_of(&from)
                ),
                from: Some(SignerInfo {
                    device_id: from.to_string(),
                    name: self.name_of(&from),
                }),
                repeat: true,
                ..Default::default()
            };
            self.fill_awaiting(&mut u, &expected);
            return Ok(u);
        }
        if !self.replied.insert(from) {
            let mut u = Update {
                message: format!("Already have {}'s reply.", self.name_of(&from)),
                from: Some(SignerInfo {
                    device_id: from.to_string(),
                    name: self.name_of(&from),
                }),
                repeat: true,
                ..Default::default()
            };
            self.fill_awaiting(&mut u, &expected);
            return Ok(u);
        }
        let result = self.handle_device_message(from, message);
        if result.is_err() {
            // let the signer try again
            self.replied.remove(&from);
        }
        let mut update = result?;
        update.from = Some(SignerInfo {
            device_id: from.to_string(),
            name: self.name_of(&from),
        });
        update.snapshot_saved = {
            self.save()?;
            self.dir.is_some()
        };
        let expected = self.expected();
        if update.finished.is_none() {
            self.fill_awaiting(&mut update, &expected);
        }
        Ok(update)
    }

    fn handle_device_message(
        &mut self,
        from: DeviceId,
        message: DeviceToCoordinatorMessage,
    ) -> Res<Update> {
        let sends = self
            .inner
            .recv_device_message(from, message)
            .map_err(|e| e.to_string())?;
        let mut update = Update {
            message: format!("Got reply from {}.", self.name_of(&from)),
            ..Default::default()
        };
        let mut to_devices = vec![];
        let mut signed = None;
        let mut finalize = None;
        let mut session_hash = None;
        for send in sends {
            match send {
                CoordinatorSend::ToDevice { .. } => to_devices.push(send),
                CoordinatorSend::ToUser(u) => match u {
                    CoordinatorToUserMessage::KeyGen { keygen_id, inner } => match inner {
                        CoordinatorToUserKeyGenMessage::CheckKeyGen { session_hash: hash } => {
                            update.check_code = Some(minifrost_wire::format_check_code(&hash.0));
                            session_hash = Some(hash);
                        }
                        CoordinatorToUserKeyGenMessage::KeyGenAck {
                            all_acks_received: true,
                            ..
                        } => {
                            finalize = Some(keygen_id);
                        }
                        _ => {}
                    },
                    CoordinatorToUserMessage::Signing(
                        CoordinatorToUserSigningMessage::Signed { signatures, .. },
                    ) => signed = Some(signatures),
                    _ => {}
                },
            }
        }

        if let Some(keygen_id) = finalize {
            let fin = self
                .inner
                .finalize_keygen(keygen_id, self.encryption_key, &mut OsRng)
                .map_err(|e| e.to_string())?;
            let key_id = fin.access_structure_ref.key_id;
            self.start_backup_run(fin.access_structure_ref);
            let name = self
                .inner
                .get_frost_key(key_id)
                .map(|k| k.key_name.clone())
                .unwrap_or_default();
            self.session = Session::Idle;
            self.replied.clear();
            update.finished = Some(Finished::KeyCreated {
                key_id: key_id.to_string(),
                name: name.clone(),
            });
            update.message = format!("Key “{name}” created.");
        }

        if !to_devices.is_empty() {
            let step = match (&self.session, update.finished.is_some()) {
                (Session::Keygen(_), _) if update.check_code.is_some() => WireStep::KeygenCheck,
                (Session::Keygen(_), _) => WireStep::KeygenCertify,
                _ => WireStep::SignNonces,
            };
            update.broadcast = Some(self.broadcast(to_devices, step));
            if let Session::Keygen(k) = &mut self.session {
                k.round = if update.check_code.is_some() {
                    KeygenRound::Acks
                } else {
                    KeygenRound::Certificates
                };
                k.session_hash = session_hash;
            }
            if !matches!(self.session, Session::Idle) {
                self.replied.clear();
                update.message = match &self.session {
                    Session::Keygen(_) if update.check_code.is_some() => {
                        "Every signer must show the same check code as below. If so, approve on each signer and confirm it here.".into()
                    }
                    Session::Keygen(_) => "Show this to all signers; each answers automatically.".into(),
                    _ => "Show this to the signers. Each verifies the transaction and shows a partial signature.".into(),
                };
                update.confirm_each_signer = update.check_code.is_some();
            }
        }

        // signing: once every chosen signer returned nonces, build the bundle
        if let Session::Signing(s) = &self.session
            && s.session_id.is_none()
            && self.replied.len() == s.devices.len()
        {
            let bundle = self.start_signing_round()?;
            update.broadcast = Some(bundle);
            update.message = "All nonces received. Show this bundle to the signers.".into();
            self.replied.clear();
        }

        if let Some(sigs) = signed {
            let Session::Signing(s) = std::mem::replace(&mut self.session, Session::Idle) else {
                return Err("signature completed without a signing session".into());
            };
            let master = self
                .inner
                .get_frost_key(s.key_id)
                .ok_or("key vanished")?
                .complete_key
                .master_appkey;
            let tx = s
                .built
                .template
                .as_seen_by(master)
                .to_signed_rust_bitcoin_tx(&sigs)
                .map_err(|e| e.to_string())?;
            self.replied.clear();
            update.finished = Some(Finished::Signed {
                txid: tx.compute_txid().to_string(),
                tx_hex: bitcoin::consensus::encode::serialize_hex(&tx),
            });
            update.message = "Signed. The transaction is ready to broadcast.".into();
        }
        Ok(update)
    }

    fn broadcast(&self, sends: Vec<CoordinatorSend>, step: WireStep) -> Broadcast {
        let entries: Vec<(Vec<DeviceId>, CoordinatorToDeviceMessage)> = sends
            .into_iter()
            .filter_map(|s| match s {
                CoordinatorSend::ToDevice {
                    message,
                    destinations,
                } => Some((destinations.into_iter().collect(), message)),
                _ => None,
            })
            .collect();
        let bytes = Packet::ToDevices { step, entries }.to_bytes();
        Broadcast {
            title: step.title().to_string(),
            frames: qr_transport::split(&bytes, qr_transport::DEFAULT_FRAME_CHARS),
        }
    }

    fn start_signing_round(&mut self) -> Res<Broadcast> {
        let Session::Signing(s) = &mut self.session else {
            unreachable!()
        };
        let session_id = self
            .inner
            .start_sign(
                s.access,
                WireSignTask::BitcoinTransaction(s.built.template.clone()),
                &s.devices,
                &mut OsRng,
            )
            .map_err(|e| e.to_string())?;
        s.session_id = Some(session_id);
        let devices = s.devices.clone();
        let sends: Vec<CoordinatorSend> = devices
            .into_iter()
            .map(|d| {
                self.inner
                    .request_device_sign(session_id, d, self.encryption_key)
                    .into()
            })
            .collect();
        Ok(self.broadcast(sends, WireStep::SignNonces))
    }

    // ------------------------------------------------------------------ wallet --------------

    fn key_and_network(&self, key_id: &str) -> Res<(KeyId, frostsnap_core::MasterAppkey, Network)> {
        let k = self
            .inner
            .iter_keys()
            .find(|k| k.key_id.to_string() == key_id)
            .ok_or("unknown key")?;
        Ok((
            k.key_id,
            k.complete_key.master_appkey,
            purpose_network(&k.purpose),
        ))
    }

    pub fn wallet_params(&self, key_id: &str) -> Res<(frostsnap_core::MasterAppkey, Network)> {
        let (_, m, n) = self.key_and_network(key_id)?;
        Ok((m, n))
    }

    pub fn store_wallet(&mut self, key_id: &str, view: WalletView) -> Res<()> {
        let (id, _, _) = self.key_and_network(key_id)?;
        self.wallets.insert(id, view);
        Ok(())
    }

    pub fn wallet(&self, key_id: &str) -> Option<&WalletView> {
        let (id, _, _) = self.key_and_network(key_id).ok()?;
        self.wallets.get(&id)
    }

    /// Build a payment from the last synced wallet view and remember it for `begin_signing`.
    pub fn prepare_send(
        &mut self,
        key_id: &str,
        to: &str,
        amount: Amt,
        fee_rate: f64,
    ) -> Res<BuiltSend> {
        let (id, master, network) = self.key_and_network(key_id)?;
        let view = self.wallets.get(&id).ok_or("Refresh the wallet first.")?;
        let built = wallet::build_send(view, master, network, to, amount, fee_rate)?;
        self.prepared = Some((id, built.clone()));
        Ok(built)
    }

    pub fn esplora(&self, network: Network) -> HttpEsplora {
        HttpEsplora::new(
            self.esplora_url
                .as_deref()
                .unwrap_or(default_esplora(network)),
        )
    }
}

pub fn purpose_network(p: &KeyPurpose) -> Network {
    match p {
        KeyPurpose::Bitcoin(n) => *n,
        _ => Network::Bitcoin,
    }
}

pub fn default_esplora(network: Network) -> &'static str {
    match network {
        Network::Bitcoin => "https://mempool.space/api",
        Network::Testnet => "https://mempool.space/testnet/api",
        // Mutinynet is a custom signet; the key/address handling is identical to signet
        Network::Signet => "https://mutinynet.com/api",
        _ => "http://localhost:3002",
    }
}
