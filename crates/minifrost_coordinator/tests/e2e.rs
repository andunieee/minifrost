//! Whole flow over QR frames only: enrol 3 signers, 2-of-3 keygen, fund the wallet on a fake
//! chain, sign with 2 signers (one of them restarted from its stored snapshot in between), verify
//! the final transaction's Schnorr signature, and refuse a tampered round-2 request.

use std::collections::BTreeMap;
use std::str::FromStr;

use bitcoin::{secp256k1, Address, Network, Txid};
use minifrost_coordinator::engine::{Coordinator, Finished, Update};
use minifrost_coordinator::wallet::{self, Amt, Esplora, EsploraUtxo};
// Compile the signer engine straight into this test instead of linking the signer crate: both
// FFI crates export the same flutter_rust_bridge symbols, so a binary can't link both.
#[path = "../../minifrost_signer/src/engine.rs"]
mod signer_engine;
use signer_engine::{Prompt, Signer, Step};

/// What a camera would do: feed frames (shuffled, repeated) to the assembler.
fn scan(frames: &[String]) -> Vec<u8> {
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

struct Chain(BTreeMap<String, Vec<EsploraUtxo>>);
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

fn frames_of(update: &Update) -> Vec<String> {
    update.broadcast.as_ref().expect("a broadcast").frames.clone()
}

/// Run one signer through a coordinator broadcast; `approve` decides whether the user taps OK.
fn signer_step(s: &mut Signer, frames: &[String]) -> Step {
    s.receive(&scan(frames)).unwrap().step
}

fn approve_to_reply(s: &mut Signer) -> Vec<String> {
    let r = s.approve().unwrap();
    match r.step {
        Step::Reply { frames, .. } => frames,
        other => panic!("expected reply, got {other:?}"),
    }
}

/// Nonce release and partial signing burn nonces: the snapshot must come with the reply.
fn approve_persisting(s: &mut Signer) -> Vec<String> {
    let r = s.approve().unwrap();
    assert!(r.snapshot.is_some(), "burned nonces must be persisted before the QR is shown");
    match r.step {
        Step::Reply { frames, .. } => frames,
        other => panic!("expected reply, got {other:?}"),
    }
}

fn auto_reply(s: &mut Signer, frames: &[String]) -> Vec<String> {
    match signer_step(s, frames) {
        Step::Reply { frames, .. } => frames,
        other => panic!("expected automatic reply, got {other:?}"),
    }
}

#[test]
fn keygen_and_sign_over_qr_only() {
    let dir = std::env::temp_dir().join(format!("minifrost-e2e-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut coord = Coordinator::open(&dir).unwrap();
    let mut signers: Vec<Signer> =
        ["alice", "bob", "carol"].iter().map(|n| Signer::create(n.to_string())).collect();

    // --- enrol
    for s in &signers {
        coord.enroll(&scan(&s.hello_frames())).unwrap();
    }
    let ids: Vec<String> = coord.signers().into_iter().map(|s| s.device_id).collect();
    assert_eq!(ids.len(), 3);

    // --- keygen, 2 of 3
    let mut up = coord.start_keygen("family".into(), 2, ids.clone(), Network::Signet).unwrap();
    let mut by_id: BTreeMap<String, usize> = BTreeMap::new();
    for (i, s) in signers.iter().enumerate() {
        by_id.insert(s.device_id().to_string(), i);
    }

    // round 1: signers must be asked, then reply
    let b1 = frames_of(&up);
    let mut replies = vec![];
    for s in signers.iter_mut() {
        match signer_step(s, &b1) {
            Step::Confirm(Prompt::JoinKeygen { threshold: 2, n_signers: 3, .. }) => {}
            other => panic!("{other:?}"),
        }
        replies.push(approve_to_reply(s));
    }
    for r in &replies {
        up = coord.receive(&scan(r)).unwrap();
    }
    // round 2: certify (automatic)
    let b2 = frames_of(&up);
    let replies: Vec<_> = signers.iter_mut().map(|s| auto_reply(s, &b2)).collect();
    for r in &replies {
        up = coord.receive(&scan(r)).unwrap();
    }
    // round 3: check codes
    let coord_code = up.check_code.clone().expect("coordinator shows check code");
    let b3 = frames_of(&up);
    let mut acks = vec![];
    for s in signers.iter_mut() {
        match signer_step(s, &b3) {
            Step::Confirm(Prompt::VerifyKeygen { check_code, .. }) => assert_eq!(check_code, coord_code),
            other => panic!("{other:?}"),
        }
        acks.push(approve_to_reply(s));
    }
    for a in &acks {
        up = coord.receive(&scan(a)).unwrap();
    }
    let Some(Finished::KeyCreated { key_id, .. }) = up.finished.clone() else { panic!("{up:?}") };
    // final: store
    let b4 = frames_of(&up);
    for s in signers.iter_mut() {
        let r = s.receive(&scan(&b4)).unwrap();
        assert!(matches!(r.step, Step::Done { .. }));
        assert!(r.snapshot.is_some(), "the share must be persisted");
        assert_eq!(s.keys().len(), 1);
    }

    // --- restart everything from storage ("woken up again")
    let snaps: Vec<Vec<u8>> = signers.iter().map(|s| s.snapshot()).collect();
    let mut signers: Vec<Signer> = snaps.iter().map(|b| Signer::from_snapshot(b).unwrap()).collect();
    drop(coord);
    let mut coord = Coordinator::open(&dir).unwrap();
    assert_eq!(coord.keys().len(), 1);

    // --- fund the wallet on a fake chain and sync
    let (master, network) = coord.wallet_params(&key_id).unwrap();
    let mut chain = Chain(BTreeMap::new());
    let funding = wallet::address(master, network, false, 0).to_string();
    chain.0.insert(
        funding,
        vec![EsploraUtxo { txid: Txid::from_str(&"11".repeat(32)).unwrap(), vout: 1, value: 100_000, confirmed: true }],
    );
    coord.store_wallet(&key_id, wallet::sync(&chain, master, network).unwrap()).unwrap();
    let dest = wallet::address(master, network, false, 50).to_string(); // any signet address will do
    let built = coord.prepare_send(&key_id, &dest, Amt::Sats(40_000), 3.0).unwrap();
    let template = built.template.clone();

    // --- signing with alice + carol
    let chosen = [0usize, 2];
    let chosen_ids: Vec<String> = chosen.iter().map(|&i| signers[i].device_id().to_string()).collect();
    let up = coord.begin_signing(&key_id, chosen_ids.clone()).unwrap();
    assert_eq!(up.awaiting.len(), 2);
    let prepare = frames_of(&up);

    // step 1: each chosen signer reviews, then releases nonces. The third signer ignores it.
    assert!(matches!(signer_step(&mut signers[1], &prepare), Step::Ignored { .. }));
    let mut nonce_replies = vec![];
    for &i in &chosen {
        match signer_step(&mut signers[i], &prepare) {
            Step::Confirm(Prompt::ReviewTransaction(sum)) => {
                assert_eq!(sum.outputs.len(), 2);
                assert!(sum.outputs.iter().any(|o| o.amount_sats == 40_000));
            }
            other => panic!("{other:?}"),
        }
        nonce_replies.push(approve_persisting(&mut signers[i]));
    }
    // a signer that restarts between the rounds still works (nonce slots are persisted)
    signers[2] = Signer::from_snapshot(&signers[2].snapshot()).unwrap();
    let mut up = coord.receive(&scan(&nonce_replies[0])).unwrap();
    assert!(up.broadcast.is_none());
    up = coord.receive(&scan(&nonce_replies[1])).unwrap();
    let bundle = frames_of(&up);

    // step 2: final verified prompt, then partial signatures
    let mut sigs = vec![];
    for &i in &chosen {
        match signer_step(&mut signers[i], &bundle) {
            Step::Confirm(Prompt::SignTransaction(sum)) => {
                assert!(sum.verified);
                assert_eq!(sum.fee_sats, template.fee().unwrap() as i64);
                // the verified view hides the single change output of an ordinary send
                assert_eq!(sum.outputs.len(), 1);
                assert_eq!(sum.outputs[0].amount_sats, 40_000);
                assert!(sum.outputs[0].own_label.is_none());
                assert!(sum.warnings.is_empty(), "{:?}", sum.warnings);
            }
            other => panic!("{other:?}"),
        }
        sigs.push(approve_persisting(&mut signers[i]));
    }
    let mut up = coord.receive(&scan(&sigs[0])).unwrap();
    assert!(up.finished.is_none());
    up = coord.receive(&scan(&sigs[1])).unwrap();
    let Some(Finished::Signed { tx_hex, .. }) = up.finished else { panic!("{up:?}") };

    // --- verify the transaction independently
    let tx: bitcoin::Transaction = bitcoin::consensus::encode::deserialize_hex(&tx_hex).unwrap();
    assert_eq!(tx.input.len(), template.inputs().len());
    let secp = secp256k1::Secp256k1::verification_only();
    for (msg, (input, _)) in template.iter_sighash().zip(tx.input.iter().zip(0..)) {
        let sig = bitcoin::taproot::Signature::from_slice(input.witness.nth(0).unwrap()).unwrap();
        let prev_spk = template.inputs()[0].txout().script_pubkey;
        let xonly = secp256k1::XOnlyPublicKey::from_slice(&prev_spk.as_bytes()[2..34]).unwrap();
        let m = secp256k1::Message::from_digest({ use bitcoin::hashes::Hash; msg.to_byte_array() });
        secp.verify_schnorr(&sig.signature, &m, &xonly).expect("valid BIP340 signature");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn signer_refuses_swapped_transaction() {
    // Same setup but the coordinator swaps the transaction between round 1 and round 2.
    let mut coord = Coordinator::in_memory();
    let mut signers: Vec<Signer> = ["a", "b"].iter().map(|n| Signer::create(n.to_string())).collect();
    for s in &signers {
        coord.enroll(&scan(&s.hello_frames())).unwrap();
    }
    let ids: Vec<String> = coord.signers().into_iter().map(|s| s.device_id).collect();
    let up = coord.start_keygen("k".into(), 2, ids.clone(), Network::Signet).unwrap();
    let mut up = run_keygen(&mut coord, &mut signers, up);
    let Some(Finished::KeyCreated { key_id, .. }) = up.finished.take() else { panic!() };
    for s in signers.iter_mut() {
        s.receive(&scan(&frames_of(&up))).unwrap();
    }
    let (master, network) = coord.wallet_params(&key_id).unwrap();
    let mut chain = Chain(BTreeMap::new());
    chain.0.insert(
        wallet::address(master, network, false, 0).to_string(),
        vec![EsploraUtxo { txid: Txid::from_str(&"22".repeat(32)).unwrap(), vout: 0, value: 100_000, confirmed: true }],
    );
    coord.store_wallet(&key_id, wallet::sync(&chain, master, network).unwrap()).unwrap();
    let dest = wallet::address(master, network, false, 9).to_string();

    // Session A (honest): signers review transaction A and release nonces. The malicious
    // coordinator keeps those nonce replies and never feeds them to session A.
    coord.prepare_send(&key_id, &dest, Amt::Sats(10_000), 3.0).unwrap();
    let up = coord.begin_signing(&key_id, ids.clone()).unwrap();
    let prepare = frames_of(&up);
    let mut nonces = vec![];
    for s in signers.iter_mut() {
        assert!(matches!(signer_step(s, &prepare), Step::Confirm(Prompt::ReviewTransaction(_))));
        nonces.push(approve_persisting(s));
    }
    // Session B (evil): same signers, different transaction, nonces replayed from A.
    coord.cancel().unwrap();
    coord.prepare_send(&key_id, &dest, Amt::Sats(60_000), 3.0).unwrap();
    coord.begin_signing(&key_id, ids.clone()).unwrap();
    let mut up = Update::default();
    for n in &nonces {
        up = coord.receive(&scan(n)).unwrap();
    }
    let bundle = frames_of(&up);
    for s in signers.iter_mut() {
        let err = s.receive(&scan(&bundle)).unwrap_err();
        assert!(err.contains("NOT the transaction you reviewed"), "{err}");
    }
}

fn run_keygen(coord: &mut Coordinator, signers: &mut [Signer], first: Update) -> Update {
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
