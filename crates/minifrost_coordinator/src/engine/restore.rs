//! Backups and wallet restoration:
//!
//! * **Backup run**: after a key is created every share is "not backed up" until its signer has
//!   displayed the 25 words and the user confirmed writing them down. A backup can be checked
//!   afterwards with the on-signer quiz. (`frostsnap_coordinator::{display_backup, check_backup,
//!   backup_run}`.)
//! * **Restoration** starts on the signers: a signer shows a *restore code* for a backup the user
//!   typed into it, or for a key it holds. The coordinator reads those codes one after another and
//!   sorts them out by itself. As soon as the shares it has determine a key it saves the wallet,
//!   and signers restored from a typed-in backup get one QR code with what they are missing, which
//!   *consolidates* their backup into a real encrypted share.
//!
//! All of it is `frostsnap_core`. A restoration in `frostsnap_core` has to be given its wallet's
//! name and network when it starts, which a backup can't tell before the wallet is rebuilt, so the
//! shares are gathered here and go through a `frostsnap_core` restoration all at once.

use std::collections::{BTreeMap, BTreeSet};

use bitcoin::Network;
use frost_backup::{FINGERPRINT, recovery::find_valid_subset};
use frostsnap_core::{
    AccessStructureRef, DeviceId, KeyId, RestorationId,
    coordinator::{CoordinatorSend, KeyLocationState, restoration::RecoverShare},
    device::KeyPurpose,
    message::{DeviceRestoration, DeviceToCoordinatorMessage},
    schnorr_fun::frost::{ShareImage, ShareIndex, SharedKey},
};
use minifrost_wire::{Packet, Step as WireStep};
use rand::rngs::OsRng;

use super::{Coordinator, Finished, Res, Session, SignerInfo, Update, purpose_network};

// ---------------------------------------------------------------------------------------------
// What the UI sees
// ---------------------------------------------------------------------------------------------

/// One share of a wallet and whether its backup was recorded.
#[derive(Debug, Clone)]
pub struct BackupShare {
    pub device_id: String,
    pub name: String,
    pub share_index: u32,
    /// Like frostsnap: a share with no backup record at all counts as backed up (it was restored
    /// from a backup, or predates backup tracking).
    pub complete: bool,
}

/// A share read from a signer whose wallet isn't rebuilt yet.
#[derive(Debug, Clone)]
pub struct RestoringShareView {
    pub id: String,
    pub device_id: String,
    pub device_name: String,
    pub share_index: u32,
    /// What the share knows about its wallet: nothing, for a typed-in backup.
    pub key_name: Option<String>,
    pub threshold: Option<u16>,
    pub network: Option<String>,
}

/// A wallet there are enough shares to rebuild, but whose name and network no share knows.
#[derive(Debug, Clone)]
pub struct UnnamedWalletView {
    pub key_id: String,
    pub threshold: u16,
    pub signers: Vec<String>,
}

/// A signer whose typed-in backup still has to be consolidated into a share of a known wallet.
#[derive(Debug, Clone)]
pub struct PendingConsolidationView {
    pub key_id: String,
    pub key_name: String,
    pub device_id: String,
    pub device_name: String,
    pub share_index: u32,
}

// ---------------------------------------------------------------------------------------------
// Sessions
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum BackupSession {
    Display {
        device: DeviceId,
        access_structure_ref: AccessStructureRef,
        share_index: ShareIndex,
    },
    Check {
        device: DeviceId,
        access_structure_ref: AccessStructureRef,
        share_index: ShareIndex,
    },
    /// The wallet is rebuilt: these signers read one code with what they are missing, and the user
    /// confirms each.
    FinishRestoring {
        access_structure_ref: AccessStructureRef,
        devices: BTreeSet<DeviceId>,
    },
}

impl BackupSession {
    pub(super) fn devices(&self) -> BTreeSet<DeviceId> {
        match self {
            Self::Display { device, .. } | Self::Check { device, .. } => [*device].into(),
            Self::FinishRestoring { devices, .. } => devices.clone(),
        }
    }
}

