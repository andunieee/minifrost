//! The signer state machine. Pure Rust, no I/O: bytes in, a [`Step`] out. The Flutter app owns the
//! camera, the screen and the encrypted-at-rest storage; this module owns every decision.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use bitcoin::hashes::{sha256, Hash, HashEngine, Hmac, HmacEngine};
use frostsnap_core::{
    device::{
        DeviceSecretDerivation, DeviceToUserMessage, FrostSigner, KeyGenPhase3, KeyPurpose,
        Mutation, SignPhase1,
    },
    device_nonces::{AbSlots, NonceStreamSlot, SecretNonceSlot},
    message::{
        keygen::Keygen, signing::CoordinatorSigning, CoordinatorToDeviceMessage, DeviceSend,
        DeviceToCoordinatorMessage,
    },
    nonce_stream::NonceStreamId,
    schnorr_fun::fun::{marker::*, KeyPair, Scalar},
    AccessStructureRef, CoordShareDecryptionContrib, DeviceId, KeyId, SymmetricKey, Versioned,
    WireSignTask,
};
use minifrost_wire::Packet;
use rand::rngs::OsRng;

const NONCE_SLOTS: usize = 4;
const SNAPSHOT_VERSION: u8 = 1;

// ---------------------------------------------------------------------------------------------
// Persistent nonce slots
// ---------------------------------------------------------------------------------------------

/// A nonce slot the snapshot code can read back. FrostSigner owns one clone, we keep the other.
#[derive(Clone, Debug, Default)]
pub struct SharedSlot(Arc<Mutex<Option<Versioned<SecretNonceSlot>>>>);

impl NonceStreamSlot for SharedSlot {
    fn read_slot_versioned(&mut self) -> Option<Versioned<SecretNonceSlot>> {
        self.0.lock().unwrap().clone()
    }
    fn write_slot_versioned(&mut self, value: Versioned<&SecretNonceSlot>) {
        *self.0.lock().unwrap() = Some(value.cloned());
    }
}

// ---------------------------------------------------------------------------------------------
// Secret derivation
// ---------------------------------------------------------------------------------------------

/// Everything secret on this signer derives from one 32-byte random `device_secret`. The share is
/// encrypted under a key that mixes in a contribution only the coordinator can supply, so
/// stealing this signer's storage alone is not enough to sign.
#[derive(Clone)]
struct Derivation {
    device_secret: [u8; 32],
}

fn hmac(key: &[u8], parts: &[&[u8]]) -> [u8; 32] {
    let mut engine = HmacEngine::<sha256::Hash>::new(key);
    for p in parts {
        engine.input(p);
    }
    *Hmac::<sha256::Hash>::from_engine(engine).as_byte_array()
}

impl Derivation {
    fn keypair(&self) -> KeyPair {
        let bytes = hmac(&self.device_secret, &[b"minifrost/device-identity-key"]);
        let scalar = Scalar::<Secret, Zero>::from_bytes_mod_order(bytes)
            .non_zero()
            .expect("hmac output is not zero");
        KeyPair::new(scalar)
    }
}

impl DeviceSecretDerivation for Derivation {
    fn get_share_encryption_key(
        &mut self,
        access_structure_ref: AccessStructureRef,
        party_index: frostsnap_core::schnorr_fun::frost::ShareIndex,
        coord_key: CoordShareDecryptionContrib,
    ) -> SymmetricKey {
        let ctx = bincode::encode_to_vec(
            (access_structure_ref, party_index, coord_key),
            bincode::config::standard(),
        )
        .expect("encodes");
        SymmetricKey(hmac(
            &hmac(&self.device_secret, &[b"minifrost/share-encryption-key"]),
            &[&ctx],
        ))
    }

    fn derive_nonce_seed(
        &mut self,
        nonce_stream_id: NonceStreamId,
        index: u32,
        seed_material: &[u8; 32],
    ) -> [u8; 32] {
        hmac(
            &hmac(&self.device_secret, &[b"minifrost/nonce-seed"]),
            &[
                nonce_stream_id.to_bytes().as_slice(),
                &index.to_le_bytes(),
                seed_material,
            ],
        )
    }
}

