//! Everything that crosses the air gap. Both apps depend on this crate and nothing else for the
//! wire format, so the signer never needs to link the coordinator (or any networking) code.
//!
//! The packets are thin wrappers around frostsnap_core's own device<->coordinator messages.

use frostsnap_core::{
    AccessStructureRef, DeviceId, KeyId, WireSignTask,
    message::{CoordinatorToDeviceMessage, DeviceToCoordinatorMessage, signing::OpenNonceStreams},
    schnorr_fun::frost::ShareIndex,
};

/// Bumped on any incompatible change to [`Packet`].
pub const WIRE_VERSION: u8 = 4;

#[derive(Debug, Clone, bincode::Encode, bincode::Decode)]
pub enum Packet {
    /// signer -> coordinator: "this is me". Shown as a QR when enrolling a signer.
    Hello { device_id: DeviceId, name: String },
    /// coordinator -> all signers. Each signer acts on the entries addressed to its device id.
    /// Used for every keygen round and for the signing round that carries the aggregated nonces.
    ToDevices {
        /// Which coordinator step this is. The signer labels its reply with the step's title.
        step: Step,
        entries: Vec<(Vec<DeviceId>, CoordinatorToDeviceMessage)>,
    },
    /// coordinator -> all signers, signing round 1: the transaction to review, plus a request for
    /// fresh nonces. The signer only releases nonces after its user looked at the transaction.
    SignPrepare(SignPrepare),
    /// signer -> coordinator: reply to the last coordinator packet.
    FromDevice {
        device_id: DeviceId,
        message: DeviceToCoordinatorMessage,
    },
    /// signer -> coordinator: the user passed the backup check quiz for this share
    /// (`CommsMisc::BackupChecked` in frostsnap).
    BackupChecked {
        device_id: DeviceId,
        access_structure_ref: AccessStructureRef,
        share_index: ShareIndex,
    },
}

#[derive(Debug, Clone, bincode::Encode, bincode::Decode)]
pub struct SignPrepare {
    /// As in [`Packet::ToDevices`]; always [`Step::SignReview`].
    pub step: Step,
    pub key_id: KeyId,
    /// What will be signed. The same task has to come back, bit for bit, in round 2.
    pub sign_task: WireSignTask,
    pub open_nonce_streams: Vec<(DeviceId, OpenNonceStreams)>,
}

#[derive(Debug)]
pub enum WireError {
    Version(u8),
    Empty,
    Decode(String),
    TrailingBytes,
}

impl std::fmt::Display for WireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WireError::Version(v) => write!(f, "unsupported wire version {v}"),
            WireError::Empty => write!(f, "empty packet"),
            WireError::Decode(e) => write!(f, "could not decode packet: {e}"),
            WireError::TrailingBytes => write!(f, "unexpected trailing bytes"),
        }
    }
}
impl std::error::Error for WireError {}

impl Packet {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = vec![WIRE_VERSION];
        out.extend(bincode::encode_to_vec(self, bincode::config::standard()).expect("encodes"));
        out
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, WireError> {
        let (&version, rest) = bytes.split_first().ok_or(WireError::Empty)?;
        if version != WIRE_VERSION {
            return Err(WireError::Version(version));
        }
        let (packet, used) = bincode::decode_from_slice(rest, bincode::config::standard())
            .map_err(|e| WireError::Decode(e.to_string()))?;
        if used != rest.len() {
            return Err(WireError::TrailingBytes);
        }
        Ok(packet)
    }
}

/// A coordinator step. Only this one-byte code crosses the air gap: each side turns it into the
/// step's title itself, so the coordinator can't put arbitrary text on the signer's screen. Each
/// step belongs to one kind of message, and the signer refuses a code whose step doesn't fit what
/// it carries (see [`Step::check`]), so a reply's label always names the step actually answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, bincode::Encode, bincode::Decode)]
pub enum Step {
    KeygenJoin,
    KeygenCertify,
    KeygenCheck,
    KeygenFinalize,
    SignReview,
    SignNonces,
    ShowBackup,
    CheckBackup,
    EnterBackup,
    SaveBackup,
    AddBackupToWallet,
    FinishRestoring,
    HeldShares,
}

