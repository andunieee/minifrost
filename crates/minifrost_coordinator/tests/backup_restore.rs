//! Backups and restoration over QR frames only, following frostsnap's flows:
//!
//! * display a signer's 25-word backup, confirm it was written down, check it with the quiz;
//! * rebuild a wallet on a fresh coordinator and fresh signers from typed-in physical backups,
//!   consolidate them, and sign with the result;
//! * rebuild a wallet whose coordinator was lost from signers that still hold their shares;
//! * refuse backups that don't fit.

use bitcoin::Network;
use minifrost_coordinator::engine::{Coordinator, Finished, Update};

mod common;
use common::{
    approve_to_reply, auto_reply, create_wallet, frames_of, fund_and_sign, scan, signer_step, Prompt, Signer, Step,
};

/// Ask `signer` to display its backup; return (share index, words) after the user confirms.
fn display_backup(coord: &mut Coordinator, signer: &mut Signer, key_id: &str) -> (u32, Vec<String>) {
    let (index, words, _) = display_backup_frames(coord, signer, key_id);
    (index, words)
}

/// Like [`display_backup`], also returning the "backup recorded" reply the coordinator scanned.
fn display_backup_frames(
    coord: &mut Coordinator,
    signer: &mut Signer,
    key_id: &str,
) -> (u32, Vec<String>, Vec<String>) {
    let up = coord.start_display_backup(key_id, &signer.device_id().to_string()).unwrap();
    let Step::Confirm(Prompt::DisplayBackup { share_index, words, threshold, key_name }) =
        signer_step(signer, &frames_of(&up))
    else {
        panic!("expected the backup to be displayed")
    };
    assert_eq!(words.len(), 25);
    assert_eq!(key_name, "family");
    assert_eq!(threshold, 2);
    let reply = approve_to_reply(signer);
    let up = coord.receive(&scan(&reply)).unwrap();
    assert!(matches!(up.finished, Some(Finished::BackupRecorded { share_index: i, .. }) if i == share_index));
    (share_index, words, reply)
}

/// The user types `words` into a signer that was asked to enter a backup; returns the reply frames.
fn type_backup(signer: &mut Signer, share_index: u32, words: &[String]) -> Vec<String> {
    match signer.submit_backup(share_index, words.to_vec()).unwrap().step {
        Step::Reply { frames, .. } => frames,
        other => panic!("expected a reply, got {other:?}"),
    }
}

fn restoration_id(coord: &Coordinator) -> String {
    let r = coord.restorations();
    assert_eq!(r.len(), 1);
    r[0].restoration_id.clone()
}

/// One full "enter a backup" round trip for a restoration: signer enters it, coordinator accepts,
/// signer saves it. Returns the final update.
fn enter_backup(
    coord: &mut Coordinator,
    signer: &mut Signer,
    start: Update,
    share_index: u32,
    words: &[String],
) -> Update {
    assert!(matches!(signer_step(signer, &frames_of(&start)), Step::EnterBackup));
    let entered = type_backup(signer, share_index, words);
    let up = coord.receive(&scan(&entered)).unwrap();
    assert!(up.finished.is_none(), "{:?}", up.finished);
    let saved = auto_reply(signer, &frames_of(&up));
    coord.receive(&scan(&saved)).unwrap()
}

