//! Watch-only taproot wallet on top of an Esplora backend: address derivation, gap-limit scan,
//! transaction building. Everything that touches the network sits behind the [`Esplora`] trait so
//! the logic can be tested without one.

use std::str::FromStr;

use bitcoin::{Address, Amount, Network, OutPoint, ScriptBuf, TxOut, Txid};
use frostsnap_core::{
    bitcoin_transaction::{LocalSpk, PushInput, TransactionTemplate},
    tweak::{BitcoinBip32Path, NormalIndex},
    MasterAppkey,
};

pub const GAP_LIMIT: u32 = 10;
const DUST: u64 = 330;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EsploraUtxo {
    pub txid: Txid,
    pub vout: u32,
    pub value: u64,
    pub confirmed: bool,
}

pub trait Esplora {
    /// Has this address ever been seen in a transaction (confirmed or in the mempool)?
    fn address_used(&self, address: &Address) -> Result<bool, String>;
    fn address_utxos(&self, address: &Address) -> Result<Vec<EsploraUtxo>, String>;
    /// sat/vB for a ~`blocks` block confirmation target
    fn fee_rate(&self, blocks: u32) -> Result<f64, String>;
    /// Returns the txid reported by the server
    fn broadcast(&self, tx_hex: &str) -> Result<String, String>;
}

pub struct HttpEsplora {
    base: String,
    agent: ureq::Agent,
}

impl HttpEsplora {
    pub fn new(base_url: &str) -> Self {
        Self {
            base: base_url.trim_end_matches('/').to_string(),
            agent: ureq::AgentBuilder::new()
                .timeout(std::time::Duration::from_secs(20))
                .build(),
        }
    }

    fn get_json(&self, path: &str) -> Result<serde_json::Value, String> {
        self.agent
            .get(&format!("{}{}", self.base, path))
            .call()
            .map_err(|e| format!("GET {path}: {e}"))?
            .into_json()
            .map_err(|e| format!("GET {path}: bad JSON: {e}"))
    }
}

impl Esplora for HttpEsplora {
    fn address_used(&self, address: &Address) -> Result<bool, String> {
        let v = self.get_json(&format!("/address/{address}"))?;
        let n = |k: &str| v[k]["tx_count"].as_u64().unwrap_or(0);
        Ok(n("chain_stats") + n("mempool_stats") > 0)
    }

    fn address_utxos(&self, address: &Address) -> Result<Vec<EsploraUtxo>, String> {
        let v = self.get_json(&format!("/address/{address}/utxo"))?;
        v.as_array()
            .ok_or("utxo response was not a list")?
            .iter()
            .map(|u| {
                Ok(EsploraUtxo {
                    txid: Txid::from_str(u["txid"].as_str().ok_or("missing txid")?)
                        .map_err(|e| e.to_string())?,
                    vout: u["vout"].as_u64().ok_or("missing vout")? as u32,
                    value: u["value"].as_u64().ok_or("missing value")?,
                    confirmed: u["status"]["confirmed"].as_bool().unwrap_or(false),
                })
            })
            .collect()
    }

    fn fee_rate(&self, blocks: u32) -> Result<f64, String> {
        let v = self.get_json("/fee-estimates")?;
        let obj = v.as_object().ok_or("fee-estimates was not an object")?;
        // closest target at or above the requested one, else the slowest known
        let mut targets: Vec<(u32, f64)> = obj
            .iter()
            .filter_map(|(k, v)| Some((k.parse().ok()?, v.as_f64()?)))
            .collect();
        targets.sort_by_key(|t| t.0);
        targets
            .iter()
            .find(|(t, _)| *t >= blocks)
            .or(targets.last())
            .map(|(_, r)| r.max(1.0))
            .ok_or_else(|| "no fee estimates available".to_string())
    }

    fn broadcast(&self, tx_hex: &str) -> Result<String, String> {
        match self
            .agent
            .post(&format!("{}/tx", self.base))
            .set("Content-Type", "text/plain")
            .send_string(tx_hex)
        {
            Ok(r) => r.into_string().map(|s| s.trim().to_string()).map_err(|e| e.to_string()),
            Err(ureq::Error::Status(code, r)) => Err(format!(
                "the server rejected the transaction ({code}): {}",
                r.into_string().unwrap_or_default()
            )),
            Err(e) => Err(format!("broadcast failed: {e}")),
        }
    }
}

// ---------------------------------------------------------------------------------------------

pub fn local_spk(master: MasterAppkey, internal: bool, index: u32) -> LocalSpk {
    let index = NormalIndex::new(index).expect("index in normal range");
    LocalSpk {
        master_appkey: master,
        bip32_path: if internal {
            BitcoinBip32Path::internal(index)
        } else {
            BitcoinBip32Path::external(index)
        },
    }
}

pub fn address(master: MasterAppkey, network: Network, internal: bool, index: u32) -> Address {
    Address::from_script(&local_spk(master, internal, index).spk(), network)
        .expect("p2tr has an address")
}

#[derive(Debug, Clone, PartialEq)]
pub struct OwnedUtxo {
    pub utxo: EsploraUtxo,
    pub internal: bool,
    pub index: u32,
}

