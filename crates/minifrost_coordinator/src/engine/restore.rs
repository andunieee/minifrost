//! Backups and wallet restoration, driven the way frostsnap drives them:
//!
//! * **Backup run**: after a key is created every share is "not backed up" until its signer has
//!   displayed the 25 words and the user confirmed writing them down. A backup can be checked
//!   afterwards with the on-signer quiz. (`frostsnap_coordinator::{display_backup, check_backup,
//!   backup_run}`.)
//! * **Restoration**: a wallet is rebuilt from at least a threshold of shares. Each share arrives
//!   either from a signer that already holds it, or from a physical backup the user types into a
//!   signer. Once the shares determine the key the restoration is finished, and signers that were
//!   restored from a physical backup are asked to *consolidate* it into a real encrypted share.
//!
//! All of it is `frostsnap_core`; this file only decides which QR goes out next and what the
//! screens may show. Each action here is a short session in the same shape as keygen and signing:
//! a broadcast QR for the signers, a scan of the reply, repeat.

use bitcoin::Network;
use frostsnap_core::{
    coordinator::{
        restoration::{PhysicalBackupPhase, RecoverShare, ShareCompatibility, ToUserRestoration},
        CoordinatorSend, CoordinatorToUserMessage,
    },
    device::KeyPurpose,
    message::{DeviceToCoordinatorMessage, HeldShare2},
    schnorr_fun::frost::ShareIndex,
    AccessStructureRef, DeviceId, EnterPhysicalId, KeyId, RestorationId,
};
use rand::rngs::OsRng;

use super::{purpose_network, Coordinator, Finished, Res, Session, Update};

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

#[derive(Debug, Clone)]
pub struct RestorationShareView {
    pub device_id: String,
    pub device_name: String,
    pub share_index: u32,
    /// `compatible`, `incompatible` or `uncertain` (not enough shares yet to tell).
    pub compatibility: String,
}

#[derive(Debug, Clone)]
pub struct RestorationView {
    pub restoration_id: String,
    pub name: String,
    pub network: String,
    pub threshold: Option<u16>,
    pub shares: Vec<RestorationShareView>,
    /// Compatible shares gathered so far (`None` while it can't be told yet).
    pub got: Option<u16>,
    pub incompatible: u16,
    /// Enough compatible shares to rebuild the key: `finish_restoration` will work.
    pub restorable: bool,
}

/// A share a signer reported holding.
#[derive(Debug, Clone)]
pub struct HeldShareView {
    pub device_id: String,
    pub share_index: u32,
    pub key_name: Option<String>,
    pub threshold: Option<u16>,
    pub network: Option<String>,
    /// A physical backup typed into the signer, not yet turned into a normal share.
    pub needs_consolidation: bool,
    /// Set when this coordinator already knows the wallet the share belongs to.
    pub known_wallet: Option<String>,
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

/// What an entered backup is for.
#[derive(Debug, Clone)]
pub enum EnterTarget {
    /// Start a brand new restoration (created when the first backup is accepted).
    NewWallet { name: String, network: Network, threshold: Option<u16> },
    Restoration(RestorationId),
    /// Add a share to a wallet this coordinator already has (a replaced or wiped signer).
    Wallet(AccessStructureRef),
}

#[derive(Debug, Clone)]
pub enum EnterStage {
    WaitingForWords,
    Saving(RestorationId),
    Consolidating,
}

#[derive(Debug, Clone)]
pub enum BackupSession {
    Display { device: DeviceId, access_structure_ref: AccessStructureRef, share_index: ShareIndex },
    Check { device: DeviceId, access_structure_ref: AccessStructureRef, share_index: ShareIndex },
    Enter { device: DeviceId, enter_id: EnterPhysicalId, target: EnterTarget, stage: EnterStage },
    Consolidate { device: DeviceId, access_structure_ref: AccessStructureRef },
    HeldShares { device: DeviceId },
}

impl BackupSession {
    pub(super) fn device(&self) -> DeviceId {
        match self {
            Self::Display { device, .. }
            | Self::Check { device, .. }
            | Self::Enter { device, .. }
            | Self::Consolidate { device, .. }
            | Self::HeldShares { device } => *device,
        }
    }
}

/// A reply from a signer that belongs to a backup session.
pub(super) enum Incoming {
    Core(DeviceToCoordinatorMessage),
    Recorded(AccessStructureRef, ShareIndex),
    Checked(AccessStructureRef, ShareIndex),
}

fn index_u32(i: ShareIndex) -> u32 {
    i.try_into().expect("share index fits in a u32")
}

fn compat_name(c: ShareCompatibility) -> &'static str {
    match c {
        ShareCompatibility::Compatible => "compatible",
        ShareCompatibility::Incompatible => "incompatible",
        ShareCompatibility::Uncertain => "uncertain",
    }
}

