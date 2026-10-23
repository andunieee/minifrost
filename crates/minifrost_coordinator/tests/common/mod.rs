//! Helpers shared by the integration tests: everything goes over QR frames only, the way a
//! camera would feed the apps.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::str::FromStr;

use bitcoin::{secp256k1, Address, Txid};
use minifrost_coordinator::engine::{Coordinator, Finished, Update};
use minifrost_coordinator::wallet::{self, Amt, Esplora, EsploraUtxo};
// Compile the signer engine straight into the test instead of linking the signer crate: both FFI
// crates export the same flutter_rust_bridge symbols, so a binary can't link both.
#[path = "../../../minifrost_signer/src/engine.rs"]
pub mod signer_engine;
pub use signer_engine::{Prompt, Signer, Step};

/// What a camera would do: feed frames (shuffled, repeated) to the assembler.
pub fn scan(frames: &[String]) -> Vec<u8> {
    let mut asm = qr_transport::Assembler::new();
    let mut order: Vec<&String> = frames.iter().collect();
    order.reverse();
    for f in order.iter().chain(order.iter()) {
        if let Some(bytes) = asm.push(f).unwrap().complete {
            return bytes;
        }
    }
    panic!("frames never completed");
}

pub struct Chain(pub BTreeMap<String, Vec<EsploraUtxo>>);
impl Esplora for Chain {
    fn address_used(&self, a: &Address) -> Result<bool, String> {
        Ok(self.0.contains_key(&a.to_string()))
    }
    fn address_utxos(&self, a: &Address) -> Result<Vec<EsploraUtxo>, String> {
        Ok(self.0.get(&a.to_string()).cloned().unwrap_or_default())
    }
    fn fee_rate(&self, _: u32) -> Result<f64, String> {
        Ok(3.0)
    }
    fn broadcast(&self, _: &str) -> Result<String, String> {
        unreachable!()
    }
}

pub fn frames_of(update: &Update) -> Vec<String> {
    update.broadcast.as_ref().expect("a broadcast").frames.clone()
}

/// Run one signer through a coordinator broadcast.
pub fn signer_step(s: &mut Signer, frames: &[String]) -> Step {
    s.receive(&scan(frames)).unwrap().step
}

pub fn approve_to_reply(s: &mut Signer) -> Vec<String> {
    match s.approve().unwrap().step {
        Step::Reply { frames, .. } => frames,
        other => panic!("expected reply, got {other:?}"),
    }
}

/// Nonce release and partial signing burn nonces: the snapshot must come with the reply.
pub fn approve_persisting(s: &mut Signer) -> Vec<String> {
    let r = s.approve().unwrap();
    assert!(r.snapshot.is_some(), "burned nonces must be persisted before the QR is shown");
    match r.step {
        Step::Reply { frames, .. } => frames,
        other => panic!("expected reply, got {other:?}"),
    }
}

pub fn auto_reply(s: &mut Signer, frames: &[String]) -> Vec<String> {
    match signer_step(s, frames) {
        Step::Reply { frames, .. } => frames,
        other => panic!("expected automatic reply, got {other:?}"),
    }
}

/// Drive a keygen from the coordinator's first broadcast to the last "check code" acks.
pub fn run_keygen(coord: &mut Coordinator, signers: &mut [Signer], first: Update) -> Update {
    let mut up = first;
    let b1 = frames_of(&up);
    let replies: Vec<_> = signers
        .iter_mut()
        .map(|s| {
            assert!(matches!(signer_step(s, &b1), Step::Confirm(Prompt::JoinKeygen { .. })));
            approve_to_reply(s)
        })
        .collect();
    for r in &replies {
        up = coord.receive(&scan(r)).unwrap();
    }
    let b2 = frames_of(&up);
    let replies: Vec<_> = signers.iter_mut().map(|s| auto_reply(s, &b2)).collect();
    for r in &replies {
        up = coord.receive(&scan(r)).unwrap();
    }
    let b3 = frames_of(&up);
    let acks: Vec<_> = signers
        .iter_mut()
        .map(|s| {
            assert!(matches!(signer_step(s, &b3), Step::Confirm(Prompt::VerifyKeygen { .. })));
            approve_to_reply(s)
        })
        .collect();
    for a in &acks {
        up = coord.receive(&scan(a)).unwrap();
    }
    up
}

