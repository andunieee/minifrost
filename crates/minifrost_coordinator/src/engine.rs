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
    coordinator::{
        BeginKeygen, CoordinatorSend, CoordinatorToUserKeyGenMessage, CoordinatorToUserMessage,
        CoordinatorToUserSigningMessage, FrostCoordinator, Mutation,
    },
    device::KeyPurpose,
    message::{signing::OpenNonceStreams, CoordinatorToDeviceMessage, DeviceToCoordinatorMessage},
    AccessStructureRef, DeviceId, KeyId, KeygenId, SignSessionId, SymmetricKey, WireSignTask,
};
use minifrost_wire::{Packet, SignPrepare};
use rand::{rngs::OsRng, RngCore};

use crate::wallet::{self, Amt, BuiltSend, HttpEsplora, WalletView};

const SNAPSHOT_VERSION: u8 = 1;
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
    KeyCreated { key_id: String, name: String },
    Signed { txid: String, tx_hex: String },
}

#[derive(Debug, Clone, Default)]
pub struct Update {
    pub message: String,
    /// QR to show to *all* signers next. `None` while there is nothing new to show.
    pub broadcast: Option<Broadcast>,
    /// Names of the signers whose reply this round is still missing.
    pub awaiting: Vec<String>,
    /// Keygen only: signers show the same code; the humans compare.
    pub check_code: Option<String>,
    pub finished: Option<Finished>,
    pub snapshot_saved: bool,
}

// ---------------------------------------------------------------------------------------------

