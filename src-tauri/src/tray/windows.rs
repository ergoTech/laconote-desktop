use tauri::{AppHandle, Manager, Runtime};
use tracing::debug;

fn show_or_focus_window<R: Runtime>(app: &AppHandle<R>, label: &str) {
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.show();
        let _ = window.unminimize();
        #[cfg(target_os = "macos")]
        {
            let w = window.clone();
            let _ = app.run_on_main_thread(move || {
                extern "C" { fn set_regular_policy(); }
                unsafe { set_regular_policy(); }

                use objc2_app_kit::NSApplication;
                use objc2_foundation::MainThreadMarker;
                unsafe {
                    let mtm = MainThreadMarker::new_unchecked();
                    let ns_app = NSApplication::sharedApplication(mtm);
                    ns_app.activate();
                }
                let _ = w.set_focus();
            });
        }
        #[cfg(not(target_os = "macos"))]
        let _ = window.set_focus();
    } else {
        debug!("{} window not found", label);
    }
}

pub fn show_settings_window<R: Runtime>(app: &AppHandle<R>) {
    show_or_focus_window(app, "settings");
}
