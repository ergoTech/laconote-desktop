use crate::upload::uploader::{ChunkRequest, Uploader};
use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use tracing::{error, info, warn};
use uuid::Uuid;

const QUEUE_DIR_NAME: &str = "offline_chunks";
const KEY_FILE_NAME: &str = ".queue.key";

/// Serializable metadata stored alongside each encrypted chunk file.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChunkMeta {
    chunk_id: String,
    meeting_id: String,
    meeting_name: Option<String>,
    meeting_type: Option<String>,
    project_id: Option<String>,
    captured_at: DateTime<Utc>,
    meeting_start_time: Option<DateTime<Utc>>,
    speakers: Vec<crate::upload::uploader::SpeakerSegment>,
    is_final: bool,
}

/// Persistent, encrypted offline queue for audio chunks that failed to upload.
///
/// Chunks are saved as `<id>.enc` (AES-256-GCM ciphertext) with a companion
/// `<id>.meta.json` metadata file. An encryption key is generated once per
/// installation and persisted in `<queue_dir>/.queue.key`.
///
/// File format for `.enc` files: `[12-byte nonce][ciphertext + 16-byte GCM tag]`
#[derive(Clone)]
pub struct OfflineQueue {
    queue_dir: PathBuf,
}

impl OfflineQueue {
    /// Create (or open) the offline queue at the default macOS path:
    /// `~/Library/Application Support/com.laconote.desktop/offline_chunks/`
    pub fn new() -> Result<Self, String> {
        let dir = default_queue_dir()?;
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create offline queue dir: {e}"))?;
        Ok(Self { queue_dir: dir })
    }

    #[cfg(test)]
    pub fn with_dir(dir: PathBuf) -> Result<Self, String> {
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create test queue dir: {e}"))?;
        Ok(Self { queue_dir: dir })
    }

    /// Encrypt and persist a failed [`ChunkRequest`] to disk.
    pub async fn enqueue(&mut self, req: &ChunkRequest) -> Result<(), String> {
        let key = self.load_or_create_key()?;
        let cipher = Aes256Gcm::new(&key);
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng::default());

        let ciphertext = cipher
            .encrypt(&nonce, req.audio_data.as_slice())
            .map_err(|e| format!("AES-GCM encrypt failed: {e}"))?;

        // Prepend nonce to ciphertext: [12-byte nonce][ciphertext+tag]
        let mut enc_file_data = Vec::with_capacity(12 + ciphertext.len());
        enc_file_data.extend_from_slice(&nonce);
        enc_file_data.extend(ciphertext);

        let chunk_id = Uuid::new_v4().to_string();

        let enc_path = self.queue_dir.join(format!("{chunk_id}.enc"));
        let meta_path = self.queue_dir.join(format!("{chunk_id}.meta.json"));

        std::fs::write(&enc_path, &enc_file_data)
            .map_err(|e| format!("Failed to write encrypted chunk: {e}"))?;
        std::fs::set_permissions(&enc_path, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("Failed to set permissions on encrypted chunk: {e}"))?;

        let meta = ChunkMeta {
            chunk_id: chunk_id.clone(),
            meeting_id: req.meeting_id.clone(),
            meeting_name: req.meeting_name.clone(),
            meeting_type: req.meeting_type.clone(),
            project_id: req.project_id.clone(),
            captured_at: req.captured_at,
            meeting_start_time: req.meeting_start_time,
            speakers: req.speakers.clone(),
            is_final: req.is_final,
        };

        let meta_json = serde_json::to_string_pretty(&meta)
            .map_err(|e| format!("Failed to serialize chunk metadata: {e}"))?;
        std::fs::write(&meta_path, meta_json)
            .map_err(|e| format!("Failed to write chunk metadata: {e}"))?;
        std::fs::set_permissions(&meta_path, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("Failed to set permissions on chunk metadata: {e}"))?;

