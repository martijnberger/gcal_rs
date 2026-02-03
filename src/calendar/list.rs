use serde::{Deserialize, Serialize};

use super::{
    CalendarAccessRole, ConferenceProperties, DefaultReminder, NotificationSettings, QueryParams,
    Sendable,
};

/* Google Calendar API: https://developers.google.com/calendar/api/v3/reference/calendarList#resource */

/// `CalendarListItem` is a single calendar returned by `CalendarList`, do not confuse this with
/// Calendar.
#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CalendarListItem {
    #[serde(default = "default_entry_kind")]
    pub kind: String,
    pub id: String,
    pub etag: String,
    pub summary: String,
    pub access_role: CalendarAccessRole,
    pub notification_settings: Option<NotificationSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary_override: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_zone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreground_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conference_properties: Option<ConferenceProperties>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub default_reminders: Vec<DefaultReminder>,

    #[serde(skip)]
    query_string: QueryParams,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CalendarList {
    #[serde(default = "default_list_kind")]
    pub kind: String,
    pub etag: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_sync_token: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<CalendarListItem>,

    #[serde(skip)]
    query_string: QueryParams,
}
impl CalendarList {
    pub fn add_query(&mut self, key: String, value: String) {
        self.query_string.insert(key, value);
    }
}

impl Sendable for CalendarListItem {
    fn path(&self, _action: Option<String>) -> String {
        format!("users/me/calendarList/{}", self.id)
    }

    fn query(&self) -> QueryParams {
        self.query_string.clone()
    }
}
impl Sendable for CalendarList {
    fn path(&self, _action: Option<String>) -> String {
        String::from("users/me/calendarList")
    }

    fn query(&self) -> QueryParams {
        self.query_string.clone()
    }
}

fn default_entry_kind() -> String {
    "calendar#calendarListEntry".to_string()
}
fn default_list_kind() -> String {
    "calendar#calendarList".to_string()
}
