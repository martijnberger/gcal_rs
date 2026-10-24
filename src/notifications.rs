use std::{fmt, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::{
    request::{segment, ApiRequest},
    AclClient, AclListOptions, AdditionalProperties, CalendarListClient, CalendarListOptions,
    ClientResult, EventClient, EventListOptions, GCalClient, SettingsClient, SettingsListOptions,
};

/// Google's notification channel response. Applications own renewal and webhook verification.
#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct Channel {
    pub kind: String,
    pub id: String,
    pub resource_id: String,
    pub resource_uri: String,
    #[serde(rename = "type")]
    pub typ: String,
    pub address: Option<String>,
    pub token: Option<String>,
    /// Unix timestamp in milliseconds, encoded as a string by Google.
    pub expiration: Option<String>,
    pub params: AdditionalProperties,
    pub payload: Option<bool>,
}

impl fmt::Debug for Channel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Channel")
            .field("id", &self.id)
            .field("resource_id", &self.resource_id)
            .field("expiration", &self.expiration)
            .field("token", &self.token.as_ref().map(|_| "[REDACTED]"))
            .finish_non_exhaustive()
    }
}

/// A new webhook subscription, without response-only resource identifiers.
#[derive(Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChannelRequest {
    pub id: String,
    #[serde(rename = "type")]
    pub typ: String,
    pub address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration: Option<String>,
    #[serde(skip_serializing_if = "AdditionalProperties::is_empty")]
    pub params: AdditionalProperties,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<bool>,
}

impl ChannelRequest {
    /// Create a webhook channel. Use an unguessable token and validate inbound headers.
    ///
    /// ```no_run
    /// use std::sync::Arc;
    /// use gcal_rs::{ChannelRequest, ChannelStop, ClientResult, EventListOptions, GCalClient};
    ///
    /// async fn subscribe(client: Arc<GCalClient>, unique_id: String, secret: String)
    ///     -> ClientResult<()>
    /// {
    ///     let mut request = ChannelRequest::new(unique_id, "https://example.com/calendar-hook");
    ///     request.token = Some(secret);
    ///     let channel = client.clone().event_client()
    ///         .watch("primary", request, &EventListOptions::default()).await?;
    ///     // Persist channel.id/resource_id/expiration for renewal and header verification.
    ///     // Stop this channel when it is no longer needed.
    ///     client.channels_client().stop(ChannelStop::from(&channel)).await
    /// }
    /// ```
    #[must_use]
    pub fn new(id: impl Into<String>, address: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            typ: "web_hook".into(),
            address: address.into(),
            token: None,
            expiration: None,
            params: AdditionalProperties::new(),
            payload: None,
        }
    }
}

impl fmt::Debug for ChannelRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ChannelRequest")
            .field("id", &self.id)
            .field("type", &self.typ)
            .field("token", &self.token.as_ref().map(|_| "[REDACTED]"))
            .finish_non_exhaustive()
    }
}

/// Stop requires both the original channel ID and Google's returned resource ID.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChannelStop {
    pub id: String,
    pub resource_id: String,
}

