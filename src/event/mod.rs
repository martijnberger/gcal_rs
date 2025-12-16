use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

mod client;
pub use client::EventClient;

pub mod types;
use types::*;

use super::*;

/* Google API Source: https://developers.google.com/calendar/api/v3/reference/events#resource */

/// Events is a listing of events on a per-page basis.
#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct Events {
    #[serde(
        default = "default_events_kind",
        skip_serializing_if = "Option::is_none"
    )]
    pub kind: Option<String>,
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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<Event>,
}

/// Event is a single event.
#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct Event {
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
    pub etag: String,
    pub event_type: EventType,
    pub gadget: EventGadget,
    #[serde(default = "default_true")]
    pub guests_invite_others: bool,
    #[serde(default = "default_true")]
    pub guests_can_see_other_guests: bool,
    pub guests_can_modify: bool,
    pub hangout_link: String,
    pub html_link: String,
    pub ical_uid: Option<String>,
    pub id: String,
    #[serde(default = "default_event_kind")]
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    pub locked: bool,
    pub organizer: EventOrganizer,
    pub original_start_time: EventCalendarDate,
    pub private_copy: bool,
    pub recurring_event_id: String,
    #[serde(skip_serializing_if = "BTreeSet::is_empty")]
    pub recurrence: BTreeSet<String>,
    pub reminders: EventReminder,
    pub sequence: u64,
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

impl Sendable for Event {
    fn path(&self, action: Option<String>) -> String {
        progenitor_support::encode_path(&format!(
            "calendars/{}/events/{}{}",
            self.calendar_id,
            self.id,
            action.map_or_else(String::new, |x| format!("/{}", x))
        ))
    }

    fn query(&self) -> QueryParams {
        self.query_string.clone()
    }
}

impl Events {
    pub fn add_calendar(&mut self, calendar_id: String) {
        self.items.iter_mut().for_each(|e| {
            e.calendar_id = calendar_id.clone();
        });
    }
}

impl Event {
    pub fn add_query(&mut self, key: String, value: String) {
        self.query_string.insert(key, value);
    }
}

fn default_event_kind() -> String {
    "calendar#event".to_string()
}
fn default_events_kind() -> Option<String> {
    Some("calendar#events".to_string())
}
fn default_true() -> bool {
    true
}

/// Taken from [google_calendar](https://github.com/oxidecomputer/third-party-api-clients/blob/720c61bf140726145503cdec3a4240c2843a6080/google/calendar/src/lib.rs#L184)
mod progenitor_support {
    use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};

    const PATH_SET: &AsciiSet = &CONTROLS
        .add(b' ')
        .add(b'"')
        .add(b'#')
        .add(b'<')
        .add(b'>')
        .add(b'?')
        .add(b'`')
        .add(b'{')
        .add(b'}');

    #[allow(dead_code)]
    pub(crate) fn encode_path(pc: &str) -> String {
        utf8_percent_encode(pc, PATH_SET).to_string()
    }
}
