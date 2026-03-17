use crate::audio::{encode_to_webm, SilenceDetector, SplitDecision, FRAME_SAMPLES};
use crate::upload::offline::OfflineQueue;
use crate::upload::uploader::{ChunkRequest, SpeakerSegment, Uploader};
use chrono::{DateTime, Utc};
use ringbuf::traits::Consumer;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

/// Configuration for a recording session's chunk pipeline.
pub struct ChunkPipelineConfig {
    pub meeting_id: String,
    pub meeting_name: Option<String>,
    pub meeting_type: Option<String>,
    pub project_id: Option<String>,
    pub meeting_start_time: DateTime<Utc>,
    pub uploader: Uploader,
    pub offline_queue: OfflineQueue,
    /// Channel for sending user-visible error messages (upload failures, auth errors).
    pub notify_tx: tokio::sync::mpsc::Sender<String>,
}

/// Internal chunk data sent from the PCM accumulator to the upload task.
struct PendingChunk {
    pcm: Vec<f32>,
    captured_at: DateTime<Utc>,
    is_final: bool,
}

/// Handle to a running chunk pipeline. Call [`stop`] to flush and shut down.
pub struct ChunkPipelineHandle {
    stop_tx: crossbeam_channel::Sender<()>,
    pub chunks_uploaded: Arc<AtomicU32>,
    pub duration_samples: Arc<AtomicU64>,
}

impl ChunkPipelineHandle {
    /// Signal the pipeline to stop. The final chunk is sent with `is_final=true`.
    pub fn stop(&self) {
        let _ = self.stop_tx.try_send(());
    }

    pub fn chunks_uploaded(&self) -> u32 {
        self.chunks_uploaded.load(Ordering::Relaxed)
    }

    pub fn duration_seconds(&self) -> f64 {
        self.duration_samples.load(Ordering::Relaxed) as f64 / 48_000.0
    }
}

/// Start the encode → chunk → upload pipeline.
///
/// Spawns two tasks:
/// 1. A blocking thread that polls the ring buffer, accumulates PCM frames, and
///    applies silence-based chunking.
/// 2. An async task that receives raw PCM chunks, encodes them to WebM/Opus, and
///    uploads them (falling back to the offline queue on network failure).
pub async fn start(
    consumer: ringbuf::HeapCons<f32>,
    config: ChunkPipelineConfig,
) -> ChunkPipelineHandle {
    let (stop_tx, stop_rx) = crossbeam_channel::bounded::<()>(1);
    let (chunk_tx, chunk_rx) = mpsc::unbounded_channel::<PendingChunk>();

    let chunks_uploaded = Arc::new(AtomicU32::new(0));
    let duration_samples = Arc::new(AtomicU64::new(0));

    let chunks_uploaded_clone = Arc::clone(&chunks_uploaded);
    let duration_clone = Arc::clone(&duration_samples);

    // Clone session metadata for the upload task
    let meeting_id = config.meeting_id.clone();
    let meeting_name = config.meeting_name.clone();
    let meeting_type = config.meeting_type.clone();
    let project_id = config.project_id.clone();
    let meeting_start_time = config.meeting_start_time;
    let uploader = config.uploader;
    let offline_queue = config.offline_queue;
    let notify_tx = config.notify_tx;

    // ── Blocking thread: PCM accumulation + silence detection ────────────────
    tokio::task::spawn_blocking(move || {
        poll_ring_buffer(consumer, stop_rx, chunk_tx, duration_clone);
    });

    // ── Async task: encode + upload ──────────────────────────────────────────
    tokio::spawn(async move {
        upload_loop(
            chunk_rx,
            meeting_id,
            meeting_name,
            meeting_type,
            project_id,
            meeting_start_time,
            uploader,
            offline_queue,
            chunks_uploaded_clone,
            notify_tx,
        )
        .await;
    });

    ChunkPipelineHandle {
        stop_tx,
        chunks_uploaded,
        duration_samples,
    }
}

// ── Ring-buffer polling (runs in spawn_blocking) ──────────────────────────────

