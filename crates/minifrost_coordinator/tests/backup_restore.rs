//! Backups and restoration over QR frames only, following frostsnap's flows:
//!
//! * display a signer's 25-word backup, confirm it was written down, check it with the quiz;
//! * rebuild a wallet on a fresh coordinator and fresh signers from backups typed into the signers,
//!   give the signers the rest, and sign with the result;
//! * rebuild a wallet whose coordinator was lost from signers that still hold their shares, or from
//!   a mix of both;
//! * sort out shares of several wallets, and refuse the ones that can't count.

use bitcoin::Network;
use minifrost_coordinator::engine::{Coordinator, Finished, Update};
use minifrost_wire::Packet;

mod common;
use common::{Prompt, Signer, Step, create_wallet, frames_of, fund_and_sign, scan, signer_step};

/// Ask `signer` to display its backup; return (share index, words) after the user confirms.
fn display_backup(
    coord: &mut Coordinator,
    signer: &mut Signer,
    key_id: &str,
) -> (u32, Vec<String>) {
    let up = coord
        .start_display_backup(key_id, &signer.device_id().to_string())
        .unwrap();
    assert_eq!(up.confirm.as_deref(), Some("I wrote down the 25 words"));
    let Step::Confirm(Prompt::DisplayBackup {
        share_index,
        words,
        threshold,
        key_name,
    }) = signer_step(signer, &frames_of(&up))
    else {
        panic!("expected the backup to be displayed")
    };
    assert_eq!(words.len(), 25);
    assert_eq!(key_name, "family");
    assert_eq!(threshold, coord.keys()[0].threshold);
    // confirming on the signer only ends its local prompt; there is no reply to show
    assert!(matches!(signer.approve().unwrap().step, Step::Done { .. }));
    let up = coord.confirm_backup().unwrap();
    assert!(
        matches!(up.finished, Some(Finished::BackupRecorded { share_index: i, .. }) if i == share_index)
    );
    (share_index, words)
}

/// The user types `words` into a signer, which checks them and shows its restore code.
fn type_backup(signer: &mut Signer, share_index: u32, words: &[String]) -> Vec<String> {
    let r = signer.submit_backup(share_index, words.to_vec()).unwrap();
    assert!(
        r.snapshot.is_some(),
        "the backup is stored before its code is shown"
    );
    restore_frames(r.step)
}

fn restore_frames(step: Step) -> Vec<String> {
    match step {
        Step::Reply { frames, .. } => frames,
        other => panic!("expected a restore code, got {other:?}"),
    }
}

/// "Restore wallet on coordinator" on the signer's only key.
fn key_code(signer: &Signer) -> Vec<String> {
    let key_id = signer.keys()[0].key_id.clone();
    restore_frames(signer.restore_code(&key_id).unwrap())
}

/// A wallet with `names` as signers and all their backups written down.
fn wallet_with_backups(
    coord: &mut Coordinator,
    names: &[&str],
    threshold: u16,
) -> (Vec<Signer>, String, Vec<(u32, Vec<String>)>) {
    let (mut signers, key_id) = create_wallet(coord, names, threshold);
    let backups = signers
        .iter_mut()
        .map(|s| display_backup(coord, s, &key_id))
        .collect();
    (signers, key_id, backups)
}

fn fresh_phones(names: &[&str]) -> Vec<Signer> {
    names
        .iter()
        .map(|n| Signer::create(n.to_string()))
        .collect()
}

/// The wallet's code reaches each signer, which saves its key; the user confirms it on the
/// coordinator. Returns the rebuilt wallet's key id.
fn finish_signers(coord: &mut Coordinator, up: &Update, signers: &mut [&mut Signer]) -> String {
    assert_eq!(
        up.broadcast.as_ref().unwrap().title,
        minifrost_wire::Step::FinishRestoring.title()
    );
    assert!(up.confirm_each_signer);
    assert_eq!(up.awaiting.len(), signers.len());
    let code = frames_of(up);
    let mut last = up.clone();
    for s in signers.iter_mut() {
        let r = s.receive(&scan(&code)).unwrap();
        assert!(matches!(r.step, Step::Done { .. }), "{:?}", r.step);
        assert!(r.snapshot.is_some(), "the key is stored");
        assert!(r.new_share.is_some(), "the app is told about the new share");
        assert_eq!(s.keys().len(), 1);
        assert!(s.pending_backups().is_empty());
        // reading the same code again changes nothing
        assert!(matches!(
            s.receive(&scan(&code)).unwrap().step,
            Step::Done { .. }
        ));
        last = coord.confirm_signer(&s.device_id().to_string()).unwrap();
    }
    match last.finished {
        Some(Finished::WalletRestored { key_id, .. }) => key_id,
        other => panic!("{other:?}"),
    }
}

