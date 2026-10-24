//! Authenticated Calendar multipart batches.
//!
//! A batch is not a transaction: Google may execute its operations in any order.
//! Results are correlated by Content-ID and returned in request order, even if the
//! wire response is reordered. A failed item does not discard successful siblings.
//! See <https://developers.google.com/workspace/calendar/api/guides/batch>.

use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue, CONTENT_LENGTH, CONTENT_TYPE};
use reqwest::{Method, StatusCode};
use serde::de::DeserializeOwned;

use crate::event::options::EventWriteMethod;
use crate::request::segment;
use crate::{
    ApiError, ClientError, ClientResult, EventPatch, EventWrite, EventWriteOptions, GCalClient,
    QueryParams,
};

const MAX_OPERATIONS: usize = 1000;
static NEXT_BOUNDARY: AtomicU64 = AtomicU64::new(1);

fn invalid(message: impl Into<String>) -> ClientError {
    ClientError::InvalidRequest(message.into())
}

/// One Calendar request. Event mutations use the same safe DTOs and options as
/// ordinary event calls; the raw constructor is an explicit JSON escape hatch.
#[derive(Debug, Clone)]
pub struct BatchOperation {
    id: Option<String>,
    method: Method,
    path: String,
    body: Option<Vec<u8>>,
    if_match: Option<String>,
}

impl BatchOperation {
    /// Build a raw Calendar operation. `path` must be an encoded absolute path
    /// beginning `/calendar/v3/`, without a query or fragment. Query values are
    /// encoded separately, including repeated keys. Only Calendar HTTP methods
    /// are accepted; authentication and framing headers cannot be overridden.
    ///
    /// Prefer the typed event constructors for event writes. Raw JSON is the
    /// caller's responsibility, including omission of response-only fields.
    /// # Errors
    /// Returns an error for an unsafe path, unsupported method, or invalid JSON.
    pub fn raw(
        method: Method,
        path: impl Into<String>,
        query: QueryParams,
        body: Option<serde_json::Value>,
    ) -> ClientResult<Self> {
        Self::from_body(
            method,
            path.into(),
            query,
            body.as_ref().map(serde_json::to_vec).transpose()?,
            None,
        )
    }

    fn from_body(
        method: Method,
        mut path: String,
        query: QueryParams,
        body: Option<Vec<u8>>,
        if_match: Option<String>,
    ) -> ClientResult<Self> {
        if ![
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
        ]
        .contains(&method)
        {
            return Err(invalid("unsupported Calendar batch method"));
        }
        validate_path(&path)?;
        if let Some(value) = &if_match {
            validate_header(value)?;
        }
        if !query.is_empty() {
            let encoded = url::form_urlencoded::Serializer::new(String::new())
                .extend_pairs(query)
                .finish();
            path.push('?');
            path.push_str(&encoded);
        }
        Ok(Self {
            id: None,
            method,
            path,
            body,
            if_match,
        })
    }

    /// Set a caller-selected Content-ID, without angle brackets. If omitted,
    /// the batch assigns `item-N`. IDs must be unique within the request.
    /// # Errors
    /// Returns an error for an empty ID or unsafe header characters.
    pub fn with_id(mut self, id: impl Into<String>) -> ClientResult<Self> {
        let id = id.into();
        validate_id(&id)?;
        self.id = Some(id);
        Ok(self)
    }

    /// Apply an optimistic concurrency precondition without retrying conflicts.
    /// # Errors
    /// Returns an error for an invalid or injected HTTP header value.
    pub fn if_match(mut self, etag: impl Into<String>) -> ClientResult<Self> {
        let etag = etag.into();
        validate_header(&etag)?;
        self.if_match = Some(etag);
        Ok(self)
    }

    /// Read an event, with optional additional response query parameters.
    /// # Errors
    /// Returns an error if the generated request is invalid.
    pub fn event_get(calendar_id: &str, event_id: &str, query: QueryParams) -> ClientResult<Self> {
        Self::from_body(
            Method::GET,
            event_path(calendar_id, Some(event_id)),
            query,
            None,
            None,
        )
    }

    /// Insert a safe writable event, enabling conference/attachment support as needed.
    /// # Errors
    /// Returns serialization, header, or conflicting conference-version errors.
    pub fn event_insert(
        calendar_id: &str,
        event: EventWrite,
        options: EventWriteOptions,
    ) -> ClientResult<Self> {
        Self::event_write(
            Method::POST,
            event_path(calendar_id, None),
            event,
            options,
            EventWriteMethod::Insert,
        )
    }

