use tauri::{AppHandle, Runtime};
use tracing::{info, warn};

use crate::config::DASHBOARD_URL;

#[tauri::command]
pub fn open_dashboard<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    use tauri::Manager;
    info!("Opening dashboard");
    if let Some(window) = app.get_webview_window("dashboard") {
        // Only navigate if not already on laconote.com — preserves recording state
        window
            .eval(format!(
                "if (!window.location.href.startsWith('https://laconote.com')) {{ window.location.href = '{DASHBOARD_URL}'; }}"
            ))
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

#[tauri::command]
pub fn log_dashboard_event(message: String) {
    warn!(target: "dashboard_webview", "{message}");
}
