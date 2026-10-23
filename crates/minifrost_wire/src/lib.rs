//! Everything that crosses the air gap. Both apps depend on this crate and nothing else for the
//! wire format, so the signer never needs to link the coordinator (or any networking) code.
//!
//! The packets are thin wrappers around frostsnap_core's own device<->coordinator messages.

use frostsnap_core::{
    message::{signing::OpenNonceStreams, CoordinatorToDeviceMessage, DeviceToCoordinatorMessage},
    schnorr_fun::frost::ShareIndex,
    AccessStructureRef, DeviceId, KeyId, WireSignTask,
};

/// Bumped on any incompatible change to [`Packet`].
pub const WIRE_VERSION: u8 = 2;

#[derive(Debug, Clone, bincode::Encode, bincode::Decode)]
pub enum Packet {
    /// signer -> coordinator: "this is me". Shown as a QR when enrolling a signer.
    Hello { device_id: DeviceId, name: String },
    /// coordinator -> all signers. Each signer acts on the entries addressed to its device id.
    /// Used for every keygen round and for the signing round that carries the aggregated nonces.
    ToDevices(Vec<(Vec<DeviceId>, CoordinatorToDeviceMessage)>),
    /// coordinator -> all signers, signing round 1: the transaction to review, plus a request for
    /// fresh nonces. The signer only releases nonces after its user looked at the transaction.
    SignPrepare(SignPrepare),
    /// signer -> coordinator: reply to the last coordinator packet.
    FromDevice {
        device_id: DeviceId,
        message: DeviceToCoordinatorMessage,
    },
    /// signer -> coordinator: the user confirmed they wrote the displayed backup down. In
    /// frostsnap this is `CommsMisc::BackupRecorded`, which is not one of `frostsnap_core`'s
    /// message types, so it gets its own packet. The share is named so a stale or replayed code
    /// can't mark the wrong share as backed up.
    BackupRecorded {
        device_id: DeviceId,
        access_structure_ref: AccessStructureRef,
        share_index: ShareIndex,
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

/// The keygen verification code. Every signer and the coordinator show this; the humans compare
/// them. Six bytes of the session hash, as groups of four hex digits.
pub fn format_check_code(hash: &[u8; 32]) -> String {
    hash[..6]
        .chunks(2)
        .map(|c| format!("{:02X}{:02X}", c[0], c[1]))
        .collect::<Vec<_>>()
        .join(" ")
}
