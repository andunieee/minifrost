//! Shares moved to the device key store: the snapshot on disk no longer carries them, a restarted
//! signer asks for the share to be opened when a message needs it, and a share can go back to disk.

use minifrost_coordinator::engine::{Coordinator, Finished};

mod common;
use common::{
    Prompt, Signer, Step, create_wallet, frames_of, fund_and_sign, move_to_keystore,
    open_from_keystore, signer_step,
};

/// Ask `signer` to display its backup, opening its share from the key store when asked to.
/// Returns whether it had to be opened.
fn display_backup(coord: &mut Coordinator, signer: &mut Signer, key_id: &str) -> bool {
    let up = coord
        .start_display_backup(key_id, &signer.device_id().to_string())
        .unwrap();
    let frames = frames_of(&up);
    let mut opened = false;
    let mut step = signer_step(signer, &frames);
    if let Step::UnlockShare {
        key_id: k,
        key_name,
    } = &step
    {
        assert_eq!(k, key_id);
        assert_eq!(key_name, "family");
        open_from_keystore(signer, key_id);
        opened = true;
        step = signer_step(signer, &frames);
    }
    let Step::Confirm(Prompt::DisplayBackup { words, .. }) = step else {
        panic!("expected the backup to be displayed, got {step:?}")
    };
    assert_eq!(words.len(), 25);
    assert!(matches!(signer.approve().unwrap().step, Step::Done { .. }));
    let up = coord.confirm_backup().unwrap();
    assert!(matches!(up.finished, Some(Finished::BackupRecorded { .. })));
    opened
}

fn restart(s: &Signer, snapshot: &[u8]) -> Signer {
    let s2 = Signer::from_snapshot(snapshot).unwrap();
    assert_eq!(s2.device_id(), s.device_id());
    s2
}

#[test]
fn share_on_keystore_is_opened_when_needed_and_can_go_back_to_disk() {
    let dir = std::env::temp_dir().join(format!("minifrost-keystore-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut coord = Coordinator::open(&dir).unwrap();
    let (mut signers, key_id) = create_wallet(&mut coord, &["alice", "bob", "carol"], 2);

    // --- move alice's share to the key store, then restart her from what is on disk
    assert!(!signers[0].keys()[0].on_keystore);
    let snap = move_to_keystore(&mut signers[0], &key_id);
    assert!(
        signers[0].share_for_keystore(&key_id).is_err(),
        "already moved"
    );
    signers[0] = restart(&signers[0], &snap);
    assert!(signers[0].keys()[0].on_keystore);
    assert!(
        signers[0].move_to_disk(&key_id).is_err(),
        "a share not opened yet can't go back to disk"
    );
    assert!(
        signers[0]
            .open_keystore_share(&key_id, b"not a share".to_vec())
            .is_err()
    );

    // signing round 2 asks for the share (fund_and_sign opens it), round 1 doesn't need it
    fund_and_sign(&mut coord, &mut signers, &[0, 1], &key_id);

    // opened for the rest of the session, closed again after a restart
    assert!(!display_backup(&mut coord, &mut signers[0], &key_id));
    signers[0] = restart(&signers[0], &signers[0].snapshot());
    assert!(display_backup(&mut coord, &mut signers[0], &key_id));

    // --- back to disk: no more opening after a restart
    let snap = signers[0].move_to_disk(&key_id).unwrap();
    signers[0] = restart(&signers[0], &snap);
    assert!(!signers[0].keys()[0].on_keystore);
    assert!(signers[0].keystore_share(&key_id).is_err());
    assert!(!display_backup(&mut coord, &mut signers[0], &key_id));

    // --- the "new shares on the key store" setting is kept
    assert!(!signers[1].new_shares_on_keystore());
    let snap = signers[1].set_new_shares_on_keystore(true);
    assert!(restart(&signers[1], &snap).new_shares_on_keystore());

    let _ = std::fs::remove_dir_all(&dir);
}
