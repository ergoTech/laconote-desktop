mod events;
pub mod icons;
pub mod menu;
pub mod shadow_updater;
pub mod windows;

use tauri::{
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Runtime,
};
use tracing::{debug, info};

pub use icons::{clear_tray_title as clear_tray_title_pub, set_tray_recording, set_tray_shadow, set_tray_warning};
pub use menu::update_tray_menu;
#[cfg(target_os = "macos")]
pub use shadow_updater::start_shadow_tray_updater;

pub fn setup_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let idle_icon = icons::load_icon(app, "tray-idle")?;

    let menu = menu::build_current_tray_menu(app)?;

    let _tray = TrayIconBuilder::with_id("main-tray")
        .icon(idle_icon)
        .icon_as_template(true)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            events::handle_menu_event(app, &event.id.0);
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                use tauri::Manager;
                let app = tray.app_handle();
                debug!("Tray left-clicked");
                // Toggle: if dashboard visible → hide, otherwise → show
                if let Some(window) = app.get_webview_window("dashboard") {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        let _ = crate::commands::open_dashboard(app.clone());
                    }
                } else {
                    let _ = crate::commands::open_dashboard(app.clone());
                }
            }
        })
        .build(app)?;

    info!("Tray icon initialized");
    Ok(())
}
