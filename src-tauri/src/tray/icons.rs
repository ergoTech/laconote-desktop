use tauri::{image::Image, AppHandle, Runtime};
use tracing::{info, warn};

static ICON_IDLE: &[u8] = include_bytes!("../../icons/tray-idle@2x.png");
static ICON_RECORDING: &[u8] = include_bytes!("../../icons/tray-recording@2x.png");
static ICON_SHADOW: &[u8] = include_bytes!("../../icons/tray-shadow@2x.png");
static ICON_WARNING: &[u8] = include_bytes!("../../icons/tray-warning@2x.png");

pub fn load_icon<R: Runtime>(_app: &AppHandle<R>, name: &str) -> tauri::Result<Image<'static>> {
    let bytes = match name {
        "tray-idle" => ICON_IDLE,
        "tray-recording" => ICON_RECORDING,
        "tray-shadow" => ICON_SHADOW,
        "tray-warning" => ICON_WARNING,
        _ => {
            warn!("Unknown tray icon name: {name}, falling back to idle");
            ICON_IDLE
        }
    };
    Image::from_bytes(bytes)
}

pub fn set_tray_warning<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(tray) = app.tray_by_id("main-tray") {
        let icon = load_icon(app, "tray-warning")?;
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
        let icon = load_icon(app, "tray-shadow")?;
        tray.set_icon(Some(icon))?;
        info!("Tray icon updated: shadow recording");
    }
    Ok(())
}

pub fn clear_tray_title<R: Runtime>(app: &AppHandle<R>) {
    if let Some(tray) = app.tray_by_id("main-tray") {
        #[cfg(target_os = "macos")]
        {
            tray.set_title(Some("")).ok();
        }
        let _ = tray;
    }
}
