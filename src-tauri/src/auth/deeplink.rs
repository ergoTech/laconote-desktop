use tauri::{AppHandle, Emitter, Manager, Runtime};
use tracing::{info, warn};
use url::Url;

use super::keychain;

const AUTH_CALLBACK_HOST: &str = "auth";
const AUTH_CALLBACK_PATH: &str = "/callback";

pub fn handle_deep_link<R: Runtime>(app: &AppHandle<R>, raw_url: &str) {
    info!("Deep link received: {raw_url}");

    let parsed = match Url::parse(raw_url) {
        Ok(u) => u,
        Err(e) => {
            warn!("Failed to parse deep link URL '{raw_url}': {e}");
            return;
        }
    };

    if parsed.scheme() != "laconote" {
        warn!("Unexpected URL scheme: {}", parsed.scheme());
        return;
    }

    let host = parsed.host_str().unwrap_or("");
    let path = parsed.path();

    if host == AUTH_CALLBACK_HOST && path == AUTH_CALLBACK_PATH {
        handle_auth_callback(app, &parsed);
    } else if host == "calendar" && path == "/callback" {
        handle_calendar_callback(app, &parsed);
    } else {
        warn!("Unhandled deep link path: {host}{path}");
    }
}

fn handle_auth_callback<R: Runtime>(app: &AppHandle<R>, url: &Url) {
    let token = url
        .query_pairs()
        .find(|(k, _)| k == "token")
        .map(|(_, v)| v.into_owned());

    let jwt = match token {
        Some(t) => t,
        None => {
            warn!("Auth callback missing 'token' query parameter");
            return;
        }
    };

    if !keychain::is_token_valid(&jwt) {
        warn!("Auth callback received an expired or malformed JWT");
        return;
    }

    match keychain::store_token(app, &jwt) {
        Ok(()) => {
            info!("Authentication successful, JWT stored");
            notify_auth_success(app, jwt);
        }
        Err(e) => {
            warn!("Failed to store JWT: {e}");
        }
    }
}

fn notify_auth_success<R: Runtime>(app: &AppHandle<R>, token: String) {
    let emit_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
        if let Err(e) = emit_handle.emit("auth-changed", ()) {
            warn!("Failed to emit auth-changed event: {e}");
        }

        if let Some(window) = emit_handle.get_webview_window("dashboard") {
            let login_url = format!("https://laconote.com/login?token={token}");
            let _ = window.eval(&format!("window.location.href = '{login_url}'"));
            let _ = window.show();
            let _ = window.set_focus();
        }

    });

    use tauri_plugin_notification::NotificationExt;
    let _ = app
        .notification()
        .builder()
        .title("Laconote")
        .body("You are now signed in. Click the menu bar icon to start recording.")
        .show();
}

fn handle_calendar_callback<R: Runtime>(app: &AppHandle<R>, url: &Url) {
    let code = url
        .query_pairs()
        .find(|(k, _)| k == "code")
        .map(|(_, v)| v.into_owned());

    let Some(code) = code else {
        warn!("Calendar callback missing 'code' query parameter");
        return;
    };

    info!("Calendar OAuth callback received, exchanging code");
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        match crate::calendar::google::exchange_code(&code).await {
            Ok(tokens) => {
                crate::calendar::scheduler::save_google_tokens(&app, &tokens);
                let mut config = crate::calendar::scheduler::load_config(&app);
                config.enabled = true;
                crate::calendar::scheduler::save_config(&app, &config);

                info!("Google Calendar connected via deep link");
                let _ = app.emit("calendar-connected", ());

                use tauri_plugin_notification::NotificationExt;
                let _ = app
                    .notification()
                    .builder()
                    .title("Laconote")
                    .body("Google Calendar connected. You'll get reminders before meetings.")
                    .show();
            }
            Err(e) => {
                warn!("Calendar OAuth code exchange failed: {e}");
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_callback_url() {
        let raw = "laconote://auth/callback?token=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ1c2VyQGV4YW1wbGUuY29tIiwiZXhwIjo5OTk5OTk5OTk5fQ.signature";
        let parsed = Url::parse(raw).unwrap();
        assert_eq!(parsed.scheme(), "laconote");
        assert_eq!(parsed.host_str(), Some("auth"));
        assert_eq!(parsed.path(), "/callback");
        let token = parsed
            .query_pairs()
            .find(|(k, _)| k == "token")
            .map(|(_, v)| v.into_owned());
        assert!(token.is_some());
    }

    #[test]
    fn test_parse_callback_url_missing_token() {
        let raw = "laconote://auth/callback";
        let parsed = Url::parse(raw).unwrap();
        let token = parsed
            .query_pairs()
            .find(|(k, _)| k == "token")
            .map(|(_, v)| v.into_owned());
        assert!(token.is_none());
    }
}
