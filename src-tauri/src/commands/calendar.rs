use serde::Serialize;
use tauri::{AppHandle, Runtime};
use tauri_plugin_opener::OpenerExt;
use tracing::info;

use crate::calendar::{self, google, CalendarConfig, CalendarEvent};

#[derive(Debug, Serialize)]
pub struct CalendarStatus {
    pub connected: bool,
    pub provider: String,
    pub config: CalendarConfig,
}

#[tauri::command]
pub fn get_calendar_status<R: Runtime>(app: AppHandle<R>) -> CalendarStatus {
    let config = calendar::scheduler::load_config(&app);
    let tokens = calendar::scheduler::load_google_tokens(&app);
    CalendarStatus {
        connected: tokens.is_some(),
        provider: config.provider.clone(),
        config,
    }
}

#[tauri::command]
pub async fn connect_google_calendar<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    info!("Starting Google Calendar OAuth flow");
    let url = google::build_oauth_url();
    app.opener()
        .open_url(&url, None::<&str>)
        .map_err(|e| format!("Failed to open OAuth URL: {e}"))?;
    Ok(())
}

#[tauri::command]
pub async fn handle_calendar_callback<R: Runtime>(
    app: AppHandle<R>,
    code: String,
) -> Result<(), String> {
    info!("Processing Google Calendar OAuth callback");
    let tokens = google::exchange_code(&code).await?;
    calendar::scheduler::save_google_tokens(&app, &tokens);

    // Enable calendar by default after connecting
    let mut config = calendar::scheduler::load_config(&app);
    config.enabled = true;
    calendar::scheduler::save_config(&app, &config);

    info!("Google Calendar connected successfully");
    Ok(())
}

#[tauri::command]
pub fn disconnect_calendar<R: Runtime>(app: AppHandle<R>) {
    info!("Disconnecting Google Calendar");
    calendar::scheduler::clear_google_tokens(&app);
    let mut config = calendar::scheduler::load_config(&app);
    config.enabled = false;
    calendar::scheduler::save_config(&app, &config);
}

#[tauri::command]
pub fn set_calendar_config<R: Runtime>(
    app: AppHandle<R>,
    enabled: Option<bool>,
    remind_minutes: Option<u32>,
    auto_record: Option<bool>,
) {
    let mut config = calendar::scheduler::load_config(&app);
    if let Some(e) = enabled {
        config.enabled = e;
    }
    if let Some(m) = remind_minutes {
        config.remind_minutes_before = m;
    }
    if let Some(a) = auto_record {
        config.auto_record = a;
    }
    calendar::scheduler::save_config(&app, &config);
    info!(?config, "Calendar config updated");
}

#[tauri::command]
pub async fn get_upcoming_events<R: Runtime>(
    app: AppHandle<R>,
) -> Result<Vec<CalendarEvent>, String> {
    let tokens = calendar::scheduler::load_google_tokens(&app)
        .ok_or_else(|| "Calendar not connected".to_string())?;
    google::fetch_upcoming_events(&tokens.access_token, 60).await
}
