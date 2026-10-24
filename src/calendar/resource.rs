use std::sync::Arc;

use super::{Calendar, CalendarPatch, CalendarWrite};
use crate::{
    request::{segment, ApiRequest},
    ClientError, ClientResult, GCalClient, QueryParams,
};

/// Access calendar metadata and lifecycle operations, distinct from the user's
/// calendar-list preferences exposed by [`super::CalendarListClient`].
#[derive(Debug, Clone)]
pub struct CalendarClient(pub(crate) Arc<GCalClient>);

impl CalendarClient {
    #[must_use]
    pub const fn new(client: Arc<GCalClient>) -> Self {
        Self(client)
    }

    /// Read a calendar's metadata, using `primary` for the user's primary calendar.
    ///
    /// # Errors
    /// Returns API, transport, URL, or response decoding errors.
    pub async fn get(&self, calendar_id: &str, options: CalendarOptions) -> ClientResult<Calendar> {
        let mut request = ApiRequest::new(calendar_path(calendar_id));
        request.query = options.extra;
        Ok(self.0.get(None, request).await?.json().await?)
    }

    /// Create a secondary calendar. Its title must be supplied in `summary`.
    ///
    /// # Errors
    /// Returns an invalid-request error if summary is absent, or API, transport,
    /// serialization, URL, or response decoding errors.
    pub async fn insert(
        &self,
        calendar: CalendarWrite,
        options: CalendarOptions,
    ) -> ClientResult<Calendar> {
        if calendar.summary.is_none() {
            return Err(ClientError::InvalidRequest(
                "summary is required when inserting a calendar".into(),
            ));
        }
        let mut request = ApiRequest::with_body("calendars".into(), calendar);
        request.query = options.extra;
        Ok(self.0.post(None, request).await?.json().await?)
    }

    /// Replace calendar metadata, optionally requiring a matching ETag.
    ///
    /// # Errors
    /// Returns API errors (including HTTP 412), transport, serialization, URL,
    /// invalid header, or response decoding errors.
    pub async fn update(
        &self,
        calendar_id: &str,
        calendar: CalendarWrite,
        options: CalendarOptions,
    ) -> ClientResult<Calendar> {
        let mut request = ApiRequest::with_body(calendar_path(calendar_id), calendar);
        request.query = options.extra;
        request.if_match = options.if_match;
        Ok(self.0.put(None, request).await?.json().await?)
    }

    /// Apply only specified metadata changes, optionally requiring a matching ETag.
    ///
    /// # Errors
    /// Returns API errors (including HTTP 412), transport, serialization, URL,
    /// invalid header, or response decoding errors.
    pub async fn patch(
        &self,
        calendar_id: &str,
        calendar: CalendarPatch,
        options: CalendarOptions,
    ) -> ClientResult<Calendar> {
        let mut request = ApiRequest::with_body(calendar_path(calendar_id), calendar);
        request.query = options.extra;
        request.if_match = options.if_match;
        Ok(self.0.patch(None, request).await?.json().await?)
    }

    /// Delete a secondary calendar. To remove events from a primary calendar use
    /// [`Self::clear`] instead. An ETag can guard against concurrent changes.
    ///
    /// # Errors
    /// Returns API errors (including HTTP 412), transport, URL, or invalid header errors.
    pub async fn delete(&self, calendar_id: &str, options: CalendarOptions) -> ClientResult<()> {
        let mut request = ApiRequest::new(calendar_path(calendar_id));
        request.query = options.extra;
        request.if_match = options.if_match;
        self.0.delete(None, request).await?;
        Ok(())
    }

    /// Delete all events in a primary calendar without deleting the calendar.
    ///
    /// # Errors
    /// Returns API, transport, or URL errors.
    pub async fn clear(&self, calendar_id: &str, options: CalendarOptions) -> ClientResult<()> {
        let mut request = ApiRequest::new(format!("{}/clear", calendar_path(calendar_id)));
        request.query = options.extra;
        self.0.post(None, request).await?;
        Ok(())
    }