fn index_u32(i: ShareIndex) -> u32 {
    i.try_into().expect("share index fits in a u32")
}

// ---------------------------------------------------------------------------------------------

impl Coordinator {
    pub(super) fn parse_device(&self, device_id: &str) -> Res<DeviceId> {
        device_id.parse().map_err(|_| "bad device id".to_string())
    }

    fn access_ref_of(&self, key_id: &str) -> Res<AccessStructureRef> {
        let key = self
            .inner
            .iter_keys()
            .find(|k| k.key_id.to_string() == key_id)
            .ok_or("unknown wallet")?;
        Ok(key.master_access_structure().access_structure_ref())
    }

    fn key_name(&self, key_id: KeyId) -> String {
        self.inner
            .get_frost_key(key_id)
            .map(|k| k.key_name.clone())
            .unwrap_or_default()
    }

    fn require_idle(&self) -> Res<()> {
        match self.session {
            Session::Idle => Ok(()),
            _ => Err(
                "Another keygen, signing or backup session is in progress. Cancel it first.".into(),
            ),
        }
    }

    fn start_backup_session(
        &mut self,
        session: BackupSession,
        sends: Vec<CoordinatorSend>,
        step: WireStep,
        message: &str,
    ) -> Res<Update> {
        let devices = session.devices();
        self.session = Session::Backup(session);
        self.replied.clear();
        self.save()?;
        let mut update = Update {
            message: message.to_string(),
            broadcast: Some(self.broadcast(sends, step)),
            ..Default::default()
        };
        self.fill_awaiting(&mut update, &devices);
        Ok(update)
    }

    // ------------------------------------------------------------------ backup run ----------

    /// Every share of the wallet with its backup status.
    pub fn backup_shares(&self, key_id: &str) -> Res<Vec<BackupShare>> {
        let aref = self.access_ref_of(key_id)?;
        let acc = self
            .inner
            .get_access_structure(aref)
            .ok_or("unknown wallet")?;
        Ok(acc
            .device_to_share_indicies()
            .into_iter()
            .map(|(device, index)| {
                let index = index_u32(index);
                BackupShare {
                    device_id: device.to_string(),
                    name: self.name_of(&device),
                    share_index: index,
                    complete: self.backups.get(&(aref, index)).copied() != Some(false),
                }
            })
            .collect())
    }

    /// Record by hand that a share's backup exists (frostsnap's "Mark as backed up").
    pub fn mark_backup_complete(&mut self, key_id: &str, share_index: u32) -> Res<()> {
        let aref = self.access_ref_of(key_id)?;
        self.backups.insert((aref, share_index), true);
        self.save()
    }

    pub(super) fn start_backup_run(&mut self, aref: AccessStructureRef) {
        let Some(acc) = self.inner.get_access_structure(aref) else {
            return;
        };
        for (_, index) in acc.iter_shares() {
            self.backups.insert((aref, index_u32(index)), false);
        }
    }

    /// Ask a signer to show its backup. It replies once the user confirms writing it down.
    pub fn start_display_backup(&mut self, key_id: &str, device_id: &str) -> Res<Update> {
        self.require_idle()?;
        let device = self.parse_device(device_id)?;
        let aref = self.access_ref_of(key_id)?;
        let share_index = self.share_index_of(aref, device)?;
        let sends = self
            .inner
            .request_device_display_backup(device, aref, self.encryption_key)
            .map_err(|e| e.to_string())?;
        let mut update = self.start_backup_session(
            BackupSession::Display { device, access_structure_ref: aref, share_index },
            sends,
            WireStep::ShowBackup,
            "Show this to the signer. It will display 25 words: write them down in order, then confirm here.",
        )?;
        update.confirm = Some("I wrote down the 25 words".into());
        Ok(update)
    }

