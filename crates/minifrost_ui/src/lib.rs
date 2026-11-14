//! What the two Minifrost apps share: the look, the animated QR code, the QR scanner (camera,
//! sound, pasted text), toasts and dialogs, and the platform glue for the desktop and Android.

pub mod camera;
pub mod format;
pub mod overlay;
pub mod platform;
pub mod qr;
pub mod scan;
pub mod sound;
pub mod theme;
pub mod tx_card;
pub mod widgets;

pub use freya;

/// Everything a screen needs.
pub mod prelude {
    pub use freya::{
        icons::lucide,
        prelude::*,
    };

    pub use crate::{
        format::*,
        overlay::*,
        platform::{
            DataDir,
            Insets,
            use_insets,
        },
        qr::*,
        scan::ScanPanel,
        theme::*,
        tx_card::*,
        widgets::*,
    };
}