        info!(chunk_id, "Offline chunk saved");
        Ok(())
    }

    /// Count pending chunks in the queue.
    pub fn pending_count(&self) -> usize {
        match std::fs::read_dir(&self.queue_dir) {
            Ok(entries) => entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| ext == "enc")
                        .unwrap_or(false)
                })
                .count(),
            Err(_) => 0,
        }
    }

    /// Attempt to re-upload all pending offline chunks via `uploader`.
    ///
    /// Successfully uploaded chunks are removed from disk. Failed chunks remain.
    pub async fn retry_pending(&mut self, uploader: &Uploader) -> usize {
        let enc_files: Vec<PathBuf> = match std::fs::read_dir(&self.queue_dir) {
            Ok(entries) => entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("enc"))
                .collect(),
            Err(e) => {
                error!(error = %e, "Failed to read offline queue directory");
                return 0;
            }
        };

        if enc_files.is_empty() {
            return 0;
        }

        info!(count = enc_files.len(), "Retrying offline chunks");
        let key = match self.load_or_create_key() {
            Ok(k) => k,
            Err(e) => {
                error!(error = %e, "Cannot load encryption key for retry");
                return 0;
            }
        };

        let cipher = Aes256Gcm::new(&key);
        let mut success_count = 0usize;

        for enc_path in enc_files {
            let stem = match enc_path.file_stem().and_then(|s| s.to_str()) {
                Some(s) => s.to_string(),
                None => continue,
            };

            let meta_path = self.queue_dir.join(format!("{stem}.meta.json"));
            let meta = match self.load_meta(&meta_path) {
                Ok(m) => m,
                Err(e) => {
                    warn!(chunk_id = %stem, error = %e, "Skipping chunk with unreadable metadata");
                    continue;
                }
            };

            let audio_data = match self.decrypt_chunk(&enc_path, &cipher) {
                Ok(d) => d,
                Err(e) => {
                    warn!(chunk_id = %stem, error = %e, "Failed to decrypt offline chunk");
                    continue;
                }
            };

            let req = ChunkRequest {
                audio_data,
                meeting_id: meta.meeting_id.clone(),
                meeting_name: meta.meeting_name.clone(),
                meeting_type: meta.meeting_type.clone(),
                project_id: meta.project_id.clone(),
                captured_at: meta.captured_at,
                meeting_start_time: meta.meeting_start_time,
                speakers: meta.speakers.clone(),
                is_final: meta.is_final,
            };

            match uploader.upload_chunk(&req).await {
                Ok(_) => {
                    info!(chunk_id = %stem, "Offline chunk uploaded successfully, removing from queue");
                    let _ = std::fs::remove_file(&enc_path);
                    let _ = std::fs::remove_file(&meta_path);
                    success_count += 1;
                }
                Err(e) => {
                    warn!(chunk_id = %stem, error = %e, "Offline chunk retry failed, keeping in queue");
                }
            }
        }

        success_count
    }

    // ── Private helpers ──────────────────────────────────────────────────────

    /// Load the persistent AES-256-GCM key from disk, generating it on first use.
    fn load_or_create_key(&self) -> Result<Key<Aes256Gcm>, String> {
        let key_path = self.queue_dir.join(KEY_FILE_NAME);

        if key_path.exists() {
            let bytes = std::fs::read(&key_path)
                .map_err(|e| format!("Failed to read queue key: {e}"))?;
            if bytes.len() != 32 {
                return Err(format!(
                    "Invalid queue key length: expected 32, got {}",
                    bytes.len()
                ));
            }
            Ok(*Key::<Aes256Gcm>::from_slice(&bytes))
        } else {
            let key = Aes256Gcm::generate_key(&mut OsRng::default());
            std::fs::write(&key_path, key.as_slice())
                .map_err(|e| format!("Failed to write queue key: {e}"))?;
            std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600))
                .map_err(|e| format!("Failed to set permissions on queue key: {e}"))?;
            info!("Generated new offline queue encryption key");
            Ok(key)
        }
    }

    fn load_meta(&self, path: &PathBuf) -> Result<ChunkMeta, String> {
        let json = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read metadata file: {e}"))?;
        serde_json::from_str(&json).map_err(|e| format!("Failed to parse metadata JSON: {e}"))
    }

    fn decrypt_chunk(
        &self,
        enc_path: &PathBuf,
        cipher: &Aes256Gcm,
    ) -> Result<Vec<u8>, String> {
        let data = std::fs::read(enc_path)
            .map_err(|e| format!("Failed to read encrypted file: {e}"))?;

        if data.len() < 12 {
            return Err(format!(
                "Encrypted file too small: {} bytes",
                data.len()
            ));
        }

        let nonce = Nonce::from_slice(&data[..12]);
        let ciphertext = &data[12..];

        cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| format!("AES-GCM decrypt failed: {e}"))
    }
}

