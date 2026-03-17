use crate::recording::AppState;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Runtime,
};
use tracing::{debug, info, warn};

pub fn setup_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let idle_icon = load_icon(app, "tray-idle")?;

    let menu = build_current_tray_menu(app)?;

    let _tray = TrayIconBuilder::with_id("main-tray")
        .icon(idle_icon)
        .icon_as_template(true)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            handle_menu_event(app, &event.id.0);
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                debug!("Tray left-clicked");
                let _ = crate::commands::open_dashboard(app.clone());
            }
        })
        .build(app)?;

    info!("Tray icon initialized");
    Ok(())
}

fn build_current_tray_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let state = app.state::<AppState>();

    let is_recording = {
        #[cfg(target_os = "macos")]
        {
            state.session.lock().map(|g| g.is_some()).unwrap_or(false)
        }
        #[cfg(not(target_os = "macos"))]
        false
    };

    #[cfg(target_os = "macos")]
    let shadow_active = state
        .shadow_session
        .lock()
        .map(|g| g.is_some())
        .unwrap_or(false);
    #[cfg(not(target_os = "macos"))]
    let shadow_active = false;

    #[cfg(target_os = "macos")]
    let shadow_label: Option<String> = if shadow_active {
        state
            .shadow_session
            .lock()
            .ok()
            .and_then(|g| {
                g.as_ref().map(|s| {
                    let status = s.status();
                    let total_ms = status.total_meeting_duration_ms;
                    let mins = total_ms / 60_000;
                    let secs = (total_ms % 60_000) / 1000;
                    format!("Shadow: {:02}:{:02} ({}%)", mins, secs, status.fill_percent)
                })
            })
    } else {
        None
    };

    let menu = Menu::new(app)?;

    let app_name = MenuItem::with_id(app, "app-name", "Laconote", false, None::<&str>)?;
    menu.append(&app_name)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;

    #[cfg(target_os = "macos")]
    {
        if shadow_active {
            if let Some(ref label) = shadow_label {
                let status_item =
                    MenuItem::with_id(app, "shadow-status", label, false, None::<&str>)?;
                menu.append(&status_item)?;
            }
            let save_record = MenuItem::with_id(
                app,
                "shadow-save-record",
                "Save & Record",
                true,
                None::<&str>,
            )?;
            let save_only = MenuItem::with_id(
                app,
                "shadow-save-only",
                "Save Only",
                true,
                None::<&str>,
            )?;
            let save_continue = MenuItem::with_id(
                app,
                "shadow-save-continue",
                "Save & Continue",
                true,
                None::<&str>,
            )?;
            let stop_shadow =
                MenuItem::with_id(app, "shadow-stop", "Stop Shadow", true, None::<&str>)?;
            menu.append(&save_record)?;
            menu.append(&save_only)?;
            menu.append(&save_continue)?;
            menu.append(&stop_shadow)?;
            menu.append(&PredefinedMenuItem::separator(app)?)?;
        } else if !is_recording {
            let start_shadow = MenuItem::with_id(
                app,
                "start-shadow",
                "Start Shadow Recording",
                true,
                None::<&str>,
            )?;
            menu.append(&start_shadow)?;
            menu.append(&PredefinedMenuItem::separator(app)?)?;
        }
    }

    let start_recording = MenuItem::with_id(
        app,
        "start-recording",
        "Start Recording...",
        !is_recording && !shadow_active,
        None::<&str>,
    )?;
    let stop_recording = MenuItem::with_id(
        app,
        "stop-recording",
        "Stop Recording",
        is_recording,
        None::<&str>,
    )?;
    menu.append(&start_recording)?;
    menu.append(&stop_recording)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;

    let open_dashboard =
        MenuItem::with_id(app, "open-dashboard", "Open Dashboard", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings...", true, None::<&str>)?;
    let check_updates =
        MenuItem::with_id(app, "check-updates", "Check for Updates", true, None::<&str>)?;
    menu.append(&open_dashboard)?;
    menu.append(&settings)?;
    menu.append(&check_updates)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;

    let quit = PredefinedMenuItem::quit(app, Some("Quit Laconote"))?;
    menu.append(&quit)?;

    Ok(menu)
}

fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, event_id: &str) {
    match event_id {
        "start-recording" => {
            info!("Menu: Start Recording clicked");
            let is_authed = crate::auth::keychain::get_token(app)
                .map(|t| crate::auth::keychain::is_token_valid(&t))
                .unwrap_or(false);
            if is_authed {
                show_recording_dialog(app);
            } else {
                let app_handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = crate::commands::open_dashboard(app_handle);
                });
            }
        }
        "stop-recording" => {
            info!("Menu: Stop Recording clicked");
            let app_handle = app.clone();
            tauri::async_runtime::spawn(async move {
                #[cfg(target_os = "macos")]
                {
                    let state = app_handle.state::<AppState>();
                    let session = {
                        let mut guard = state.session.lock().unwrap();
                        guard.take()
                    };
                    if let Some(s) = session {
                        let meeting_id = s.meeting_id.clone();
                        s.stop();
                        set_tray_recording(&app_handle, false).ok();
                        update_tray_menu(&app_handle).ok();
                        use tauri_plugin_opener::OpenerExt;
                        let url =
                            format!("https://laconote.com/meeting/{meeting_id}");
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
                            show_recording_dialog(&app_handle);
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
                            let url =
                                format!("https://laconote.com/meeting/{meeting_id}");
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

pub fn show_recording_dialog<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("recording") {
        let _ = window.show();
        let _ = window.unminimize();
        #[cfg(target_os = "macos")]
        {
            let w = window.clone();
            let _ = app.run_on_main_thread(move || {
                use objc2_app_kit::NSApplication;
                use objc2_foundation::MainThreadMarker;
                unsafe {
                    let mtm = MainThreadMarker::new_unchecked();
                    let ns_app = NSApplication::sharedApplication(mtm);
                    ns_app.activate();
                }
                let _ = w.set_focus();
            });
        }
        #[cfg(not(target_os = "macos"))]
        let _ = window.set_focus();
    } else {
        debug!("Recording window not found");
    }
}

fn show_settings_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.show();
        let _ = window.unminimize();
        #[cfg(target_os = "macos")]
        {
            let w = window.clone();
            let _ = app.run_on_main_thread(move || {
                use objc2_app_kit::NSApplication;
                use objc2_foundation::MainThreadMarker;
                unsafe {
                    let mtm = MainThreadMarker::new_unchecked();
                    let ns_app = NSApplication::sharedApplication(mtm);
                    ns_app.activate();
                }
                let _ = w.set_focus();
            });
        }
        #[cfg(not(target_os = "macos"))]
        let _ = window.set_focus();
    } else {
        debug!("Settings window not found");
    }
}

fn load_icon<R: Runtime>(app: &AppHandle<R>, name: &str) -> tauri::Result<Image<'static>> {
    let resource_dir = app.path().resource_dir().unwrap_or_default();
    let icon_path = resource_dir.join("icons").join(format!("{}.png", name));

    if icon_path.exists() {
        return Image::from_path(&icon_path);
    }

    let fallback_path = resource_dir.join("icons").join("32x32.png");
    if fallback_path.exists() {
        return Image::from_path(&fallback_path);
    }

    Ok(create_fallback_icon())
}

fn create_fallback_icon() -> Image<'static> {
    let size = 16u32;
    let mut pixels: Vec<u8> = Vec::with_capacity((size * size * 4) as usize);
    for _ in 0..size * size {
        pixels.push(128);
        pixels.push(128);
        pixels.push(128);
        pixels.push(255);
    }
    Image::new_owned(pixels, size, size)
}

pub fn set_tray_warning<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(tray) = app.tray_by_id("main-tray") {
        let icon = load_icon(app, "tray-warning")
            .or_else(|_| load_icon(app, "tray-idle"))
            .unwrap_or_else(|_| create_fallback_icon());
        tray.set_icon(Some(icon))?;
        info!("Tray icon updated: warning state");
    }
    Ok(())
}

pub fn set_tray_recording<R: Runtime>(
    app: &AppHandle<R>,
    is_recording: bool,
) -> tauri::Result<()> {
    if let Some(tray) = app.tray_by_id("main-tray") {
        let icon_name = if is_recording {
            "tray-recording"
        } else {
            "tray-idle"
        };
        let icon = load_icon(app, icon_name)?;
        tray.set_icon(Some(icon))?;
        info!("Tray icon updated: recording={}", is_recording);
    }
    Ok(())
}

pub fn set_tray_shadow<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(tray) = app.tray_by_id("main-tray") {
        let icon = load_icon(app, "tray-shadow")
            .or_else(|_| load_icon(app, "tray-idle"))
            .unwrap_or_else(|_| create_fallback_icon());
        tray.set_icon(Some(icon))?;
        info!("Tray icon updated: shadow recording");
    }
    Ok(())
}

pub fn clear_tray_title_pub<R: Runtime>(app: &AppHandle<R>) {
    clear_tray_title(app);
}

fn clear_tray_title<R: Runtime>(app: &AppHandle<R>) {
    if let Some(tray) = app.tray_by_id("main-tray") {
        #[cfg(target_os = "macos")]
        {
            tray.set_title(Some("")).ok();
        }
        let _ = tray;
    }
}

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

pub fn update_tray_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(tray) = app.tray_by_id("main-tray") {
        let menu = build_current_tray_menu(app)?;
        tray.set_menu(Some(menu))?;
        debug!("Tray menu updated");
    }
    Ok(())
}
