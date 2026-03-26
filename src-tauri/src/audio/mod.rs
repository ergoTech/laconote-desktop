#[cfg(target_os = "macos")]
pub mod cat_tap;
#[cfg(target_os = "windows")]
pub mod wasapi_capture;
#[cfg(any(target_os = "macos", target_os = "windows"))]
pub mod mic_capture;
pub mod mixer;
pub mod encoder;
pub mod silence;
pub mod health;
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

#[cfg(target_os = "windows")]
pub use wasapi_capture::{
    start as start_wasapi_capture, WasapiHandle,
    CaptureError as WasapiCaptureError,
};

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub use mic_capture::{
    list_devices as list_mic_devices, start as start_mic_capture, MicCaptureHandle, MicError,
};

pub use mixer::{
    start as start_mixer, AudioLevels, MixerConfig, MixerConsumer, MixerHandle,
    DEFAULT_MIC_GAIN, DEFAULT_SYSTEM_GAIN,
};

pub use encoder::{encode_to_webm, FRAME_SAMPLES, SAMPLE_RATE};

pub use silence::{compute_rms, SilenceDetector, SplitDecision, SILENCE_THRESHOLD};

pub use health::HealthMonitor;
