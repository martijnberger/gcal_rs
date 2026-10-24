use serde::{Deserialize, Serialize};

use super::{
    CalendarAccessRole, ConferenceProperties, DefaultReminder, NotificationSettings, QueryParams,
    Sendable,
};

/* Google Calendar API: https://developers.google.com/calendar/api/v3/reference/calendarList#resource */

/// `CalendarListItem` is a single calendar returned by `CalendarList`, do not confuse this with
/// Calendar.
#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct CalendarListItem {
    #[serde(default = "default_entry_kind")]
    pub kind: String,
    pub id: String,
    pub etag: String,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_role: Option<CalendarAccessRole>,
    #[serde(skip_serializing_if = "Option::is_none")]
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
    pub data_owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_accept_invitations: Option<bool>,
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
#[serde(default, rename_all = "camelCase")]
pub struct CalendarList {
    #[serde(default = "default_list_kind")]
    pub kind: String,
    pub etag: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_sync_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<CalendarListItem>,

    #[serde(skip)]
    query_string: QueryParams,
}
impl CalendarList {
    /// Replace all query pairs, retaining repeated keys.
    pub fn set_query(&mut self, query: QueryParams) {
        self.query_string = query;
    }

    pub fn add_query(&mut self, key: String, value: String) {
        self.query_string.insert(key, value);
    }
}

impl CalendarListItem {
    /// Replace all query pairs, retaining repeated keys.
    pub fn set_query(&mut self, query: QueryParams) {
        self.query_string = query;
    }

    /// Set a query parameter for direct requests using this resource.
    pub fn add_query(&mut self, key: String, value: String) {
        self.query_string.insert(key, value);
    }
}

impl Sendable for CalendarListItem {
    fn path(&self, _action: Option<String>) -> String {
        format!(
            "users/me/calendarList/{}",
            crate::request::segment(&self.id)
        )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sparse_deleted_entries_do_not_invent_an_access_role() {
        let list: CalendarList = serde_json::from_str(
            r#"{"items":[{"id":"removed","deleted":true},{"id":"shared","accessRole":"writerWithoutPrivateAccess"},{"id":"future","accessRole":"futureRole"}],"nextPageToken":"page","nextSyncToken":"sync"}"#
        ).unwrap();
        assert_eq!(list.items[0].access_role, None);
        assert_eq!(list.items[0].deleted, Some(true));
        assert!(list.items[0].default_reminders.is_empty());
        assert_eq!(
            list.items[1].access_role,
            Some(CalendarAccessRole::WriterWithoutPrivateAccess)
        );
        assert_eq!(
            list.items[2].access_role,
            Some(CalendarAccessRole::Unknown("futureRole".into()))
        );
        assert_eq!(
            list.items[2].access_role.as_ref().unwrap().to_string(),
            "futureRole"
        );
        let json = serde_json::to_value(&list).unwrap();
        assert!(json["items"][0].get("accessRole").is_none());
        assert_eq!(json["items"][1]["accessRole"], "writerWithoutPrivateAccess");
        assert_eq!(json["items"][2]["accessRole"], "futureRole");
        assert_eq!(json["nextPageToken"], "page");
        assert_eq!(json["nextSyncToken"], "sync");
    }

    #[test]
    fn current_calendar_list_fields_and_nested_future_enums_round_trip() {
        let item: CalendarListItem = serde_json::from_str(r##"{
            "id":"resource","dataOwner":"owner@example.com","autoAcceptInvitations":false,
            "notificationSettings":{"notifications":[{"method":"futureDelivery","type":"futureNotification"}]},
            "conferenceProperties":{"allowedConferenceSolutionTypes":["futureMeeting"]},
            "defaultReminders":[{"method":"futureReminder","minutes":40320}]
        }"##).unwrap();
        assert_eq!(item.data_owner.as_deref(), Some("owner@example.com"));
        assert_eq!(item.auto_accept_invitations, Some(false));
        let json = serde_json::to_value(item).unwrap();
        assert_eq!(
            json["notificationSettings"]["notifications"][0]["method"],
            "futureDelivery"
        );
        assert_eq!(
            json["notificationSettings"]["notifications"][0]["type"],
            "futureNotification"
        );
        assert_eq!(
            json["conferenceProperties"]["allowedConferenceSolutionTypes"][0],
            "futureMeeting"
        );
        assert_eq!(json["defaultReminders"][0]["method"], "futureReminder");
        assert_eq!(json["defaultReminders"][0]["minutes"], 40320);
    }

    #[test]
    fn calendar_list_paths_encode_identifiers_and_queries_keep_repeated_keys() {
        let mut item = CalendarListItem {
            id: "team/a+b@example.com".into(),
            ..CalendarListItem::default()
        };
        assert_eq!(
            item.path(None),
            "users/me/calendarList/team%2Fa%2Bb%40example.com"
        );
        let mut query = QueryParams::new();
        query.append("future".into(), "one".into());
        query.append("future".into(), "two".into());
        item.set_query(query.clone());
        assert_eq!(item.query(), query);
        let mut list = CalendarList::default();
        list.set_query(query.clone());
        assert_eq!(list.query(), query);
    }
}
