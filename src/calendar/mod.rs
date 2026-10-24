//! Calendar metadata, lifecycle operations, and per-user calendar-list preferences.
//!
//! `CalendarListClient::list_page` returns continuation and synchronization
//! tokens unchanged; `list_all` and `list` follow all continuation pages,
//! including empty ones. Use `CalendarListOptions` for typed filters and its
//! `extra` query pairs for global or future API parameters.
//!
//! Sparse and deleted list entries may omit their access role; it is represented
//! by `Option<CalendarAccessRole>`. Unknown roles and nested string-enum values
//! round-trip as `Unknown(String)` rather than failing deserialization.
//!
//! Use `CalendarClient` with `CalendarWrite`/`CalendarPatch` for metadata changes,
//! and `CalendarListClient` with `CalendarListEntryWrite`/`CalendarListEntryPatch`
//! for per-user preferences. These payloads exclude read-only response metadata.
//! Patches distinguish omitted fields from JSON null and explicit empty values.
//! Mutations accept optional ETag preconditions; HTTP 412 is surfaced unchanged.
//! Calendar-list RGB writes require `CalendarListEntryOptions::color_rgb_format`.
//! Ownership transfer uses a bodyless administrator action; clearing a calendar
//! removes events, while deleting a secondary calendar removes the calendar itself.
//!
use serde::{Deserialize, Serialize};

mod client;
pub use client::*;

/// Calendar List, the normal way to get at the list of calendars available.
pub mod list;
pub use list::*;

mod types;
pub use types::*;

mod resource;
pub use resource::*;

mod write;
pub use write::*;

use crate::{QueryParams, Sendable};

/* Google Calendar API: https://developers.google.com/calendar/api/v3/reference/calendars#resource */

/// Calendar is a single calendar.
#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct Calendar {
    #[serde(default = "default_kind")]
    pub kind: String,
    pub id: String,
    pub etag: String,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_zone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_accept_invitations: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_properties: Option<LabelProperties>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conference_properties: Option<types::ConferenceProperties>,
}
impl Sendable for Calendar {
    fn path(&self, _action: Option<String>) -> String {
        format!("calendars/{}", crate::request::segment(&self.id))
    }

    fn query(&self) -> QueryParams {
        QueryParams::default()
    }
}
fn default_kind() -> String {
    "calendar#calendar".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_calendar_fields_and_labels_round_trip() {
        let calendar: Calendar = serde_json::from_str(r##"{
            "id":"team/a+b@example.com","dataOwner":"owner@example.com",
            "autoAcceptInvitations":false,
            "labelProperties":{"eventLabels":[{"id":"label-id","name":"Work","backgroundColor":"#039be5"}]},
            "conferenceProperties":{}
        }"##).unwrap();
        assert_eq!(calendar.path(None), "calendars/team%2Fa%2Bb%40example.com");
        assert_eq!(calendar.data_owner.as_deref(), Some("owner@example.com"));
        assert_eq!(calendar.auto_accept_invitations, Some(false));
        assert!(calendar
            .conference_properties
            .as_ref()
            .unwrap()
            .allowed_solution_types
            .is_empty());
        let value = serde_json::to_value(calendar).unwrap();
        assert_eq!(
            value["labelProperties"]["eventLabels"][0]["backgroundColor"],
            "#039be5"
        );
        assert_eq!(value["autoAcceptInvitations"], false);
        assert!(value["conferenceProperties"]
            .get("defaultReminders")
            .is_none());
    }

    #[test]
    fn labels_distinguish_omission_from_explicit_clear() {
        let omitted = serde_json::to_value(LabelProperties::default()).unwrap();
        let cleared = serde_json::to_value(LabelProperties {
            event_labels: Some(vec![]),
        })
        .unwrap();
        assert!(omitted.get("eventLabels").is_none());
        assert_eq!(cleared["eventLabels"], serde_json::json!([]));
    }
}
