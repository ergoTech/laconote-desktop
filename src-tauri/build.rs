fn main() {
    // Load .env file from project root and pass as cargo:rustc-env for env!() macro
    let env_path = std::path::PathBuf::from("../.env");
    if env_path.exists() {
        for line in std::fs::read_to_string(&env_path).unwrap_or_default().lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') { continue; }
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();
                // Only set if not already in environment
                if std::env::var(key).is_err() {
                    println!("cargo:rustc-env={key}={value}");
                }
            }
        }
        println!("cargo:rerun-if-changed=../.env");
    }
    // Also forward from actual env vars (CI, manual export)
    for key in &["GOOGLE_CLIENT_ID", "GOOGLE_CLIENT_SECRET"] {
        if let Ok(val) = std::env::var(key) {
            println!("cargo:rustc-env={key}={val}");
        }
    }
    #[cfg(target_os = "macos")]
    {
        if std::env::var("MACOSX_DEPLOYMENT_TARGET").is_err() {
            std::env::set_var("MACOSX_DEPLOYMENT_TARGET", "14.2");
        }

        cc::Build::new()
            .file("src/audio/cat_tap_bridge.m")
            .flag("-fobjc-arc")
            .flag("-mmacosx-version-min=14.2")
            .compile("cat_tap_bridge");

        println!("cargo:rustc-link-lib=framework=CoreAudio");
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=framework=AVFoundation");
        println!("cargo:rustc-link-lib=framework=AppKit");
        println!("cargo:rerun-if-changed=src/audio/cat_tap_bridge.m");
    }

    tauri_build::build()
}
