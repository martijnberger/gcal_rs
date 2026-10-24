//! Settings API types for Google Calendar.
//!
//! Settings represent user preferences that can be read via the Calendar API,
//! such as timezone, time format, locale, etc.
//!
//! `SettingsClient::get` reads an individual preference. `list_page` returns
//! one page with its continuation/synchronization tokens; `list_all` follows
//! every continuation, including empty intermediate pages, and retains the final
//! synchronization token. `SettingsListOptions` exposes typed paging parameters
//! and raw, repeatable `extra` query pairs without managing synchronization state.

use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::{ClientResult, GCalClient, QueryParams, Sendable};

/// Represents a single Calendar setting from the Settings API.
///
/// See: <https://developers.google.com/calendar/api/v3/reference/settings>
#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(default)]
pub struct Setting {
    /// Type of the resource ("calendar#setting")
    #[serde(default = "default_setting_kind")]
    pub kind: String,
    /// `ETag` of the resource
    pub etag: String,
    /// The id of the user setting (e.g., "timezone", "format24HourTime", "locale")
    pub id: String,
    /// Value of the user setting. Format depends on the setting ID.
    pub value: String,
}

/// Request type for fetching a single setting by ID.
///
/// Common setting IDs:
/// - "timezone" - IANA timezone (e.g., "Europe/Amsterdam")
/// - "format24HourTime" - "true" or "false"
/// - "locale" - User's locale (e.g., "en", "nl")
/// - "weekStart" - "0" (Sunday), "1" (Monday), or "6" (Saturday)
#[derive(Serialize, Default, Debug)]
pub struct SettingRequest {
    #[serde(skip)]
    setting_id: String,
    #[serde(skip)]
    query_string: QueryParams,
}

impl SettingRequest {
    /// Create a new request for a specific setting.
    ///
    /// # Example
    /// ```ignore
    /// let req = SettingRequest::new("timezone");
    /// ```
    #[must_use]
    pub fn new(setting_id: impl Into<String>) -> Self {
        Self {
            setting_id: setting_id.into(),
            query_string: QueryParams::new(),
        }
    }

    /// Replace all query pairs, retaining repeated keys.
    pub fn set_query(&mut self, query: QueryParams) {
        self.query_string = query;
    }

    /// Set a single query parameter.
    pub fn add_query(&mut self, key: String, value: String) {
        self.query_string.insert(key, value);
    }
}

impl Sendable for SettingRequest {
    fn path(&self, _action: Option<String>) -> String {
        format!(
            "users/me/settings/{}",
            crate::request::segment(&self.setting_id)
        )
    }

    fn query(&self) -> QueryParams {
        self.query_string.clone()
    }
}

/// Response type for listing all settings.
#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct SettingsList {
    /// Type of the resource ("calendar#settings")
    #[serde(default = "default_list_kind")]
    pub kind: String,
    /// `ETag` of the collection
    pub etag: String,
    /// List of user settings
    pub items: Vec<Setting>,
    /// Token for pagination (if more results exist)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
    /// Token for the next incremental read, returned on the final page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_sync_token: Option<String>,
}

/// Request type for listing all settings.
#[derive(Serialize, Default, Debug)]
pub struct SettingsListRequest {
    #[serde(skip)]
    query_string: QueryParams,
}

impl SettingsListRequest {
    #[must_use]
    pub fn new() -> Self {
        Self {
            query_string: QueryParams::new(),
        }
    }

    /// Replace all query pairs, retaining repeated keys.
    pub fn set_query(&mut self, query: QueryParams) {
        self.query_string = query;
    }

    /// Set a single query parameter.
    pub fn add_query(&mut self, key: String, value: String) {
        self.query_string.insert(key, value);
    }
}

impl Sendable for SettingsListRequest {
    fn path(&self, _action: Option<String>) -> String {
        "users/me/settings".to_string()
    }

    fn query(&self) -> QueryParams {
        self.query_string.clone()
    }
}

fn default_setting_kind() -> String {
    "calendar#setting".into()
}

fn default_list_kind() -> String {
    "calendar#settings".into()
}

/// Parameters for `settings.list`. Typed fields override matching raw keys.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SettingsListOptions {
    pub max_results: Option<u32>,
    pub page_token: Option<String>,
    pub sync_token: Option<String>,
    /// Additional query pairs, including repeated keys and global API parameters.
    pub extra: QueryParams,
}

impl SettingsListOptions {
    /// Build query pairs without modifying the options.
    #[must_use]
    pub fn query(&self) -> QueryParams {
        let mut query = self.extra.clone();
        if let Some(value) = self.max_results {
            query.insert("maxResults".into(), value.to_string());
        }
        if let Some(value) = &self.page_token {
            query.insert("pageToken".into(), value.clone());
        }
        if let Some(value) = &self.sync_token {
            query.insert("syncToken".into(), value.clone());
        }
        query
    }
}

/// Read access to the authenticated user's Calendar settings.
#[derive(Debug, Clone)]
pub struct SettingsClient(pub(crate) Arc<GCalClient>);