// ---------------------------------------------------------------------------------------------

impl Coordinator {
    fn parse_device(&self, device_id: &str) -> Res<DeviceId> {
        device_id.parse().map_err(|_| "bad device id".to_string())
    }

    fn parse_restoration(&self, id: &str) -> Res<RestorationId> {
        id.parse().map_err(|_| "bad restoration id".to_string())
    }

    fn access_ref_of(&self, key_id: &str) -> Res<AccessStructureRef> {
        let key = self.inner.iter_keys().find(|k| k.key_id.to_string() == key_id).ok_or("unknown wallet")?;
        Ok(key.master_access_structure().access_structure_ref())
    }

    fn key_name(&self, key_id: KeyId) -> String {
        self.inner.get_frost_key(key_id).map(|k| k.key_name.clone()).unwrap_or_default()
    }

    fn require_idle(&self) -> Res<()> {
        match self.session {
            Session::Idle => Ok(()),
            _ => Err("Another keygen, signing or backup session is in progress. Cancel it first.".into()),
        }
    }

    fn start_backup_session(
        &mut self,
        session: BackupSession,
        sends: Vec<CoordinatorSend>,
        title: &str,
        message: &str,
    ) -> Res<Update> {
        let device = session.device();
        self.session = Session::Backup(session);
        self.replied.clear();
        self.save()?;
        let mut update = Update {
            message: message.to_string(),
            broadcast: Some(self.broadcast(sends, title)),
            ..Default::default()
        };
        self.fill_awaiting(&mut update, &[device].into());
        Ok(update)
    }

    // ------------------------------------------------------------------ backup run ----------

    /// Every share of the wallet with its backup status.
    pub fn backup_shares(&self, key_id: &str) -> Res<Vec<BackupShare>> {
        let aref = self.access_ref_of(key_id)?;
        let acc = self.inner.get_access_structure(aref).ok_or("unknown wallet")?;
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
        let Some(acc) = self.inner.get_access_structure(aref) else { return };
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
        self.start_backup_session(
            BackupSession::Display { device, access_structure_ref: aref, share_index },
            sends,
            "Show the secret backup",
            "Show this to the signer. It will display 25 words: write them down in order, then confirm.",
        )
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
        self.start_backup_session(
            BackupSession::Check { device, access_structure_ref: aref, share_index },
            sends,
            "Check the backup",
            "Show this to the signer. The user answers its quiz using the paper backup.",
        )
    }

    fn share_index_of(&self, aref: AccessStructureRef, device: DeviceId) -> Res<ShareIndex> {
        self.inner
            .get_access_structure(aref)
            .and_then(|a| a.device_to_share_indicies().get(&device).copied())
            .ok_or_else(|| format!("{} holds no share of this wallet.", self.name_of(&device)))
    }

    // ------------------------------------------------------------------ restoration ---------

    pub fn restorations(&self) -> Vec<RestorationView> {
        self.inner
            .restoring()
            .map(|r| {
                let status = r.status();
                let count = status.share_count();
                let mut shares: Vec<_> = status
                    .shares
                    .iter()
                    .map(|s| RestorationShareView {
                        device_id: s.device_id.to_string(),
                        device_name: self.name_of(&s.device_id),
                        share_index: s.index as u32,
                        compatibility: compat_name(s.compatibility).to_string(),
                    })
                    .collect();
                shares.sort_by_key(|s| s.share_index);
                RestorationView {
                    restoration_id: r.restoration_id.to_string(),
                    name: r.key_name.clone(),
                    network: purpose_network(&r.key_purpose).to_string(),
                    threshold: status.threshold,
                    shares,
                    got: count.got,
                    incompatible: count.incompatible,
                    restorable: r.is_restorable(),
                }
            })
            .collect()
    }

