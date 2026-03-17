use crate::audio::silence::{compute_rms, SILENCE_THRESHOLD};

const SAMPLE_RATE: u64 = 48_000;

const SPEECH_ONSET_SAMPLES: u64 = (SAMPLE_RATE * 100) / 1000;
const SILENCE_ONSET_SAMPLES: u64 = (SAMPLE_RATE * 500) / 1000;

pub struct ShadowVAD {
    is_speech: bool,
    consecutive_speech_samples: u64,
    consecutive_silence_samples: u64,
    total_frames: u64,
    speech_frames: u64,
}

impl ShadowVAD {
    pub fn new() -> Self {
        Self {
            is_speech: false,
            consecutive_speech_samples: 0,
            consecutive_silence_samples: 0,
            total_frames: 0,
            speech_frames: 0,
        }
    }

    pub fn process(&mut self, buffer: &[f32]) {
        if buffer.is_empty() {
            return;
        }

        let rms = compute_rms(buffer);
        let frame_is_speech = rms >= SILENCE_THRESHOLD;
        let n = buffer.len() as u64;

        self.total_frames += 1;
        if frame_is_speech {
            self.speech_frames += 1;
        }

        if frame_is_speech {
            self.consecutive_speech_samples += n;
            self.consecutive_silence_samples = 0;

            if !self.is_speech && self.consecutive_speech_samples >= SPEECH_ONSET_SAMPLES {
                self.is_speech = true;
            }
        } else {
            self.consecutive_silence_samples += n;
            self.consecutive_speech_samples = 0;

            if self.is_speech && self.consecutive_silence_samples >= SILENCE_ONSET_SAMPLES {
                self.is_speech = false;
            }
        }
    }

    pub fn is_speech(&self) -> bool {
        self.is_speech
    }

    pub fn speech_ratio(&self) -> f32 {
        if self.total_frames == 0 {
            return 0.0;
        }
        self.speech_frames as f32 / self.total_frames as f32
    }

    pub fn reset(&mut self) {
        self.is_speech = false;
        self.consecutive_speech_samples = 0;
        self.consecutive_silence_samples = 0;
        self.total_frames = 0;
        self.speech_frames = 0;
    }
}

impl Default for ShadowVAD {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME: usize = 960;

    fn silence_buf() -> Vec<f32> {
        vec![0.0_f32; FRAME]
    }

    fn speech_buf() -> Vec<f32> {
        (0..FRAME)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 48_000.0).sin() * 0.5)
            .collect()
    }

    fn feed_ms(vad: &mut ShadowVAD, speech: bool, ms: u64) {
        let total_samples = (SAMPLE_RATE * ms) / 1000;
        let mut fed = 0u64;
        while fed < total_samples {
            let buf = if speech { speech_buf() } else { silence_buf() };
            let n = buf.len().min((total_samples - fed) as usize);
            vad.process(&buf[..n]);
            fed += n as u64;
        }
    }

    #[test]
    fn test_new_vad_is_silence() {
        let vad = ShadowVAD::new();
        assert!(!vad.is_speech());
        assert_eq!(vad.speech_ratio(), 0.0);
    }

    #[test]
    fn test_silence_stays_silent() {
        let mut vad = ShadowVAD::new();
        feed_ms(&mut vad, false, 1000);
        assert!(!vad.is_speech());
    }

    #[test]
    fn test_speech_onset_hysteresis() {
        let mut vad = ShadowVAD::new();
        feed_ms(&mut vad, true, 50);
        assert!(!vad.is_speech(), "should not switch after only 50ms speech");

        feed_ms(&mut vad, true, 60);
        assert!(vad.is_speech(), "should switch after 110ms total speech");
    }

    #[test]
    fn test_silence_onset_hysteresis() {
        let mut vad = ShadowVAD::new();
        feed_ms(&mut vad, true, 150);
        assert!(vad.is_speech());

        feed_ms(&mut vad, false, 300);
        assert!(vad.is_speech(), "should not switch after only 300ms silence");

        feed_ms(&mut vad, false, 250);
        assert!(!vad.is_speech(), "should switch after 550ms total silence");
    }

    #[test]
    fn test_speech_interrupts_silence_onset() {
        let mut vad = ShadowVAD::new();
        feed_ms(&mut vad, true, 150);
        assert!(vad.is_speech());

        feed_ms(&mut vad, false, 400);
        assert!(vad.is_speech(), "still speech after 400ms silence");

        feed_ms(&mut vad, true, 20);
        feed_ms(&mut vad, false, 400);
        assert!(vad.is_speech(), "silence counter reset by speech burst");
    }

    #[test]
    fn test_silence_interrupts_speech_onset() {
        let mut vad = ShadowVAD::new();
        feed_ms(&mut vad, true, 80);
        assert!(!vad.is_speech());

        feed_ms(&mut vad, false, 20);
        feed_ms(&mut vad, true, 80);
        assert!(!vad.is_speech(), "speech counter reset by silence");

        feed_ms(&mut vad, true, 30);
        assert!(vad.is_speech(), "now crosses 100ms consecutive speech");
    }

    #[test]
    fn test_speech_ratio_all_speech() {
        let mut vad = ShadowVAD::new();
        feed_ms(&mut vad, true, 500);
        assert!((vad.speech_ratio() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_speech_ratio_all_silence() {
        let mut vad = ShadowVAD::new();
        feed_ms(&mut vad, false, 500);
        assert_eq!(vad.speech_ratio(), 0.0);
    }

    #[test]
    fn test_speech_ratio_mixed() {
        let mut vad = ShadowVAD::new();
        feed_ms(&mut vad, true, 500);
        feed_ms(&mut vad, false, 500);
        let ratio = vad.speech_ratio();
        assert!(ratio > 0.4 && ratio < 0.6, "expected ~0.5, got {ratio}");
    }

    #[test]
    fn test_reset_clears_state() {
        let mut vad = ShadowVAD::new();
        feed_ms(&mut vad, true, 200);
        assert!(vad.is_speech());
        assert!(vad.speech_ratio() > 0.0);

        vad.reset();
        assert!(!vad.is_speech());
        assert_eq!(vad.speech_ratio(), 0.0);
    }

    #[test]
    fn test_empty_buffer_is_noop() {
        let mut vad = ShadowVAD::new();
        vad.process(&[]);
        assert!(!vad.is_speech());
        assert_eq!(vad.speech_ratio(), 0.0);
    }

    #[test]
    fn test_rapid_transitions() {
        let mut vad = ShadowVAD::new();
        feed_ms(&mut vad, true, 150);
        assert!(vad.is_speech());

        for _ in 0..10 {
            feed_ms(&mut vad, false, 50);
            feed_ms(&mut vad, true, 50);
        }
        assert!(vad.is_speech(), "rapid toggling should not flip state due to hysteresis");
    }
}
