use crate::recording::AppState;
use tauri::{AppHandle, Manager, Runtime};

use super::icons::clear_tray_title;

#[cfg(target_os = "macos")]
pub fn start_shadow_tray_updater<R: Runtime + 'static>(app: &AppHandle<R>) {
    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
        loop {
            interval.tick().await;
            let state = app_handle.state::<AppState>();
            let status = {
                let guard = state
                    .shadow_session
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                guard.as_ref().map(|s| s.status())
            };
            match status {
                Some(status) => {
                    if let Some(tray) = app_handle.tray_by_id("main-tray") {
                        let total_ms = status.total_meeting_duration_ms;
                        let mins = total_ms / 60_000;
                        let secs = (total_ms % 60_000) / 1000;
                        let title = format!("{:02}:{:02}", mins, secs);
                        tray.set_title(Some(&title)).ok();
                    }
                }
                None => {
                    clear_tray_title(&app_handle);
                    break;
                }
            }
        }
    });
}
