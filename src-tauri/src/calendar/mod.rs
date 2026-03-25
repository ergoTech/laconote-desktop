pub mod google;
pub mod scheduler;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub id: String,
    pub summary: String,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: chrono::DateTime<chrono::Utc>,
    pub meeting_link: Option<String>,
    pub platform: Option<MeetingPlatform>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MeetingPlatform {
    Zoom,
    GoogleMeet,
    MicrosoftTeams,
    Discord,
    Webex,
    Slack,
    Other,
}

impl CalendarEvent {
    pub fn detect_platform(link: &str) -> Option<MeetingPlatform> {
        let lower = link.to_lowercase();
        if lower.contains("zoom.us") || lower.contains("zoom.com") {
            Some(MeetingPlatform::Zoom)
        } else if lower.contains("meet.google.com") {
            Some(MeetingPlatform::GoogleMeet)
        } else if lower.contains("teams.microsoft.com") || lower.contains("teams.live.com") {
            Some(MeetingPlatform::MicrosoftTeams)
        } else if lower.contains("discord.gg") || lower.contains("discord.com") {
            Some(MeetingPlatform::Discord)
        } else if lower.contains("webex.com") {
            Some(MeetingPlatform::Webex)
        } else if lower.contains("slack.com/calls") || lower.contains("app.slack.com") {
            Some(MeetingPlatform::Slack)
        } else if lower.starts_with("http") {
            Some(MeetingPlatform::Other)
        } else {
            None
        }
    }

    pub fn minutes_until(&self) -> i64 {
        let now = chrono::Utc::now();
        (self.start_time - now).num_minutes()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarConfig {
    pub enabled: bool,
    pub provider: String,
    pub remind_minutes_before: u32,
    pub auto_record: bool,
}

impl Default for CalendarConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: "google".into(),
            remind_minutes_before: 2,
            auto_record: false,
        }
    }
}
