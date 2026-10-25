//! Data over sound. The same frames that are shown as an animated QR code are played as a loop of
//! ggwave chirps by one device and picked up by the microphone of the other, in any order and
//! with repeats, so a [`qr_transport::Assembler`] reassembles them exactly as it does for QR.
//!
//! ggwave carries at most ~140 bytes per chirp, so the payload is re-split into smaller frames
//! and sent in the compact binary form of [`qr_transport::Frame`] (8 byte header + 100 bytes).
//!
//! The ggwave C library keeps its instances in unsynchronised global state, so every call into it
//! goes through [`Codec`], which serialises them.

use std::mem::ManuallyDrop;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat, SizedSample, StreamConfig};
use ggwave_rs::{GgWave, ProtocolId, SampleFormat as GgFormat};

/// Characters of base32 per chirp: 160 chars = 100 bytes, a multiple of 5 so each chunk decodes
/// on its own (see `qr_transport::split`).
const CHUNK_CHARS: usize = 160;
const PROTOCOL: ProtocolId = ProtocolId::GGWAVE_PROTOCOL_AUDIBLE_FAST;
const VOLUME: i32 = 60;
/// Silence after each chirp, so the receiver's decoder sees the end of the message.
const GAP_SECONDS: f32 = 0.4;
/// ggwave's internal frame; the decoder is fed whole multiples of it.
const SAMPLES_PER_FRAME: usize = 1024;

/// cpal's Android backend reads the `JavaVM` and the application `Context` from `ndk_context`,
/// which a Flutter app never fills in (the library is `dlopen`ed by Dart, so `JNI_OnLoad` does not
/// run) and panics without. The activity hands them over through a native method, see the
/// `Java_…_initAudio` exports of the app crates.
#[cfg(target_os = "android")]
pub mod android {
    use std::ffi::c_void;
    use std::sync::Once;

    // Indices into the JNIEnv function table (jni.h).
    const NEW_GLOBAL_REF: usize = 21;
    const GET_JAVA_VM: usize = 219;

    /// # Safety
    /// `env` must be the `JNIEnv*` of the calling thread and `context` a live `android.content.Context`.
    pub unsafe fn init(env: *mut c_void, context: *mut c_void) {
        static ONCE: Once = Once::new();
        ONCE.call_once(|| unsafe {
            let table = *(env as *const *const *const c_void);
            let new_global_ref: unsafe extern "system" fn(*mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(*table.add(NEW_GLOBAL_REF));
            let get_java_vm: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> i32 =
                std::mem::transmute(*table.add(GET_JAVA_VM));
            let mut vm = std::ptr::null_mut();
            if get_java_vm(env, &mut vm) == 0 {
                // the global reference is kept for the life of the process
                ndk_context::initialize_android_context(vm, new_global_ref(env, context));
            }
        });
    }
}

static GGWAVE: Mutex<()> = Mutex::new(());

struct Codec(ManuallyDrop<GgWave>);

impl Codec {
    fn new(tx: bool, rate: u32) -> Result<Codec, String> {
        let _g = GGWAVE.lock().unwrap_or_else(|e| e.into_inner());
        // ggwave prints every decode to stderr unless told otherwise
        unsafe { ggwave_rs::ffi::ggwave_setLogFile(std::ptr::null_mut()) };
        let mut p = ggwave_rs::default_parameters();
        p.sampleFormatInp = GgFormat::GGWAVE_SAMPLE_FORMAT_F32;
        p.sampleFormatOut = GgFormat::GGWAVE_SAMPLE_FORMAT_F32;
        p.sampleRateInp = rate as f32;
        p.sampleRateOut = rate as f32;
        p.operatingMode = if tx { ggwave_rs::GGWAVE_OPERATING_MODE_TX } else { ggwave_rs::GGWAVE_OPERATING_MODE_RX };
        let wave = GgWave::new(p).map_err(|e| e.to_string())?;
        Ok(Codec(ManuallyDrop::new(wave)))
    }

    fn encode(&self, packet: &[u8]) -> Result<Vec<f32>, String> {
        let _g = GGWAVE.lock().unwrap_or_else(|e| e.into_inner());
        let bytes = self.0.encode(packet, PROTOCOL, VOLUME).map_err(|e| e.to_string())?;
        Ok(bytes.chunks_exact(4).map(|b| f32::from_ne_bytes([b[0], b[1], b[2], b[3]])).collect())
    }

    fn decode(&self, samples: &[f32]) -> Option<Vec<u8>> {
        let bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_ne_bytes()).collect();
        let _g = GGWAVE.lock().unwrap_or_else(|e| e.into_inner());
        self.0.decode(&bytes).ok().flatten()
    }
}

impl Drop for Codec {
    fn drop(&mut self) {
        let _g = GGWAVE.lock().unwrap_or_else(|e| e.into_inner());
        unsafe { ManuallyDrop::drop(&mut self.0) };
    }
}

