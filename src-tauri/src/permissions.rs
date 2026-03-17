use serde::{Deserialize, Serialize};
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PermissionState {
    Granted,
    NotGranted,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionStatus {
    pub system_audio_status: PermissionState,
    pub system_audio: PermissionState,
    pub screen_capture_access: PermissionState,
    pub system_audio_capture_ready: PermissionState,
    pub microphone: PermissionState,
}

pub fn check_permissions() -> PermissionStatus {
    let (system_audio_status, screen_capture_access, system_audio_capture_ready) =
        check_system_audio();
    let microphone = check_microphone();
    info!(
        ?system_audio_status,
        ?screen_capture_access,
        ?system_audio_capture_ready,
        ?microphone,
        "Permission status checked"
    );
    PermissionStatus {
        system_audio: system_audio_status.clone(),
        system_audio_status,
        screen_capture_access,
        system_audio_capture_ready,
        microphone,
    }
}

#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
}

fn derive_system_audio_status(
    screen_capture_access: &PermissionState,
    system_audio_capture_ready: &PermissionState,
) -> PermissionState {
    use PermissionState::{Granted, NotGranted, Unknown};

    match system_audio_capture_ready {
        Granted => Granted,
        NotGranted => NotGranted,
        Unknown => match screen_capture_access {
            Granted | NotGranted | Unknown => Unknown,
        },
    }
}

