use reqwest::Client;
use serde::Deserialize;
use tracing::info;

use super::CalendarEvent;

const MS_TOKEN_URL: &str = "https://login.microsoftonline.com/common/oauth2/v2.0/token";
const MS_GRAPH_API: &str = "https://graph.microsoft.com/v1.0";

const MS_CLIENT_ID: &str = ""; // TODO: Register in Azure AD Portal
const MS_REDIRECT_URI: &str = "http://localhost:19847/calendar/callback";
const MS_SCOPE: &str = "Calendars.Read offline_access";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OutlookTokens {
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
struct EventListResponse {
    value: Option<Vec<OutlookEvent>>,
}

#[derive(Debug, Deserialize)]
struct OutlookEvent {
    id: Option<String>,
    subject: Option<String>,
    start: Option<OutlookDateTime>,
    end: Option<OutlookDateTime>,
    #[serde(rename = "onlineMeeting")]
    online_meeting: Option<OnlineMeeting>,
    #[serde(rename = "bodyPreview")]
    body_preview: Option<String>,
    location: Option<Location>,
}

#[derive(Debug, Deserialize)]
struct OutlookDateTime {
    #[serde(rename = "dateTime")]
    date_time: Option<String>,
    #[serde(rename = "timeZone")]
    #[allow(dead_code)]
    time_zone: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OnlineMeeting {
    #[serde(rename = "joinUrl")]
    join_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Location {
    #[serde(rename = "displayName")]
    display_name: Option<String>,
}

pub fn build_oauth_url() -> String {
    format!(
        "https://login.microsoftonline.com/common/oauth2/v2.0/authorize?\
         client_id={MS_CLIENT_ID}&\
         redirect_uri={}&\
         response_type=code&\
         scope={}&\
         response_mode=query",
        urlencoding::encode(MS_REDIRECT_URI),
        urlencoding::encode(MS_SCOPE),
    )
}

pub async fn exchange_code(code: &str) -> Result<OutlookTokens, String> {
    let client = Client::new();
    let resp = client
        .post(MS_TOKEN_URL)
        .form(&[
            ("code", code),
            ("client_id", MS_CLIENT_ID),
            ("redirect_uri", MS_REDIRECT_URI),
            ("scope", MS_SCOPE),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await
        .map_err(|e| format!("Outlook token exchange failed: {e}"))?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Outlook token exchange error: {body}"));
    }

    let token_resp: TokenResponse = resp.json().await.map_err(|e| format!("Parse error: {e}"))?;
    let expires_at = chrono::Utc::now().timestamp() + token_resp.expires_in.unwrap_or(3600);

    info!("Outlook Calendar OAuth token exchanged");
    Ok(OutlookTokens {
        access_token: token_resp.access_token,
        refresh_token: token_resp.refresh_token,
        expires_at,
    })
}

pub async fn refresh_access_token(refresh_token: &str) -> Result<OutlookTokens, String> {
    let client = Client::new();
    let resp = client
        .post(MS_TOKEN_URL)
        .form(&[
            ("refresh_token", refresh_token),
            ("client_id", MS_CLIENT_ID),
            ("scope", MS_SCOPE),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await
        .map_err(|e| format!("Outlook token refresh failed: {e}"))?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Outlook token refresh error: {body}"));
    }

    let token_resp: TokenResponse = resp.json().await.map_err(|e| format!("Parse error: {e}"))?;
    let expires_at = chrono::Utc::now().timestamp() + token_resp.expires_in.unwrap_or(3600);

    Ok(OutlookTokens {
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
        .get(format!("{MS_GRAPH_API}/me/calendarview"))
        .bearer_auth(access_token)
        .query(&[
            ("startDateTime", now.to_rfc3339()),
            ("endDateTime", time_max.to_rfc3339()),
            ("$top", "10".to_string()),
            ("$orderby", "start/dateTime".to_string()),
            ("$select", "id,subject,start,end,onlineMeeting,bodyPreview,location".to_string()),
        ])
        .send()
        .await
        .map_err(|e| format!("Outlook API request failed: {e}"))?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Outlook API error: {body}"));
    }

    let list: EventListResponse = resp.json().await.map_err(|e| format!("Parse error: {e}"))?;

    let events = list
        .value
        .unwrap_or_default()
        .into_iter()
        .filter_map(convert_event)
        .collect::<Vec<_>>();

    info!(count = events.len(), "Fetched Outlook calendar events");
    Ok(events)
}

fn convert_event(e: OutlookEvent) -> Option<CalendarEvent> {
    let summary = e.subject.unwrap_or_else(|| "(No title)".into());
    let start = parse_outlook_time(&e.start?)?;
    let end = parse_outlook_time(&e.end?).unwrap_or(start + chrono::Duration::hours(1));

    let meeting_link = e
        .online_meeting
        .and_then(|om| om.join_url)
        .or_else(|| extract_link(&e.body_preview.unwrap_or_default()))
        .or_else(|| extract_link(&e.location.and_then(|l| l.display_name).unwrap_or_default()));

    let platform = meeting_link.as_ref().and_then(|l| CalendarEvent::detect_platform(l));

    Some(CalendarEvent {
        id: e.id.unwrap_or_default(),
        summary,
        start_time: start,
        end_time: end,
        meeting_link,
        platform,
    })
}

fn parse_outlook_time(t: &OutlookDateTime) -> Option<chrono::DateTime<chrono::Utc>> {
    let dt_str = t.date_time.as_ref()?;
    // Outlook returns "2026-03-25T14:00:00.0000000" without timezone suffix
    // timeZone field indicates the zone, but we assume UTC for calendarview
    chrono::NaiveDateTime::parse_from_str(dt_str, "%Y-%m-%dT%H:%M:%S%.f")
        .ok()
        .map(|naive| naive.and_utc())
}

fn extract_link(text: &str) -> Option<String> {
    let domains = ["zoom.us", "meet.google.com", "teams.microsoft.com", "teams.live.com", "discord.gg", "webex.com"];
    for word in text.split_whitespace() {
        let trimmed = word.trim_matches(|c: char| !c.is_alphanumeric() && c != ':' && c != '/' && c != '.' && c != '?' && c != '=' && c != '&' && c != '-' && c != '_');
        if trimmed.starts_with("http") {
            for d in &domains {
                if trimmed.contains(d) { return Some(trimmed.to_string()); }
            }
        }
    }
    None
}