    pub fn cancel_restoration(&mut self, restoration_id: &str) -> Res<()> {
        let id = self.parse_restoration(restoration_id)?;
        self.inner.cancel_restoration(id);
        self.save()
    }

    pub fn delete_restoration_share(&mut self, restoration_id: &str, device_id: &str) -> Res<()> {
        let id = self.parse_restoration(restoration_id)?;
        let device = self.parse_device(device_id)?;
        self.inner.delete_restoration_share(id, device);
        self.save()
    }

    /// Turn a restoration that has enough shares into a wallet. Signers restored from a physical
    /// backup then show up in [`Coordinator::pending_consolidations`].
    pub fn finish_restoration(&mut self, restoration_id: &str) -> Res<String> {
        let id = self.parse_restoration(restoration_id)?;
        let aref = self
            .inner
            .finish_restoring(id, self.encryption_key, &mut OsRng)
            .map_err(|e| e.to_string())?;
        self.save()?;
        Ok(aref.key_id.to_string())
    }

    /// Ask a signer to type in a physical backup: it shows its entry screen, the user enters the
    /// key number and 25 words, and the coordinator checks the share fits before the signer saves it.
    pub fn start_enter_backup(&mut self, device_id: &str, target: EnterTarget) -> Res<Update> {
        self.require_idle()?;
        let device = self.parse_device(device_id)?;
        match &target {
            EnterTarget::NewWallet { name, .. } if name.trim().is_empty() => {
                return Err("Give the wallet a name.".into())
            }
            EnterTarget::NewWallet { threshold: Some(t), .. } if *t == 0 => {
                return Err("The threshold must be at least 1.".into())
            }
            EnterTarget::Restoration(id) if !self.inner.restoring().any(|r| r.restoration_id == *id) => {
                return Err("That restoration no longer exists.".into())
            }
            EnterTarget::Wallet(aref) if self.inner.get_access_structure(*aref).is_none() => {
                return Err("That wallet is not known to this coordinator.".into())
            }
            _ => {}
        }
        let enter_id = EnterPhysicalId::new(&mut OsRng);
        let sends: Vec<_> = self.inner.tell_device_to_load_physical_backup(enter_id, device).into_iter().collect();
        self.start_backup_session(
            BackupSession::Enter { device, enter_id, target, stage: EnterStage::WaitingForWords },
            sends,
            "Enter a backup",
            "Show this to the signer, then type the key number and 25 words into it.",
        )
    }

    pub fn start_enter_backup_new_wallet(
        &mut self,
        device_id: &str,
        name: String,
        network: Network,
        threshold: Option<u16>,
    ) -> Res<Update> {
        self.start_enter_backup(device_id, EnterTarget::NewWallet { name: name.trim().to_string(), network, threshold })
    }

    pub fn start_enter_backup_restoration(&mut self, device_id: &str, restoration_id: &str) -> Res<Update> {
        let id = self.parse_restoration(restoration_id)?;
        self.start_enter_backup(device_id, EnterTarget::Restoration(id))
    }

    pub fn start_enter_backup_wallet(&mut self, device_id: &str, key_id: &str) -> Res<Update> {
        let aref = self.access_ref_of(key_id)?;
        self.start_enter_backup(device_id, EnterTarget::Wallet(aref))
    }

    // ------------------------------------------------------------------ shares on signers ----

    /// Ask a signer which shares it holds. Lets a wallet be rebuilt from signers that kept their
    /// shares when this coordinator was lost.
    pub fn start_request_held_shares(&mut self, device_id: &str) -> Res<Update> {
        self.require_idle()?;
        let device = self.parse_device(device_id)?;
        let sends: Vec<_> = self.inner.request_held_shares(device).collect();
        self.start_backup_session(
            BackupSession::HeldShares { device },
            sends,
            "Which shares does this signer hold?",
            "Show this to the signer; it answers by itself.",
        )
    }

    fn held_share(&self, device: DeviceId, share_index: u32) -> Res<RecoverShare> {
        let held = self
            .held
            .get(&device)
            .and_then(|shares| shares.iter().find(|s| index_u32(s.share_image.index) == share_index))
            .ok_or("That signer has not reported this share. Ask it again.")?;
        Ok(RecoverShare { held_by: device, held_share: held.clone() })
    }