    /// Replace an event using only writable properties.
    /// # Errors
    /// Returns serialization, header, or conflicting conference-version errors.
    pub fn event_update(
        calendar_id: &str,
        event_id: &str,
        event: EventWrite,
        options: EventWriteOptions,
    ) -> ClientResult<Self> {
        Self::event_write(
            Method::PUT,
            event_path(calendar_id, Some(event_id)),
            event,
            options,
            EventWriteMethod::Update,
        )
    }

    fn event_write(
        method: Method,
        path: String,
        event: EventWrite,
        options: EventWriteOptions,
        write_method: EventWriteMethod,
    ) -> ClientResult<Self> {
        let query = options.payload_query(
            write_method,
            event.conference_data.is_some(),
            event.attachments.is_some(),
        )?;
        Self::from_body(
            method,
            path,
            query,
            Some(serde_json::to_vec(&event)?),
            options.if_match,
        )
    }

    /// Patch only supplied properties, preserving omitted/set/null distinctions.
    /// # Errors
    /// Returns serialization, header, or conflicting conference-version errors.
    pub fn event_patch(
        calendar_id: &str,
        event_id: &str,
        event: EventPatch,
        options: EventWriteOptions,
    ) -> ClientResult<Self> {
        let query = options.payload_query(
            EventWriteMethod::Patch,
            !event.conference_data.is_unset(),
            !event.attachments.is_unset(),
        )?;
        Self::from_body(
            Method::PATCH,
            event_path(calendar_id, Some(event_id)),
            query,
            Some(serde_json::to_vec(&event)?),
            options.if_match,
        )
    }

    /// Delete an event, preserving notification options and `If-Match`.
    /// # Errors
    /// Returns an error for an invalid conditional header.
    pub fn event_delete(
        calendar_id: &str,
        event_id: &str,
        options: EventWriteOptions,
    ) -> ClientResult<Self> {
        let query = options.query_for(EventWriteMethod::Delete);
        Self::from_body(
            Method::DELETE,
            event_path(calendar_id, Some(event_id)),
            query,
            None,
            options.if_match,
        )
    }
}

fn event_path(calendar: &str, event: Option<&str>) -> String {
    let mut path = format!("/calendar/v3/calendars/{}/events", segment(calendar));
    if let Some(event) = event {
        path.push('/');
        path.push_str(&segment(event));
    }
    path
}

fn validate_path(path: &str) -> ClientResult<()> {
    if !path.starts_with("/calendar/v3/")
        || path.len() == "/calendar/v3/".len()
        || path
            .bytes()
            .any(|byte| !(0x21..=0x7e).contains(&byte) || matches!(byte, b'\\' | b'?' | b'#'))
    {
        return Err(invalid(
            "batch paths must be encoded paths inside /calendar/v3/, without queries or fragments",
        ));
    }
    // URL parsing normalizes literal and percent-encoded dot segments. Reject
    // normalization rather than allowing an apparent Calendar path to escape.
    let parsed = url::Url::parse(&format!("https://www.googleapis.com{path}"))?;
    if parsed.path() != path {
        return Err(invalid(
            "batch path must not contain dot segments or require URL normalization",
        ));
    }
    Ok(())
}

fn validate_header(value: &str) -> ClientResult<()> {
    if value.bytes().any(|byte| byte == b'\r' || byte == b'\n')
        || HeaderValue::from_str(value).is_err()
    {
        return Err(invalid("invalid batch HTTP header value"));
    }
    Ok(())
}

fn validate_id(id: &str) -> ClientResult<()> {
    if id.is_empty()
        || id
            .bytes()
            .any(|byte| !(0x21..=0x7e).contains(&byte) || matches!(byte, b'<' | b'>'))
    {
        return Err(invalid(
            "batch Content-ID must be nonempty visible ASCII without angle brackets",
        ));
    }
    Ok(())
}

/// Up to 1000 independent operations. Adding an invalid/duplicate operation leaves
/// the request unchanged. Empty requests are rejected before transport.
#[derive(Debug, Clone, Default)]
pub struct BatchRequest {
    operations: Vec<BatchOperation>,
}

