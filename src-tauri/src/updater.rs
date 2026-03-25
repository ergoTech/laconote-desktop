use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;
use tracing::{info, warn};

pub async fn check_for_updates<R: tauri::Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    info!("Checking for updates...");

    let updater = app.updater().map_err(|e| {
        tauri::Error::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            e.to_string(),
        ))
    })?;

    match updater.check().await {
        Ok(Some(update)) => {
            info!(
                "Update available: {} → {}",
                update.current_version, update.version
            );
            if let Err(e) = update.download_and_install(|_, _| {}, || {}).await {
                warn!("Update installation failed: {e}");
            }
        }
        Ok(None) => {
            info!("App is up to date");
        }
        Err(e) => {
            warn!("Update check error: {e}");
        }
    }

    Ok(())
}

pub async fn check_for_updates_silent<R: tauri::Runtime>(app: &AppHandle<R>) {
    if let Err(e) = check_for_updates(app).await {
        warn!("Background update check failed: {e}");
    }
}
