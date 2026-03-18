use std::sync::Mutex;
use std::time::Instant;
use tracing::info;

#[cfg(target_os = "macos")]
use crate::audio::{
    start_catap_capture, start_mic_capture, start_mixer,
    check_catap_compatibility, catap_macos_version, probe_catap_capture_readiness,
    CaTapCompatibility, CaptureReadiness, CaTapHandle, HealthMonitor,
    MicCaptureHandle, MixerConfig, MixerHandle,
};
#[cfg(target_os = "macos")]
use crate::upload::{
    chunker::{self, ChunkPipelineConfig},
    ChunkPipelineHandle, OfflineQueue, Uploader,
};
#[cfg(target_os = "macos")]
use chrono::Utc;
#[cfg(target_os = "macos")]
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

pub struct AppState {
    #[cfg(target_os = "macos")]
    pub session: Mutex<Option<RecordingSession>>,
    #[cfg(not(target_os = "macos"))]
    pub session: Mutex<Option<std::convert::Infallible>>,

    #[cfg(target_os = "macos")]
    pub shadow_session: Mutex<Option<crate::shadow::ShadowSession>>,

    #[cfg(target_os = "macos")]
    pub pending_meeting_id: Mutex<Option<String>>,
}

impl Default for AppState {
    fn default() -> Self {
        AppState {
            session: Mutex::new(None),

            #[cfg(target_os = "macos")]
            shadow_session: Mutex::new(None),

            #[cfg(target_os = "macos")]
            pending_meeting_id: Mutex::new(None),
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

    let perm_status = crate::permissions::check_permissions();
    if perm_status.microphone != crate::permissions::PermissionState::Granted {
        return Err("Microphone permission is not granted. Please enable it in System Settings.".to_string());
    }

    let compat = check_catap_compatibility();
    match compat {
        CaTapCompatibility::Unsupported => {
            let (major, minor, _) = catap_macos_version();
            return Err(format!(
                "System audio capture requires macOS 14.2 or later. \
                 Current version: {major}.{minor}. \
                 Please upgrade macOS to use this feature."
            ));
        }
        CaTapCompatibility::Unknown => {
            let (major, minor, patch) = catap_macos_version();
            tracing::warn!(
                major, minor, patch,
                "Unknown macOS version — CATap compatibility unverified, attempting anyway"
            );
        }
        CaTapCompatibility::Supported => {}
    }

    let audio_already_confirmed = perm_status.system_audio_capture_ready
        == crate::permissions::PermissionState::Granted;

    if !audio_already_confirmed {
        let system_audio_probe =
            probe_catap_capture_readiness(std::time::Duration::from_millis(1200));
        match system_audio_probe.state {
            CaptureReadiness::Ready | CaptureReadiness::AuthorizedButSilent => {
                tracing::info!(detail = %system_audio_probe.detail, "System audio readiness probe succeeded (or authorized but silent)");
            }
            CaptureReadiness::Unknown => {
                tracing::warn!(detail = %system_audio_probe.detail, "System audio readiness probe was inconclusive; continuing with live start");
            }
            CaptureReadiness::NotReady => {
                return Err(format!(
                    "System audio capture is not ready. {}. Open System Settings → Privacy & Security → Screen & System Audio Recording and verify that Laconote is allowed.",
                    system_audio_probe.detail
                ));
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    } else {
        tracing::info!("System audio permission already confirmed via CATap probe; skipping readiness probe");
    }

    let (sys_rx, system_handle) = start_catap_capture()
        .or_else(|first_err| {
            tracing::warn!(error = %first_err, "CATap start failed, retrying after 300ms");
            std::thread::sleep(std::time::Duration::from_millis(300));
            start_catap_capture()
        })
        .map_err(|e| {
            let (major, minor, patch) = catap_macos_version();
            if matches!(check_catap_compatibility(), CaTapCompatibility::Unknown) {
                format!(
                    "System audio capture (CATap) failed on macOS {major}.{minor}.{patch} (unverified version). \
                     Error: {e}. This macOS version may not support CATap."
                )
            } else {
                format!("System audio capture (CATap) failed: {e}")
            }
        })?;

    let (mic_rx, mic_handle) = start_mic_capture(config.mic_device).map_err(|e| {
        if e.to_string().contains("permission") || e.to_string().contains("denied") {
            format!(
                "Microphone access denied. Open System Settings → Privacy & Security → Microphone and enable Laconote."
            )
        } else {
            format!("Microphone capture failed: {e}")
        }
    })?;

    let (mixer_consumer, mixer_handle) = start_mixer(sys_rx, mic_rx, MixerConfig::default())
        .map_err(|e| format!("Audio mixer failed: {e}"))?;

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