/// Turn the `MF1:` frames of one message into the binary chirp payloads to transmit.
fn packets(frames: &[String]) -> Result<Vec<Vec<u8>>, String> {
    let mut asm = qr_transport::Assembler::new();
    let mut payload = None;
    for f in frames {
        let p = asm.push(f).map_err(|e| e.to_string())?;
        if p.complete.is_some() {
            payload = p.complete;
        }
    }
    let payload = payload.ok_or("incomplete message, nothing to play")?;
    qr_transport::split(&payload, CHUNK_CHARS)
        .iter()
        .map(|t| qr_transport::Frame::parse(t).map(|f| f.to_bytes()).ok_or_else(|| "bad frame".to_string()))
        .collect()
}

/// One loop of the whole message as mono samples, each chirp followed by a gap.
fn waveform(frames: &[String], rate: u32) -> Result<Vec<f32>, String> {
    let codec = Codec::new(true, rate)?;
    let gap = (rate as f32 * GAP_SECONDS) as usize;
    let mut out = Vec::new();
    for p in packets(frames)? {
        out.extend(codec.encode(&p)?);
        out.resize(out.len() + gap, 0.0);
    }
    Ok(out)
}

// ------------------------------------------------------------------------------------------
// Audio devices

struct Format {
    rate: u32,
    channels: u16,
    format: SampleFormat,
}

impl Format {
    fn config(&self) -> StreamConfig {
        StreamConfig { channels: self.channels, sample_rate: self.rate, buffer_size: cpal::BufferSize::Default }
    }
}

/// Android's AAudio converts whatever we ask for, and asking skips a JNI device query; elsewhere
/// the device's own default avoids unsupported combinations.
#[cfg(target_os = "android")]
fn pick_format(_device: &cpal::Device, _input: bool) -> Result<Format, String> {
    Ok(Format { rate: 48000, channels: 1, format: SampleFormat::F32 })
}

#[cfg(not(target_os = "android"))]
fn pick_format(device: &cpal::Device, input: bool) -> Result<Format, String> {
    let c = if input { device.default_input_config() } else { device.default_output_config() }
        .map_err(|e| e.to_string())?;
    Ok(Format { rate: c.sample_rate(), channels: c.channels(), format: c.sample_format() })
}

fn out_stream<T: SizedSample + FromSample<f32>>(
    device: &cpal::Device,
    fmt: &Format,
    wave: Vec<f32>,
) -> Result<cpal::Stream, String> {
    let channels = fmt.channels as usize;
    let mut pos = 0usize;
    device
        .build_output_stream(
            fmt.config(),
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                for frame in data.chunks_mut(channels) {
                    let v = T::from_sample(wave[pos]);
                    pos = (pos + 1) % wave.len();
                    frame.fill(v);
                }
            },
            |e| eprintln!("sound output error: {e}"),
            None,
        )
        .map_err(|e| e.to_string())
}

fn in_stream<T: SizedSample>(
    device: &cpal::Device,
    fmt: &Format,
    send: mpsc::Sender<Vec<f32>>,
) -> Result<cpal::Stream, String>
where
    f32: FromSample<T>,
{
    let channels = fmt.channels as usize;
    device
        .build_input_stream(
            fmt.config(),
            move |data: &[T], _: &cpal::InputCallbackInfo| {
                // keep the first channel only
                let mono = data.chunks(channels).map(|f| f32::from_sample(f[0])).collect();
                let _ = send.send(mono);
            },
            |e| eprintln!("sound input error: {e}"),
            None,
        )
        .map_err(|e| e.to_string())
}

