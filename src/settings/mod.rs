//! Settings API types for Google Calendar.
//!
//! Settings represent user preferences that can be read via the Calendar API,
//! such as timezone, time format, locale, etc.

use serde::{Deserialize, Serialize};

use super::{QueryParams, Sendable};

/// Represents a single Calendar setting from the Settings API.
///
/// See: https://developers.google.com/calendar/api/v3/reference/settings
#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(default)]
pub struct Setting {
    /// Type of the resource ("calendar#setting")
    pub kind: String,
    /// ETag of the resource
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
    pub fn new(setting_id: impl Into<String>) -> Self {
        Self {
            setting_id: setting_id.into(),
            query_string: QueryParams::new(),
        }
    }
}

impl Sendable for SettingRequest {
    fn path(&self, _action: Option<String>) -> String {
        format!("users/me/settings/{}", self.setting_id)
    }

    fn query(&self) -> QueryParams {
        self.query_string.clone()
    }
}

/// Response type for listing all settings.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct SettingsList {
    /// Type of the resource ("calendar#settings")
    pub kind: String,
    /// ETag of the collection
    pub etag: String,
    /// List of user settings
    pub items: Vec<Setting>,
    /// Token for pagination (if more results exist)
    pub next_page_token: Option<String>,
}

/// Request type for listing all settings.
#[derive(Serialize, Default, Debug)]
pub struct SettingsListRequest {
    #[serde(skip)]
    query_string: QueryParams,
}

impl SettingsListRequest {
    pub fn new() -> Self {
        Self {
            query_string: QueryParams::new(),
        }
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
