//! Windows system audio capture via WASAPI Loopback.
//!
//! Mirrors the interface of `cat_tap.rs` (macOS) so the rest of the pipeline
//! (mixer → encoder → chunker → uploader) works identically on both platforms.

use crossbeam_channel::{bounded, Receiver, Sender};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use tracing::{error, info, warn};
use wasapi::{self, Device, Direction, ShareMode};

use super::util::mix_to_mono;

/// Same error type as CaTap's CaptureError for a uniform pipeline interface.
#[derive(Debug)]
pub enum CaptureError {
    StreamError(String),
}

impl std::fmt::Display for CaptureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CaptureError::StreamError(s) => write!(f, "WASAPI stream error: {s}"),
        }
    }
}

impl std::error::Error for CaptureError {}

/// Handle to control the WASAPI loopback capture thread.
pub struct WasapiHandle {
    running: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

unsafe impl Send for WasapiHandle {}
unsafe impl Sync for WasapiHandle {}

impl WasapiHandle {
    pub fn stop(mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
        info!("WASAPI loopback capture stopped");
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    pub fn running_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.running)
    }
}

impl Drop for WasapiHandle {
    fn drop(&mut self) {
        if self.running.load(Ordering::Relaxed) {
            error!("WasapiHandle dropped without calling stop() — stopping now");
            self.running.store(false, Ordering::SeqCst);
        }
    }
}

/// Start WASAPI loopback capture on the default render endpoint.
///
/// Returns `(audio_rx, handle, sample_rate)` — same signature as `cat_tap::start()`.
/// Audio is delivered as mono f32 PCM via the crossbeam channel.
pub fn start() -> Result<(Receiver<Vec<f32>>, WasapiHandle, u32), CaptureError> {
    // Initialize COM for this function scope (thread will init its own)
    let _ = wasapi::initialize_mta();

    // Get the default render device for loopback capture
    let device = get_default_device(&Direction::Render)
        .map_err(|e| CaptureError::StreamError(format!("Failed to get default render device: {e}")))?;

    let device_name = device.get_friendlyname()
        .unwrap_or_else(|_| "Unknown Device".to_string());
    info!(device = %device_name, "WASAPI loopback: using default render device");

    // Query the mix format (what the audio engine uses)
    let mix_format = device.get_mixformat()
        .map_err(|e| CaptureError::StreamError(format!("Failed to get mix format: {e}")))?;

    let sample_rate = mix_format.get_samplespersec();
    let channels = mix_format.get_nchannels() as usize;
    let bits_per_sample = mix_format.get_bitspersample();

    info!(
        sample_rate,
        channels,
        bits_per_sample,
        "WASAPI loopback format"
    );

    let (audio_tx, audio_rx) = bounded::<Vec<f32>>(256);
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = Arc::clone(&running);

    let thread = thread::Builder::new()
        .name("laconote-wasapi-loopback".into())
        .spawn(move || {
            if let Err(e) = capture_loop(device, running_clone, audio_tx, channels) {
                error!("WASAPI capture loop exited with error: {e}");
            }
        })
        .map_err(|e| CaptureError::StreamError(format!("Failed to spawn capture thread: {e}")))?;

    Ok((
        audio_rx,
        WasapiHandle {
            running,
            thread: Some(thread),
        },
        sample_rate,
    ))
}

/// Main capture loop running in a dedicated thread.
fn capture_loop(
    device: Device,
    running: Arc<AtomicBool>,
    tx: Sender<Vec<f32>>,
    channels: usize,
) -> Result<(), String> {
    // Initialize COM on this thread
    wasapi::initialize_mta().map_err(|e| format!("COM init failed: {e}"))?;

    // Create the audio client in loopback mode
    let audio_client = device.get_iaudioclient()
        .map_err(|e| format!("Failed to get IAudioClient: {e}"))?;

    let mix_format = device.get_mixformat()
        .map_err(|e| format!("Failed to get mix format: {e}"))?;

    // Initialize in shared mode with loopback flag
    // Buffer duration: 200ms (in 100-nanosecond units)
    let buffer_duration = 2_000_000i64;

    audio_client
        .initialize_client(
            &mix_format,
            buffer_duration,
            &Direction::Capture,
            &ShareMode::Shared,
            true, // loopback
        )
        .map_err(|e| format!("Failed to initialize loopback client: {e}"))?;

    let capture_client = audio_client.get_audiocaptureclient()
        .map_err(|e| format!("Failed to get capture client: {e}"))?;

    let handle = audio_client.start_stream()
        .map_err(|e| format!("Failed to start loopback stream: {e}"))?;

    info!("WASAPI loopback stream started");

    // Capture loop: poll for available frames
    while running.load(Ordering::Relaxed) {
        // Sleep briefly to avoid busy-waiting; WASAPI delivers data in ~10ms chunks
        thread::sleep(std::time::Duration::from_millis(5));

        match capture_client.read_bytes_from_device() {
            Ok(frames) => {
                if frames.is_empty() {
                    continue;
                }
                // Convert raw bytes to f32 samples
                let f32_samples = bytes_to_f32(&frames);
                if f32_samples.is_empty() {
                    continue;
                }
                // Mix down to mono
                let mono = mix_to_mono(&f32_samples, channels);
                if let Err(e) = tx.try_send(mono) {
                    warn!("WASAPI audio channel full, dropping buffer: {e}");
                }
            }
            Err(e) => {
                // If the device was disconnected or an error occurred
                if !running.load(Ordering::Relaxed) {
                    break;
                }
                warn!("WASAPI read error: {e}");
                thread::sleep(std::time::Duration::from_millis(50));
            }
        }
    }

    // Stop the stream
    drop(handle);
    info!("WASAPI loopback capture loop ended");
    Ok(())
}

/// Convert raw byte buffer from WASAPI to f32 samples.
/// WASAPI typically delivers 32-bit float in shared mode.
fn bytes_to_f32(bytes: &[u8]) -> Vec<f32> {
    if bytes.len() % 4 != 0 {
        warn!("WASAPI buffer size {} not aligned to 4 bytes", bytes.len());
        return Vec::new();
    }
    bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}
