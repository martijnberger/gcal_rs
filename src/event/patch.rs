use serde::Serialize;

use super::types::{
    EventAttachmentWrite, EventAttendeeWrite, EventCalendarDate, EventConferenceDataWrite,
    EventExtendedProperties, EventFocusTimeProperties, EventOutOfOfficeProperties,
    EventReminderWrite, EventSourceWrite, EventStatus, EventTransparency, EventVisibility,
    EventWorkingLocationWrite,
};
use crate::Patch;

/// Partial event changes: `Unset` omits, `Null` clears, and `Value` sets a property.
///
/// Arrays are replaced in their entirety, including an explicitly empty array.
/// Creation-only identifiers, event type, birthday properties, and import-only organizer
/// are intentionally absent. Use [`super::EventWrite`] for insertion and import.
#[derive(Serialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventPatch {
    #[serde(default, skip_serializing_if = "Patch::is_unset")]
    pub anyone_can_add_self: Patch<bool>,
    #[serde(default, skip_serializing_if = "Patch::is_unset")]
    pub attachments: Patch<Vec<EventAttachmentWrite>>,
    #[serde(default, skip_serializing_if = "Patch::is_unset")]
    pub attendees: Patch<Vec<EventAttendeeWrite>>,
    #[serde(default, skip_serializing_if = "Patch::is_unset")]
    pub attendees_omitted: Patch<bool>,
    #[serde(default, skip_serializing_if = "Patch::is_unset")]
    pub color_id: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_unset")]
    pub conference_data: Patch<EventConferenceDataWrite>,
    #[serde(default, skip_serializing_if = "Patch::is_unset")]
    pub description: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_unset")]
    pub end: Patch<EventCalendarDate>,
    #[serde(default, skip_serializing_if = "Patch::is_unset")]
    pub end_time_unspecified: Patch<bool>,
    #[serde(default, skip_serializing_if = "Patch::is_unset")]
    pub event_label_id: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_unset")]
    pub extended_properties: Patch<EventExtendedProperties>,
    #[serde(default, skip_serializing_if = "Patch::is_unset")]
    pub focus_time_properties: Patch<EventFocusTimeProperties>,
    #[serde(default, skip_serializing_if = "Patch::is_unset")]
    pub guests_can_invite_others: Patch<bool>,
    #[serde(default, skip_serializing_if = "Patch::is_unset")]
    pub guests_can_see_other_guests: Patch<bool>,
    #[serde(default, skip_serializing_if = "Patch::is_unset")]
    pub guests_can_modify: Patch<bool>,
    #[serde(default, skip_serializing_if = "Patch::is_unset")]
    pub location: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_unset")]
    pub out_of_office_properties: Patch<EventOutOfOfficeProperties>,
    #[serde(default, skip_serializing_if = "Patch::is_unset")]
    pub recurrence: Patch<Vec<String>>,
    #[serde(default, skip_serializing_if = "Patch::is_unset")]
    pub reminders: Patch<EventReminderWrite>,
    #[serde(default, skip_serializing_if = "Patch::is_unset")]
    pub sequence: Patch<i32>,
    #[serde(default, skip_serializing_if = "Patch::is_unset")]
    pub source: Patch<EventSourceWrite>,
    #[serde(default, skip_serializing_if = "Patch::is_unset")]
    pub start: Patch<EventCalendarDate>,
    #[serde(default, skip_serializing_if = "Patch::is_unset")]
    pub status: Patch<EventStatus>,
    #[serde(default, skip_serializing_if = "Patch::is_unset")]
    pub summary: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_unset")]
    pub transparency: Patch<EventTransparency>,
    #[serde(default, skip_serializing_if = "Patch::is_unset")]
    pub visibility: Patch<EventVisibility>,
    #[serde(
        default,
        rename = "workingLocationProperties",
        skip_serializing_if = "Patch::is_unset"
    )]
    pub working_location: Patch<EventWorkingLocationWrite>,
}
