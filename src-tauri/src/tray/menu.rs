use crate::recording::AppState;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    AppHandle, Manager, Runtime,
};
use tracing::debug;

pub fn build_current_tray_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
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

pub fn update_tray_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(tray) = app.tray_by_id("main-tray") {
        let menu = build_current_tray_menu(app)?;
        tray.set_menu(Some(menu))?;
        debug!("Tray menu updated");
    }
    Ok(())
}
