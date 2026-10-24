use serde::{Deserialize, Serialize};

use super::{DefaultReminder, LabelProperties, NotificationSetting};
use crate::Patch;

/// Writable calendar metadata, without IDs, ETags, owners, or response defaults.
///
/// `summary` is required when inserting a calendar. Label properties replace the
/// existing definitions when supplied; `event_labels: Some(vec![])` clears them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CalendarWrite {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_zone: Option<String>,
    /// Only valid for resource calendars.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_accept_invitations: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_properties: Option<LabelProperties>,
}

/// Calendar metadata changes: unset properties are omitted, null clears a
/// property, and values replace it. Label arrays replace the entire array.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CalendarPatch {
    #[serde(skip_serializing_if = "Patch::is_unset")]
    pub summary: Patch<String>,
    #[serde(skip_serializing_if = "Patch::is_unset")]
    pub description: Patch<String>,
    #[serde(skip_serializing_if = "Patch::is_unset")]
    pub location: Patch<String>,
    #[serde(skip_serializing_if = "Patch::is_unset")]
    pub time_zone: Patch<String>,
    #[serde(skip_serializing_if = "Patch::is_unset")]
    pub auto_accept_invitations: Patch<bool>,
    #[serde(skip_serializing_if = "Patch::is_unset")]
    pub label_properties: Patch<LabelProperties>,
}

/// Notification preferences for writes. An empty list explicitly removes all
/// notifications rather than serializing as an omitted list.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct NotificationSettingsWrite {
    pub notifications: Vec<NotificationSetting>,
}

/// Writable per-user calendar-list preferences only.
///
/// The calendar ID is a separate argument to `insert` and is included in that
/// request's body only. Calendar metadata such as summary, owner, access role,
/// automatic invitation acceptance, and conferencing capabilities is read-only
/// on this resource and cannot be sent with this payload.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CalendarListEntryWrite {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary_override: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreground_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_reminders: Option<Vec<DefaultReminder>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notification_settings: Option<NotificationSettingsWrite>,
}

/// Changes to per-user calendar-list preferences. Empty reminder and notification
/// lists explicitly remove all entries; null and omitted properties stay distinct.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CalendarListEntryPatch {
    #[serde(skip_serializing_if = "Patch::is_unset")]
    pub summary_override: Patch<String>,
    #[serde(skip_serializing_if = "Patch::is_unset")]
    pub color_id: Patch<String>,
    #[serde(skip_serializing_if = "Patch::is_unset")]
    pub background_color: Patch<String>,
    #[serde(skip_serializing_if = "Patch::is_unset")]
    pub foreground_color: Patch<String>,
    #[serde(skip_serializing_if = "Patch::is_unset")]
    pub hidden: Patch<bool>,
    #[serde(skip_serializing_if = "Patch::is_unset")]
    pub selected: Patch<bool>,
    #[serde(skip_serializing_if = "Patch::is_unset")]
    pub default_reminders: Patch<Vec<DefaultReminder>>,
    #[serde(skip_serializing_if = "Patch::is_unset")]
    pub notification_settings: Patch<NotificationSettingsWrite>,
}