#[test]
fn display_backup_then_pass_the_quiz() {
    let mut coord = Coordinator::in_memory();
    let (mut signers, key_id) = create_wallet(&mut coord, &["alice", "bob", "carol"], 2);

    // a fresh wallet has no backups yet
    let shares = coord.backup_shares(&key_id).unwrap();
    assert_eq!(shares.len(), 3);
    assert!(shares.iter().all(|s| !s.complete));

    let (index, words) = display_backup(&mut coord, &mut signers[0], &key_id);
    let shares = coord.backup_shares(&key_id).unwrap();
    assert_eq!(shares.iter().filter(|s| s.complete).count(), 1);
    assert_eq!(
        shares.iter().find(|s| s.share_index == index).unwrap().name,
        "alice"
    );
    // the words are a well-formed backup of that very share
    let refs: [&str; 25] = words
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();
    frost_backup::ShareBackup::from_words(index, refs).expect("the displayed words decode");

    // quiz: wrong answers keep the same question, right ones advance, all 26 finish
    let up = coord
        .start_check_backup(&key_id, &signers[0].device_id().to_string())
        .unwrap();
    assert!(up.confirm.is_some(), "the check is confirmed with a button");
    let Step::CheckBackup { key_name } = signer_step(&mut signers[0], &frames_of(&up)) else {
        panic!()
    };
    assert_eq!(key_name, "family");
    let mut done = None;
    for screen in 0..26u32 {
        let q = signers[0].quiz_question().unwrap();
        assert_eq!(q.screen, screen);
        assert_eq!(q.total, 26);
        assert_eq!(q.options.len(), 3);
        let right = if screen == 0 {
            format!("#{index}")
        } else {
            words[screen as usize - 1].clone()
        };
        let right_pos = q
            .options
            .iter()
            .position(|o| *o == right)
            .expect("the right answer is offered") as u32;
        let wrong_pos = (right_pos + 1) % 3;
        let wrong = signers[0].quiz_answer(wrong_pos).unwrap();
        assert!(!wrong.correct && wrong.next.is_none() && wrong.done.is_none());
        assert_eq!(
            signers[0].quiz_question().unwrap().screen,
            screen,
            "a wrong answer stays put"
        );
        let ans = signers[0].quiz_answer(right_pos).unwrap();
        assert!(ans.correct);
        if screen < 25 {
            assert_eq!(ans.next.unwrap().screen, screen + 1);
        } else {
            done = ans.done;
        }
    }
    let Some(done) = done else {
        panic!("the last answer finishes the quiz")
    };
    // the signer only says so on its own screen; the user confirms it on the coordinator
    assert!(
        matches!(done.step, Step::Done { .. }),
        "no reply code after the quiz"
    );
    let up = coord.confirm_backup().unwrap();
    assert!(
        matches!(up.finished, Some(Finished::BackupChecked { share_index: i, .. }) if i == index)
    );

    // a backup stays unconfirmed until the user presses the coordinator's button
    let up = coord
        .start_display_backup(&key_id, &signers[1].device_id().to_string())
        .unwrap();
    assert!(matches!(
        signer_step(&mut signers[1], &frames_of(&up)),
        Step::Confirm(Prompt::DisplayBackup { .. })
    ));
    let bob = |coord: &Coordinator| {
        coord
            .backup_shares(&key_id)
            .unwrap()
            .into_iter()
            .find(|s| s.name == "bob")
            .unwrap()
            .complete
    };
    assert!(!bob(&coord));
    assert!(matches!(
        signers[1].approve().unwrap().step,
        Step::Done { .. }
    ));
    assert!(
        !bob(&coord),
        "the signer's local confirmation does not change the coordinator"
    );
    let up = coord.confirm_backup().unwrap();
    assert!(matches!(
        up.finished,
        Some(Finished::BackupRecorded { share_index: i, .. }) if i != index
    ));
    assert!(bob(&coord));
    assert!(coord.confirm_backup().is_err(), "nothing left to confirm");
}

/// As in frostsnap, a signer's share key mixes in the coordinator's contribution (a hash of the
/// root polynomial): with a wrong one the signer can't decrypt its share to show the backup.
#[test]
fn signer_needs_the_coordinator_to_decrypt_its_backup() {
    use frostsnap_core::message::{CoordinatorRestoration, CoordinatorToDeviceMessage};

    let mut coord = Coordinator::in_memory();
    let (mut signers, key_id) = create_wallet(&mut coord, &["alice", "bob"], 2);
    let up = coord
        .start_display_backup(&key_id, &signers[0].device_id().to_string())
        .unwrap();
    let mut packet = Packet::from_bytes(&scan(&frames_of(&up))).unwrap();
    let Packet::ToDevices { entries, .. } = &mut packet else {
        panic!("{packet:?}")
    };
    let CoordinatorToDeviceMessage::Restoration(CoordinatorRestoration::DisplayBackup {
        coord_share_decryption_contrib,
        ..
    }) = &mut entries[0].1
    else {
        panic!()
    };
    *coord_share_decryption_contrib =
        "0404040404040404040404040404040404040404040404040404040404040404"
            .parse()
            .unwrap();
    let err = signers[0].receive(&packet.to_bytes()).unwrap_err();
    assert!(err.contains("could not read this signer's share"), "{err}");
}

