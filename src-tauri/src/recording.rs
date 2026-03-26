use std::sync::Mutex;
use std::time::Instant;
use tracing::info;

#[cfg(target_os = "macos")]
use crate::audio::{
    start_catap_capture, start_mic_capture, start_mixer,
    check_catap_compatibility, catap_macos_version,
    CaTapCompatibility, CaTapHandle, HealthMonitor,
    MicCaptureHandle, MixerConfig, MixerHandle,
};
#[cfg(target_os = "windows")]
use crate::audio::{
    start_wasapi_capture, start_mic_capture, start_mixer,
    WasapiHandle, HealthMonitor,
    MicCaptureHandle, MixerConfig, MixerHandle,
};
#[cfg(any(target_os = "macos", target_os = "windows"))]
use crate::upload::{
    chunker::{self, ChunkPipelineConfig},
    ChunkPipelineHandle, OfflineQueue, Uploader,
};
#[cfg(any(target_os = "macos", target_os = "windows"))]
use chrono::Utc;
#[cfg(any(target_os = "macos", target_os = "windows"))]
use uuid::Uuid;

pub struct RecordingConfig {
    pub meeting_name: Option<String>,
    pub meeting_type: Option<String>,
    pub project_id: Option<String>,
    pub mic_device: Option<String>,
}

#[cfg(target_os = "macos")]
pub struct RecordingSession {
    pub meeting_id: String,
    pub started_at: Instant,
    system_handle: CaTapHandle,
    mic_handle: MicCaptureHandle,
    mixer_handle: MixerHandle,
    pub chunk_handle: ChunkPipelineHandle,
    health_monitor: HealthMonitor,
}

#[cfg(target_os = "macos")]
impl RecordingSession {
    pub fn duration_seconds(&self) -> u64 {
        self.started_at.elapsed().as_secs()
    }

    pub fn chunks_uploaded(&self) -> u32 {
        self.chunk_handle.chunks_uploaded()
    }

    pub fn stop(self) {
        info!(meeting_id = %self.meeting_id, "Stopping recording session");
        self.health_monitor.stop();
        self.mic_handle.stop();
        self.system_handle.stop();
        std::thread::sleep(std::time::Duration::from_millis(100));
        self.mixer_handle.stop();
        std::thread::sleep(std::time::Duration::from_millis(100));
        self.chunk_handle.stop();
        info!("All audio pipelines stopped");
    }
}

#[cfg(target_os = "windows")]
pub struct RecordingSession {
    pub meeting_id: String,
    pub started_at: Instant,
    system_handle: WasapiHandle,
    mic_handle: MicCaptureHandle,
    mixer_handle: MixerHandle,
    pub chunk_handle: ChunkPipelineHandle,
    health_monitor: HealthMonitor,
}

#[cfg(target_os = "windows")]
impl RecordingSession {
    pub fn duration_seconds(&self) -> u64 {
        self.started_at.elapsed().as_secs()
    }

    pub fn chunks_uploaded(&self) -> u32 {
        self.chunk_handle.chunks_uploaded()
    }

    pub fn stop(self) {
        info!(meeting_id = %self.meeting_id, "Stopping recording session");
        self.health_monitor.stop();
        self.mic_handle.stop();
        self.system_handle.stop();
        std::thread::sleep(std::time::Duration::from_millis(100));
        self.mixer_handle.stop();
        std::thread::sleep(std::time::Duration::from_millis(100));
        self.chunk_handle.stop();
        info!("All audio pipelines stopped");
    }
}

pub struct AppState {
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    pub session: Mutex<Option<RecordingSession>>,
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    pub session: Mutex<Option<std::convert::Infallible>>,

    #[cfg(target_os = "macos")]
    pub shadow_session: Mutex<Option<crate::shadow::ShadowSession>>,

    #[cfg(any(target_os = "macos", target_os = "windows"))]
    pub pending_meeting_id: Mutex<Option<String>>,

    /// Prevents concurrent start_recording calls (race condition guard)
    pub starting: std::sync::atomic::AtomicBool,

    /// Prevents concurrent shadow start calls (race condition guard)
    #[cfg(target_os = "macos")]
    pub shadow_starting: std::sync::atomic::AtomicBool,
}

