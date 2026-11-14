//! Camera -> QR texts and a preview to show. The webcam on Linux (V4L2, JPEG frames), the back
//! camera on Android (Camera2 in `QrCamera.java`, the luma plane of each frame). One decoder
//! thread always works on the newest frame and drops the rest.

use std::sync::{
    Arc,
    Condvar,
    Mutex,
    atomic::{
        AtomicBool,
        Ordering,
    },
};
use std::time::Duration;

use futures_channel::mpsc;

/// Longest side of the preview images.
const PREVIEW_SIDE: u32 = 640;

pub enum CameraEvent {
    /// RGBA pixels to show as the live preview.
    Preview { width: u32, height: u32, rgba: Vec<u8> },
    /// Text of a QR code seen in the picture.
    Qr(String),
    Error(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct CameraInfo {
    pub index: u32,
    pub name: String,
}

enum Raw {
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    Jpeg(Vec<u8>),
    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    Luma {
        width: u32,
        height: u32,
        stride: u32,
        rotation: u32,
        data: Vec<u8>,
    },
}

struct Shared {
    stop: AtomicBool,
    latest: Mutex<Option<Raw>>,
    ready: Condvar,
    events: mpsc::UnboundedSender<CameraEvent>,
}

impl Shared {
    fn put(&self, raw: Raw) {
        *self.latest.lock().unwrap() = Some(raw);
        self.ready.notify_one();
    }

    fn send(&self, event: CameraEvent) -> bool {
        self.events.unbounded_send(event).is_ok()
    }
}

/// A running camera; stops when dropped.
pub struct Camera {
    shared: Arc<Shared>,
}

impl Drop for Camera {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::Relaxed);
        self.shared.ready.notify_one();
        #[cfg(target_os = "android")]
        android_stop(&self.shared);
    }
}

fn start_decoder() -> (Arc<Shared>, mpsc::UnboundedReceiver<CameraEvent>) {
    let (tx, rx) = mpsc::unbounded();
    let shared = Arc::new(Shared {
        stop: AtomicBool::new(false),
        latest: Mutex::new(None),
        ready: Condvar::new(),
        events: tx,
    });
    let s = shared.clone();
    std::thread::spawn(move || {
        while !s.stop.load(Ordering::Relaxed) {
            let raw = {
                let mut latest = s.latest.lock().unwrap();
                if latest.is_none() {
                    latest = s.ready.wait_timeout(latest, Duration::from_millis(200)).unwrap().0;
                }
                latest.take()
            };
            let Some(raw) = raw else { continue };
            let (preview, texts) = process(raw);
            if let Some((width, height, rgba)) = preview
                && !s.send(CameraEvent::Preview { width, height, rgba })
            {
                return;
            }
            for text in texts {
                if !s.send(CameraEvent::Qr(text)) {
                    return;
                }
            }
        }
    });
    (shared, rx)
}

/// The preview (width, height, RGBA) and the QR texts in one frame.
fn process(raw: Raw) -> (Option<(u32, u32, Vec<u8>)>, Vec<String>) {
    match raw {
        #[cfg(target_os = "linux")]
        Raw::Jpeg(jpeg) => {
            let Ok(img) = image::load_from_memory_with_format(&jpeg, image::ImageFormat::Jpeg) else {
                return (None, vec![]);
            };
            let rgb = img.to_rgb8();
            let (w, h) = rgb.dimensions();
            let luma: Vec<u8> = rgb
                .pixels()
                .map(|p| ((p[0] as u32 * 77 + p[1] as u32 * 150 + p[2] as u32 * 29) >> 8) as u8)
                .collect();
            let texts = decode_luma(w, h, w, &luma);
            let step = w.max(h).div_ceil(PREVIEW_SIDE).max(1);
            let (pw, ph) = (w / step, h / step);
            let mut rgba = Vec::with_capacity((pw * ph * 4) as usize);
            for y in 0..ph {
                for x in 0..pw {
                    let p = rgb.get_pixel(x * step, y * step);
                    rgba.extend_from_slice(&[p[0], p[1], p[2], 255]);
                }
            }
            (Some((pw, ph, rgba)), texts)
        }
        #[cfg(not(target_os = "linux"))]
        Raw::Jpeg(_) => (None, vec![]),
        Raw::Luma {
            width,
            height,
            stride,
            rotation,
            data,
        } => {
            let texts = decode_luma(width, height, stride, &data);
            (Some(gray_preview(width, height, stride, rotation, &data)), texts)
        }
    }
}

/// A small RGBA picture of a luma plane, turned upright (`rotation` is clockwise degrees).
fn gray_preview(width: u32, height: u32, stride: u32, rotation: u32, luma: &[u8]) -> (u32, u32, Vec<u8>) {
    let step = width.max(height).div_ceil(PREVIEW_SIDE).max(1);
    let (sw, sh) = (width / step, height / step);
    let (pw, ph) = if rotation % 180 == 90 { (sh, sw) } else { (sw, sh) };
    let mut rgba = vec![255u8; (pw * ph * 4) as usize];
    for y in 0..ph {
        for x in 0..pw {
            // the source pixel that ends up at (x, y)
            let (sx, sy) = match rotation {
                90 => (y, sh - 1 - x),
                180 => (sw - 1 - x, sh - 1 - y),
                270 => (sw - 1 - y, x),
                _ => (x, y),
            };
            let v = luma
                .get((sy * step * stride + sx * step) as usize)
                .copied()
                .unwrap_or(0);
            let i = ((y * pw + x) * 4) as usize;
            rgba[i..i + 3].fill(v);
        }
    }
    (pw, ph, rgba)
}