#[test]
fn restore_wallet_from_physical_backups_and_sign() {
    // --- an original wallet; write its backups down
    let mut orig = Coordinator::in_memory();
    let (_, orig_key, backups) = wallet_with_backups(&mut orig, &["alice", "bob", "carol"], 2);
    let (orig_master, _) = orig.wallet_params(&orig_key).unwrap();

    // --- everything is lost: new coordinator, new phones nobody enrolled
    let mut coord = Coordinator::in_memory();
    let mut phones = fresh_phones(&["new-alice", "new-bob", "new-carol"]);

    // mistyped words are refused by the signer, which stores nothing
    let mut bad = backups[0].1.clone();
    bad[3] = "notaword".into();
    let err = phones[0].submit_backup(backups[0].0, bad).unwrap_err();
    assert!(err.contains("Word 4"), "{err}");
    let mut swapped = backups[0].1.clone();
    swapped.swap(0, 1);
    assert!(
        phones[0].submit_backup(backups[0].0, swapped).is_err(),
        "checksum catches reordered words"
    );
    assert!(
        phones[0]
            .submit_backup(backups[0].0, backups[0].1[..24].to_vec())
            .is_err()
    );
    assert!(phones[0].submit_backup(0, backups[0].1.clone()).is_err());
    assert!(phones[0].pending_backups().is_empty());

    // the first code registers the signer and is not enough
    let first = type_backup(&mut phones[0], backups[0].0, &backups[0].1);
    let up = coord.read_restore_code(&scan(&first)).unwrap();
    assert!(up.broadcast.is_none() && up.finished.is_none() && up.unnamed_wallet.is_none());
    assert_eq!(up.from.as_ref().unwrap().name, "new-alice");
    assert_eq!(coord.signers().len(), 1);
    assert_eq!(
        coord.signers()[0].device_id,
        phones[0].device_id().to_string()
    );
    assert_eq!(coord.restoring_shares().len(), 1);
    assert_eq!(coord.restoring_shares()[0].share_index, backups[0].0);
    let pending = phones[0].pending_backups();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].share_index, backups[0].0);

    // the camera keeps seeing the same code; the code on the home screen is the same share
    assert!(coord.read_restore_code(&scan(&first)).unwrap().repeat);
    let again = restore_frames(pending[0].restore_code.clone());
    assert!(coord.read_restore_code(&scan(&again)).unwrap().repeat);
    assert_eq!(coord.restoring_shares().len(), 1);

    // the phones get restarted while the coordinator is still collecting
    let mut phones: Vec<Signer> = phones
        .iter()
        .map(|p| Signer::from_snapshot(&p.snapshot()).unwrap())
        .collect();
    assert_eq!(phones[0].pending_backups().len(), 1);

    // the second code is enough, but nobody knows the wallet's name and network
    let second = type_backup(&mut phones[1], backups[1].0, &backups[1].1);
    let up = coord.read_restore_code(&scan(&second)).unwrap();
    let unnamed = up.unnamed_wallet.expect("asks for the name");
    assert_eq!(unnamed.threshold, 2);
    assert_eq!(unnamed.signers, ["new-alice", "new-bob"]);
    assert_eq!(unnamed.key_id, orig_key, "same key found");
    assert!(coord.keys().is_empty());
    assert_eq!(coord.unnamed_wallets().len(), 1, "stays offered");
    assert!(
        coord
            .name_restored_wallet(&unnamed.key_id, " ".into(), Network::Signet)
            .is_err()
    );

    // --- named: the wallet is saved right away and the code for the signers comes up
    let up = coord
        .name_restored_wallet(&unnamed.key_id, " family ".into(), Network::Signet)
        .unwrap();
    assert!(coord.restoring_shares().is_empty() && coord.unnamed_wallets().is_empty());
    assert_eq!(coord.keys().len(), 1);
    assert_eq!(coord.keys()[0].name, "family");
    assert_eq!(coord.keys()[0].threshold, 2);
    let (master, _) = coord.wallet_params(&orig_key).unwrap();
    assert_eq!(master, orig_master, "same wallet, same addresses");
    assert_eq!(coord.pending_consolidations().len(), 2);

    // ... and it can be left and shown again
    coord.cancel().unwrap();
    let up2 = coord.start_finish_restoring(&orig_key).unwrap();
    assert_eq!(frames_of(&up2).len(), frames_of(&up).len());

    // a phone that isn't part of it ignores the code
    assert!(matches!(
        phones[2].receive(&scan(&frames_of(&up2))).unwrap().step,
        Step::Ignored { .. }
    ));
    let [alice, bob, carol] = &mut phones[..] else {
        panic!()
    };
    assert!(
        coord
            .confirm_signer(&carol.device_id().to_string())
            .is_err(),
        "carol's phone is not in this session"
    );
    let key_id = finish_signers(&mut coord, &up2, &mut [alice, bob]);
    assert_eq!(key_id, orig_key);
    assert!(coord.pending_consolidations().is_empty());
    // a restored wallet counts as backed up: its owner has the paper
    assert!(
        coord
            .backup_shares(&key_id)
            .unwrap()
            .iter()
            .all(|s| s.complete)
    );

    // --- a third phone joins the finished wallet the same way, nothing to choose
    let third = type_backup(&mut phones[2], backups[2].0, &backups[2].1);
    let up = coord.read_restore_code(&scan(&third)).unwrap();
    assert_eq!(coord.signers().len(), 3);
    assert_eq!(coord.keys()[0].signers.len(), 3);
    let [_, _, carol] = &mut phones[..] else {
        panic!()
    };
    finish_signers(&mut coord, &up, &mut [carol]);
    assert!(
        coord.read_restore_code(&scan(&third)).unwrap().repeat,
        "reading it again is harmless"
    );

    // --- and the restored wallet really signs, with phones that never saw the original keygen
    fund_and_sign(&mut coord, &mut phones, &[0, 2], &key_id);

    // phones survive a restart
    let snaps: Vec<_> = phones.iter().map(|p| p.snapshot()).collect();
    let mut phones: Vec<Signer> = snaps
        .iter()
        .map(|b| Signer::from_snapshot(b).unwrap())
        .collect();
    fund_and_sign(&mut coord, &mut phones, &[1, 2], &key_id);
}

