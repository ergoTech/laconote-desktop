use std::fs;
use std::path::PathBuf;
use tracing::{info, warn};

const MAX_BACKUP_AGE_DAYS: u64 = 7;

/// Returns the local backup directory for a meeting.
/// Creates it if it doesn't exist.
pub fn backup_dir(meeting_id: &str) -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let dir = PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("com.laconote.desktop")
        .join("recordings")
        .join(meeting_id);
    fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

/// Save a WebM chunk to local backup.
pub fn save_chunk(meeting_id: &str, chunk_index: u32, webm_data: &[u8]) {
    let Some(dir) = backup_dir(meeting_id) else {
        warn!("Failed to create backup directory for {meeting_id}");
        return;
    };

    let filename = format!("chunk_{:04}.webm", chunk_index);
    let path = dir.join(&filename);

    match fs::write(&path, webm_data) {
        Ok(()) => {
            info!(
                meeting_id,
                chunk_index,
                size_bytes = webm_data.len(),
                path = %path.display(),
                "Audio chunk backed up locally"
            );
        }
        Err(e) => {
            warn!(
                meeting_id,
                chunk_index,
                "Failed to backup audio chunk: {e}"
            );
        }
    }
}

/// Clean up old backups older than MAX_BACKUP_AGE_DAYS.
pub fn cleanup_old_backups() {
    let home = match std::env::var("HOME") {
        Ok(h) => h,
        Err(_) => return,
    };
    let recordings_dir = PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("com.laconote.desktop")
        .join("recordings");

    if !recordings_dir.exists() {
        return;
    }

    let cutoff = std::time::SystemTime::now()
        - std::time::Duration::from_secs(MAX_BACKUP_AGE_DAYS * 24 * 3600);

    let Ok(entries) = fs::read_dir(&recordings_dir) else {
        return;
    };

    let mut cleaned = 0u32;
    for entry in entries.flatten() {
        let Ok(metadata) = entry.metadata() else { continue };
        if !metadata.is_dir() { continue; }
        let Ok(modified) = metadata.modified() else { continue };
        if modified < cutoff {
            if fs::remove_dir_all(entry.path()).is_ok() {
                cleaned += 1;
            }
        }
    }

    if cleaned > 0 {
        info!(cleaned, "Cleaned up old audio backups");
    }
}
