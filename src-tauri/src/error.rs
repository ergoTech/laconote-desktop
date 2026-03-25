use std::fmt;

use serde::ser::Serializer;
use serde::Serialize;

#[derive(Debug)]
pub enum AppError {
    NotAuthenticated(String),
    SessionLockPoisoned(String),
    PlatformUnsupported(String),
    AudioCapture(String),
    Microphone(String),
    Encoding(String),
    Upload(String),
    Permission(String),
    Internal(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAuthenticated(msg) => write!(f, "{msg}"),
            Self::SessionLockPoisoned(msg) => write!(f, "{msg}"),
            Self::PlatformUnsupported(msg) => write!(f, "{msg}"),
            Self::AudioCapture(msg) => write!(f, "{msg}"),
            Self::Microphone(msg) => write!(f, "{msg}"),
            Self::Encoding(msg) => write!(f, "{msg}"),
            Self::Upload(msg) => write!(f, "{msg}"),
            Self::Permission(msg) => write!(f, "{msg}"),
            Self::Internal(msg) => write!(f, "{msg}"),
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
