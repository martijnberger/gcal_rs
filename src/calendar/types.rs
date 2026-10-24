use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CalendarAccessRole {
    #[default]
    Owner,
    Reader,
    Writer,
    WriterWithoutPrivateAccess,
    FreeBusyReader,
    #[serde(untagged)]
    Unknown(String),
}
impl CalendarAccessRole {
    #[must_use]
    pub fn to_str(&self) -> &str {
        match self {
            Self::Owner => "owner",
            Self::Reader => "reader",
            Self::Writer => "writer",
            Self::WriterWithoutPrivateAccess => "writerWithoutPrivateAccess",
            Self::FreeBusyReader => "freeBusyReader",
            Self::Unknown(value) => value,
        }
    }
}
impl std::fmt::Display for CalendarAccessRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct NotificationSettings {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notifications: Vec<NotificationSetting>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NotificationSetting {
    pub method: NotificationSettingMethod,
    #[serde(rename = "type")]
    pub typ: NotificationSettingType,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NotificationSettingMethod {
    #[serde(rename = "email")]
    #[default]
    EMail,
    #[serde(untagged)]
    Unknown(String),
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NotificationSettingType {
    #[default]
    EventCreation,
    EventChange,
    EventCancellation,
    EventResponse,
    Agenda,
    #[serde(untagged)]
    Unknown(String),
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct ConferenceProperties {
    #[serde(rename = "allowedConferenceSolutionTypes")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub allowed_solution_types: Vec<AllowedSolutionType>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DefaultReminder {
    pub method: ReminderMethod,
    pub minutes: i32,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ReminderMethod {
    #[serde(rename = "email")]
    EMail,
    #[serde(rename = "popup")]
    #[default]
    PopUp,
    #[serde(untagged)]
    Unknown(String),
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AllowedSolutionType {
    #[default]
    EventHangout,
    EventNamedHangout,
    HangoutsMeet,
    #[serde(untagged)]
    Unknown(String),
}

/// Label definitions attached to a calendar.
#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct LabelProperties {
    /// `Some(vec![])` explicitly clears the calendar's label definitions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_labels: Option<Vec<EventLabel>>,
}

/// A label defined on a calendar and referenced by an event's label ID.
#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct EventLabel {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,
}
