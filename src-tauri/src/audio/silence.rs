use tracing::debug;

/// RMS amplitude threshold below which audio is considered silence.
/// Equivalent to 500/32768 ≈ 1.5% of full-scale (f32 range −1.0..1.0).
pub const SILENCE_THRESHOLD: f32 = 500.0 / 32768.0;

const SAMPLE_RATE: u64 = 48_000;

const FIVE_MIN_SAMPLES: u64 = 5 * 60 * SAMPLE_RATE;
const NINE_MIN_SAMPLES: u64 = 9 * 60 * SAMPLE_RATE;
const TEN_MIN_SAMPLES: u64 = 10 * 60 * SAMPLE_RATE;

const SILENCE_FOR_SPLIT_5_9_MIN_SAMPLES: u64 = 5 * SAMPLE_RATE; // 5 s
const SILENCE_FOR_SPLIT_9_10_MIN_SAMPLES: u64 = 2 * SAMPLE_RATE; // 2 s

/// Decision returned by [`SilenceDetector::process`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDecision {
    /// Continue recording; no split needed yet.
    Continue,
    /// A chunk split should be triggered now.
    ShouldSplit,
}

/// Adaptive silence-based chunk splitter.
///
/// Tracks elapsed audio (by sample count) and the duration of consecutive silence.
/// Returns [`SplitDecision::ShouldSplit`] according to this schedule:
///
/// | Elapsed          | Split on silence |
/// |------------------|-----------------|
/// | < 5 min          | never            |
/// | 5 – 9 min        | 5 s of silence   |
/// | 9 – 10 min       | 2 s of silence   |
/// | ≥ 10 min         | immediately      |
pub struct SilenceDetector {
    total_samples: u64,
    silence_samples: u64,
}

impl SilenceDetector {
    pub fn new() -> Self {
        Self {
            total_samples: 0,
            silence_samples: 0,
        }
    }

    /// Feed a PCM buffer into the detector.
    ///
    /// Returns [`SplitDecision::ShouldSplit`] when the current policy dictates a split.
    /// Callers should reset the detector (via [`Self::reset`]) after acting on a split.
    pub fn process(&mut self, buffer: &[f32]) -> SplitDecision {
        let n = buffer.len() as u64;
        self.total_samples += n;

        let rms = compute_rms(buffer);
        let is_silent = rms < SILENCE_THRESHOLD;

        if is_silent {
            self.silence_samples += n;
        } else {
            self.silence_samples = 0;
        }

        debug!(
            elapsed_secs = self.elapsed_secs(),
            rms,
            silence_secs = self.silence_secs(),
            "SilenceDetector::process"
        );

        self.evaluate()
    }

    fn evaluate(&self) -> SplitDecision {
        if self.total_samples >= TEN_MIN_SAMPLES {
            return SplitDecision::ShouldSplit;
        }
        if self.total_samples >= NINE_MIN_SAMPLES {
            if self.silence_samples >= SILENCE_FOR_SPLIT_9_10_MIN_SAMPLES {
                return SplitDecision::ShouldSplit;
            }
        } else if self.total_samples >= FIVE_MIN_SAMPLES {
            if self.silence_samples >= SILENCE_FOR_SPLIT_5_9_MIN_SAMPLES {
                return SplitDecision::ShouldSplit;
            }
        }
        SplitDecision::Continue
    }

    /// Reset internal counters after a split has been performed.
    pub fn reset(&mut self) {
        self.total_samples = 0;
        self.silence_samples = 0;
    }

    /// Elapsed recording time in seconds (based on sample count).
    pub fn elapsed_secs(&self) -> f64 {
        self.total_samples as f64 / SAMPLE_RATE as f64
    }

    /// Duration of the current consecutive silence run in seconds.
    pub fn silence_secs(&self) -> f64 {
        self.silence_samples as f64 / SAMPLE_RATE as f64
    }
}

