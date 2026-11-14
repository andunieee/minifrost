use minifrost_signer::api::{
    KeyInfo,
    PendingBackup,
};
use minifrost_ui::prelude::*;

use crate::{
    ctl::Ctl,
    keystore,
    nav::{
        self,
        EXPECTING_ANY,
        Nav,
        Screen,
    },
};

const SCREEN_LOCK_RISK: &str = "If you ever remove the screen lock, Android permanently deletes the device key store key, \
    and every share on the key store can no longer be used. You would have to restore each one from its backup words.";

#[derive(PartialEq)]
pub struct Home;

impl Component for Home {
    fn render(&self) -> impl IntoElement {
        let p = use_palette();
        let ctl = use_consume::<Ctl>();
        let nav = use_consume::<Nav>();
        let data = ctl.data.read().clone();
        let color = signer_color(&ctl.device_id, p.dark);

        let menu = {
            let ctl = ctl.clone();
            move |_| {
                let ctl = ctl.clone();
                run(async move {
                    let on = ctl.data.peek().new_on_keystore;
                    let choice = Choice::new(
                        if on { lucide::square_check() } else { lucide::square() },
                        "Store new shares on the device key store",
                    );
                    if nav.overlay.choose("Settings", "", vec![choice]).await == Some(0) {
                        toggle_new_on_keystore(ctl, nav).await;
                    }
                });
            }
        };
        let hello = {
            let ctl = ctl.clone();
            move |_| {
                nav.push(Screen::Reply {
                    frames: ctl.hello_frames(),
                    title: "Enrolment code".into(),
                    what: "Let the coordinator scan this to add you as a signer.".into(),
                    next: None,
                })
            }
        };

        rect()
            .expanded()
            .child(
                app_bar(&p, "Minifrost Signer", None).maybe(keystore::SUPPORTED, |el| {
                    el.child(IconBtn::new(lucide::ellipsis_vertical(), menu))
                }),
            )
            .child(
                page(720.)
                    .child(
                        ListRow::new(signer_name(&p, &ctl.device_id, ctl.name.clone()).font_size(16.))
                            .icon(lucide::shield(), color)
                            .subtitle(format!("id {}", short_id(&ctl.device_id, 16)))
                            .border(color),
                    )
                    .child(title("Keys held by this signer"))
                    .maybe(data.keys.is_empty(), |el| {
                        el.child(text(
                            "None yet. Show your enrolment code to a coordinator, then scan its key generation codes, \
                             or restore a key from its backup.",
                        ))
                    })
                    .children(data.keys.iter().map(|k| KeyRow { key: k.clone() }.key(&k.key_id).into_element()))
                    .maybe(!data.pending.is_empty(), |el| {
                        el.child(gap(4.)).child(title("Restoring")).children(
                            data.pending.iter().map(|b| PendingRow { backup: b.clone() }.key(&b.id).into_element()),
                        )
                    })
                    .child(gap(4.))
                    .child(
                        Btn::new("Restore from a backup")
                            .outline()
                            .icon(lucide::notebook_pen())
                            .fill()
                            .on_press(move |_| nav.push(Screen::EnterBackup)),
                    )
                    .child(Btn::new("Show enrolment code").outline().icon(lucide::qr_code()).fill().on_press(hello))
                    // room for the scan button
                    .child(gap(72.)),
            )
            .child(floating(Btn::new("Scan coordinator").icon(lucide::scan_qr_code()).on_press(move |_| {
                nav.push(Screen::Scan {
                    expecting: EXPECTING_ANY.into(),
                })
            })))
    }
}

#[derive(PartialEq)]
struct KeyRow {
    key: KeyInfo,
}

impl KeyRow {
    fn key(self, id: &str) -> impl IntoElement {
        rect().key(id.to_string()).child(self)
    }
}

impl Component for KeyRow {
    fn render(&self) -> impl IntoElement {
        let p = use_palette();
        let ctl = use_consume::<Ctl>();
        let nav = use_consume::<Nav>();
        let k = self.key.clone();
        let place = if k.on_keystore { "on keystore" } else { "on disk" };
        let subtitle = format!(
            "{}-of-? · {} · {place}\n{}",
            k.threshold,
            network_label(&k.network),
            short_id(&k.key_id, 16)
        );
        ListRow::new(label().text(k.name.clone()))
            .icon(lucide::key_round(), p.text_muted)
            .subtitle(subtitle)
            .on_press(move |_| run(key_menu(ctl.clone(), nav, k.clone())))
    }
}

