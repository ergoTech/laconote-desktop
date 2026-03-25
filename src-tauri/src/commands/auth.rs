use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Runtime};
use tracing::info;

use crate::auth::keychain;
use crate::config::LOGIN_URL;
use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthStatus {
    pub is_authenticated: bool,
    pub user_email: Option<String>,
}

#[tauri::command]
pub fn get_auth_status<R: Runtime>(app: AppHandle<R>) -> AuthStatus {
    match keychain::get_token(&app) {
        Some(token) if keychain::is_token_valid(&token) => AuthStatus {
            is_authenticated: true,
            user_email: keychain::extract_email(&token),
        },
        _ => AuthStatus {
            is_authenticated: false,
            user_email: None,
        },
    }
}

#[tauri::command]
pub async fn login<R: Runtime>(app: AppHandle<R>) -> Result<(), AppError> {
    use tauri_plugin_opener::OpenerExt;
    info!("Opening system browser for Google OAuth login");
    app.opener()
        .open_url(LOGIN_URL, None::<&str>)
        .map_err(|e| AppError::Internal(format!("Failed to open browser: {e}")))?;
    Ok(())
}

#[tauri::command]
pub fn logout<R: Runtime>(app: AppHandle<R>) -> Result<(), AppError> {
    info!("Logging out");
    keychain::delete_token(&app).map_err(|e| AppError::Internal(e))
}

#[tauri::command]
pub fn get_token<R: Runtime>(app: AppHandle<R>) -> Option<String> {
    keychain::get_token(&app)
}

#[tauri::command]
pub fn save_token<R: Runtime>(app: AppHandle<R>, token: String) -> Result<(), AppError> {
    use tauri::Emitter;
    info!("Web app pushed a token for native storage");
    keychain::store_token(&app, &token).map_err(|e| AppError::Internal(e))?;
    app.emit("auth-changed", ())
        .map_err(|e| AppError::Internal(format!("Failed to emit auth-changed: {e}")))?;

    Ok(())
}