// ---------------------------------------------------------------------------------------------
// Snapshot (what is stored, encrypted, on the phone)
// ---------------------------------------------------------------------------------------------

#[derive(bincode::Encode, bincode::Decode)]
struct Snapshot {
    device_secret: [u8; 32],
    name: String,
    history: Vec<Mutation>,
    nonce_slots: Vec<Option<Versioned<SecretNonceSlot>>>,
    /// transactions the user approved in round 1 and has not signed yet: (key id, txid)
    reviewed: Vec<([u8; 32], [u8; 32])>,
}

// ---------------------------------------------------------------------------------------------
// What the UI sees
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct OutputSummary {
    pub address: String,
    pub amount_sats: i64,
    /// `Some("Change #2")` when this output pays back into this very wallet.
    pub own_label: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TxSummary {
    pub key_name: String,
    pub network: String,
    pub txid: String,
    pub n_inputs: u32,
    pub outputs: Vec<OutputSummary>,
    pub fee_sats: i64,
    pub fee_rate: Option<f64>,
    /// `true` once the signer verified the outputs against its own key (final round only). In the
    /// preview round every output is shown as the coordinator claims it.
    pub verified: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum Prompt {
    /// The coordinator wants to create a new key with this signer.
    JoinKeygen {
        key_name: String,
        threshold: u16,
        n_signers: u16,
        network: String,
    },
    /// All signers must show the same code. Compare them before approving.
    VerifyKeygen {
        key_name: String,
        threshold: u16,
        n_signers: u16,
        check_code: String,
    },
    /// Round 1: look at the transaction; approving releases fresh nonces.
    ReviewTransaction(TxSummary),
    /// Round 2: final, verified view. Approving releases the partial signature.
    SignTransaction(TxSummary),
}

#[derive(Debug, Clone)]
pub enum Step {
    /// Packet was valid but needs nothing from this signer.
    Ignored { reason: String },
    /// Ask the user, then call `approve` or `reject`.
    Confirm(Prompt),
    /// Show these frames as a looping QR code for the coordinator to scan.
    Reply { frames: Vec<String>, what: String },
    /// Finished locally; nothing to show the coordinator.
    Done { message: String },
}

#[derive(Debug, Clone)]
pub struct StepResult {
    pub step: Step,
    /// If `Some`, the signer's persistent state changed. The caller MUST durably store these bytes
    /// before showing `step`: after signing, they contain the burned nonce.
    pub snapshot: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct KeyInfo {
    pub key_id: String,
    pub name: String,
    pub threshold: u16,
    pub network: String,
}

enum Pending {
    Keygen(CoordinatorToDeviceMessage),
    CheckKeygen(Box<KeyGenPhase3>),
    Nonces(CoordinatorToDeviceMessage, bitcoin::Txid, KeyId),
    Sign(Box<SignPhase1>),
}

pub struct Signer {
    inner: FrostSigner<SharedSlot>,
    slots: Vec<SharedSlot>,
    derivation: Derivation,
    name: String,
    history: Vec<Mutation>,
    pending: Option<Pending>,
    /// transactions the user reviewed in round 1, by key
    reviewed: BTreeMap<KeyId, bitcoin::Txid>,
    dirty: bool,
}

fn build(
    device_secret: [u8; 32],
    name: String,
    history: Vec<Mutation>,
    slot_values: Vec<Option<Versioned<SecretNonceSlot>>>,
    reviewed: Vec<([u8; 32], [u8; 32])>,
) -> Signer {
    let slots: Vec<SharedSlot> = (0..NONCE_SLOTS)
        .map(|i| SharedSlot(Arc::new(Mutex::new(slot_values.get(i).cloned().flatten()))))
        .collect();
    let derivation = Derivation { device_secret };
    let mut inner = FrostSigner::new(derivation.keypair(), AbSlots::new(slots.clone()));
    for m in &history {
        inner.apply_mutation(m.clone());
    }
    Signer {
        inner,
        slots,
        derivation,
        name,
        history,
        pending: None,
        reviewed: reviewed
            .into_iter()
            .map(|(k, t)| (KeyId(k), bitcoin::Txid::from_byte_array(t)))
            .collect(),
        dirty: false,
    }
}

type Res<T> = Result<T, String>;

impl Signer {
    /// First launch.
    pub fn create(name: String) -> Signer {
        use rand::RngCore;
        let mut secret = [0u8; 32];
        OsRng.fill_bytes(&mut secret);
        let mut s = build(secret, name, vec![], vec![], vec![]);
        s.dirty = true;
        s
    }

    pub fn from_snapshot(bytes: &[u8]) -> Res<Signer> {
        let (&version, rest) = bytes.split_first().ok_or("empty snapshot")?;
        if version != SNAPSHOT_VERSION {
            return Err(format!("unknown snapshot version {version}"));
        }
        let (snap, _): (Snapshot, _) = bincode::decode_from_slice(rest, bincode::config::standard())
            .map_err(|e| format!("corrupt snapshot: {e}"))?;
        Ok(build(snap.device_secret, snap.name, snap.history, snap.nonce_slots, snap.reviewed))
    }

    pub fn snapshot(&self) -> Vec<u8> {
        let snap = Snapshot {
            device_secret: self.derivation.device_secret,
            name: self.name.clone(),
            history: self.history.clone(),
            nonce_slots: self.slots.iter().map(|s| s.0.lock().unwrap().clone()).collect(),
            reviewed: self.reviewed.iter().map(|(k, t)| (k.0, t.to_byte_array())).collect(),
        };
        let mut out = vec![SNAPSHOT_VERSION];
        out.extend(bincode::encode_to_vec(&snap, bincode::config::standard()).expect("encodes"));
        out
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn device_id(&self) -> DeviceId {
        self.inner.device_id()
    }

    /// Frames of the QR code that enrols this signer with a coordinator.
    pub fn hello_frames(&self) -> Vec<String> {
        let bytes = Packet::Hello { device_id: self.device_id(), name: self.name.clone() }.to_bytes();
        qr_transport::split(&bytes, qr_transport::DEFAULT_FRAME_CHARS)
    }

    pub fn keys(&self) -> Vec<KeyInfo> {
        use frostsnap_core::device::keys::KeyMutation::*;
        let mut out: BTreeMap<KeyId, KeyInfo> = BTreeMap::new();
        for m in &self.history {
            if let Mutation::Keygen(k) = m {
                match k {
                    NewKey { key_id, key_name, purpose } => {
                        out.insert(
                            *key_id,
                            KeyInfo {
                                key_id: key_id.to_string(),
                                name: key_name.clone(),
                                threshold: 0,
                                network: network_name(purpose),
                            },
                        );
                    }
                    NewAccessStructure { access_structure_ref, threshold, .. } => {
                        if let Some(k) = out.get_mut(&access_structure_ref.key_id) {
                            k.threshold = *threshold;
                        }
                    }
                    SaveShare(_) => {}
                }
            }
        }
        out.into_values().collect()
    }

    fn take_snapshot(&mut self) -> Option<Vec<u8>> {
        // pull newly staged mutations into our durable history
        let staged: Vec<Mutation> = self.inner.staged_mutations().drain(..).collect();
        if !staged.is_empty() {
            self.dirty = true;
            self.history.extend(staged);
        }
        std::mem::take(&mut self.dirty).then(|| self.snapshot())
    }

    fn result(&mut self, step: Step) -> StepResult {
        // Nonce slots change inside sign_ack without staged mutations, so a signing step always
        // forces a snapshot.
        StepResult { snapshot: self.take_snapshot(), step }
    }

    // -----------------------------------------------------------------------------------------

    /// Feed in a fully reassembled QR payload.
    pub fn receive(&mut self, payload: &[u8]) -> Res<StepResult> {
        let packet = Packet::from_bytes(payload).map_err(|e| e.to_string())?;
        let me = self.device_id();
        let step = match packet {
            Packet::Hello { .. } | Packet::FromDevice { .. } => Step::Ignored {
                reason: "This code is meant for a coordinator, not for a signer.".into(),
            },
            Packet::SignPrepare(prep) => self.receive_prepare(me, prep)?,
            Packet::ToDevices(entries) => {
                match entries.into_iter().find(|(to, _)| to.contains(&me)) {
                    None => Step::Ignored { reason: "This code is not addressed to this signer.".into() },
                    Some((_, msg)) => self.receive_message(msg)?,
                }
            }
        };
        Ok(self.result(step))
    }

    fn receive_prepare(&mut self, me: DeviceId, prep: minifrost_wire::SignPrepare) -> Res<Step> {
        let Some((_, open)) = prep.open_nonce_streams.into_iter().find(|(d, _)| *d == me) else {
            return Ok(Step::Ignored { reason: "This signing request is not addressed to this signer.".into() });
        };
        let key = self
            .keys()
            .into_iter()
            .find(|k| k.key_id == prep.key_id.to_string())
            .ok_or("This signer does not hold the key being used.")?;
        let WireSignTask::BitcoinTransaction(template) = &prep.sign_task else {
            return Err("Only Bitcoin transactions can be signed by this app.".into());
        };
        let network = self
            .inner
            .wallet_network(prep.key_id)
            .ok_or("This key is not a Bitcoin key.")?;
        let txid = template.txid();
        let summary = preview_summary(&key.name, network, template);
        let msg = CoordinatorToDeviceMessage::Signing(CoordinatorSigning::OpenNonceStreams(open));
        self.pending = Some(Pending::Nonces(msg, txid, prep.key_id));
        Ok(Step::Confirm(Prompt::ReviewTransaction(summary)))
    }

    fn receive_message(&mut self, msg: CoordinatorToDeviceMessage) -> Res<Step> {
        match &msg {
            CoordinatorToDeviceMessage::KeyGen(Keygen::Begin(begin)) => {
                let network = match begin.purpose {
                    KeyPurpose::Bitcoin(n) => n.to_string(),
                    _ => return Err("Only Bitcoin keys are supported.".into()),
                };
                let prompt = Prompt::JoinKeygen {
                    key_name: begin.key_name.clone(),
                    threshold: begin.threshold,
                    n_signers: begin.devices.len() as u16,
                    network,
                };
                self.pending = Some(Pending::Keygen(msg));
                Ok(Step::Confirm(prompt))
            }
            CoordinatorToDeviceMessage::Signing(CoordinatorSigning::RequestSign(_)) => {
                let sends = self.inner.recv_coordinator_message(msg, &mut OsRng).map_err(|e| e.to_string())?;
                for send in sends {
                    if let DeviceSend::ToUser(u) = send {
                        if let DeviceToUserMessage::SignatureRequest { phase } = *u {
                            return self.prompt_final_sign(phase);
                        }
                    }
                }
                Err("Signing request was not accepted.".into())
            }
            _ => {
                // CertifyPlease / Check / Finalize
                let sends = self.inner.recv_coordinator_message(msg, &mut OsRng).map_err(|e| e.to_string())?;
                self.handle_sends(sends)
            }
        }
    }

    fn prompt_final_sign(&mut self, phase: Box<SignPhase1>) -> Res<Step> {
        use frostsnap_core::SignTask;
        let task = phase.sign_task();
        let SignTask::BitcoinTransaction { tx_template, network } = &task.inner else {
            return Err("Only Bitcoin transactions can be signed by this app.".into());
        };
        let key_id = phase.sign_task().master_appkey.key_id();
        let key_name = self
            .keys()
            .into_iter()
            .find(|k| k.key_id == key_id.to_string())
            .map(|k| k.name)
            .ok_or("unknown key")?;
        let txid = tx_template.txid();
        let mut warnings = vec![];
        match self.reviewed.get(&key_id) {
            Some(seen) if *seen != txid => {
                return Err(
                    "This is NOT the transaction you reviewed in the previous step. Refusing to sign."
                        .into(),
                )
            }
            Some(_) => {}
            None => warnings.push(
                "This signer did not review this transaction in the previous step (it was restarted or skipped it)."
                    .to_string(),
            ),
        }
        let summary = verified_summary(&key_name, *network, tx_template, txid, warnings);
        self.pending = Some(Pending::Sign(phase));
        Ok(Step::Confirm(Prompt::SignTransaction(summary)))
    }

    fn handle_sends(&mut self, sends: Vec<DeviceSend>) -> Res<Step> {
        let me = self.device_id();
        let mut step = None;
        for send in sends {
            match send {
                DeviceSend::ToCoordinator(m) => {
                    step = Some(reply(me, *m, "Show this to the coordinator"));
                }
                DeviceSend::ToUser(u) => match *u {
                    DeviceToUserMessage::CheckKeyGen { phase } => {
                        let (t, n) = phase.t_of_n();
                        let prompt = Prompt::VerifyKeygen {
                            key_name: phase.key_name().to_string(),
                            threshold: t,
                            n_signers: n,
                            check_code: minifrost_wire::format_check_code(&phase.session_hash().0),
                        };
                        self.pending = Some(Pending::CheckKeygen(phase));
                        step = Some(Step::Confirm(prompt));
                    }
                    DeviceToUserMessage::FinalizeKeyGen { key_name } => {
                        step = Some(Step::Done {
                            message: format!("Key “{key_name}” was created and its share stored securely."),
                        });
                    }
                    _ => {}
                },
            }
        }
        step.ok_or_else(|| "Nothing to do for that message.".to_string())
    }

    /// The user tapped "approve" on the current [`Prompt`].
    pub fn approve(&mut self) -> Res<StepResult> {
        let me = self.device_id();
        let pending = self.pending.take().ok_or("nothing is waiting for approval")?;
        let step = match pending {
            Pending::Keygen(msg) => {
                let sends = self.inner.recv_coordinator_message(msg, &mut OsRng).map_err(|e| e.to_string())?;
                self.handle_sends(sends)?
            }
            Pending::CheckKeygen(phase) => {
                let ack = self
                    .inner
                    .keygen_ack(*phase, &mut self.derivation, &mut OsRng)
                    .map_err(|e| e.to_string())?;
                reply(me, DeviceToCoordinatorMessage::KeyGen(frostsnap_core::message::keygen::DeviceKeygen::Ack(ack)), "Show this to the coordinator")
            }
            Pending::Nonces(msg, txid, key_id) => {
                let sends = self.inner.recv_coordinator_message(msg, &mut OsRng).map_err(|e| e.to_string())?;
                let mut segments = vec![];
                for send in sends {
                    match send {
                        DeviceSend::ToUser(u) => {
                            if let DeviceToUserMessage::NonceJobs(mut batch) = *u {
                                batch.run_until_finished(&mut self.derivation);
                                segments.extend(batch.into_segments());
                            }
                        }
                        DeviceSend::ToCoordinator(m) => {
                            // "no new nonces needed" answer
                            let _ = m;
                        }
                    }
                }
                self.reviewed.insert(key_id, txid);
                self.dirty = true; // nonce slots changed
                reply(
                    me,
                    DeviceToCoordinatorMessage::Signing(
                        frostsnap_core::message::signing::DeviceSigning::NonceResponse { segments },
                    ),
                    "Nonces — show this to the coordinator",
                )
            }
            Pending::Sign(phase) => {
                let key_id = phase.sign_task().master_appkey.key_id();
                let sends = self
                    .inner
                    .sign_ack(*phase, &mut self.derivation)
                    .map_err(|e| e.to_string())?;
                self.reviewed.remove(&key_id);
                self.dirty = true; // a nonce was burned
                let m = sends
                    .into_iter()
                    .find_map(|s| match s {
                        DeviceSend::ToCoordinator(m) => Some(*m),
                        _ => None,
                    })
                    .ok_or("signing produced no signature share")?;
                reply(me, m, "Partial signature — show this to the coordinator")
            }
        };
        Ok(self.result(step))
    }

    pub fn reject(&mut self) {
        self.pending = None;
        self.inner.clear_unfinished_keygens();
    }
}

fn network_name(p: &KeyPurpose) -> String {
    match p {
        KeyPurpose::Bitcoin(n) => n.to_string(),
        KeyPurpose::Test => "test".into(),
        _ => "other".into(),
    }
}

fn reply(me: DeviceId, message: DeviceToCoordinatorMessage, what: &str) -> Step {
    let bytes = Packet::FromDevice { device_id: me, message }.to_bytes();
    Step::Reply {
        frames: qr_transport::split(&bytes, qr_transport::DEFAULT_FRAME_CHARS),
        what: what.to_string(),
    }
}

fn preview_summary(
    key_name: &str,
    network: bitcoin::Network,
    template: &frostsnap_core::bitcoin_transaction::TransactionTemplate,
) -> TxSummary {
    let outputs = template
        .outputs()
        .iter()
        .map(|o| OutputSummary {
            address: bitcoin::Address::from_script(&o.owner.spk(), network)
                .map(|a| a.to_string())
                .unwrap_or_else(|_| "(non-standard script)".into()),
            amount_sats: o.value as i64,
            own_label: None,
        })
        .collect();
    TxSummary {
        key_name: key_name.to_string(),
        network: network.to_string(),
        txid: template.txid().to_string(),
        n_inputs: template.inputs().len() as u32,
        outputs,
        fee_sats: template.fee().unwrap_or(0) as i64,
        fee_rate: None,
        verified: false,
        warnings: vec![
            "Preview: outputs are shown as the coordinator presents them. Ownership of change is verified in the final step.".into(),
        ],
    }
}

fn verified_summary(
    key_name: &str,
    network: bitcoin::Network,
    template: &frostsnap_core::bitcoin_transaction::TransactionTemplate<frostsnap_core::bitcoin_transaction::ScopedTo>,
    txid: bitcoin::Txid,
    mut warnings: Vec<String>,
) -> TxSummary {
    let prompt = template.user_prompt(network);
    let outputs = prompt
        .recipients
        .iter()
        .map(|r| OutputSummary {
            address: match &r.destination {
                frostsnap_core::bitcoin_transaction::PromptDestination::Address(a) => a.to_string(),
                frostsnap_core::bitcoin_transaction::PromptDestination::UnrecognizedScript(s) => {
                    format!("(non-standard script {})", s.to_hex_string())
                }
            },
            amount_sats: r.amount.to_sat() as i64,
            own_label: r.owned.map(|p| p.label()),
        })
        .collect();
    if let Some(rate) = prompt.fee_rate_sats_per_vbyte {
        if rate > 500.0 {
            warnings.push(format!("Very high fee rate: {rate:.0} sat/vB"));
        }
    }
    if prompt.fee.to_sat() > prompt.value_at_risk().to_sat() / 10 && prompt.value_at_risk().to_sat() > 0 {
        warnings.push("The fee is more than 10% of the amount being sent.".into());
    }
    TxSummary {
        key_name: key_name.to_string(),
        network: network.to_string(),
        txid: txid.to_string(),
        n_inputs: template.inputs().len() as u32,
        outputs,
        fee_sats: prompt.fee.to_sat() as i64,
        fee_rate: prompt.fee_rate_sats_per_vbyte,
        verified: true,
        warnings,
    }
}
