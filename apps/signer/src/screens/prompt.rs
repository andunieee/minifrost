use minifrost_signer::api::{
    Prompt,
    TxSummary,
};
use minifrost_ui::prelude::*;

use crate::{
    ctl::Ctl,
    nav::{
        self,
        Nav,
    },
};

/// Asks the user to approve or reject what the coordinator asked for.
#[derive(PartialEq)]
pub struct PromptScreen {
    pub prompt: Prompt,
}

impl Component for PromptScreen {
    fn render(&self) -> impl IntoElement {
        let p = use_palette();
        let ctl = use_consume::<Ctl>();
        let nav = use_consume::<Nav>();
        let mut busy = use_state(|| false);

        let (screen_title, body, approve): (&str, Element, &str) = match &self.prompt {
            Prompt::JoinKeygen {
                key_name,
                threshold,
                n_signers,
                network,
            } => (
                "New key",
                big_text(format!(
                    "Create the {} wallet “{key_name}”.\n\n{threshold} of {n_signers} signers will be needed to spend \
                     from it. Approve only if you started this on your coordinator.",
                    network_label(network).to_uppercase()
                ))
                .into(),
                "Join",
            ),
            Prompt::VerifyKeygen {
                key_name,
                threshold,
                n_signers,
                check_code,
            } => (
                "Compare check code",
                rect()
                    .width(Size::fill())
                    .spacing(16.)
                    .child(big_text(format!(
                        "Key “{key_name}” ({threshold} of {n_signers}).\n\nEvery signer and the coordinator must show \
                         exactly this code. If any differs, reject: someone is not seeing the same key."
                    )))
                    .child(
                        mono(check_code.clone(), 34.)
                            .width(Size::fill())
                            .font_weight(FontWeight::BOLD)
                            .text_align(TextAlign::Center),
                    )
                    .into(),
                "Codes match, store my share",
            ),
            Prompt::ReviewTransaction(s) => (
                "Review transaction",
                tx(&p, s, "Approving releases fresh nonces so signing can start. Nothing is signed yet.").into(),
                "Release nonces",
            ),
            Prompt::SignTransaction(s) => (
                "Sign transaction",
                tx(&p, s, "Approving produces your partial signature. This cannot be undone.").into(),
                "Sign",
            ),
            Prompt::DisplayBackup {
                key_name,
                threshold,
                share_index,
                words,
            } => ("Secret backup", backup(&p, key_name, *threshold, *share_index, words).into(), "I wrote it down"),
        };

        let reject = {
            let ctl = ctl.clone();
            move |_| {
                ctl.reject();
                nav.home();
            }
        };
        let approve_press = move |_| {
            busy.set(true);
            let ctl = ctl.clone();
            run(async move {
                let result = ctl.approve().await;
                busy.set(false);
                nav::next(nav, result).await;
            });
        };

        rect()
            .expanded()
            .content(Content::flex())
            .child(app_bar(&p, screen_title, back(move |_| nav.pop())))
            .child(page(720.).child(body))
            // pinned, so the buttons stay reachable however long the content is
            .child(
                rect()
                    .horizontal()
                    .width(Size::fill())
                    .content(Content::flex())
                    .spacing(12.)
                    .padding((8., 16., 12., 16.))
                    .background(p.bg)
                    .child(rect().width(Size::flex(1.)).child(Btn::new("Reject").outline().fill().enabled(!busy()).on_press(reject)))
                    .child(
                        rect()
                            .width(Size::flex(1.))
                            .child(Btn::new(approve).fill().busy(busy()).on_press(approve_press)),
                    ),
            )
    }
}

fn big_text(s: String) -> Label {
    text(s).font_size(16.)
}

fn tx(p: &Palette, s: &TxSummary, note: &str) -> Rect {
    rect()
        .width(Size::fill())
        .spacing(8.)
        .child(
            text(format!("Wallet “{}” · {}", s.key_name, network_label(&s.network))).font_weight(FontWeight::BOLD),
        )
        .child(tx_card(
            p,
            Tx {
                outputs: s
                    .outputs
                    .iter()
                    .map(|o| TxOutput {
                        address: o.address.clone(),
                        sats: o.amount_sats,
                        own_label: o.own_label.clone(),
                    })
                    .collect(),
                fee_sats: s.fee_sats,
                fee_rate: s.fee_rate,
                n_inputs: Some(s.n_inputs),
                txid: Some(s.txid.clone()),
                warnings: s.warnings.clone(),
                verified: s.verified,
            },
        ))
        .child(text(note))
}

fn backup(p: &Palette, name: &str, threshold: u16, share_index: u32, words: &[String]) -> Rect {
    rect()
        .width(Size::fill())
        .spacing(12.)
        .child(
            tinted_card(p.error_soft, p.on_error_soft).child(
                note(
                    lucide::triangle_alert(),
                    p.on_error_soft,
                    format!(
                        "This backup is secret. Anyone holding {threshold} of the wallet’s backups can take all of its \
                         bitcoin. Write the words down on paper, in order. Do not photograph or type them anywhere."
                    ),
                )
                .font_weight(FontWeight::BOLD),
            ),
        )
        .child(text(format!("Wallet “{name}” · {threshold} signatures needed")).font_size(16.))
        .child(label().font_size(28.).text(format!("Key #{share_index}")))
        .child(
            rect()
                .horizontal()
                .width(Size::fill())
                .content(Content::wrap_spacing(8.))
                .spacing(8.)
                .children(words.iter().enumerate().map(|(i, w)| {
                    rect()
                        .horizontal()
                        .width(Size::px(160.))
                        .spacing(8.)
                        .cross_align(Alignment::center())
                        .child(
                            label()
                                .width(Size::px(28.))
                                .text_align(TextAlign::End)
                                .color(p.text_muted)
                                .text(format!("{}", i + 1)),
                        )
                        .child(mono(w.clone(), 18.).font_weight(FontWeight::BOLD))
                        .into_element()
                })),
        )
        .child(text("Also note the key number, the threshold and the wallet name next to the words."))
}
