use serde::{Deserialize, Serialize};

mod client;
pub use client::EventClient;
pub(crate) mod options;
pub use options::*;
mod patch;
pub use patch::*;

pub mod types;
use types::{
    EventAttachment, EventAttachmentWrite, EventAttendeeWrite, EventAttendees,
    EventBirthdayProperties, EventBirthdayPropertiesWrite, EventCalendarDate, EventConferenceData,
    EventConferenceDataWrite, EventCreator, EventExtendedProperties, EventFocusTimeProperties,
    EventGadget, EventOrganizer, EventOrganizerWrite, EventOutOfOfficeProperties, EventReminder,
    EventReminderWrite, EventSource, EventSourceWrite, EventStatus, EventTransparency, EventType,
    EventVisibility, EventWorkingLocation, EventWorkingLocationWrite, SendUpdates,
};

use super::{CalendarAccessRole, DefaultReminder, QueryParams, Sendable};

/* Google API Source: https://developers.google.com/calendar/api/v3/reference/events#resource */

/// Events is a listing of events on a per-page basis.
#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct Events {
    #[serde(default = "default_events_kind")]
    pub kind: String,
    pub etag: String,
    pub summary: String,
    pub description: String,
    pub updated: String,
    pub time_zone: String,
    pub access_role: CalendarAccessRole,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub default_reminders: Vec<DefaultReminder>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_sync_token: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<Event>,
}

/// Writable event properties for insert, import, and full replacement.
///
/// `None` omits a property. `Some(false)` and `Some(Vec::new())` are sent explicitly.
/// Unlike [`Event`], this payload never injects response metadata or default values.
#[derive(Serialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventWrite {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anyone_can_add_self: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub birthday_properties: Option<EventBirthdayPropertiesWrite>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<EventAttachmentWrite>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attendees: Option<Vec<EventAttendeeWrite>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attendees_omitted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conference_data: Option<EventConferenceDataWrite>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<EventCalendarDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time_unspecified: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_label_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_type: Option<EventType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extended_properties: Option<EventExtendedProperties>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focus_time_properties: Option<EventFocusTimeProperties>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guests_can_invite_others: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guests_can_see_other_guests: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guests_can_modify: Option<bool>,
    #[serde(rename = "iCalUID", skip_serializing_if = "Option::is_none")]
    pub ical_uid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organizer: Option<EventOrganizerWrite>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub out_of_office_properties: Option<EventOutOfOfficeProperties>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_copy: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recurrence: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reminders: Option<EventReminderWrite>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<EventSourceWrite>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<EventCalendarDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<EventStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transparency: Option<EventTransparency>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<EventVisibility>,
    #[serde(
        rename = "workingLocationProperties",
        skip_serializing_if = "Option::is_none"
    )]
    pub working_location: Option<EventWorkingLocationWrite>,
}

/// Event is a single event.
#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
#[allow(clippy::struct_excessive_bools)]
pub struct Event {
    pub anyone_can_add_self: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub birthday_properties: Option<EventBirthdayProperties>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<EventAttachment>,
    pub attendees_omitted: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub attendees: Vec<EventAttendees>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_id: Option<String>,
    pub conference_data: EventConferenceData,
    pub created: String,
    pub creator: EventCreator,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub end: EventCalendarDate,
    pub end_time_unspecified: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_label_id: Option<String>,
    pub etag: String,
    pub event_type: EventType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extended_properties: Option<EventExtendedProperties>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focus_time_properties: Option<EventFocusTimeProperties>,
    pub gadget: EventGadget,
    #[serde(default = "default_true", rename = "guestsCanInviteOthers")]
    pub guests_can_invite_others: bool,
    #[serde(default = "default_true")]
    pub guests_can_see_other_guests: bool,
    pub guests_can_modify: bool,
    pub hangout_link: String,
    pub html_link: String,
    #[serde(rename = "iCalUID")]
    pub ical_uid: Option<String>,
    pub id: String,
    #[serde(default = "default_event_kind")]
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    pub locked: bool,
    pub organizer: EventOrganizer,
    pub original_start_time: EventCalendarDate,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub out_of_office_properties: Option<EventOutOfOfficeProperties>,
    pub private_copy: bool,
    pub recurring_event_id: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub recurrence: Vec<String>,
    pub reminders: EventReminder,
    pub sequence: i32,
    pub source: EventSource,
    pub start: EventCalendarDate,
    pub status: EventStatus,
    pub summary: String,
    pub transparency: EventTransparency,
    pub updated: String,
    pub visibility: EventVisibility,
    #[serde(rename = "workingLocationProperties")]
    pub working_location: EventWorkingLocation,

