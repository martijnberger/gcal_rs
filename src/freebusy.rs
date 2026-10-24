//! Free/busy queries with independent calendar and group results.
//!
//! A successful HTTP response can contain partial failures. Inspect each calendar's
//! and group's `errors` before interpreting an empty busy list as availability.
//! Time strings follow RFC3339; busy periods include their start and exclude their end.
//!
//! See <https://developers.google.com/workspace/calendar/api/v3/reference/freebusy/query>.

use std::{collections::BTreeMap, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::{request::ApiRequest, ClientResult, GCalClient};

/// Query window and calendars/groups. Optional values are omitted, not default-injected.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FreeBusyRequest {
    /// Inclusive query start, formatted as RFC3339.
    pub time_min: String,
    /// Exclusive query end, formatted as RFC3339.
    pub time_max: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_zone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_expansion_max: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calendar_expansion_max: Option<i32>,
    pub items: Vec<FreeBusyRequestItem>,
}

impl FreeBusyRequest {
    /// Construct a query with Google's default response timezone and expansion limits.
    #[must_use]
    pub fn new(
        time_min: impl Into<String>,
        time_max: impl Into<String>,
        items: Vec<FreeBusyRequestItem>,
    ) -> Self {
        Self {
            time_min: time_min.into(),
            time_max: time_max.into(),
            time_zone: None,
            group_expansion_max: None,
            calendar_expansion_max: None,
            items,
        }
    }
}

/// Calendar or group identifier to expand.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct FreeBusyRequestItem {
    pub id: String,
}

/// Domain/reason information for a failed calendar lookup or group expansion.
/// Unknown future reasons are preserved as strings.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct FreeBusyError {
    pub domain: String,
    pub reason: String,
}

/// A half-open busy interval: `start` is inclusive and `end` is exclusive.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimePeriod {
    /// RFC3339 timestamp.
    pub start: String,
    /// RFC3339 timestamp.
    pub end: String,
}

/// Results for one calendar. Errors are not promoted to a whole-request failure.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct FreeBusyCalendar {
    pub errors: Vec<FreeBusyError>,
    pub busy: Vec<TimePeriod>,
}

/// Results for one group, including any partial expansion errors.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct FreeBusyGroup {
    pub errors: Vec<FreeBusyError>,
    pub calendars: Vec<String>,
}

/// Successful HTTP response, potentially containing per-calendar/group errors.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct FreeBusyResponse {
    pub kind: String,
    pub time_min: String,
    pub time_max: String,
    pub groups: BTreeMap<String, FreeBusyGroup>,
    pub calendars: BTreeMap<String, FreeBusyCalendar>,
}

/// Query calendar availability without discarding partial successes or failures.
#[derive(Debug, Clone)]
pub struct FreeBusyClient(pub(crate) Arc<GCalClient>);

impl FreeBusyClient {
    #[must_use]
    pub const fn new(client: Arc<GCalClient>) -> Self {
        Self(client)
    }

