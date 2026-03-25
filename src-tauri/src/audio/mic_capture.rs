use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::{bounded, Receiver, Sender};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{error, info, warn};

use super::util::mix_to_mono;

#[derive(Debug)]
pub enum MicError {
    NoDevice(String),
    StreamBuild(String),
    ThreadError(String),
}

impl std::fmt::Display for MicError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MicError::NoDevice(s) => write!(f, "Microphone device error: {s}"),
            MicError::StreamBuild(s) => write!(f, "Failed to build mic stream: {s}"),
            MicError::ThreadError(s) => write!(f, "Mic capture thread error: {s}"),
        }
    }
}

impl std::error::Error for MicError {}

/// Returns the names of all available audio input devices.
pub fn list_devices() -> Vec<String> {
    let host = cpal::default_host();
    match host.input_devices() {
        Ok(devices) => devices
            .filter_map(|d| d.name().ok())
            .collect(),
        Err(e) => {
            warn!("Failed to enumerate input devices: {e}");
            Vec::new()
        }
    }
}

/// Opaque handle to an active microphone capture session.
pub struct MicCaptureHandle {
    running: Arc<AtomicBool>,
    stop_tx: std::sync::mpsc::SyncSender<()>,
}

impl MicCaptureHandle {
    /// Stop microphone capture and release resources.
    pub fn stop(self) {
        self.running.store(false, Ordering::SeqCst);
        let _ = self.stop_tx.send(());
        info!("Microphone capture stop signal sent");
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    pub fn running_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.running)
    }
}

/// Start capturing audio from the microphone.
///
/// If `device_name` is `None`, the system default input device is used.
/// Returns a channel [`Receiver`] of `Vec<f32>` PCM samples (48 kHz, mono, f32)
/// and a [`MicCaptureHandle`] to stop the session.
pub fn start(device_name: Option<String>) -> Result<(Receiver<Vec<f32>>, MicCaptureHandle), MicError> {
    let (audio_tx, audio_rx) = bounded::<Vec<f32>>(256);
    let (stop_tx, stop_rx) = std::sync::mpsc::sync_channel::<()>(1);
    let (init_tx, init_rx) = std::sync::mpsc::channel::<Result<(), MicError>>();

    let running = Arc::new(AtomicBool::new(true));
    let running_for_thread = Arc::clone(&running);

    std::thread::Builder::new()
        .name("laconote-mic-audio".into())
        .spawn(move || {
            let host = cpal::default_host();

            let device = match select_device(&host, device_name.as_deref()) {
                Ok(d) => d,
                Err(e) => {
                    let _ = init_tx.send(Err(e));
                    return;
                }
            };

            let device_name_str = device.name().unwrap_or_else(|_| "unknown".into());

            let config = match build_stream_config(&device) {
                Ok(c) => c,
                Err(e) => {
                    let _ = init_tx.send(Err(e));
                    return;
                }
            };

            info!(
                device = %device_name_str,
                sample_rate = config.sample_rate.0,
                channels = config.channels,
                "Building microphone input stream"
            );

            let channels = config.channels as usize;
            let running_for_cb = Arc::clone(&running_for_thread);
            let tx_for_cb: Sender<Vec<f32>> = audio_tx;

            let stream = device.build_input_stream(
                &config,
                move |data: &[f32], _info: &cpal::InputCallbackInfo| {
                    if !running_for_cb.load(Ordering::Relaxed) {
                        return;
                    }
                    let mono = mix_to_mono(data, channels);
                    if !mono.is_empty() {
                        if let Err(e) = tx_for_cb.try_send(mono) {
                            warn!("Mic channel full, dropping buffer: {e}");
                        }
                    }
                },
                move |err| {
                    error!("Microphone stream error: {err}");
                },
                None,
            );

            let stream = match stream {
                Ok(s) => s,
                Err(e) => {
                    let _ = init_tx.send(Err(MicError::StreamBuild(e.to_string())));
                    return;
                }
            };

            if let Err(e) = stream.play() {
                let _ = init_tx.send(Err(MicError::StreamBuild(e.to_string())));
                return;
            }

            info!("Microphone capture started (device: {device_name_str})");
            let _ = init_tx.send(Ok(()));

            let _ = stop_rx.recv();
            running_for_thread.store(false, Ordering::SeqCst);

            drop(stream);
            info!("Microphone capture stopped");
        })
        .map_err(|e| MicError::ThreadError(e.to_string()))?;

    match init_rx.recv() {
        Ok(Ok(())) => Ok((audio_rx, MicCaptureHandle { running, stop_tx })),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(MicError::ThreadError(
            "Mic capture thread exited before initialization".into(),
        )),
    }
}

fn select_device(host: &cpal::Host, device_name: Option<&str>) -> Result<cpal::Device, MicError> {
    match device_name {
        None => host
            .default_input_device()
            .ok_or_else(|| MicError::NoDevice("No default input device found".into())),
        Some(name) => {
            let devices = host
                .input_devices()
                .map_err(|e| MicError::NoDevice(e.to_string()))?;
            devices
                .filter_map(|d| {
                    let n = d.name().ok()?;
                    if n == name { Some(d) } else { None }
                })
                .next()
                .ok_or_else(|| MicError::NoDevice(format!("Input device '{name}' not found")))
        }
    }
}

fn build_stream_config(device: &cpal::Device) -> Result<cpal::StreamConfig, MicError> {
    let supported = device
        .supported_input_configs()
        .map_err(|e| MicError::StreamBuild(e.to_string()))?;

    let target_rate = cpal::SampleRate(48_000);

    let best = supported
        .filter(|c| c.sample_format() == cpal::SampleFormat::F32)
        .find(|c| c.min_sample_rate() <= target_rate && target_rate <= c.max_sample_rate());

    if let Some(range) = best {
        return Ok(range.with_sample_rate(target_rate).into());
    }

    device
        .default_input_config()
        .map(|c| c.into())
        .map_err(|e| MicError::StreamBuild(format!("No suitable input config: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_list_devices_does_not_crash() {
        let devices = list_devices();
        println!("Available input devices: {devices:?}");
    }

    #[test]
    fn test_mix_to_mono_single_channel() {
        let data = vec![0.1, 0.5, -0.3];
        let result = mix_to_mono(&data, 1);
        assert_eq!(result, data);
    }

    #[test]
    fn test_mix_to_mono_stereo() {
        let data = vec![0.2_f32, 0.4, -0.2, 0.6];
        let result = mix_to_mono(&data, 2);
        assert_eq!(result.len(), 2);
        assert!((result[0] - 0.3).abs() < 1e-6);
        assert!((result[1] - 0.2).abs() < 1e-6);
    }

    #[test]
    #[ignore = "requires a physical microphone input device"]
    fn test_mic_capture_starts_and_stops() {
        let (rx, handle) = start(None).expect("Failed to start mic capture");

        let mut received_buffers = 0usize;
        let deadline = std::time::Instant::now() + Duration::from_secs(3);

        while std::time::Instant::now() < deadline {
            if let Ok(pcm) = rx.recv_timeout(Duration::from_millis(100)) {
                assert!(!pcm.is_empty(), "received empty PCM buffer");
                received_buffers += 1;
            }
        }

        handle.stop();

        assert!(
            received_buffers > 0,
            "Expected non-empty PCM buffers from mic, got 0"
        );
    }
}