    #[serde(skip)]
    pub calendar_id: String,
    #[serde(skip)]
    query_string: QueryParams,
}

impl Event {
    pub fn add_query(&mut self, key: String, value: String) {
        self.query_string.insert(key, value);
    }
}

impl Sendable for Event {
    fn path(&self, action: Option<String>) -> String {
        let mut path = if self.id.is_empty() {
            collection_path(&self.calendar_id)
        } else {
            item_path(&self.calendar_id, &self.id)
        };
        if let Some(action) = action.filter(|action| !action.is_empty()) {
            path.push('/');
            path.push_str(&crate::request::segment(&action));
        }
        path
    }

    fn query(&self) -> QueryParams {
        self.query_string.clone()
    }
}

impl Events {
    pub fn add_calendar(&mut self, calendar_id: &str) {
        let calendar_id = calendar_id.to_string();
        self.items
            .iter_mut()
            .for_each(|e| e.calendar_id.clone_from(&calendar_id));
    }
}

fn collection_path(calendar_id: &str) -> String {
    format!("calendars/{}/events", crate::request::segment(calendar_id))
}

fn item_path(calendar_id: &str, event_id: &str) -> String {
    format!(
        "{}/{}",
        collection_path(calendar_id),
        crate::request::segment(event_id)
    )
}