impl BatchRequest {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            operations: Vec::new(),
        }
    }

    /// Append an operation, assigning an ID if needed.
    /// # Errors
    /// Returns an error for duplicate IDs or more than 1000 operations.
    pub fn push(&mut self, mut operation: BatchOperation) -> ClientResult<&mut Self> {
        if self.operations.len() == MAX_OPERATIONS {
            return Err(invalid("Calendar batches allow at most 1000 operations"));
        }
        if operation.id.is_none() {
            let mut suffix = self.operations.len() + 1;
            loop {
                let id = format!("item-{suffix}");
                if !self
                    .operations
                    .iter()
                    .any(|existing| existing.id.as_ref() == Some(&id))
                {
                    operation.id = Some(id);
                    break;
                }
                suffix += 1;
            }
        }
        if self
            .operations
            .iter()
            .any(|existing| existing.id == operation.id)
        {
            return Err(invalid("duplicate batch request Content-ID"));
        }
        self.operations.push(operation);
        Ok(self)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.operations.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }

    fn encode(&self) -> ClientResult<(String, Vec<u8>)> {
        if self.operations.is_empty() || self.operations.len() > MAX_OPERATIONS {
            return Err(invalid(
                "Calendar batches require 1 through 1000 operations",
            ));
        }
        let boundary = loop {
            let candidate = format!(
                "gcal_rs_batch_{}",
                NEXT_BOUNDARY.fetch_add(1, Ordering::Relaxed)
            );
            if !self.operations.iter().any(|operation| {
                operation
                    .body
                    .as_ref()
                    .is_some_and(|body| find_bytes(body, candidate.as_bytes()).is_some())
                    || operation.path.contains(&candidate)
                    || operation
                        .id
                        .as_ref()
                        .is_some_and(|id| id.contains(&candidate))
                    || operation
                        .if_match
                        .as_ref()
                        .is_some_and(|etag| etag.contains(&candidate))
            }) {
                break candidate;
            }
        };
        let mut body = Vec::new();
        for operation in &self.operations {
            let id = operation
                .id
                .as_deref()
                .ok_or_else(|| invalid("missing batch request Content-ID"))?;
            write!(body, "--{boundary}\r\nContent-Type: application/http\r\nContent-ID: <{id}>\r\n\r\n{} {} HTTP/1.1\r\n", operation.method, operation.path)
                .expect("writing to Vec<u8> cannot fail");
            if let Some(etag) = &operation.if_match {
                write!(body, "If-Match: {etag}\r\n").expect("writing to Vec<u8> cannot fail");
            }
            if let Some(payload) = &operation.body {
                write!(
                    body,
                    "Content-Type: application/json\r\nContent-Length: {}\r\n",
                    payload.len()
                )
                .expect("writing to Vec<u8> cannot fail");
            }
            body.extend_from_slice(b"\r\n");
            if let Some(payload) = &operation.body {
                body.extend_from_slice(payload);
            }
            body.extend_from_slice(b"\r\n");
        }
        write!(body, "--{boundary}--\r\n").expect("writing to Vec<u8> cannot fail");
        Ok((format!("multipart/mixed; boundary={boundary}"), body))
    }
}

/// Authenticated batch endpoint. The outer transport supplies the existing
/// client's credentials, refresh handling, HTTPS policy, and redirect protection.
#[derive(Debug, Clone)]
pub struct BatchClient(pub(crate) Arc<GCalClient>);

impl BatchClient {
    #[must_use]
    pub const fn new(client: Arc<GCalClient>) -> Self {
        Self(client)
    }

    /// Send independent operations in a single authenticated multipart request.
    ///
    /// Individual API failures remain in the response; only outer HTTP/transport
    /// failures or malformed/incomplete multipart responses fail this call.
    /// # Errors
    /// Returns validation, transport, outer API, or malformed-response errors.
    pub async fn send(&self, request: BatchRequest) -> ClientResult<BatchResponse> {
        let (content_type, body) = request.encode()?;
        let response = self
            .0
            .send_multipart("/batch/calendar/v3", &content_type, body)
            .await?;
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .ok_or_else(|| invalid("batch response is missing Content-Type"))?
            .to_str()
            .map_err(|_| invalid("invalid batch response Content-Type"))?
            .to_string();
        let bytes = response.bytes().await?;
        parse_response(&content_type, &bytes, &request)
    }
}

/// A complete response, in original request order (not server execution order).
#[derive(Debug, Clone)]
pub struct BatchResponse {
    pub items: Vec<BatchItem>,
}

impl BatchResponse {
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&BatchItem> {
        self.items.iter().find(|item| item.id == id)
    }
}

/// One correlated response. Raw bytes, headers and status remain accessible even
/// on an API error or an empty HTTP 204 response.
#[derive(Debug, Clone)]
pub struct BatchItem {
    pub id: String,
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Vec<u8>,
    pub error: Option<ApiError>,
}

impl BatchItem {
    /// Interpret an unsuccessful item as the ordinary structured API error.
    /// # Errors
    /// Returns the item's API error for a non-success status.
    pub fn result(&self) -> ClientResult<&Self> {
        match &self.error {
            Some(error) => Err(ClientError::Api(Box::new(error.clone()))),
            None => Ok(self),
        }
    }