#[test]
fn restore_wallet_from_signers_when_the_coordinator_is_lost() {
    let mut orig = Coordinator::in_memory();
    let (mut signers, key_id) = create_wallet(&mut orig, &["alice", "bob", "carol"], 2);
    drop(orig); // the coordinator and its file are gone; the signers kept their shares

    let mut coord = Coordinator::in_memory();
    let up = coord
        .read_restore_code(&scan(&key_code(&signers[0])))
        .unwrap();
    assert!(up.finished.is_none() && up.unnamed_wallet.is_none());
    let share = &coord.restoring_shares()[0];
    assert_eq!(share.key_name.as_deref(), Some("family"));
    assert_eq!(share.threshold, Some(2));

    // bob's code rebuilds it with no question asked, and no code for signers that kept their keys
    let up = coord
        .read_restore_code(&scan(&key_code(&signers[1])))
        .unwrap();
    assert!(up.broadcast.is_none(), "real shares need no consolidation");
    assert!(matches!(
        up.finished,
        Some(Finished::WalletRestored { key_id: ref k, ref name }) if *k == key_id && name == "family"
    ));
    assert!(coord.restoring_shares().is_empty());
    assert_eq!(coord.keys()[0].threshold, 2);
    assert_eq!(coord.signers().len(), 2);

    // carol rejoins through the same code
    let up = coord
        .read_restore_code(&scan(&key_code(&signers[2])))
        .unwrap();
    assert!(matches!(up.finished, Some(Finished::WalletRestored { .. })));
    assert_eq!(coord.keys()[0].signers.len(), 3);
    assert!(coord.pending_consolidations().is_empty());
    assert!(
        coord
            .read_restore_code(&scan(&key_code(&signers[2])))
            .unwrap()
            .repeat
    );

    fund_and_sign(&mut coord, &mut signers, &[0, 2], &key_id);
}

#[test]
fn signer_codes_and_backups_mix() {
    let mut orig = Coordinator::in_memory();
    let (mut signers, key_id, backups) = wallet_with_backups(&mut orig, &["alice", "bob"], 2);
    drop(orig);

    // alice still has her phone; bob's backup goes into a new one. The share that knows the name
    // and network saves the question, in whichever order they are read.
    let mut coord = Coordinator::in_memory();
    let mut phone = Signer::create("new-bob".into());
    let code = type_backup(&mut phone, backups[1].0, &backups[1].1);
    coord.read_restore_code(&scan(&code)).unwrap();
    let up = coord
        .read_restore_code(&scan(&key_code(&signers[0])))
        .unwrap();
    assert!(up.unnamed_wallet.is_none());
    assert_eq!(coord.keys()[0].name, "family");
    // only the typed-in backup has anything to wait for
    assert_eq!(up.awaiting.len(), 1);
    assert_eq!(up.awaiting[0].name, "new-bob");
    finish_signers(&mut coord, &up, &mut [&mut phone]);

    let mut all = vec![signers.remove(0), phone];
    fund_and_sign(&mut coord, &mut all, &[0, 1], &key_id);
}

