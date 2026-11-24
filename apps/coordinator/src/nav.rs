//! Which page is up: one of the four tabs, with pages stacked over it.

use std::sync::Arc;

use minifrost_coordinator::api::{
    Broadcast,
    CoordinatorApp,
    RestoringShareView,
    SendPreview,
    UnnamedWalletView,
    Update,
};
use minifrost_ui::prelude::*;

/// The coordinator, shared with the threads that talk to Esplora.
#[derive(Clone)]
pub struct App(pub Arc<CoordinatorApp>);

impl PartialEq for App {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl std::ops::Deref for App {
    type Target = CoordinatorApp;
    fn deref(&self) -> &CoordinatorApp {
        &self.0
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tab {
    Wallets,
    Restore,
    Signers,
    Settings,
}

/// What the name of a restored wallet is asked for.
#[derive(Clone, PartialEq)]
pub enum Naming {
    /// Enough shares were read; none knows the wallet's name and network.
    Unnamed(UnnamedWalletView),
    /// A one-signature wallet from a single share.
    OneSignature(RestoringShareView),
}

#[derive(Clone, PartialEq)]
pub enum Page {
    WalletDetail { key_id: String },
    /// A keygen, signing, backup or restoring session. `key_id` is needed to publish a signed
    /// transaction.
    Ceremony {
        title: String,
        update: Update,
        key_id: Option<String>,
    },
    NewWallet,
    Enroll,
    NameWallet(Naming),
    ConfirmSend { key_id: String, preview: SendPreview },
    /// A QR code for the wallet's signers, who show the receive address from their own key.
    VerifyAddress {
        wallet: String,
        label: String,
        address: String,
        code: Broadcast,
    },
}

/// The tab and the pages over it. `Copy`, so flows keep it across awaits.
#[derive(Clone, Copy, PartialEq)]
pub struct Nav {
    pub tab: State<Tab>,
    stack: State<Vec<Page>>,
    /// Bumped after every change to the coordinator, so pages read it again.
    pub rev: State<u64>,
    pub overlay: Overlay,
}

impl Nav {
    pub fn new(overlay: Overlay) -> Self {
        Self {
            tab: State::create(Tab::Wallets),
            stack: State::create(vec![]),
            rev: State::create(0),
            overlay,
        }
    }

    pub fn top(&self) -> Option<Page> {
        self.stack.read().last().cloned()
    }

    pub fn push(mut self, page: Page) {
        self.stack.write().push(page);
    }

    /// Replaces the page on top (a form, by what it led to).
    pub fn replace(mut self, page: Page) {
        let mut stack = self.stack.write();
        stack.pop();
        stack.push(page);
    }

    pub fn pop(mut self) {
        self.stack.write().pop();
        self.changed();
    }

    pub fn set_tab(mut self, tab: Tab) {
        self.stack.write().clear();
        self.tab.set(tab);
    }

    /// A wallet's page over the Wallets tab, in place of whatever led to it (creating or restoring
    /// the wallet).
    pub fn open_wallet(self, key_id: String) {
        self.set_tab(Tab::Wallets);
        self.push(Page::WalletDetail { key_id });
        self.changed();
    }

    /// Something changed in the coordinator: the pages read it again.
    pub fn changed(mut self) {
        *self.rev.write() += 1;
    }

    /// Subscribes the calling component to changes.
    pub fn watch(&self) -> u64 {
        *self.rev.read()
    }

    /// The Back key: closes a dialog, else the page on top (a session asks first). False when
    /// there is nothing left to close.
    pub fn back(self, app: &App) -> bool {
        if self.overlay.dismiss() {
            return true;
        }
        let top = self.stack.peek().last().cloned();
        match top {
            None => false,
            Some(Page::Ceremony { update, .. }) => {
                run(crate::pages::ceremony::close(app.clone(), self, update.finished.is_some()));
                true
            }
            Some(_) => {
                self.pop();
                true
            }
        }
    }
}

/// Runs a call that may go to the network on another thread.
pub async fn off_thread<T: Send + 'static>(app: &App, f: impl FnOnce(&CoordinatorApp) -> T + Send + 'static) -> T {
    let app = app.0.clone();
    thread(move || f(&app)).await
}