struct KeygenSession {
    id: KeygenId,
    devices: Vec<DeviceId>,
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
        Self::from_parts(Snapshot { encryption_key: key, signers: Default::default(), history: vec![], esplora_url: None })
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
        }
    }

    fn from_snapshot(bytes: &[u8]) -> Res<Self> {
        let (&v, rest) = bytes.split_first().ok_or("empty coordinator file")?;
        if v != SNAPSHOT_VERSION {
            return Err(format!("unknown coordinator file version {v}"));
        }
        let (snap, _): (Snapshot, _) = bincode::decode_from_slice(rest, bincode::config::standard())
            .map_err(|e| format!("corrupt coordinator file: {e}"))?;
        Ok(Self::from_parts(snap))
    }

    fn snapshot_bytes(&self) -> Vec<u8> {
        let snap = Snapshot {
            encryption_key: self.encryption_key.0,
            signers: self.signers.clone(),
            history: self.history.clone(),
            esplora_url: self.esplora_url.clone(),
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
            f.write_all(&self.snapshot_bytes()).map_err(|e| e.to_string())?;
            f.sync_all().map_err(|e| e.to_string())?;
        }
        std::fs::rename(&tmp, dir.join(FILE)).map_err(|e| e.to_string())
    }

    // ------------------------------------------------------------------ signers & keys ------

    pub fn signers(&self) -> Vec<SignerInfo> {
        self.signers
            .iter()
            .map(|(id, name)| SignerInfo { device_id: id.to_string(), name: name.clone() })
            .collect()
    }

    fn name_of(&self, id: &DeviceId) -> String {
        self.signers.get(id).cloned().unwrap_or_else(|| id.to_string()[..8].to_string())
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
                        .map(|d| SignerInfo { device_id: d.to_string(), name: self.name_of(&d) })
                        .collect(),
                }
            })
            .collect()
    }

    /// A signer's "hello" QR. Returns its name.
    pub fn enroll(&mut self, payload: &[u8]) -> Res<SignerInfo> {
        match Packet::from_bytes(payload).map_err(|e| e.to_string())? {
            Packet::Hello { device_id, name } => {
                let name = name.trim().chars().take(40).collect::<String>();
                self.signers.insert(device_id, name.clone());
                self.save()?;
                Ok(SignerInfo { device_id: device_id.to_string(), name })
            }
            _ => Err("That is not a signer enrolment code.".into()),
        }
    }

    pub fn forget_signer(&mut self, device_id: &str) -> Res<()> {
        let id: DeviceId = device_id.parse().map_err(|_| "bad device id")?;
        if self.inner.iter_keys().any(|k| k.master_access_structure().contains_device(id)) {
            return Err("This signer holds a share of an existing key.".into());
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
            return Err("Another keygen or signing session is in progress. Cancel it first.".into());
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
        let sends = self.inner.begin_keygen(begin, &mut OsRng).map_err(|e| e.to_string())?;
        self.session = Session::Keygen(KeygenSession { id: keygen_id, devices: devices.clone() });
        self.replied.clear();
        let mut update = Update {
            message: format!("Show this to all {} signers and have each approve it.", devices.len()),
            broadcast: Some(self.broadcast(sends.into_iter().collect(), "Round 1 · Join key generation")),
            ..Default::default()
        };
        self.fill_awaiting(&mut update, &devices.iter().copied().collect());
        Ok(update)
    }

    // ------------------------------------------------------------------ signing -------------

    pub fn begin_signing(&mut self, key_id: &str, device_ids: Vec<String>) -> Res<Update> {
        if !matches!(self.session, Session::Idle) {
            return Err("Another keygen or signing session is in progress. Cancel it first.".into());
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
        let request = self.inner.maybe_request_nonce_replenishment(&devices, 1, &mut OsRng);
        let open: Vec<(DeviceId, OpenNonceStreams)> = request.into_open_nonce_streams().collect();
        let packet = Packet::SignPrepare(SignPrepare {
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
                title: "Step 1 · Transaction to review".into(),
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
            Session::Idle => {}
        }
        self.replied.clear();
        self.save()
    }

    // ------------------------------------------------------------------ receiving -----------

    fn fill_awaiting(&self, update: &mut Update, expected: &BTreeSet<DeviceId>) {
        update.awaiting =
            expected.iter().filter(|d| !self.replied.contains(d)).map(|d| self.name_of(d)).collect();
    }

    fn expected(&self) -> BTreeSet<DeviceId> {
        match &self.session {
            Session::Idle => Default::default(),
            Session::Keygen(k) => k.devices.iter().copied().collect(),
            Session::Signing(s) => s.devices.clone(),
        }
    }

    /// A reassembled QR payload from a signer.
    pub fn receive(&mut self, payload: &[u8]) -> Res<Update> {
        let packet = Packet::from_bytes(payload).map_err(|e| e.to_string())?;
        let (from, message) = match packet {
            Packet::FromDevice { device_id, message } => (device_id, message),
            Packet::Hello { .. } => return Err("That is an enrolment code, not a reply.".into()),
            _ => return Err("That code is meant for signers.".into()),
        };
        if matches!(self.session, Session::Idle) {
            return Err("No keygen or signing session is running.".into());
        }
        let expected = self.expected();
        if !expected.contains(&from) {
            return Err(format!("{} is not part of this session.", self.name_of(&from)));
        }
        if !self.replied.insert(from) {
            let mut u = Update { message: format!("Already have {}'s reply.", self.name_of(&from)), ..Default::default() };
            self.fill_awaiting(&mut u, &expected);
            return Ok(u);
        }
        let result = self.handle_device_message(from, message);
        if result.is_err() {
            // let the signer try again
            self.replied.remove(&from);
        }
        let mut update = result?;
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

    fn handle_device_message(&mut self, from: DeviceId, message: DeviceToCoordinatorMessage) -> Res<Update> {
        let sends = self.inner.recv_device_message(from, message).map_err(|e| e.to_string())?;
        let mut update = Update { message: format!("Got reply from {}.", self.name_of(&from)), ..Default::default() };
        let mut to_devices = vec![];
        let mut signed = None;
        let mut finalize = None;
        for send in sends {
            match send {
                CoordinatorSend::ToDevice { .. } => to_devices.push(send),
                CoordinatorSend::ToUser(u) => match u {
                    CoordinatorToUserMessage::KeyGen { keygen_id, inner } => match inner {
                        CoordinatorToUserKeyGenMessage::CheckKeyGen { session_hash } => {
                            update.check_code = Some(minifrost_wire::format_check_code(&session_hash.0));
                        }
                        CoordinatorToUserKeyGenMessage::KeyGenAck { all_acks_received: true, .. } => {
                            finalize = Some(keygen_id);
                        }
                        _ => {}
                    },
                    CoordinatorToUserMessage::Signing(CoordinatorToUserSigningMessage::Signed {
                        signatures, ..
                    }) => signed = Some(signatures),
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
            let name = self.inner.get_frost_key(key_id).map(|k| k.key_name.clone()).unwrap_or_default();
            to_devices.extend(fin);
            self.session = Session::Idle;
            self.replied.clear();
            update.finished = Some(Finished::KeyCreated { key_id: key_id.to_string(), name: name.clone() });
            update.message = format!("Key “{name}” created. Show the last code so every signer stores its share.");
        }

        if !to_devices.is_empty() {
            let title = match (&self.session, update.finished.is_some()) {
                (_, true) => "Final · Store the key".to_string(),
                (Session::Keygen(_), _) if update.check_code.is_some() => "Round 3 · Compare the check code".to_string(),
                (Session::Keygen(_), _) => "Round 2 · Certify".to_string(),
                _ => "Step 2 · Aggregated nonces".to_string(),
            };
            update.broadcast = Some(self.broadcast(to_devices, &title));
            if !matches!(self.session, Session::Idle) {
                self.replied.clear();
                update.message = match &self.session {
                    Session::Keygen(_) if update.check_code.is_some() => {
                        "Every signer must show the same check code as below. If so, approve on each signer.".into()
                    }
                    Session::Keygen(_) => "Show this to all signers; each answers automatically.".into(),
                    _ => "Show this to the signers. Each verifies the transaction and shows a partial signature.".into(),
                };
            }
        }

        // signing: once every chosen signer returned nonces, build the bundle
        if let Session::Signing(s) = &self.session {
            if s.session_id.is_none() && self.replied.len() == s.devices.len() {
                let bundle = self.start_signing_round()?;
                update.broadcast = Some(bundle);
                update.message = "All nonces received. Show this bundle to the signers.".into();
                self.replied.clear();
            }
        }

        if let Some(sigs) = signed {
            let Session::Signing(s) = std::mem::replace(&mut self.session, Session::Idle) else {
                return Err("signature completed without a signing session".into());
            };
            let master = self.inner.get_frost_key(s.key_id).ok_or("key vanished")?.complete_key.master_appkey;
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

    fn broadcast(&self, sends: Vec<CoordinatorSend>, title: &str) -> Broadcast {
        let entries: Vec<(Vec<DeviceId>, CoordinatorToDeviceMessage)> = sends
            .into_iter()
            .filter_map(|s| match s {
                CoordinatorSend::ToDevice { message, destinations } => {
                    Some((destinations.into_iter().collect(), message))
                }
                _ => None,
            })
            .collect();
        let bytes = Packet::ToDevices(entries).to_bytes();
        Broadcast { title: title.to_string(), frames: qr_transport::split(&bytes, qr_transport::DEFAULT_FRAME_CHARS) }
    }

    fn start_signing_round(&mut self) -> Res<Broadcast> {
        let Session::Signing(s) = &mut self.session else { unreachable!() };
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
            .map(|d| self.inner.request_device_sign(session_id, d, self.encryption_key).into())
            .collect();
        Ok(self.broadcast(sends, "Step 2 · Aggregated nonces"))
    }

    // ------------------------------------------------------------------ wallet --------------

    fn key_and_network(&self, key_id: &str) -> Res<(KeyId, frostsnap_core::MasterAppkey, Network)> {
        let k = self.inner.iter_keys().find(|k| k.key_id.to_string() == key_id).ok_or("unknown key")?;
        Ok((k.key_id, k.complete_key.master_appkey, purpose_network(&k.purpose)))
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
    pub fn prepare_send(&mut self, key_id: &str, to: &str, amount: Amt, fee_rate: f64) -> Res<BuiltSend> {
        let (id, master, network) = self.key_and_network(key_id)?;
        let view = self.wallets.get(&id).ok_or("Refresh the wallet first.")?;
        let built = wallet::build_send(view, master, network, to, amount, fee_rate)?;
        self.prepared = Some((id, built.clone()));
        Ok(built)
    }

    pub fn esplora(&self, network: Network) -> HttpEsplora {
        HttpEsplora::new(self.esplora_url.as_deref().unwrap_or(default_esplora(network)))
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
        Network::Signet => "https://mempool.space/signet/api",
        _ => "http://localhost:3002",
    }
}