#[test]
fn display_backup_then_pass_the_quiz() {
    let mut coord = Coordinator::in_memory();
    let (mut signers, key_id) = create_wallet(&mut coord, &["alice", "bob", "carol"], 2);

    // a fresh wallet has no backups yet
    let shares = coord.backup_shares(&key_id).unwrap();
    assert_eq!(shares.len(), 3);
    assert!(shares.iter().all(|s| !s.complete));

    let (index, words, alice_recorded) = display_backup_frames(&mut coord, &mut signers[0], &key_id);
    let shares = coord.backup_shares(&key_id).unwrap();
    assert_eq!(shares.iter().filter(|s| s.complete).count(), 1);
    assert_eq!(shares.iter().find(|s| s.share_index == index).unwrap().name, "alice");
    // the words are a well-formed backup of that very share
    let refs: [&str; 25] = words.iter().map(String::as_str).collect::<Vec<_>>().try_into().unwrap();
    frost_backup::ShareBackup::from_words(index, refs).expect("the displayed words decode");

    // quiz: wrong answers keep the same question, right ones advance, all 26 finish
    let up = coord.start_check_backup(&key_id, &signers[0].device_id().to_string()).unwrap();
    let Step::CheckBackup { key_name } = signer_step(&mut signers[0], &frames_of(&up)) else { panic!() };
    assert_eq!(key_name, "family");
    let mut done = None;
    for screen in 0..26u32 {
        let q = signers[0].quiz_question().unwrap();
        assert_eq!(q.screen, screen);
        assert_eq!(q.total, 26);
        assert_eq!(q.options.len(), 3);
        let right = if screen == 0 { format!("#{index}") } else { words[screen as usize - 1].clone() };
        let right_pos = q.options.iter().position(|o| *o == right).expect("the right answer is offered") as u32;
        let wrong_pos = (right_pos + 1) % 3;
        let wrong = signers[0].quiz_answer(wrong_pos).unwrap();
        assert!(!wrong.correct && wrong.next.is_none() && wrong.done.is_none());
        assert_eq!(signers[0].quiz_question().unwrap().screen, screen, "a wrong answer stays put");
        let ans = signers[0].quiz_answer(right_pos).unwrap();
        assert!(ans.correct);
        if screen < 25 {
            assert_eq!(ans.next.unwrap().screen, screen + 1);
        } else {
            done = ans.done;
        }
    }
    let Some(done) = done else { panic!("the last answer finishes the quiz") };
    let Step::Reply { frames, .. } = done.step else { panic!() };
    let up = coord.receive(&scan(&frames)).unwrap();
    assert!(matches!(up.finished, Some(Finished::BackupChecked { share_index: i, .. }) if i == index));

    // an old "backup recorded" code cannot confirm another signer's backup
    let up = coord.start_display_backup(&key_id, &signers[1].device_id().to_string()).unwrap();
    assert!(matches!(
        signer_step(&mut signers[1], &frames_of(&up)),
        Step::Confirm(Prompt::DisplayBackup { .. })
    ));
    let err = coord.receive(&scan(&alice_recorded)).unwrap_err();
    assert!(err.contains("not part of this session"), "{err}");
    assert!(!coord.backup_shares(&key_id).unwrap().iter().find(|s| s.name == "bob").unwrap().complete);
    coord.cancel().unwrap();
    signers[1].reject();
}

