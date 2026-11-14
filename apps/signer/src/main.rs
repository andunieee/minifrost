//! The desktop signer. Its key shares live in a plain file: not for serious funds.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use minifrost_ui::{
    platform::{
        DataDir,
        desktop_data_dir,
    },
    prelude::*,
};

const ICON: &[u8] = include_bytes!("../../../assets/logo.png");

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("MINIFROST_LOG").unwrap_or_else(|_| "warn".into()),
        )
        .init();
    let dir = DataDir(desktop_data_dir("dev.minifrost.minifrost_signer", "dev.minifrost.minifrostSigner"));
    launch(
        LaunchConfig::new().with_window(
            WindowConfig::new(minifrost_signer_app::app)
                .with_root_context(dir)
                .with_title("Minifrost Signer")
                .with_app_id("dev.minifrost.minifrost_signer")
                .with_icon(LaunchConfig::window_icon(ICON))
                .with_size(480., 860.)
                .with_min_size(360., 560.),
        ),
    )
}
