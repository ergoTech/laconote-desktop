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

async fn encode_and_upload(
    pcm: Vec<f32>,
    jwt: String,
    meeting_id: String,
    meeting_name: Option<String>,
    meeting_type: Option<String>,
    project_id: Option<String>,
    total_duration_ms: u64,
    is_final: bool,
) {
    let duration_secs = pcm.len() as f64 / 48_000.0;
    let now = Utc::now();
    let meeting_start_time = now - chrono::Duration::milliseconds(total_duration_ms as i64);

    let audio_data = match tokio::task::spawn_blocking(move || encode_to_webm(&pcm)).await {
        Ok(Ok(bytes)) => bytes,
        Ok(Err(e)) => {
            error!(meeting_id = %meeting_id, error = %e, "Failed to encode shadow buffer");
            return;
        }
        Err(e) => {
            error!(meeting_id = %meeting_id, error = %e, "Encoding task panicked");
            return;
        }
    };

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
        is_final,
    };

    let uploader = match Uploader::new(jwt) {
        Ok(u) => u,
        Err(e) => {
            error!(meeting_id = %meeting_id, error = %e, "Failed to create uploader");
            return;
        }
    };

    match uploader.upload_chunk(&req).await {
        Ok(result) => {
            info!(
                meeting_id = %meeting_id,
                api_meeting_id = ?result.meeting_id,
                "Shadow buffer uploaded successfully"
            );
        }
        Err(e) => {
            warn!(
                meeting_id = %meeting_id,
                error = %e,
                "Shadow upload failed – saving to offline queue"
            );
            if let Ok(mut offline_queue) = OfflineQueue::new() {
                if let Err(save_err) = offline_queue.enqueue(&req).await {
                    error!(
                        meeting_id = %meeting_id,
                        error = %save_err,
                        "Failed to save chunk to offline queue"
                    );
                }
            }
        }
    }
}

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
    info!(meeting_id = %meeting_id, duration_secs = pcm.len() as f64 / 48_000.0, "Encoding shadow buffer for save_only");

    let jwt = keychain::get_token(app)
        .ok_or_else(|| "Not authenticated – please log in first".to_string())?;

    encode_and_upload(pcm, jwt, meeting_id.clone(), meeting_name, meeting_type, project_id, total_duration_ms, true).await;

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
    info!(meeting_id = %meeting_id, duration_secs = pcm.len() as f64 / 48_000.0, "Encoding shadow buffer for save_and_record");

    let jwt = keychain::get_token(app)
        .ok_or_else(|| "Not authenticated – please log in first".to_string())?;

    let mid = meeting_id.clone();
    tokio::spawn(async move {
        encode_and_upload(pcm, jwt, mid, meeting_name, meeting_type, project_id, total_duration_ms, false).await;
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
    info!(meeting_id = %meeting_id, duration_secs = pcm.len() as f64 / 48_000.0, "Snapshot shadow buffer for save_and_continue");

    let jwt = keychain::get_token(app)
        .ok_or_else(|| "Not authenticated – please log in first".to_string())?;

    let mid = meeting_id.clone();
    tokio::spawn(async move {
        encode_and_upload(pcm, jwt, mid, meeting_name, meeting_type, project_id, total_duration_ms, true).await;
    });

    Ok(meeting_id)
}