#[test]
fn shares_of_several_wallets_sort_themselves_out() {
    let mut a = Coordinator::in_memory();
    let (_, key_a, backups_a) = wallet_with_backups(&mut a, &["a1", "a2"], 2);
    let mut b = Coordinator::in_memory();
    let (_, key_b, backups_b) = wallet_with_backups(&mut b, &["b1", "b2", "b3"], 3);

    let mut coord = Coordinator::in_memory();
    let mut phones = fresh_phones(&["p1", "p2", "p3", "p4", "p5"]);
    // one share of each is not a wallet, even with several signers' worth of them
    let mut read = |phone: &mut Signer, backup: &(u32, Vec<String>)| {
        coord
            .read_restore_code(&scan(&type_backup(phone, backup.0, &backup.1)))
            .unwrap()
    };
    assert!(read(&mut phones[0], &backups_a[0]).unnamed_wallet.is_none());
    assert!(read(&mut phones[1], &backups_b[0]).unnamed_wallet.is_none());
    assert!(read(&mut phones[2], &backups_b[1]).unnamed_wallet.is_none());
    // the second of A completes A and leaves B's alone
    let found = read(&mut phones[3], &backups_a[1]).unnamed_wallet.unwrap();
    assert_eq!(found.key_id, key_a);
    assert_eq!(found.signers, ["p1", "p4"]);
    // B needs its third even while A waits for a name
    let up = read(&mut phones[4], &backups_b[2]);
    let found = up.unnamed_wallet.unwrap();
    assert_eq!(coord.unnamed_wallets().len(), 2);
    assert!(found.key_id == key_a || found.key_id == key_b);

    let up = coord
        .name_restored_wallet(&key_b, "b".into(), Network::Signet)
        .unwrap();
    assert_eq!(coord.keys()[0].threshold, 3);
    assert_eq!(up.awaiting.len(), 3);
    coord.cancel().unwrap();
    coord
        .name_restored_wallet(&key_a, "a".into(), Network::Bitcoin)
        .unwrap();
    assert!(coord.restoring_shares().is_empty());
    assert_eq!(coord.keys().len(), 2);
    assert_eq!(coord.pending_consolidations().len(), 5);
}

#[test]
fn wrong_backups_are_refused() {
    let mut coord = Coordinator::in_memory();
    let (mut signers, key_id, backups) = wallet_with_backups(&mut coord, &["alice", "bob"], 2);

    // a signer that holds the key is told so instead of keeping a second copy
    let err = signers[0]
        .submit_backup(backups[0].0, backups[0].1.clone())
        .unwrap_err();
    assert!(err.contains("already holds"), "{err}");

    let mut twice = Coordinator::in_memory();
    let mut phone = Signer::create("phone".into());
    let code0 = type_backup(&mut phone, backups[0].0, &backups[0].1);
    let code1 = type_backup(&mut phone, backups[1].0, &backups[1].1);
    twice.read_restore_code(&scan(&code0)).unwrap();
    // two keys of one wallet on one signer can't rebuild it
    let up = twice.read_restore_code(&scan(&code1)).unwrap();
    assert!(up.unnamed_wallet.is_none() && up.broadcast.is_none());
    assert!(up.message.contains("of the same wallet"), "{}", up.message);
    assert!(twice.keys().is_empty() && twice.unnamed_wallets().is_empty());
    // ... until one is taken off the list and typed into another signer
    let second = twice
        .restoring_shares()
        .into_iter()
        .find(|s| s.share_index == backups[1].0)
        .unwrap();
    twice.remove_restoring_share(&second.id).unwrap();
    let mut other = Signer::create("other".into());
    let code = type_backup(&mut other, backups[1].0, &backups[1].1);
    assert!(
        twice
            .read_restore_code(&scan(&code))
            .unwrap()
            .unnamed_wallet
            .is_some()
    );

    // a known wallet: a signer already in it can't take another of its keys
    let mut third = Signer::create("third".into());
    let code = type_backup(&mut third, backups[0].0, &backups[0].1);
    let up = coord.read_restore_code(&scan(&code)).unwrap();
    coord.cancel().unwrap();
    assert!(up.confirm_each_signer);
    let code = type_backup(&mut third, backups[1].0, &backups[1].1);
    let err = coord.read_restore_code(&scan(&code)).unwrap_err();
    assert!(err.contains("already holds"), "{err}");

    // a backup of another wallet doesn't fit: it just stays on the list, and the wallet is unchanged
    let mut foreign = Coordinator::in_memory();
    let (_, _, foreign_backups) = wallet_with_backups(&mut foreign, &["x", "y"], 2);
    let mut phone = Signer::create("phone".into());
    let code = type_backup(&mut phone, foreign_backups[0].0, &foreign_backups[0].1);
    let up = coord.read_restore_code(&scan(&code)).unwrap();
    assert!(up.finished.is_none() && up.broadcast.is_none());
    assert_eq!(coord.keys().len(), 1);
    assert_eq!(coord.keys()[0].key_id, key_id);
    assert_eq!(coord.restoring_shares().len(), 1);

    // only a restore code is read here, and only there
    assert!(
        coord
            .read_restore_code(&scan(&signers[0].hello_frames()))
            .is_err()
    );
    let up = coord
        .start_display_backup(&key_id, &signers[0].device_id().to_string())
        .unwrap();
    assert!(
        coord
            .read_restore_code(&scan(&key_code(&signers[0])))
            .is_err(),
        "only one session at a time"
    );
    assert!(coord.receive(&scan(&key_code(&signers[0]))).is_err());
    assert!(up.confirm.is_some());
}

