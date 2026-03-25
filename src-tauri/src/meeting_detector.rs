use std::collections::{HashMap, HashSet};
use std::process::Command;
use std::sync::Mutex;
use std::time::Instant;
use tauri::{AppHandle, Emitter, Runtime};
use tracing::info;

const CHECK_INTERVAL_SECS: u64 = 5;
const NOTIFICATION_COOLDOWN_SECS: u64 = 1800; // 30 minutes
const STORE_FILE: &str = "app-settings.json";

/// Known meeting apps and their process names on macOS.
const MEETING_PROCESSES: &[(&str, &str)] = &[
    ("zoom.us", "Zoom"),
    ("us.zoom.xos", "Zoom"),
    ("Microsoft Teams", "Microsoft Teams"),
    ("com.microsoft.teams2", "Microsoft Teams"),
    ("Discord", "Discord"),
    ("Slack", "Slack"),
    ("Cisco Webex", "Webex"),
    ("FaceTime", "FaceTime"),
];

/// Browser-based meetings detected via window titles.
const BROWSER_MEETING_KEYWORDS: &[(&str, &str)] = &[
    ("meet.google.com", "Google Meet"),
    ("teams.microsoft.com", "Microsoft Teams"),
    ("zoom.us/j/", "Zoom"),
    ("discord.com/channels", "Discord"),
];

static KNOWN_ACTIVE: Mutex<Option<HashMap<String, Instant>>> = Mutex::new(None);

#[derive(Debug, Clone, serde::Serialize)]
pub struct MeetingDetected {
    pub app_name: String,
    pub platform: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DetectorConfig {
    pub enabled: bool,
    pub auto_record: bool,
    pub notify: bool,
}

impl Default for DetectorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            notify: true,
            auto_record: false,
        }
    }
}

pub fn load_config<R: Runtime>(app: &AppHandle<R>) -> DetectorConfig {
    let Ok(store) = tauri_plugin_store::StoreBuilder::new(app, STORE_FILE).build() else {
        return DetectorConfig::default();
    };
    DetectorConfig {
        enabled: store.get("detector_enabled").and_then(|v| v.as_bool()).unwrap_or(true),
        notify: store.get("detector_notify").and_then(|v| v.as_bool()).unwrap_or(true),
        auto_record: store.get("detector_auto_record").and_then(|v| v.as_bool()).unwrap_or(false),
    }
}

pub fn save_config<R: Runtime>(app: &AppHandle<R>, config: &DetectorConfig) {
    let Ok(store) = tauri_plugin_store::StoreBuilder::new(app, STORE_FILE).build() else {
        return;
    };
    store.set("detector_enabled", config.enabled);
    store.set("detector_notify", config.notify);
    store.set("detector_auto_record", config.auto_record);
    let _ = store.save();
}

pub fn start_detector<R: Runtime + 'static>(app: &AppHandle<R>) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        info!("Meeting detector started (check every {CHECK_INTERVAL_SECS}s)");
        let mut interval =
            tokio::time::interval(std::time::Duration::from_secs(CHECK_INTERVAL_SECS));

        loop {
            interval.tick().await;

            let config = load_config(&app);
            if !config.enabled {
                continue;
            }

            let detected = tokio::task::spawn_blocking(detect_meeting_apps)
                .await
                .unwrap_or_default();

            for meeting in detected {
                if mark_if_new(&meeting.app_name) {
                    info!(
                        app = %meeting.app_name,
                        platform = %meeting.platform,
                        "Meeting app detected"
                    );

                    let _ = app.emit("meeting-detected", &meeting);

                    if config.notify {
                        send_meeting_detected_notification(&app, &meeting);
                    }
                }
            }
        }
    });
}

fn detect_meeting_apps() -> Vec<MeetingDetected> {
    let mut results = Vec::new();

    // Check running processes via `ps`
    let Ok(output) = Command::new("ps")
        .args(["-eo", "comm="])
        .output()
    else {
        return results;
    };

    let ps_output = String::from_utf8_lossy(&output.stdout);
    let running: HashSet<&str> = ps_output.lines().map(|l| l.trim()).collect();

    for &(process_name, platform) in MEETING_PROCESSES {
        if running.iter().any(|p| p.contains(process_name)) {
            results.push(MeetingDetected {
                app_name: process_name.to_string(),
                platform: platform.to_string(),
            });
        }
    }

    // Check browser windows for meeting URLs via AppleScript
    if let Some(browser_meeting) = detect_browser_meeting() {
        results.push(browser_meeting);
    }

    results
}

fn detect_browser_meeting() -> Option<MeetingDetected> {
    // Check browser active tab URLs via AppleScript.
    // IMPORTANT: Only query browsers that are ALREADY RUNNING.
    // "tell application X" launches the app if not running!
    let browsers = [
        ("Google Chrome", "Google Chrome", "tell application \"Google Chrome\" to get URL of active tab of front window"),
        ("Safari", "Safari", "tell application \"Safari\" to get URL of front document"),
        ("Arc", "Arc", "tell application \"Arc\" to get URL of active tab of front window"),
    ];

    // Get list of running apps first
    let running_apps = Command::new("osascript")
        .args(["-e", "tell application \"System Events\" to get name of every process whose background only is false"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    for (browser, process_name, script) in browsers {
        // Skip if browser is not running — avoids launching it
        if !running_apps.contains(process_name) {
            continue;
        }

        let Ok(output) = Command::new("osascript")
            .args(["-e", script])
            .output()
        else {
            continue;
        };

        if !output.status.success() {
            continue;
        }

        let url = String::from_utf8_lossy(&output.stdout);
        let url = url.trim();

        for &(keyword, platform) in BROWSER_MEETING_KEYWORDS {
            if url.contains(keyword) {
                return Some(MeetingDetected {
                    app_name: format!("{browser} ({platform})"),
                    platform: platform.to_string(),
                });
            }
        }
    }
    None
}

fn mark_if_new(app_name: &str) -> bool {
    let mut guard = KNOWN_ACTIVE.lock().unwrap_or_else(|e| e.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);

    if let Some(last_notified) = map.get(app_name) {
        if last_notified.elapsed().as_secs() < NOTIFICATION_COOLDOWN_SECS {
            return false; // Still in cooldown
        }
    }

    map.insert(app_name.to_string(), Instant::now());

    // Prune old entries
    if map.len() > 50 {
        map.retain(|_, v| v.elapsed().as_secs() < NOTIFICATION_COOLDOWN_SECS);
    }
    true
}

fn send_meeting_detected_notification<R: Runtime>(app: &AppHandle<R>, meeting: &MeetingDetected) {
    use tauri_plugin_notification::NotificationExt;
    let _ = app
        .notification()
        .builder()
        .title("Meeting detected")
        .body(&format!(
            "{} is running. Click the tray icon to start recording.",
            meeting.platform
        ))
        .show();
}
