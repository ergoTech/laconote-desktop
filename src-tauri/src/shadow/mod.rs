pub mod buffer;
pub mod cleanup;
pub mod save;
pub mod schedule;
pub mod vad;

use crate::audio::{start_mic_capture, MicCaptureHandle};
use buffer::ShadowBuffer;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use tracing::{info, warn};
use vad::ShadowVAD;

const DEFAULT_BUFFER_MINUTES: u32 = 20;

#[derive(Debug, Clone)]
pub enum ShadowState {
    Inactive,
    Active {
        buffered_duration_ms: u64,
        buffer_capacity_ms: u64,
        total_meeting_duration_ms: u64,
    },
    Saving {
        progress: f32,
    },
    RecordingAndUploading {
        buffered_duration_ms: u64,
        upload_progress: f32,
        total_meeting_duration_ms: u64,
    },
    Error {
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowStatus {
    pub state: String,
    pub buffered_duration_ms: u64,
    pub buffer_capacity_ms: u64,
    pub fill_percent: u8,
    pub total_meeting_duration_ms: u64,
    pub upload_progress: Option<f32>,
    pub error_message: Option<String>,
    pub speech_ratio: Option<f32>,
}

pub struct ShadowSession {
    mic_handle: MicCaptureHandle,
    worker_handle: Option<JoinHandle<()>>,
    worker_running: Arc<AtomicBool>,
    pub buffer: Arc<ShadowBuffer>,
    pub speech_ratio: Arc<std::sync::Mutex<f32>>,
}

impl ShadowSession {
    pub fn start(
        mic_device: Option<String>,
        buffer_minutes: Option<u32>,
    ) -> Result<Self, String> {
        let minutes = buffer_minutes.unwrap_or(DEFAULT_BUFFER_MINUTES);
        let buffer = Arc::new(ShadowBuffer::new(minutes));
        let (mic_rx, mic_handle, _mic_sr) =
            start_mic_capture(mic_device).map_err(|e| format!("Mic capture failed: {e}"))?;

        let worker_running = Arc::new(AtomicBool::new(true));
        let running = Arc::clone(&worker_running);
        let buf = Arc::clone(&buffer);
        let speech_ratio = Arc::new(std::sync::Mutex::new(0.0_f32));
        let ratio_ref = Arc::clone(&speech_ratio);

        let worker_handle = std::thread::Builder::new()
            .name("laconote-shadow-worker".into())
            .spawn(move || {
                let mut vad = ShadowVAD::new();
                while running.load(Ordering::Relaxed) {
                    match mic_rx.recv_timeout(std::time::Duration::from_millis(100)) {
                        Ok(samples) => {
                            buf.write(&samples);
                            vad.process(&samples);
                            if let Ok(mut r) = ratio_ref.lock() {
                                *r = vad.speech_ratio();
                            }
                        }
                        Err(crossbeam_channel::RecvTimeoutError::Timeout) => continue,
                        Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                            warn!("Shadow worker: mic channel disconnected");
                            break;
                        }
                    }
                }
                info!("Shadow worker thread exiting");
            })
            .map_err(|e| format!("Failed to spawn shadow worker: {e}"))?;

        info!(buffer_minutes = minutes, "Shadow recording started");

        Ok(Self {
            mic_handle,
            worker_handle: Some(worker_handle),
            worker_running,
            buffer,
            speech_ratio,
        })
    }

    pub fn status(&self) -> ShadowStatus {
        let buffered_duration_ms = self.buffer.duration_ms();
        let buffer_capacity_ms = self.buffer.capacity_ms();
        let total_meeting_duration_ms = self.buffer.total_written_duration_ms();
        let fill_percent = self.buffer.fill_percent();
        let ratio = self.speech_ratio.lock().map(|r| *r).unwrap_or(0.0);

        ShadowStatus {
            state: "active".into(),
            buffered_duration_ms,
            buffer_capacity_ms,
            fill_percent,
            total_meeting_duration_ms,
            upload_progress: None,
            error_message: None,
            speech_ratio: Some(ratio),
        }
    }

    pub fn stop(mut self) {
        info!("Stopping shadow recording");
        self.worker_running.store(false, Ordering::SeqCst);
        self.mic_handle.stop();
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }
        info!("Shadow recording stopped");
    }

    pub fn stop_and_drain(mut self) -> Vec<f32> {
        info!("Stopping shadow recording and draining buffer");
        self.worker_running.store(false, Ordering::SeqCst);
        self.mic_handle.stop();
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }
        let pcm = self.buffer.drain();
        info!(samples = pcm.len(), "Shadow buffer drained");
        pcm
    }
}

impl From<&ShadowState> for ShadowStatus {
    fn from(state: &ShadowState) -> Self {
        match state {
            ShadowState::Inactive => ShadowStatus {
                state: "inactive".into(),
                buffered_duration_ms: 0,
                buffer_capacity_ms: 0,
                fill_percent: 0,
                total_meeting_duration_ms: 0,
                upload_progress: None,
                error_message: None,
                speech_ratio: None,
            },
            ShadowState::Active {
                buffered_duration_ms,
                buffer_capacity_ms,
                total_meeting_duration_ms,
            } => ShadowStatus {
                state: "active".into(),
                buffered_duration_ms: *buffered_duration_ms,
                buffer_capacity_ms: *buffer_capacity_ms,
                fill_percent: if *buffer_capacity_ms > 0 {
                    ((*buffered_duration_ms * 100) / *buffer_capacity_ms) as u8
                } else {
                    0
                },
                total_meeting_duration_ms: *total_meeting_duration_ms,
                upload_progress: None,
                error_message: None,
                speech_ratio: None,
            },
            ShadowState::Saving { progress } => ShadowStatus {
                state: "saving".into(),
                buffered_duration_ms: 0,
                buffer_capacity_ms: 0,
                fill_percent: 0,
                total_meeting_duration_ms: 0,
                upload_progress: Some(*progress),
                error_message: None,
                speech_ratio: None,
            },
            ShadowState::RecordingAndUploading {
                buffered_duration_ms,
                upload_progress,
                total_meeting_duration_ms,
            } => ShadowStatus {
                state: "uploading".into(),
                buffered_duration_ms: *buffered_duration_ms,
                buffer_capacity_ms: 0,
                fill_percent: 0,
                total_meeting_duration_ms: *total_meeting_duration_ms,
                upload_progress: Some(*upload_progress),
                error_message: None,
                speech_ratio: None,
            },
            ShadowState::Error { message } => ShadowStatus {
                state: "error".into(),
                buffered_duration_ms: 0,
                buffer_capacity_ms: 0,
                fill_percent: 0,
                total_meeting_duration_ms: 0,
                upload_progress: None,
                error_message: Some(message.clone()),
                speech_ratio: None,
            },
        }
    }
}
