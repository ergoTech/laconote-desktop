use std::path::PathBuf;
use std::time::SystemTime;
use tracing::{info, warn};

const MAX_AGE_DAYS: u64 = 7;
const OFFLINE_CHUNKS_DIR: &str = "offline_chunks";

fn app_support_dir() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(
        PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("com.laconote.desktop"),
    )
}

pub fn run_cleanup() {
    let dir = match app_support_dir() {
        Some(d) => d,
        None => {
            warn!("Cleanup: could not determine app support directory");
            return;
        }
    };

    cleanup_orphaned_offline_chunks(&dir.join(OFFLINE_CHUNKS_DIR));
    cleanup_old_temp_files(&dir);
}

fn cleanup_orphaned_offline_chunks(queue_dir: &PathBuf) {
    let entries = match std::fs::read_dir(queue_dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    let max_age = std::time::Duration::from_secs(MAX_AGE_DAYS * 24 * 60 * 60);
    let now = SystemTime::now();
    let mut removed = 0u32;

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "enc" && ext != "json" {
            continue;
        }

        let modified = match entry.metadata().and_then(|m| m.modified()) {
            Ok(t) => t,
            Err(_) => continue,
        };

        if let Ok(age) = now.duration_since(modified) {
            if age > max_age && std::fs::remove_file(&path).is_ok() {
                removed += 1;
            }
        }
    }

    if removed > 0 {
        info!(removed, "Cleanup: removed old offline chunk files");
    }
}

fn cleanup_old_temp_files(app_dir: &PathBuf) {
    let entries = match std::fs::read_dir(app_dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    let max_age = std::time::Duration::from_secs(MAX_AGE_DAYS * 24 * 60 * 60);
    let now = SystemTime::now();
    let mut removed = 0u32;

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };

        let is_temp = name.ends_with(".tmp")
            || name.ends_with(".webm.part")
            || name.starts_with("shadow_buffer_");

        if !is_temp {
            continue;
        }

        let modified = match entry.metadata().and_then(|m| m.modified()) {
            Ok(t) => t,
            Err(_) => continue,
        };

        if let Ok(age) = now.duration_since(modified) {
            if age > max_age && std::fs::remove_file(&path).is_ok() {
                removed += 1;
            }
        }
    }

    if removed > 0 {
        info!(removed, "Cleanup: removed old temp files");
    }
}
