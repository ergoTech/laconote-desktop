#[cfg(target_os = "macos")]
pub mod cat_tap;
#[cfg(target_os = "macos")]
pub mod mic_capture;
#[cfg(target_os = "macos")]
pub mod mixer;
#[cfg(target_os = "macos")]
pub mod encoder;
#[cfg(target_os = "macos")]
pub mod silence;
#[cfg(target_os = "macos")]
pub mod health;
#[cfg(target_os = "macos")]
pub mod util;

#[cfg(target_os = "macos")]
pub use cat_tap::{
    is_available as catap_available,
    start as start_catap_capture,
    macos_version as catap_macos_version,
    probe_capture_readiness as probe_catap_capture_readiness,
    probe_permission as probe_catap_permission,
    last_diagnostic as catap_last_diagnostic,
    check_catap_compatibility,
    CaTapHandle, CaTapCompatibility, CaptureError, CaptureProbeResult, CaptureReadiness,
};

#[cfg(target_os = "macos")]
pub use mic_capture::{
    list_devices as list_mic_devices, start as start_mic_capture, MicCaptureHandle, MicError,
};

#[cfg(target_os = "macos")]
pub use mixer::{
    start as start_mixer, MixerConfig, MixerConsumer, MixerHandle,
    DEFAULT_MIC_GAIN, DEFAULT_SYSTEM_GAIN,
};

#[cfg(target_os = "macos")]
pub use encoder::{encode_to_webm, FRAME_SAMPLES, SAMPLE_RATE};

#[cfg(target_os = "macos")]
pub use silence::{compute_rms, SilenceDetector, SplitDecision, SILENCE_THRESHOLD};

#[cfg(target_os = "macos")]
pub use health::HealthMonitor;