    /// Confirm the current no-reply step: the user wrote a displayed backup down, or passed the
    /// signer's backup check. Both are recorded here instead of with a scanned reply.
    pub fn confirm_backup(&mut self) -> Res<Update> {
        let displayed = match &self.session {
            Session::Backup(BackupSession::Display {
                device,
                access_structure_ref,
                share_index,
            }) => Some((*device, *access_structure_ref, *share_index)),
            _ => None,
        };
        if let Some((device, aref, share_index)) = displayed {
            return self.finish_displayed_backup(device, aref, share_index);
        }
        let checked = match &self.session {
            Session::Backup(BackupSession::Check {
                device,
                access_structure_ref,
                share_index,
            }) => Some((*device, *access_structure_ref, *share_index)),
            _ => None,
        };
        if let Some((device, aref, share_index)) = checked {
            self.session = Session::Idle;
            self.replied.clear();
            return Ok(Update {
                message: format!("{} passed the backup check.", self.name_of(&device)),
                finished: Some(Finished::BackupChecked {
                    key_name: self.key_name(aref.key_id),
                    share_index: index_u32(share_index),
                }),
                ..Default::default()
            });
        }
        Err("Nothing is waiting for a confirmation.".into())
    }

    fn finish_displayed_backup(
        &mut self,
        device: DeviceId,
        aref: AccessStructureRef,
        share_index: ShareIndex,
    ) -> Res<Update> {
        self.backups.insert((aref, index_u32(share_index)), true);
        self.session = Session::Idle;
        self.replied.clear();
        self.save()?;
        Ok(Update {
            message: format!(
                "{}'s backup was marked as written down.",
                self.name_of(&device)
            ),
            finished: Some(Finished::BackupRecorded {
                key_name: self.key_name(aref.key_id),
                share_index: index_u32(share_index),
            }),
            snapshot_saved: self.dir.is_some(),
            ..Default::default()
        })
    }

    /// Ask a signer to quiz the user on their paper backup.
    pub fn start_check_backup(&mut self, key_id: &str, device_id: &str) -> Res<Update> {
        self.require_idle()?;
        let device = self.parse_device(device_id)?;
        let aref = self.access_ref_of(key_id)?;
        let share_index = self.share_index_of(aref, device)?;
        let sends = self
            .inner
            .request_device_check_backup(device, aref, self.encryption_key)
            .map_err(|e| e.to_string())?;
        let mut update = self.start_backup_session(
            BackupSession::Check {
                device,
                access_structure_ref: aref,
                share_index,
            },
            sends,
            WireStep::CheckBackup,
            "Show this to the signer, then answer its quiz using your paper backup. When the signer says the check is complete, confirm here.",
        )?;
        update.confirm = Some("The signer said the check is complete".into());
        Ok(update)
    }

    fn share_index_of(&self, aref: AccessStructureRef, device: DeviceId) -> Res<ShareIndex> {
        self.inner
            .get_access_structure(aref)
            .and_then(|a| a.device_to_share_indicies().get(&device).copied())
            .ok_or_else(|| format!("{} holds no share of this wallet.", self.name_of(&device)))
    }

    // ------------------------------------------------------------------ restoration ---------

    /// A signer's restore code, read on the Restore page. It doesn't matter whether the share came
    /// from a typed-in backup or from a key the signer holds, or whether this coordinator knows
    /// the signer or the wallet: the share either belongs to a wallet this coordinator has, or it
    /// joins the shares gathered so far, which may now be enough to rebuild a wallet.
    pub fn read_restore_code(&mut self, payload: &[u8]) -> Res<Update> {
        self.require_idle()?;
        let packet = Packet::from_bytes(payload).map_err(|e| e.to_string())?;
        let Packet::RestoreShare {
            device_id,
            name,
            share,
        } = packet
        else {
            return Err("That is not a restore code. On the signer, type in a backup or tap a key and choose “Restore wallet on coordinator”.".into());
        };
        let from = self.register_signer(device_id, &name);
        let share = RecoverShare {
            held_by: device_id,
            held_share: share,
        };
        let wallet = self.wallet_of(&share);
        let mut update = match wallet {
            Some(aref) => self.add_to_wallet(aref, &share)?,
            None => self.gather(share)?,
        };
        update.from = Some(from);
        self.save()?;
        update.snapshot_saved = self.dir.is_some();
        Ok(update)
    }

