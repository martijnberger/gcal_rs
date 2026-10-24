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

// Response string values are extensible: retain unfamiliar values without failing a page.
macro_rules! open_enum {
    ($name:ident, $default:ident, {$($variant:ident => $wire:literal),+ $(,)?}) => {
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub enum $name {
            $($variant,)+
            Unknown(String),
        }
        impl Default for $name {
            fn default() -> Self { Self::$default }
        }
        impl $name {
            #[must_use]
            pub fn as_str(&self) -> &str {
                match self {
                    $(Self::$variant => $wire,)+
                    Self::Unknown(value) => value,
                }
            }
        }
        impl Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(self.as_str())
            }
        }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                struct StringVisitor;
                impl<'de> serde::de::Visitor<'de> for StringVisitor {
                    type Value = $name;

                    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        f.write_str("a string")
                    }

                    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<$name, E> {
                        Ok(match value {
                            $($wire => $name::$variant,)+
                            _ => $name::Unknown(value.to_owned()),
                        })
                    }

                    fn visit_string<E: serde::de::Error>(self, value: String) -> Result<$name, E> {
                        Ok(match value.as_str() {
                            $($wire => $name::$variant,)+
                            _ => $name::Unknown(value),
                        })
                    }
                }
                deserializer.deserialize_str(StringVisitor)
            }
        }
    };
}

open_enum!(EventOfficeLocationType, HomeOffice, {
    HomeOffice => "homeOffice", OfficeLocation => "officeLocation", CustomLocation => "customLocation"
});
open_enum!(EventVisibility, Default, {
    Default => "default", Public => "public", Private => "private", Confidential => "confidential"
});
open_enum!(EventTransparency, Opaque, { Opaque => "opaque", Transparent => "transparent" });
open_enum!(EventStatus, Confirmed, {
    Confirmed => "confirmed", Tentative => "tentative", Cancelled => "cancelled"
});
open_enum!(EventGadgetDisplay, Icon, { Icon => "icon", Chip => "chip" });
open_enum!(EventType, Default, {
    Default => "default", FromGmail => "fromGmail", OutOfOffice => "outOfOffice",
    FocusTime => "focusTime", WorkingLocation => "workingLocation", Birthday => "birthday"
});
open_enum!(EventConferenceEntryPointType, Video, {
    Video => "video", Phone => "phone", SIP => "sip", More => "more"
});
open_enum!(EventConferenceStatusCode, Pending, {
    Pending => "pending", Success => "success", Failure => "failure"
});
open_enum!(EventConferenceSolutionKeyType, HangoutsMeet, {
    EventHangout => "eventHangout", EventNamedHangout => "eventNamedHangout",
    HangoutsMeet => "hangoutsMeet", AddOn => "addOn"
});
open_enum!(EventResponseStatus, NeedsAction, {
    NeedsAction => "needsAction", Declined => "declined", Tentative => "tentative", Accepted => "accepted"
});
open_enum!(EventAutoDeclineMode, DeclineNone, {
    DeclineNone => "declineNone",
    DeclineAllConflictingInvitations => "declineAllConflictingInvitations",
    DeclineOnlyNewConflictingInvitations => "declineOnlyNewConflictingInvitations"
});
open_enum!(EventChatStatus, Available, { Available => "available", DoNotDisturb => "doNotDisturb" });
open_enum!(EventBirthdayType, Birthday, {
    Anniversary => "anniversary", Birthday => "birthday", Custom => "custom", Other => "other", SelfBirthday => "self"
});

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

// `homeOffice` has discovery type `any`, including explicit JSON null.
fn deserialize_present_value<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<serde_json::Value>, D::Error> {
    serde_json::Value::deserialize(deserializer).map(Some)
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct EventWorkingLocation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_location: Option<EventCustomLocation>,
    #[serde(
        deserialize_with = "deserialize_present_value",
        skip_serializing_if = "Option::is_none"
    )]
    pub home_office: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub office_location: Option<EventOfficeLocation>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub typ: Option<EventOfficeLocationType>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventReminder {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overrides: Option<Vec<DefaultReminder>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_default: Option<bool>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventOrganizer {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "self", skip_serializing_if = "Option::is_none")]
    pub appears_as_self: Option<bool>,
}

