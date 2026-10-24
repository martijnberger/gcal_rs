//! Global calendar and event color palettes.
//!
//! See <https://developers.google.com/workspace/calendar/api/v3/reference/colors/get>.

use std::{collections::BTreeMap, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::{request::ApiRequest, ClientResult, GCalClient};

/// Foreground/background CSS color values for a palette entry.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ColorDefinition {
    pub background: String,
    pub foreground: String,
}

/// Global palettes keyed by the color IDs used on calendars and events.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Colors {
    pub kind: String,
    /// Last palette update in RFC3339 format, when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated: Option<String>,
    pub calendar: BTreeMap<String, ColorDefinition>,
    pub event: BTreeMap<String, ColorDefinition>,
}

/// Read-only access to global color definitions.
#[derive(Debug, Clone)]
pub struct ColorsClient(pub(crate) Arc<GCalClient>);

impl ColorsClient {
    #[must_use]
    pub const fn new(client: Arc<GCalClient>) -> Self {
        Self(client)
    }

    /// Fetch both global palettes and their last update time.
    ///
    /// # Errors
    /// Returns transport, API, or response decoding errors.
    pub async fn get(&self) -> ClientResult<Colors> {
        Ok(self
            .0
            .get(None, ApiRequest::new("colors".into()))
            .await?
            .json()
            .await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::fixture;

    #[tokio::test]
    async fn get_preserves_distinct_palettes_and_future_color_ids() {
        let (client, observed) = fixture(vec![(
            200,
            r##"{
            "kind":"calendar#colors","updated":"2026-10-24T08:00:00Z",
            "calendar":{"1":{"background":"#ac725e","foreground":"#1d1d1d"}},
            "event":{"1":{"background":"#a4bdfc","foreground":"#1d1d1d"},
                     "future-id":{"background":"#ffffff","foreground":"#000000"}}
        }"##,
        )])
        .await;
        let colors = ColorsClient::new(client).get().await.unwrap();
        assert_eq!(colors.kind, "calendar#colors");
        assert_eq!(colors.updated.as_deref(), Some("2026-10-24T08:00:00Z"));
        assert_eq!(colors.calendar["1"].background, "#ac725e");
        assert_eq!(colors.event["1"].background, "#a4bdfc");
        assert_eq!(colors.event["future-id"].foreground, "#000000");
        let round_trip: Colors =
            serde_json::from_value(serde_json::to_value(&colors).unwrap()).unwrap();
        assert_eq!(round_trip, colors);
        let requests = observed.await.unwrap();
        assert_eq!(requests[0].method, "GET");
        assert_eq!(requests[0].target, "/calendar/v3/colors");
        assert!(requests[0].body.is_empty());
    }
}
