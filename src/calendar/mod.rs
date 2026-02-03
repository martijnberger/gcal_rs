use serde::{Deserialize, Serialize};

mod client;
pub use client::*;

/// Calendar List, the normal way to get at the list of calendars available.
pub mod list;
pub use list::*;

mod types;
pub use types::*;

use crate::{QueryParams, Sendable};

/* Google Calendar API: https://developers.google.com/calendar/api/v3/reference/calendars#resource */

/// Calendar is a single calendar.
#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
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
    pub conference_properties: Option<types::ConferenceProperties>,
}
impl Sendable for Calendar {
    fn path(&self, _action: Option<String>) -> String {
        format!("calendars/{}", self.id)
    }

    fn query(&self) -> QueryParams {
        QueryParams::default()
    }
}
fn default_kind() -> String {
    "calendar#calendar".to_string()
}
