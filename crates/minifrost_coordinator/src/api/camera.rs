//! Webcam -> QR text. Capture and decoding both happen in Rust; Dart gets preview JPEGs to show
//! and decoded QR strings to feed into a `QrScanner`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use desktop_camera::{Camera, CameraSink, DeviceInfo, Frame};

use crate::frb_generated::StreamSink;

#[derive(Debug, Clone)]
pub struct CameraInfo {
    pub index: u32,
    pub name: String,
}

#[derive(Clone)]
pub enum CameraEvent {
    /// A JPEG to display as the live preview.
    Preview(Vec<u8>),
    /// Text of a QR code seen in the picture.
    Qr(String),
}

static RUNNING: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);

pub fn list_cameras() -> Vec<CameraInfo> {
    Camera::list_devices()
        .unwrap_or_default()
        .into_iter()
        .map(|d| CameraInfo { index: d.index as u32, name: d.name })
        .collect()
}

struct Sink {
    stop: Arc<AtomicBool>,
    events: StreamSink<CameraEvent>,
    latest: Arc<Mutex<Option<Vec<u8>>>>,
    n: Mutex<u64>,
}

impl CameraSink for Sink {
    fn send_frame(&self, frame: Frame) -> desktop_camera::Result<()> {
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
pub fn start_camera(index: u32, events: StreamSink<CameraEvent>) -> Result<(), String> {
    stop_camera();
    let info: DeviceInfo = Camera::list_devices()
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

    let sink = Sink { stop: stop.clone(), events, latest, n: Mutex::new(0) };
    let camera = Camera::open(&info, sink).map_err(|e| e.to_string())?;
    std::mem::forget(camera); // the capture thread ends itself when `stop` is set
    Ok(())
}

pub fn stop_camera() {
    if let Some(flag) = RUNNING.lock().unwrap().take() {
        flag.store(true, Ordering::Relaxed);
    }
}

fn decode_qr(jpeg: &[u8]) -> Vec<String> {
    let Ok(img) = image::load_from_memory_with_format(jpeg, image::ImageFormat::Jpeg) else {
        return vec![];
    };
    let luma = img.to_luma8();
    let mut prepared = rqrr::PreparedImage::prepare(luma);
    prepared.detect_grids().into_iter().filter_map(|g| g.decode().ok().map(|(_, text)| text)).collect()
}