fn poll_ring_buffer(
    mut consumer: ringbuf::HeapCons<f32>,
    stop_rx: crossbeam_channel::Receiver<()>,
    chunk_tx: mpsc::UnboundedSender<PendingChunk>,
    duration_samples: Arc<AtomicU64>,
) {
    let mut frame_buffer: Vec<f32> = Vec::with_capacity(FRAME_SAMPLES * 2);
    let mut chunk_pcm: Vec<f32> = Vec::new();
    let mut silence_detector = SilenceDetector::new();
    let mut chunk_start_time = Utc::now();
    let mut read_buf = vec![0f32; FRAME_SAMPLES];

    info!("Chunk pipeline polling thread started");

    loop {
        let should_stop = stop_rx.try_recv().is_ok();

        // Drain available samples from the ring buffer
        let n = consumer.pop_slice(&mut read_buf);
        if n > 0 {
            frame_buffer.extend_from_slice(&read_buf[..n]);
            duration_samples.fetch_add(n as u64, Ordering::Relaxed);
        }

        // Process all complete FRAME_SAMPLES frames in frame_buffer
        while frame_buffer.len() >= FRAME_SAMPLES {
            let frame: Vec<f32> = frame_buffer[..FRAME_SAMPLES].to_vec();
            frame_buffer.drain(..FRAME_SAMPLES);
            chunk_pcm.extend_from_slice(&frame);

            let decision = silence_detector.process(&frame);

            if decision == SplitDecision::ShouldSplit {
                let pcm = std::mem::take(&mut chunk_pcm);
                let captured_at = chunk_start_time;
                chunk_start_time = Utc::now();
                silence_detector.reset();

                info!(
                    duration_secs = pcm.len() as f64 / 48_000.0,
                    "Silence-based chunk split"
                );

                if chunk_tx
                    .send(PendingChunk {
                        pcm,
                        captured_at,
                        is_final: false,
                    })
                    .is_err()
                {
                    warn!("Upload task dropped channel – stopping PCM loop");
                    return;
                }
            }
        }

        // On stop: flush partial frame + remaining chunk_pcm as final chunk
        if should_stop {
            // Include any incomplete frame
            chunk_pcm.extend_from_slice(&frame_buffer);

            if !chunk_pcm.is_empty() {
                let pcm = std::mem::take(&mut chunk_pcm);
                info!(
                    duration_secs = pcm.len() as f64 / 48_000.0,
                    "Flushing final chunk on stop"
                );
                let _ = chunk_tx.send(PendingChunk {
                    pcm,
                    captured_at: chunk_start_time,
                    is_final: true,
                });
            }
            break;
        }

        // Sleep briefly when the ring buffer is empty to avoid busy-spinning
        if n == 0 {
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    info!("Chunk pipeline polling thread stopped");
}

// ── Async upload loop ─────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
async fn upload_loop(
    mut chunk_rx: mpsc::UnboundedReceiver<PendingChunk>,
    meeting_id: String,
    meeting_name: Option<String>,
    meeting_type: Option<String>,
    project_id: Option<String>,
    meeting_start_time: DateTime<Utc>,
    uploader: Uploader,
    mut offline_queue: OfflineQueue,
    chunks_uploaded: Arc<AtomicU32>,
    notify_tx: tokio::sync::mpsc::Sender<String>,
) {
    info!("Chunk upload loop started");

    while let Some(pending) = chunk_rx.recv().await {
        let duration_secs = pending.pcm.len() as f64 / 48_000.0;

        // Encode PCM → WebM/Opus
        let audio_data = match encode_to_webm(&pending.pcm) {
            Ok(bytes) => bytes,
            Err(e) => {
                error!(error = %e, "Failed to encode PCM to WebM – skipping chunk");
                continue;
            }
        };

        // Build a default single-speaker segment covering the whole chunk
        let speakers = vec![SpeakerSegment {
            speaker: "Speaker 1".to_string(),
            start: 0.0,
            end: duration_secs,
        }];

        let req = ChunkRequest {
            audio_data: audio_data.clone(),
            meeting_id: meeting_id.clone(),
            meeting_name: meeting_name.clone(),
            meeting_type: meeting_type.clone(),
            project_id: project_id.clone(),
            captured_at: pending.captured_at,
            meeting_start_time: Some(meeting_start_time),
            speakers,
            is_final: pending.is_final,
        };

        match uploader.upload_chunk(&req).await {
            Ok(result) => {
                info!(
                    meeting_id = %meeting_id,
                    is_final = pending.is_final,
                    api_meeting_id = ?result.meeting_id,
                    "Chunk uploaded"
                );
                chunks_uploaded.fetch_add(1, Ordering::Relaxed);
            }
            Err(e) => {
                warn!(
                    error = %e,
                    is_final = pending.is_final,
                    "Upload failed after retries – saving to offline queue"
                );
                let notify_msg = if e.contains("HTTP 401") || e.contains("401 Unauthorized") {
                    "Session expired — please log in again to resume uploads.".to_string()
                } else {
                    "Upload failed — chunk saved locally and will retry when connected."
                        .to_string()
                };
                let _ = notify_tx.try_send(notify_msg);
                if let Err(save_err) = offline_queue.enqueue(&req).await {
                    error!(error = %save_err, "Failed to save chunk to offline queue");
                }
            }
        }
    }

    info!("Chunk upload loop finished");
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::SAMPLE_RATE;

    fn sine_pcm(duration_secs: f32) -> Vec<f32> {
        let samples = (SAMPLE_RATE as f32 * duration_secs) as usize;
        (0..samples)
            .map(|i| {
                (2.0 * std::f32::consts::PI * 440.0 * i as f32 / SAMPLE_RATE as f32).sin() * 0.5
            })
            .collect()
    }

    #[test]
    fn test_handle_stop_signals_correctly() {
        let (stop_tx, _stop_rx) = crossbeam_channel::bounded(1);
        let handle = ChunkPipelineHandle {
            stop_tx,
            chunks_uploaded: Arc::new(AtomicU32::new(5)),
            duration_samples: Arc::new(AtomicU64::new(48_000 * 60)),
        };

        assert_eq!(handle.chunks_uploaded(), 5);
        assert!((handle.duration_seconds() - 60.0).abs() < 0.01);
        handle.stop(); // should not panic
    }

    #[test]
    fn test_pcm_poll_splits_on_silence_at_5_min() {
        // Simulate 5+ minutes of speech then 5+ seconds of silence
        // This verifies the polling logic sends a chunk via the channel

        let (stop_tx, stop_rx) = crossbeam_channel::bounded(1);
        let (chunk_tx, mut chunk_rx) = mpsc::unbounded_channel::<PendingChunk>();
        let duration_samples = Arc::new(AtomicU64::new(0));

        let rb = ringbuf::HeapRb::<f32>::new(48_000 * 15 * 60);
        let (mut prod, cons) = {
            use ringbuf::traits::Split;
            rb.split()
        };

        // 5.1 min of speech (sine wave > threshold) then 5.1 s silence
        let speech = sine_pcm(5.1 * 60.0);
        let silence = vec![0.0f32; (5.1 * 48_000.0) as usize];

        for &s in &speech {
            use ringbuf::traits::Producer;
            let _ = prod.try_push(s);
        }
        for &s in &silence {
            use ringbuf::traits::Producer;
            let _ = prod.try_push(s);
        }
        // Stop signal after data
        drop(prod);
        let _ = stop_tx.try_send(());

        // Run poll loop in a thread (it will run until stop signal)
        let handle = std::thread::spawn(move || {
            poll_ring_buffer(cons, stop_rx, chunk_tx, duration_samples);
        });

        handle.join().unwrap();

        // Should have received at least one chunk (the silence split + final flush)
        let mut received = Vec::new();
        while let Ok(chunk) = chunk_rx.try_recv() {
            received.push(chunk);
        }

        assert!(
            !received.is_empty(),
            "Expected at least one chunk from the pipeline"
        );

        // The last chunk should be marked final
        let last = received.last().unwrap();
        assert!(last.is_final, "Last chunk must be final");
    }
}
