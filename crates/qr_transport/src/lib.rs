//! Air-gap transport: any byte payload becomes a cycle of short text frames that are shown as an
//! animated QR code and re-assembled by the scanning side, in any order and with repeats.
//!
//! Frame text: `MF1:<id>:<index>:<total>:<base32 chunk>` where `index` is 1-based and `id` is the
//! first 4 bytes of the SHA-256 of the *whole* payload (hex). Everything is upper-case and uses
//! only characters from the QR alphanumeric set so the codes stay small and easy to scan. The id
//! doubles as an integrity check: a reassembled payload that doesn't hash to its id is rejected.

use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

const PREFIX: &str = "MF1";
const B32: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

/// Default characters of payload per frame. Keeps each QR around version 8-10 at ECC level L.
pub const DEFAULT_FRAME_CHARS: usize = 220;

fn b32_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity((data.len() * 8).div_ceil(5));
    let (mut acc, mut bits) = (0u32, 0u32);
    for &b in data {
        acc = (acc << 8) | b as u32;
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(B32[((acc >> bits) & 31) as usize] as char);
        }
    }
    if bits > 0 {
        out.push(B32[((acc << (5 - bits)) & 31) as usize] as char);
    }
    out
}

fn b32_decode(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len() * 5 / 8);
    let (mut acc, mut bits) = (0u32, 0u32);
    for c in s.bytes() {
        let v = B32.iter().position(|&x| x == c)? as u32;
        acc = (acc << 5) | v;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    // leftover bits must be zero padding, otherwise the encoding was not canonical
    if bits > 0 && (acc & ((1 << bits) - 1)) != 0 {
        return None;
    }
    Some(out)
}

fn payload_id(payload: &[u8]) -> u32 {
    let h = Sha256::digest(payload);
    u32::from_be_bytes([h[0], h[1], h[2], h[3]])
}

/// Split `payload` into frames of at most `frame_chars` base32 characters each.
/// An empty payload still yields one frame so the receiver can tell "empty" from "nothing".
pub fn split(payload: &[u8], frame_chars: usize) -> Vec<String> {
    let encoded = b32_encode(payload);
    // base32 chunks must stay a multiple of 8 chars so every chunk decodes independently
    let step = (frame_chars.max(8) / 8) * 8;
    let chunks: Vec<&str> = if encoded.is_empty() {
        vec![""]
    } else {
        encoded
            .as_bytes()
            .chunks(step)
            .map(|c| std::str::from_utf8(c).expect("ascii"))
            .collect()
    };
    let id = payload_id(payload);
    let total = chunks.len();
    chunks
        .into_iter()
        .enumerate()
        .map(|(i, c)| format!("{PREFIX}:{id:08X}:{}:{total}:{c}", i + 1))
        .collect()
}

/// One parsed frame. The compact binary form ([`Frame::to_bytes`]) carries the same data without
/// base32's 60% overhead, for transports where size matters (sound).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub id: u32,
    /// 1-based.
    pub index: u16,
    pub total: u16,
    pub data: Vec<u8>,
}

impl Frame {
    pub fn parse(text: &str) -> Option<Frame> {
        let mut it = text.trim().splitn(5, ':');
        if it.next() != Some(PREFIX) {
            return None;
        }
        let id = u32::from_str_radix(it.next()?, 16).ok()?;
        let index: u16 = it.next()?.parse().ok()?;
        let total: u16 = it.next()?.parse().ok()?;
        let data = b32_decode(it.next()?)?;
        (index != 0 && index <= total).then_some(Frame { id, index, total, data })
    }

    /// The `MF1:` text form, which an [`Assembler`] accepts.
    pub fn to_text(&self) -> String {
        format!("{PREFIX}:{:08X}:{}:{}:{}", self.id, self.index, self.total, b32_encode(&self.data))
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(8 + self.data.len());
        out.extend_from_slice(&self.id.to_be_bytes());
        out.extend_from_slice(&self.index.to_be_bytes());
        out.extend_from_slice(&self.total.to_be_bytes());
        out.extend_from_slice(&self.data);
        out
    }

    pub fn from_bytes(b: &[u8]) -> Option<Frame> {
        if b.len() < 8 {
            return None;
        }
        let frame = Frame {
            id: u32::from_be_bytes(b[0..4].try_into().ok()?),
            index: u16::from_be_bytes(b[4..6].try_into().ok()?),
            total: u16::from_be_bytes(b[6..8].try_into().ok()?),
            data: b[8..].to_vec(),
        };
        (frame.index != 0 && frame.index <= frame.total).then_some(frame)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PushError {
    NotOurs,
    Malformed,
    /// All frames arrived but the payload doesn't match its id (corruption or mixed-up codes).
    BadChecksum,
}

impl std::fmt::Display for PushError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            PushError::NotOurs => "not a minifrost QR code",
            PushError::Malformed => "malformed QR frame",
            PushError::BadChecksum => "QR payload failed its checksum",
        })
    }
}
impl std::error::Error for PushError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Progress {
    pub received: usize,
    pub total: usize,
    pub complete: Option<Vec<u8>>,
}