impl Default for AppState {
    fn default() -> Self {
        AppState {
            session: Mutex::new(None),

            #[cfg(target_os = "macos")]
            shadow_session: Mutex::new(None),

            #[cfg(any(target_os = "macos", target_os = "windows"))]
            pending_meeting_id: Mutex::new(None),

            starting: std::sync::atomic::AtomicBool::new(false),

            #[cfg(target_os = "macos")]
            shadow_starting: std::sync::atomic::AtomicBool::new(false),
        }
    }
}

#[cfg(target_os = "macos")]
pub async fn start_session<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    config: RecordingConfig,
    notify_tx: tokio::sync::mpsc::Sender<String>,
    existing_meeting_id: Option<String>,
) -> Result<RecordingSession, String> {
    use crate::auth::keychain;

    let jwt = keychain::get_token(app)
        .ok_or_else(|| "Not authenticated – please log in first".to_string())?;

    if !keychain::is_token_valid(&jwt) {
        return Err("Session expired – please log in again".to_string());
    }

    let meeting_id = existing_meeting_id.unwrap_or_else(|| Uuid::new_v4().to_string());
    let meeting_start_time = Utc::now();
    let (major, minor, patch) = catap_macos_version();
    let dev_hint = if cfg!(debug_assertions) {
        " In dev mode, permissions may reset after rebuild — re-grant and restart the app."
    } else {
        ""
    };

    // Step 1: Check permissions
    info!("Step 1/6: Checking permissions (macOS {major}.{minor}.{patch})");
    let perm_status = crate::permissions::check_permissions();
    if perm_status.microphone != crate::permissions::PermissionState::Granted {
        return Err(format!(
            "Microphone permission is not granted on macOS {major}.{minor}.{patch}. \
             Please enable it in System Settings -> Privacy & Security -> Microphone.{dev_hint}"
        ));
    }

    // Step 2: Check CATap compatibility
    info!("Step 2/6: Checking CATap compatibility");
    let compat = check_catap_compatibility();
    match compat {
        CaTapCompatibility::Unsupported => {
            return Err(format!(
                "System audio capture requires macOS 14.2 or later. \
                 Current version: {major}.{minor}. \
                 Please upgrade macOS to use this feature."
            ));
        }
        CaTapCompatibility::Unknown => {
            tracing::warn!(
                major, minor, patch,
                "Unknown macOS version — CATap compatibility unverified, attempting anyway"
            );
        }
        CaTapCompatibility::Supported => {
            info!("CATap compatible (macOS {major}.{minor}.{patch})");
        }
    }

    // Step 3: Probe system audio permission (lightweight — no full CATap lifecycle)
    info!("Step 3/6: Probing system audio permission");
    let audio_already_confirmed = perm_status.system_audio_capture_ready
        == crate::permissions::PermissionState::Granted;

    if !audio_already_confirmed {
        crate::permissions::invalidate_catap_probe_cache();
        let probe_ok = crate::audio::probe_catap_permission();
        if !probe_ok {
            let diagnostic = crate::audio::catap_last_diagnostic();
            let detail = if diagnostic.is_empty() {
                "permission probe returned false".to_string()
            } else {
                diagnostic
            };
            return Err(format!(
                "System audio capture permission not granted on macOS {major}.{minor}.{patch}: {detail}. \
                 Open System Settings -> Privacy & Security -> Screen & System Audio Recording \
                 and verify that Laconote is allowed.{dev_hint}"
            ));
        }
        info!("System audio permission probe succeeded");
        std::thread::sleep(std::time::Duration::from_millis(100));
    } else {
        info!("System audio permission already confirmed; skipping probe");
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    crate::permissions::invalidate_catap_probe_cache();

    // Step 4: Start microphone capture FIRST.
    // Opening the mic before creating the CATap aggregate device avoids
    // disrupting the CoreAudio device graph while the mic is already in use
    // by other apps (e.g. Google Meet via WebRTC).
    info!("Step 4/6: Starting microphone capture");
    let (mic_rx, mic_handle, mic_sr) = start_mic_capture(config.mic_device).map_err(|e| {
        let e_str = e.to_string();
        if e_str.contains("permission") || e_str.contains("denied") {
            format!(
                "Microphone access denied. Open System Settings → Privacy & Security → Microphone and enable Laconote."
            )
        } else {
            format!("Microphone capture failed: {e}")
        }
    })?;

    // Step 5: Start CATap system audio capture (with retries).
    // Done after mic so the aggregate device creation doesn't interfere
    // with microphone negotiation for other apps.
    info!("Step 5/6: Starting CATap system audio capture");
    let (sys_rx, system_handle, system_sr) = {
        let retry_delays = [0u64, 500, 1000];
        let mut last_err = String::new();
        let mut result = None;
        for (attempt, delay_ms) in retry_delays.iter().enumerate() {
            if *delay_ms > 0 {
                tracing::warn!(attempt, delay_ms, error = %last_err, "CATap start failed, retrying");
                std::thread::sleep(std::time::Duration::from_millis(*delay_ms));
            }
            match start_catap_capture() {
                Ok(handles) => {
                    if attempt > 0 {
                        info!(attempt, "CATap start succeeded on retry");
                    }
                    result = Some(handles);
                    break;
                }
                Err(e) => {
                    last_err = e.to_string();
                }
            }
        }
        result.ok_or_else(|| {
            format!(
                "System audio capture (CATap) failed after 3 attempts on macOS {major}.{minor}.{patch}: \
                 {last_err}. Open System Settings -> Privacy & Security -> Screen & System Audio Recording \
                 and verify that Laconote is allowed.{dev_hint}"
            )
        })?
    };

    // Step 6: Start audio mixer and encoder pipeline
    info!(system_sr, mic_sr, "Step 6/6: Starting audio mixer and encoder pipeline");
    let mixer_config = MixerConfig {
        system_gain: crate::audio::DEFAULT_SYSTEM_GAIN,
        mic_gain: crate::audio::DEFAULT_MIC_GAIN,
        system_sample_rate: system_sr,
        mic_sample_rate: mic_sr,
    };
    let (mixer_consumer, mixer_handle, levels_rx) = start_mixer(sys_rx, mic_rx, mixer_config)
        .map_err(|e| format!("Audio mixer failed: {e}"))?;

    // Emit audio levels to frontend for visualization
    let app_for_levels = app.clone();
    std::thread::Builder::new()
        .name("laconote-audio-levels".into())
        .spawn(move || {
            use tauri::Emitter;
            let mut counter: u64 = 0;
            while let Ok(levels) = levels_rx.recv() {
                let _ = app_for_levels.emit("audio-levels", &levels);
                counter += 1;
                // Log every ~5 seconds (levels arrive at 10 Hz)
                if counter % 50 == 0 {
                    tracing::info!(
                        system_rms = %format!("{:.4}", levels.system_rms),
                        mic_rms = %format!("{:.4}", levels.mic_rms),
                        mixed_rms = %format!("{:.4}", levels.mixed_rms),
                        "Audio levels snapshot"
                    );
                }
            }
        })
        .ok();

    let uploader = Uploader::new(jwt)?;
    let offline_queue = OfflineQueue::new()?;

    let pipeline_config = ChunkPipelineConfig {
        meeting_id: meeting_id.clone(),
        meeting_name: config.meeting_name,
        meeting_type: config.meeting_type,
        project_id: config.project_id,
        meeting_start_time,
        uploader,
        offline_queue,
        notify_tx,
    };

    let chunk_handle = chunker::start(mixer_consumer, pipeline_config).await;

    let (health_tx, health_rx) = crossbeam_channel::bounded::<String>(32);
    let health_monitor = HealthMonitor::start(
        mixer_handle.running_flag(),
        system_handle.running_flag(),
        mic_handle.running_flag(),
        health_tx,
    );

    let app_for_health = app.clone();
    std::thread::Builder::new()
        .name("laconote-health-bridge".into())
        .spawn(move || {
            use tauri::Emitter;
            while let Ok(msg) = health_rx.recv() {
                let _ = app_for_health.emit("recording-health-warning", &msg);
            }
        })
        .ok();

    info!(meeting_id = %meeting_id, "Recording session started");

    Ok(RecordingSession {
        meeting_id,
        started_at: Instant::now(),
        system_handle,
        mic_handle,
        mixer_handle,
        chunk_handle,
        health_monitor,
    })
}

#[cfg(target_os = "windows")]
pub async fn start_session<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    config: RecordingConfig,
    notify_tx: tokio::sync::mpsc::Sender<String>,
    existing_meeting_id: Option<String>,
) -> Result<RecordingSession, String> {
    use crate::auth::keychain;

    let jwt = keychain::get_token(app)
        .ok_or_else(|| "Not authenticated – please log in first".to_string())?;

    if !keychain::is_token_valid(&jwt) {
        return Err("Session expired – please log in again".to_string());
    }

    let meeting_id = existing_meeting_id.unwrap_or_else(|| Uuid::new_v4().to_string());
    let meeting_start_time = Utc::now();

    // Step 1: Check microphone permission (Windows grants by default for desktop apps)
    info!("Step 1/4: Checking permissions");
    let perm_status = crate::permissions::check_permissions();
    if perm_status.microphone != crate::permissions::PermissionState::Granted {
        return Err(
            "Microphone permission is not granted. \
             Please enable it in Windows Settings -> Privacy -> Microphone."
                .to_string(),
        );
    }

    // Step 2: Start microphone capture
    info!("Step 2/4: Starting microphone capture");
    let (mic_rx, mic_handle, mic_sr) = start_mic_capture(config.mic_device).map_err(|e| {
        format!("Microphone capture failed: {e}")
    })?;

    // Step 3: Start WASAPI loopback system audio capture
    info!("Step 3/4: Starting WASAPI loopback system audio capture");
    let (sys_rx, system_handle, system_sr) = start_wasapi_capture()
        .map_err(|e| format!("System audio capture (WASAPI loopback) failed: {e}"))?;

    // Step 4: Start audio mixer and encoder pipeline
    info!(system_sr, mic_sr, "Step 4/4: Starting audio mixer and encoder pipeline");
    let mixer_config = MixerConfig {
        system_gain: crate::audio::DEFAULT_SYSTEM_GAIN,
        mic_gain: crate::audio::DEFAULT_MIC_GAIN,
        system_sample_rate: system_sr,
        mic_sample_rate: mic_sr,
    };
    let (mixer_consumer, mixer_handle, levels_rx) = start_mixer(sys_rx, mic_rx, mixer_config)
        .map_err(|e| format!("Audio mixer failed: {e}"))?;

    // Emit audio levels to frontend for visualization
    let app_for_levels = app.clone();
    std::thread::Builder::new()
        .name("laconote-audio-levels".into())
        .spawn(move || {
            use tauri::Emitter;
            let mut counter: u64 = 0;
            while let Ok(levels) = levels_rx.recv() {
                let _ = app_for_levels.emit("audio-levels", &levels);
                counter += 1;
                if counter % 50 == 0 {
                    tracing::info!(
                        system_rms = %format!("{:.4}", levels.system_rms),
                        mic_rms = %format!("{:.4}", levels.mic_rms),
                        mixed_rms = %format!("{:.4}", levels.mixed_rms),
                        "Audio levels snapshot"
                    );
                }
            }
        })
        .ok();

    let uploader = Uploader::new(jwt)?;
    let offline_queue = OfflineQueue::new()?;

    let pipeline_config = ChunkPipelineConfig {
        meeting_id: meeting_id.clone(),
        meeting_name: config.meeting_name,
        meeting_type: config.meeting_type,
        project_id: config.project_id,
        meeting_start_time,
        uploader,
        offline_queue,
        notify_tx,
    };

    let chunk_handle = chunker::start(mixer_consumer, pipeline_config).await;

    let (health_tx, health_rx) = crossbeam_channel::bounded::<String>(32);
    let health_monitor = HealthMonitor::start(
        mixer_handle.running_flag(),
        system_handle.running_flag(),
        mic_handle.running_flag(),
        health_tx,
    );

    let app_for_health = app.clone();
    std::thread::Builder::new()
        .name("laconote-health-bridge".into())
        .spawn(move || {
            use tauri::Emitter;
            while let Ok(msg) = health_rx.recv() {
                let _ = app_for_health.emit("recording-health-warning", &msg);
            }
        })
        .ok();

    info!(meeting_id = %meeting_id, "Recording session started");

    Ok(RecordingSession {
        meeting_id,
        started_at: Instant::now(),
        system_handle,
        mic_handle,
        mixer_handle,
        chunk_handle,
        health_monitor,
    })
}
