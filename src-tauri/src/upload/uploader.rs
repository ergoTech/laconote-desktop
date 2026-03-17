use chrono::{DateTime, Utc};
use reqwest::{multipart, Client};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{info, warn};

const API_BASE_URL: &str = "https://meet.laconote.com";
const UPLOAD_TIMEOUT_SECS: u64 = 30;
const MAX_RETRIES: u32 = 5;
const BASE_DELAY_SECS: u64 = 1;
const MAX_DELAY_SECS: u64 = 30;

/// A single speaker segment as required by the API's `speakers` JSON field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeakerSegment {
    pub speaker: String,
    pub start: f64,
    pub end: f64,
}

/// All fields required to upload one audio chunk to `POST /api/v1/speech-buffered-chunk`.
#[derive(Debug, Clone)]
pub struct ChunkRequest {
    pub audio_data: Vec<u8>,
    pub meeting_id: String,
    pub meeting_name: Option<String>,
    pub meeting_type: Option<String>,
    pub project_id: Option<String>,
    pub captured_at: DateTime<Utc>,
    pub meeting_start_time: Option<DateTime<Utc>>,
    pub speakers: Vec<SpeakerSegment>,
    pub is_final: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadResult {
    pub success: bool,
    pub message: Option<String>,
    pub meeting_id: Option<String>,
}

/// HTTP client for uploading audio chunks with exponential-backoff retry.
#[derive(Clone)]
pub struct Uploader {
    client: Client,
    base_url: String,
    jwt_token: String,
}

impl Uploader {
    pub fn new(jwt_token: String) -> Result<Self, String> {
        let client = Client::builder()
            .timeout(Duration::from_secs(UPLOAD_TIMEOUT_SECS))
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {e}"))?;
        Ok(Self {
            client,
            base_url: API_BASE_URL.to_string(),
            jwt_token,
        })
    }

    #[cfg(test)]
    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }

    /// Upload `req` with up to [`MAX_RETRIES`] attempts and exponential backoff.
    ///
    /// Returns [`Err`] only after all retries are exhausted.
    pub async fn upload_chunk(&self, req: &ChunkRequest) -> Result<UploadResult, String> {
        let mut attempt = 0u32;
        let mut delay = BASE_DELAY_SECS;

        loop {
            match self.try_upload_once(req).await {
                Ok(result) => {
                    info!(
                        meeting_id = %req.meeting_id,
                        is_final = req.is_final,
                        attempt,
                        bytes = req.audio_data.len(),
                        "Chunk uploaded successfully"
                    );
                    return Ok(result);
                }
                Err(e) if attempt < MAX_RETRIES => {
                    warn!(
                        meeting_id = %req.meeting_id,
                        attempt,
                        error = %e,
                        delay_secs = delay,
                        "Chunk upload failed, retrying"
                    );
                    tokio::time::sleep(Duration::from_secs(delay)).await;
                    delay = (delay * 2).min(MAX_DELAY_SECS);
                    attempt += 1;
                }
                Err(e) => {
                    return Err(format!(
                        "Upload failed after {MAX_RETRIES} retries: {e}"
                    ));
                }
            }
        }
    }

    async fn try_upload_once(&self, req: &ChunkRequest) -> Result<UploadResult, String> {
        let speakers_json = serde_json::to_string(&req.speakers)
            .map_err(|e| format!("Failed to serialize speakers: {e}"))?;

        let audio_part = multipart::Part::bytes(req.audio_data.clone())
            .file_name("chunk.webm")
            .mime_str("audio/webm")
            .map_err(|e| format!("Failed to set MIME type: {e}"))?;

        let mut form = multipart::Form::new()
            .part("audio_file", audio_part)
            .text("meeting_id", req.meeting_id.clone())
            .text("captured_at", req.captured_at.to_rfc3339())
            .text("speakers", speakers_json)
            .text("is_final", if req.is_final { "true" } else { "false" });

        if let Some(ref name) = req.meeting_name {
            form = form.text("meeting_name", name.clone());
        }
        if let Some(ref mtype) = req.meeting_type {
            form = form.text("meeting_type", mtype.clone());
        }
        if let Some(ref pid) = req.project_id {
            form = form.text("project_id", pid.clone());
        }
        if let Some(ref start) = req.meeting_start_time {
            form = form.text("meeting_start_time", start.to_rfc3339());
        }

        let url = format!("{}/api/v1/speech-buffered-chunk", self.base_url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.jwt_token))
            .multipart(form)
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {e}"))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(format!("HTTP {status}: {body}"));
        }

        let result: UploadResult = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response JSON: {e}"))?;

        Ok(result)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_request(is_final: bool) -> ChunkRequest {
        ChunkRequest {
            audio_data: vec![0u8; 128],
            meeting_id: "test-meeting-123".to_string(),
            meeting_name: Some("Test Meeting".to_string()),
            meeting_type: Some("standup".to_string()),
            project_id: None,
            captured_at: Utc::now(),
            meeting_start_time: None,
            speakers: vec![SpeakerSegment {
                speaker: "Speaker 1".to_string(),
                start: 0.0,
                end: 60.0,
            }],
            is_final,
        }
    }

    #[test]
    fn test_uploader_construction() {
        let uploader = Uploader::new("test-jwt".to_string());
        assert!(uploader.is_ok());
    }

    #[test]
    fn test_speaker_segment_serializes_correctly() {
        let seg = SpeakerSegment {
            speaker: "Alice".to_string(),
            start: 0.0,
            end: 45.5,
        };
        let json = serde_json::to_string(&seg).unwrap();
        assert!(json.contains("\"speaker\":\"Alice\""));
        assert!(json.contains("\"start\":0.0"));
        assert!(json.contains("\"end\":45.5"));
    }

    #[test]
    fn test_speakers_array_serializes_correctly() {
        let req = make_request(false);
        let json = serde_json::to_string(&req.speakers).unwrap();
        assert!(json.starts_with('['));
        assert!(json.ends_with(']'));
        assert!(json.contains("Speaker 1"));
    }

    #[tokio::test]
    async fn test_upload_fails_with_mock_server_returning_401() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/api/v1/speech-buffered-chunk")
            .with_status(401)
            .with_body(r#"{"error":"Unauthorized"}"#)
            .expect_at_least(1)
            .create_async()
            .await;

        let uploader = Uploader::new("invalid-token".to_string())
            .unwrap()
            .with_base_url(server.url());

        let req = make_request(false);
        let result = uploader.try_upload_once(&req).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("HTTP 401"), "Expected 401 in error, got: {err}");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_upload_succeeds_with_mock_server() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/api/v1/speech-buffered-chunk")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"success":true,"message":"chunk accepted for processing","meeting_id":"test-meeting-123"}"#,
            )
            .expect(1)
            .create_async()
            .await;

        let uploader = Uploader::new("valid-token".to_string())
            .unwrap()
            .with_base_url(server.url());

        let req = make_request(true);
        let result = uploader.try_upload_once(&req).await.unwrap();

        assert!(result.success);
        assert_eq!(result.meeting_id.as_deref(), Some("test-meeting-123"));

        mock.assert_async().await;
    }
}