#[test]
fn the_same_key_on_two_signers_counts_once() {
    let mut coord = Coordinator::in_memory();
    let (mut signers, key_id, backups) =
        wallet_with_backups(&mut coord, &["alice", "bob", "carol"], 2);

    // alice's backup goes into a spare phone as well: the wallet keeps both, as one key
    let mut spare = Signer::create("spare".into());
    let code = type_backup(&mut spare, backups[0].0, &backups[0].1);
    let up = coord.read_restore_code(&scan(&code)).unwrap();
    finish_signers(&mut coord, &up, &mut [&mut spare]);
    assert!(up.message.contains("hold the same key #1"), "{}", up.message);
    assert!(up.message.contains("alice") && up.message.contains("spare"));
    let key = &coord.keys()[0];
    assert_eq!((key.threshold, key.n_keys, key.signers.len()), (2, 3, 4));
    assert_eq!(
        key.signers.iter().map(|s| s.share_index).collect::<Vec<_>>(),
        [1, 1, 2, 3]
    );

    // the same key twice is one signature, so it can't sign alone
    let both = [&signers[0], &spare].map(|s| s.device_id().to_string());
    let err = coord.begin_signing(&key_id, both.to_vec()).unwrap_err();
    assert!(err.contains("same key #1"), "{err}");
    signers.push(spare);
    fund_and_sign(&mut coord, &mut signers, &[3, 1], &key_id);

    // forgetting that alice holds it leaves the spare with key #1
    let alice = signers[0].device_id().to_string();
    coord.forget_share(&key_id, &alice).unwrap();
    assert!(coord.forget_share(&key_id, &alice).is_err(), "already forgotten");
    let key = &coord.keys()[0];
    assert_eq!((key.n_keys, key.signers.len()), (3, 3));
    assert_eq!(coord.signers().len(), 4, "alice is still enrolled");

    // forgetting bob altogether takes his key #2 out of the wallet
    let bob = signers[1].device_id().to_string();
    coord.forget_signer(&bob).unwrap();
    assert_eq!(coord.signers().len(), 3);
    let key = &coord.keys()[0];
    assert_eq!((key.n_keys, key.signers.len()), (2, 2));
    fund_and_sign(&mut coord, &mut signers, &[3, 2], &key_id);

    // alice rejoins with her restore code, since she kept her key
    let up = coord.read_restore_code(&scan(&key_code(&signers[0]))).unwrap();
    assert!(up.message.contains("same key #1"), "{}", up.message);
    assert_eq!(coord.keys()[0].signers.len(), 3);
}

#[test]
fn the_same_backup_typed_into_two_phones_while_restoring() {
    let mut orig = Coordinator::in_memory();
    let (_, key_id, backups) = wallet_with_backups(&mut orig, &["alice", "bob", "carol"], 2);

    let mut coord = Coordinator::in_memory();
    let mut phones = fresh_phones(&["p1", "p2", "p3"]);
    coord
        .read_restore_code(&scan(&type_backup(&mut phones[0], backups[0].0, &backups[0].1)))
        .unwrap();
    // a second copy of key #1 is not a second key
    let up = coord
        .read_restore_code(&scan(&type_backup(&mut phones[1], backups[0].0, &backups[0].1)))
        .unwrap();
    assert!(up.unnamed_wallet.is_none(), "one key is not enough");
    assert!(up.message.contains("the same key p1 gave"), "{}", up.message);
    assert_eq!(coord.restoring_shares().len(), 2);

    let up = coord
        .read_restore_code(&scan(&type_backup(&mut phones[2], backups[1].0, &backups[1].1)))
        .unwrap();
    let unnamed = up.unnamed_wallet.unwrap();
    assert_eq!(unnamed.signers.len(), 2, "one entry per key");
    assert!(unnamed.signers.iter().any(|s| s.contains("p1") && s.contains("p2") && s.ends_with("(same key #1)")));
    assert!(unnamed.signers.contains(&"p3".to_string()));
    let up = coord
        .name_restored_wallet(&unnamed.key_id, "family".into(), Network::Signet)
        .unwrap();
    assert!(up.message.contains("hold the same key #1"), "{}", up.message);
    let key = &coord.keys()[0];
    assert_eq!(key.key_id, key_id);
    assert_eq!((key.threshold, key.n_keys, key.signers.len()), (2, 2, 3));
    let [p1, p2, p3] = &mut phones[..] else { panic!() };
    finish_signers(&mut coord, &up, &mut [p1, p2, p3]);
    fund_and_sign(&mut coord, &mut phones, &[1, 2], &key_id);
}