/// QR codes in one greyscale picture. `stride` is the bytes per row, which can be wider than
/// `width`.
pub fn decode_luma(width: u32, height: u32, stride: u32, luma: &[u8]) -> Vec<String> {
    let (w, h, stride) = (width as usize, height as usize, stride as usize);
    if w == 0 || h == 0 || stride < w || luma.len() < stride * (h - 1) + w {
        return vec![];
    }
    let mut img = rqrr::PreparedImage::prepare_from_greyscale(w, h, |x, y| luma[y * stride + x]);
    img.detect_grids()
        .into_iter()
        .filter_map(|g| g.decode().ok().map(|(_, text)| text))
        .collect()
}

// ------------------------------------------------------------------------------------------
// Linux webcams

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
struct Sink(Arc<Shared>);

#[cfg(target_os = "linux")]
impl desktop_camera::CameraSink for Sink {
    fn send_frame(&self, frame: desktop_camera::Frame) -> desktop_camera::Result<()> {
        if self.0.stop.load(Ordering::Relaxed) {
            return Err(desktop_camera::CameraError::DeviceError("stopped".into()));
        }
        self.0.put(Raw::Jpeg(frame.data));
        Ok(())
    }
}

/// Starts webcam `index`.
#[cfg(target_os = "linux")]
pub async fn open(index: u32) -> Result<(Camera, mpsc::UnboundedReceiver<CameraEvent>), String> {
    let info = desktop_camera::Camera::list_devices()
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|d| d.index == index as usize)
        .ok_or("camera not found")?;
    let (shared, rx) = start_decoder();
    let camera = desktop_camera::Camera::open(&info, Sink(shared.clone())).map_err(|e| e.to_string())?;
    std::mem::forget(camera); // the capture thread ends itself once `stop` is set
    Ok((Camera { shared }, rx))
}

// ------------------------------------------------------------------------------------------
// Android: QrCamera.java streams the back camera's luma planes in

#[cfg(target_os = "android")]
static ANDROID: Mutex<Option<Arc<Shared>>> = Mutex::new(None);

#[cfg(target_os = "android")]
pub fn list_cameras() -> Result<Vec<CameraInfo>, String> {
    Ok(vec![CameraInfo {
        index: 0,
        name: "Back camera".into(),
    }])
}

/// Starts the back camera, asking for the permission first.
#[cfg(target_os = "android")]
pub async fn open(_index: u32) -> Result<(Camera, mpsc::UnboundedReceiver<CameraEvent>), String> {
    if !crate::platform::ensure_permission(crate::platform::Permission::Camera).await {
        return Err("Camera permission is needed to read QR codes.".into());
    }
    let (shared, rx) = start_decoder();
    *ANDROID.lock().unwrap() = Some(shared.clone());
    crate::platform::android::request("camera_start", &[]).await?;
    Ok((Camera { shared }, rx))
}

#[cfg(target_os = "android")]
fn android_stop(shared: &Arc<Shared>) {
    let mut current = ANDROID.lock().unwrap();
    // a newer camera may have taken over already
    if current.as_ref().is_some_and(|c| Arc::ptr_eq(c, shared)) {
        *current = None;
        crate::platform::android::send("camera_stop", &[]);
    }
}

/// Whether a frame would be used now: skips copying frames while the decoder is busy.
#[cfg(target_os = "android")]
pub(crate) fn android_wants_frame() -> bool {
    ANDROID
        .lock()
        .unwrap()
        .as_ref()
        .is_some_and(|s| s.latest.lock().unwrap().is_none())
}

#[cfg(target_os = "android")]
pub(crate) fn android_frame(width: u32, height: u32, stride: u32, rotation: u32, data: Vec<u8>) {
    if let Some(shared) = ANDROID.lock().unwrap().as_ref() {
        shared.put(Raw::Luma {
            width,
            height,
            stride,
            rotation,
            data,
        });
    }
}

#[cfg(target_os = "android")]
pub(crate) fn android_error(message: String) {
    if let Some(shared) = ANDROID.lock().unwrap().as_ref() {
        shared.send(CameraEvent::Error(message));
    }
}

// ------------------------------------------------------------------------------------------
// Elsewhere (macOS): no webcam support yet

#[cfg(not(any(target_os = "linux", target_os = "android")))]
pub fn list_cameras() -> Result<Vec<CameraInfo>, String> {
    Err("Webcam scanning is not available on this system.".into())
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
pub async fn open(_index: u32) -> Result<(Camera, mpsc::UnboundedReceiver<CameraEvent>), String> {
    let _ = start_decoder;
    Err("Webcam scanning is not available on this system.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_turns_upright() {
        // 4x2, values are x + 10 * y
        let luma: Vec<u8> = (0..2).flat_map(|y| (0..4).map(move |x| x + 10 * y)).collect();
        let (w, h, rgba) = gray_preview(4, 2, 4, 90, &luma);
        assert_eq!((w, h), (2, 4));
        let px = |x: u32, y: u32| rgba[((y * w + x) * 4) as usize];
        // turned clockwise: the bottom-left source pixel lands top-left
        assert_eq!(px(0, 0), 10);
        assert_eq!(px(1, 0), 0);
        assert_eq!(px(0, 3), 13);
    }
}
