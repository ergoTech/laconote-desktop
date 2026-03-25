fn main() {
    // Load .env file from project root for build-time env vars (GOOGLE_CLIENT_ID, etc.)
    if let Ok(env_path) = std::fs::canonicalize("../.env") {
        if env_path.exists() {
            for line in std::fs::read_to_string(&env_path).unwrap_or_default().lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') { continue; }
                if let Some((key, value)) = line.split_once('=') {
                    if std::env::var(key.trim()).is_err() {
                        std::env::set_var(key.trim(), value.trim());
                    }
                }
            }
            println!("cargo:rerun-if-changed=../.env");
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
