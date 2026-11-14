//! A transaction as the user must see it before approving anything.

use freya::{
    icons::lucide,
    prelude::*,
};

use crate::{
    format::*,
    theme::Palette,
    widgets::*,
};

pub struct TxOutput {
    pub address: String,
    pub sats: i64,
    /// Set for an output that comes back to the same wallet.
    pub own_label: Option<String>,
}

pub struct Tx {
    pub outputs: Vec<TxOutput>,
    pub fee_sats: i64,
    pub fee_rate: Option<f64>,
    pub n_inputs: Option<u32>,
    pub txid: Option<String>,
    pub warnings: Vec<String>,
    /// Checked against this signer's own key (always true on the coordinator, which built it).
    pub verified: bool,
}

pub fn tx_card(p: &Palette, tx: Tx) -> Rect {
    let fee = match tx.fee_rate {
        Some(rate) => format!("Network fee: {}  ({rate:.1} sat/vB)", format_sats(tx.fee_sats)),
        None => format!("Network fee: {}", format_sats(tx.fee_sats)),
    };
    card(p)
        .child(small("Sending", p.text_muted).font_weight(FontWeight::SEMI_BOLD))
        .children(tx.outputs.into_iter().map(|o| {
            rect()
                .width(Size::fill())
                .spacing(4.)
                .child(label().font_size(24.).text(format_btc(o.sats)))
                .child(small(format_sats(o.sats), p.text_muted))
                .map(o.own_label, |el, l| el.child(chip(p, format!("Back to this wallet · {l}"))))
                .maybe(!o.address.is_empty(), |el| {
                    el.child(
                        SelectableText::new()
                            .width(Size::fill())
                            .font_family(MONO)
                            .font_size(15.)
                            .span(group_address(&o.address)),
                    )
                })
                .child(gap(4.))
                .child(divider(p))
                .into_element()
        }))
        .child(text(fee))
        .map(tx.n_inputs, |el, n| el.child(text(format!("Inputs: {n}"))))
        .map(tx.txid, |el, txid| {
            el.child(
                SelectableText::new()
                    .width(Size::fill())
                    .font_family(MONO)
                    .font_size(11.)
                    .span(format!("txid {txid}")),
            )
        })
        .child(if tx.verified {
            note(lucide::shield_check(), p.success, "Checked against this signer’s own key.")
        } else {
            note(
                lucide::info(),
                p.warn,
                "Not yet checked against this signer’s key: shown as the coordinator presents it.",
            )
        })
        .children(tx.warnings.into_iter().map(|w| note(lucide::triangle_alert(), p.error, w).into_element()))
}