    /// The wallet this coordinator has that `share` belongs to.
    fn wallet_of(&self, share: &RecoverShare) -> Option<AccessStructureRef> {
        let location = self
            .inner
            .find_share(share.held_share.share_image, self.encryption_key)?;
        match location.key_state {
            KeyLocationState::Complete {
                access_structure_ref,
            } => Some(access_structure_ref),
            KeyLocationState::Restoring { .. } => None,
        }
    }

    fn add_to_wallet(&mut self, aref: AccessStructureRef, share: &RecoverShare) -> Res<Update> {
        let device = share.held_by;
        let index = share.held_share.share_image.index;
        let name = self.name_of(&device);
        let key_name = self.key_name(aref.key_id);
        match self.share_index_of(aref, device) {
            Ok(held) if held == index => {
                return Ok(Update {
                    message: format!("{name} is already part of “{key_name}”."),
                    repeat: true,
                    ..Default::default()
                });
            }
            Ok(held) => {
                return Err(format!(
                    "{name} already holds key #{} of “{key_name}”, and a signer holds one key of a wallet. Type this backup into another signer.",
                    index_u32(held)
                ));
            }
            Err(_) => {}
        }
        self.inner
            .recover_share(aref, share, self.encryption_key)
            .map_err(|e| e.to_string())?;
        self.save()?;
        self.finish_signers(
            aref,
            format!("Key #{} of “{key_name}” is on {name}.", index_u32(index)),
        )
    }

    fn gather(&mut self, share: RecoverShare) -> Res<Update> {
        let name = self.name_of(&share.held_by);
        let index = index_u32(share.held_share.share_image.index);
        if self.restoring.contains(&share) {
            let mut update = self.rebuild_if_possible(Update {
                message: format!("Already read key #{index} from {name}."),
                repeat: true,
                ..Default::default()
            })?;
            // asked once, when the share came in; the Restore page keeps offering it
            update.unnamed_wallet = None;
            return Ok(update);
        }
        self.restoring.push(share);
        self.rebuild_if_possible(Update {
            message: format!(
                "Read key #{index} from {name}. The wallet is rebuilt as soon as there are enough codes."
            ),
            ..Default::default()
        })
    }

    /// Rebuild a wallet if the shares gathered so far determine one. A wallet nobody knows the name
    /// and network of comes back as `Update::unnamed_wallet`: its shares stay gathered until the
    /// user gives them with `name_restored_wallet`.
    fn rebuild_if_possible(&mut self, mut update: Update) -> Res<Update> {
        for found in self.rebuilds() {
            if let Some(problem) = self.signer_twice(&found) {
                update.message = problem;
                continue;
            }
            match found.details() {
                Some((name, purpose)) => return self.rebuild(found, name, purpose),
                None => {
                    update.unnamed_wallet = Some(self.unnamed_view(&found));
                    update.message = "There are enough codes to rebuild a wallet.".into();
                }
            }
        }
        Ok(update)
    }

    /// Every wallet the shares gathered so far determine, with the shares that belong to it.
    fn rebuilds(&self) -> Vec<Rebuild> {
        let mut left = self.restoring.clone();
        let mut found = vec![];
        while let Some(key) = find_key(&left) {
            let (shares, rest): (Vec<_>, Vec<_>) = left.into_iter().partition(|s| {
                let image = s.held_share.share_image;
                key.share_image(image.index) == image
            });
            left = rest;
            if shares.is_empty() {
                break;
            }
            found.push(Rebuild { key, shares });
        }
        found
    }

