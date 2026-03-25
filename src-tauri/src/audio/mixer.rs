use crossbeam_channel::{select, bounded, Receiver};
use ringbuf::{
    traits::{Producer, Split},
    HeapRb,
};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{info, warn};

pub const DEFAULT_SYSTEM_GAIN: f32 = 0.8;
pub const DEFAULT_MIC_GAIN: f32 = 1.0;

const RING_CAPACITY: usize = 48_000 * 5;
const TARGET_SAMPLE_RATE: u32 = 48_000;
const MAX_DEQUE_SAMPLES: usize = 48_000 * 2;
const LEVEL_EMIT_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioLevels {
    pub system_rms: f32,
    pub mic_rms: f32,
    pub mixed_rms: f32,
}

pub type MixerConsumer = ringbuf::HeapCons<f32>;

pub struct MixerConfig {
    pub system_gain: f32,
    pub mic_gain: f32,
    pub system_sample_rate: u32,
    pub mic_sample_rate: u32,
}

impl Default for MixerConfig {
    fn default() -> Self {
        Self {
            system_gain: DEFAULT_SYSTEM_GAIN,
            mic_gain: DEFAULT_MIC_GAIN,
            system_sample_rate: TARGET_SAMPLE_RATE,
            mic_sample_rate: TARGET_SAMPLE_RATE,
        }
    }
}

pub struct MixerHandle {
    running: Arc<AtomicBool>,
}

impl MixerHandle {
    pub fn stop(self) {
        self.running.store(false, Ordering::SeqCst);
        info!("Audio mixer stop signal sent");
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    pub fn running_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.running)
    }
}

/// Linear interpolation resampler from `src_rate` to `TARGET_SAMPLE_RATE`.
fn resample(input: &[f32], src_rate: u32) -> Vec<f32> {
    if src_rate == TARGET_SAMPLE_RATE || input.is_empty() {
        return input.to_vec();
    }
    let ratio = src_rate as f64 / TARGET_SAMPLE_RATE as f64;
    let out_len = ((input.len() as f64) / ratio).ceil() as usize;
    let mut output = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src_pos = i as f64 * ratio;
        let idx = src_pos as usize;
        let frac = (src_pos - idx as f64) as f32;
        let a = input.get(idx).copied().unwrap_or(0.0);
        let b = input.get(idx + 1).copied().unwrap_or(a);
        output.push(a + frac * (b - a));
    }
    output
}

/// Tanh-based soft limiter that prevents clipping while preserving dynamics.
#[inline]
fn soft_limit(sample: f32) -> f32 {
    sample.tanh()
}

/// Start the audio mixer.
///
/// Reads PCM buffers from `system_rx` (system audio) and `mic_rx` (microphone),
/// applies per-source gain, mixes with a tanh soft-limiter, and writes the result
/// into a lock-free ring buffer. Returns the consumer side of the ring buffer
/// (for the encoder) and a [`MixerHandle`] to stop the mixer.
fn compute_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
    (sum_sq / samples.len() as f32).sqrt()
}