    /// Decode a successful item's JSON. Empty HTTP 204 bodies should instead be
    /// inspected via `result()`/`status`; no synthetic JSON is supplied.
    /// # Errors
    /// Returns the item's API error or a JSON decoding error.
    pub fn json<T: DeserializeOwned>(&self) -> ClientResult<T> {
        self.result()?;
        Ok(serde_json::from_slice(&self.body)?)
    }
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn response_boundary(content_type: &str) -> ClientResult<String> {
    // Split parameters only outside quoted strings, so quoted boundaries and
    // unrelated quoted parameters do not confuse the MIME parser.
    let mut parameters = Vec::new();
    let mut start = 0;
    let mut quoted = false;
    let mut escaped = false;
    for (index, byte) in content_type.bytes().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }
        match byte {
            b'\\' if quoted => escaped = true,
            b'"' => quoted = !quoted,
            b';' if !quoted => {
                parameters.push(&content_type[start..index]);
                start = index + 1;
            }
            b'\r' | b'\n' => return Err(invalid("invalid multipart Content-Type")),
            _ => {}
        }
    }
    if quoted || escaped {
        return Err(invalid("unterminated multipart Content-Type quote"));
    }
    parameters.push(&content_type[start..]);
    if !parameters[0].trim().eq_ignore_ascii_case("multipart/mixed") {
        return Err(invalid(
            "batch response Content-Type must be multipart/mixed",
        ));
    }
    let mut boundary = None;
    for parameter in &parameters[1..] {
        let (name, value) = parameter
            .trim()
            .split_once('=')
            .ok_or_else(|| invalid("malformed multipart parameter"))?;
        if !name.trim().eq_ignore_ascii_case("boundary") {
            continue;
        }
        if boundary.is_some() {
            return Err(invalid("duplicate multipart boundary"));
        }
        let value = value.trim();
        let value = if value.starts_with('"') {
            let interior = value
                .strip_prefix('"')
                .and_then(|value| value.strip_suffix('"'))
                .ok_or_else(|| invalid("malformed quoted boundary"))?;
            let mut decoded = String::new();
            let mut chars = interior.chars();
            while let Some(character) = chars.next() {
                if character == '\\' {
                    decoded.push(
                        chars
                            .next()
                            .ok_or_else(|| invalid("malformed quoted boundary escape"))?,
                    );
                } else {
                    decoded.push(character);
                }
            }
            decoded
        } else {
            if value
                .bytes()
                .any(|byte| !byte.is_ascii_alphanumeric() && !b"!#$%&'*+-.^_`|~".contains(&byte))
            {
                return Err(invalid(
                    "multipart boundary containing MIME separators must be quoted",
                ));
            }
            value.to_string()
        };
        if value.is_empty()
            || value.len() > 70
            || value.ends_with(' ')
            || value
                .bytes()
                .any(|byte| !byte.is_ascii_alphanumeric() && !b"'()+_,-./:=? ".contains(&byte))
        {
            return Err(invalid("invalid multipart boundary"));
        }
        boundary = Some(value);
    }
    boundary.ok_or_else(|| invalid("batch response is missing multipart boundary"))
}

fn parse_headers(bytes: &[u8]) -> ClientResult<HeaderMap> {
    let text = std::str::from_utf8(bytes).map_err(|_| invalid("non-UTF-8 batch header"))?;
    let mut headers = HeaderMap::new();
    if text.is_empty() {
        return Ok(headers);
    }
    for line in text.split("\r\n") {
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| invalid("malformed batch header"))?;
        let name = HeaderName::from_bytes(name.as_bytes())
            .map_err(|_| invalid("invalid batch header name"))?;
        let value = value.trim_matches([' ', '\t']);
        validate_header(value)?;
        let value =
            HeaderValue::from_str(value).map_err(|_| invalid("invalid batch header value"))?;
        if (name == CONTENT_TYPE || name == CONTENT_LENGTH || name.as_str() == "content-id")
            && headers.contains_key(&name)
        {
            return Err(invalid("duplicate batch framing header"));
        }
        headers.append(name, value);
    }
    Ok(headers)
}

fn split_headers(bytes: &[u8]) -> ClientResult<(&[u8], &[u8])> {
    let position = find_bytes(bytes, b"\r\n\r\n")
        .ok_or_else(|| invalid("batch part is missing CRLF header terminator"))?;
    Ok((&bytes[..position], &bytes[position + 4..]))
}

