use thiserror::Error;

pub type ClientResult<T, E = ClientError> = std::result::Result<T, E>;

/// ClientError provides a mechanism to determine when the access token has expired. All other
/// errors will be encapsulated by `UnknownError`.
#[derive(Debug, Error)]
pub enum ClientError {
    #[error("Invalid Access Token")]
    InvalidToken,
    #[error("HTTP Error: {0}")]
    HttpError(reqwest::Error),
    #[error("Unknown Error: {0}")]
    UnknownError(String),
}

impl From<anyhow::Error> for ClientError {
    fn from(value: anyhow::Error) -> Self {
        Self::UnknownError(value.to_string())
    }
}

impl From<serde_json::Error> for ClientError {
    fn from(value: serde_json::Error) -> Self {
        Self::UnknownError(value.to_string())
    }
}

impl From<url::ParseError> for ClientError {
    fn from(value: url::ParseError) -> Self {
        Self::UnknownError(value.to_string())
    }
}

impl From<reqwest::Error> for ClientError {
    fn from(value: reqwest::Error) -> Self {
        Self::UnknownError(value.to_string())
    }
}

impl From<reqwest::header::ToStrError> for ClientError {
    fn from(value: reqwest::header::ToStrError) -> Self {
        Self::UnknownError(value.to_string())
    }
}