#[test]
fn restore_wallet_from_physical_backups_and_sign() {
    // --- an original wallet; write its backups down
    let mut orig = Coordinator::in_memory();
    let (mut orig_signers, orig_key) = create_wallet(&mut orig, &["alice", "bob", "carol"], 2);
    let backups: Vec<(u32, Vec<String>)> =
        orig_signers.iter_mut().map(|s| display_backup(&mut orig, s, &orig_key)).collect();
    let (orig_master, _) = orig.wallet_params(&orig_key).unwrap();

    // --- everything is lost: new coordinator, new phones
    let mut coord = Coordinator::in_memory();
    let mut phones: Vec<Signer> =
        ["new-alice", "new-bob", "new-carol"].iter().map(|n| Signer::create(n.to_string())).collect();
    for p in &phones {
        coord.enroll(&scan(&p.hello_frames())).unwrap();
    }
    let ids: Vec<String> = phones.iter().map(|p| p.device_id().to_string()).collect();

    // first backup starts the restoration (name and network asked up front, threshold unknown)
    let start = coord.start_enter_backup_new_wallet(&ids[0], "family".into(), Network::Signet, None).unwrap();
    assert!(matches!(signer_step(&mut phones[0], &frames_of(&start)), Step::EnterBackup));

    // mistyped words are refused by the signer and it keeps waiting
    let mut bad = backups[0].1.clone();
    bad[3] = "notaword".into();
    let err = phones[0].submit_backup(backups[0].0, bad).unwrap_err();
    assert!(err.contains("Word 4"), "{err}");
    let mut swapped = backups[0].1.clone();
    swapped.swap(0, 1);
    assert!(phones[0].submit_backup(backups[0].0, swapped).is_err(), "checksum catches reordered words");
    assert!(phones[0].submit_backup(backups[0].0, backups[0].1[..24].to_vec()).is_err());
    assert!(phones[0].submit_backup(0, backups[0].1.clone()).is_err());

    let entered = type_backup(&mut phones[0], backups[0].0, &backups[0].1);
    let up = coord.receive(&scan(&entered)).unwrap();
    let saved = auto_reply(&mut phones[0], &frames_of(&up));
    let up = coord.receive(&scan(&saved)).unwrap();
    let Some(Finished::BackupSaved { restorable, .. }) = up.finished else { panic!("{up:?}") };
    assert!(!restorable, "one share of an unknown threshold is not enough");
    let rid = restoration_id(&coord);
    let pending = phones[0].pending_backups();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].share_index, backups[0].0);

    // the same phone entering the same share again is refused
    let again = coord.start_enter_backup_restoration(&ids[0], &rid).unwrap();
    assert!(matches!(signer_step(&mut phones[0], &frames_of(&again)), Step::EnterBackup));
    let entered = type_backup(&mut phones[0], backups[0].0, &backups[0].1);
    let up = coord.receive(&scan(&entered)).unwrap();
    assert!(matches!(up.finished, Some(Finished::BackupRejected { .. })), "{:?}", up.finished);
    assert_eq!(coord.restorations().len(), 1, "the restoration itself survives a refused backup");

    // the second backup makes it restorable
    let start = coord.start_enter_backup_restoration(&ids[1], &rid).unwrap();
    let up = enter_backup(&mut coord, &mut phones[1], start, backups[1].0, &backups[1].1);
    let Some(Finished::BackupSaved { restorable, .. }) = up.finished else { panic!("{up:?}") };
    assert!(restorable);
    let view = &coord.restorations()[0];
    assert!(view.restorable);
    assert_eq!(view.got, Some(2));

    // --- finish: the wallet is back with the same key, and two phones await consolidation
    let key_id = coord.finish_restoration(&rid).unwrap();
    assert!(coord.restorations().is_empty());
    let (master, _) = coord.wallet_params(&key_id).unwrap();
    assert_eq!(master, orig_master, "same wallet, same addresses");
    assert_eq!(coord.keys()[0].threshold, 2);
    let pending = coord.pending_consolidations();
    assert_eq!(pending.len(), 2);
    for p in pending {
        let i = phones.iter().position(|s| s.device_id().to_string() == p.device_id).unwrap();
        let up = coord.start_consolidate(&key_id, &p.device_id).unwrap();
        let reply = auto_reply(&mut phones[i], &frames_of(&up));
        let up = coord.receive(&scan(&reply)).unwrap();
        assert!(matches!(up.finished, Some(Finished::Consolidated { .. })));
    }
    assert!(coord.pending_consolidations().is_empty());
    assert!(phones[0].pending_backups().is_empty());
    assert_eq!(phones[0].keys().len(), 1);
    // a restored wallet counts as backed up: its owner has the paper
    assert!(coord.backup_shares(&key_id).unwrap().iter().all(|s| s.complete));

    // --- the third phone joins the finished wallet with its backup
    let up = coord.start_enter_backup_wallet(&ids[2], &key_id).unwrap();
    assert!(matches!(signer_step(&mut phones[2], &frames_of(&up)), Step::EnterBackup));
    let entered = type_backup(&mut phones[2], backups[2].0, &backups[2].1);
    let up = coord.receive(&scan(&entered)).unwrap();
    let consolidate = auto_reply(&mut phones[2], &frames_of(&up));
    let up = coord.receive(&scan(&consolidate)).unwrap();
    assert!(matches!(up.finished, Some(Finished::Consolidated { .. })));
    assert_eq!(coord.keys()[0].signers.len(), 3);
    assert_eq!(phones[2].keys().len(), 1);

    // --- and the restored wallet really signs, with phones that never saw the original keygen
    fund_and_sign(&mut coord, &mut phones, &[0, 2], &key_id);

    // phones survive a restart
    let snaps: Vec<_> = phones.iter().map(|p| p.snapshot()).collect();
    let mut phones: Vec<Signer> = snaps.iter().map(|b| Signer::from_snapshot(b).unwrap()).collect();
    fund_and_sign(&mut coord, &mut phones, &[1, 2], &key_id);
}