/// Runs `body` on its own thread (cpal streams and ggwave instances stay on the thread that made
/// them). `setup` errors come back to the caller; afterwards the thread lives until `stop`.
struct Worker {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Worker {
    fn spawn<F>(body: F) -> Result<Worker, String>
    where
        F: FnOnce(&AtomicBool, mpsc::Sender<Result<(), String>>) + Send + 'static,
    {
        let stop = Arc::new(AtomicBool::new(false));
        let (ready_tx, ready_rx) = mpsc::channel();
        let thread = {
            let stop = stop.clone();
            std::thread::spawn(move || body(&stop, ready_tx))
        };
        let w = Worker { stop, thread: Some(thread) };
        match ready_rx.recv() {
            Ok(Ok(())) => Ok(w),
            Ok(Err(e)) => Err(e),
            Err(_) => Err("audio thread failed".into()),
        }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// Plays a message on a loop through the default speaker until dropped.
pub struct Player(#[allow(dead_code)] Worker);

impl Player {
    /// `frames`: the complete set of `MF1:` frames of one message (what a QR code would show).
    pub fn start(frames: &[String]) -> Result<Player, String> {
        let frames = frames.to_vec();
        packets(&frames)?; // validate before touching the audio device
        let w = Worker::spawn(move |stop, ready| {
            let setup = || -> Result<cpal::Stream, String> {
                let device = cpal::default_host().default_output_device().ok_or("no speaker found")?;
                let fmt = pick_format(&device, false)?;
                let wave = waveform(&frames, fmt.rate)?;
                let stream = match fmt.format {
                    SampleFormat::F32 => out_stream::<f32>(&device, &fmt, wave),
                    SampleFormat::I16 => out_stream::<i16>(&device, &fmt, wave),
                    SampleFormat::I32 => out_stream::<i32>(&device, &fmt, wave),
                    SampleFormat::U16 => out_stream::<u16>(&device, &fmt, wave),
                    other => Err(format!("unsupported speaker sample format {other}")),
                }?;
                stream.play().map_err(|e| e.to_string())?;
                Ok(stream)
            };
            match setup() {
                Ok(stream) => {
                    let _ = ready.send(Ok(()));
                    while !stop.load(Ordering::Relaxed) {
                        std::thread::sleep(Duration::from_millis(50));
                    }
                    drop(stream);
                }
                Err(e) => {
                    let _ = ready.send(Err(e));
                }
            }
        })?;
        Ok(Player(w))
    }
}

/// Listens on the default microphone until dropped, calling `on_frame` with the `MF1:` text of
/// every frame heard (feed them to a `qr_transport::Assembler`).
pub struct Listener(#[allow(dead_code)] Worker);

impl Listener {
    pub fn start(on_frame: impl Fn(String) + Send + 'static) -> Result<Listener, String> {
        let w = Worker::spawn(move |stop, ready| {
            let (tx, rx) = mpsc::channel();
            let setup = || -> Result<(cpal::Stream, Codec), String> {
                let device = cpal::default_host().default_input_device().ok_or("no microphone found")?;
                let fmt = pick_format(&device, true)?;
                let codec = Codec::new(false, fmt.rate)?;
                let stream = match fmt.format {
                    SampleFormat::F32 => in_stream::<f32>(&device, &fmt, tx),
                    SampleFormat::I16 => in_stream::<i16>(&device, &fmt, tx),
                    SampleFormat::I32 => in_stream::<i32>(&device, &fmt, tx),
                    SampleFormat::U16 => in_stream::<u16>(&device, &fmt, tx),
                    other => Err(format!("unsupported microphone sample format {other}")),
                }?;
                stream.play().map_err(|e| e.to_string())?;
                Ok((stream, codec))
            };
            let (stream, codec) = match setup() {
                Ok(x) => x,
                Err(e) => {
                    let _ = ready.send(Err(e));
                    return;
                }
            };
            let _ = ready.send(Ok(()));
            let mut pending: Vec<f32> = Vec::new();
            while !stop.load(Ordering::Relaxed) {
                match rx.recv_timeout(Duration::from_millis(100)) {
                    Ok(chunk) => pending.extend(chunk),
                    Err(mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
                let whole = pending.len() / SAMPLES_PER_FRAME * SAMPLES_PER_FRAME;
                for block in pending[..whole].chunks(SAMPLES_PER_FRAME) {
                    if let Some(frame) = codec.decode(block).and_then(|b| qr_transport::Frame::from_bytes(&b)) {
                        on_frame(frame.to_text());
                    }
                }
                pending.drain(..whole);
            }
            drop(stream);
        })?;
        Ok(Listener(w))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_survives_the_waveform() {
        let rate = 48000;
        let payload: Vec<u8> = (0..260).map(|i| (i * 7 + 3) as u8).collect();
        let frames = qr_transport::split(&payload, qr_transport::DEFAULT_FRAME_CHARS);
        let wave = waveform(&frames, rate).unwrap();

        // play it back through the decoder in microphone-sized pieces, with a little noise-free
        // lead-in, the way Listener does
        let codec = Codec::new(false, rate).unwrap();
        let mut asm = qr_transport::Assembler::new();
        let mut done = None;
        let mut heard = 0;
        let mut samples = vec![0.0; 5000];
        samples.extend_from_slice(&wave);
        samples.extend(std::iter::repeat(0.0).take(SAMPLES_PER_FRAME * 4));
        for block in samples.chunks_exact(SAMPLES_PER_FRAME) {
            if let Some(f) = codec.decode(block).and_then(|b| qr_transport::Frame::from_bytes(&b)) {
                heard += 1;
                if let Some(p) = asm.push(&f.to_text()).unwrap().complete {
                    done = Some(p);
                }
            }
        }
        assert_eq!(done.as_deref(), Some(&payload[..]), "heard {heard} chirps");
    }

    #[test]
    fn incomplete_messages_are_refused() {
        let frames = qr_transport::split(&[1u8; 500], 64);
        assert!(packets(&frames[..2]).is_err());
        assert!(Player::start(&frames[..2]).is_err());
    }
}