impl Default for SilenceDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute the root-mean-square amplitude of `buf`.
pub fn compute_rms(buf: &[f32]) -> f32 {
    if buf.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = buf.iter().map(|s| s * s).sum();
    (sum_sq / buf.len() as f32).sqrt()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME: usize = 960; // 20 ms at 48 kHz

    fn silence_buf() -> Vec<f32> {
        vec![0.0_f32; FRAME]
    }

    fn speech_buf() -> Vec<f32> {
        (0..FRAME)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 48_000.0).sin() * 0.5)
            .collect()
    }

    fn feed_secs(det: &mut SilenceDetector, is_silent: bool, secs: f64) -> SplitDecision {
        let frames = (secs * 48_000.0 / FRAME as f64) as usize;
        let mut decision = SplitDecision::Continue;
        for _ in 0..frames {
            let buf = if is_silent { silence_buf() } else { speech_buf() };
            decision = det.process(&buf);
        }
        decision
    }

    #[test]
    fn test_no_split_before_5_min() {
        let mut det = SilenceDetector::new();
        let result = feed_secs(&mut det, true, 4.9 * 60.0);
        assert_eq!(result, SplitDecision::Continue);
    }

    #[test]
    fn test_split_at_10_min_regardless_of_silence() {
        let mut det = SilenceDetector::new();
        feed_secs(&mut det, false, 9.99 * 60.0);
        let result = feed_secs(&mut det, false, 0.02 * 60.0);
        assert_eq!(result, SplitDecision::ShouldSplit);
    }

    #[test]
    fn test_split_on_5s_silence_between_5_and_9_min() {
        let mut det = SilenceDetector::new();
        feed_secs(&mut det, false, 5.1 * 60.0); // past 5-min mark with speech
        let result = feed_secs(&mut det, true, 5.1); // 5+ seconds of silence
        assert_eq!(result, SplitDecision::ShouldSplit);
    }

    #[test]
    fn test_no_split_on_short_silence_between_5_and_9_min() {
        let mut det = SilenceDetector::new();
        feed_secs(&mut det, false, 5.1 * 60.0);
        let result = feed_secs(&mut det, true, 4.0); // only 4 s silence — not enough
        assert_eq!(result, SplitDecision::Continue);
    }

    #[test]
    fn test_split_on_2s_silence_between_9_and_10_min() {
        let mut det = SilenceDetector::new();
        feed_secs(&mut det, false, 9.1 * 60.0);
        let result = feed_secs(&mut det, true, 2.1);
        assert_eq!(result, SplitDecision::ShouldSplit);
    }

    #[test]
    fn test_no_split_on_1s_silence_between_9_and_10_min() {
        let mut det = SilenceDetector::new();
        feed_secs(&mut det, false, 9.1 * 60.0);
        let result = feed_secs(&mut det, true, 1.0);
        assert_eq!(result, SplitDecision::Continue);
    }

    #[test]
    fn test_speech_resets_silence_counter() {
        let mut det = SilenceDetector::new();
        feed_secs(&mut det, false, 5.1 * 60.0);
        feed_secs(&mut det, true, 4.0); // 4 s silence (not enough)
        let result = feed_secs(&mut det, false, 0.5); // speech resets counter
        assert_eq!(result, SplitDecision::Continue);
        assert!(det.silence_secs() < 1.0, "Silence counter should reset on speech");
    }

    #[test]
    fn test_reset_clears_all_counters() {
        let mut det = SilenceDetector::new();
        feed_secs(&mut det, false, 6.0 * 60.0);
        det.reset();
        assert_eq!(det.elapsed_secs(), 0.0);
        assert_eq!(det.silence_secs(), 0.0);
        let result = feed_secs(&mut det, true, 5.5); // would trigger at 5+ min but now we reset
        assert_eq!(result, SplitDecision::Continue);
    }

    #[test]
    fn test_compute_rms_silence_is_zero() {
        let buf = vec![0.0_f32; 480];
        assert_eq!(compute_rms(&buf), 0.0);
    }

    #[test]
    fn test_compute_rms_above_threshold() {
        let buf = speech_buf();
        let rms = compute_rms(&buf);
        assert!(
            rms > SILENCE_THRESHOLD,
            "speech RMS {rms:.6} should exceed SILENCE_THRESHOLD {SILENCE_THRESHOLD:.6}"
        );
    }

    #[test]
    fn test_compute_rms_empty_is_zero() {
        assert_eq!(compute_rms(&[]), 0.0);
    }
}