#[derive(Debug, Clone, Default)]
pub struct WalletView {
    pub utxos: Vec<OwnedUtxo>,
    pub next_external: u32,
    pub next_internal: u32,
}

impl WalletView {
    pub fn balance(&self) -> u64 {
        self.utxos.iter().map(|u| u.utxo.value).sum()
    }
    pub fn confirmed_balance(&self) -> u64 {
        self.utxos.iter().filter(|u| u.utxo.confirmed).map(|u| u.utxo.value).sum()
    }
}

/// Scan both keychains until `GAP_LIMIT` consecutive unused addresses.
pub fn sync(
    esplora: &dyn Esplora,
    master: MasterAppkey,
    network: Network,
) -> Result<WalletView, String> {
    let mut view = WalletView::default();
    for internal in [false, true] {
        let (mut index, mut gap, mut next) = (0u32, 0u32, 0u32);
        while gap < GAP_LIMIT {
            let addr = address(master, network, internal, index);
            if esplora.address_used(&addr)? {
                gap = 0;
                next = index + 1;
                for utxo in esplora.address_utxos(&addr)? {
                    view.utxos.push(OwnedUtxo { utxo, internal, index });
                }
            } else {
                gap += 1;
            }
            index += 1;
        }
        if internal {
            view.next_internal = next;
        } else {
            view.next_external = next;
        }
    }
    Ok(view)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Amt {
    Sats(u64),
    Max,
}

#[derive(Debug, Clone)]
pub struct BuiltSend {
    pub template: TransactionTemplate,
    pub fee: u64,
    pub fee_rate: f64,
    pub recipient: Address,
    pub amount: u64,
    pub change: Option<u64>,
}

fn out_vbytes(spk: &ScriptBuf) -> f64 {
    // value(8) + script length varint(1) + script
    (9 + spk.len()) as f64
}

fn vsize(n_inputs: usize, out_spks: &[&ScriptBuf]) -> u64 {
    // overhead 10.5 (incl. segwit marker+flag), taproot key-path input 57.5
    (10.5 + 57.5 * n_inputs as f64 + out_spks.iter().map(|s| out_vbytes(s)).sum::<f64>()).ceil()
        as u64
}

pub fn build_send(
    view: &WalletView,
    master: MasterAppkey,
    network: Network,
    to: &str,
    amount: Amt,
    fee_rate: f64,
) -> Result<BuiltSend, String> {
    let recipient = Address::from_str(to.trim())
        .map_err(|e| format!("invalid address: {e}"))?
        .require_network(network)
        .map_err(|_| format!("that address is not a {network} address"))?;
    if !(fee_rate.is_finite() && fee_rate >= 1.0) {
        return Err("fee rate must be at least 1 sat/vB".into());
    }
    let to_spk = recipient.script_pubkey();
    let change_spk = local_spk(master, true, view.next_internal).spk();

    // Largest first: fewest inputs, so smallest fee. Unconfirmed coins last.
    let mut coins: Vec<&OwnedUtxo> = view.utxos.iter().collect();
    coins.sort_by_key(|c| (!c.utxo.confirmed, std::cmp::Reverse(c.utxo.value)));

    let fee_for = |n: usize, spks: &[&ScriptBuf]| (vsize(n, spks) as f64 * fee_rate).ceil() as u64;

    let (selected, amount, change, fee): (Vec<&OwnedUtxo>, u64, Option<u64>, u64) = match amount {
        Amt::Max => {
            if coins.is_empty() {
                return Err("the wallet has no coins".into());
            }
            let total: u64 = coins.iter().map(|c| c.utxo.value).sum();
            let fee = fee_for(coins.len(), &[&to_spk]);
            let amount = total.checked_sub(fee).ok_or("fees exceed the wallet balance")?;
            (coins, amount, None, fee)
        }
        Amt::Sats(amount) => {
            if amount < DUST {
                return Err(format!("amount is below the dust limit ({DUST} sats)"));
            }
            let mut selected = vec![];
            let mut total = 0u64;
            let mut result = None;
            for coin in coins {
                selected.push(coin);
                total += coin.utxo.value;
                let fee_no_change = fee_for(selected.len(), &[&to_spk]);
                let fee_change = fee_for(selected.len(), &[&to_spk, &change_spk]);
                if total >= amount + fee_change && total - amount - fee_change >= DUST {
                    result = Some((amount, Some(total - amount - fee_change), fee_change));
                    break;
                }
                if total >= amount + fee_no_change {
                    // leftover would be dust (or zero): give it to the miners
                    result = Some((amount, None, total - amount));
                    break;
                }
            }
            let (amount, change, fee) = result.ok_or("not enough funds")?;
            (selected, amount, change, fee)
        }
    };

    let mut template = TransactionTemplate::new();
    for c in &selected {
        let owner = local_spk(master, c.internal, c.index);
        let txout = TxOut { value: Amount::from_sat(c.utxo.value), script_pubkey: owner.spk() };
        template
            .push_owned_input(
                PushInput::spend_outpoint(&txout, OutPoint { txid: c.utxo.txid, vout: c.utxo.vout }),
                owner,
            )
            .map_err(|e| format!("wallet bug: {e:?}"))?;
    }
    template.push_foreign_output(TxOut { value: Amount::from_sat(amount), script_pubkey: to_spk });
    if let Some(change) = change {
        template.push_owned_output(Amount::from_sat(change), local_spk(master, true, view.next_internal));
    }
    debug_assert_eq!(template.fee(), Some(fee));
    Ok(BuiltSend { template, fee, fee_rate, recipient, amount, change })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    /// In-memory chain: addresses with coins, plus a set of "used" ones.
    #[derive(Default)]
    struct Fake {
        utxos: BTreeMap<String, Vec<EsploraUtxo>>,
        used: BTreeSet<String>,
    }
    impl Esplora for Fake {
        fn address_used(&self, a: &Address) -> Result<bool, String> {
            Ok(self.used.contains(&a.to_string()))
        }
        fn address_utxos(&self, a: &Address) -> Result<Vec<EsploraUtxo>, String> {
            Ok(self.utxos.get(&a.to_string()).cloned().unwrap_or_default())
        }
        fn fee_rate(&self, _: u32) -> Result<f64, String> {
            Ok(2.0)
        }
        fn broadcast(&self, _: &str) -> Result<String, String> {
            Err("n/a".into())
        }
    }

    fn master() -> MasterAppkey {
        // any valid key works for derivation tests
        use frostsnap_core::schnorr_fun::fun::{prelude::*, Point};
        let pk = Point::<Normal, Public, NonZero>::from_bytes(
            hex_to_33("0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798"),
        )
        .unwrap();
        MasterAppkey::derive_from_rootkey(pk)
    }
    fn hex_to_33(s: &str) -> [u8; 33] {
        let mut out = [0u8; 33];
        for i in 0..33 {
            out[i] = u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap();
        }
        out
    }

    fn txid(n: u8) -> Txid {
        Txid::from_str(&format!("{:064x}", n)).unwrap()
    }

    #[test]
    fn scan_respects_gap_and_builds_a_balanced_tx() {
        let m = master();
        let mut chain = Fake::default();
        // coins on receive #0 and receive #3 (gap of 2 in between is under the limit)
        for (i, (idx, value)) in [(0u32, 50_000u64), (3, 30_000)].into_iter().enumerate() {
            let a = address(m, Network::Signet, false, idx).to_string();
            chain.used.insert(a.clone());
            chain.utxos.entry(a).or_default().push(EsploraUtxo {
                txid: txid(i as u8 + 1),
                vout: 0,
                value,
                confirmed: true,
            });
        }
        // a used-and-spent change address at #0
        chain.used.insert(address(m, Network::Signet, true, 0).to_string());

        let view = sync(&chain, m, Network::Signet).unwrap();
        assert_eq!(view.balance(), 80_000);
        assert_eq!((view.next_external, view.next_internal), (4, 1));

        let dest = address(m, Network::Signet, false, 99).to_string();
        let built = build_send(&view, m, Network::Signet, &dest, Amt::Sats(20_000), 2.0).unwrap();
        assert_eq!(built.template.inputs().len(), 1, "largest coin alone suffices");
        let change = built.change.unwrap();
        assert_eq!(20_000 + change + built.fee, 50_000);
        assert_eq!(built.template.fee(), Some(built.fee));
        // change goes to the first unused internal index
        let (_, _, spk) = built.template.as_seen_by(m).iter_our_outputs().map(|(a, b, c)| (a, b.value, c.clone())).next().map(|(a, b, c)| (a, b, c)).unwrap();
        assert_eq!((spk.bip32_path.index.to_u32(), spk.bip32_path.account_keychain.keychain), (1, frostsnap_core::tweak::Keychain::Internal));

        let max = build_send(&view, m, Network::Signet, &dest, Amt::Max, 2.0).unwrap();
        assert_eq!(max.template.inputs().len(), 2);
        assert_eq!(max.amount + max.fee, 80_000);
        assert!(max.change.is_none());
    }

    #[test]
    fn rejects_wrong_network_dust_and_overspend() {
        let m = master();
        let mut chain = Fake::default();
        let a = address(m, Network::Signet, false, 0).to_string();
        chain.used.insert(a.clone());
        chain.utxos.insert(a, vec![EsploraUtxo { txid: txid(1), vout: 0, value: 10_000, confirmed: true }]);
        let view = sync(&chain, m, Network::Signet).unwrap();
        let mainnet = address(m, Network::Bitcoin, false, 0).to_string();
        assert!(build_send(&view, m, Network::Signet, &mainnet, Amt::Sats(1000), 2.0).is_err());
        let dest = address(m, Network::Signet, false, 5).to_string();
        assert!(build_send(&view, m, Network::Signet, &dest, Amt::Sats(100), 2.0).unwrap_err().contains("dust"));
        assert!(build_send(&view, m, Network::Signet, &dest, Amt::Sats(10_000), 2.0).unwrap_err().contains("not enough"));
    }
}