async fn key_menu(ctl: Ctl, nav: Nav, k: KeyInfo) {
    let mut choices = vec![Choice::new(lucide::qr_code(), "Restore wallet on coordinator")];
    if keystore::SUPPORTED {
        choices.push(if k.on_keystore {
            Choice::new(lucide::save(), "Move this share back to disk")
        } else {
            Choice::new(lucide::lock(), "Move this share to the device key store")
        });
    }
    choices.push(Choice::new(lucide::trash_2(), "Delete this share").danger());
    let place = if k.on_keystore { "On keystore" } else { "On disk" };
    let Some(i) = nav.overlay.choose(k.name.clone(), place, choices).await else {
        return;
    };
    let delete = if keystore::SUPPORTED { 2 } else { 1 };
    if i == 0 {
        match ctl.restore_code(&k.key_id).await {
            Ok(step) => nav::handle(nav, step).await,
            Err(e) => nav::error(nav, e).await,
        }
    } else if i == delete {
        delete_key(ctl, nav, k).await;
    } else if k.on_keystore {
        let ok = nav
            .overlay
            .confirm(
                "Move this share back to disk?",
                "Confirm your screen lock to take the share out of the device key store. It is then stored in \
                 this app's file, like before.",
                "Move to disk",
                false,
            )
            .await;
        if ok {
            report(nav, ctl.move_to_disk(&k.key_id).await, "The share is on disk.");
        }
    } else {
        if !screen_lock_ready(nav).await {
            return;
        }
        let ok = nav
            .overlay
            .confirm(
                "Move this share to the device key store?",
                format!(
                    "Using this share (signing, showing or checking its backup) will then need this phone's screen \
                     lock: PIN, pattern or password.\n\n{SCREEN_LOCK_RISK}\n\nMake sure this share's backup is \
                     written down and checked before you continue."
                ),
                "Move",
                false,
            )
            .await;
        if ok {
            report(nav, ctl.move_to_keystore(&k.key_id).await, "The share is on the device key store.");
        }
    }
}

/// Three steps, since this is the one action on the signer that destroys a key.
async fn delete_key(ctl: Ctl, nav: Nav, k: KeyInfo) {
    let (name, t) = (&k.name, k.threshold);
    let first = nav
        .overlay
        .confirm(
            format!("Delete this share of “{name}”?"),
            format!(
                "The share is erased from this signer for good. Only its 25 backup words can bring it back.\n\n\
                 If you don't have its backup, and the wallet's other signers can't make up {t} signatures without \
                 this one, the money in “{name}” is lost forever."
            ),
            "Continue",
            true,
        )
        .await;
    if !first {
        return;
    }
    let second = nav
        .overlay
        .confirm(
            "Do you have this share's backup?",
            format!(
                "Check that the 25 words and key number of this share are written down, or that {t} other signers \
                 of “{name}” still hold their keys.\n\nThe coordinator still lists this signer for the wallet: \
                 forget it there too, on the wallet's page."
            ),
            "I understand",
            true,
        )
        .await;
    if !second {
        return;
    }
    let sure = nav
        .overlay
        .confirm_typing("Delete this share", "This can't be undone.", name.clone(), "Delete share")
        .await;
    if sure {
        report(nav, ctl.delete_key(&k.key_id), &format!("The share of “{name}” was deleted."));
    }
}

#[derive(PartialEq)]
struct PendingRow {
    backup: PendingBackup,
}

impl PendingRow {
    fn key(self, id: &str) -> impl IntoElement {
        rect().key(id.to_string()).child(self)
    }
}

impl Component for PendingRow {
    fn render(&self) -> impl IntoElement {
        let p = use_palette();
        let ctl = use_consume::<Ctl>();
        let nav = use_consume::<Nav>();
        let b = self.backup.clone();
        let of = b.key_name.as_ref().map(|n| format!(" of “{n}”")).unwrap_or_default();
        let code = b.restore_code.clone();
        let delete = {
            let b = b.clone();
            move |_| {
                let (ctl, b) = (ctl.clone(), b.clone());
                run(async move {
                    let ok = nav
                        .overlay
                        .confirm(
                            format!("Delete backup #{}?", b.share_index),
                            "The backup you typed in is erased from this signer. Your paper copy is unaffected: you \
                             can type it in again any time.",
                            "Delete",
                            true,
                        )
                        .await;
                    if ok {
                        report(nav, ctl.delete_pending_backup(&b.id), "The backup was deleted.");
                    }
                })
            }
        };
        ListRow::new(label().text(format!("Backup #{}{of}", b.share_index)))
            .icon(lucide::history(), p.text_muted)
            .subtitle(
                "Saved on this signer. It becomes a normal key once the coordinator has rebuilt the wallet. Tap to show \
                 its restore code.",
            )
            .on_press(move |_| run(nav::handle(nav, code.clone())))
            .trailing(IconBtn::new(lucide::trash_2(), delete))
    }
}

async fn toggle_new_on_keystore(ctl: Ctl, nav: Nav) {
    if ctl.data.peek().new_on_keystore {
        report(nav, ctl.set_new_on_keystore(false), "New shares will be stored on disk.");
        return;
    }
    if !screen_lock_ready(nav).await {
        return;
    }
    let ok = nav
        .overlay
        .confirm(
            "Store new shares on the device key store?",
            format!(
                "Every share this signer creates from now on goes to the device key store as soon as it is stored. \
                 Using those shares will need this phone's screen lock: PIN, pattern or password. Shares already on \
                 this signer stay where they are.\n\n{SCREEN_LOCK_RISK}\n\nWrite down and check the backup of each \
                 new share."
            ),
            "Turn on",
            false,
        )
        .await;
    if ok {
        report(nav, ctl.set_new_on_keystore(true), "New shares will be stored on the device key store.");
    }
}

/// The key store needs a screen lock; says so when there is none.
async fn screen_lock_ready(nav: Nav) -> bool {
    if keystore::screen_lock_set().await {
        return true;
    }
    nav.overlay
        .alert(
            "No screen lock",
            "The device key store needs a screen lock. Set up a PIN, pattern or password in the phone's settings first.",
        )
        .await;
    false
}

fn report(nav: Nav, result: Result<(), String>, done: &str) {
    match result {
        Ok(()) => nav.overlay.toast(done),
        Err(e) => nav.overlay.toast(e),
    }
}