pub fn start(
    system_rx: Receiver<Vec<f32>>,
    mic_rx: Receiver<Vec<f32>>,
    config: MixerConfig,
) -> Result<(MixerConsumer, MixerHandle, Receiver<AudioLevels>), String> {
    let rb = HeapRb::<f32>::new(RING_CAPACITY);
    let (mut prod, cons): (ringbuf::HeapProd<f32>, ringbuf::HeapCons<f32>) = rb.split();
    let (levels_tx, levels_rx) = bounded::<AudioLevels>(8);

    let running = Arc::new(AtomicBool::new(true));
    let running_thread = Arc::clone(&running);

    std::thread::Builder::new()
        .name("laconote-audio-mixer".into())
        .spawn(move || {
            info!(
                system_gain = config.system_gain,
                mic_gain = config.mic_gain,
                system_sample_rate = config.system_sample_rate,
                mic_sample_rate = config.mic_sample_rate,
                target_sample_rate = TARGET_SAMPLE_RATE,
                "Audio mixer thread started"
            );

            let mut sys_buf: VecDeque<f32> = VecDeque::new();
            let mut mic_buf: VecDeque<f32> = VecDeque::new();

            let drain_system = |sys_buf: &mut VecDeque<f32>, config: &MixerConfig| {
                while let Ok(chunk) = system_rx.try_recv() {
                    let resampled = resample(&chunk, config.system_sample_rate);
                    let available = MAX_DEQUE_SAMPLES.saturating_sub(sys_buf.len());
                    let take = resampled.len().min(available);
                    if take < resampled.len() {
                        warn!(
                            "System audio deque at capacity, dropping {} samples",
                            resampled.len() - take
                        );
                    }
                    sys_buf.extend(resampled[..take].iter().map(|&s| s * config.system_gain));
                }
            };

            let drain_mic = |mic_buf: &mut VecDeque<f32>, config: &MixerConfig| {
                while let Ok(chunk) = mic_rx.try_recv() {
                    let resampled = resample(&chunk, config.mic_sample_rate);
                    let available = MAX_DEQUE_SAMPLES.saturating_sub(mic_buf.len());
                    let take = resampled.len().min(available);
                    if take < resampled.len() {
                        warn!(
                            "Mic audio deque at capacity, dropping {} samples",
                            resampled.len() - take
                        );
                    }
                    mic_buf.extend(resampled[..take].iter().map(|&s| s * config.mic_gain));
                }
            };

            let mut last_level_emit = Instant::now();
            let mut sys_rms_acc: Vec<f32> = Vec::new();
            let mut mic_rms_acc: Vec<f32> = Vec::new();
            let mut mixed_rms_acc: Vec<f32> = Vec::new();

            while running_thread.load(Ordering::Relaxed) {
                select! {
                    recv(system_rx) -> msg => {
                        if let Ok(chunk) = msg {
                            let resampled = resample(&chunk, config.system_sample_rate);
                            let available = MAX_DEQUE_SAMPLES.saturating_sub(sys_buf.len());
                            let take = resampled.len().min(available);
                            if take < resampled.len() {
                                warn!(
                                    "System audio deque at capacity, dropping {} samples",
                                    resampled.len() - take
                                );
                            }
                            sys_buf.extend(resampled[..take].iter().map(|&s| s * config.system_gain));
                        }
                    },
                    recv(mic_rx) -> msg => {
                        if let Ok(chunk) = msg {
                            let resampled = resample(&chunk, config.mic_sample_rate);
                            let available = MAX_DEQUE_SAMPLES.saturating_sub(mic_buf.len());
                            let take = resampled.len().min(available);
                            if take < resampled.len() {
                                warn!(
                                    "Mic audio deque at capacity, dropping {} samples",
                                    resampled.len() - take
                                );
                            }
                            mic_buf.extend(resampled[..take].iter().map(|&s| s * config.mic_gain));
                        }
                    },
                    default(Duration::from_millis(5)) => {},
                }

                drain_system(&mut sys_buf, &config);
                drain_mic(&mut mic_buf, &config);

                let mix_count = sys_buf.len().max(mic_buf.len());
                if mix_count > 0 {
                    for _ in 0..mix_count {
                        let sys_sample = sys_buf.pop_front().unwrap_or(0.0);
                        let mic_sample = mic_buf.pop_front().unwrap_or(0.0);
                        let mixed = soft_limit(sys_sample + mic_sample);
                        if prod.try_push(mixed).is_err() {
                            warn!("Mixer ring buffer full, dropping sample");
                        }
                        sys_rms_acc.push(sys_sample);
                        mic_rms_acc.push(mic_sample);
                        mixed_rms_acc.push(mixed);
                    }
                }

                // Emit audio levels every ~100ms
                if last_level_emit.elapsed() >= LEVEL_EMIT_INTERVAL {
                    let levels = AudioLevels {
                        system_rms: compute_rms(&sys_rms_acc),
                        mic_rms: compute_rms(&mic_rms_acc),
                        mixed_rms: compute_rms(&mixed_rms_acc),
                    };
                    let _ = levels_tx.try_send(levels);
                    sys_rms_acc.clear();
                    mic_rms_acc.clear();
                    mixed_rms_acc.clear();
                    last_level_emit = Instant::now();
                }
            }

            info!("Audio mixer thread stopped");
        })
        .map_err(|e| e.to_string())?;

    Ok((cons, MixerHandle { running }, levels_rx))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam_channel::bounded;
    use ringbuf::traits::Consumer;
    use std::f32::consts::PI;

    #[test]
    fn test_resample_same_rate_is_passthrough() {
        let input = vec![0.1_f32, 0.2, 0.3, 0.4];
        let output = resample(&input, TARGET_SAMPLE_RATE);
        assert_eq!(output, input);
    }

    #[test]
    fn test_resample_empty_input() {
        let output = resample(&[], 24_000);
        assert!(output.is_empty());
    }

    #[test]
    fn test_resample_upsamples_from_half_rate() {
        let input = vec![0.0_f32, 1.0, 0.0, -1.0];
        let output = resample(&input, 24_000);
        assert!(
            output.len() > input.len(),
            "Upsample from 24kHz→48kHz should produce more samples, got {}",
            output.len()
        );
        assert_eq!(output.len(), 8);
    }

    #[test]
    fn test_resample_downsamples_from_double_rate() {
        let input: Vec<f32> = (0..8).map(|i| i as f32).collect();
        let output = resample(&input, 96_000);
        assert!(
            output.len() < input.len(),
            "Downsample from 96kHz→48kHz should produce fewer samples"
        );
        assert_eq!(output.len(), 4);
    }

    #[test]
    fn test_soft_limit_saturates_large_values() {
        let big_pos = soft_limit(10.0);
        assert!(big_pos <= 1.0, "soft_limit(10.0) should be <= 1.0, got {big_pos}");
        assert!(big_pos > 0.99, "soft_limit(10.0) should be close to 1.0, got {big_pos}");

        let big_neg = soft_limit(-10.0);
        assert!(big_neg >= -1.0, "soft_limit(-10.0) should be >= -1.0, got {big_neg}");
        assert!(big_neg < -0.99, "soft_limit(-10.0) should be close to -1.0, got {big_neg}");
    }

    #[test]
    fn test_soft_limit_preserves_small_values() {
        let small = soft_limit(0.1);
        assert!(
            (small - 0.1_f32.tanh()).abs() < 1e-6,
            "soft_limit(0.1) should equal tanh(0.1), got {small}"
        );
    }

    #[test]
    fn test_mixer_constant_signals() {
        let (sys_tx, sys_rx) = bounded::<Vec<f32>>(64);
        let (mic_tx, mic_rx) = bounded::<Vec<f32>>(64);

        let config = MixerConfig::default();
        let (mut cons, handle, _levels_rx) = start(sys_rx, mic_rx, config).expect("mixer start failed");

        let sys_signal = vec![0.5_f32; 100];
        let mic_signal = vec![0.3_f32; 100];

        sys_tx.send(sys_signal).unwrap();
        mic_tx.send(mic_signal).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(50));

        let mut output = Vec::new();
        let mut buf = [0.0_f32; 256];
        loop {
            let n = cons.pop_slice(&mut buf);
            if n == 0 {
                break;
            }
            output.extend_from_slice(&buf[..n]);
        }

        handle.stop();

        assert_eq!(
            output.len(),
            100,
            "Expected 100 mixed samples, got {}",
            output.len()
        );

        let expected = (0.5_f32 * DEFAULT_SYSTEM_GAIN + 0.3_f32 * DEFAULT_MIC_GAIN).tanh();
        for (i, &sample) in output.iter().enumerate() {
            assert!(
                (sample - expected).abs() < 1e-5_f32,
                "Sample {i}: expected {expected:.6}, got {sample:.6}"
            );
        }
    }

    #[test]
    fn test_mixer_sine_waves_output_matches_formula() {
        let (sys_tx, sys_rx) = bounded::<Vec<f32>>(64);
        let (mic_tx, mic_rx) = bounded::<Vec<f32>>(64);

        let config = MixerConfig::default();
        let (mut cons, handle, _levels_rx) = start(sys_rx, mic_rx, config).expect("mixer start failed");

        let sample_count = 480;
        let f1 = 440.0_f32;
        let f2 = 880.0_f32;
        let sr = TARGET_SAMPLE_RATE as f32;

        let sys_wave: Vec<f32> = (0..sample_count)
            .map(|i| (2.0 * PI * f1 * i as f32 / sr).sin())
            .collect();
        let mic_wave: Vec<f32> = (0..sample_count)
            .map(|i| (2.0 * PI * f2 * i as f32 / sr).sin())
            .collect();

        sys_tx.send(sys_wave.clone()).unwrap();
        mic_tx.send(mic_wave.clone()).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(50));

        let mut output = Vec::new();
        let mut buf = [0.0_f32; 1024];
        loop {
            let n = cons.pop_slice(&mut buf);
            if n == 0 {
                break;
            }
            output.extend_from_slice(&buf[..n]);
        }

        handle.stop();

        assert!(
            !output.is_empty(),
            "Mixer should produce output samples from two sine waves"
        );
        assert_eq!(
            output.len(),
            sample_count,
            "Output length should match input length"
        );

        for (i, &sample) in output.iter().enumerate() {
            let sys_contrib = sys_wave[i] * DEFAULT_SYSTEM_GAIN;
            let mic_contrib = mic_wave[i] * DEFAULT_MIC_GAIN;
            let expected = (sys_contrib + mic_contrib).tanh();
            assert!(
                (sample - expected).abs() < 1e-5_f32,
                "Sample {i}: expected {expected:.6} (sys={sys_contrib:.4} + mic={mic_contrib:.4}), got {sample:.6}"
            );
        }
    }

    #[test]
    fn test_mixer_stop_is_clean() {
        let (_sys_tx, sys_rx) = bounded::<Vec<f32>>(4);
        let (_mic_tx, mic_rx) = bounded::<Vec<f32>>(4);

        let (_, handle, _levels_rx) = start(sys_rx, mic_rx, MixerConfig::default()).expect("mixer start failed");
        assert!(handle.is_running());
        handle.stop();
    }

    #[test]
    fn test_mixer_mic_only_no_system_audio() {
        let (_sys_tx, sys_rx) = bounded::<Vec<f32>>(64);
        let (mic_tx, mic_rx) = bounded::<Vec<f32>>(64);

        let config = MixerConfig::default();
        let (mut cons, handle, _levels_rx) = start(sys_rx, mic_rx, config).expect("mixer start failed");

        let mic_signal = vec![0.5_f32; 200];
        mic_tx.send(mic_signal).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(50));

        let mut output = Vec::new();
        let mut buf = [0.0_f32; 512];
        loop {
            let n = cons.pop_slice(&mut buf);
            if n == 0 {
                break;
            }
            output.extend_from_slice(&buf[..n]);
        }

        handle.stop();

        assert_eq!(
            output.len(),
            200,
            "Mic-only recording should produce 200 samples even without system audio, got {}",
            output.len()
        );

        let expected = (0.5_f32 * DEFAULT_MIC_GAIN).tanh();
        for (i, &sample) in output.iter().enumerate() {
            assert!(
                (sample - expected).abs() < 1e-5_f32,
                "Sample {i}: expected {expected:.6} (mic only, sys=silence), got {sample:.6}"
            );
        }
    }
}