impl SettingsClient {
    #[must_use]
    pub const fn new(client: Arc<GCalClient>) -> Self {
        Self(client)
    }

    /// Fetch one setting by its identifier.
    ///
    /// # Errors
    /// Returns an error if the request fails or the response cannot be parsed.
    pub async fn get(&self, setting: impl AsRef<str>) -> ClientResult<Setting> {
        Ok(self
            .0
            .get(None, SettingRequest::new(setting.as_ref()))
            .await?
            .json::<Setting>()
            .await?)
    }

    /// Fetch one page without consuming or modifying reusable options.
    ///
    /// # Errors
    /// Returns an error if the request fails or the response cannot be parsed.
    pub async fn list_page(&self, options: &SettingsListOptions) -> ClientResult<SettingsList> {
        let mut request = SettingsListRequest::new();
        request.set_query(options.query());
        Ok(self
            .0
            .get(None, request)
            .await?
            .json::<SettingsList>()
            .await?)
    }

    /// Collect every page and preserve the final synchronization token.
    ///
    /// # Errors
    /// Returns an error if any request fails or a response cannot be parsed.
    pub async fn list_all(&self, mut options: SettingsListOptions) -> ClientResult<SettingsList> {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::fixture;

    #[tokio::test]
    async fn all_settings_pages_keep_query_parameters_and_final_token() {
        let (client, observed) = fixture(vec![
            (200, r#"{"items":[{"id":"timezone","value":"Europe/Amsterdam"}],"nextPageToken":"middle +/"}"#),
            (200, r#"{"nextPageToken":"last"}"#),
            (200, r#"{"items":[{"id":"locale","value":"nl"}],"nextSyncToken":"final-sync"}"#),
        ]).await;
        let mut options = SettingsListOptions {
            max_results: Some(2),
            sync_token: Some("previous-sync".into()),
            ..SettingsListOptions::default()
        };
        options.extra.append("futureFilter".into(), "one".into());
        options.extra.append("futureFilter".into(), "two".into());
        let result = SettingsClient::new(client).list_all(options).await.unwrap();
        assert_eq!(
            result
                .items
                .iter()
                .map(|setting| setting.id.as_str())
                .collect::<Vec<_>>(),
            ["timezone", "locale"]
        );
        assert_eq!(result.next_page_token, None);
        assert_eq!(result.next_sync_token.as_deref(), Some("final-sync"));
        for (index, request) in observed.await.unwrap().iter().enumerate() {
            assert_eq!(request.method, "GET");
            let url = url::Url::parse(&format!("http://localhost{}", request.target)).unwrap();
            assert_eq!(url.path(), "/calendar/v3/users/me/settings");
            let query: Vec<_> = url.query_pairs().collect();
            assert!(query
                .iter()
                .any(|pair| pair.0 == "maxResults" && pair.1 == "2"));
            assert!(query
                .iter()
                .any(|pair| pair.0 == "syncToken" && pair.1 == "previous-sync"));
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
    async fn settings_page_options_are_reusable_and_get_encodes_one_segment() {
        let (client, observed) = fixture(vec![
            (200, r#"{"nextPageToken":"next"}"#),
            (200, r#"{"nextSyncToken":"done"}"#),
            (200, r#"{"id":"custom/id","value":"value"}"#),
        ])
        .await;
        let client = SettingsClient::new(client);
        let options = SettingsListOptions {
            page_token: Some("initial".into()),
            ..SettingsListOptions::default()
        };
        let original = options.clone();
        assert_eq!(
            client
                .list_page(&options)
                .await
                .unwrap()
                .next_page_token
                .as_deref(),
            Some("next")
        );
        assert_eq!(
            client
                .list_page(&options)
                .await
                .unwrap()
                .next_sync_token
                .as_deref(),
            Some("done")
        );
        assert_eq!(options, original);
        assert_eq!(client.get("custom/id+@").await.unwrap().value, "value");
        let requests = observed.await.unwrap();
        assert_eq!(requests[0].target, requests[1].target);
        assert_eq!(
            requests[2].target,
            "/calendar/v3/users/me/settings/custom%2Fid%2B%40"
        );
    }

    #[test]
    fn setting_requests_expose_raw_queries_with_repeated_keys() {
        let mut query = QueryParams::new();
        query.append("future".into(), "one".into());
        query.append("future".into(), "two".into());
        let mut setting = SettingRequest::new("timezone");
        setting.set_query(query.clone());
        assert_eq!(setting.query(), query);
        setting.add_query("fields".into(), "value".into());
        assert_eq!(
            setting.query().get("fields").map(String::as_str),
            Some("value")
        );
        let mut list = SettingsListRequest::new();
        list.set_query(query.clone());
        assert_eq!(list.query(), query);
        list.add_query("fields".into(), "items".into());
        assert_eq!(
            list.query().get("fields").map(String::as_str),
            Some("items")
        );
        let response: SettingsList = serde_json::from_str(r#"{"nextSyncToken":"sync"}"#).unwrap();
        assert!(response.items.is_empty());
        assert_eq!(response.next_sync_token.as_deref(), Some("sync"));
    }
}
