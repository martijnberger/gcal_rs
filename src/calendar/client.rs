use std::sync::Arc;

use super::{
    CalendarAccessRole, CalendarList, CalendarListEntryPatch, CalendarListEntryWrite,
    CalendarListItem,
};
use crate::{
    request::{segment, ApiRequest},
    ClientError, ClientResult, GCalClient, QueryParams,
};

/// `CalendarListClient` is the method of accessing the calendar list. You must provide it with a
/// Google Calendar client.
#[derive(Debug, Clone)]
pub struct CalendarListClient(pub(crate) Arc<GCalClient>);

impl CalendarListClient {
    /// Construct a `CalendarListClient`. Requires a Google Calendar Client.
    #[must_use]
    pub const fn new(client: Arc<GCalClient>) -> Self {
        Self(client)
    }

    /// Fetch one entry in the authenticated user's calendar list.
    ///
    /// # Errors
    /// Returns API, transport, URL, or response decoding errors.
    pub async fn get(
        &self,
        calendar_id: &str,
        options: CalendarListEntryOptions,
    ) -> ClientResult<CalendarListItem> {
        let mut request = ApiRequest::new(calendar_list_entry_path(calendar_id));
        request.query = options.extra;
        Ok(self.0.get(None, request).await?.json().await?)
    }

    /// Add an existing calendar to the user's list. The required calendar ID is
    /// sent in the collection request body, not appended to its endpoint.
    ///
    /// # Errors
    /// Returns API, transport, serialization, URL, or response decoding errors.
    pub async fn insert(
        &self,
        calendar_id: &str,
        entry: CalendarListEntryWrite,
        options: CalendarListEntryOptions,
    ) -> ClientResult<CalendarListItem> {
        #[derive(serde::Serialize)]
        struct InsertEntry<'a> {
            id: &'a str,
            #[serde(flatten)]
            entry: CalendarListEntryWrite,
        }
        let mut request = ApiRequest::with_body(
            "users/me/calendarList".into(),
            InsertEntry {
                id: calendar_id,
                entry,
            },
        );
        request.query = options.query();
        Ok(self.0.post(None, request).await?.json().await?)
    }

    /// Replace writable calendar-list preferences. Enable `color_rgb_format`
    /// when setting foreground/background colors.
    ///
    /// # Errors
    /// Returns API errors (including HTTP 412), transport, serialization, URL,
    /// invalid header, or response decoding errors.
    pub async fn update(
        &self,
        calendar_id: &str,
        entry: CalendarListEntryWrite,
        options: CalendarListEntryOptions,
    ) -> ClientResult<CalendarListItem> {
        let mut request = ApiRequest::with_body(calendar_list_entry_path(calendar_id), entry);
        request.query = options.query();
        request.if_match = options.if_match;
        Ok(self.0.put(None, request).await?.json().await?)
    }

    /// Change only specified per-user preferences. Empty reminder/notification
    /// arrays explicitly remove all entries.
    ///
    /// # Errors
    /// Returns API errors (including HTTP 412), transport, serialization, URL,
    /// invalid header, or response decoding errors.
    pub async fn patch(
        &self,
        calendar_id: &str,
        entry: CalendarListEntryPatch,
        options: CalendarListEntryOptions,
    ) -> ClientResult<CalendarListItem> {
        let mut request = ApiRequest::with_body(calendar_list_entry_path(calendar_id), entry);
        request.query = options.query();
        request.if_match = options.if_match;
        Ok(self.0.patch(None, request).await?.json().await?)
    }

    /// Remove a calendar from the user's list, without deleting the calendar.
    ///
    /// # Errors
    /// Returns API errors (including HTTP 412), transport, URL, or invalid header errors.
    pub async fn delete(
        &self,
        calendar_id: &str,
        options: CalendarListEntryOptions,
    ) -> ClientResult<()> {
        let mut request = ApiRequest::new(calendar_list_entry_path(calendar_id));
        request.query = options.extra;
        request.if_match = options.if_match;
        self.0.delete(None, request).await?;
        Ok(())
    }
    /// List every page of calendars, including empty intermediate pages.
    ///
    /// # Errors
    /// Returns an error if the request fails or the response cannot be parsed.
    pub async fn list(
        &self,
        hidden: bool,
        access_role: CalendarAccessRole,
    ) -> ClientResult<Vec<CalendarListItem>> {
        Ok(self
            .list_all(CalendarListOptions {
                show_hidden: Some(hidden),
                min_access_role: Some(access_role),
                ..CalendarListOptions::default()
            })
            .await?
            .items)
    }

    /// Fetch a single page without consuming or modifying reusable options.
    ///
    /// # Errors
    /// Returns an error if sync filters are invalid, the request fails, or the response
    /// cannot be parsed. HTTP 410 is returned unchanged for application-managed full sync.
    pub async fn list_page(&self, options: &CalendarListOptions) -> ClientResult<CalendarList> {
        let query = options.query();
        CalendarListOptions::validate(&query)?;
        let mut request = CalendarList::default();
        request.set_query(query);
        Ok(self
            .0
            .get(None, request)
            .await?
            .json::<CalendarList>()
            .await?)
    }

    /// Collect all continuation pages, preserving the final synchronization token.
    ///
    /// # Errors
    /// Returns an error if any request fails or a response cannot be parsed.
    pub async fn list_all(&self, mut options: CalendarListOptions) -> ClientResult<CalendarList> {
        let mut result = self.list_page(&options).await?;
        while let Some(token) = result.next_page_token.take() {
            options.page_token = Some(token);
            let mut page = self.list_page(&options).await?;
            result.items.append(&mut page.items);
            result.next_page_token = page.next_page_token;
            result.next_sync_token = page.next_sync_token;
        }
        Ok(result)
    }

    /// Collect every page of changes since `sync_token`, retaining deleted entries
    /// and the final synchronization token.
    ///
    /// The supplied token overrides any token in `options`, including raw query pairs.
    /// This method never modifies application storage or retries an expired token.
    /// If the error's [`ClientError::requires_full_sync`] is true (HTTP 410), discard
    /// the application's stored sync token and perform a new full synchronization.
    ///
    /// # Errors
    /// Returns an error for invalid sync filters, a failed request, or an invalid response.
    pub async fn sync(
        &self,
        sync_token: impl Into<String>,
        mut options: CalendarListOptions,
    ) -> ClientResult<CalendarList> {
        options.sync_token = Some(sync_token.into());
        self.list_all(options).await
    }
}

