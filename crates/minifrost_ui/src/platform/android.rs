//! The bridge to the Java side (`dev.minifrost.MinifrostActivity`, shared by both apps).
//!
//! Rust asks with `activity.request(op, id, data)` and Java answers, on its own time, through the
//! static native `Native.result(id, data, error)`. The activity also reports the window insets and
//! camera frames through `Native`. The natives live in that one shared class, so their symbols are
//! the same in both apps.

use std::{
    collections::HashMap,
    future::Future,
    sync::{
        Mutex,
        OnceLock,
        atomic::{
            AtomicI64,
            Ordering,
        },
    },
};

use freya::prelude::*;
use futures_channel::{
    mpsc,
    oneshot,
};
use futures_util::StreamExt;
use jni::{
    EnvUnowned,
    JValue,
    jni_sig,
    jni_str,
    objects::{
        JByteArray,
        JClass,
        JObject,
        JString,
    },
};
use winit::platform::android::activity::AndroidApp;

use super::Insets;

static APP: OnceLock<AndroidApp> = OnceLock::new();
static NEXT_ID: AtomicI64 = AtomicI64::new(1);
static PENDING: Mutex<Option<HashMap<i64, oneshot::Sender<Result<Vec<u8>, String>>>>> = Mutex::new(None);
static INSETS: Mutex<(Option<Insets>, Option<mpsc::UnboundedSender<Insets>>)> = Mutex::new((None, None));

/// Keeps the app handle for calls into Java; call first thing in `android_main`.
pub fn init(app: AndroidApp) {
    let _ = APP.set(app);
}

/// The app's internal storage (`Context.getFilesDir()`), where the Flutter builds kept their data.
pub fn files_dir() -> std::path::PathBuf {
    APP.get()
        .and_then(|app| app.internal_data_path())
        .expect("android_main must call platform::android::init first")
}

fn call_request(op: &str, id: i64, data: &[u8]) -> jni::errors::Result<()> {
    let Some(app) = APP.get() else {
        return Err(jni::errors::Error::NullPtr("android app"));
    };
    let vm = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr() as *mut _) };
    vm.attach_current_thread(|env| -> jni::errors::Result<()> {
        let activity = unsafe { JObject::from_raw(env, app.activity_as_ptr() as *mut _) };
        let op = env.new_string(op)?;
        let data = env.byte_array_from_slice(data)?;
        env.call_method(
            &activity,
            jni_str!("request"),
            jni_sig!("(Ljava/lang/String;J[B)V"),
            &[JValue::Object(&op), JValue::Long(id), JValue::Object(&data)],
        )?;
        Ok(())
    })
}

/// Asks the activity to do `op`; resolves with what it answers. Errors are `code\nmessage` or a
/// message alone.
pub fn request(op: &str, data: &[u8]) -> impl Future<Output = Result<Vec<u8>, String>> + use<> {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let (tx, rx) = oneshot::channel();
    PENDING.lock().unwrap().get_or_insert_default().insert(id, tx);
    let sent = call_request(op, id, data);
    if let Err(err) = &sent {
        PENDING.lock().unwrap().get_or_insert_default().remove(&id);
        tracing::error!("android request {op}: {err}");
    }
    let failed = sent.err().map(|e| e.to_string());
    async move {
        if let Some(err) = failed {
            return Err(err);
        }
        rx.await.unwrap_or_else(|_| Err("the activity went away".into()))
    }
}

/// Asks the activity to do `op` without waiting for it.
pub fn send(op: &str, data: &[u8]) {
    if let Err(err) = call_request(op, 0, data) {
        tracing::error!("android request {op}: {err}");
    }
}

/// Feeds the insets the activity reports into `state`, for the life of the app.
pub(crate) fn watch_insets(mut state: State<Insets>) {
    let (tx, mut rx) = mpsc::unbounded();
    {
        let mut slot = INSETS.lock().unwrap();
        if let Some(insets) = slot.0 {
            state.set(insets);
        }
        slot.1 = Some(tx);
    }
    spawn_in_window(async move {
        while let Some(insets) = rx.next().await {
            state.set_if_modified(insets);
        }
    });
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_minifrost_Native_result(
    mut unowned_env: EnvUnowned<'_>,
    _class: JClass<'_>,
    id: i64,
    data: JByteArray<'_>,
    error: JString<'_>,
) {
    let outcome = unowned_env.with_env(|env| -> jni::errors::Result<()> {
        let result = if error.is_null() {
            Ok(if data.is_null() { vec![] } else { env.convert_byte_array(&data)? })
        } else {
            Err(error.try_to_string(env)?)
        };
        let tx = PENDING.lock().unwrap().get_or_insert_default().remove(&id);
        if let Some(tx) = tx {
            let _ = tx.send(result);
        }
        Ok(())
    });
    outcome.resolve::<jni::errors::ThrowRuntimeExAndDefault>();
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_minifrost_Native_insets(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
) {
    let insets = Insets {
        left,
        top,
        right,
        bottom,
    };
    let mut slot = INSETS.lock().unwrap();
    slot.0 = Some(insets);
    if let Some(tx) = &slot.1 {
        let _ = tx.unbounded_send(insets);
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_minifrost_Native_cameraFrame(
    mut unowned_env: EnvUnowned<'_>,
    _class: JClass<'_>,
    luma: JByteArray<'_>,
    width: i32,
    height: i32,
    row_stride: i32,
    rotation: i32,
) {
    let outcome = unowned_env.with_env(|env| -> jni::errors::Result<()> {
        if crate::camera::android_wants_frame() {
            let data = env.convert_byte_array(&luma)?;
            crate::camera::android_frame(width as u32, height as u32, row_stride as u32, rotation as u32, data);
        }
        Ok(())
    });
    outcome.resolve::<jni::errors::ThrowRuntimeExAndDefault>();
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_minifrost_Native_cameraError(
    mut unowned_env: EnvUnowned<'_>,
    _class: JClass<'_>,
    message: JString<'_>,
) {
    let outcome = unowned_env.with_env(|env| -> jni::errors::Result<()> {
        crate::camera::android_error(message.try_to_string(env)?);
        Ok(())
    });
    outcome.resolve::<jni::errors::ThrowRuntimeExAndDefault>();
}
