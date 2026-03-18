use crossbeam_channel::{bounded, Receiver, RecvTimeoutError};
use serde::{Deserialize, Serialize};
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};

#[derive(Debug)]
pub enum CaptureError {
    StreamError(String),
}

impl std::fmt::Display for CaptureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CaptureError::StreamError(s) => write!(f, "Stream error: {s}"),
        }
    }
}

impl std::error::Error for CaptureError {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CaptureReadiness {
    Ready,
    NotReady,
    Unknown,
    AuthorizedButSilent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureProbeResult {
    pub state: CaptureReadiness,
    pub detail: String,
}

type AudioCb = extern "C" fn(*const f32, i32, i32, *mut c_void);

extern "C" {
    fn catap_start(callback: AudioCb, user_data: *mut c_void) -> i32;
    fn catap_stop();
    fn catap_is_available() -> i32;
    fn catap_macos_version(major: *mut u64, minor: *mut u64, patch: *mut u64);
    fn catap_probe_permission() -> i32;
}

pub fn is_available() -> bool {
    unsafe { catap_is_available() != 0 }
}

pub fn probe_permission() -> bool {
    unsafe { catap_probe_permission() != 0 }
}

pub fn macos_version() -> (u64, u64, u64) {
    let mut major: u64 = 0;
    let mut minor: u64 = 0;
    let mut patch: u64 = 0;
    unsafe { catap_macos_version(&mut major, &mut minor, &mut patch) };
    (major, minor, patch)
}

#[derive(Debug, Clone, PartialEq)]
pub enum CaTapCompatibility {
    Supported,
    Unsupported,
    Unknown,
}

pub fn check_catap_compatibility() -> CaTapCompatibility {
    let (major, minor, _) = macos_version();
    match major {
        0..=13 => CaTapCompatibility::Unsupported,
        14 if minor < 2 => CaTapCompatibility::Unsupported,
        14 | 15 => CaTapCompatibility::Supported,
        _ => CaTapCompatibility::Unknown,
    }
}

struct CallbackData {
    sender: crossbeam_channel::Sender<Vec<f32>>,
    running: Arc<AtomicBool>,
}

extern "C" fn audio_callback(
    samples: *const f32,
    count: i32,
    channels: i32,
    user_data: *mut c_void,
) {
    if samples.is_null() || count <= 0 || user_data.is_null() {
        return;
    }
    let cb = unsafe { &*(user_data as *const CallbackData) };
    if !cb.running.load(Ordering::Relaxed) {
        return;
    }
    let raw = unsafe { std::slice::from_raw_parts(samples, count as usize) };
    let mono = mix_to_mono(raw, channels.max(1) as usize);
    if let Err(e) = cb.sender.try_send(mono) {
        warn!("CATap audio channel full, dropping buffer: {e}");
    }
}

fn mix_to_mono(data: &[f32], channels: usize) -> Vec<f32> {
    if channels <= 1 {
        return data.to_vec();
    }
    let frames = data.len() / channels;
    let mut mono = Vec::with_capacity(frames);
    for frame in 0..frames {
        let sum: f32 = (0..channels).map(|ch| data[frame * channels + ch]).sum();
        mono.push(sum / channels as f32);
    }
    mono
}

pub struct CaTapHandle {
    running: Arc<AtomicBool>,
    cb_data_ptr: *mut CallbackData,
}

unsafe impl Send for CaTapHandle {}
unsafe impl Sync for CaTapHandle {}

impl CaTapHandle {
    pub fn stop(self) {
        self.running.store(false, Ordering::SeqCst);
        unsafe {
            catap_stop();
            if !self.cb_data_ptr.is_null() {
                drop(Box::from_raw(self.cb_data_ptr));
            }
        }
        info!("CATap system audio capture stopped");
        std::mem::forget(self);
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    pub fn running_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.running)
    }
}

impl Drop for CaTapHandle {
    fn drop(&mut self) {
        if !self.cb_data_ptr.is_null() {
            error!("CaTapHandle dropped without calling stop() — leaking resources");
        }
    }
}

pub fn start() -> Result<(Receiver<Vec<f32>>, CaTapHandle), CaptureError> {
    if !is_available() {
        return Err(CaptureError::StreamError(
            "CATap requires macOS 14.2 or later".into(),
        ));
    }

    let (audio_tx, audio_rx) = bounded::<Vec<f32>>(256);
    let running = Arc::new(AtomicBool::new(true));

    let cb_data = Box::new(CallbackData {
        sender: audio_tx,
        running: Arc::clone(&running),
    });
    let cb_data_ptr = Box::into_raw(cb_data);

    let status = unsafe { catap_start(audio_callback, cb_data_ptr as *mut c_void) };

    if status != 0 {
        unsafe {
            drop(Box::from_raw(cb_data_ptr));
        }
        return Err(CaptureError::StreamError(format!(
            "CATap AudioDeviceCreateIOProcID/Start failed with OSStatus {status}"
        )));
    }

    info!("CATap system audio capture started (global stereo tap, mixed to mono)");

    Ok((audio_rx, CaTapHandle { running, cb_data_ptr }))
}

pub fn probe_capture_readiness(timeout: Duration) -> CaptureProbeResult {
    match start() {
        Ok((audio_rx, handle)) => {
            let result = match audio_rx.recv_timeout(timeout) {
                Ok(samples) if !samples.is_empty() => CaptureProbeResult {
                    state: CaptureReadiness::Ready,
                    detail: "CATap started and delivered audio frames during the readiness probe".into(),
                },
                Ok(_) => CaptureProbeResult {
                    state: CaptureReadiness::Unknown,
                    detail: "CATap started, but the readiness probe only observed an empty audio frame".into(),
                },
                Err(RecvTimeoutError::Timeout) => CaptureProbeResult {
                    state: CaptureReadiness::AuthorizedButSilent,
                    detail: "CATap started successfully, but no audio frames arrived before the readiness probe timed out (authorized but silent)".into(),
                },
                Err(RecvTimeoutError::Disconnected) => CaptureProbeResult {
                    state: CaptureReadiness::NotReady,
                    detail: "CATap started, but the audio callback channel disconnected during the readiness probe".into(),
                },
            };

            handle.stop();
            result
        }
        Err(error) => CaptureProbeResult {
            state: CaptureReadiness::NotReady,
            detail: error.to_string(),
        },
    }
}