fn calendar_list_entry_path(calendar_id: &str) -> String {
    format!("users/me/calendarList/{}", segment(calendar_id))
}

/// Options for calendar-list entry operations. Typed color options override
/// matching raw query pairs on insert, update, and patch. `if_match` applies to
/// update, patch, and delete; global/future query parameters use `extra`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CalendarListEntryOptions {
    pub color_rgb_format: Option<bool>,
    pub if_match: Option<String>,
    pub extra: QueryParams,
}

impl CalendarListEntryOptions {
    #[must_use]
    pub fn query(&self) -> QueryParams {
        let mut query = self.extra.clone();
        if let Some(value) = self.color_rgb_format {
            query.insert("colorRgbFormat".into(), value.to_string());
        }
        query
    }
}

/// Parameters for `calendarList.list`. Typed fields override matching raw keys.
///
/// With a synchronization token, `min_access_role` and `show_own_organization_only`
/// are forbidden, and `show_deleted` and `show_hidden` cannot be false. Validation
/// includes raw query pairs after typed overrides. Options never manage stored sync state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CalendarListOptions {
    pub max_results: Option<u32>,
    pub min_access_role: Option<CalendarAccessRole>,
    pub page_token: Option<String>,
    pub show_deleted: Option<bool>,
    pub show_hidden: Option<bool>,
    pub show_own_organization_only: Option<bool>,
    pub sync_token: Option<String>,
    /// Additional query pairs, including repeated keys and global API parameters.
    pub extra: QueryParams,
}

impl CalendarListOptions {
    /// Build query pairs without modifying the options.
    #[must_use]
    pub fn query(&self) -> QueryParams {
        let mut query = self.extra.clone();
        if let Some(value) = self.max_results {
            query.insert("maxResults".into(), value.to_string());
        }
        if let Some(value) = &self.min_access_role {
            query.insert("minAccessRole".into(), value.to_string());
        }
        if let Some(value) = &self.page_token {
            query.insert("pageToken".into(), value.clone());
        }
        if let Some(value) = self.show_deleted {
            query.insert("showDeleted".into(), value.to_string());
        }
        if let Some(value) = self.show_hidden {
            query.insert("showHidden".into(), value.to_string());
        }
        if let Some(value) = self.show_own_organization_only {
            query.insert("showOwnOrganizationOnly".into(), value.to_string());
        }
        if let Some(value) = &self.sync_token {
            query.insert("syncToken".into(), value.clone());
        }
        query
    }

