use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Runtime, State};
use tracing::info;

use crate::recording::{AppState, RecordingConfig};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingStatus {
    pub is_recording: bool,
    pub duration_seconds: u64,
    pub chunks_uploaded: u32,
}

#[tauri::command]
pub async fn start_recording<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    meeting_name: Option<String>,
    meeting_type: Option<String>,
    project_id: Option<String>,
    mic_device: Option<String>,
) -> Result<String, String> {
    #[cfg(not(target_os = "macos"))]
    return Err("Recording is only supported on macOS".to_string());

    #[cfg(target_os = "macos")]
    {
        {
            let guard = state
                .session
                .lock()
                .map_err(|_| "Session lock poisoned".to_string())?;
            if guard.is_some() {
                return Err("Already recording".to_string());
            }
        }

        {
            let guard = state
                .shadow_session
                .lock()
                .map_err(|_| "Shadow session lock poisoned".to_string())?;
            if guard.is_some() {
                return Err("Shadow recording is active. Use Save & Record to transition.".to_string());
            }
        }

        let existing_meeting_id = {
            let mut guard = state
                .pending_meeting_id
                .lock()
                .map_err(|_| "Pending meeting ID lock poisoned".to_string())?;
            guard.take()
        };

        let config = RecordingConfig {
            meeting_name,
            meeting_type,
            project_id,
            mic_device,
        };

        let (notify_tx, mut notify_rx) = tokio::sync::mpsc::channel::<String>(32);

        let app_for_notify = app.clone();
        tokio::spawn(async move {
            while let Some(msg) = notify_rx.recv().await {
                use tauri_plugin_notification::NotificationExt;
                let _ = app_for_notify
                    .notification()
                    .builder()
                    .title("Laconote")
                    .body(&msg)
                    .show();
                crate::tray::set_tray_warning(&app_for_notify).ok();
            }
        });

        let session = crate::recording::start_session(&app, config, notify_tx, existing_meeting_id)
            .await
            .map_err(|e| {
                if e.contains("Not authenticated") || e.contains("Session expired") {
                    use tauri::Emitter;
                    info!("Auth error during start_recording, clearing token");
                    let _ = crate::auth::keychain::delete_token(&app);
                    let _ = app.emit("auth-changed", ());
                }
                e
            })?;
        let meeting_id = session.meeting_id.clone();

        {
            let mut guard = state
                .session
                .lock()
                .map_err(|_| "Session lock poisoned".to_string())?;
            *guard = Some(session);
        }

        crate::tray::set_tray_recording(&app, true).ok();
        crate::tray::update_tray_menu(&app).ok();

        info!("Recording started: {meeting_id}");
        Ok(meeting_id)
    }
}

#[tauri::command]
pub async fn stop_recording<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    #[cfg(not(target_os = "macos"))]
    return Err("Recording is only supported on macOS".to_string());

    #[cfg(target_os = "macos")]
    {
        let session = {
            let mut guard = state
                .session
                .lock()
                .map_err(|_| "Session lock poisoned".to_string())?;
            guard.take()
        };

        match session {
            Some(s) => {
                let meeting_id = s.meeting_id.clone();
                s.stop();
                crate::tray::set_tray_recording(&app, false).ok();
                crate::tray::update_tray_menu(&app).ok();

                info!("Recording stopped: {meeting_id}");
                Ok(meeting_id)
            }
            None => Err("Not currently recording".to_string()),
        }
    }
}

#[tauri::command]
pub fn get_recording_status(state: State<'_, AppState>) -> RecordingStatus {
    #[cfg(target_os = "macos")]
    {
        let guard = state.session.lock().unwrap_or_else(|e| e.into_inner());
        return match &*guard {
            Some(session) => RecordingStatus {
                is_recording: true,
                duration_seconds: session.duration_seconds(),
                chunks_uploaded: session.chunks_uploaded(),
            },
            None => RecordingStatus {
                is_recording: false,
                duration_seconds: 0,
                chunks_uploaded: 0,
            },
        };
    }

    #[allow(unreachable_code)]
    RecordingStatus {
        is_recording: false,
        duration_seconds: 0,
        chunks_uploaded: 0,
    }
}

#[tauri::command]
pub fn list_audio_devices() -> Vec<String> {
    #[cfg(target_os = "macos")]
    {
        return crate::audio::list_mic_devices();
    }

    #[allow(unreachable_code)]
    Vec::new()
}