    /// Start a restoration from a share a signer reported (needs the signer's own wallet name
    /// and network, which only a consolidated share or a saved backup with metadata carries).
    pub fn start_restoration_from_held_share(&mut self, device_id: &str, share_index: u32) -> Res<String> {
        let device = self.parse_device(device_id)?;
        let share = self.held_share(device, share_index)?;
        if let Some(aref) = share.held_share.access_structure_ref {
            if self.inner.get_access_structure(aref).is_some() {
                return Err("This coordinator already has that wallet.".into());
            }
        }
        let id = RestorationId::new(&mut OsRng);
        self.inner
            .start_restoring_key_from_recover_share(&share, id)
            .map_err(|e| format!("This share cannot start a restoration: {e:?}"))?;
        self.save()?;
        Ok(id.to_string())
    }

    pub fn add_held_share_to_restoration(
        &mut self,
        restoration_id: &str,
        device_id: &str,
        share_index: u32,
    ) -> Res<()> {
        let id = self.parse_restoration(restoration_id)?;
        let device = self.parse_device(device_id)?;
        let share = self.held_share(device, share_index)?;
        self.inner
            .add_recovery_share_to_restoration(id, &share, self.encryption_key)
            .map_err(|e| e.to_string())?;
        self.save()
    }

    /// Give a wallet this coordinator already has a signer that holds one of its shares.
    pub fn add_held_share_to_wallet(&mut self, key_id: &str, device_id: &str, share_index: u32) -> Res<()> {
        let aref = self.access_ref_of(key_id)?;
        let device = self.parse_device(device_id)?;
        let share = self.held_share(device, share_index)?;
        self.inner.recover_share(aref, &share, self.encryption_key).map_err(|e| e.to_string())?;
        self.save()
    }

    // ------------------------------------------------------------------ consolidation -------

