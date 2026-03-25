use std::collections::HashSet;
use std::sync::Mutex;
use tauri::{AppHandle, Runtime};
use tracing::{info, warn};

use super::{google, CalendarConfig, CalendarEvent};

const POLL_INTERVAL_SECS: u64 = 60;
const LOOKAHEAD_MINUTES: i64 = 30;
const STORE_FILE: &str = "app-settings.json";

static NOTIFIED_EVENTS: Mutex<Option<HashSet<String>>> = Mutex::new(None);

fn was_notified(event_id: &str) -> bool {
    let guard = NOTIFIED_EVENTS.lock().unwrap_or_else(|e| e.into_inner());
    guard.as_ref().is_some_and(|s| s.contains(event_id))
}

fn mark_notified(event_id: &str) {
    let mut guard = NOTIFIED_EVENTS.lock().unwrap_or_else(|e| e.into_inner());
    let set = guard.get_or_insert_with(HashSet::new);
    set.insert(event_id.to_string());
    // Prune old entries if too many
    if set.len() > 100 {
        set.clear();
    }
}

pub fn load_config<R: Runtime>(app: &AppHandle<R>) -> CalendarConfig {
    let Ok(store) = tauri_plugin_store::StoreBuilder::new(app, STORE_FILE).build() else {
        return CalendarConfig::default();
    };
    CalendarConfig {
        enabled: store.get("calendar_enabled").and_then(|v| v.as_bool()).unwrap_or(false),
        provider: store.get("calendar_provider").and_then(|v| v.as_str().map(String::from)).unwrap_or_else(|| "google".into()),
        remind_minutes_before: store.get("calendar_remind_minutes").and_then(|v| v.as_u64()).unwrap_or(2) as u32,
        auto_record: store.get("calendar_auto_record").and_then(|v| v.as_bool()).unwrap_or(false),
    }
}

pub fn save_config<R: Runtime>(app: &AppHandle<R>, config: &CalendarConfig) {
    let Ok(store) = tauri_plugin_store::StoreBuilder::new(app, STORE_FILE).build() else {
        warn!("Failed to open store for calendar config");
        return;
    };
    store.set("calendar_enabled", config.enabled);
    store.set("calendar_provider", config.provider.clone());
    store.set("calendar_remind_minutes", config.remind_minutes_before as u64);
    store.set("calendar_auto_record", config.auto_record);
    let _ = store.save();
}

pub fn load_google_tokens<R: Runtime>(app: &AppHandle<R>) -> Option<google::GoogleTokens> {
    let store = tauri_plugin_store::StoreBuilder::new(app, STORE_FILE).build().ok()?;
    let access_token = store.get("google_access_token")?.as_str()?.to_string();
    let refresh_token = store.get("google_refresh_token").and_then(|v| v.as_str().map(String::from));
    let expires_at = store.get("google_token_expires_at").and_then(|v| v.as_i64()).unwrap_or(0);
    Some(google::GoogleTokens { access_token, refresh_token, expires_at })
}

pub fn save_google_tokens<R: Runtime>(app: &AppHandle<R>, tokens: &google::GoogleTokens) {
    let Ok(store) = tauri_plugin_store::StoreBuilder::new(app, STORE_FILE).build() else {
        return;
    };
    store.set("google_access_token", tokens.access_token.clone());
    if let Some(rt) = &tokens.refresh_token {
        store.set("google_refresh_token", rt.clone());
    }
    store.set("google_token_expires_at", tokens.expires_at);
    let _ = store.save();
}

pub fn clear_google_tokens<R: Runtime>(app: &AppHandle<R>) {
    let Ok(store) = tauri_plugin_store::StoreBuilder::new(app, STORE_FILE).build() else {
        return;
    };
    let _ = store.delete("google_access_token");
    let _ = store.delete("google_refresh_token");
    let _ = store.delete("google_token_expires_at");
    let _ = store.save();
}

pub fn start_calendar_scheduler<R: Runtime + 'static>(app: &AppHandle<R>) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        info!("Calendar scheduler started (poll every {POLL_INTERVAL_SECS}s)");
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(POLL_INTERVAL_SECS));

        loop {
            interval.tick().await;

            let config = load_config(&app);
            if !config.enabled {
                continue;
            }

            let Some(mut tokens) = load_google_tokens(&app) else {
                continue;
            };

            // Refresh token if expired
            let now = chrono::Utc::now().timestamp();
            if now >= tokens.expires_at - 60 {
                if let Some(refresh_token) = &tokens.refresh_token {
                    match google::refresh_access_token(refresh_token).await {
                        Ok(new_tokens) => {
                            save_google_tokens(&app, &new_tokens);
                            tokens = new_tokens;
                        }
                        Err(e) => {
                            warn!("Failed to refresh Google token: {e}");
                            continue;
                        }
                    }
                } else {
                    warn!("Google token expired and no refresh token available");
                    continue;
                }
            }

            // Fetch upcoming events
            match google::fetch_upcoming_events(&tokens.access_token, LOOKAHEAD_MINUTES).await {
                Ok(events) => {
                    process_events(&app, &events, &config);
                }
                Err(e) => {
                    warn!("Failed to fetch calendar events: {e}");
                }
            }
        }
    });
}

fn process_events<R: Runtime>(app: &AppHandle<R>, events: &[CalendarEvent], config: &CalendarConfig) {
    use tauri::Emitter;

    // Emit events to frontend for display
    let _ = app.emit("calendar-events", events);

    for event in events {
        let minutes = event.minutes_until();

        // Only notify for events with meeting links
        if event.meeting_link.is_none() {
            continue;
        }

        // Notify if within reminder window and not already notified
        if minutes <= config.remind_minutes_before as i64 && minutes >= -1 && !was_notified(&event.id) {
            mark_notified(&event.id);
            send_meeting_notification(app, event);
        }
    }
}

fn send_meeting_notification<R: Runtime>(app: &AppHandle<R>, event: &CalendarEvent) {
    use tauri_plugin_notification::NotificationExt;

    let platform_str = event.platform.as_ref().map(|p| format!(" ({:?})", p)).unwrap_or_default();
    let minutes = event.minutes_until();
    let time_str = if minutes <= 0 {
        "starting now".to_string()
    } else if minutes == 1 {
        "in 1 minute".to_string()
    } else {
        format!("in {} minutes", minutes)
    };

    let body = format!(
        "{}{} — {}\nClick the tray icon to start recording.",
        event.summary, platform_str, time_str
    );

    let _ = app
        .notification()
        .builder()
        .title("Upcoming Meeting")
        .body(&body)
        .show();

    info!(
        event_id = %event.id,
        summary = %event.summary,
        minutes_until = minutes,
        "Meeting reminder notification sent"
    );
}
