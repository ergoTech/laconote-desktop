use tauri::{image::Image, AppHandle, Manager, Runtime};
use tracing::info;

pub fn load_icon<R: Runtime>(app: &AppHandle<R>, name: &str) -> tauri::Result<Image<'static>> {
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

pub fn create_fallback_icon() -> Image<'static> {
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

pub fn clear_tray_title<R: Runtime>(app: &AppHandle<R>) {
    if let Some(tray) = app.tray_by_id("main-tray") {
        #[cfg(target_os = "macos")]
        {
            tray.set_title(Some("")).ok();
        }
        let _ = tray;
    }
}
