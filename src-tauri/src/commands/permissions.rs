use serde::Serialize;
use tracing::info;

use crate::permissions::PermissionStatus;

#[derive(Debug, Serialize)]
pub struct AudioDiagnostic {
    pub macos_version: String,
    pub catap_available: bool,
    pub catap_permission_probe: bool,
    pub screen_capture_preflight: bool,
    pub mic_authorized: bool,
    pub is_dev_build: bool,
    pub last_catap_error: Option<String>,
}

#[tauri::command]
pub async fn check_permissions() -> PermissionStatus {
    tokio::task::spawn_blocking(|| crate::permissions::check_permissions())
        .await
        .unwrap_or_else(|_| PermissionStatus {
            system_audio_status: crate::permissions::PermissionState::Unknown,
            system_audio: crate::permissions::PermissionState::Unknown,
            screen_capture_access: crate::permissions::PermissionState::Unknown,
            system_audio_capture_ready: crate::permissions::PermissionState::Unknown,
            microphone: crate::permissions::PermissionState::Unknown,
        })
}

#[tauri::command]
pub fn open_system_settings(pane: String) -> bool {
    info!("Opening system settings pane: {pane}");
    crate::permissions::open_system_settings(&pane)
}

#[tauri::command]
pub async fn request_mic_permission() -> bool {
    info!("Requesting microphone permission");
    tokio::task::spawn_blocking(|| crate::permissions::request_mic_permission())
        .await
        .unwrap_or(false)
}

#[tauri::command]
pub async fn probe_system_audio_capture() -> crate::audio::CaptureProbeResult {
    info!("Running system audio capture readiness probe");
    tokio::task::spawn_blocking(|| {
        crate::audio::probe_catap_capture_readiness(std::time::Duration::from_millis(1200))
    })
    .await
    .unwrap_or_else(|_| crate::audio::CaptureProbeResult {
        state: crate::audio::CaptureReadiness::Unknown,
        detail: "Permission probe task panicked".into(),
    })
}

#[tauri::command]
pub async fn get_audio_diagnostic() -> AudioDiagnostic {
    tokio::task::spawn_blocking(|| {
        #[cfg(target_os = "macos")]
        {
            let (major, minor, patch) = crate::audio::catap_macos_version();

            #[link(name = "CoreGraphics", kind = "framework")]
            extern "C" {
                fn CGPreflightScreenCaptureAccess() -> bool;
            }

            let diagnostic_str = crate::audio::catap_last_diagnostic();
            let last_catap_error = if diagnostic_str.is_empty() {
                None
            } else {
                Some(diagnostic_str)
            };

            AudioDiagnostic {
                macos_version: format!("{major}.{minor}.{patch}"),
                catap_available: crate::audio::catap_available(),
                catap_permission_probe: crate::audio::probe_catap_permission(),
                screen_capture_preflight: unsafe { CGPreflightScreenCaptureAccess() },
                mic_authorized: crate::permissions::check_microphone_public(),
                is_dev_build: cfg!(debug_assertions),
                last_catap_error,
            }
        }

        #[cfg(not(target_os = "macos"))]
        AudioDiagnostic {
            macos_version: "N/A".into(),
            catap_available: false,
            catap_permission_probe: false,
            screen_capture_preflight: false,
            mic_authorized: false,
            is_dev_build: cfg!(debug_assertions),
            last_catap_error: None,
        }
    })
    .await
    .unwrap_or(AudioDiagnostic {
        macos_version: "unknown".into(),
        catap_available: false,
        catap_permission_probe: false,
        screen_capture_preflight: false,
        mic_authorized: false,
        is_dev_build: false,
        last_catap_error: None,
    })
}