    /// Signers restored from a typed-in backup whose share still has to be consolidated.
    pub fn pending_consolidations(&self) -> Vec<PendingConsolidationView> {
        let mut devices: std::collections::BTreeSet<DeviceId> = self.signers.keys().copied().collect();
        for k in self.inner.iter_keys() {
            devices.extend(k.master_access_structure().devices());
        }
        let mut out = vec![];
        for device in devices {
            for aref in self.inner.pending_physical_consolidations(device) {
                let Some(share_index) = self.share_index_of(aref, device).ok() else { continue };
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

    pub fn start_consolidate(&mut self, key_id: &str, device_id: &str) -> Res<Update> {
        self.require_idle()?;
        let device = self.parse_device(device_id)?;
        let aref = self.access_ref_of(key_id)?;
        let send = self
            .inner
            .consolidate_pending_physical_backups(device, aref, self.encryption_key)
            .map_err(|e| format!("{e:?}"))?
            .ok_or("Nothing is waiting to be consolidated on that signer.")?;
        self.start_backup_session(
            BackupSession::Consolidate { device, access_structure_ref: aref },
            vec![send],
            "Finish restoring this signer",
            "Show this to the signer; it turns the typed-in backup into its normal encrypted share by itself.",
        )
    }

    // ------------------------------------------------------------------ replies -------------

    fn rejected(&mut self, reason: impl Into<String>) -> Update {
        self.session = Session::Idle;
        Update {
            message: "The backup was not accepted.".into(),
            finished: Some(Finished::BackupRejected { reason: reason.into() }),
            ..Default::default()
        }
    }

    pub(super) fn receive_backup(&mut self, from: DeviceId, incoming: Incoming) -> Res<Update> {
        let Session::Backup(session) = &self.session else { unreachable!("checked by the caller") };
        let session = session.clone();
        let name = self.name_of(&from);
        let update = match (session, incoming) {
            (
                BackupSession::Display { access_structure_ref, share_index, .. },
                Incoming::Recorded(aref, index),
            ) => {
                if aref != access_structure_ref || index != share_index {
                    return Err("That reply is for a different backup.".into());
                }
                self.backups.insert((aref, index_u32(index)), true);
                self.session = Session::Idle;
                Update {
                    message: format!("{name} confirmed the backup was written down."),
                    finished: Some(Finished::BackupRecorded {
                        key_name: self.key_name(aref.key_id),
                        share_index: index_u32(index),
                    }),
                    ..Default::default()
                }
            }
            (BackupSession::Check { access_structure_ref, share_index, .. }, Incoming::Checked(aref, index)) => {
                if aref != access_structure_ref || index != share_index {
                    return Err("That reply is for a different backup.".into());
                }
                self.session = Session::Idle;
                Update {
                    message: format!("{name} passed the backup check."),
                    finished: Some(Finished::BackupChecked {
                        key_name: self.key_name(aref.key_id),
                        share_index: index_u32(index),
                    }),
                    ..Default::default()
                }
            }
            (BackupSession::Enter { enter_id, target, stage, device }, Incoming::Core(message)) => {
                let sends = self.inner.recv_device_message(from, message).map_err(|e| e.to_string())?;
                self.receive_enter(device, enter_id, target, stage, sends)?
            }
            (BackupSession::Consolidate { access_structure_ref, .. }, Incoming::Core(message)) => {
                let sends = self.inner.recv_device_message(from, message).map_err(|e| e.to_string())?;
                let done = sends.into_iter().find_map(|s| match s {
                    CoordinatorSend::ToUser(CoordinatorToUserMessage::Restoration(
                        ToUserRestoration::FinishedConsolidation { share_index, .. },
                    )) => Some(share_index),
                    _ => None,
                });
                let Some(share_index) = done else {
                    return Err("That reply is not a finished consolidation.".into());
                };
                self.session = Session::Idle;
                Update {
                    message: format!("{name} turned its backup into a normal share."),
                    finished: Some(Finished::Consolidated {
                        key_name: self.key_name(access_structure_ref.key_id),
                        share_index: index_u32(share_index),
                    }),
                    ..Default::default()
                }
            }
            (BackupSession::HeldShares { .. }, Incoming::Core(message)) => {
                let sends = self.inner.recv_device_message(from, message).map_err(|e| e.to_string())?;
                let shares = sends.into_iter().find_map(|s| match s {
                    CoordinatorSend::ToUser(CoordinatorToUserMessage::Restoration(
                        ToUserRestoration::GotHeldShares { held_by, shares },
                    )) if held_by == from => Some(shares),
                    _ => None,
                });
                let Some(shares) = shares else {
                    return Err("That reply does not list shares.".into());
                };
                let views = self.held_share_views(from, &shares);
                self.held.insert(from, shares);
                self.session = Session::Idle;
                Update {
                    message: format!("{name} reported {} share(s).", views.len()),
                    finished: Some(Finished::HeldShares { device_name: name, shares: views }),
                    ..Default::default()
                }
            }
            _ => return Err("That reply does not belong to what this session is waiting for.".into()),
        };
        Ok(update)
    }

    fn held_share_views(&self, device: DeviceId, shares: &[HeldShare2]) -> Vec<HeldShareView> {
        shares
            .iter()
            .map(|s| HeldShareView {
                device_id: device.to_string(),
                share_index: index_u32(s.share_image.index),
                key_name: s.key_name.clone(),
                threshold: s.threshold,
                network: s.purpose.as_ref().map(|p| purpose_network(p).to_string()),
                needs_consolidation: s.needs_consolidation,
                known_wallet: s
                    .access_structure_ref
                    .filter(|a| self.inner.get_access_structure(*a).is_some())
                    .map(|a| self.key_name(a.key_id)),
            })
            .collect()
    }

    /// The signer answered an "enter a backup" request (`PhysicalEntered`) or a save / consolidate
    /// request that followed it.
    fn receive_enter(
        &mut self,
        device: DeviceId,
        enter_id: EnterPhysicalId,
        target: EnterTarget,
        stage: EnterStage,
        sends: Vec<CoordinatorSend>,
    ) -> Res<Update> {
        let to_user = sends.into_iter().find_map(|s| match s {
            CoordinatorSend::ToUser(CoordinatorToUserMessage::Restoration(r)) => Some(r),
            _ => None,
        });
        match (stage, to_user) {
            (EnterStage::WaitingForWords, Some(ToUserRestoration::PhysicalBackupEntered(phase))) => {
                if phase.backup.enter_physical_id != enter_id || phase.from != device {
                    return Err("That is not the backup this session asked for.".into());
                }
                self.handle_entered_backup(device, enter_id, target, *phase)
            }
            (EnterStage::Saving(restoration_id), Some(ToUserRestoration::PhysicalBackupSaved { share_index, .. })) => {
                let restorable = self
                    .inner
                    .get_restoration_state(restoration_id)
                    .map(|s| s.is_restorable())
                    .unwrap_or(false);
                self.session = Session::Idle;
                Ok(Update {
                    message: "The backup was saved on the signer.".into(),
                    finished: Some(Finished::BackupSaved {
                        restoration_id: restoration_id.to_string(),
                        share_index: index_u32(share_index),
                        restorable,
                    }),
                    ..Default::default()
                })
            }
            (
                EnterStage::Consolidating,
                Some(ToUserRestoration::FinishedConsolidation { access_structure_ref, share_index, .. }),
            ) => {
                self.session = Session::Idle;
                Ok(Update {
                    message: "The signer now holds its share of the wallet.".into(),
                    finished: Some(Finished::Consolidated {
                        key_name: self.key_name(access_structure_ref.key_id),
                        share_index: index_u32(share_index),
                    }),
                    ..Default::default()
                })
            }
            _ => Err("That reply does not belong to what this session is waiting for.".into()),
        }
    }

    /// The signer typed in a backup: check it fits where it is meant to go, then tell the signer
    /// to keep it.
    fn handle_entered_backup(
        &mut self,
        device: DeviceId,
        enter_id: EnterPhysicalId,
        target: EnterTarget,
        phase: PhysicalBackupPhase,
    ) -> Res<Update> {
        let key = self.encryption_key;
        match target.clone() {
            EnterTarget::Wallet(aref) => {
                let tell = match self.inner.tell_device_to_consolidate_physical_backup(phase, aref, key) {
                    Ok(t) => t,
                    Err(e) => return Ok(self.rejected(e.to_string())),
                };
                let sends: Vec<_> = tell.into_iter().collect();
                self.session = Session::Backup(BackupSession::Enter {
                    device,
                    enter_id,
                    target,
                    stage: EnterStage::Consolidating,
                });
                self.replied.clear();
                Ok(self.next_step(
                    device,
                    sends,
                    "Add the backup to the wallet",
                    "The backup fits this wallet. Show this to the signer to finish adding it.",
                ))
            }
            EnterTarget::NewWallet { .. } | EnterTarget::Restoration(_) => {
                let (restoration_id, created) = match &target {
                    EnterTarget::NewWallet { name, network, threshold } => {
                        let id = RestorationId::new(&mut OsRng);
                        self.inner.start_restoring_key(
                            name.clone(),
                            *threshold,
                            KeyPurpose::Bitcoin(*network),
                            id,
                        );
                        (id, true)
                    }
                    EnterTarget::Restoration(id) => (*id, false),
                    EnterTarget::Wallet(_) => unreachable!(),
                };
                if let Err(e) = self.inner.check_physical_backup_compatible_with_restoration(restoration_id, phase, key) {
                    if created {
                        // don't leave an empty restoration behind
                        self.inner.cancel_restoration(restoration_id);
                    }
                    return Ok(self.rejected(e.to_string()));
                }
                let sends: Vec<_> = self.inner.tell_device_to_save_physical_backup(phase, restoration_id).into_iter().collect();
                self.session = Session::Backup(BackupSession::Enter {
                    device,
                    enter_id,
                    target,
                    stage: EnterStage::Saving(restoration_id),
                });
                self.replied.clear();
                Ok(self.next_step(
                    device,
                    sends,
                    "Save the backup on the signer",
                    "The backup is accepted. Show this to the signer so it keeps it.",
                ))
            }
        }
    }

    fn next_step(&mut self, device: DeviceId, sends: Vec<CoordinatorSend>, title: &str, message: &str) -> Update {
        let mut update = Update {
            message: message.to_string(),
            broadcast: Some(self.broadcast(sends, title)),
            ..Default::default()
        };
        self.fill_awaiting(&mut update, &[device].into());
        update
    }
}