    /// A signer holds one key of a wallet, so two of them typed into one signer can't both count.
    fn signer_twice(&self, found: &Rebuild) -> Option<String> {
        let mut seen = BTreeMap::new();
        for s in &found.shares {
            let index = index_u32(s.held_share.share_image.index);
            if let Some(other) = seen.insert(s.held_by, index)
                && other != index
            {
                return Some(format!(
                    "{} has keys #{other} and #{index} of the same wallet, and a signer holds one key of a wallet. Remove one from the list, then type that backup into another signer.",
                    self.name_of(&s.held_by)
                ));
            }
        }
        None
    }

    /// Save the wallet, then give the signers that typed in a backup what they are missing.
    fn rebuild(&mut self, found: Rebuild, name: String, purpose: KeyPurpose) -> Res<Update> {
        if let Some(problem) = self.signer_twice(&found) {
            return Err(problem);
        }
        let key = self.encryption_key;
        let id = RestorationId::new(&mut OsRng);
        let threshold = found.key.threshold() as u16;
        self.inner
            .start_restoring_key(name.clone(), Some(threshold), purpose, id);
        let added = found.shares.iter().try_for_each(|s| {
            self.inner
                .add_recovery_share_to_restoration(id, s, key)
                .map_err(|e| e.to_string())
        });
        let aref = added.and_then(|()| {
            self.inner
                .finish_restoring(id, key, &mut OsRng)
                .map_err(|e| e.to_string())
        });
        let aref = match aref {
            Ok(aref) => aref,
            Err(e) => {
                self.inner.cancel_restoration(id);
                return Err(e);
            }
        };
        self.restoring.retain(|s| !found.shares.contains(s));
        self.save()?;
        self.finish_signers(aref, format!("Wallet “{name}” is rebuilt and saved."))
    }

    /// Every signer of the wallet still waiting for the coordinator's answer gets it from one QR
    /// code. With none waiting, there is nothing left to do.
    fn finish_signers(&mut self, aref: AccessStructureRef, done: String) -> Res<Update> {
        let key_id = aref.key_id.to_string();
        let devices = self
            .pending_consolidations()
            .iter()
            .filter(|p| p.key_id == key_id)
            .map(|p| self.parse_device(&p.device_id))
            .collect::<Res<BTreeSet<_>>>()?;
        if devices.is_empty() {
            return Ok(Update {
                message: done,
                finished: Some(Finished::WalletRestored {
                    key_id,
                    name: self.key_name(aref.key_id),
                }),
                snapshot_saved: self.dir.is_some(),
                ..Default::default()
            });
        }
        let mut sends = vec![];
        for &device in &devices {
            sends.extend(
                self.inner
                    .consolidate_pending_physical_backups(device, aref, self.encryption_key)
                    .map_err(|e| e.to_string())?,
            );
        }
        self.session = Session::Backup(BackupSession::FinishRestoring {
            access_structure_ref: aref,
            devices: devices.clone(),
        });
        self.replied.clear();
        let mut update = self.next_step(
            sends,
            WireStep::FinishRestoring,
            &format!("{done} Show this to each signer that took a backup: it saves its key. Then confirm it here."),
        );
        update.confirm_each_signer = true;
        self.fill_awaiting(&mut update, &devices);
        update.snapshot_saved = self.dir.is_some();
        Ok(update)
    }