fn parse_item(part: &[u8]) -> ClientResult<BatchItem> {
    let (mime, http) = split_headers(part)?;
    let mime = parse_headers(mime)?;
    let media_type = mime
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .map(str::trim);
    if !media_type.is_some_and(|value| value.eq_ignore_ascii_case("application/http")) {
        return Err(invalid("batch response part must be application/http"));
    }
    let id = mime
        .get("content-id")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix('<'))
        .and_then(|value| value.strip_suffix('>'))
        .and_then(|value| value.strip_prefix("response-"))
        .ok_or_else(|| invalid("batch response is missing a valid response Content-ID"))?;
    validate_id(id)?;
    let (head, body) = split_headers(http)?;
    let status_end = find_bytes(head, b"\r\n").unwrap_or(head.len());
    let status_line = std::str::from_utf8(&head[..status_end])
        .map_err(|_| invalid("invalid batch HTTP status line"))?;
    let mut words = status_line.splitn(3, ' ');
    if !matches!(words.next(), Some("HTTP/1.1" | "HTTP/1.0")) {
        return Err(invalid("invalid batch HTTP version"));
    }
    let code = words
        .next()
        .ok_or_else(|| invalid("missing batch HTTP status"))?;
    if code.len() != 3 || !code.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(invalid("invalid batch HTTP status"));
    }
    let status = StatusCode::from_bytes(code.as_bytes())
        .map_err(|_| invalid("invalid batch HTTP status"))?;
    if status.is_informational() {
        return Err(invalid("batch part must contain a final HTTP status"));
    }
    let headers = if status_end == head.len() {
        HeaderMap::new()
    } else {
        parse_headers(&head[status_end + 2..])?
    };
    if headers.contains_key("transfer-encoding") {
        return Err(invalid(
            "unsupported transfer encoding in batch response part",
        ));
    }
    if let Some(length) = headers.get(CONTENT_LENGTH) {
        let length = length
            .to_str()
            .map_err(|_| invalid("invalid batch Content-Length"))?;
        if length.is_empty() || !length.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(invalid("invalid batch Content-Length"));
        }
        let length: usize = length
            .parse()
            .map_err(|_| invalid("invalid batch Content-Length"))?;
        if length != body.len() {
            return Err(invalid("batch response Content-Length mismatch"));
        }
    }
    if status == StatusCode::NO_CONTENT && !body.is_empty() {
        return Err(invalid("HTTP 204 batch part contains a body"));
    }
    let error = (!status.is_success()).then(|| ApiError::from_response(status, body));
    Ok(BatchItem {
        id: id.to_string(),
        status,
        headers,
        body: body.to_vec(),
        error,
    })
}

