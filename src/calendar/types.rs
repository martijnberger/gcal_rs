use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CalendarAccessRole {
    #[default]
    Owner,
    Reader,
    Writer,
    FreeBusyReader,
}
impl CalendarAccessRole {
    pub const fn to_str(&self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Reader => "reader",
            Self::Writer => "writer",
            Self::FreeBusyReader => "freeBusyReader",
        }
    }
}
impl std::fmt::Display for CalendarAccessRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
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

#[derive(Serialize, Deserialize, Default, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NotificationSettingMethod {
    #[serde(rename = "email")]
    #[default]
    EMail,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NotificationSettingType {
    #[default]
    EventCreation,
    EventChange,
    EventCancellation,
    EventResponse,
    Agenda,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConferenceProperties {
    #[serde(rename = "allowedConferenceSolutionTypes")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub allowed_solution_types: Vec<AllowedSolutionType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_reminders: Option<Vec<DefaultReminder>>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DefaultReminder {
    pub method: ReminderMethod,
    pub minutes: u16,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ReminderMethod {
    #[serde(rename = "email")]
    EMail,
    #[serde(rename = "popup")]
    #[default]
    PopUp,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AllowedSolutionType {
    #[default]
    EventHangout,
    EventNamedHangout,
    HangoutsMeet,
}