    /// A signer read the code of `finish_signers` and saved its key. Its `FinishedConsolidation`
    /// says only that, so the coordinator feeds frostsnap the message itself instead of scanning a
    /// QR back.
    pub(super) fn confirm_restored_signer(&mut self, device: DeviceId) -> Res<Update> {
        let Session::Backup(BackupSession::FinishRestoring {
            access_structure_ref: aref,
            devices,
        }) = &self.session
        else {
            return Err("No signer is waiting for a confirmation.".into());
        };
        let (aref, devices) = (*aref, devices.clone());
        let name = self.name_of(&device);
        if !devices.contains(&device) {
            return Err(format!("{name} is not part of this session."));
        }
        if self.replied.contains(&device) {
            return Err(format!("{name} was already confirmed."));
        }
        let share_index = self.share_index_of(aref, device)?;
        self.inner
            .recv_device_message(
                device,
                DeviceToCoordinatorMessage::Restoration(DeviceRestoration::FinishedConsolidation {
                    access_structure_ref: aref,
                    share_index,
                }),
            )
            .map_err(|e| e.to_string())?;
        self.replied.insert(device);
        let mut update = Update {
            message: format!("{name} confirmed."),
            from: Some(SignerInfo {
                device_id: device.to_string(),
                name,
            }),
            ..Default::default()
        };
        if self.replied.len() == devices.len() {
            self.session = Session::Idle;
            self.replied.clear();
            let key_name = self.key_name(aref.key_id);
            update.message = format!("Every signer saved its key of “{key_name}”.");
            update.finished = Some(Finished::WalletRestored {
                key_id: aref.key_id.to_string(),
                name: key_name,
            });
        } else {
            update.confirm_each_signer = true;
            self.fill_awaiting(&mut update, &devices);
        }
        self.save()?;
        update.snapshot_saved = self.dir.is_some();
        Ok(update)
    }

    /// The code that gives a wallet's waiting signers their keys, shown again.
    pub fn start_finish_restoring(&mut self, key_id: &str) -> Res<Update> {
        self.require_idle()?;
        let aref = self.access_ref_of(key_id)?;
        if !self
            .pending_consolidations()
            .iter()
            .any(|p| p.key_id == key_id)
        {
            return Err("No signer of this wallet is waiting for its key.".into());
        }
        self.finish_signers(aref, format!("Wallet “{}”.", self.key_name(aref.key_id)))
    }

    /// The shares read so far whose wallet isn't rebuilt yet.
    pub fn restoring_shares(&self) -> Vec<RestoringShareView> {
        self.restoring
            .iter()
            .map(|s| RestoringShareView {
                id: share_id(s),
                device_id: s.held_by.to_string(),
                device_name: self.name_of(&s.held_by),
                share_index: index_u32(s.held_share.share_image.index),
                key_name: s.held_share.key_name.clone(),
                threshold: s.held_share.threshold,
                network: s
                    .held_share
                    .purpose
                    .as_ref()
                    .map(|p| purpose_network(p).to_string()),
            })
            .collect()
    }

    /// Wallets there are enough shares for, waiting for the user to name them.
    pub fn unnamed_wallets(&self) -> Vec<UnnamedWalletView> {
        self.rebuilds()
            .iter()
            .filter(|found| found.details().is_none() && self.signer_twice(found).is_none())
            .map(|found| self.unnamed_view(found))
            .collect()
    }

    fn unnamed_view(&self, found: &Rebuild) -> UnnamedWalletView {
        UnnamedWalletView {
            key_id: found.key_id().to_string(),
            threshold: found.key.threshold() as u16,
            signers: found
                .shares
                .iter()
                .map(|s| self.name_of(&s.held_by))
                .collect(),
        }
    }

    pub fn remove_restoring_share(&mut self, id: &str) -> Res<()> {
        self.restoring.retain(|s| share_id(s) != id);
        self.save()
    }

    /// The name and network a backup doesn't carry, for a wallet of `unnamed_wallets`.
    pub fn name_restored_wallet(
        &mut self,
        key_id: &str,
        name: String,
        network: Network,
    ) -> Res<Update> {
        self.require_idle()?;
        let name = valid_wallet_name(name)?;
        let found = self
            .rebuilds()
            .into_iter()
            .find(|found| found.key_id().to_string() == key_id)
            .ok_or("Those shares are no longer here.")?;
        self.rebuild(found, name, KeyPurpose::Bitcoin(network))
    }