#[cfg(target_os = "macos")]
fn check_system_audio() -> (PermissionState, PermissionState, PermissionState) {
    use crate::audio::{check_catap_compatibility, CaTapCompatibility};

    match check_catap_compatibility() {
        CaTapCompatibility::Unsupported => {
            info!("CATap is unsupported on this macOS version; reporting system audio as not granted");
            (
                PermissionState::NotGranted,
                PermissionState::NotGranted,
                PermissionState::NotGranted,
            )
        }
        CaTapCompatibility::Supported | CaTapCompatibility::Unknown => {
            let screen_capture_access = if unsafe { CGPreflightScreenCaptureAccess() } {
                info!("Screen capture access satisfied by CoreGraphics preflight");
                PermissionState::Granted
            } else {
                PermissionState::NotGranted
            };
            let system_audio_capture_ready = PermissionState::Unknown;
            let system_audio_status =
                derive_system_audio_status(&screen_capture_access, &system_audio_capture_ready);

            (
                system_audio_status,
                screen_capture_access,
                system_audio_capture_ready,
            )
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn check_system_audio() -> (PermissionState, PermissionState, PermissionState) {
    (
        PermissionState::Granted,
        PermissionState::Granted,
        PermissionState::Granted,
    )
}

#[cfg(target_os = "macos")]
fn check_microphone() -> PermissionState {
    extern "C" {
        fn check_mic_authorization() -> i32;
    }
    if unsafe { check_mic_authorization() } == 1 {
        PermissionState::Granted
    } else {
        PermissionState::NotGranted
    }
}

#[cfg(not(target_os = "macos"))]
fn check_microphone() -> PermissionState {
    PermissionState::Granted
}

fn resolve_system_settings_urls(pane: &str) -> &'static [&'static str] {
    match pane {
        "system_audio" | "screen_recording" => &[
            "x-apple.systempreferences:com.apple.settings.PrivacySecurity.extension?Privacy_ScreenCapture",
            "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture",
            "x-apple.systempreferences:com.apple.settings.PrivacySecurity.extension",
            "x-apple.systempreferences:com.apple.preference.security?Privacy",
            "x-apple.systempreferences:com.apple.preference.security",
        ],
        "microphone" => &[
            "x-apple.systempreferences:com.apple.settings.PrivacySecurity.extension?Privacy_Microphone",
            "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone",
            "x-apple.systempreferences:com.apple.settings.PrivacySecurity.extension",
            "x-apple.systempreferences:com.apple.preference.security?Privacy",
            "x-apple.systempreferences:com.apple.preference.security",
        ],
        _ => &[],
    }
}

pub fn open_system_settings(pane: &str) -> bool {
    let urls = resolve_system_settings_urls(pane);
    if urls.is_empty() {
        warn!(pane, "Unknown system settings pane requested");
        return false;
    }

    #[cfg(target_os = "macos")]
    {
        use std::ffi::CString;

        extern "C" {
            fn open_url_nsworkspace(url: *const std::os::raw::c_char) -> i32;
        }

        for url in urls {
            let Ok(cstr) = CString::new(*url) else {
                warn!(pane, url = *url, "Failed to encode system settings URL");
                continue;
            };

            let opened = unsafe { open_url_nsworkspace(cstr.as_ptr()) == 1 };
            info!(pane, url = *url, opened, "Attempted to open system settings URL");
            if opened {
                return true;
            }
        }

        warn!(pane, "Failed to open any system settings URL for requested pane");
        return false;
    }

    #[allow(unreachable_code)]
    {
        for url in urls {
            if std::process::Command::new("open").arg(url).spawn().is_ok() {
                return true;
            }
        }
        false
    }
}

#[cfg(target_os = "macos")]
pub fn request_mic_permission() -> bool {
    extern "C" {
        fn request_mic_authorization_sync() -> i32;
    }
    unsafe { request_mic_authorization_sync() == 1 }
}

#[cfg(not(target_os = "macos"))]
pub fn request_mic_permission() -> bool {
    true
}

#[cfg(target_os = "macos")]
pub fn request_system_audio_permission() -> bool {
    info!("Opening System Settings for manual system audio verification");
    open_system_settings("system_audio")
}

#[cfg(not(target_os = "macos"))]
pub fn request_system_audio_permission() -> bool {
    true
}

#[allow(dead_code)]
pub fn request_permissions_after_auth<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    use tauri_plugin_notification::NotificationExt;

    let mic_state = check_microphone();
    if mic_state != PermissionState::Granted {
        info!("Requesting microphone permission after auth");
        request_mic_permission();
    }

    let (system_audio_status, _, _) = check_system_audio();
    if system_audio_status != PermissionState::Granted {
        info!("System audio not fully verified – prompting user via notification");
        let _ = app
            .notification()
            .builder()
            .title("Laconote — Verify System Audio")
            .body(
                "Open System Settings → Privacy & Security → Screen & System Audio Recording and verify that Laconote is allowed to record system audio. Laconote confirms readiness when the audio tap can actually start.",
            )
            .show();
    }
}

#[cfg(test)]
mod tests {
    use super::{derive_system_audio_status, resolve_system_settings_urls, PermissionState};

    #[test]
    fn system_audio_alias_uses_same_settings_urls() {
        assert_eq!(
            resolve_system_settings_urls("system_audio"),
            resolve_system_settings_urls("screen_recording")
        );
    }

    #[test]
    fn unknown_pane_has_no_urls() {
        assert!(resolve_system_settings_urls("unknown").is_empty());
    }

    #[test]
    fn derived_system_audio_status_prefers_verified_granted() {
        assert_eq!(
            derive_system_audio_status(&PermissionState::Granted, &PermissionState::Unknown),
            PermissionState::Unknown
        );
        assert_eq!(
            derive_system_audio_status(&PermissionState::Unknown, &PermissionState::Granted),
            PermissionState::Granted
        );
    }

    #[test]
    fn derived_system_audio_status_remains_unknown_without_verified_signal() {
        assert_eq!(
            derive_system_audio_status(&PermissionState::Unknown, &PermissionState::Unknown),
            PermissionState::Unknown
        );
        assert_eq!(
            derive_system_audio_status(&PermissionState::NotGranted, &PermissionState::Unknown),
            PermissionState::Unknown
        );
        assert_eq!(
            derive_system_audio_status(&PermissionState::Granted, &PermissionState::Unknown),
            PermissionState::Unknown
        );
    }

    #[test]
    fn derived_system_audio_status_reports_not_granted_when_probe_fails() {
        assert_eq!(
            derive_system_audio_status(&PermissionState::Granted, &PermissionState::NotGranted),
            PermissionState::NotGranted
        );
        assert_eq!(
            derive_system_audio_status(&PermissionState::NotGranted, &PermissionState::NotGranted),
            PermissionState::NotGranted
        );
    }
}
