//! The desktop coordinator.

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
    let dir = DataDir(desktop_data_dir("dev.minifrost.minifrost_coordinator", "dev.minifrost.minifrostCoordinator"));
    launch(
        LaunchConfig::new().with_window(
            WindowConfig::new(minifrost_coordinator_app::app)
                .with_root_context(dir)
                .with_title("Minifrost Coordinator")
                .with_app_id("dev.minifrost.minifrost_coordinator")
                .with_icon(LaunchConfig::window_icon(ICON))
                .with_size(1100., 780.)
                .with_min_size(360., 560.),
        ),
    )
}