#[test]
fn delete_a_wallet_and_restore_it_from_the_signers() {
    let mut coord = Coordinator::in_memory();
    let (mut signers, key_id) = create_wallet(&mut coord, &["alice", "bob"], 2);
    let up = coord
        .start_display_backup(&key_id, &signers[0].device_id().to_string())
        .unwrap();
    assert!(up.confirm.is_some());
    assert!(coord.delete_wallet(&key_id).is_err(), "not during a session");
    coord.cancel().unwrap();

    coord.delete_wallet(&key_id).unwrap();
    assert!(coord.keys().is_empty());
    assert!(coord.backup_shares(&key_id).is_err());
    assert!(coord.delete_wallet(&key_id).is_err());
    // the signers are still enrolled, and with no wallet left they can be forgotten as before
    assert_eq!(coord.signers().len(), 2);

    // the signers kept their keys: their codes bring the wallet back
    for s in &signers {
        coord.read_restore_code(&scan(&key_code(s))).unwrap();
    }
    assert_eq!(coord.keys()[0].key_id, key_id);
    fund_and_sign(&mut coord, &mut signers, &[0, 1], &key_id);
}

#[test]
fn a_signer_deletes_a_key_or_a_typed_in_backup() {
    let mut coord = Coordinator::in_memory();
    let (mut signers, key_id, backups) = wallet_with_backups(&mut coord, &["alice", "bob"], 2);

    // a typed-in backup that was finished into a key: deleting the key doesn't bring the
    // backup back, not even after a restart
    let mut spare = Signer::create("spare".into());
    let code = type_backup(&mut spare, backups[0].0, &backups[0].1);
    let up = coord.read_restore_code(&scan(&code)).unwrap();
    finish_signers(&mut coord, &up, &mut [&mut spare]);
    let snap = spare.delete_key(&key_id).unwrap();
    assert!(spare.keys().is_empty() && spare.pending_backups().is_empty());
    let spare = Signer::from_snapshot(&snap).unwrap();
    assert!(spare.keys().is_empty() && spare.pending_backups().is_empty());
    assert!(spare.restore_code(&key_id).is_err());

    // a backup still waiting for the coordinator
    let mut phone = Signer::create("phone".into());
    type_backup(&mut phone, backups[1].0, &backups[1].1);
    let id = phone.pending_backups()[0].id.clone();
    let snap = phone.delete_pending_backup(&id).unwrap();
    assert!(phone.pending_backups().is_empty());
    assert!(phone.delete_pending_backup(&id).is_err());
    let mut phone = Signer::from_snapshot(&snap).unwrap();
    assert!(phone.pending_backups().is_empty());
    // ... and it can be typed in again
    type_backup(&mut phone, backups[1].0, &backups[1].1);

    // a key on the device key store goes with its sealed copy, opened or not
    let snapshot = common::move_to_keystore(&mut signers[1], &key_id);
    let mut bob = Signer::from_snapshot(&snapshot).unwrap();
    let snap = bob.delete_key(&key_id).unwrap();
    assert!(bob.keys().is_empty());
    assert!(bob.keystore_share(&key_id).is_err());
    assert!(Signer::from_snapshot(&snap).unwrap().keys().is_empty());

    // alice is unaffected
    assert_eq!(signers[0].keys().len(), 1);
    assert!(signers[0].restore_code(&key_id).is_ok());
}

#[test]
fn one_signature_wallets() {
    let mut orig = Coordinator::in_memory();
    let (signers, key_id, backups) = wallet_with_backups(&mut orig, &["alice", "bob"], 1);

    // a key knows its threshold, so it needs nothing else
    let mut coord = Coordinator::in_memory();
    let up = coord
        .read_restore_code(&scan(&key_code(&signers[0])))
        .unwrap();
    assert!(matches!(up.finished, Some(Finished::WalletRestored { .. })));
    assert_eq!(coord.keys()[0].threshold, 1);

    // a backup can't tell, so the user says it is one: nothing can be checked
    let mut coord = Coordinator::in_memory();
    let mut phone = Signer::create("phone".into());
    let code = type_backup(&mut phone, backups[0].0, &backups[0].1);
    let up = coord.read_restore_code(&scan(&code)).unwrap();
    assert!(up.unnamed_wallet.is_none());
    let id = coord.restoring_shares()[0].id.clone();
    let up = coord
        .restore_one_signature_wallet(&id, "solo".into(), Network::Signet)
        .unwrap();
    assert_eq!(coord.keys()[0].key_id, key_id);
    let key = finish_signers(&mut coord, &up, &mut [&mut phone]);
    let mut all = vec![phone];
    fund_and_sign(&mut coord, &mut all, &[0], &key);
}

