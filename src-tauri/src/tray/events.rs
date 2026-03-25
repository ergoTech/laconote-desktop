use crate::config;
use crate::recording::AppState;
use tauri::{AppHandle, Manager, Runtime};
use tracing::{info, warn};

use super::icons::{clear_tray_title, set_tray_recording, set_tray_shadow};
use super::menu::update_tray_menu;
use super::shadow_updater::start_shadow_tray_updater;
use super::windows::show_settings_window;

pub fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, event_id: &str) {
    match event_id {
        "start-recording" => {
            info!("Menu: Start Recording clicked");
            let app_handle = app.clone();
            tauri::async_runtime::spawn(async move {
                let _ = crate::commands::open_dashboard(app_handle);
            });
        }
        "stop-recording" => {
            info!("Menu: Stop Recording clicked");
            let app_handle = app.clone();
            tauri::async_runtime::spawn(async move {
                #[cfg(target_os = "macos")]
                {
                    let state = app_handle.state::<AppState>();
                    let session = {
                        let mut guard = state.session.lock().unwrap_or_else(|e| e.into_inner());
                        guard.take()
                    };
                    if let Some(s) = session {
                        let meeting_id = s.meeting_id.clone();
                        s.stop();
                        set_tray_recording(&app_handle, false).ok();
                        update_tray_menu(&app_handle).ok();
                        use tauri_plugin_opener::OpenerExt;
                        let url = config::meeting_url(&meeting_id);
                        app_handle.opener().open_url(&url, None::<&str>).ok();
                    }
                }
            });
        }
        "open-dashboard" | "app-name" => {
            info!("Menu: Open Dashboard clicked");
            let app_handle = app.clone();
            tauri::async_runtime::spawn(async move {
                let _ = crate::commands::open_dashboard(app_handle);
            });
        }
        "settings" => {
            info!("Menu: Settings clicked");
            show_settings_window(app);
        }
        "check-updates" => {
            info!("Menu: Check for Updates clicked");
            let app_handle = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = crate::updater::check_for_updates(&app_handle).await {
                    warn!("Update check failed: {e}");
                }
            });
        }
        _ => {
            #[cfg(target_os = "macos")]
            handle_shadow_menu_event(app, event_id);
        }
    }
}

