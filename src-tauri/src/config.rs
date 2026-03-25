pub const API_BASE_URL: &str = "https://meet.laconote.com";
pub const DASHBOARD_URL: &str = "https://laconote.com/";
pub const LOGIN_URL: &str = "https://laconote.com/login?redirect=laconote%3A%2F%2Fauth%2Fcallback";
pub const MEETING_URL_PREFIX: &str = "https://laconote.com/meeting/";

pub const UPLOAD_TIMEOUT_SECS: u64 = 30;
pub const MAX_UPLOAD_RETRIES: u32 = 5;
pub const BASE_RETRY_DELAY_SECS: u64 = 1;
pub const MAX_RETRY_DELAY_SECS: u64 = 30;

pub fn meeting_url(meeting_id: &str) -> String {
    format!("{MEETING_URL_PREFIX}{meeting_id}")
}
