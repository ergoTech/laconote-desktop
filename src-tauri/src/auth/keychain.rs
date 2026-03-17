use tauri::{AppHandle, Manager, Runtime};
use std::fs;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use tracing::{debug, info, warn};

const KEYCHAIN_SERVICE: &str = "com.laconote.desktop";
const KEYCHAIN_ACCOUNT: &str = "jwt-token";

pub fn store_token<R: Runtime>(app: &AppHandle<R>, jwt: &str) -> Result<(), String> {
    match keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT) {
        Ok(entry) => {
            if let Err(e) = entry.set_password(jwt) {
                warn!("Failed to store token in keychain: {e}, falling back to token.txt");
                store_token_to_file(app, jwt)?;
            } else {
                info!("JWT stored securely in macOS Keychain");
            }
        }
        Err(e) => {
            warn!("Failed to create keychain entry: {e}, falling back to token.txt");
            store_token_to_file(app, jwt)?;
        }
    }
    Ok(())
}

fn store_token_to_file<R: Runtime>(app: &AppHandle<R>, jwt: &str) -> Result<(), String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create dir: {}", e))?;
    let path = dir.join("token.txt");
    fs::write(&path, jwt).map_err(|e| format!("Failed to write token.txt: {}", e))?;
    info!("JWT stored in fallback token.txt");
    Ok(())
}

pub fn get_token<R: Runtime>(app: &AppHandle<R>) -> Option<String> {
    match keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT) {
        Ok(entry) => match entry.get_password() {
            Ok(token) => {
                debug!("JWT retrieved from macOS Keychain");
                Some(token)
            }
            Err(_) => migrate_token_from_file(app),
        },
        Err(e) => {
            warn!("Failed to initialize keychain entry: {e}");
            migrate_token_from_file(app)
        }
    }
}

fn migrate_token_from_file<R: Runtime>(app: &AppHandle<R>) -> Option<String> {
    let dir = app.path().app_data_dir().ok()?;
    let path = dir.join("token.txt");
    if !path.exists() {
        debug!("No JWT found in keychain or app data directory");
        return None;
    }
    let token = fs::read_to_string(&path).ok()?.trim().to_string();
    if token.is_empty() {
        return None;
    }
    if let Ok(entry) = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT) {
        if entry.set_password(&token).is_ok() {
            if let Err(e) = fs::remove_file(&path) {
                warn!("Failed to remove legacy token.txt after migration: {e}");
            } else {
                info!("JWT migrated from token.txt to macOS Keychain");
            }
        } else {
            warn!("Failed to migrate token to keychain, keeping token.txt");
        }
    }
    Some(token)
}

pub fn delete_token<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    match keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT) {
        Ok(entry) => match entry.delete_credential() {
            Ok(()) => info!("JWT deleted from macOS Keychain"),
            Err(keyring::Error::NoEntry) => debug!("No JWT found in keychain to delete"),
            Err(e) => warn!("Failed to delete token from keychain: {e}"),
        },
        Err(e) => warn!("Failed to initialize keychain entry for deletion: {e}"),
    }
    if let Ok(dir) = app.path().app_data_dir() {
        let path = dir.join("token.txt");
        if path.exists() {
            let _ = fs::remove_file(&path);
        }
    }
    Ok(())
}

pub fn is_token_valid(token: &str) -> bool {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return false;
    }
    let payload_b64 = parts[1];
    let decoded = match URL_SAFE_NO_PAD.decode(payload_b64) {
        Ok(bytes) => bytes,
        Err(_) => return false,
    };
    let Ok(payload) = String::from_utf8(decoded) else {
        return false;
    };
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&payload) {
        if let Some(exp) = json.get("exp").and_then(|v| v.as_i64()) {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            return exp > now;
        }
    }
    true
}

pub fn extract_email(token: &str) -> Option<String> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let payload_b64 = parts[1];
    let decoded = URL_SAFE_NO_PAD.decode(payload_b64).ok()?;
    let payload = String::from_utf8(decoded).ok()?;
    let json: serde_json::Value = serde_json::from_str(&payload).ok()?;
    json.get("email")
        .or_else(|| json.get("sub"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_jwt_with_payload(payload: &str) -> String {
        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"HS256","typ":"JWT"}"#);
        let body = URL_SAFE_NO_PAD.encode(payload);
        format!("{header}.{body}.signature")
    }

    #[test]
    fn test_is_token_valid_malformed() {
        assert!(!is_token_valid("not.a.valid.jwt.at.all"));
        assert!(!is_token_valid("only-two.parts"));
        assert!(!is_token_valid(""));
    }

    #[test]
    fn test_is_token_valid_expired() {
        let jwt = make_jwt_with_payload(r#"{"exp":1}"#);
        assert!(!is_token_valid(&jwt));
    }

    #[test]
    fn test_is_token_valid_future() {
        let jwt = make_jwt_with_payload(r#"{"exp":9999999999}"#);
        assert!(is_token_valid(&jwt));
    }

    #[test]
    fn test_is_token_valid_no_exp_returns_true() {
        let jwt = make_jwt_with_payload(r#"{"sub":"user@example.com"}"#);
        assert!(is_token_valid(&jwt));
    }

    #[test]
    fn test_extract_email_returns_none_for_invalid() {
        assert!(extract_email("bad").is_none());
        assert!(extract_email("").is_none());
    }

    #[test]
    fn test_extract_email_from_email_field() {
        let jwt = make_jwt_with_payload(r#"{"email":"user@example.com","exp":9999999999}"#);
        assert_eq!(extract_email(&jwt), Some("user@example.com".to_string()));
    }

    #[test]
    fn test_extract_email_falls_back_to_sub() {
        let jwt = make_jwt_with_payload(r#"{"sub":"sub@example.com","exp":9999999999}"#);
        assert_eq!(extract_email(&jwt), Some("sub@example.com".to_string()));
    }

    #[test]
    fn test_keyring_manual_run() {
        println!("Testing keyring...");
        match keyring::Entry::new("com.laconote.desktop", "jwt-token") {
            Ok(entry) => {
                println!("Entry created!");
                match entry.set_password("test_token") {
                    Ok(_) => println!("Password set!"),
                    Err(e) => println!("Set password error: {:?}", e),
                }
            }
            Err(e) => println!("Entry new error: {:?}", e),
        }
    }
}
