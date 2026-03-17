fn main() {
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