#[cfg(target_os = "macos")]
fn handle_shadow_menu_event<R: Runtime>(app: &AppHandle<R>, event_id: &str) {
    match event_id {
        "start-shadow" => {
            info!("Menu: Start Shadow Recording clicked");
            let app_handle = app.clone();
            tauri::async_runtime::spawn(async move {
                let is_authed = crate::auth::keychain::get_token(&app_handle)
                    .map(|t| crate::auth::keychain::is_token_valid(&t))
                    .unwrap_or(false);
                if !is_authed {
                    let _ = crate::commands::open_dashboard(app_handle);
                    return;
                }

                let state = app_handle.state::<AppState>();
                let can_start = {
                    let session_active = state
                        .session
                        .lock()
                        .map(|g| g.is_some())
                        .unwrap_or(false);
                    let shadow_active = state
                        .shadow_session
                        .lock()
                        .map(|g| g.is_some())
                        .unwrap_or(false);
                    !session_active && !shadow_active
                };

                if !can_start {
                    warn!("Cannot start shadow: recording or shadow already active");
                    return;
                }

                match crate::shadow::ShadowSession::start(None, None) {
                    Ok(session) => {
                        {
                            let mut guard = state
                                .shadow_session
                                .lock()
                                .unwrap_or_else(|e| e.into_inner());
                            *guard = Some(session);
                        }
                        set_tray_shadow(&app_handle).ok();
                        update_tray_menu(&app_handle).ok();
                        start_shadow_tray_updater(&app_handle);
                        info!("Shadow recording started from tray");
                    }
                    Err(e) => {
                        warn!("Failed to start shadow recording from tray: {e}");
                    }
                }
            });
        }
        "shadow-save-record" => {
            info!("Menu: Shadow Save & Record clicked");
            let app_handle = app.clone();
            tauri::async_runtime::spawn(async move {
                let state = app_handle.state::<AppState>();
                let session = {
                    let mut guard = state
                        .shadow_session
                        .lock()
                        .unwrap_or_else(|e| e.into_inner());
                    guard.take()
                };
                if let Some(session) = session {
                    clear_tray_title(&app_handle);
                    match crate::shadow::save::save_and_record(
                        session,
                        &app_handle,
                        None,
                        None,
                        None,
                    )
                    .await
                    {
                        Ok(meeting_id) => {
                            {
                                let mut guard = state
                                    .pending_meeting_id
                                    .lock()
                                    .unwrap_or_else(|e| e.into_inner());
                                *guard = Some(meeting_id.clone());
                            }
                            set_tray_recording(&app_handle, false).ok();
                            update_tray_menu(&app_handle).ok();
                            let _ = crate::commands::open_dashboard(app_handle.clone());
                            info!("Shadow Save & Record complete: {meeting_id}");
                        }
                        Err(e) => {
                            warn!("Shadow Save & Record failed: {e}");
                            update_tray_menu(&app_handle).ok();
                        }
                    }
                }
            });
        }
        "shadow-save-only" => {
            info!("Menu: Shadow Save Only clicked");
            let app_handle = app.clone();
            tauri::async_runtime::spawn(async move {
                let state = app_handle.state::<AppState>();
                let session = {
                    let mut guard = state
                        .shadow_session
                        .lock()
                        .unwrap_or_else(|e| e.into_inner());
                    guard.take()
                };
                if let Some(session) = session {
                    clear_tray_title(&app_handle);
                    match crate::shadow::save::save_only(
                        session,
                        &app_handle,
                        None,
                        None,
                        None,
                    )
                    .await
                    {
                        Ok(meeting_id) => {
                            set_tray_recording(&app_handle, false).ok();
                            update_tray_menu(&app_handle).ok();
                            use tauri_plugin_opener::OpenerExt;
                            let url = config::meeting_url(&meeting_id);
                            app_handle.opener().open_url(&url, None::<&str>).ok();
                            info!("Shadow Save Only complete: {meeting_id}");
                        }
                        Err(e) => {
                            warn!("Shadow Save Only failed: {e}");
                            update_tray_menu(&app_handle).ok();
                        }
                    }
                }
            });
        }
        "shadow-save-continue" => {
            info!("Menu: Shadow Save & Continue clicked");
            let app_handle = app.clone();
            tauri::async_runtime::spawn(async move {
                let state = app_handle.state::<AppState>();
                let buffer = {
                    let guard = state
                        .shadow_session
                        .lock()
                        .unwrap_or_else(|e| e.into_inner());
                    guard
                        .as_ref()
                        .map(|s| std::sync::Arc::clone(&s.buffer))
                };
                if let Some(buffer) = buffer {
                    match crate::shadow::save::save_and_continue(
                        buffer,
                        &app_handle,
                        None,
                        None,
                        None,
                    )
                    .await
                    {
                        Ok(meeting_id) => {
                            info!("Shadow Save & Continue complete: {meeting_id}");
                        }
                        Err(e) => {
                            warn!("Shadow Save & Continue failed: {e}");
                        }
                    }
                }
            });
        }
        "shadow-stop" => {
            info!("Menu: Stop Shadow clicked");
            let app_handle = app.clone();
            tauri::async_runtime::spawn(async move {
                let state = app_handle.state::<AppState>();
                let session = {
                    let mut guard = state
                        .shadow_session
                        .lock()
                        .unwrap_or_else(|e| e.into_inner());
                    guard.take()
                };
                if let Some(s) = session {
                    s.stop();
                    clear_tray_title(&app_handle);
                    set_tray_recording(&app_handle, false).ok();
                    update_tray_menu(&app_handle).ok();
                    info!("Shadow recording stopped from tray");
                }
            });
        }
        _ => {}
    }
}
