use serde::{Deserialize, Serialize};

use super::DefaultReminder;
use crate::AdditionalProperties;

#[derive(Serialize, Deserialize, Default, Debug, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub enum SendUpdates {
    #[default]
    All,
    ExternalOnly,
    None,
}
impl SendUpdates {
    #[must_use]
    pub const fn to_str(&self) -> &'static str {
        match self {
            Self::All => "all",
            Self::ExternalOnly => "externalOnly",
            Self::None => "none",
        }
    }
}
impl std::fmt::Display for SendUpdates {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EventOfficeLocationType {
    #[default]
    HomeOffice,
    OfficeLocation,
    CustomLocation,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventOfficeLocation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub building_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub desk_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub floor_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub floor_section_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventCustomLocation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventHomeOffice {}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct EventWorkingLocation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_location: Option<EventCustomLocation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub home_office: Option<EventHomeOffice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub office_location: Option<EventOfficeLocation>,
    #[serde(rename = "type")]
    pub typ: EventOfficeLocationType,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EventVisibility {
    #[default]
    Default,
    Public,
    Private,
    Confidential,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EventTransparency {
    #[default]
    Opaque,
    Transparent,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EventStatus {
    Confirmed,
    #[default]
    Tentative,
    Cancelled,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct EventSource {
    pub title: String,
    pub url: String,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventReminder {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overrides: Option<Vec<DefaultReminder>>,
    pub use_default: bool,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventOrganizer {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "self")]
    pub appears_as_self: Option<bool>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EventGadgetDisplay {
    #[default]
    Icon,
    Chip,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct EventGadget {
    pub display: EventGadgetDisplay,
    pub preferences: AdditionalProperties,
    // a lot of deprecated fields in this struct
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventExtendedProperties {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private: Option<AdditionalProperties>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shared: Option<AdditionalProperties>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EventType {
    #[default]
    Default,
    FromGmail,
    OutOfOffice,
    FocusTime,
    WorkingLocation,
    Birthday,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventCalendarDate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_zone: Option<String>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventConferenceData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conference_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conference_solution: Option<EventConferenceSolution>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_request: Option<EventCreateConferenceRequest>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub entry_points: Vec<EventConferenceEntryPoint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventCreator {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "self")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub appears_as_self: Option<bool>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventConferenceEntryPoint {
    entry_point_type: EventConferenceEntryPointType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    meeting_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    passcode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    uri: Option<String>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EventConferenceEntryPointType {
    #[default]
    Video,
    Phone,
    #[serde(alias = "sip")]
    SIP,
    More,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventCreateConferenceRequest {
    conference_solution_key: EventConferenceSolutionKey,
    request_id: String,
    status: EventConferenceStatus,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventConferenceStatus {
    status_code: EventConferenceStatusCode,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EventConferenceStatusCode {
    #[default]
    Pending,
    Success,
    Failure,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventConferenceSolution {
    pub icon_uri: String,
    pub key: EventConferenceSolutionKey,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventConferenceSolutionKey {
    #[serde(rename = "type")]
    pub typ: EventConferenceSolutionKeyType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EventConferenceSolutionKeyType {
    EventHangout,
    EventNamedHangout,
    #[default]
    HangoutsMeet,
    AddOn,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventAttendees {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_guests: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organizer: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource: Option<bool>,
    pub response_status: EventResponseStatus,
    #[serde(rename = "self")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub appears_as_self: Option<bool>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EventResponseStatus {
    #[default]
    NeedsAction,
    Declined,
    Tentative,
    Accepted,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventAttachment {
    pub file_id: String,
    pub file_url: String,
    pub icon_link: String,
    pub mime_type: String,
    pub title: String,
}

/// Attachment input. Google populates attachment metadata in responses.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventAttachmentWrite {
    pub file_url: String,
}

/// Attendee input without response-only identity and organizer flags.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventAttendeeWrite {
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_guests: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_status: Option<EventResponseStatus>,
}

/// Organizer input for imports. Other writes cannot change the organizer.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventOrganizerWrite {
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

/// Source input with an explicitly supplied URL.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventSourceWrite {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Reminder input requires an explicit choice of calendar defaults.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventReminderWrite {
    pub use_default: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overrides: Option<Vec<DefaultReminder>>,
}

/// Working-location input always includes its discriminator and matching value.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum EventWorkingLocationWrite {
    HomeOffice {
        #[serde(rename = "homeOffice")]
        home_office: EventHomeOffice,
    },
    OfficeLocation {
        #[serde(rename = "officeLocation")]
        office_location: EventOfficeLocation,
    },
    CustomLocation {
        #[serde(rename = "customLocation")]
        custom_location: EventCustomLocation,
    },
}

/// Conference creation input; server-generated conference status is not writable.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventCreateConferenceRequestWrite {
    pub request_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conference_solution_key: Option<EventConferenceSolutionKeyWrite>,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct EventConferenceSolutionKeyWrite {
    #[serde(rename = "type")]
    pub typ: EventConferenceSolutionKeyType,
}

/// Writable conference creation properties, without response metadata.
#[derive(Serialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventConferenceDataWrite {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_request: Option<EventCreateConferenceRequestWrite>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[cfg(test)]
mod write_tests {
    use super::*;

    #[test]
    fn nested_inputs_do_not_inject_response_fields_or_defaults() {
        let attendee = EventAttendeeWrite {
            email: "guest@example.com".to_string(),
            additional_guests: None,
            comment: None,
            display_name: None,
            optional: Some(false),
            resource: None,
            response_status: None,
        };
        assert_eq!(
            serde_json::to_value(attendee).unwrap(),
            serde_json::json!({
                "email": "guest@example.com", "optional": false
            })
        );
        let organizer = EventOrganizerWrite {
            email: "owner@example.com".to_string(),
            display_name: None,
        };
        assert_eq!(
            serde_json::to_value(organizer).unwrap(),
            serde_json::json!({
                "email": "owner@example.com"
            })
        );
        let source = EventSourceWrite {
            url: "https://example.com/event".to_string(),
            title: None,
        };
        assert_eq!(
            serde_json::to_value(source).unwrap(),
            serde_json::json!({
                "url": "https://example.com/event"
            })
        );
        let reminders = EventReminderWrite {
            use_default: false,
            overrides: Some(Vec::new()),
        };
        assert_eq!(
            serde_json::to_value(reminders).unwrap(),
            serde_json::json!({
                "useDefault": false, "overrides": []
            })
        );
        let location = EventWorkingLocationWrite::HomeOffice {
            home_office: EventHomeOffice {},
        };
        assert_eq!(
            serde_json::to_value(location).unwrap(),
            serde_json::json!({
                "type": "homeOffice", "homeOffice": {}
            })
        );
        let conference = EventConferenceDataWrite {
            create_request: Some(EventCreateConferenceRequestWrite {
                request_id: "request-id".to_string(),
                conference_solution_key: Some(EventConferenceSolutionKeyWrite {
                    typ: EventConferenceSolutionKeyType::HangoutsMeet,
                }),
            }),
            notes: None,
        };
        assert_eq!(
            serde_json::to_value(conference).unwrap(),
            serde_json::json!({
                "createRequest": {
                    "requestId": "request-id",
                    "conferenceSolutionKey": {"type": "hangoutsMeet"}
                }
            })
        );
    }
}
