pub mod audio;
pub mod auth;
pub mod calendar;
mod commands;
pub mod config;
pub mod error;
pub mod meeting_detector;
pub mod permissions;
pub mod recording;
mod tray;
pub mod updater;
#[cfg(target_os = "macos")]
pub mod shadow;
#[cfg(target_os = "macos")]
pub mod upload;

use recording::AppState;
use tauri::Manager;
use tauri_plugin_deep_link::DeepLinkExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tracing::{info, warn};

pub fn run() {
    init_logging();
    info!("Starting Laconote Desktop");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_global_shortcut::Builder::default().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::default().build())
        .plugin(tauri_plugin_deep_link::init())
        .manage(AppState::default())
        .setup(|app| {
            let handle = app.handle().clone();

            #[cfg(target_os = "macos")]
            {
                // Just hide dock icon — don't request mic permission at startup.
                // Mic will be requested when user actually starts recording.
                extern "C" { fn set_accessory_policy(); }
                unsafe { set_accessory_policy(); }
            }

            tray::setup_tray(&handle)?;
            setup_dashboard_window(app)?;
            setup_deep_links(app, &handle)?;
            register_global_shortcut(&handle)?;

            #[cfg(target_os = "macos")]
            setup_scheduler(&handle);

            setup_updater(&handle);
            calendar::scheduler::start_calendar_scheduler(&handle);
            meeting_detector::start_detector(&handle);

            // Don't auto-show dashboard — just notify user via tray.
            // User clicks tray icon when ready.
            {
                let token = auth::keychain::get_token(&handle);
                if token.is_none() {
                    info!("No auth token found — user can log in via tray icon");
                    use tauri_plugin_notification::NotificationExt;
                    let _ = handle
                        .notification()
                        .builder()
                        .title("Laconote")
                        .body("Click the menu bar icon to log in and start recording.")
                        .show();
                }
            }

            info!("Laconote Desktop initialized");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_recording_status,
            commands::get_auth_status,
            commands::get_token,
            commands::save_token,
            commands::login,
            commands::logout,
            commands::open_dashboard,
            commands::start_recording,
            commands::stop_recording,
            commands::list_audio_devices,
            commands::get_audio_levels,
            commands::check_permissions,
            commands::open_system_settings,
            commands::request_mic_permission,
            commands::probe_system_audio_capture,
            commands::get_audio_diagnostic,
            commands::restart_app,
            commands::check_for_updates,
            commands::log_dashboard_event,
            commands::start_shadow_recording,
            commands::stop_shadow_recording,
            commands::get_shadow_status,
            commands::save_shadow_buffer,
            commands::get_shadow_schedule,
            commands::set_shadow_schedule,
            commands::set_shadow_buffer_duration,
            commands::get_calendar_status,
            commands::connect_google_calendar,
            commands::disconnect_calendar,
            commands::set_calendar_config,
            commands::get_upcoming_events,
            commands::get_detector_config,
            commands::set_detector_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Laconote Desktop");
}

fn setup_dashboard_window(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.handle().clone();
    let dashboard_url: url::Url = config::DASHBOARD_URL
        .parse()
        .expect("Invalid dashboard URL");
    tauri::WebviewWindowBuilder::new(
        app,
        "dashboard",
        tauri::WebviewUrl::External(dashboard_url),
    )
    .title("Laconote")
    .inner_size(1280.0, 800.0)
    .min_inner_size(900.0, 600.0)
    .resizable(true)
    .visible(false)
    .decorations(true)
    .center()
    .initialization_script(include_str!("dashboard_diagnostics.js"))
    .on_navigation(move |url| {
        if url.scheme() == "laconote" {
            info!("Intercepted laconote:// URL in dashboard webview: {}", url);
            auth::deeplink::handle_deep_link(&handle, url.as_str());
            return false;
        }
        true
    })
    .build()
    .map_err(|e| {
        warn!("Failed to create dashboard window: {e}");
        e
    })?;
    Ok(())
}

fn setup_deep_links(
    app: &tauri::App,
    handle: &tauri::AppHandle,
) -> Result<(), Box<dyn std::error::Error>> {
    let dl_handle = handle.clone();
    app.deep_link().on_open_url(move |event| {
        for url in event.urls() {
            auth::deeplink::handle_deep_link(&dl_handle, url.as_str());
        }
    });

    let initial_handle = handle.clone();
    if let Ok(Some(urls)) = app.deep_link().get_current() {
        for url in urls {
            auth::deeplink::handle_deep_link(&initial_handle, url.as_str());
        }
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn setup_scheduler(handle: &tauri::AppHandle) {
    shadow::cleanup::run_cleanup();

    let sched_handle = handle.clone();
    let config = shadow::schedule::load_schedule_config(&sched_handle);
    if config.enabled {
        let _scheduler = shadow::schedule::ShadowScheduler::start(&sched_handle);
        std::mem::forget(_scheduler);

        if shadow::schedule::is_within_window(&config, &chrono::Local::now()) {
            let auto_handle = sched_handle.clone();
            tauri::async_runtime::spawn(async move {
                let is_authed = crate::auth::keychain::get_token(&auto_handle)
                    .map(|t| crate::auth::keychain::is_token_valid(&t))
                    .unwrap_or(false);
                if is_authed {
                    let state = auto_handle.state::<AppState>();
                    let can_start = {
                        let no_rec = state.session.lock().map(|g| g.is_none()).unwrap_or(true);
                        let no_shadow = state.shadow_session.lock().map(|g| g.is_none()).unwrap_or(true);
                        no_rec && no_shadow
                    };
                    if can_start {
                        match shadow::ShadowSession::start(None, Some(config.buffer_minutes)) {
                            Ok(session) => {
                                {
                                    let mut guard = state.shadow_session.lock().unwrap_or_else(|e| e.into_inner());
                                    *guard = Some(session);
                                }
                                if let Ok(store) = tauri_plugin_store::StoreBuilder::new(&auto_handle, "app-settings.json").build() {
                                    store.set("shadow_auto_started", true);
                                }
                                tray::set_tray_shadow(&auto_handle).ok();
                                tray::update_tray_menu(&auto_handle).ok();
                                tray::start_shadow_tray_updater(&auto_handle);
                                info!("Shadow recording auto-started on launch (within schedule window)");
                            }
                            Err(e) => {
                                warn!("Failed to auto-start shadow on launch: {e}");
                            }
                        }
                    }
                }
            });
        }
    }
}

fn setup_updater(handle: &tauri::AppHandle) {
    let update_handle = handle.clone();
    tauri::async_runtime::spawn(async move {
        updater::check_for_updates_silent(&update_handle).await;

        let mut interval =
            tokio::time::interval(std::time::Duration::from_secs(6 * 60 * 60));
        interval.tick().await;
        loop {
            interval.tick().await;
            updater::check_for_updates_silent(&update_handle).await;
        }
    });
}

fn register_global_shortcut<R: tauri::Runtime>(
    handle: &tauri::AppHandle<R>,
) -> Result<(), Box<dyn std::error::Error>> {
    let shadow_handle = handle.clone();
    handle
        .global_shortcut()
        .on_shortcut("CommandOrControl+Shift+S", move |_app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                #[cfg(target_os = "macos")]
                {
                    let state = shadow_handle.state::<AppState>();
                    let shadow_active = state
                        .shadow_session
                        .lock()
                        .map(|g| g.is_some())
                        .unwrap_or(false);

                    if shadow_active {
                        info!("Global shortcut ⌘⇧S: triggering Save & Continue");
                        let h = shadow_handle.clone();
                        tauri::async_runtime::spawn(async move {
                            let st = h.state::<AppState>();
                            let buffer = {
                                let guard = st
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
                                    &h,
                                    None,
                                    None,
                                    None,
                                )
                                .await
                                {
                                    Ok(meeting_id) => {
                                        info!("Save & Continue via ⌘⇧S: {meeting_id}");
                                    }
                                    Err(e) => {
                                        warn!("Save & Continue via ⌘⇧S failed: {e}");
                                    }
                                }
                            }
                        });
                    }
                }
            }
        })
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    info!("Global shortcut registered: CommandOrControl+Shift+S");

    let shortcut_handle = handle.clone();

    handle
        .global_shortcut()
        .on_shortcut("CommandOrControl+Shift+R", move |_app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                let state = shortcut_handle.state::<AppState>();
                let is_recording = {
                    #[cfg(target_os = "macos")]
                    {
                        state
                            .session
                            .lock()
                            .map(|g: std::sync::MutexGuard<Option<recording::RecordingSession>>| {
                                g.is_some()
                            })
                            .unwrap_or(false)
                    }
                    #[cfg(not(target_os = "macos"))]
                    false
                };

                if is_recording {
                    let h = shortcut_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        #[cfg(target_os = "macos")]
                        {
                            let st = h.state::<AppState>();
                            let session = {
                                let mut guard = st.session.lock().unwrap();
                                guard.take()
                            };
                            if let Some(s) = session {
                                let meeting_id = s.meeting_id.clone();
                                s.stop();
                                tray::set_tray_recording(&h, false).ok();
                                tray::update_tray_menu(&h).ok();
                                use tauri_plugin_opener::OpenerExt;
                                let url = config::meeting_url(&meeting_id);
                                h.opener().open_url(&url, None::<&str>).ok();
                                info!("Recording stopped via global shortcut");
                            }
                        }
                    });
                } else {
                    info!("Global shortcut: show recording dialog");
                    tray::show_recording_dialog(&shortcut_handle);
                }
            }
        })
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    info!("Global shortcut registered: CommandOrControl+Shift+R");
    Ok(())
}

fn init_logging() {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let log_dir = dirs_next_log_dir();
    let file_appender = tracing_appender::rolling::daily(&log_dir, "laconote-desktop.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    std::mem::forget(_guard);

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,laconote_desktop=debug"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt::layer().with_writer(std::io::stdout).with_ansi(true))
        .with(fmt::layer().with_writer(non_blocking).with_ansi(false))
        .init();
}

fn dirs_next_log_dir() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let log_dir = std::path::PathBuf::from(home)
        .join("Library")
        .join("Logs")
        .join("com.laconote.desktop");
    std::fs::create_dir_all(&log_dir).ok();
    log_dir
}