    pub(crate) fn validate(query: &QueryParams) -> ClientResult<()> {
        if query.get("syncToken").is_none() {
            return Ok(());
        }
        for (key, value) in query.iter() {
            if matches!(key.as_str(), "minAccessRole" | "showOwnOrganizationOnly") {
                return Err(ClientError::InvalidRequest(format!(
                    "{key} cannot be combined with syncToken"
                )));
            }
            if matches!(key.as_str(), "showDeleted" | "showHidden") && value == "false" {
                return Err(ClientError::InvalidRequest(format!(
                    "{key} cannot be false when syncToken is set"
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::fixture;
    use crate::{calendar::NotificationSettingsWrite, Patch};

    #[tokio::test]
    async fn entry_lifecycle_uses_collection_insert_and_safe_color_preferences() {
        let (client, observed) = fixture(vec![
            (
                200,
                r#"{"id":"team/a+b@example.com","summary":"Team","accessRole":"reader"}"#,
            ),
            (
                200,
                r##"{"id":"team/a+b@example.com","backgroundColor":"#0088aa"}"##,
            ),
            (200, r#"{"id":"team/a+b@example.com","selected":false}"#),
            (
                200,
                r#"{"id":"team/a+b@example.com","defaultReminders":[]}"#,
            ),
            (204, ""),
        ])
        .await;
        let calendars = CalendarListClient::new(client);
        let id = "team/a+b@example.com";
        let fetched = calendars
            .get(id, CalendarListEntryOptions::default())
            .await
            .unwrap();
        assert_eq!(fetched.summary, "Team");
        assert_eq!(fetched.access_role, Some(CalendarAccessRole::Reader));
        let mut options = CalendarListEntryOptions {
            color_rgb_format: Some(true),
            ..CalendarListEntryOptions::default()
        };
        options
            .extra
            .append("colorRgbFormat".into(), "false".into());
        options
            .extra
            .append("colorRgbFormat".into(), "false".into());
        options.extra.append("future".into(), "one".into());
        options.extra.append("future".into(), "two".into());
        let original = options.clone();
        let inserted = calendars
            .insert(
                id,
                CalendarListEntryWrite {
                    background_color: Some("#0088aa".into()),
                    foreground_color: Some("#ffffff".into()),
                    hidden: Some(false),
                    default_reminders: Some(vec![]),
                    notification_settings: Some(NotificationSettingsWrite::default()),
                    ..CalendarListEntryWrite::default()
                },
                options.clone(),
            )
            .await
            .unwrap();
        assert_eq!(options, original);
        assert_eq!(inserted.background_color.as_deref(), Some("#0088aa"));
        calendars
            .update(
                id,
                CalendarListEntryWrite {
                    selected: Some(false),
                    ..CalendarListEntryWrite::default()
                },
                CalendarListEntryOptions {
                    color_rgb_format: Some(false),
                    if_match: Some("\"entry-one\"".into()),
                    ..CalendarListEntryOptions::default()
                },
            )
            .await
            .unwrap();
        calendars
            .patch(
                id,
                CalendarListEntryPatch {
                    summary_override: Patch::Null,
                    selected: Patch::Value(false),
                    default_reminders: Patch::Value(vec![]),
                    notification_settings: Patch::Value(NotificationSettingsWrite::default()),
                    ..CalendarListEntryPatch::default()
                },
                CalendarListEntryOptions {
                    color_rgb_format: Some(true),
                    if_match: Some("\"entry-two\"".into()),
                    ..CalendarListEntryOptions::default()
                },
            )
            .await
            .unwrap();
        calendars
            .delete(
                id,
                CalendarListEntryOptions {
                    if_match: Some("\"entry-three\"".into()),
                    ..CalendarListEntryOptions::default()
                },
            )
            .await
            .unwrap();
        let requests = observed.await.unwrap();
        for (index, request) in requests.iter().enumerate() {
            assert_eq!(
                request.method,
                ["GET", "POST", "PUT", "PATCH", "DELETE"][index]
            );
            let url = url::Url::parse(&format!("http://localhost{}", request.target)).unwrap();
            assert_eq!(
                url.path(),
                if index == 1 {
                    "/calendar/v3/users/me/calendarList"
                } else {
                    "/calendar/v3/users/me/calendarList/team%2Fa%2Bb%40example.com"
                }
            );
            let pairs: Vec<_> = url.query_pairs().collect();
            assert_eq!(
                pairs
                    .iter()
                    .filter(|pair| pair.0 == "colorRgbFormat")
                    .map(|pair| pair.1.as_ref())
                    .collect::<Vec<_>>(),
                match index {
                    1 | 3 => vec!["true"],
                    2 => vec!["false"],
                    _ => vec![],
                }
            );
            if index == 1 {
                assert_eq!(
                    pairs
                        .iter()
                        .filter(|pair| pair.0 == "future")
                        .map(|pair| pair.1.as_ref())
                        .collect::<Vec<_>>(),
                    ["one", "two"]
                );
            }
        }
        assert!(requests[0].body.is_empty());
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&requests[1].body).unwrap(),
            serde_json::json!({
                "id":id, "backgroundColor":"#0088aa", "foregroundColor":"#ffffff",
                "hidden":false, "defaultReminders":[], "notificationSettings":{"notifications":[]}
            })
        );
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&requests[2].body).unwrap(),
            serde_json::json!({"selected":false})
        );
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&requests[3].body).unwrap(),
            serde_json::json!({
                "summaryOverride":null, "selected":false, "defaultReminders":[],
                "notificationSettings":{"notifications":[]}
            })
        );
        for (index, etag) in [
            (2, "\"entry-one\""),
            (3, "\"entry-two\""),
            (4, "\"entry-three\""),
        ] {
            assert_eq!(requests[index].headers["if-match"], etag);
        }
        for request in &requests[1..4] {
            assert_eq!(request.headers["content-type"], "application/json");
        }
        assert!(requests[4].body.is_empty());
    }

    #[tokio::test]
    async fn stale_list_entry_etag_returns_structured_conflict_without_retry() {
        let (client, observed) = fixture(vec![(
            412,
            r#"{"error":{"code":412,"message":"Precondition failed"}}"#,
        )])
        .await;
        let error = CalendarListClient::new(client)
            .update(
                "primary",
                CalendarListEntryWrite {
                    hidden: Some(false),
                    ..CalendarListEntryWrite::default()
                },
                CalendarListEntryOptions {
                    if_match: Some("\"stale\"".into()),
                    ..CalendarListEntryOptions::default()
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

    #[tokio::test]
    async fn sync_keeps_sparse_deletions_all_pages_and_final_token() {
        let (client, observed) = fixture(vec![
            (
                200,
                r#"{"items":[{"id":"changed"}],"nextPageToken":"middle +/"}"#,
            ),
            (200, r#"{"nextPageToken":"last"}"#),
            (
                200,
                r#"{"items":[{"id":"removed","deleted":true}],"nextSyncToken":"new-sync"}"#,
            ),
        ])
        .await;
        let mut options = CalendarListOptions {
            max_results: Some(2),
            show_deleted: Some(true),
            show_hidden: Some(true),
            sync_token: Some("overridden-typed".into()),
            ..CalendarListOptions::default()
        };
        options
            .extra
            .append("syncToken".into(), "overridden-raw".into());
        options.extra.append("future".into(), "one".into());
        options.extra.append("future".into(), "two".into());
        let original = options.clone();
        let list = CalendarListClient::new(client)
            .sync("delta +/", options.clone())
            .await
            .unwrap();
        assert_eq!(options, original);
        assert_eq!(list.items.len(), 2);
        assert_eq!(list.items[0].id, "changed");
        assert_eq!(list.items[0].access_role, None);
        assert_eq!(list.items[1].id, "removed");
        assert_eq!(list.items[1].deleted, Some(true));
        assert_eq!(list.next_page_token, None);
        assert_eq!(list.next_sync_token.as_deref(), Some("new-sync"));
        for (index, request) in observed.await.unwrap().iter().enumerate() {
            assert_eq!(request.method, "GET");
            let url = url::Url::parse(&format!("http://localhost{}", request.target)).unwrap();
            assert_eq!(url.path(), "/calendar/v3/users/me/calendarList");
            let query: Vec<_> = url.query_pairs().collect();
            for (key, value) in [
                ("syncToken", "delta +/"),
                ("maxResults", "2"),
                ("showDeleted", "true"),
                ("showHidden", "true"),
            ] {
                assert_eq!(
                    query
                        .iter()
                        .filter(|pair| pair.0 == key)
                        .map(|pair| pair.1.as_ref())
                        .collect::<Vec<_>>(),
                    [value]
                );
            }
            assert_eq!(
                query
                    .iter()
                    .filter(|pair| pair.0 == "future")
                    .map(|pair| pair.1.as_ref())
                    .collect::<Vec<_>>(),
                ["one", "two"]
            );
            assert_eq!(
                query
                    .iter()
                    .find(|pair| pair.0 == "pageToken")
                    .map(|pair| pair.1.as_ref()),
                [None, Some("middle +/"), Some("last")][index]
            );
        }
    }

    #[tokio::test]
    async fn sync_expiration_requires_full_sync_without_retry_or_option_mutation() {
        let (client, observed) = fixture(vec![(
            410,
            r#"{"error":{"code":410,"message":"Sync token expired"}}"#,
        )])
        .await;
        let options = CalendarListOptions::default();
        let original = options.clone();
        let error = CalendarListClient::new(client)
            .sync("expired", options.clone())
            .await
            .unwrap_err();
        assert!(error.requires_full_sync());
        assert_eq!(error.status(), Some(reqwest::StatusCode::GONE));
        assert_eq!(options, original);
        let requests = observed.await.unwrap();
        assert_eq!(requests.len(), 1);
        assert!(requests[0].target.contains("syncToken=expired"));
    }

    #[tokio::test]
    async fn invalid_sync_options_are_rejected_before_any_request_and_remain_reusable() {
        let (client, observed) = fixture(vec![(200, r#"{"nextSyncToken":"done"}"#)]).await;
        let client = CalendarListClient::new(client);
        let mut invalid = vec![
            CalendarListOptions {
                sync_token: Some("sync".into()),
                min_access_role: Some(CalendarAccessRole::Reader),
                ..CalendarListOptions::default()
            },
            CalendarListOptions {
                sync_token: Some("sync".into()),
                show_own_organization_only: Some(false),
                ..CalendarListOptions::default()
            },
            CalendarListOptions {
                sync_token: Some("sync".into()),
                show_deleted: Some(false),
                ..CalendarListOptions::default()
            },
            CalendarListOptions {
                sync_token: Some("sync".into()),
                show_hidden: Some(false),
                ..CalendarListOptions::default()
            },
        ];
        for (key, value) in [
            ("minAccessRole", "reader"),
            ("showOwnOrganizationOnly", "false"),
            ("showDeleted", "false"),
            ("showHidden", "false"),
        ] {
            for raw_token in [false, true] {
                let mut options = CalendarListOptions::default();
                if raw_token {
                    options.extra.append("syncToken".into(), "sync".into());
                } else {
                    options.sync_token = Some("sync".into());
                }
                options.extra.append(key.into(), value.into());
                if matches!(key, "showDeleted" | "showHidden") {
                    options.extra.append(key.into(), "true".into());
                }
                invalid.push(options);
            }
        }
        for options in invalid {
            let original = options.clone();
            let error = client.list_page(&options).await.unwrap_err();
            assert!(matches!(error, ClientError::InvalidRequest(_)));
            assert_eq!(options, original);
        }
        let mut valid = CalendarListOptions {
            sync_token: Some("typed-sync".into()),
            show_deleted: Some(true),
            show_hidden: Some(true),
            ..CalendarListOptions::default()
        };
        valid.extra.append("syncToken".into(), "raw-sync".into());
        valid.extra.append("showDeleted".into(), "false".into());
        valid.extra.append("showHidden".into(), "false".into());
        assert_eq!(
            client
                .list_page(&valid)
                .await
                .unwrap()
                .next_sync_token
                .as_deref(),
            Some("done")
        );
        let requests = observed.await.unwrap();
        assert_eq!(requests.len(), 1);
        assert!(requests[0].target.contains("syncToken=typed-sync"));
        assert!(!requests[0].target.contains("false"));
    }

    #[tokio::test]
    async fn all_pages_preserve_filters_repeated_extras_and_final_sync_token() {
        let (client, observed) = fixture(vec![
            (200, r#"{"items":[{"id":"first","accessRole":"writerWithoutPrivateAccess"}],"nextPageToken":"middle +/"}"#),
            (200, r#"{"nextPageToken":"last"}"#),
            (200, r#"{"items":[{"id":"removed","deleted":true}],"nextSyncToken":"final-sync"}"#),
        ]).await;
        let mut options = CalendarListOptions {
            max_results: Some(2),
            min_access_role: Some(CalendarAccessRole::WriterWithoutPrivateAccess),
            show_deleted: Some(true),
            show_hidden: Some(true),
            show_own_organization_only: Some(false),
            ..CalendarListOptions::default()
        };
        options.extra.append("futureFilter".into(), "one".into());
        options.extra.append("futureFilter".into(), "two".into());
        options
            .extra
            .append("minAccessRole".into(), "reader".into());
        options.extra.append("minAccessRole".into(), "owner".into());
        let list = CalendarListClient::new(client)
            .list_all(options)
            .await
            .unwrap();
        assert_eq!(list.items.len(), 2);
        assert_eq!(list.items[1].deleted, Some(true));
        assert_eq!(list.items[1].access_role, None);
        assert_eq!(list.next_sync_token.as_deref(), Some("final-sync"));
        assert_eq!(list.next_page_token, None);
        for (index, request) in observed.await.unwrap().iter().enumerate() {
            assert_eq!(request.method, "GET");
            let url = url::Url::parse(&format!("http://localhost{}", request.target)).unwrap();
            assert_eq!(url.path(), "/calendar/v3/users/me/calendarList");
            let query: Vec<_> = url.query_pairs().collect();
            for (key, value) in [
                ("maxResults", "2"),
                ("minAccessRole", "writerWithoutPrivateAccess"),
                ("showDeleted", "true"),
                ("showHidden", "true"),
                ("showOwnOrganizationOnly", "false"),
            ] {
                assert!(query.iter().any(|pair| pair.0 == key && pair.1 == value));
            }
            assert_eq!(
                query
                    .iter()
                    .filter(|pair| pair.0 == "minAccessRole")
                    .count(),
                1
            );
            assert_eq!(
                query
                    .iter()
                    .filter(|pair| pair.0 == "futureFilter")
                    .map(|pair| pair.1.as_ref())
                    .collect::<Vec<_>>(),
                ["one", "two"]
            );
            let token = query
                .iter()
                .find(|pair| pair.0 == "pageToken")
                .map(|pair| pair.1.as_ref());
            assert_eq!(token, [None, Some("middle +/"), Some("last")][index]);
        }
    }

    #[tokio::test]
    async fn page_options_are_reusable_and_preserve_response_tokens() {
        let (client, observed) = fixture(vec![
            (200, r#"{"nextPageToken":"next"}"#),
            (200, r#"{"nextSyncToken":"done"}"#),
        ])
        .await;
        let client = CalendarListClient::new(client);
        let options = CalendarListOptions {
            page_token: Some("initial".into()),
            sync_token: Some("previous-sync".into()),
            ..CalendarListOptions::default()
        };
        let original = options.clone();
        let first = client.list_page(&options).await.unwrap();
        let second = client.list_page(&options).await.unwrap();
        assert_eq!(first.next_page_token.as_deref(), Some("next"));
        assert_eq!(second.next_sync_token.as_deref(), Some("done"));
        assert_eq!(options, original);
        let requests = observed.await.unwrap();
        assert_eq!(requests[0].target, requests[1].target);
        assert!(requests[0].target.contains("syncToken=previous-sync"));
    }

    #[tokio::test]
    async fn convenience_list_does_not_stop_at_an_empty_page() {
        let (client, observed) = fixture(vec![
            (
                200,
                r#"{"items":[{"id":"first"}],"nextPageToken":"middle"}"#,
            ),
            (200, r#"{"items":[],"nextPageToken":"last"}"#),
            (200, r#"{"items":[{"id":"last"}]}"#),
        ])
        .await;
        let items = CalendarListClient::new(client)
            .list(false, CalendarAccessRole::Reader)
            .await
            .unwrap();
        assert_eq!(
            items
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            ["first", "last"]
        );
        assert_eq!(observed.await.unwrap().len(), 3);
    }
}