fn default_event_kind() -> String {
    "calendar#event".to_string()
}
fn default_events_kind() -> String {
    "calendar#events".to_string()
}
const fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::{types::EventType, Event, EventWrite};
    use crate::Sendable;

    #[test]
    fn writes_omit_unspecified_properties_and_preserve_false_and_empty() {
        assert_eq!(
            serde_json::to_value(EventWrite::default()).unwrap(),
            serde_json::json!({})
        );

        let event = EventWrite {
            guests_can_invite_others: Some(false),
            guests_can_modify: Some(false),
            summary: Some(String::new()),
            attendees: Some(Vec::new()),
            ical_uid: Some("uid@example.com".to_string()),
            ..Default::default()
        };
        assert_eq!(
            serde_json::to_value(event).unwrap(),
            serde_json::json!({
                "guestsCanInviteOthers": false,
                "guestsCanModify": false,
                "summary": "",
                "attendees": [],
                "iCalUID": "uid@example.com"
            })
        );
    }

    #[test]
    fn reads_google_event_wire_names() {
        let event: Event = serde_json::from_value(serde_json::json!({
            "guestsCanInviteOthers": false,
            "iCalUID": "uid@example.com"
        }))
        .unwrap();
        assert!(!event.guests_can_invite_others);
        assert_eq!(event.ical_uid.as_deref(), Some("uid@example.com"));
    }

    #[test]
    fn event_paths_encode_identifiers_without_encoding_separators() {
        let mut event = Event {
            calendar_id: "team/a+b@example.com".to_string(),
            id: "event/id?%".to_string(),
            ..Default::default()
        };
        assert_eq!(
            event.path(None),
            "calendars/team%2Fa%2Bb%40example.com/events/event%2Fid%3F%25"
        );
        assert_eq!(
            event.path(Some("instances".to_string())),
            "calendars/team%2Fa%2Bb%40example.com/events/event%2Fid%3F%25/instances"
        );
        event.id.clear();
        assert_eq!(
            event.path(None),
            "calendars/team%2Fa%2Bb%40example.com/events"
        );
    }

    #[test]
    fn deserializes_from_gmail_events() {
        let event: Event = serde_json::from_str(
            r#"{
                "kind": "calendar#event",
                "etag": "\"etag\"",
                "id": "gmail-event",
                "status": "confirmed",
                "htmlLink": "https://calendar.google.com/event",
                "created": "2026-04-28T10:00:00Z",
                "updated": "2026-04-28T10:00:00Z",
                "summary": "Flight to Copenhagen",
                "creator": {"email": "user@example.com"},
                "organizer": {"email": "user@example.com"},
                "start": {"dateTime": "2026-05-01T10:00:00Z"},
                "end": {"dateTime": "2026-05-01T11:00:00Z"},
                "eventType": "fromGmail"
            }"#,
        )
        .expect("fromGmail event payload should deserialize");

        assert_eq!(event.event_type, EventType::FromGmail);
    }

    #[test]
    fn deserializes_sparse_working_location_payloads() {
        let event: Event = serde_json::from_str(
            r#"{
                "kind": "calendar#event",
                "etag": "\"etag\"",
                "id": "working-location-event",
                "status": "confirmed",
                "htmlLink": "https://calendar.google.com/event",
                "created": "2026-04-28T10:00:00Z",
                "updated": "2026-04-28T10:00:00Z",
                "creator": {"email": "user@example.com"},
                "organizer": {"email": "user@example.com"},
                "start": {"date": "2026-05-01"},
                "end": {"date": "2026-05-02"},
                "eventType": "workingLocation",
                "workingLocationProperties": {}
            }"#,
        )
        .expect("working location payload should deserialize when type is omitted");

        assert_eq!(event.event_type, EventType::WorkingLocation);
    }

    #[test]
    fn incremental_sync_accepts_cancelled_and_future_event_types() {
        let page: super::Events = serde_json::from_value(serde_json::json!({
            "nextSyncToken": "sync-token",
            "items": [
                {"id": "deleted", "status": "cancelled"},
                {
                    "id": "exception", "status": "cancelled",
                    "recurringEventId": "series",
                    "originalStartTime": {"date": "2026-10-24"}
                },
                {
                    "id": "future", "eventType": "futureEventType", "status": "futureStatus",
                    "visibility": "futureVisibility", "transparency": "futureTransparency",
                    "workingLocationProperties": {"type": "futureLocation"}
                }
            ]
        }))
        .unwrap();
        assert_eq!(page.next_sync_token.as_deref(), Some("sync-token"));
        assert_eq!(page.items[0].status, super::types::EventStatus::Cancelled);
        assert_eq!(
            page.items[1].original_start_time.date.as_deref(),
            Some("2026-10-24")
        );
        assert_eq!(
            page.items[2].event_type,
            EventType::Unknown("futureEventType".to_string())
        );
        assert_eq!(
            page.items[2].working_location.typ,
            Some(super::types::EventOfficeLocationType::Unknown(
                "futureLocation".to_string()
            ))
        );
    }

    #[test]
    fn complete_special_event_metadata_is_preserved() {
        let payload = serde_json::json!({
            "id": "special-event",
            "eventType": "birthday",
            "anyoneCanAddSelf": true,
            "eventLabelId": "label-id",
            "extendedProperties": {
                "private": {"private-key": "private-value"},
                "shared": {"shared-key": "shared-value"}
            },
            "focusTimeProperties": {
                "autoDeclineMode": "declineOnlyNewConflictingInvitations",
                "chatStatus": "doNotDisturb", "declineMessage": "Focusing"
            },
            "outOfOfficeProperties": {
                "autoDeclineMode": "declineAllConflictingInvitations", "declineMessage": "Away"
            },
            "birthdayProperties": {
                "type": "custom", "contact": "people/c12345", "customTypeName": "Special day"
            },
            "gadget": {
                "type": "birthday", "display": "icon", "height": 2147483647, "width": 1024,
                "iconLink": "https://example.com/icon", "link": "https://example.com",
                "title": "Birthday", "preferences": {"key": "value"}
            },
            "sequence": 2147483647,
            "recurrence": ["RDATE:20261024", "RRULE:FREQ=YEARLY", "RDATE:20261024"]
        });
        let event: Event = serde_json::from_value(payload.clone()).unwrap();
        let serialized = serde_json::to_value(event).unwrap();
        for (key, value) in payload.as_object().unwrap() {
            assert_eq!(&serialized[key], value, "field {key}");
        }
    }

    #[test]
    fn special_event_writes_only_send_explicit_writable_fields() {
        use super::types::{
            EventAutoDeclineMode, EventBirthdayPropertiesWrite, EventBirthdayType, EventChatStatus,
            EventFocusTimeProperties, EventOutOfOfficeProperties,
        };
        let event = EventWrite {
            anyone_can_add_self: Some(false),
            birthday_properties: Some(EventBirthdayPropertiesWrite {
                typ: Some(EventBirthdayType::Birthday),
            }),
            event_label_id: Some(String::new()),
            focus_time_properties: Some(EventFocusTimeProperties {
                auto_decline_mode: Some(EventAutoDeclineMode::DeclineNone),
                chat_status: Some(EventChatStatus::Available),
                decline_message: None,
            }),
            out_of_office_properties: Some(EventOutOfOfficeProperties::default()),
            private_copy: Some(false),
            sequence: Some(i32::MAX),
            ..Default::default()
        };
        assert_eq!(
            serde_json::to_value(event).unwrap(),
            serde_json::json!({
                "anyoneCanAddSelf": false,
                "birthdayProperties": {"type": "birthday"},
                "eventLabelId": "",
                "focusTimeProperties": {"autoDeclineMode": "declineNone", "chatStatus": "available"},
                "outOfOfficeProperties": {},
                "privateCopy": false,
                "sequence": 2147483647
            })
        );
    }
}
