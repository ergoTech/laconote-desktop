use crate::audio::encode_to_webm;
use crate::auth::keychain;
use crate::shadow::buffer::ShadowBuffer;
use crate::shadow::ShadowSession;
use crate::upload::uploader::SpeakerSegment;
use crate::upload::{ChunkRequest, OfflineQueue, Uploader};
use chrono::Utc;
use std::sync::Arc;
use tauri::{AppHandle, Runtime};
use tracing::{error, info, warn};
use uuid::Uuid;

pub async fn save_only<R: Runtime>(
    session: ShadowSession,
    app: &AppHandle<R>,
    meeting_name: Option<String>,
    meeting_type: Option<String>,
    project_id: Option<String>,
) -> Result<String, String> {
    let total_duration_ms = session.buffer.total_written_duration_ms();
    let pcm = session.stop_and_drain();
    if pcm.is_empty() {
        return Err("Shadow buffer is empty".to_string());
    }

    let meeting_id = Uuid::new_v4().to_string();
    let duration_secs = pcm.len() as f64 / 48_000.0;
    let now = Utc::now();
    let meeting_start_time =
        now - chrono::Duration::milliseconds(total_duration_ms as i64);

    info!(meeting_id = %meeting_id, duration_secs, "Encoding shadow buffer for save_only");

    let audio_data = tokio::task::spawn_blocking(move || encode_to_webm(&pcm))
        .await
        .map_err(|e| format!("Encoding task panicked: {e}"))?
        .map_err(|e| format!("Failed to encode shadow buffer: {e}"))?;

    let jwt = keychain::get_token(app)
        .ok_or_else(|| "Not authenticated – please log in first".to_string())?;

    let req = ChunkRequest {
        audio_data,
        meeting_id: meeting_id.clone(),
        meeting_name,
        meeting_type,
        project_id,
        captured_at: now,
        meeting_start_time: Some(meeting_start_time),
        speakers: vec![SpeakerSegment {
            speaker: "Speaker 1".to_string(),
            start: 0.0,
            end: duration_secs,
        }],
        is_final: true,
    };

    let uploader = Uploader::new(jwt)?;
    match uploader.upload_chunk(&req).await {
        Ok(result) => {
            info!(
                meeting_id = %meeting_id,
                api_meeting_id = ?result.meeting_id,
                "Shadow buffer uploaded successfully"
            );
        }
        Err(e) => {
            warn!(error = %e, "Shadow upload failed – saving to offline queue");
            let mut offline_queue = OfflineQueue::new()?;
            offline_queue.enqueue(&req).await?;
        }
    }

    Ok(meeting_id)
}

pub async fn save_and_record<R: Runtime>(
    session: ShadowSession,
    app: &AppHandle<R>,
    meeting_name: Option<String>,
    meeting_type: Option<String>,
    project_id: Option<String>,
) -> Result<String, String> {
    let total_duration_ms = session.buffer.total_written_duration_ms();
    let pcm = session.stop_and_drain();
    if pcm.is_empty() {
        return Err("Shadow buffer is empty".to_string());
    }

    let meeting_id = Uuid::new_v4().to_string();
    let duration_secs = pcm.len() as f64 / 48_000.0;
    let now = Utc::now();
    let meeting_start_time =
        now - chrono::Duration::milliseconds(total_duration_ms as i64);

    info!(meeting_id = %meeting_id, duration_secs, "Encoding shadow buffer for save_and_record");

    let jwt = keychain::get_token(app)
        .ok_or_else(|| "Not authenticated – please log in first".to_string())?;

    let mid = meeting_id.clone();
    let mn = meeting_name.clone();
    let mt = meeting_type.clone();
    let pid = project_id.clone();
    tokio::spawn(async move {
        let audio_data = match tokio::task::spawn_blocking(move || encode_to_webm(&pcm)).await {
            Ok(Ok(bytes)) => bytes,
            Ok(Err(e)) => {
                error!(meeting_id = %mid, error = %e, "Failed to encode shadow buffer for save_and_record");
                return;
            }
            Err(e) => {
                error!(meeting_id = %mid, error = %e, "Encoding task panicked");
                return;
            }
        };

        let req = ChunkRequest {
            audio_data,
            meeting_id: mid.clone(),
            meeting_name: mn,
            meeting_type: mt,
            project_id: pid,
            captured_at: now,
            meeting_start_time: Some(meeting_start_time),
            speakers: vec![SpeakerSegment {
                speaker: "Speaker 1".to_string(),
                start: 0.0,
                end: duration_secs,
            }],
            is_final: false,
        };

        let uploader = match Uploader::new(jwt) {
            Ok(u) => u,
            Err(e) => {
                error!(meeting_id = %mid, error = %e, "Failed to create uploader for save_and_record");
                return;
            }
        };

        match uploader.upload_chunk(&req).await {
            Ok(result) => {
                info!(
                    meeting_id = %mid,
                    api_meeting_id = ?result.meeting_id,
                    "Shadow buffer uploaded as first chunk (save_and_record)"
                );
            }
            Err(e) => {
                warn!(
                    meeting_id = %mid,
                    error = %e,
                    "Shadow save_and_record upload failed – saving to offline queue"
                );
                if let Ok(mut offline_queue) = OfflineQueue::new() {
                    if let Err(save_err) = offline_queue.enqueue(&req).await {
                        error!(
                            meeting_id = %mid,
                            error = %save_err,
                            "Failed to save save_and_record chunk to offline queue"
                        );
                    }
                }
            }
        }
    });

    Ok(meeting_id)
}