fn default_queue_dir() -> Result<PathBuf, String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    Ok(PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("com.laconote.desktop")
        .join(QUEUE_DIR_NAME))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::upload::uploader::SpeakerSegment;

    fn sample_request() -> ChunkRequest {
        ChunkRequest {
            audio_data: vec![0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE],
            meeting_id: "meet-offline-test".to_string(),
            meeting_name: Some("Offline Test".to_string()),
            meeting_type: Some("standup".to_string()),
            project_id: None,
            captured_at: Utc::now(),
            meeting_start_time: None,
            speakers: vec![SpeakerSegment {
                speaker: "Test Speaker".to_string(),
                start: 0.0,
                end: 5.0,
            }],
            is_final: false,
        }
    }

    #[tokio::test]
    async fn test_enqueue_creates_enc_and_meta_files() {
        let dir = tempfile::tempdir().unwrap();
        let mut queue = OfflineQueue::with_dir(dir.path().to_path_buf()).unwrap();

        let req = sample_request();
        queue.enqueue(&req).await.unwrap();

        let enc_files: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("enc"))
            .collect();

        let meta_files: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("json"))
            .collect();

        assert_eq!(enc_files.len(), 1, "Expected 1 .enc file");
        assert_eq!(meta_files.len(), 1, "Expected 1 .meta.json file");
    }

    #[tokio::test]
    async fn test_enqueue_increments_pending_count() {
        let dir = tempfile::tempdir().unwrap();
        let mut queue = OfflineQueue::with_dir(dir.path().to_path_buf()).unwrap();

        assert_eq!(queue.pending_count(), 0);
        queue.enqueue(&sample_request()).await.unwrap();
        assert_eq!(queue.pending_count(), 1);
        queue.enqueue(&sample_request()).await.unwrap();
        assert_eq!(queue.pending_count(), 2);
    }

    #[tokio::test]
    async fn test_round_trip_encrypt_decrypt() {
        let dir = tempfile::tempdir().unwrap();
        let mut queue = OfflineQueue::with_dir(dir.path().to_path_buf()).unwrap();

        let original_data = b"Hello, offline audio chunk!".to_vec();
        let mut req = sample_request();
        req.audio_data = original_data.clone();

        queue.enqueue(&req).await.unwrap();

        // Find the .enc file and decrypt it directly
        let enc_path = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .find(|e| e.path().extension().and_then(|x| x.to_str()) == Some("enc"))
            .unwrap()
            .path();

        let key = queue.load_or_create_key().unwrap();
        let cipher = Aes256Gcm::new(&key);
        let decrypted = queue.decrypt_chunk(&enc_path, &cipher).unwrap();

        assert_eq!(decrypted, original_data, "Decrypted data must match original");
    }

    #[tokio::test]
    async fn test_key_is_reused_across_instances() {
        let dir = tempfile::tempdir().unwrap();

        let queue1 = OfflineQueue::with_dir(dir.path().to_path_buf()).unwrap();
        let key1 = queue1.load_or_create_key().unwrap();

        let queue2 = OfflineQueue::with_dir(dir.path().to_path_buf()).unwrap();
        let key2 = queue2.load_or_create_key().unwrap();

        assert_eq!(
            key1.as_slice(),
            key2.as_slice(),
            "Same directory should produce same key"
        );
    }

    #[tokio::test]
    async fn test_enqueue_files_have_restricted_permissions() {
        let dir = tempfile::tempdir().unwrap();
        let mut queue = OfflineQueue::with_dir(dir.path().to_path_buf()).unwrap();

        let req = sample_request();
        queue.enqueue(&req).await.unwrap();

        for entry in std::fs::read_dir(dir.path()).unwrap().filter_map(|e| e.ok()) {
            let path = entry.path();
            let ext = path.extension().and_then(|x| x.to_str()).unwrap_or("");
            if ext == "enc" || ext == "json" || path.file_name().and_then(|n| n.to_str()) == Some(KEY_FILE_NAME) {
                let perms = std::fs::metadata(&path).unwrap().permissions();
                assert_eq!(
                    perms.mode() & 0o777,
                    0o600,
                    "File {:?} should have mode 0600",
                    path
                );
            }
        }

        let key_path = dir.path().join(KEY_FILE_NAME);
        let perms = std::fs::metadata(&key_path).unwrap().permissions();
        assert_eq!(perms.mode() & 0o777, 0o600, ".queue.key should have mode 0600");
    }

    #[tokio::test]
    async fn test_meta_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let mut queue = OfflineQueue::with_dir(dir.path().to_path_buf()).unwrap();

        let req = sample_request();
        queue.enqueue(&req).await.unwrap();

        let meta_path = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .find(|e| e.path().extension().and_then(|x| x.to_str()) == Some("json"))
            .unwrap()
            .path();

        let meta = queue.load_meta(&meta_path).unwrap();

        assert_eq!(meta.meeting_id, "meet-offline-test");
        assert_eq!(meta.meeting_name.as_deref(), Some("Offline Test"));
        assert_eq!(meta.speakers.len(), 1);
        assert!(!meta.is_final);
    }
}