impl From<&Channel> for ChannelStop {
    fn from(channel: &Channel) -> Self {
        Self {
            id: channel.id.clone(),
            resource_id: channel.resource_id.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChannelsClient(pub(crate) Arc<GCalClient>);

impl ChannelsClient {
    #[must_use]
    pub const fn new(client: Arc<GCalClient>) -> Self {
        Self(client)
    }

    /// Stop an existing notification channel.
    /// # Errors
    /// Returns transport or API errors; unknown channels are not silently ignored.
    pub async fn stop(&self, channel: ChannelStop) -> ClientResult<()> {
        self.0
            .post(None, ApiRequest::with_body("channels/stop".into(), channel))
            .await?;
        Ok(())
    }
}

impl EventClient {
    /// Watch events matching the same query options as list. Renew channels before expiration.
    /// # Errors
    /// Returns invalid sync-filter, transport, decoding, or API errors.
    pub async fn watch(
        &self,
        calendar_id: &str,
        channel: ChannelRequest,
        options: &EventListOptions,
    ) -> ClientResult<Channel> {
        let mut request = ApiRequest::with_body(
            format!("calendars/{}/events/watch", segment(calendar_id)),
            channel,
        );
        request.query = options.query();
        EventListOptions::validate(&request.query)?;
        Ok(self.0.post(None, request).await?.json().await?)
    }
}

impl CalendarListClient {
    /// Watch changes to the authenticated user's calendar list.
    /// # Errors
    /// Returns invalid sync-filter, transport, decoding, or API errors.
    pub async fn watch(
        &self,
        channel: ChannelRequest,
        options: &CalendarListOptions,
    ) -> ClientResult<Channel> {
        let mut request = ApiRequest::with_body("users/me/calendarList/watch".into(), channel);
        request.query = options.query();
        CalendarListOptions::validate(&request.query)?;
        Ok(self.0.post(None, request).await?.json().await?)
    }
}

impl SettingsClient {
    /// Watch changes to the authenticated user's settings.
    /// # Errors
    /// Returns transport, decoding, or API errors.
    pub async fn watch(
        &self,
        channel: ChannelRequest,
        options: &SettingsListOptions,
    ) -> ClientResult<Channel> {
        let mut request = ApiRequest::with_body("users/me/settings/watch".into(), channel);
        request.query = options.query();
        Ok(self.0.post(None, request).await?.json().await?)
    }
}

impl AclClient {
    /// Watch changes to calendar sharing rules.
    /// # Errors
    /// Returns invalid sync-filter, transport, decoding, or API errors.
    pub async fn watch(
        &self,
        calendar_id: &str,
        channel: ChannelRequest,
        options: &AclListOptions,
    ) -> ClientResult<Channel> {
        let mut request = ApiRequest::with_body(
            format!("calendars/{}/acl/watch", segment(calendar_id)),
            channel,
        );
        request.query = options.query();
        AclListOptions::validate_query(&request.query)?;
        Ok(self.0.post(None, request).await?.json().await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::fixture;

    #[tokio::test]
    async fn subscriptions_return_stop_identity_and_stop_uses_only_required_identity() {
        let response = r#"{"id":"watch","resourceId":"stable-resource","resourceUri":"https://www.googleapis.com/calendar/v3/calendars/primary/events","expiration":"1800000000000"}"#;
        let (transport, observed) = fixture(vec![
            (200, response),
            (200, response),
            (200, response),
            (200, response),
            (204, ""),
        ])
        .await;
        let mut channel = ChannelRequest::new("watch", "https://example.com/calendar-hook");
        channel.token = Some("verification-secret".into());
        assert!(!format!("{channel:?}").contains("verification-secret"));
        let event = transport
            .clone()
            .event_client()
            .watch("primary", channel.clone(), &EventListOptions::default())
            .await
            .unwrap();
        transport
            .clone()
            .calendar_client()
            .watch(channel.clone(), &CalendarListOptions::default())
            .await
            .unwrap();
        transport
            .clone()
            .settings_client()
            .watch(channel.clone(), &SettingsListOptions::default())
            .await
            .unwrap();
        transport
            .clone()
            .acl_client()
            .watch("primary", channel, &AclListOptions::default())
            .await
            .unwrap();
        assert_eq!(event.expiration.as_deref(), Some("1800000000000"));
        transport
            .channels_client()
            .stop((&event).into())
            .await
            .unwrap();
        let requests = observed.await.unwrap();
        for request in &requests[..4] {
            let payload: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
            assert_eq!(payload["type"], "web_hook");
            assert_eq!(payload["token"], "verification-secret");
            assert!(payload.get("resourceId").is_none());
        }
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&requests[4].body).unwrap(),
            serde_json::json!({"id":"watch","resourceId":"stable-resource"})
        );
        assert_eq!(requests[4].target, "/calendar/v3/channels/stop");
    }
}