/// Enrol fresh signers named `names`, create a `threshold`-of-n signet wallet over QR, store the
/// shares. Returns the signers and the wallet's key id.
pub fn create_wallet(coord: &mut Coordinator, names: &[&str], threshold: u16) -> (Vec<Signer>, String) {
    let mut signers: Vec<Signer> = names.iter().map(|n| Signer::create(n.to_string())).collect();
    for s in &signers {
        coord.enroll(&scan(&s.hello_frames())).unwrap();
    }
    let ids: Vec<String> = signers.iter().map(|s| s.device_id().to_string()).collect();
    let up = coord.start_keygen("family".into(), threshold, ids, bitcoin::Network::Signet).unwrap();
    let mut up = run_keygen(coord, &mut signers, up);
    let Some(Finished::KeyCreated { key_id, .. }) = up.finished.take() else { panic!("{up:?}") };
    for s in signers.iter_mut() {
        let r = s.receive(&scan(&frames_of(&up))).unwrap();
        assert!(matches!(r.step, Step::Done { .. }));
        assert!(r.snapshot.is_some());
    }
    (signers, key_id)
}

/// Fund the wallet on a fake chain, pay 40k sats with the signers at `chosen`, return the signed
/// transaction after verifying its BIP340 signature against the wallet's own address.
pub fn fund_and_sign(coord: &mut Coordinator, signers: &mut [Signer], chosen: &[usize], key_id: &str) -> String {
    let (master, network) = coord.wallet_params(key_id).unwrap();
    let mut chain = Chain(BTreeMap::new());
    chain.0.insert(
        wallet::address(master, network, false, 0).to_string(),
        vec![EsploraUtxo { txid: Txid::from_str(&"11".repeat(32)).unwrap(), vout: 1, value: 100_000, confirmed: true }],
    );
    coord.store_wallet(key_id, wallet::sync(&chain, master, network).unwrap()).unwrap();
    let dest = wallet::address(master, network, false, 50).to_string();
    let built = coord.prepare_send(key_id, &dest, Amt::Sats(40_000), 3.0).unwrap();
    let template = built.template.clone();

    let chosen_ids: Vec<String> = chosen.iter().map(|&i| signers[i].device_id().to_string()).collect();
    let up = coord.begin_signing(key_id, chosen_ids).unwrap();
    let prepare = frames_of(&up);
    let mut nonce_replies = vec![];
    for &i in chosen {
        assert!(matches!(signer_step(&mut signers[i], &prepare), Step::Confirm(Prompt::ReviewTransaction(_))));
        nonce_replies.push(approve_persisting(&mut signers[i]));
    }
    let mut up = Update::default();
    for r in &nonce_replies {
        up = coord.receive(&scan(r)).unwrap();
    }
    let bundle = frames_of(&up);
    let mut sigs = vec![];
    for &i in chosen {
        assert!(matches!(signer_step(&mut signers[i], &bundle), Step::Confirm(Prompt::SignTransaction(_))));
        sigs.push(approve_persisting(&mut signers[i]));
    }
    let mut up = Update::default();
    for s in &sigs {
        up = coord.receive(&scan(s)).unwrap();
    }
    let Some(Finished::Signed { tx_hex, .. }) = up.finished else { panic!("{up:?}") };

    let tx: bitcoin::Transaction = bitcoin::consensus::encode::deserialize_hex(&tx_hex).unwrap();
    let secp = secp256k1::Secp256k1::verification_only();
    for (msg, input) in template.iter_sighash().zip(tx.input.iter()) {
        let sig = bitcoin::taproot::Signature::from_slice(input.witness.nth(0).unwrap()).unwrap();
        let prev_spk = template.inputs()[0].txout().script_pubkey;
        let xonly = secp256k1::XOnlyPublicKey::from_slice(&prev_spk.as_bytes()[2..34]).unwrap();
        let m = secp256k1::Message::from_digest({
            use bitcoin::hashes::Hash;
            msg.to_byte_array()
        });
        secp.verify_schnorr(&sig.signature, &m, &xonly).expect("valid BIP340 signature");
    }
    tx_hex
}
