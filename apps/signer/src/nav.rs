//! Which screen is up, and the flows that move between them: scan a coordinator code and walk the
//! user through whatever it asks.

use minifrost_signer::api::{
    Prompt,
    Step,
    StepResult,
};
use minifrost_ui::prelude::*;

use crate::ctl::Ctl;

#[derive(Clone, PartialEq)]
pub enum Screen {
    Home,
    /// Reads a coordinator code, then follows wherever it leads.
    Scan { expecting: String },
    /// A code for the coordinator to scan. With `next`, the title of the coordinator code that
    /// follows it, which the user can scan from here.
    Reply {
        frames: Vec<String>,
        title: String,
        what: String,
        next: Option<String>,
    },
    Prompt(Prompt),
    Quiz { key_name: String },
    EnterBackup,
}

/// The screens, as a stack over Home. `Copy`, so flows keep it across awaits.
#[derive(Clone, Copy, PartialEq)]
pub struct Nav {
    stack: State<Vec<Screen>>,
    pub overlay: Overlay,
}

pub const EXPECTING_ANY: &str = "coordinator message (key generation, transaction, backup request…)";

impl Nav {
    pub fn new(overlay: Overlay) -> Self {
        Self {
            stack: State::create(vec![Screen::Home]),
            overlay,
        }
    }

    pub fn top(&self) -> Screen {
        self.stack.read().last().cloned().unwrap_or(Screen::Home)
    }

    pub fn push(mut self, screen: Screen) {
        self.stack.write().push(screen);
    }

    pub fn pop(mut self) {
        let mut stack = self.stack.write();
        if stack.len() > 1 {
            stack.pop();
        }
    }

    /// Back to Home, where every flow starts and ends.
    pub fn home(mut self) {
        self.stack.write().truncate(1);
    }

    /// The Back key: closes a dialog, else leaves the screen. False on Home.
    pub fn back(self, ctl: &Ctl) -> bool {
        if self.overlay.dismiss() {
            return true;
        }
        let top = self.stack.peek().last().cloned();
        match top.unwrap_or(Screen::Home) {
            Screen::Home => false,
            // giving up on a backup check
            Screen::Quiz { .. } => {
                ctl.reject();
                self.home();
                true
            }
            _ => {
                self.pop();
                true
            }
        }
    }
}

/// What the coordinator code that was just read asked for; None when it needs nothing.
fn read_kind(step: &Step) -> Option<&'static str> {
    Some(match step {
        Step::Confirm(prompt) => match prompt {
            Prompt::JoinKeygen { .. } => "key generation invitation",
            Prompt::VerifyKeygen { .. } => "key generation check code",
            Prompt::ReviewTransaction(_) => "transaction to review",
            Prompt::SignTransaction(_) => "transaction to sign",
            Prompt::DisplayBackup { .. } => "backup request",
        },
        Step::CheckBackup { .. } => "request to check a backup",
        Step::Reply { .. } | Step::Done { .. } => "coordinator message",
        Step::Ignored { .. } | Step::UnlockShare { .. } => return None,
    })
}

/// A coordinator code was read: hand it to the signer and show what it asks for.
pub async fn receive(ctl: Ctl, nav: Nav, payload: Vec<u8>) {
    nav.home();
    match ctl.receive(&payload).await {
        Ok(r) => {
            if let Some(kind) = read_kind(&r.step) {
                nav.overlay.success(format!("Read from coordinator: {kind}"), None);
            }
            handle(nav, r.step).await;
        }
        Err(e) => error(nav, e).await,
    }
}

/// Shows what a step asks of the user.
pub async fn handle(nav: Nav, step: Step) {
    match step {
        Step::Ignored { reason } => nav.overlay.toast(reason),
        // the controller opens the share and reads the code again; this is only left if that failed
        Step::UnlockShare { key_name, .. } => nav.overlay.toast(format!("The share of “{key_name}” could not be opened.")),
        Step::Done { message } => nav.overlay.alert("Done", message).await,
        Step::CheckBackup { key_name } => nav.push(Screen::Quiz { key_name }),
        Step::Reply { frames, what, next } => nav.push(Screen::Reply {
            frames,
            title: "Show to coordinator".into(),
            what,
            next,
        }),
        Step::Confirm(prompt) => nav.push(Screen::Prompt(prompt)),
    }
}

/// What came of a step on one of the flow's screens: back to Home, then on to the next step.
pub async fn next(nav: Nav, result: Result<StepResult, String>) {
    match result {
        Ok(r) => {
            nav.home();
            handle(nav, r.step).await;
        }
        Err(e) => error(nav, e).await,
    }
}

pub async fn error(nav: Nav, e: String) {
    nav.overlay.alert("Can’t continue", e).await
}
