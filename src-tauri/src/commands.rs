use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Runtime, State};
use tracing::info;

use crate::auth::keychain;
use crate::permissions::PermissionStatus;
use crate::recording::{AppState, RecordingConfig};

const DASHBOARD_URL: &str = "https://laconote.com/";
const LOGIN_URL: &str = "https://laconote.com/login?redirect=laconote%3A%2F%2Fauth%2Fcallback";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingStatus {
    pub is_recording: bool,
    pub duration_seconds: u64,
    pub chunks_uploaded: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthStatus {
    pub is_authenticated: bool,
    pub user_email: Option<String>,
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

        let session = crate::recording::start_session(&app, config, notify_tx, existing_meeting_id).await?;
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

#[tauri::command]
pub fn get_auth_status<R: Runtime>(app: AppHandle<R>) -> AuthStatus {
    match keychain::get_token(&app) {
        Some(token) if keychain::is_token_valid(&token) => AuthStatus {
            is_authenticated: true,
            user_email: keychain::extract_email(&token),
        },
        _ => AuthStatus {
            is_authenticated: false,
            user_email: None,
        },
    }
}

#[tauri::command]
pub async fn login<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    info!("Opening system browser for Google OAuth login");
    app.opener()
        .open_url(LOGIN_URL, None::<&str>)
        .map_err(|e| format!("Failed to open browser: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn open_dashboard<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    use tauri::Manager;
    info!("Opening dashboard");
    if let Some(window) = app.get_webview_window("dashboard") {
        window
            .eval(format!("window.location.href = '{DASHBOARD_URL}'"))
            .map_err(|e| format!("Failed to navigate dashboard: {e}"))?;
        window.show().map_err(|e| format!("Failed to show dashboard: {e}"))?;
        window.unminimize().ok();
        #[cfg(target_os = "macos")]
        {
            let w = window.clone();
            app.run_on_main_thread(move || {
                use objc2_app_kit::NSApplication;
                use objc2_foundation::MainThreadMarker;
                unsafe {
                    let mtm = MainThreadMarker::new_unchecked();
                    let ns_app = NSApplication::sharedApplication(mtm);
                    ns_app.activate();
                }
                let _ = w.set_focus();
            })
            .map_err(|e| format!("Failed to activate app: {e}"))?;
        }
        #[cfg(not(target_os = "macos"))]
        window.set_focus().map_err(|e| format!("Failed to focus dashboard: {e}"))?;
    }
    Ok(())
}

#[tauri::command]
pub fn logout<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    info!("Logging out");
    keychain::delete_token(&app)
}

#[tauri::command]
pub fn get_token<R: Runtime>(app: AppHandle<R>) -> Option<String> {
    keychain::get_token(&app)
}

#[tauri::command]
pub fn save_token<R: Runtime>(app: AppHandle<R>, token: String) -> Result<(), String> {
    use tauri::Emitter;
    info!("Web app pushed a token for native storage");
    keychain::store_token(&app, &token)?;
    app.emit("auth-changed", ()).map_err(|e| format!("Failed to emit auth-changed: {e}"))?;

    Ok(())
}

#[tauri::command]
pub fn check_permissions() -> PermissionStatus {
    crate::permissions::check_permissions()
}

#[tauri::command]
pub fn open_system_settings(pane: String) -> bool {
    info!("Opening system settings pane: {pane}");
    crate::permissions::open_system_settings(&pane)
}

#[tauri::command]
pub fn request_mic_permission() -> bool {
    info!("Requesting microphone permission");
    crate::permissions::request_mic_permission()
}

#[tauri::command]
pub fn probe_system_audio_capture() -> crate::audio::CaptureProbeResult {
    info!("Running system audio capture readiness probe");
    crate::audio::probe_catap_capture_readiness(std::time::Duration::from_millis(1200))
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn start_shadow_recording(
    app: AppHandle<tauri::Wry>,
    state: State<'_, AppState>,
    mic_device: Option<String>,
    buffer_minutes: Option<u32>,
) -> Result<(), String> {
    {
        let guard = state
            .session
            .lock()
            .map_err(|_| "Session lock poisoned".to_string())?;
        if guard.is_some() {
            return Err("Standard recording is active. Stop it first or use Save & Record.".to_string());
        }
    }

    {
        let guard = state
            .shadow_session
            .lock()
            .map_err(|_| "Shadow session lock poisoned".to_string())?;
        if guard.is_some() {
            return Err("Shadow recording is already active".to_string());
        }
    }

    let session =
        crate::shadow::ShadowSession::start(mic_device, buffer_minutes)?;

    {
        let mut guard = state
            .shadow_session
            .lock()
            .map_err(|_| "Shadow session lock poisoned".to_string())?;
        *guard = Some(session);
    }

    crate::tray::set_tray_shadow(&app).ok();
    crate::tray::update_tray_menu(&app).ok();
    crate::tray::start_shadow_tray_updater(&app);

    info!("Shadow recording started");
    Ok(())
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub async fn start_shadow_recording(
    _state: State<'_, AppState>,
    _mic_device: Option<String>,
    _buffer_minutes: Option<u32>,
) -> Result<(), String> {
    Err("Shadow recording is only supported on macOS".to_string())
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn stop_shadow_recording(
    app: AppHandle<tauri::Wry>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let session = {
        let mut guard = state
            .shadow_session
            .lock()
            .map_err(|_| "Shadow session lock poisoned".to_string())?;
        guard.take()
    };

    match session {
        Some(s) => {
            s.stop();
            crate::tray::set_tray_recording(&app, false).ok();
            crate::tray::update_tray_menu(&app).ok();
            info!("Shadow recording stopped and discarded");
            Ok(())
        }
        None => Err("No active shadow recording".to_string()),
    }
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub async fn stop_shadow_recording(
    _state: State<'_, AppState>,
) -> Result<(), String> {
    Err("Shadow recording is only supported on macOS".to_string())
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub fn get_shadow_status(state: State<'_, AppState>) -> crate::shadow::ShadowStatus {
    let guard = state
        .shadow_session
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    match &*guard {
        Some(session) => session.status(),
        None => crate::shadow::ShadowStatus::from(&crate::shadow::ShadowState::Inactive),
    }
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn save_shadow_buffer<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    action: String,
    meeting_name: Option<String>,
    meeting_type: Option<String>,
    project_id: Option<String>,
) -> Result<String, String> {
    match action.as_str() {
        "save_only" => {
            let session = {
                let mut guard = state
                    .shadow_session
                    .lock()
                    .map_err(|_| "Shadow session lock poisoned".to_string())?;
                guard.take()
            }
            .ok_or("No active shadow recording")?;

            let result = crate::shadow::save::save_only(session, &app, meeting_name, meeting_type, project_id)
                .await;
            crate::tray::set_tray_recording(&app, false).ok();
            crate::tray::update_tray_menu(&app).ok();
            result
        }
        "save_and_continue" => {
            let buffer = {
                let guard = state
                    .shadow_session
                    .lock()
                    .map_err(|_| "Shadow session lock poisoned".to_string())?;
                guard
                    .as_ref()
                    .map(|s| std::sync::Arc::clone(&s.buffer))
                    .ok_or("No active shadow recording")?
            };

            crate::shadow::save::save_and_continue(
                buffer,
                &app,
                meeting_name,
                meeting_type,
                project_id,
            )
            .await
        }
        "save_and_record" => {
            let session = {
                let mut guard = state
                    .shadow_session
                    .lock()
                    .map_err(|_| "Shadow session lock poisoned".to_string())?;
                guard.take()
            }
            .ok_or("No active shadow recording")?;

            let meeting_id = crate::shadow::save::save_and_record(
                session,
                &app,
                meeting_name,
                meeting_type,
                project_id,
            )
            .await?;

            {
                let mut guard = state
                    .pending_meeting_id
                    .lock()
                    .map_err(|_| "Pending meeting ID lock poisoned".to_string())?;
                *guard = Some(meeting_id.clone());
            }

            crate::tray::set_tray_recording(&app, false).ok();
            crate::tray::update_tray_menu(&app).ok();

            Ok(meeting_id)
        }
        _ => Err(format!("Unknown save action: {action}")),
    }
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub async fn save_shadow_buffer(
    _action: String,
    _meeting_name: Option<String>,
    _meeting_type: Option<String>,
    _project_id: Option<String>,
) -> Result<String, String> {
    Err("Shadow recording is only supported on macOS".to_string())
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub fn get_shadow_status(_state: State<'_, AppState>) -> serde_json::Value {
    serde_json::json!({
        "state": "inactive",
        "buffered_duration_ms": 0,
        "buffer_capacity_ms": 0,
        "fill_percent": 0,
        "total_meeting_duration_ms": 0,
        "upload_progress": null,
        "error_message": null,
        "speech_ratio": null
    })
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub fn get_shadow_schedule<R: Runtime>(
    app: AppHandle<R>,
) -> crate::shadow::schedule::ScheduleConfig {
    crate::shadow::schedule::load_schedule_config(&app)
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub fn get_shadow_schedule() -> serde_json::Value {
    serde_json::json!({
        "enabled": false,
        "start_time": "09:00",
        "end_time": "18:00",
        "days_bitmask": 30,
        "buffer_minutes": 20
    })
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub fn set_shadow_schedule<R: Runtime>(
    app: AppHandle<R>,
    enabled: bool,
    start_time: String,
    end_time: String,
    days_bitmask: u8,
) {
    let mut config = crate::shadow::schedule::load_schedule_config(&app);
    config.enabled = enabled;
    config.start_time = start_time;
    config.end_time = end_time;
    config.days_bitmask = days_bitmask;
    crate::shadow::schedule::save_schedule_config(&app, &config);
    info!("Shadow schedule updated: enabled={enabled}");
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub fn set_shadow_schedule(
    _enabled: bool,
    _start_time: String,
    _end_time: String,
    _days_bitmask: u8,
) {
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub fn set_shadow_buffer_duration<R: Runtime>(app: AppHandle<R>, minutes: u32) {
    let mut config = crate::shadow::schedule::load_schedule_config(&app);
    config.buffer_minutes = minutes;
    crate::shadow::schedule::save_schedule_config(&app, &config);
    info!("Shadow buffer duration updated: {minutes} min");
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub fn set_shadow_buffer_duration(_minutes: u32) {}

#[tauri::command]
pub fn restart_app<R: Runtime>(app: AppHandle<R>) {
    app.restart();
}

#[tauri::command]
pub async fn check_for_updates<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    info!("Manual update check triggered from UI");
    crate::updater::check_for_updates(&app)
        .await
        .map_err(|e| e.to_string())
}