#[test]
fn restore_wallet_from_signers_when_the_coordinator_is_lost() {
    let mut orig = Coordinator::in_memory();
    let (mut signers, _) = create_wallet(&mut orig, &["alice", "bob", "carol"], 2);
    drop(orig); // the coordinator and its file are gone; the signers kept their shares

    let mut coord = Coordinator::in_memory();
    for s in &signers {
        coord.enroll(&scan(&s.hello_frames())).unwrap();
    }
    let ids: Vec<String> = signers.iter().map(|s| s.device_id().to_string()).collect();

    // ask alice what she holds
    let up = coord.start_request_held_shares(&ids[0]).unwrap();
    let reply = auto_reply(&mut signers[0], &frames_of(&up));
    let up = coord.receive(&scan(&reply)).unwrap();
    let Some(Finished::HeldShares { shares, .. }) = up.finished else { panic!("{up:?}") };
    assert_eq!(shares.len(), 1);
    assert_eq!(shares[0].key_name.as_deref(), Some("family"));
    assert_eq!(shares[0].threshold, Some(2));
    assert!(!shares[0].needs_consolidation);
    assert!(shares[0].known_wallet.is_none());

    let rid = coord.start_restoration_from_held_share(&ids[0], shares[0].share_index).unwrap();
    assert!(!coord.restorations()[0].restorable);

    // bob adds the second share
    let up = coord.start_request_held_shares(&ids[1]).unwrap();
    let reply = auto_reply(&mut signers[1], &frames_of(&up));
    let up = coord.receive(&scan(&reply)).unwrap();
    let Some(Finished::HeldShares { shares: bobs, .. }) = up.finished else { panic!("{up:?}") };
    coord.add_held_share_to_restoration(&rid, &ids[1], bobs[0].share_index).unwrap();
    assert!(coord.restorations()[0].restorable);
    // adding the same share twice is refused
    assert!(coord.add_held_share_to_restoration(&rid, &ids[1], bobs[0].share_index).is_err());

    let key_id = coord.finish_restoration(&rid).unwrap();
    assert!(coord.pending_consolidations().is_empty(), "real shares need no consolidation");
    assert_eq!(coord.keys()[0].threshold, 2);

    // carol rejoins the wallet through her held share
    let up = coord.start_request_held_shares(&ids[2]).unwrap();
    let reply = auto_reply(&mut signers[2], &frames_of(&up));
    let up = coord.receive(&scan(&reply)).unwrap();
    let Some(Finished::HeldShares { shares: carols, .. }) = up.finished else { panic!("{up:?}") };
    assert_eq!(carols[0].known_wallet.as_deref(), Some("family"));
    coord.add_held_share_to_wallet(&key_id, &ids[2], carols[0].share_index).unwrap();
    assert_eq!(coord.keys()[0].signers.len(), 3);

    fund_and_sign(&mut coord, &mut signers, &[0, 2], &key_id);
}

#[test]
fn wrong_backups_are_refused() {
    let mut coord = Coordinator::in_memory();
    let (mut signers, key_id) = create_wallet(&mut coord, &["alice", "bob"], 2);
    let (index, words) = display_backup(&mut coord, &mut signers[0], &key_id);

    let mut phone = Signer::create("phone".into());
    coord.enroll(&scan(&phone.hello_frames())).unwrap();
    let phone_id = phone.device_id().to_string();

    // A backup of a share this coordinator already has can't start another restoration ...
    let start = coord.start_enter_backup_new_wallet(&phone_id, "again".into(), Network::Signet, None).unwrap();
    assert!(matches!(signer_step(&mut phone, &frames_of(&start)), Step::EnterBackup));
    let entered = type_backup(&mut phone, index, &words);
    let up = coord.receive(&scan(&entered)).unwrap();
    let Some(Finished::BackupRejected { reason }) = up.finished else { panic!("{up:?}") };
    assert!(reason.contains("family"), "{reason}");
    assert!(coord.restorations().is_empty(), "a refused first backup leaves no restoration behind");

    // ... and a backup from another wallet doesn't fit this one.
    let mut other = Coordinator::in_memory();
    let (mut other_signers, other_key) = create_wallet(&mut other, &["x", "y"], 2);
    let (other_index, other_words) = display_backup(&mut other, &mut other_signers[0], &other_key);
    let start = coord.start_enter_backup_wallet(&phone_id, &key_id).unwrap();
    // only one session at a time
    assert!(coord.start_display_backup(&key_id, &signers[0].device_id().to_string()).is_err());
    assert!(matches!(signer_step(&mut phone, &frames_of(&start)), Step::EnterBackup));
    let entered = type_backup(&mut phone, other_index, &other_words);
    let up = coord.receive(&scan(&entered)).unwrap();
    assert!(matches!(up.finished, Some(Finished::BackupRejected { .. })), "{:?}", up.finished);
    assert_eq!(coord.keys()[0].signers.len(), 2, "the wallet is unchanged");
}

fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("minifrost-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn backup_and_restoration_state_survive_a_restart() {
    let dir = temp_dir("restart");
    let mut coord = Coordinator::open(&dir).unwrap();
    let (mut signers, key_id) = create_wallet(&mut coord, &["alice", "bob", "carol"], 2);
    display_backup(&mut coord, &mut signers[1], &key_id);

    let mut coord = Coordinator::open(&dir).unwrap();
    let shares = coord.backup_shares(&key_id).unwrap();
    assert_eq!(shares.iter().filter(|s| s.complete).map(|s| s.name.as_str()).collect::<Vec<_>>(), ["bob"]);
    // the unfinished ones can be marked by hand, as in frostsnap
    let carol = shares.iter().find(|s| s.name == "carol").unwrap().share_index;
    coord.mark_backup_complete(&key_id, carol).unwrap();
    let coord = Coordinator::open(&dir).unwrap();
    assert_eq!(coord.backup_shares(&key_id).unwrap().iter().filter(|s| s.complete).count(), 2);
    drop(coord);

    // a restoration in progress is kept too
    let dir2 = temp_dir("restart-restoring");
    let mut lost = Coordinator::open(&dir2).unwrap();
    for s in &signers {
        lost.enroll(&scan(&s.hello_frames())).unwrap();
    }
    let id = signers[0].device_id().to_string();
    let up = lost.start_request_held_shares(&id).unwrap();
    let reply = auto_reply(&mut signers[0], &frames_of(&up));
    let up = lost.receive(&scan(&reply)).unwrap();
    let Some(Finished::HeldShares { shares, .. }) = up.finished else { panic!("{up:?}") };
    lost.start_restoration_from_held_share(&id, shares[0].share_index).unwrap();
    let lost = Coordinator::open(&dir2).unwrap();
    assert_eq!(lost.restorations().len(), 1);
    assert_eq!(lost.restorations()[0].shares.len(), 1);
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&dir2);
}

#[test]
fn a_version_1_coordinator_file_opens_and_its_wallets_need_backing_up() {
    let dir = temp_dir("v1");
    let mut coord = Coordinator::open(&dir).unwrap();
    let (_signers, key_id) = create_wallet(&mut coord, &["alice", "bob", "carol"], 2);
    drop(coord);

    // Rebuild the file as version 1 wrote it: the same fields without the trailing backup list
    // (a length byte, then per share a 64 byte wallet reference, the share index and a flag).
    let path = dir.join("coordinator.bin");
    let v2 = std::fs::read(&path).unwrap();
    assert_eq!(v2[0], 2);
    let backup_list = 1 + 3 * (64 + 1 + 1);
    let mut v1 = v2[..v2.len() - backup_list].to_vec();
    v1[0] = 1;
    std::fs::write(&path, v1).unwrap();

    let mut coord = Coordinator::open(&dir).unwrap();
    assert_eq!(coord.keys().len(), 1);
    assert!(coord.backup_shares(&key_id).unwrap().iter().all(|s| !s.complete));
    // opening rewrote it in the current version
    assert_eq!(std::fs::read(&path).unwrap()[0], 2);
    coord.mark_backup_complete(&key_id, 1).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
}
