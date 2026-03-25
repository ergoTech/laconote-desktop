use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::info;

use super::CalendarEvent;

const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_CALENDAR_API: &str = "https://www.googleapis.com/calendar/v3";

// These would normally be in a config/env, but for a desktop app they're embedded
// (Google OAuth for installed apps uses PKCE, so client_secret is not truly secret)
const GOOGLE_CLIENT_ID: &str = "YOUR_GOOGLE_CLIENT_ID.apps.googleusercontent.com";
const GOOGLE_CLIENT_SECRET: &str = "YOUR_GOOGLE_CLIENT_SECRET";
const GOOGLE_REDIRECT_URI: &str = "laconote://calendar/callback";
const GOOGLE_SCOPE: &str = "https://www.googleapis.com/auth/calendar.readonly";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: i64,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct CalendarListResponse {
    items: Option<Vec<GoogleEvent>>,
}

#[derive(Debug, Deserialize)]
struct GoogleEvent {
    id: Option<String>,
    summary: Option<String>,
    start: Option<EventTime>,
    end: Option<EventTime>,
    #[serde(rename = "hangoutLink")]
    hangout_link: Option<String>,
    #[serde(rename = "conferenceData")]
    conference_data: Option<ConferenceData>,
    description: Option<String>,
    location: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EventTime {
    #[serde(rename = "dateTime")]
    date_time: Option<String>,
    #[allow(dead_code)]
    date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ConferenceData {
    #[serde(rename = "entryPoints")]
    entry_points: Option<Vec<EntryPoint>>,
}

#[derive(Debug, Deserialize)]
struct EntryPoint {
    #[serde(rename = "entryPointType")]
    entry_point_type: Option<String>,
    uri: Option<String>,
}

pub fn build_oauth_url() -> String {
    format!(
        "https://accounts.google.com/o/oauth2/v2/auth?\
         client_id={GOOGLE_CLIENT_ID}&\
         redirect_uri={}&\
         response_type=code&\
         scope={GOOGLE_SCOPE}&\
         access_type=offline&\
         prompt=consent",
        urlencoding::encode(GOOGLE_REDIRECT_URI),
    )
}

pub async fn exchange_code(code: &str) -> Result<GoogleTokens, String> {
    let client = Client::new();
    let resp = client
        .post(GOOGLE_TOKEN_URL)
        .form(&[
            ("code", code),
            ("client_id", GOOGLE_CLIENT_ID),
            ("client_secret", GOOGLE_CLIENT_SECRET),
            ("redirect_uri", GOOGLE_REDIRECT_URI),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await
        .map_err(|e| format!("Token exchange request failed: {e}"))?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Token exchange failed: {body}"));
    }

    let token_resp: TokenResponse = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse token response: {e}"))?;

    let expires_at = chrono::Utc::now().timestamp() + token_resp.expires_in.unwrap_or(3600);

    info!("Google Calendar OAuth token exchanged successfully");
    Ok(GoogleTokens {
        access_token: token_resp.access_token,
        refresh_token: token_resp.refresh_token,
        expires_at,
    })
}

pub async fn refresh_access_token(refresh_token: &str) -> Result<GoogleTokens, String> {
    let client = Client::new();
    let resp = client
        .post(GOOGLE_TOKEN_URL)
        .form(&[
            ("refresh_token", refresh_token),
            ("client_id", GOOGLE_CLIENT_ID),
            ("client_secret", GOOGLE_CLIENT_SECRET),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await
        .map_err(|e| format!("Token refresh request failed: {e}"))?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Token refresh failed: {body}"));
    }

    let token_resp: TokenResponse = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse refresh response: {e}"))?;

    let expires_at = chrono::Utc::now().timestamp() + token_resp.expires_in.unwrap_or(3600);

    Ok(GoogleTokens {
        access_token: token_resp.access_token,
        refresh_token: Some(refresh_token.to_string()),
        expires_at,
    })
}

pub async fn fetch_upcoming_events(
    access_token: &str,
    minutes_ahead: i64,
) -> Result<Vec<CalendarEvent>, String> {
    let now = chrono::Utc::now();
    let time_max = now + chrono::Duration::minutes(minutes_ahead);

    let client = Client::new();
    let resp = client
        .get(format!("{GOOGLE_CALENDAR_API}/calendars/primary/events"))
        .bearer_auth(access_token)
        .query(&[
            ("timeMin", now.to_rfc3339()),
            ("timeMax", time_max.to_rfc3339()),
            ("singleEvents", "true".to_string()),
            ("orderBy", "startTime".to_string()),
            ("maxResults", "10".to_string()),
        ])
        .send()
        .await
        .map_err(|e| format!("Calendar API request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Calendar API error {status}: {body}"));
    }

    let list: CalendarListResponse = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse calendar response: {e}"))?;

    let events = list
        .items
        .unwrap_or_default()
        .into_iter()
        .filter_map(|e| convert_event(e))
        .collect::<Vec<_>>();

    info!(count = events.len(), "Fetched upcoming calendar events");
    Ok(events)
}

fn convert_event(e: GoogleEvent) -> Option<CalendarEvent> {
    let summary = e.summary.unwrap_or_else(|| "(No title)".into());
    let start = parse_event_time(&e.start?)?;
    let end = parse_event_time(&e.end?).unwrap_or(start + chrono::Duration::hours(1));

    // Extract meeting link from multiple sources
    let meeting_link = e
        .hangout_link
        .or_else(|| {
            e.conference_data
                .and_then(|cd| cd.entry_points)
                .and_then(|eps| {
                    eps.into_iter()
                        .find(|ep| ep.entry_point_type.as_deref() == Some("video"))
                        .and_then(|ep| ep.uri)
                })
        })
        .or_else(|| extract_link_from_text(&e.description.unwrap_or_default()))
        .or_else(|| extract_link_from_text(&e.location.unwrap_or_default()));

    let platform = meeting_link
        .as_ref()
        .and_then(|l| CalendarEvent::detect_platform(l));

    Some(CalendarEvent {
        id: e.id.unwrap_or_default(),
        summary,
        start_time: start,
        end_time: end,
        meeting_link,
        platform,
    })
}

fn parse_event_time(t: &EventTime) -> Option<chrono::DateTime<chrono::Utc>> {
    if let Some(dt) = &t.date_time {
        chrono::DateTime::parse_from_rfc3339(dt)
            .ok()
            .map(|d| d.with_timezone(&chrono::Utc))
    } else {
        // All-day event — skip
        None
    }
}

fn extract_link_from_text(text: &str) -> Option<String> {
    let meeting_domains = [
        "zoom.us", "zoom.com", "meet.google.com", "teams.microsoft.com",
        "teams.live.com", "discord.gg", "discord.com", "webex.com",
    ];
    for word in text.split_whitespace() {
        let trimmed = word.trim_matches(|c: char| !c.is_alphanumeric() && c != ':' && c != '/' && c != '.' && c != '?' && c != '=' && c != '&' && c != '-' && c != '_');
        if trimmed.starts_with("http") {
            for domain in &meeting_domains {
                if trimmed.contains(domain) {
                    return Some(trimmed.to_string());
                }
            }
        }
    }
    None
}
