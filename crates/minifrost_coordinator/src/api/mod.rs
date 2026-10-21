//! The API Flutter sees. Everything here is generated into Dart by flutter_rust_bridge.

pub mod camera;

use std::sync::Mutex;

pub use crate::engine::{Broadcast, Finished, KeyView, SignerInfo, Update};
use crate::engine::{default_esplora, Coordinator};
use crate::wallet::{self, Amt};

fn parse_network(n: &str) -> Result<bitcoin::Network, String> {
    match n {
        "bitcoin" | "mainnet" => Ok(bitcoin::Network::Bitcoin),
        "testnet" => Ok(bitcoin::Network::Testnet),
        "signet" => Ok(bitcoin::Network::Signet),
        other => Err(format!("unsupported network {other}")),
    }
}

#[derive(Debug, Clone)]
pub struct WalletSummary {
    pub balance_sats: i64,
    pub confirmed_sats: i64,
    pub n_coins: u32,
    /// First unused receive address.
    pub receive_address: String,
}

#[derive(Debug, Clone)]
pub struct SendPreview {
    pub recipient: String,
    pub amount_sats: i64,
    pub fee_sats: i64,
    pub fee_rate: f64,
    pub change_sats: Option<i64>,
    pub n_inputs: u32,
}

pub struct CoordinatorApp {
    inner: Mutex<Coordinator>,
}

/// Open the coordinator stored in `dir` (created on first use).
pub fn open_coordinator(dir: String) -> Result<CoordinatorApp, String> {
    Ok(CoordinatorApp { inner: Mutex::new(Coordinator::open(std::path::Path::new(&dir))?) })
}

impl CoordinatorApp {
    pub fn signers(&self) -> Vec<SignerInfo> {
        self.inner.lock().unwrap().signers()
    }
    pub fn keys(&self) -> Vec<KeyView> {
        self.inner.lock().unwrap().keys()
    }
    /// Scan of a signer's "hello" QR.
    pub fn enroll(&self, payload: Vec<u8>) -> Result<SignerInfo, String> {
        self.inner.lock().unwrap().enroll(&payload)
    }
    pub fn forget_signer(&self, device_id: String) -> Result<(), String> {
        self.inner.lock().unwrap().forget_signer(&device_id)
    }
    pub fn esplora_url(&self, network: String) -> Result<String, String> {
        let net = parse_network(&network)?;
        Ok(self.inner.lock().unwrap().esplora_url().unwrap_or_else(|| default_esplora(net).to_string()))
    }
    pub fn set_esplora_url(&self, url: Option<String>) -> Result<(), String> {
        self.inner.lock().unwrap().set_esplora_url(url)
    }

    // ---- sessions
    pub fn start_keygen(
        &self,
        key_name: String,
        threshold: u16,
        device_ids: Vec<String>,
        network: String,
    ) -> Result<Update, String> {
        self.inner.lock().unwrap().start_keygen(key_name, threshold, device_ids, parse_network(&network)?)
    }
    pub fn cancel_session(&self) -> Result<(), String> {
        self.inner.lock().unwrap().cancel()
    }
    /// A signer's reply, fully scanned.
    pub fn receive(&self, payload: Vec<u8>) -> Result<Update, String> {
        self.inner.lock().unwrap().receive(&payload)
    }

    // ---- wallet (talks to Esplora; never holds the lock across a request)
    pub fn refresh_wallet(&self, key_id: String) -> Result<WalletSummary, String> {
        let (master, network, esplora) = {
            let c = self.inner.lock().unwrap();
            let (m, n) = c.wallet_params(&key_id)?;
            (m, n, c.esplora(n))
        };
        let view = wallet::sync(&esplora, master, network)?;
        let summary = WalletSummary {
            balance_sats: view.balance() as i64,
            confirmed_sats: view.confirmed_balance() as i64,
            n_coins: view.utxos.len() as u32,
            receive_address: wallet::address(master, network, false, view.next_external).to_string(),
        };
        self.inner.lock().unwrap().store_wallet(&key_id, view)?;
        Ok(summary)
    }

    /// Fee rate (sat/vB) for roughly the next `blocks` blocks.
    pub fn suggested_fee_rate(&self, key_id: String, blocks: u32) -> Result<f64, String> {
        use crate::wallet::Esplora;
        let esplora = {
            let c = self.inner.lock().unwrap();
            let (_, n) = c.wallet_params(&key_id)?;
            c.esplora(n)
        };
        esplora.fee_rate(blocks)
    }

    /// `amount_sats = None` sends everything.
    pub fn prepare_send(
        &self,
        key_id: String,
        to_address: String,
        amount_sats: Option<i64>,
        fee_rate: f64,
    ) -> Result<SendPreview, String> {
        let amount = match amount_sats {
            Some(a) if a > 0 => Amt::Sats(a as u64),
            Some(_) => return Err("Amount must be positive.".into()),
            None => Amt::Max,
        };
        let b = self.inner.lock().unwrap().prepare_send(&key_id, &to_address, amount, fee_rate)?;
        Ok(SendPreview {
            recipient: b.recipient.to_string(),
            amount_sats: b.amount as i64,
            fee_sats: b.fee as i64,
            fee_rate: b.fee_rate,
            change_sats: b.change.map(|c| c as i64),
            n_inputs: b.template.inputs().len() as u32,
        })
    }

    /// Start the signing ceremony for the payment from `prepare_send`.
    pub fn begin_signing(&self, key_id: String, device_ids: Vec<String>) -> Result<Update, String> {
        self.inner.lock().unwrap().begin_signing(&key_id, device_ids)
    }

    /// Publish a signed transaction through the Esplora backend. Returns the txid.
    pub fn broadcast(&self, key_id: String, tx_hex: String) -> Result<String, String> {
        use crate::wallet::Esplora;
        let esplora = {
            let c = self.inner.lock().unwrap();
            let (_, n) = c.wallet_params(&key_id)?;
            c.esplora(n)
        };
        esplora.broadcast(&tx_hex)
    }
}

// ------------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ScanProgress {
    pub received: u32,
    pub total: u32,
    pub payload: Option<Vec<u8>>,
    pub problem: Option<String>,
}

/// Collects the frames of an animated QR code.
pub struct QrScanner {
    inner: Mutex<qr_transport::Assembler>,
}

pub fn new_qr_scanner() -> QrScanner {
    QrScanner { inner: Mutex::new(qr_transport::Assembler::new()) }
}

impl QrScanner {
    pub fn push(&self, text: String) -> ScanProgress {
        match self.inner.lock().unwrap().push(&text) {
            Ok(p) => ScanProgress {
                received: p.received as u32,
                total: p.total as u32,
                payload: p.complete,
                problem: None,
            },
            Err(e) => ScanProgress { received: 0, total: 0, payload: None, problem: Some(e.to_string()) },
        }
    }
    pub fn reset(&self) {
        self.inner.lock().unwrap().reset()
    }
}

/// Decode QR codes from one camera frame's luma (Y) plane. `row_stride` is the plane's bytes per
/// row, which can be wider than `width`.
pub fn decode_qr_luma(width: u32, height: u32, row_stride: u32, luma: Vec<u8>) -> Vec<String> {
    let (w, h, stride) = (width as usize, height as usize, row_stride as usize);
    if w == 0 || h == 0 || stride < w || luma.len() < stride * (h - 1) + w {
        return vec![];
    }
    let mut img = rqrr::PreparedImage::prepare_from_greyscale(w, h, |x, y| luma[y * stride + x]);
    img.detect_grids()
        .into_iter()
        .filter_map(|g| g.decode().ok().map(|(_, text)| text))
        .collect()
}