impl Step {
    pub fn title(self) -> &'static str {
        match self {
            Step::KeygenJoin => "Round 1 · Join key generation",
            Step::KeygenCertify => "Round 2 · Certify",
            Step::KeygenCheck => "Round 3 · Compare the check code",
            Step::KeygenFinalize => "Final · Store the key",
            Step::SignReview => "Step 1 · Transaction to review",
            Step::SignNonces => "Step 2 · Aggregated nonces",
            Step::ShowBackup => "Show the secret backup",
            Step::CheckBackup => "Check the backup",
            Step::EnterBackup => "Enter a backup",
            Step::SaveBackup => "Save the backup on the signer",
            Step::AddBackupToWallet => "Add the backup to the wallet",
            Step::FinishRestoring => "Finish restoring this signer",
            Step::HeldShares => "Which shares does this signer hold?",
        }
    }

    /// Whether a coordinator step carrying `message` can be this step. A [`SignPrepare`] is
    /// always [`Step::SignReview`], which no `message` fits.
    fn fits(self, message: &CoordinatorToDeviceMessage) -> bool {
        use CoordinatorToDeviceMessage as M;
        use frostsnap_core::message::{
            CoordinatorRestoration as R, keygen::Keygen, signing::CoordinatorSigning,
        };
        match (self, message) {
            (Step::KeygenJoin, M::KeyGen(Keygen::Begin(_)))
            | (Step::KeygenCertify, M::KeyGen(Keygen::CertifyPlease { .. }))
            | (Step::KeygenCheck, M::KeyGen(Keygen::Check { .. }))
            | (Step::KeygenFinalize, M::KeyGen(Keygen::Finalize { .. }))
            | (Step::SignNonces, M::Signing(CoordinatorSigning::RequestSign(_)))
            | (Step::ShowBackup, M::Restoration(R::DisplayBackup { .. }))
            | (Step::CheckBackup, M::Restoration(R::CheckBackup { .. }))
            | (Step::EnterBackup, M::Restoration(R::EnterPhysicalBackup { .. }))
            | (
                Step::SaveBackup,
                M::Restoration(R::SavePhysicalBackup { .. } | R::SavePhysicalBackup2(_)),
            )
            | (Step::HeldShares, M::Restoration(R::RequestHeldShares)) => true,
            // the same request finishes both flows
            (
                Step::AddBackupToWallet | Step::FinishRestoring,
                M::Restoration(R::Consolidate(_)),
            ) => true,
            _ => false,
        }
    }

    /// Refuse a coordinator code whose step doesn't match the message it carries.
    pub fn check(self, message: &CoordinatorToDeviceMessage) -> Result<(), String> {
        if self.fits(message) {
            Ok(())
        } else {
            Err(format!(
                "This code is labelled “{}” but does not carry that step. Refusing it.",
                self.title()
            ))
        }
    }

    /// As [`Step::check`], for the step of a [`SignPrepare`].
    pub fn check_sign_prepare(self) -> Result<(), String> {
        if self == Step::SignReview {
            Ok(())
        } else {
            Err(format!(
                "This code is labelled “{}” but carries a transaction to review. Refusing it.",
                self.title()
            ))
        }
    }
}

/// What a signer calls its reply to the coordinator step titled `title`, so the user can see it
/// answers the step the coordinator is waiting for.
pub fn reply_label(title: &str) -> String {
    format!("Reply to “{title}”")
}

/// The keygen verification code. Every signer and the coordinator show this; the humans compare
/// them. Six bytes of the session hash, as groups of four hex digits.
pub fn format_check_code(hash: &[u8; 32]) -> String {
    hash[..6]
        .chunks(2)
        .map(|c| format!("{:02X}{:02X}", c[0], c[1]))
        .collect::<Vec<_>>()
        .join(" ")
}