    /// Rebuild a one-signature wallet from a typed-in backup. Only here does the user decide: there
    /// is nothing to check a single backup against, while any higher threshold is found by itself.
    pub fn restore_one_signature_wallet(
        &mut self,
        id: &str,
        name: String,
        network: Network,
    ) -> Res<Update> {
        self.require_idle()?;
        let name = valid_wallet_name(name)?;
        let image = self
            .restoring
            .iter()
            .find(|s| share_id(s) == id)
            .ok_or("That share is no longer here.")?
            .held_share
            .share_image;
        let key = single_share_key(image).ok_or("That share can't be a wallet.")?;
        let shares = self
            .restoring
            .iter()
            .filter(|s| s.held_share.share_image == image)
            .cloned()
            .collect();
        self.rebuild(Rebuild { key, shares }, name, KeyPurpose::Bitcoin(network))
    }

    /// Signers restored from a typed-in backup whose key is still waiting for the coordinator.
    pub fn pending_consolidations(&self) -> Vec<PendingConsolidationView> {
        let mut devices: BTreeSet<DeviceId> = self.signers.keys().copied().collect();
        for k in self.inner.iter_keys() {
            devices.extend(k.master_access_structure().devices());
        }
        let mut out = vec![];
        for device in devices {
            for aref in self.inner.pending_physical_consolidations(device) {
                let Some(share_index) = self.share_index_of(aref, device).ok() else {
                    continue;
                };
                out.push(PendingConsolidationView {
                    key_id: aref.key_id.to_string(),
                    key_name: self.key_name(aref.key_id),
                    device_id: device.to_string(),
                    device_name: self.name_of(&device),
                    share_index: index_u32(share_index),
                });
            }
        }
        out
    }

    /// A coordinator file from before the shares were gathered here: move each restoration in
    /// progress over. Their shares already carry the wallet's name and network.
    pub(super) fn migrate_restorations(&mut self) {
        for r in self.inner.restoring().collect::<Vec<_>>() {
            self.restoring.extend(r.access_structure.held_shares);
            self.inner.cancel_restoration(r.restoration_id);
        }
    }

    fn next_step(&mut self, sends: Vec<CoordinatorSend>, step: WireStep, message: &str) -> Update {
        Update {
            message: message.to_string(),
            broadcast: Some(self.broadcast(sends, step)),
            ..Default::default()
        }
    }
}

/// Shares that determine a wallet.
struct Rebuild {
    key: SharedKey,
    shares: Vec<RecoverShare>,
}

impl Rebuild {
    fn key_id(&self) -> KeyId {
        AccessStructureRef::from_root_shared_key(&self.key).key_id
    }

    /// The wallet's name and network, when a share knows them: a backup alone doesn't.
    fn details(&self) -> Option<(String, KeyPurpose)> {
        self.shares
            .iter()
            .find_map(|s| Some((s.held_share.key_name.clone()?, s.held_share.purpose?)))
    }
}

/// The key `shares`, or some of them, determine.
fn find_key(shares: &[RecoverShare]) -> Option<SharedKey> {
    // A share that knows its wallet needs one signature is that wallet's key by itself. Nothing
    // else can say so: the fingerprint only tells shares apart from two of them up.
    let single = shares
        .iter()
        .filter(|s| s.held_share.threshold == Some(1))
        .find_map(|s| single_share_key(s.held_share.share_image));
    if single.is_some() {
        return single;
    }
    let images: Vec<_> = shares.iter().map(|s| s.held_share.share_image).collect();
    let (_, key) = find_valid_subset(&images, FINGERPRINT, None)?;
    let key = key.non_zero()?;
    // The fingerprint is a short hash. A share that knows its wallet can overrule it.
    let aref = AccessStructureRef::from_root_shared_key(&key);
    shares
        .iter()
        .filter(|s| key.share_image(s.held_share.share_image.index) == s.held_share.share_image)
        .all(|s| s.held_share.access_structure_ref.is_none_or(|a| a == aref))
        .then_some(key)
}

fn single_share_key(image: ShareImage) -> Option<SharedKey> {
    SharedKey::from_share_images(vec![image]).non_zero()
}

fn share_id(s: &RecoverShare) -> String {
    format!("{}:{}", s.held_by, s.held_share.share_image.image)
}

fn valid_wallet_name(name: String) -> Res<String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("Give the wallet a name.".into());
    }
    Ok(name)
}
