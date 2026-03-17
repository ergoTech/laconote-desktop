use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime};
use tokio::sync::oneshot;
use tracing::{info, warn};

use crate::recording::AppState;

const SCHEDULE_CHECK_INTERVAL_SECS: u64 = 15 * 60;
const STORE_FILE: &str = "app-settings.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleConfig {
    pub enabled: bool,
    pub start_time: String,
    pub end_time: String,
    pub days_bitmask: u8,
    pub buffer_minutes: u32,
}

impl Default for ScheduleConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            start_time: "09:00".into(),
            end_time: "18:00".into(),
            days_bitmask: 0b0001_1111, // Mon-Fri
            buffer_minutes: 20,
        }
    }
}

pub fn day_bit(weekday: chrono::Weekday) -> u8 {
    match weekday {
        chrono::Weekday::Mon => 1,
        chrono::Weekday::Tue => 2,
        chrono::Weekday::Wed => 4,
        chrono::Weekday::Thu => 8,
        chrono::Weekday::Fri => 16,
        chrono::Weekday::Sat => 32,
        chrono::Weekday::Sun => 64,
    }
}

fn parse_time(s: &str) -> Option<(u32, u32)> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    let h: u32 = parts[0].parse().ok()?;
    let m: u32 = parts[1].parse().ok()?;
    if h >= 24 || m >= 60 {
        return None;
    }
    Some((h, m))
}

fn time_to_minutes(h: u32, m: u32) -> u32 {
    h * 60 + m
}

pub fn is_within_window(config: &ScheduleConfig, now: &chrono::DateTime<chrono::Local>) -> bool {
    use chrono::Datelike;
    use chrono::Timelike;

    let weekday = now.weekday();
    if config.days_bitmask & day_bit(weekday) == 0 {
        return false;
    }

    let (start_h, start_m) = match parse_time(&config.start_time) {
        Some(t) => t,
        None => return false,
    };
    let (end_h, end_m) = match parse_time(&config.end_time) {
        Some(t) => t,
        None => return false,
    };

    let start_min = time_to_minutes(start_h, start_m);
    let end_min = time_to_minutes(end_h, end_m);
    let now_min = time_to_minutes(now.hour(), now.minute());

    if start_min <= end_min {
        now_min >= start_min && now_min < end_min
    } else {
        now_min >= start_min || now_min < end_min
    }
}

pub fn load_schedule_config<R: Runtime>(app: &AppHandle<R>) -> ScheduleConfig {
    let store = match tauri_plugin_store::StoreBuilder::new(app, STORE_FILE).build() {
        Ok(s) => s,
        Err(_) => return ScheduleConfig::default(),
    };

    let enabled = store
        .get("shadow_schedule_enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let start_time = store
        .get("shadow_schedule_start")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "09:00".into());
    let end_time = store
        .get("shadow_schedule_end")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "18:00".into());
    let days_bitmask = store
        .get("shadow_schedule_days")
        .and_then(|v| v.as_u64())
        .map(|v| v as u8)
        .unwrap_or(0b0001_1111);
    let buffer_minutes = store
        .get("shadow_buffer_duration")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32)
        .unwrap_or(20);

    ScheduleConfig {
        enabled,
        start_time,
        end_time,
        days_bitmask,
        buffer_minutes,
    }
}

pub fn save_schedule_config<R: Runtime>(app: &AppHandle<R>, config: &ScheduleConfig) {
    let store = match tauri_plugin_store::StoreBuilder::new(app, STORE_FILE).build() {
        Ok(s) => s,
        Err(e) => {
            warn!("Failed to open store for schedule config: {e}");
            return;
        }
    };

    store.set("shadow_schedule_enabled", config.enabled);
    store.set("shadow_schedule_start", config.start_time.clone());
    store.set("shadow_schedule_end", config.end_time.clone());
    store.set("shadow_schedule_days", config.days_bitmask as u64);
    store.set("shadow_buffer_duration", config.buffer_minutes as u64);
}

pub struct ShadowScheduler {
    cancel_tx: Option<oneshot::Sender<()>>,
}

impl ShadowScheduler {
    pub fn start<R: Runtime + 'static>(app: &AppHandle<R>) -> Self {
        let (cancel_tx, cancel_rx) = oneshot::channel::<()>();
        let app_handle = app.clone();

