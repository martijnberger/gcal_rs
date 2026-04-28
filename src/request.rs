use reqwest::header::{HeaderMap, HeaderValue, IF_MATCH};
use serde::{Serialize, Serializer};

use crate::{QueryParams, Sendable};

/// Percent-encode a single path identifier, never the separators between identifiers.
pub(crate) fn segment(value: &str) -> String {
    const SET: &percent_encoding::AsciiSet = &percent_encoding::NON_ALPHANUMERIC
        .remove(b'-')
        .remove(b'_')
        .remove(b'.')
        .remove(b'~');
    percent_encoding::utf8_percent_encode(value, SET).to_string()
}

pub(crate) struct ApiRequest<T = ()> {
    path: String,
    body: Option<T>,
    pub query: QueryParams,
    pub if_match: Option<String>,
}

impl ApiRequest {
    pub fn new(path: String) -> Self {
        Self {
            path,
            body: None,
            query: QueryParams::new(),
            if_match: None,
        }
    }
}

impl<T> ApiRequest<T> {
    pub fn with_body(path: String, body: T) -> Self {
        Self {
            path,
            body: Some(body),
            query: QueryParams::new(),
            if_match: None,
        }
    }
}

impl<T: Serialize> Serialize for ApiRequest<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.body.serialize(serializer)
    }
}

impl<T: Serialize> Sendable for ApiRequest<T> {
    fn path(&self, _: Option<String>) -> String {
        self.path.clone()
    }
    fn query(&self) -> QueryParams {
        self.query.clone()
    }
    fn has_body(&self) -> bool {
        self.body.is_some()
    }
    fn headers(&self) -> crate::ClientResult<HeaderMap> {
        let mut headers = HeaderMap::new();
        if let Some(etag) = &self.if_match {
            headers.insert(
                IF_MATCH,
                HeaderValue::from_str(etag)
                    .map_err(|e| crate::ClientError::InvalidRequest(e.to_string()))?,
            );
        }
        Ok(headers)
    }
}
