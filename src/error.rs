use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type ClientResult<T, E = ClientError> = std::result::Result<T, E>;

/// Google's structured error reason and location.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct ApiErrorDetail {
    pub domain: String,
    pub reason: String,
    pub message: String,
    pub location: Option<String>,
    pub location_type: Option<String>,
}

/// An unsuccessful HTTP response, including Google's structured payload when available.
#[derive(Debug, Clone, Error)]
#[error("Google Calendar HTTP {status}: {message}")]
pub struct ApiError {
    pub status: reqwest::StatusCode,
    pub message: String,
    pub errors: Vec<ApiErrorDetail>,
    pub details: Vec<serde_json::Value>,
    pub body: serde_json::Value,
}

impl ApiError {
    pub(crate) fn from_response(status: reqwest::StatusCode, bytes: &[u8]) -> Self {
        let body: serde_json::Value = serde_json::from_slice(bytes).unwrap_or_else(|_| {
            serde_json::Value::String(String::from_utf8_lossy(bytes).into_owned())
        });
        let error = body.get("error").unwrap_or(&body);
        Self {
            status,
            message: error
                .get("message")
                .and_then(serde_json::Value::as_str)
                .map_or_else(
                    || status.canonical_reason().unwrap_or("API error").to_string(),
                    str::to_string,
                ),
            errors: error
                .get("errors")
                .cloned()
                .and_then(|v| serde_json::from_value(v).ok())
                .unwrap_or_default(),
            details: error
                .get("details")
                .and_then(serde_json::Value::as_array)
                .cloned()
                .unwrap_or_default(),
            body,
        }
    }
}

/// Transport, API, validation, or serialization failure. HTTP status is never discarded.
#[derive(Debug, Error)]
pub enum ClientError {
    #[error(transparent)]
    Api(#[from] Box<ApiError>),
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("URL error: {0}")]
    Url(#[from] url::ParseError),
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    #[error("OAuth error: {0}")]
    UnknownError(String),
}

impl ClientError {
    /// HTTP status, when this failure came from an HTTP response.
    #[must_use]
    pub fn status(&self) -> Option<reqwest::StatusCode> {
        match self {
            Self::Api(error) => Some(error.status),
            Self::HttpError(error) => error.status(),
            _ => None,
        }
    }

    /// A sync token expired; the application must perform a new full sync.
    #[must_use]
    pub fn requires_full_sync(&self) -> bool {
        self.status() == Some(reqwest::StatusCode::GONE)
    }
}

impl From<anyhow::Error> for ClientError {
    fn from(value: anyhow::Error) -> Self {
        Self::UnknownError(value.to_string())
    }
}