#[test]
fn a_key_on_the_key_store_is_opened_for_its_restore_code() {
    let mut orig = Coordinator::in_memory();
    let (mut signers, key_id) = create_wallet(&mut orig, &["alice", "bob"], 2);
    let snapshot = common::move_to_keystore(&mut signers[0], &key_id);
    let mut alice = Signer::from_snapshot(&snapshot).unwrap();
    assert!(matches!(
        alice.restore_code(&key_id).unwrap(),
        Step::UnlockShare { .. }
    ));
    common::open_from_keystore(&mut alice, &key_id);
    let mut coord = Coordinator::in_memory();
    coord
        .read_restore_code(&scan(&restore_frames(alice.restore_code(&key_id).unwrap())))
        .unwrap();
    assert_eq!(
        coord.restoring_shares()[0].key_name.as_deref(),
        Some("family")
    );
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
    assert_eq!(
        shares
            .iter()
            .filter(|s| s.complete)
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>(),
        ["bob"]
    );
    // the unfinished ones can be marked by hand, as in frostsnap
    let carol = shares
        .iter()
        .find(|s| s.name == "carol")
        .unwrap()
        .share_index;
    coord.mark_backup_complete(&key_id, carol).unwrap();
    let coord = Coordinator::open(&dir).unwrap();
    assert_eq!(
        coord
            .backup_shares(&key_id)
            .unwrap()
            .iter()
            .filter(|s| s.complete)
            .count(),
        2
    );
    drop(coord);

    // shares read so far are kept too, signers included
    let dir2 = temp_dir("restart-restoring");
    let mut lost = Coordinator::open(&dir2).unwrap();
    lost.read_restore_code(&scan(&key_code(&signers[0])))
        .unwrap();
    let lost = Coordinator::open(&dir2).unwrap();
    assert_eq!(lost.restoring_shares().len(), 1);
    assert_eq!(lost.signers().len(), 1);
    let mut lost = lost;
    let up = lost
        .read_restore_code(&scan(&key_code(&signers[1])))
        .unwrap();
    assert!(matches!(up.finished, Some(Finished::WalletRestored { .. })));
    let lost = Coordinator::open(&dir2).unwrap();
    assert_eq!(lost.keys().len(), 1);
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&dir2);
}

/// A coordinator file from before the shares were gathered outside `frostsnap_core`: a restoration
/// in progress there carries on, and its wallet can be rebuilt by the next code.
#[test]
fn a_restoration_from_an_older_file_carries_on() {
    use frostsnap_core::RestorationId;
    use frostsnap_core::coordinator::{Mutation, restoration::RestorationMutation};
    use frostsnap_core::device::KeyPurpose;

    let mut orig = Coordinator::in_memory();
    let (signers, key_id) = create_wallet(&mut orig, &["alice", "bob"], 2);
    drop(orig);

    // alice's share is in a restoration, as the older flow left it
    let Packet::RestoreShare {
        device_id, share, ..
    } = Packet::from_bytes(&scan(&key_code(&signers[0]))).unwrap()
    else {
        panic!()
    };
    let restoration_id = RestorationId([7; 16]);
    let history = vec![
        Mutation::Restoration(RestorationMutation::NewRestoration2 {
            restoration_id,
            key_name: "family".into(),
            starting_threshold: Some(2),
            key_purpose: KeyPurpose::Bitcoin(Network::Signet),
        }),
        Mutation::Restoration(RestorationMutation::RestorationProgress2 {
            restoration_id,
            device_id,
            held_share: share,
        }),
    ];
    // what version 2 wrote, field by field
    let signers_map = std::collections::BTreeMap::from([(device_id, "alice".to_string())]);
    let backups: Vec<(frostsnap_core::AccessStructureRef, u32, bool)> = vec![];
    let v2 = ([9u8; 32], signers_map, history, None::<String>, backups);
    let mut bytes = vec![2u8];
    bytes.extend(bincode::encode_to_vec(&v2, bincode::config::standard()).unwrap());

    let dir = temp_dir("migrate");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("coordinator.bin"), bytes).unwrap();
    let mut coord = Coordinator::open(&dir).unwrap();
    assert_eq!(coord.restoring_shares().len(), 1);
    assert_eq!(coord.signers().len(), 1);
    drop(coord);
    let mut coord = Coordinator::open(&dir).unwrap();
    assert_eq!(coord.restoring_shares().len(), 1, "moved for good");
    let up = coord
        .read_restore_code(&scan(&key_code(&signers[1])))
        .unwrap();
    assert!(matches!(
        up.finished,
        Some(Finished::WalletRestored { key_id: ref k, .. }) if *k == key_id
    ));
    let _ = std::fs::remove_dir_all(&dir);
}
