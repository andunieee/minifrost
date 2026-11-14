use std::collections::BTreeSet;

use minifrost_ui::prelude::*;

use crate::nav::{
    App,
    Nav,
    Page,
};

#[derive(PartialEq)]
pub struct WalletsPage;

impl Component for WalletsPage {
    fn render(&self) -> impl IntoElement {
        let p = use_palette();
        let app = use_consume::<App>();
        let nav = use_consume::<Nav>();
        nav.watch();
        let keys = app.keys();
        let create = {
            let app = app.clone();
            move |_| {
                if app.signers().is_empty() {
                    nav.overlay.toast("Add signers first.");
                } else {
                    nav.push(Page::NewWallet);
                }
            }
        };

        rect()
            .expanded()
            .child(app_bar(&p, "Wallets", None))
            .child(
                page(720.)
                    .maybe(keys.is_empty(), |el| el.child(text("No wallets yet. Create one with your signers.")))
                    .children(keys.into_iter().map(|k| {
                        let names: Vec<&str> = k.signers.iter().map(|s| s.name.as_str()).collect();
                        let subtitle = format!(
                            "{}-of-{} · {} · {}",
                            k.threshold,
                            k.n_keys,
                            network_label(&k.network),
                            names.join(", ")
                        );
                        let key_id = k.key_id.clone();
                        ListRow::new(label().text(k.name.clone()))
                            .icon(lucide::wallet(), p.accent)
                            .subtitle(subtitle)
                            .trailing(icon(lucide::chevron_right(), 20., p.text_muted))
                            .on_press(move |_| nav.push(Page::WalletDetail { key_id: key_id.clone() }))
                            .key(&k.key_id)
                            .into_element()
                    }))
                    .child(gap(72.)),
            )
            .child(floating(Btn::new("New wallet").icon(lucide::plus()).on_press(create)))
    }
}

/// Name a new wallet, pick its network, signers and threshold, then start the key generation.
#[derive(PartialEq)]
pub struct NewWalletPage;

impl Component for NewWalletPage {
    fn render(&self) -> impl IntoElement {
        let p = use_palette();
        let app = use_consume::<App>();
        let nav = use_consume::<Nav>();
        let signers = use_hook({
            let app = app.clone();
            move || app.signers()
        });
        let name = use_state(String::new);
        let mut network = use_state(|| "signet".to_string());
        let mut chosen = use_state({
            let signers = signers.clone();
            move || signers.iter().map(|s| s.device_id.clone()).collect::<BTreeSet<_>>()
        });
        let mut threshold = use_state(|| None::<usize>);

        let n = chosen.read().len();
        let effective = threshold().unwrap_or(n.div_ceil(2)).clamp(1, n.max(1));
        let valid = !name.read().trim().is_empty() && n > 0;
        let start = move |_| {
            let key_name = name.peek().trim().to_string();
            let ids: Vec<String> = chosen.peek().iter().cloned().collect();
            match app.start_keygen(key_name.clone(), effective as u16, ids, network.peek().clone()) {
                Ok(update) => nav.replace(Page::Ceremony {
                    title: format!("Create “{key_name}”"),
                    update,
                    key_id: None,
                }),
                Err(e) => nav.overlay.toast(e),
            }
        };

        rect().expanded().child(app_bar(&p, "New wallet", back(move |_| nav.pop()))).child(
            page(560.)
                .child(field(&p, "Wallet name", Input::new(name).width(Size::fill()).auto_focus(true)))
                .child(small("Network", p.text_muted))
                .children(NETWORKS.iter().map(|(id, label)| {
                    let id = id.to_string();
                    radio_row(&p, *network.read() == id, *label, {
                        let id = id.clone();
                        move |_| network.set(id.clone())
                    })
                    .key(id)
                    .into_element()
                }))
                .child(small("Signers", p.text_muted))
                .children(signers.iter().map(|s| {
                    let id = s.device_id.clone();
                    check_row(&p, chosen.read().contains(&id), signer_name(&p, &id, s.name.clone()), {
                        let id = id.clone();
                        move |_| {
                            let mut set = chosen.write();
                            if !set.remove(&id) {
                                set.insert(id.clone());
                            }
                        }
                    })
                    .key(id)
                    .into_element()
                }))
                .child(small("Signatures needed", p.text_muted))
                .child(button_row().children((1..=n.max(1)).map(|t| {
                    Btn::new(format!("{t} of {n}"))
                        .kind(if t == effective { BtnKind::Filled } else { BtnKind::Outline })
                        .on_press(move |_| threshold.set(Some(t)))
                        .key(t)
                        .into_element()
                })))
                .child(gap(8.))
                .child(Btn::new("Start").fill().enabled(valid).on_press(start)),
        )
    }
}
