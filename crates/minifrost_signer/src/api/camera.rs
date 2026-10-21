//! Webcam -> QR text on the desktop. A near-copy of the coordinator's camera api: capture and
//! decoding both happen in Rust; Dart gets preview JPEGs to show and decoded QR strings to feed
//! into a `QrScanner`. On non-desktop targets the real implementation is compiled out so the
//! Android build never links V4L2; the generated bindings still compile against stubs there.
//!
//! NOTE: the pub functions must stay at module top level — flutter_rust_bridge bakes their
//! full path into the generated bindings.

use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(target_os = "linux")]
use std::sync::{Arc, Mutex};

use crate::frb_generated::StreamSink;

#[derive(Debug, Clone)]
pub struct CameraInfo {
    pub index: u32,
    pub name: String,
}

#[derive(Clone)]
#[allow(dead_code)] // the Qr variant is only sent on desktop
pub enum CameraEvent {
    /// A JPEG to display as the live preview.
    Preview(Vec<u8>),
    /// Text of a QR code seen in the picture.
    Qr(String),
}

// ------------------------------------------------------------------------------------------
// Linux (V4L2) implementation

#[cfg(target_os = "linux")]
static RUNNING: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);

#[cfg(target_os = "linux")]
pub fn list_cameras() -> Result<Vec<CameraInfo>, String> {
    Ok(desktop_camera::Camera::list_devices()
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|d| CameraInfo {
            index: d.index as u32,
            name: d.name,
        })
        .collect())
}

#[cfg(target_os = "linux")]
struct Sink {
    stop: Arc<AtomicBool>,
    events: StreamSink<CameraEvent>,
    latest: Arc<Mutex<Option<Vec<u8>>>>,
    n: Mutex<u64>,
}

#[cfg(target_os = "linux")]
impl desktop_camera::CameraSink for Sink {
    fn send_frame(&self, frame: desktop_camera::Frame) -> desktop_camera::Result<()> {
        if self.stop.load(Ordering::Relaxed) {
            return Err(desktop_camera::CameraError::DeviceError("stopped".into()));
        }
        let mut n = self.n.lock().unwrap();
        *n += 1;
        // ~15 fps preview from a 30 fps camera
        if *n % 2 == 0 {
            self.events
                .add(CameraEvent::Preview(frame.data.clone()))
                .map_err(|_| desktop_camera::CameraError::DeviceError("ui closed".into()))?;
        }
        *self.latest.lock().unwrap() = Some(frame.data);
        Ok(())
    }
}

/// Start streaming from camera `index`. Any previously running camera is stopped.
#[cfg(target_os = "linux")]
pub fn start_camera(index: u32, events: StreamSink<CameraEvent>) -> Result<(), String> {
    stop_camera();
    let info: desktop_camera::DeviceInfo = desktop_camera::Camera::list_devices()
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|d| d.index == index as usize)
        .ok_or("camera not found")?;

    let stop = Arc::new(AtomicBool::new(false));
    *RUNNING.lock().unwrap() = Some(stop.clone());
    let latest = Arc::new(Mutex::new(None::<Vec<u8>>));

    // decoder thread: always works on the newest frame, never queues
    {
        let (stop, latest, events) = (stop.clone(), latest.clone(), events.clone());
        std::thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                let Some(jpeg) = latest.lock().unwrap().take() else {
                    std::thread::sleep(std::time::Duration::from_millis(15));
                    continue;
                };
                for text in decode_qr(&jpeg) {
                    if events.add(CameraEvent::Qr(text)).is_err() {
                        return;
                    }
                }
            }
        });
    }

    let sink = Sink {
        stop: stop.clone(),
        events,
        latest,
        n: Mutex::new(0),
    };
    let camera = desktop_camera::Camera::open(&info, sink).map_err(|e| e.to_string())?;
    std::mem::forget(camera); // the capture thread ends itself when `stop` is set
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn stop_camera() {
    if let Some(flag) = RUNNING.lock().unwrap().take() {
        flag.store(true, Ordering::Relaxed);
    }
}

#[cfg(target_os = "linux")]
fn decode_qr(jpeg: &[u8]) -> Vec<String> {
    let Ok(img) = image::load_from_memory_with_format(jpeg, image::ImageFormat::Jpeg) else {
        return vec![];
    };
    let luma = img.to_luma8();
    let mut prepared = rqrr::PreparedImage::prepare(luma);
    prepared
        .detect_grids()
        .into_iter()
        .filter_map(|g| g.decode().ok().map(|(_, text)| text))
        .collect()
}

// ------------------------------------------------------------------------------------------
// Non-desktop stubs

/// Stubs so the generated bindings compile on Android too; the Dart side never calls them
/// there (it uses the `camera` plugin instead).
#[cfg(not(target_os = "linux"))]
pub fn list_cameras() -> Result<Vec<CameraInfo>, String> {
    Err("webcam scanning is only available in the desktop build".into())
}

#[cfg(not(target_os = "linux"))]
pub fn start_camera(_index: u32, _events: StreamSink<CameraEvent>) -> Result<(), String> {
    Err("webcam scanning is only available in the desktop build".into())
}

#[cfg(not(target_os = "linux"))]
pub fn stop_camera() {}