/// Deprecated gadget metadata, still returned for birthday calendars.
#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventGadget {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<EventGadgetDisplay>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferences: Option<AdditionalProperties>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_link: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub typ: Option<String>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventExtendedProperties {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private: Option<AdditionalProperties>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shared: Option<AdditionalProperties>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventFocusTimeProperties {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_decline_mode: Option<EventAutoDeclineMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat_status: Option<EventChatStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decline_message: Option<String>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventOutOfOfficeProperties {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_decline_mode: Option<EventAutoDeclineMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decline_message: Option<String>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventBirthdayProperties {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_type_name: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub typ: Option<EventBirthdayType>,
}

/// Birthday input excludes contact-derived, read-only metadata.
#[derive(Serialize, Default, Debug, Clone, PartialEq, Eq)]
pub struct EventBirthdayPropertiesWrite {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub typ: Option<EventBirthdayType>,
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
#[serde(default, rename_all = "camelCase")]
pub struct EventConferenceData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conference_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conference_solution: Option<EventConferenceSolution>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_request: Option<EventCreateConferenceRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_points: Option<Vec<EventConferenceEntryPoint>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<EventConferenceParameters>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventConferenceParameters {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub add_on_parameters: Option<EventConferenceAddOnParameters>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
pub struct EventConferenceAddOnParameters {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<AdditionalProperties>,
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
    #[serde(rename = "self", skip_serializing_if = "Option::is_none")]
    pub appears_as_self: Option<bool>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventConferenceEntryPoint {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_point_type: Option<EventConferenceEntryPointType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_point_features: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meeting_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passcode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region_code: Option<String>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventCreateConferenceRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conference_solution_key: Option<EventConferenceSolutionKey>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<EventConferenceStatus>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventConferenceStatus {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<EventConferenceStatusCode>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventConferenceSolution {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<EventConferenceSolutionKey>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
pub struct EventConferenceSolutionKey {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub typ: Option<EventConferenceSolutionKeyType>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventAttendees {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_guests: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub async_operation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organizer: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_status: Option<EventResponseStatus>,
    #[serde(rename = "self", skip_serializing_if = "Option::is_none")]
    pub appears_as_self: Option<bool>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventAttachment {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_link: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Attachment input excludes the server-generated file ID.
#[derive(Serialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventAttachmentWrite {
    pub file_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// Writable for custom third-party attachments only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_link: Option<String>,
}

/// Attendee input without response-only identity and organizer flags.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventAttendeeWrite {
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_guests: Option<i32>,
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
        home_office: serde_json::Value,
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

/// Entry-point input excludes the backend-populated phone region code.
#[derive(Serialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventConferenceEntryPointWrite {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_point_type: Option<EventConferenceEntryPointType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_point_features: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meeting_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passcode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
}

/// Writable conference properties, excluding the server-generated signature and status.
#[derive(Serialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventConferenceDataWrite {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_request: Option<EventCreateConferenceRequestWrite>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conference_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conference_solution: Option<EventConferenceSolution>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_points: Option<Vec<EventConferenceEntryPointWrite>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<EventConferenceParameters>,
}

#[cfg(test)]
mod tests {
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
            home_office: serde_json::json!({}),
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
            ..Default::default()
        };
        assert_eq!(
            serde_json::to_value(conference).unwrap(),
            serde_json::json!({
                "createRequest": {
                    "requestId": "request-id", "conferenceSolutionKey": {"type": "hangoutsMeet"}
                }
            })
        );
        assert_eq!(
            serde_json::to_value(EventConferenceDataWrite::default()).unwrap(),
            serde_json::json!({})
        );
        assert_eq!(
            serde_json::to_value(EventBirthdayPropertiesWrite::default()).unwrap(),
            serde_json::json!({})
        );
        assert_eq!(
            serde_json::to_value(EventConferenceEntryPointWrite::default()).unwrap(),
            serde_json::json!({})
        );
    }

    #[test]
    fn sparse_nested_responses_round_trip_without_inventing_fields() {
        macro_rules! round_trip {
            ($typ:ty, $value:tt) => {{
                let payload = serde_json::json!($value);
                let parsed: $typ = serde_json::from_value(payload.clone()).unwrap();
                assert_eq!(serde_json::to_value(parsed).unwrap(), payload);
            }};
        }
        round_trip!(EventOrganizer, {"id": "profile-id", "self": false});
        round_trip!(EventAttachment, {"fileUrl": "https://example.com/file"});
        round_trip!(EventAttendees, {"additionalGuests": 2147483647, "asyncOperation": "inProgress"});
        round_trip!(EventSource, {});
        round_trip!(EventReminder, {});
        round_trip!(EventConferenceData, {});
        round_trip!(EventConferenceData, {"createRequest": {"status": {"statusCode": "failure"}}});
        round_trip!(EventConferenceData, {"createRequest": {"requestId": "pending-request"}});
        round_trip!(EventConferenceData, {"conferenceSolution": {"key": {}}, "entryPoints": []});
        round_trip!(EventWorkingLocation, {});
        round_trip!(EventWorkingLocation, {"type": "officeLocation", "officeLocation": {}});
        round_trip!(EventWorkingLocation, {"type": "homeOffice", "homeOffice": null});
        round_trip!(EventWorkingLocation, {"type": "homeOffice", "homeOffice": [true, "any", 7]});
    }

    #[test]
    fn conference_fields_and_future_types_are_preserved() {
        let payload = serde_json::json!({
            "conferenceId": "conference",
            "conferenceSolution": {
                "name": "Future provider", "iconUri": "https://example.com/icon",
                "key": {"type": "futureProvider"}
            },
            "createRequest": {
                "requestId": "request", "conferenceSolutionKey": {"type": ""},
                "status": {"statusCode": "futureStatus"}
            },
            "entryPoints": [{
                "entryPointType": "sip", "uri": "sip:123@example.com", "label": "SIP",
                "accessCode": "access", "meetingCode": "meeting", "passcode": "passcode",
                "password": "password", "pin": "pin", "regionCode": "SE",
                "entryPointFeatures": ["toll", "futureFeature"]
            }, {"entryPointType": "futureEntry", "uri": "https://example.com/join"}],
            "parameters": {"addOnParameters": {"parameters": {"streaming": "enabled"}}},
            "notes": "instructions", "signature": "server-signature"
        });
        let conference: EventConferenceData = serde_json::from_value(payload.clone()).unwrap();
        let entries = conference.entry_points.as_ref().unwrap();
        assert_eq!(
            entries[0].entry_point_type,
            Some(EventConferenceEntryPointType::SIP)
        );
        assert_eq!(
            entries[1].entry_point_type,
            Some(EventConferenceEntryPointType::Unknown(
                "futureEntry".to_string()
            ))
        );
        assert_eq!(serde_json::to_value(conference).unwrap(), payload);
    }

    #[test]
    fn every_open_enum_preserves_unknown_wire_values() {
        macro_rules! assert_unknown {
            ($($typ:ty),+ $(,)?) => {$(
                let value: $typ = serde_json::from_str("\"futureValue\"").unwrap();
                assert_eq!(value.as_str(), "futureValue");
                assert_eq!(serde_json::to_string(&value).unwrap(), "\"futureValue\"");
            )+};
        }
        assert_unknown!(
            EventOfficeLocationType,
            EventVisibility,
            EventTransparency,
            EventStatus,
            EventGadgetDisplay,
            EventType,
            EventConferenceEntryPointType,
            EventConferenceStatusCode,
            EventConferenceSolutionKeyType,
            EventResponseStatus,
            EventAutoDeclineMode,
            EventChatStatus,
            EventBirthdayType
        );
    }
}