    /// Transfer an active secondary calendar to another user in the same Workspace
    /// organization. Google requires administrator Manage Calendars privileges and
    /// `use_admin_access: true`; this method never retries a failed transfer.
    ///
    /// # Errors
    /// Returns API, transport, or URL errors.
    pub async fn transfer_ownership(
        &self,
        calendar_id: &str,
        options: CalendarTransferOwnershipOptions,
    ) -> ClientResult<()> {
        let mut request =
            ApiRequest::new(format!("{}/transferOwnership", calendar_path(calendar_id)));
        request.query = options.extra;
        request
            .query
            .insert("newDataOwner".into(), options.new_data_owner);
        request.query.insert(
            "useAdminAccess".into(),
            options.use_admin_access.to_string(),
        );
        self.0.post(None, request).await?;
        Ok(())
    }
}

fn calendar_path(calendar_id: &str) -> String {
    format!("calendars/{}", segment(calendar_id))
}

/// Options for calendar operations. ETag preconditions are used by update,
/// patch, and delete only. Raw pairs support global and future query parameters.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CalendarOptions {
    pub if_match: Option<String>,
    pub extra: QueryParams,
}

/// Required ownership-transfer arguments. Typed values override matching raw pairs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarTransferOwnershipOptions {
    pub new_data_owner: String,
    /// Google currently accepts only `true` and requires administrator privileges.
    pub use_admin_access: bool,
    pub extra: QueryParams,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{calendar::LabelProperties, test_support::fixture, Patch};

    #[tokio::test]
    async fn metadata_writes_use_safe_payloads_and_preserve_explicit_clears() {
        let (client, observed) = fixture(vec![
            (200, r#"{"id":"created","summary":"Team","etag":"one"}"#),
            (200, r#"{"id":"created","summary":"Renamed","etag":"two"}"#),
            (
                200,
                r#"{"id":"created","summary":"Renamed","etag":"three"}"#,
            ),
            (
                200,
                r#"{"id":"created","summary":"Renamed","dataOwner":"owner@example.com"}"#,
            ),
        ])
        .await;
        let calendars = CalendarClient::new(client);
        let created = calendars
            .insert(
                CalendarWrite {
                    summary: Some("Team".into()),
                    auto_accept_invitations: Some(false),
                    ..CalendarWrite::default()
                },
                CalendarOptions::default(),
            )
            .await
            .unwrap();
        assert_eq!(created.id, "created");
        let updated = calendars
            .update(
                "team/a+b@example.com",
                CalendarWrite {
                    summary: Some("Renamed".into()),
                    label_properties: Some(LabelProperties {
                        event_labels: Some(vec![]),
                    }),
                    ..CalendarWrite::default()
                },
                CalendarOptions {
                    if_match: Some("\"one\"".into()),
                    ..CalendarOptions::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(updated.etag, "two");
        let patched = calendars
            .patch(
                "team/a+b@example.com",
                CalendarPatch {
                    description: Patch::Null,
                    auto_accept_invitations: Patch::Value(false),
                    ..CalendarPatch::default()
                },
                CalendarOptions {
                    if_match: Some("\"two\"".into()),
                    ..CalendarOptions::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(patched.etag, "three");
        let fetched = calendars
            .get("team/a+b@example.com", CalendarOptions::default())
            .await
            .unwrap();
        assert_eq!(fetched.data_owner.as_deref(), Some("owner@example.com"));
        let requests = observed.await.unwrap();
        assert_eq!(requests[0].method, "POST");
        assert_eq!(requests[0].target, "/calendar/v3/calendars");
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&requests[0].body).unwrap(),
            serde_json::json!({"summary":"Team","autoAcceptInvitations":false})
        );
        assert_eq!(requests[1].method, "PUT");
        assert_eq!(
            requests[1].target,
            "/calendar/v3/calendars/team%2Fa%2Bb%40example.com"
        );
        assert_eq!(requests[1].headers["if-match"], "\"one\"");
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&requests[1].body).unwrap(),
            serde_json::json!({"summary":"Renamed","labelProperties":{"eventLabels":[]}})
        );
        assert_eq!(requests[2].method, "PATCH");
        assert_eq!(requests[2].headers["if-match"], "\"two\"");
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&requests[2].body).unwrap(),
            serde_json::json!({"description":null,"autoAcceptInvitations":false})
        );
        assert_eq!(requests[3].method, "GET");
        assert!(requests[3].body.is_empty());
        for request in &requests[..3] {
            assert_eq!(request.headers["content-type"], "application/json");
        }
    }

    #[tokio::test]
    async fn lifecycle_actions_are_bodyless_and_transfer_uses_required_query_arguments() {
        let (client, observed) = fixture(vec![(204, ""), (204, ""), (204, "")]).await;
        let calendars = CalendarClient::new(client);
        calendars
            .delete(
                "team/a+b@example.com",
                CalendarOptions {
                    if_match: Some("\"current\"".into()),
                    ..CalendarOptions::default()
                },
            )
            .await
            .unwrap();
        calendars
            .clear("primary", CalendarOptions::default())
            .await
            .unwrap();
        let mut extra = QueryParams::new();
        extra.append("newDataOwner".into(), "wrong".into());
        extra.append("newDataOwner".into(), "also-wrong".into());
        extra.append("useAdminAccess".into(), "false".into());
        extra.append("fields".into(), "id".into());
        calendars
            .transfer_ownership(
                "team/a+b@example.com",
                CalendarTransferOwnershipOptions {
                    new_data_owner: "new+owner@example.com".into(),
                    use_admin_access: true,
                    extra,
                },
            )
            .await
            .unwrap();
        let requests = observed.await.unwrap();
        assert_eq!(requests[0].method, "DELETE");
        assert_eq!(
            requests[0].target,
            "/calendar/v3/calendars/team%2Fa%2Bb%40example.com"
        );
        assert_eq!(requests[0].headers["if-match"], "\"current\"");
        assert_eq!(requests[1].method, "POST");
        assert_eq!(requests[1].target, "/calendar/v3/calendars/primary/clear");
        assert_eq!(requests[2].method, "POST");
        let url = url::Url::parse(&format!("http://localhost{}", requests[2].target)).unwrap();
        assert_eq!(
            url.path(),
            "/calendar/v3/calendars/team%2Fa%2Bb%40example.com/transferOwnership"
        );
        let pairs: Vec<_> = url.query_pairs().collect();
        assert_eq!(
            pairs
                .iter()
                .map(|(key, value)| (key.as_ref(), value.as_ref()))
                .collect::<Vec<_>>(),
            [
                ("fields", "id"),
                ("newDataOwner", "new+owner@example.com"),
                ("useAdminAccess", "true")
            ]
        );
        for request in requests {
            assert!(request.body.is_empty());
            assert!(!request.headers.contains_key("content-type"));
        }
    }

    #[tokio::test]
    async fn stale_calendar_etag_surfaces_conflict_without_retry() {
        let (client, observed) = fixture(vec![(
            412,
            r#"{"error":{"code":412,"message":"Precondition failed"}}"#,
        )])
        .await;
        let error = CalendarClient::new(client)
            .patch(
                "primary",
                CalendarPatch {
                    summary: Patch::Value("Changed".into()),
                    ..CalendarPatch::default()
                },
                CalendarOptions {
                    if_match: Some("\"stale\"".into()),
                    ..CalendarOptions::default()
                },
            )
            .await
            .unwrap_err();
        assert_eq!(
            error.status(),
            Some(reqwest::StatusCode::PRECONDITION_FAILED)
        );
        assert_eq!(observed.await.unwrap().len(), 1);
    }
}