        tauri::async_runtime::spawn(async move {
            let mut interval =
                tokio::time::interval(std::time::Duration::from_secs(SCHEDULE_CHECK_INTERVAL_SECS));
            tokio::pin!(cancel_rx);

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        check_and_apply_schedule(&app_handle);
                    }
                    _ = &mut cancel_rx => {
                        info!("Shadow scheduler cancelled");
                        break;
                    }
                }
            }
        });

        info!("Shadow scheduler started");
        Self {
            cancel_tx: Some(cancel_tx),
        }
    }

    pub fn stop(&mut self) {
        if let Some(tx) = self.cancel_tx.take() {
            let _ = tx.send(());
            info!("Shadow scheduler stopped");
        }
    }
}

impl Drop for ShadowScheduler {
    fn drop(&mut self) {
        self.stop();
    }
}

fn check_and_apply_schedule<R: Runtime + 'static>(app: &AppHandle<R>) {
    let config = load_schedule_config(app);
    if !config.enabled {
        return;
    }

    let now = chrono::Local::now();
    let within = is_within_window(&config, &now);

    let state = app.state::<AppState>();

    let is_recording = state
        .session
        .lock()
        .map(|g| g.is_some())
        .unwrap_or(false);

    let shadow_active = state
        .shadow_session
        .lock()
        .map(|g| g.is_some())
        .unwrap_or(false);

    let is_auto_started = {
        let store = tauri_plugin_store::StoreBuilder::new(app, STORE_FILE)
            .build()
            .ok();
        store
            .and_then(|s| s.get("shadow_auto_started").and_then(|v| v.as_bool()))
            .unwrap_or(false)
    };

    let is_authed = crate::auth::keychain::get_token(app)
        .map(|t| crate::auth::keychain::is_token_valid(&t))
        .unwrap_or(false);

    if within && !shadow_active && !is_recording && is_authed {
        info!("Schedule: within window, starting shadow recording");
        match crate::shadow::ShadowSession::start(None, Some(config.buffer_minutes)) {
            Ok(session) => {
                {
                    let mut guard = state
                        .shadow_session
                        .lock()
                        .unwrap_or_else(|e| e.into_inner());
                    *guard = Some(session);
                }
                if let Ok(store) =
                    tauri_plugin_store::StoreBuilder::new(app, STORE_FILE).build()
                {
                    store.set("shadow_auto_started", true);
                }
                crate::tray::set_tray_shadow(app).ok();
                crate::tray::update_tray_menu(app).ok();
                crate::tray::start_shadow_tray_updater(app);
                info!("Schedule: shadow recording auto-started");
            }
            Err(e) => {
                warn!("Schedule: failed to auto-start shadow recording: {e}");
            }
        }
    } else if !within && shadow_active && is_auto_started {
        info!("Schedule: outside window, auto-stopping shadow recording");
        let session = {
            let mut guard = state
                .shadow_session
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            guard.take()
        };
        if let Some(session) = session {
            let app_handle = app.clone();
            tauri::async_runtime::spawn(async move {
                match crate::shadow::save::save_only(session, &app_handle, None, None, None).await {
                    Ok(meeting_id) => {
                        info!("Schedule: auto-save complete, meeting_id={meeting_id}");
                    }
                    Err(e) => {
                        warn!("Schedule: auto-save failed: {e}");
                    }
                }
                if let Ok(store) =
                    tauri_plugin_store::StoreBuilder::new(&app_handle, STORE_FILE).build()
                {
                    store.set("shadow_auto_started", false);
                }
                crate::tray::clear_tray_title_pub(&app_handle);
                crate::tray::set_tray_recording(&app_handle, false).ok();
                crate::tray::update_tray_menu(&app_handle).ok();
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveDateTime, NaiveTime, TimeZone};

    fn make_local(year: i32, month: u32, day: u32, hour: u32, min: u32) -> chrono::DateTime<chrono::Local> {
        let ndt = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(year, month, day).unwrap(),
            NaiveTime::from_hms_opt(hour, min, 0).unwrap(),
        );
        chrono::Local.from_local_datetime(&ndt).earliest().unwrap()
    }

    fn weekdays_config(start: &str, end: &str) -> ScheduleConfig {
        ScheduleConfig {
            enabled: true,
            start_time: start.into(),
            end_time: end.into(),
            days_bitmask: 0b0001_1111, // Mon-Fri
            buffer_minutes: 20,
        }
    }

    fn every_day_config(start: &str, end: &str) -> ScheduleConfig {
        ScheduleConfig {
            enabled: true,
            start_time: start.into(),
            end_time: end.into(),
            days_bitmask: 0b0111_1111, // all days
            buffer_minutes: 20,
        }
    }

    #[test]
    fn test_within_normal_window() {
        let config = weekdays_config("09:00", "18:00");
        // 2026-03-16 is Monday
        let now = make_local(2026, 3, 16, 10, 30);
        assert!(is_within_window(&config, &now));
    }

    #[test]
    fn test_outside_normal_window_before() {
        let config = weekdays_config("09:00", "18:00");
        let now = make_local(2026, 3, 16, 8, 59);
        assert!(!is_within_window(&config, &now));
    }

    #[test]
    fn test_outside_normal_window_after() {
        let config = weekdays_config("09:00", "18:00");
        let now = make_local(2026, 3, 16, 18, 0);
        assert!(!is_within_window(&config, &now));
    }

    #[test]
    fn test_at_start_boundary() {
        let config = weekdays_config("09:00", "18:00");
        let now = make_local(2026, 3, 16, 9, 0);
        assert!(is_within_window(&config, &now));
    }

    #[test]
    fn test_at_end_boundary_exclusive() {
        let config = weekdays_config("09:00", "18:00");
        let now = make_local(2026, 3, 16, 18, 0);
        assert!(!is_within_window(&config, &now));
    }

    #[test]
    fn test_overnight_window_evening() {
        let config = every_day_config("22:00", "06:00");
        let now = make_local(2026, 3, 16, 23, 30);
        assert!(is_within_window(&config, &now));
    }

    #[test]
    fn test_overnight_window_morning() {
        let config = every_day_config("22:00", "06:00");
        let now = make_local(2026, 3, 17, 3, 0);
        assert!(is_within_window(&config, &now));
    }

    #[test]
    fn test_overnight_window_outside() {
        let config = every_day_config("22:00", "06:00");
        let now = make_local(2026, 3, 16, 12, 0);
        assert!(!is_within_window(&config, &now));
    }

    #[test]
    fn test_weekend_excluded() {
        let config = weekdays_config("09:00", "18:00");
        // 2026-03-14 is Saturday
        let now = make_local(2026, 3, 14, 10, 0);
        assert!(!is_within_window(&config, &now));
    }

    #[test]
    fn test_sunday_excluded() {
        let config = weekdays_config("09:00", "18:00");
        // 2026-03-15 is Sunday
        let now = make_local(2026, 3, 15, 10, 0);
        assert!(!is_within_window(&config, &now));
    }

    #[test]
    fn test_day_bitmask_individual() {
        assert_eq!(day_bit(chrono::Weekday::Mon), 1);
        assert_eq!(day_bit(chrono::Weekday::Tue), 2);
        assert_eq!(day_bit(chrono::Weekday::Wed), 4);
        assert_eq!(day_bit(chrono::Weekday::Thu), 8);
        assert_eq!(day_bit(chrono::Weekday::Fri), 16);
        assert_eq!(day_bit(chrono::Weekday::Sat), 32);
        assert_eq!(day_bit(chrono::Weekday::Sun), 64);
    }

    #[test]
    fn test_invalid_time_format() {
        let config = ScheduleConfig {
            enabled: true,
            start_time: "invalid".into(),
            end_time: "18:00".into(),
            days_bitmask: 0b0111_1111,
            buffer_minutes: 20,
        };
        let now = make_local(2026, 3, 16, 10, 0);
        assert!(!is_within_window(&config, &now));
    }

    #[test]
    fn test_only_specific_day() {
        let config = ScheduleConfig {
            enabled: true,
            start_time: "09:00".into(),
            end_time: "18:00".into(),
            days_bitmask: day_bit(chrono::Weekday::Wed),
            buffer_minutes: 20,
        };
        // 2026-03-18 is Wednesday
        let now_wed = make_local(2026, 3, 18, 10, 0);
        assert!(is_within_window(&config, &now_wed));

        // 2026-03-16 is Monday
        let now_mon = make_local(2026, 3, 16, 10, 0);
        assert!(!is_within_window(&config, &now_mon));
    }

    #[test]
    fn test_parse_time_valid() {
        assert_eq!(parse_time("09:00"), Some((9, 0)));
        assert_eq!(parse_time("23:59"), Some((23, 59)));
        assert_eq!(parse_time("00:00"), Some((0, 0)));
    }

    #[test]
    fn test_parse_time_invalid() {
        assert_eq!(parse_time("25:00"), None);
        assert_eq!(parse_time("12:60"), None);
        assert_eq!(parse_time("abc"), None);
        assert_eq!(parse_time(""), None);
    }
}