fn parse_response(
    content_type: &str,
    bytes: &[u8],
    request: &BatchRequest,
) -> ClientResult<BatchResponse> {
    let boundary = response_boundary(content_type)?;
    let delimiter = format!("--{boundary}");
    let delimiter = delimiter.as_bytes();
    let mut cursor = 0;
    let mut part_start = None;
    let mut parts = HashMap::new();
    let expected: HashSet<&str> = request
        .operations
        .iter()
        .filter_map(|operation| operation.id.as_deref())
        .collect();
    let mut opened = false;
    let mut closed = false;
    while cursor <= bytes.len().saturating_sub(delimiter.len()) {
        let Some(relative) = find_bytes(&bytes[cursor..], delimiter) else {
            break;
        };
        let position = cursor + relative;
        let after = position + delimiter.len();
        cursor = after;
        if position != 0 && (position < 2 || &bytes[position - 2..position] != b"\r\n") {
            continue;
        }
        let closing = bytes.get(after..after + 2) == Some(b"--");
        let line_end = if closing { after + 2 } else { after };
        // RFC 2046 permits trailing linear whitespace on delimiter lines.
        let mut end = line_end;
        while matches!(bytes.get(end), Some(b' ' | b'\t')) {
            end += 1;
        }
        if end != bytes.len() && bytes.get(end..end + 2) != Some(b"\r\n") {
            continue;
        }
        if let Some(start) = part_start {
            let part_end = position
                .checked_sub(2)
                .ok_or_else(|| invalid("malformed batch delimiter"))?;
            if part_end < start {
                return Err(invalid("empty batch response part"));
            }
            let item = parse_item(&bytes[start..part_end])?;
            if !expected.contains(item.id.as_str()) {
                return Err(invalid("unexpected batch response Content-ID"));
            }
            let id = item.id.clone();
            if parts.insert(id, item).is_some() {
                return Err(invalid("duplicate batch response Content-ID"));
            }
        }
        if closing {
            closed = true;
            break;
        }
        opened = true;
        if end == bytes.len() {
            return Err(invalid("truncated batch delimiter"));
        }
        part_start = Some(end + 2);
        cursor = end + 2;
    }
    if !opened || !closed {
        return Err(invalid(
            "batch response is missing opening or closing boundary",
        ));
    }
    if parts.len() != request.operations.len() {
        return Err(invalid("batch response is missing operation results"));
    }
    let mut items = Vec::with_capacity(parts.len());
    for operation in &request.operations {
        let id = operation
            .id
            .as_deref()
            .ok_or_else(|| invalid("missing batch request ID"))?;
        items.push(
            parts
                .remove(id)
                .ok_or_else(|| invalid("batch response is missing operation result"))?,
        );
    }
    Ok(BatchResponse { items })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::types::EventConferenceDataWrite;
    use crate::test_support::fixture_with_content_type;
    use crate::Patch;

    const MIXED: &str = "--reply\r\nContent-Type: application/http\r\nContent-ID: <response-conflict>\r\n\r\nHTTP/1.1 412 Precondition Failed\r\nContent-Type: application/json\r\n\r\n{\"error\":{\"message\":\"stale event\",\"errors\":[{\"reason\":\"conditionNotMet\"}]}}\r\n--reply\r\nContent-Type: application/http\r\nContent-ID: <response-deleted>\r\n\r\nHTTP/1.1 204 No Content\r\n\r\n\r\n--reply\r\nContent-Type: application/http\r\nContent-ID: <response-created>\r\n\r\nHTTP/1.1 201 Created\r\nContent-Type: application/json\r\nETag: \"new\"\r\n\r\n{\"id\":\"new-event\",\"summary\":\"Created\"}\r\n--reply--\r\n";

    fn read_operation() -> BatchOperation {
        BatchOperation::event_get("primary", "event", QueryParams::new()).unwrap()
    }

    fn single_request() -> BatchRequest {
        let mut request = BatchRequest::new();
        request
            .push(read_operation().with_id("one").unwrap())
            .unwrap();
        request
    }

    fn part(id: &str, status: &str, headers: &str, body: &str) -> String {
        format!("--b\r\nContent-Type: application/http\r\nContent-ID: <response-{id}>\r\n\r\nHTTP/1.1 {status}\r\n{headers}\r\n{body}\r\n")
    }

    #[tokio::test]
    async fn sends_authenticated_safe_writes_and_preserves_mixed_results_in_request_order() {
        let (client, observed) =
            fixture_with_content_type(vec![(200, MIXED)], "multipart/mixed; boundary=\"reply\"")
                .await;
        let mut request = BatchRequest::new();
        request
            .push(
                BatchOperation::event_insert(
                    "team/a+b@example.com",
                    EventWrite {
                        summary: Some("Created".into()),
                        guests_can_invite_others: Some(false),
                        conference_data: Some(EventConferenceDataWrite::default()),
                        attachments: Some(vec![]),
                        ..Default::default()
                    },
                    EventWriteOptions::default(),
                )
                .unwrap()
                .with_id("created")
                .unwrap(),
            )
            .unwrap();
        request
            .push(
                BatchOperation::event_delete("primary", "gone/id", EventWriteOptions::default())
                    .unwrap()
                    .with_id("deleted")
                    .unwrap(),
            )
            .unwrap();
        request
            .push(
                BatchOperation::event_patch(
                    "primary",
                    "changed",
                    EventPatch {
                        description: Patch::Null,
                        ..Default::default()
                    },
                    EventWriteOptions {
                        if_match: Some("\"old\"".into()),
                        ..Default::default()
                    },
                )
                .unwrap()
                .with_id("conflict")
                .unwrap(),
            )
            .unwrap();
        let response = BatchClient::new(client).send(request).await.unwrap();
        assert_eq!(
            response
                .items
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            ["created", "deleted", "conflict"]
        );
        let event: crate::Event = response.get("created").unwrap().json().unwrap();
        assert_eq!(event.id, "new-event");
        assert_eq!(response.items[0].headers.get("etag").unwrap(), "\"new\"");
        assert_eq!(response.items[1].status, StatusCode::NO_CONTENT);
        assert!(response.items[1].result().unwrap().body.is_empty());
        let error = response.items[2].json::<crate::Event>().unwrap_err();
        assert_eq!(error.status(), Some(StatusCode::PRECONDITION_FAILED));
        assert_eq!(
            response.items[2].error.as_ref().unwrap().errors[0].reason,
            "conditionNotMet"
        );
        let requests = observed.await.unwrap();
        let outer = &requests[0];
        assert_eq!(outer.method, "POST");
        assert_eq!(outer.target, "/batch/calendar/v3");
        assert!(outer
            .headers
            .get("authorization")
            .unwrap()
            .starts_with("Bearer "));
        let boundary = response_boundary(outer.headers.get("content-type").unwrap()).unwrap();
        let encoded = std::str::from_utf8(&outer.body).unwrap();
        let parts: Vec<_> = encoded
            .split(&format!("--{boundary}\r\n"))
            .skip(1)
            .collect();
        assert_eq!(parts.len(), 3);
        let (_, insert) = split_headers(parts[0].as_bytes()).unwrap();
        let (head, body) = split_headers(insert).unwrap();
        let head = std::str::from_utf8(head).unwrap();
        let target = head
            .lines()
            .next()
            .unwrap()
            .split_whitespace()
            .nth(1)
            .unwrap();
        let url = url::Url::parse(&format!("https://www.googleapis.com{target}")).unwrap();
        assert_eq!(
            url.path(),
            "/calendar/v3/calendars/team%2Fa%2Bb%40example.com/events"
        );
        let query: HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(query.get("conferenceDataVersion").unwrap(), "1");
        assert_eq!(query.get("supportsAttachments").unwrap(), "true");
        let json: serde_json::Value = serde_json::from_slice(body.trim_ascii_end()).unwrap();
        assert_eq!(json["guestsCanInviteOthers"], false);
        assert!(json.get("kind").is_none());
        assert!(json.get("etag").is_none());
        assert!(
            parts[1].contains("DELETE /calendar/v3/calendars/primary/events/gone%2Fid HTTP/1.1")
        );
        assert!(parts[2].contains("If-Match: \"old\"\r\n"));
        assert!(parts[2].contains("\"description\":null"));
    }

    #[test]
    fn rejects_unsafe_paths_headers_ids_and_duplicate_requests() {
        for path in [
            "https://evil.test/calendar/v3/events",
            "//evil.test/calendar/v3/events",
            "/drive/v3/files",
            "/calendar/v3/../../drive/v3/files",
            "/calendar/v3/%2e%2e/escape",
            "/calendar/v3/events\r\nHost: evil",
            "/calendar/v3/events?access_token=secret",
            "/calendar/v3/events#fragment",
            "/calendar/v3/\\evil",
        ] {
            assert!(
                BatchOperation::raw(Method::GET, path, QueryParams::new(), None).is_err(),
                "{path}"
            );
        }
        assert!(BatchOperation::raw(
            Method::CONNECT,
            "/calendar/v3/events",
            QueryParams::new(),
            None
        )
        .is_err());
        assert!(read_operation()
            .if_match("safe\r\nAuthorization: injected")
            .is_err());
        assert!(read_operation().with_id("bad\r\nX: y").is_err());
        assert!(read_operation().with_id("<wrapped>").is_err());
        let mut request = single_request();
        assert!(request
            .push(read_operation().with_id("one").unwrap())
            .is_err());
        assert_eq!(request.len(), 1);
    }

    #[test]
    fn enforces_limits_and_assigns_unique_ids_without_overwriting_explicit_ids() {
        assert!(BatchRequest::new().encode().is_err());
        let mut request = BatchRequest::new();
        request
            .push(read_operation().with_id("item-2").unwrap())
            .unwrap();
        request.push(read_operation()).unwrap();
        assert_eq!(request.operations[1].id.as_deref(), Some("item-3"));
        for _ in 2..MAX_OPERATIONS {
            request.push(read_operation()).unwrap();
        }
        assert!(request.encode().is_ok());
        assert!(request.push(read_operation()).is_err());
        assert_eq!(request.len(), 1000);
    }

    #[test]
    fn typed_writes_preserve_options_and_reject_conflicting_conference_versions() {
        let event = EventWrite {
            conference_data: Some(EventConferenceDataWrite::default()),
            ..Default::default()
        };
        assert!(BatchOperation::event_insert(
            "primary",
            event.clone(),
            EventWriteOptions {
                conference_data_version: Some(0),
                ..Default::default()
            }
        )
        .is_err());
        assert!(BatchOperation::event_patch(
            "primary",
            "id",
            EventPatch {
                conference_data: Patch::Null,
                ..Default::default()
            },
            EventWriteOptions {
                conference_data_version: Some(0),
                ..Default::default()
            }
        )
        .is_err());
        let update = BatchOperation::event_update(
            "primary",
            "id",
            event,
            EventWriteOptions {
                if_match: Some("\"etag\"".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(update.method, Method::PUT);
        assert!(update.path.contains("conferenceDataVersion=1"));
        assert_eq!(update.if_match.as_deref(), Some("\"etag\""));
        let patch = BatchOperation::event_patch(
            "primary",
            "id",
            EventPatch {
                attachments: Patch::Null,
                ..Default::default()
            },
            EventWriteOptions::default(),
        )
        .unwrap();
        assert!(patch.path.contains("supportsAttachments=true"));
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(patch.body.as_ref().unwrap()).unwrap()
                ["attachments"],
            serde_json::Value::Null
        );
    }

    #[test]
    fn validates_mime_boundaries_and_part_completeness() {
        let request = single_request();
        let valid = format!(
            "{}--b--\r\n",
            part("one", "200 OK", "Content-Type: application/json\r\n", "{}")
        );
        assert!(parse_response(
            "multipart/mixed; charset=\"semi;colon\"; boundary=\"b\"",
            valid.as_bytes(),
            &request
        )
        .is_ok());
        for content_type in [
            "application/json",
            "multipart/mixed",
            "multipart/mixed; boundary=",
            "multipart/mixed; boundary=\"b",
            "multipart/mixed; boundary=b; boundary=b",
            "multipart/mixed; boundary=\"bad;boundary\"",
        ] {
            assert!(
                parse_response(content_type, valid.as_bytes(), &request).is_err(),
                "{content_type}"
            );
        }
        for body in [
            valid.replace("--b--\r\n", ""),
            format!(
                "{}{}--b--\r\n",
                part("one", "200 OK", "", "{}"),
                part("one", "412 Conflict", "", "{}")
            ),
            format!("{}--b--\r\n", part("unexpected", "200 OK", "", "{}")),
            "--b--\r\n".into(),
            valid.replace("Content-ID: <response-one>\r\n", ""),
            valid.replace(
                "Content-ID: <response-one>\r\n",
                "Content-ID: <response-one>\r\nContent-ID: <response-one>\r\n",
            ),
            valid.replace("HTTP/1.1 200", "HTTP/1.1 nope"),
            valid.replace(
                "Content-Type: application/json\r\n",
                "Content-Length: 500\r\n",
            ),
            valid.replace(
                "Content-Type: application/json\r\n",
                "Content-Length: +2\r\n",
            ),
            valid.replace(
                "Content-Type: application/json\r\n",
                "Transfer-Encoding: chunked\r\n",
            ),
            valid.replace("HTTP/1.1 200 OK", "HTTP/1.1 204 No Content"),
            valid.replace(
                "Content-Type: application/json\r\n",
                "Folded: value\r\n injected\r\n",
            ),
            valid.replace("\r\n", "\n"),
        ] {
            assert!(
                parse_response("multipart/mixed; boundary=b", body.as_bytes(), &request).is_err(),
                "{body:?}"
            );
        }
    }

    #[test]
    fn preserves_raw_bytes_and_ignores_boundary_substrings_inside_bodies() {
        let mut body = part(
            "one",
            "200 OK",
            "X-Custom: retained\r\n",
            "before\r\n--b-not-a-delimiter\r\nafter",
        )
        .into_bytes();
        body.extend_from_slice(b"--b--\r\n");
        let response =
            parse_response("multipart/mixed; boundary=b", &body, &single_request()).unwrap();
        assert_eq!(
            response.items[0].body,
            b"before\r\n--b-not-a-delimiter\r\nafter"
        );
        assert_eq!(response.items[0].headers["x-custom"], "retained");
        assert!(response.items[0].json::<serde_json::Value>().is_err());
        let mut binary = part("one", "200 OK", "", "").into_bytes();
        binary.truncate(binary.len() - 2);
        binary.extend_from_slice(b"\xff\x00\r\n--b--\r\n");
        let response =
            parse_response("multipart/mixed; boundary=b", &binary, &single_request()).unwrap();
        assert_eq!(response.items[0].body, b"\xff\x00");
    }

    #[test]
    fn encoder_avoids_payload_boundary_collisions_and_preserves_repeated_queries() {
        let candidate = format!("gcal_rs_batch_{}", NEXT_BOUNDARY.load(Ordering::Relaxed));
        let payload = serde_json::json!({ "description": format!("\r\n--{candidate}\r\n") });
        let mut query = QueryParams::new();
        query.append("privateExtendedProperty".into(), "a=one".into());
        query.append("privateExtendedProperty".into(), "b=two".into());
        let operation = BatchOperation::raw(
            Method::POST,
            "/calendar/v3/calendars/primary/events",
            query,
            Some(payload),
        )
        .unwrap();
        let original_body = operation.body.clone().unwrap();
        let mut request = BatchRequest::new();
        request.push(operation).unwrap();
        let (content_type, bytes) = request.encode().unwrap();
        let boundary = response_boundary(&content_type).unwrap();
        assert!(find_bytes(&original_body, boundary.as_bytes()).is_none());
        let encoded = std::str::from_utf8(&bytes).unwrap();
        let (_, http) = split_headers(encoded.as_bytes()).unwrap();
        let (headers, _) = split_headers(http).unwrap();
        let line = std::str::from_utf8(headers)
            .unwrap()
            .lines()
            .next()
            .unwrap();
        let target = line.split_whitespace().nth(1).unwrap();
        let url = url::Url::parse(&format!("https://www.googleapis.com{target}")).unwrap();
        assert_eq!(
            url.query_pairs()
                .filter(|(name, _)| name == "privateExtendedProperty")
                .count(),
            2
        );
    }

    #[tokio::test]
    async fn malformed_transport_response_fails_instead_of_losing_results() {
        let (client, observed) =
            fixture_with_content_type(vec![(200, "--b--\r\n")], "multipart/mixed; boundary=b")
                .await;
        assert!(BatchClient::new(client)
            .send(single_request())
            .await
            .is_err());
        assert_eq!(observed.await.unwrap().len(), 1);
    }
}
