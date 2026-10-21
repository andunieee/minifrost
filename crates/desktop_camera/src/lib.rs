//! Webcam capture for the coordinator desktop app (Linux/V4L2). Ported from frostsnap's
//! `desktop_camera` crate, trimmed to the Linux backend.
pub mod common;
pub mod decode;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::LinuxCamera as Camera;

pub use common::{
    CameraError, CameraSink, DeviceChange, DeviceInfo, Frame, FrameFormat, Resolution, Result,
};