    /// Query availability. A 2xx response with item errors remains a successful result.
    ///
    /// # Errors
    /// Returns transport, API, or response decoding errors. Calendar/group failures
    /// inside a successful response are retained in their respective `errors` vectors.
    pub async fn query(&self, request: FreeBusyRequest) -> ClientResult<FreeBusyResponse> {
        Ok(self
            .0
            .post(None, ApiRequest::with_body("freeBusy".into(), request))
            .await?
            .json()
            .await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::fixture;

    #[tokio::test]
    async fn query_preserves_valid_busy_intervals_and_all_partial_errors() {
        let (client, observed) = fixture(vec![(200, r#"{
            "kind":"calendar#freeBusy",
            "timeMin":"2026-10-24T08:00:00Z","timeMax":"2026-10-24T18:00:00Z",
            "groups":{
                "team@example.com":{"calendars":["available","busy"],"errors":[{"domain":"calendar","reason":"groupTooBig"}]},
                "future-group":{"errors":[{"domain":"future","reason":"newExpansionFailure"}]}
            },
            "calendars":{
                "available":{"busy":[]},
                "busy":{"busy":[{"start":"2026-10-24T10:00:00+02:00","end":"2026-10-24T11:30:00+02:00"}]},
                "unavailable":{"errors":[{"domain":"global","reason":"notFound"},{"domain":"calendar","reason":"internalError"}]}
            }
        }"#)]).await;
        let mut request = FreeBusyRequest::new(
            "2026-10-24T08:00:00Z",
            "2026-10-24T18:00:00Z",
            vec![
                FreeBusyRequestItem {
                    id: "team@example.com".into(),
                },
                FreeBusyRequestItem {
                    id: "unavailable".into(),
                },
            ],
        );
        request.time_zone = Some("Europe/Amsterdam".into());
        request.group_expansion_max = Some(100);
        request.calendar_expansion_max = Some(50);
        let result = FreeBusyClient::new(client).query(request).await.unwrap();
        assert_eq!(result.kind, "calendar#freeBusy");
        assert_eq!(
            result.groups["team@example.com"].calendars,
            ["available", "busy"]
        );
        assert_eq!(
            result.groups["team@example.com"].errors[0].reason,
            "groupTooBig"
        );
        assert_eq!(
            result.groups["future-group"].errors[0].reason,
            "newExpansionFailure"
        );
        assert_eq!(result.calendars["unavailable"].errors.len(), 2);
        assert_eq!(result.calendars["unavailable"].errors[0].reason, "notFound");
        assert_eq!(
            result.calendars["unavailable"].errors[1].reason,
            "internalError"
        );
        assert!(result.calendars["available"].errors.is_empty());
        assert!(result.calendars["available"].busy.is_empty());
        let interval = &result.calendars["busy"].busy[0];
        let start = chrono::DateTime::parse_from_rfc3339(&interval.start).unwrap();
        let end = chrono::DateTime::parse_from_rfc3339(&interval.end).unwrap();
        let window_start = chrono::DateTime::parse_from_rfc3339(&result.time_min).unwrap();
        let window_end = chrono::DateTime::parse_from_rfc3339(&result.time_max).unwrap();
        assert!(window_start <= start && start < end && end <= window_end);
        assert_eq!(end.signed_duration_since(start).num_minutes(), 90);
        let requests = observed.await.unwrap();
        assert_eq!(requests[0].method, "POST");
        assert_eq!(requests[0].target, "/calendar/v3/freeBusy");
        assert_eq!(requests[0].headers["content-type"], "application/json");
        let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
        assert_eq!(
            body,
            serde_json::json!({
                "timeMin":"2026-10-24T08:00:00Z","timeMax":"2026-10-24T18:00:00Z",
                "timeZone":"Europe/Amsterdam","groupExpansionMax":100,"calendarExpansionMax":50,
                "items":[{"id":"team@example.com"},{"id":"unavailable"}]
            })
        );
    }

    #[test]
    fn request_defaults_do_not_inject_options_and_sparse_results_decode() {
        let request = FreeBusyRequest::new(
            "2026-10-24T08:00:00Z",
            "2026-10-24T09:00:00Z",
            vec![FreeBusyRequestItem {
                id: "primary".into(),
            }],
        );
        assert_eq!(
            serde_json::to_value(request).unwrap(),
            serde_json::json!({
                "timeMin":"2026-10-24T08:00:00Z","timeMax":"2026-10-24T09:00:00Z",
                "items":[{"id":"primary"}]
            })
        );
        let response: FreeBusyResponse = serde_json::from_value(serde_json::json!({
            "calendars":{"failed":{"errors":[{"domain":"calendar","reason":"futureFailure"}]}},
            "groups":{"empty":{}}
        }))
        .unwrap();
        assert!(response.calendars["failed"].busy.is_empty());
        assert_eq!(
            response.calendars["failed"].errors[0].reason,
            "futureFailure"
        );
        assert!(response.groups["empty"].calendars.is_empty());
        let round_trip: FreeBusyResponse =
            serde_json::from_value(serde_json::to_value(&response).unwrap()).unwrap();
        assert_eq!(response, round_trip);
    }
}