/// Collects frames of one message at a time. Seeing a frame with a different id restarts it, so
/// pointing the camera at a new code just works.
#[derive(Default)]
pub struct Assembler {
    current: Option<(u32, usize, BTreeMap<usize, Vec<u8>>)>,
}

impl Assembler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.current = None;
    }

    pub fn push(&mut self, frame: &str) -> Result<Progress, PushError> {
        let mut it = frame.trim().splitn(5, ':');
        if it.next() != Some(PREFIX) {
            return Err(PushError::NotOurs);
        }
        let id = u32::from_str_radix(it.next().ok_or(PushError::Malformed)?, 16)
            .map_err(|_| PushError::Malformed)?;
        let index: usize = it.next().ok_or(PushError::Malformed)?.parse().map_err(|_| PushError::Malformed)?;
        let total: usize = it.next().ok_or(PushError::Malformed)?.parse().map_err(|_| PushError::Malformed)?;
        let data = b32_decode(it.next().ok_or(PushError::Malformed)?).ok_or(PushError::Malformed)?;
        if index == 0 || index > total || total > 4096 {
            return Err(PushError::Malformed);
        }

        match &mut self.current {
            Some((cid, ctotal, _)) if *cid == id && *ctotal == total => {}
            _ => self.current = Some((id, total, BTreeMap::new())),
        }
        let (_, _, parts) = self.current.as_mut().expect("just set");
        parts.insert(index, data);

        if parts.len() == total {
            let payload: Vec<u8> = parts.values().flatten().copied().collect();
            if payload_id(&payload) != id {
                self.current = None;
                return Err(PushError::BadChecksum);
            }
            self.current = None;
            return Ok(Progress { received: total, total, complete: Some(payload) });
        }
        Ok(Progress { received: parts.len(), total, complete: None })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_any_order_with_repeats() {
        for len in [0usize, 1, 5, 6, 7, 100, 1000, 5000] {
            let payload: Vec<u8> = (0..len).map(|i| (i * 31 + 7) as u8).collect();
            let mut frames = split(&payload, 64);
            assert!(frames.iter().all(|f| f.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == ':')));
            frames.reverse();
            let doubled: Vec<_> = frames.iter().chain(frames.iter()).cloned().collect();
            let mut asm = Assembler::new();
            let mut out = None;
            for f in &doubled {
                if let Some(p) = asm.push(f).unwrap().complete {
                    out = Some(p);
                    break;
                }
            }
            assert_eq!(out.unwrap(), payload, "len {len}");
        }
    }

    #[test]
    fn binary_frames_roundtrip_through_text() {
        let payload: Vec<u8> = (0..777).map(|i| (i * 13) as u8).collect();
        let mut asm = Assembler::new();
        let mut out = None;
        // 160 chars = 100 bytes, a multiple of 5 so chunks stay independently decodable
        for text in split(&payload, 160) {
            let bytes = Frame::parse(&text).unwrap().to_bytes();
            assert_eq!(bytes.len() <= 108, true);
            let again = Frame::from_bytes(&bytes).unwrap().to_text();
            assert_eq!(again, text);
            out = asm.push(&again).unwrap().complete.or(out);
        }
        assert_eq!(out.unwrap(), payload);
        assert!(Frame::from_bytes(&[0; 7]).is_none());
        assert!(Frame::parse("hello").is_none());
    }

    #[test]
    fn new_message_restarts_and_corruption_is_caught() {
        let a = split(&[1u8; 300], 64);
        let b = split(&[2u8; 300], 64);
        let mut asm = Assembler::new();
        asm.push(&a[0]).unwrap();
        // switching to another message discards the partial one
        for f in &b[..b.len() - 1] {
            assert!(asm.push(f).unwrap().complete.is_none());
        }
        assert_eq!(asm.push(b.last().unwrap()).unwrap().complete.unwrap(), vec![2u8; 300]);

        // flip a frame from message a to hold message b's data under a's id
        let mut mixed = a.clone();
        let tail = &b[1];
        let mut parts: Vec<&str> = tail.splitn(5, ':').collect();
        let a1: Vec<&str> = a[1].splitn(5, ':').collect();
        parts[1] = a1[1];
        parts[2] = a1[2];
        mixed[1] = parts.join(":");
        let mut asm = Assembler::new();
        let mut last = Ok(Progress { received: 0, total: 0, complete: None });
        for f in &mixed {
            last = asm.push(f);
        }
        assert_eq!(last, Err(PushError::BadChecksum));
        assert_eq!(asm.push("hello"), Err(PushError::NotOurs));
    }
}
