use tauri::{AppHandle, Runtime, State};
use tracing::info;

use crate::recording::AppState;

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