pub async fn save_and_continue<R: Runtime>(
    buffer: Arc<ShadowBuffer>,
    app: &AppHandle<R>,
    meeting_name: Option<String>,
    meeting_type: Option<String>,
    project_id: Option<String>,
) -> Result<String, String> {
    let total_duration_ms = buffer.total_written_duration_ms();
    let pcm = buffer.snapshot_and_clear();
    if pcm.is_empty() {
        return Err("Shadow buffer is empty".to_string());
    }

    let meeting_id = Uuid::new_v4().to_string();
    let duration_secs = pcm.len() as f64 / 48_000.0;
    let now = Utc::now();
    let meeting_start_time =
        now - chrono::Duration::milliseconds(total_duration_ms as i64);

    info!(
        meeting_id = %meeting_id,
        duration_secs,
        "Snapshot shadow buffer for save_and_continue"
    );

    let jwt = keychain::get_token(app)
        .ok_or_else(|| "Not authenticated – please log in first".to_string())?;

    let mid = meeting_id.clone();
    tokio::spawn(async move {
        let audio_data = match tokio::task::spawn_blocking(move || encode_to_webm(&pcm)).await {
            Ok(Ok(bytes)) => bytes,
            Ok(Err(e)) => {
                error!(meeting_id = %mid, error = %e, "Failed to encode shadow snapshot");
                return;
            }
            Err(e) => {
                error!(meeting_id = %mid, error = %e, "Encoding task panicked");
                return;
            }
        };

        let req = ChunkRequest {
            audio_data,
            meeting_id: mid.clone(),
            meeting_name,
            meeting_type,
            project_id,
            captured_at: now,
            meeting_start_time: Some(meeting_start_time),
            speakers: vec![SpeakerSegment {
                speaker: "Speaker 1".to_string(),
                start: 0.0,
                end: duration_secs,
            }],
            is_final: true,
        };

        let uploader = match Uploader::new(jwt) {
            Ok(u) => u,
            Err(e) => {
                error!(meeting_id = %mid, error = %e, "Failed to create uploader for snapshot");
                return;
            }
        };

        match uploader.upload_chunk(&req).await {
            Ok(result) => {
                info!(
                    meeting_id = %mid,
                    api_meeting_id = ?result.meeting_id,
                    "Shadow snapshot uploaded"
                );
            }
            Err(e) => {
                warn!(
                    meeting_id = %mid,
                    error = %e,
                    "Shadow snapshot upload failed – saving to offline queue"
                );
                if let Ok(mut offline_queue) = OfflineQueue::new() {
                    if let Err(save_err) = offline_queue.enqueue(&req).await {
                        error!(
                            meeting_id = %mid,
                            error = %save_err,
                            "Failed to save snapshot to offline queue"
                        );
                    }
                }
            }
        }
    });

    Ok(meeting_id)
}
