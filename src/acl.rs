//! Calendar access-control rules, safe writes, and incremental synchronization.
//!
//! Deleted rules have role [`AclRole::None`] and may omit their scope. Keep those
//! tombstones when applying a delta. [`AclClient::sync`] collects every page but
//! never changes application storage; HTTP 410 requires a fresh full sync. Writes
//! support `If-Match` and return HTTP 412 conflicts without retrying.
//!
//! See <https://developers.google.com/workspace/calendar/api/v3/reference/acl>.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::{
    request::{segment, ApiRequest},
    ClientError, ClientResult, GCalClient, Patch, QueryParams,
};

// Like the event models, preserve unknown response strings instead of losing a page.
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

open_enum!(AclRole, None, {
    None => "none", FreeBusyReader => "freeBusyReader", Reader => "reader",
    WriterWithoutPrivateAccess => "writerWithoutPrivateAccess", Writer => "writer", Owner => "owner"
});
open_enum!(AclScopeType, Default, {
    Default => "default", User => "user", Group => "group", Domain => "domain"
});

/// Principal receiving access. A public (`default`) scope omits `value`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct AclScope {
    #[serde(rename = "type")]
    pub typ: AclScopeType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// Read model, including sparse deletion tombstones.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct AclRule {
    pub kind: String,
    pub etag: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<AclScope>,
    pub role: AclRole,
}

/// One ACL page, or an aggregated result with the last page's sync token.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct Acl {
    pub kind: String,
    pub etag: String,
    pub items: Vec<AclRule>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_sync_token: Option<String>,
}

/// Writable rule properties only; no resource ID, kind, or ETag enters the body.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AclRuleWrite {
    pub scope: AclScope,
    pub role: AclRole,
}

/// Nested scope patch: changing `value` does not inject a default scope type.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct AclScopePatch {
    #[serde(rename = "type", skip_serializing_if = "Patch::is_unset")]
    pub typ: Patch<AclScopeType>,
    #[serde(skip_serializing_if = "Patch::is_unset")]
    pub value: Patch<String>,
}

/// Omitted properties remain unchanged; `Patch::Null` explicitly clears a property.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct AclRulePatch {
    #[serde(skip_serializing_if = "Patch::is_unset")]
    pub scope: Patch<AclScopePatch>,
    #[serde(skip_serializing_if = "Patch::is_unset")]
    pub role: Patch<AclRole>,
}

/// List/watch parameters. Typed fields override matching raw pairs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AclListOptions {
    pub max_results: Option<u32>,
    pub page_token: Option<String>,
    pub show_deleted: Option<bool>,
    pub sync_token: Option<String>,
    pub extra: QueryParams,
}

impl AclListOptions {
    #[must_use]
    pub fn query(&self) -> QueryParams {
        let mut query = self.extra.clone();
        if let Some(value) = self.max_results {
            query.insert("maxResults".into(), value.to_string());
        }
        if let Some(value) = &self.page_token {
            query.insert("pageToken".into(), value.clone());
        }
        if let Some(value) = self.show_deleted {
            query.insert("showDeleted".into(), value.to_string());
        }
        if let Some(value) = &self.sync_token {
            query.insert("syncToken".into(), value.clone());
        }
        query
    }

    pub(crate) fn validate_query(query: &QueryParams) -> ClientResult<()> {
        if query.get("syncToken").is_some()
            && query
                .iter()
                .any(|(key, value)| key == "showDeleted" && value == "false")
        {
            return Err(ClientError::InvalidRequest(
                "ACL syncToken cannot be combined with showDeleted=false".into(),
            ));
        }
        Ok(())
    }
}

/// Sharing notifications default to Google's behavior when omitted.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AclWriteOptions {
    pub send_notifications: Option<bool>,
    pub if_match: Option<String>,
    pub extra: QueryParams,
}

impl AclWriteOptions {
    #[must_use]
    pub fn query(&self) -> QueryParams {
        let mut query = self.extra.clone();
        if let Some(value) = self.send_notifications {
            query.insert("sendNotifications".into(), value.to_string());
        }
        query
    }
}

/// Deletion supports conditional mutation, but has no notification flag.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AclDeleteOptions {
    pub if_match: Option<String>,
    pub extra: QueryParams,
}

fn collection(calendar_id: &str) -> String {
    format!("calendars/{}/acl", segment(calendar_id))
}

fn resource(calendar_id: &str, rule_id: &str) -> String {
    format!("{}/{}", collection(calendar_id), segment(rule_id))
}

/// Typed ACL operations. Watch subscriptions are exposed with the Channel API.
#[derive(Debug, Clone)]
pub struct AclClient(pub(crate) Arc<GCalClient>);

impl AclClient {
    #[must_use]
    pub const fn new(client: Arc<GCalClient>) -> Self {
        Self(client)
    }

    /// Fetch one permission rule.
    ///
    /// # Errors
    /// Returns transport, API, or response decoding errors.
    pub async fn get(&self, calendar_id: &str, rule_id: &str) -> ClientResult<AclRule> {
        Ok(self
            .0
            .get(None, ApiRequest::new(resource(calendar_id, rule_id)))
            .await?
            .json()
            .await?)
    }

    /// Read one page, preserving both continuation tokens and deletion tombstones.
    ///
    /// # Errors
    /// Rejects `showDeleted=false` with a sync token, including raw query pairs.
    /// Also returns transport, API, or response decoding errors.
    pub async fn list_page(
        &self,
        calendar_id: &str,
        options: &AclListOptions,
    ) -> ClientResult<Acl> {
        let mut request = ApiRequest::new(collection(calendar_id));
        request.query = options.query();
        AclListOptions::validate_query(&request.query)?;
        Ok(self.0.get(None, request).await?.json().await?)
    }

    /// Collect all pages, including empty intermediate pages, without changing options.
    ///
    /// # Errors
    /// Returns validation, transport, API, or response decoding errors for any page.
    pub async fn list_all(
        &self,
        calendar_id: &str,
        mut options: AclListOptions,
    ) -> ClientResult<Acl> {
        let mut result = self.list_page(calendar_id, &options).await?;
        while let Some(token) = result.next_page_token.take() {
            options.page_token = Some(token);
            let mut page = self.list_page(calendar_id, &options).await?;
            result.items.append(&mut page.items);
            result.next_page_token = page.next_page_token;
            result.next_sync_token = page.next_sync_token;
        }
        Ok(result)
    }

    /// Read every delta page since a token. The argument overrides typed/raw sync tokens.
    ///
    /// HTTP 410 is returned unchanged: use `ClientError::requires_full_sync` to decide
    /// when to discard the stored token and read a new full snapshot.
    ///
    /// # Errors
    /// Returns validation, transport, API, or response decoding errors for any page.
    pub async fn sync(
        &self,
        calendar_id: &str,
        sync_token: impl Into<String>,
        mut options: AclListOptions,
    ) -> ClientResult<Acl> {
        options.sync_token = Some(sync_token.into());
        self.list_all(calendar_id, options).await
    }

    /// Grant calendar access with optional sharing notifications.
    ///
    /// # Errors
    /// Returns invalid-header, transport, API, or response decoding errors.
    pub async fn insert(
        &self,
        calendar_id: &str,
        rule: AclRuleWrite,
        options: &AclWriteOptions,
    ) -> ClientResult<AclRule> {
        let mut request = ApiRequest::with_body(collection(calendar_id), rule);
        request.query = options.query();
        request.if_match.clone_from(&options.if_match);
        Ok(self.0.post(None, request).await?.json().await?)
    }

    /// Replace the writable permission properties, optionally requiring an ETag match.
    ///
    /// # Errors
    /// Returns invalid-header, transport, API (including HTTP 412), or decoding errors.
    pub async fn update(
        &self,
        calendar_id: &str,
        rule_id: &str,
        rule: AclRuleWrite,
        options: &AclWriteOptions,
    ) -> ClientResult<AclRule> {
        let mut request = ApiRequest::with_body(resource(calendar_id, rule_id), rule);
        request.query = options.query();
        request.if_match.clone_from(&options.if_match);
        Ok(self.0.put(None, request).await?.json().await?)
    }

    /// Modify only supplied permission properties, optionally requiring an ETag match.
    ///
    /// # Errors
    /// Returns invalid-header, transport, API (including HTTP 412), or decoding errors.
    pub async fn patch(
        &self,
        calendar_id: &str,
        rule_id: &str,
        rule: AclRulePatch,
        options: &AclWriteOptions,
    ) -> ClientResult<AclRule> {
        let mut request = ApiRequest::with_body(resource(calendar_id, rule_id), rule);
        request.query = options.query();
        request.if_match.clone_from(&options.if_match);
        Ok(self.0.patch(None, request).await?.json().await?)
    }

    /// Revoke access, accepting an empty successful response.
    ///
    /// # Errors
    /// Returns invalid-header, transport, or API errors, including HTTP 412 conflicts.
    pub async fn delete(
        &self,
        calendar_id: &str,
        rule_id: &str,
        options: &AclDeleteOptions,
    ) -> ClientResult<()> {
        let mut request = ApiRequest::new(resource(calendar_id, rule_id));
        request.query.clone_from(&options.extra);
        request.if_match.clone_from(&options.if_match);
        self.0.delete(None, request).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::fixture;

    fn grant(role: AclRole) -> AclRuleWrite {
        AclRuleWrite {
            scope: AclScope {
                typ: AclScopeType::User,
                value: Some("person@example.com".into()),
            },
            role,
        }
    }

    #[tokio::test]
    async fn permissions_lifecycle_uses_safe_bodies_notifications_and_preconditions() {
        let (client, observed) = fixture(vec![
            (200, r#"{"id":"user:person@example.com","role":"reader","etag":"v1","scope":{"type":"user","value":"person@example.com"}}"#),
            (200, r#"{"id":"user:person@example.com","role":"reader","etag":"v1"}"#),
            (200, r#"{"id":"user:person@example.com","role":"writerWithoutPrivateAccess","etag":"v2"}"#),
            (200, r#"{"id":"user:person@example.com","role":"writer","etag":"v3"}"#),
            (412, r#"{"error":{"message":"stale permission","errors":[{"reason":"conditionNotMet"}]}}"#),
            (204, ""),
        ]).await;
        let client = AclClient::new(client);
        let calendar = "calendar/a@example.com";
        let id = "user:person@example.com";
        let quiet = AclWriteOptions {
            send_notifications: Some(false),
            ..Default::default()
        };
        let inserted = client
            .insert(calendar, grant(AclRole::Reader), &quiet)
            .await
            .unwrap();
        assert_eq!(
            inserted.scope.unwrap().value.as_deref(),
            Some("person@example.com")
        );
        let fetched = client.get(calendar, id).await.unwrap();
        assert_eq!(fetched.role, AclRole::Reader);
        let conditional = AclWriteOptions {
            send_notifications: Some(false),
            if_match: Some("\"v1\"".into()),
            ..Default::default()
        };
        let updated = client
            .update(
                calendar,
                id,
                grant(AclRole::WriterWithoutPrivateAccess),
                &conditional,
            )
            .await
            .unwrap();
        assert_eq!(updated.role, AclRole::WriterWithoutPrivateAccess);
        let patched = client
            .patch(
                calendar,
                id,
                AclRulePatch {
                    role: Patch::Value(AclRole::Writer),
                    ..Default::default()
                },
                &AclWriteOptions {
                    if_match: Some("\"v2\"".into()),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(patched.etag, "v3");
        let stale = AclDeleteOptions {
            if_match: Some("\"v1\"".into()),
            ..Default::default()
        };
        let error = client.delete(calendar, id, &stale).await.unwrap_err();
        assert_eq!(
            error.status(),
            Some(reqwest::StatusCode::PRECONDITION_FAILED)
        );
        let ClientError::Api(api) = error else {
            panic!("expected structured API error")
        };
        assert_eq!(api.errors[0].reason, "conditionNotMet");
        client
            .delete(
                calendar,
                id,
                &AclDeleteOptions {
                    if_match: Some("\"v3\"".into()),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        let requests = observed.await.unwrap();
        assert_eq!(
            requests
                .iter()
                .map(|request| request.method.as_str())
                .collect::<Vec<_>>(),
            ["POST", "GET", "PUT", "PATCH", "DELETE", "DELETE"]
        );
        for (index, request) in requests.iter().enumerate() {
            let expected = if index == 0 {
                "/calendar/v3/calendars/calendar%2Fa%40example.com/acl"
            } else {
                "/calendar/v3/calendars/calendar%2Fa%40example.com/acl/user%3Aperson%40example.com"
            };
            assert_eq!(request.target.split('?').next().unwrap(), expected);
        }
        for index in [0, 2] {
            let body: serde_json::Value = serde_json::from_slice(&requests[index].body).unwrap();
            assert_eq!(body.as_object().unwrap().len(), 2);
            assert_eq!(body["scope"]["type"], "user");
            assert!(requests[index].target.contains("sendNotifications=false"));
            assert_eq!(requests[index].headers["content-type"], "application/json");
        }
        assert_eq!(requests[2].headers["if-match"], "\"v1\"");
        assert_eq!(requests[3].headers["if-match"], "\"v2\"");
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&requests[3].body).unwrap(),
            serde_json::json!({"role":"writer"})
        );
        assert!(!requests[3].target.contains("sendNotifications"));
        assert!(requests[4].body.is_empty());
        assert_eq!(requests[5].headers["if-match"], "\"v3\"");
    }

    #[tokio::test]
    async fn delta_follows_empty_pages_and_keeps_deleted_rules_and_final_token() {
        let (client, observed) = fixture(vec![
            (200, r#"{"items":[{"id":"deleted","role":"none"}],"nextPageToken":"middle +/"}"#),
            (200, r#"{"nextPageToken":"last"}"#),
            (200, r#"{"items":[{"id":"future","role":"newRole","scope":{"type":"futureScope","value":"x"}}],"nextSyncToken":"new-sync"}"#),
        ]).await;
        let mut options = AclListOptions {
            max_results: Some(2),
            show_deleted: Some(true),
            ..Default::default()
        };
        options.extra.append("future".into(), "one".into());
        options.extra.append("future".into(), "two".into());
        options
            .extra
            .append("syncToken".into(), "overridden".into());
        let result = AclClient::new(client)
            .sync("primary", "old-sync", options)
            .await
            .unwrap();
        assert_eq!(result.items.len(), 2);
        assert_eq!(result.items[0].role, AclRole::None);
        assert!(result.items[0].scope.is_none());
        assert_eq!(result.items[1].role, AclRole::Unknown("newRole".into()));
        assert_eq!(
            result.items[1].scope.as_ref().unwrap().typ,
            AclScopeType::Unknown("futureScope".into())
        );
        assert_eq!(result.next_sync_token.as_deref(), Some("new-sync"));
        assert!(result.next_page_token.is_none());
        for (index, request) in observed.await.unwrap().iter().enumerate() {
            let url = url::Url::parse(&format!("http://localhost{}", request.target)).unwrap();
            let pairs: Vec<_> = url.query_pairs().collect();
            assert_eq!(
                pairs
                    .iter()
                    .filter(|(key, _)| key == "syncToken")
                    .map(|(_, value)| value.as_ref())
                    .collect::<Vec<_>>(),
                ["old-sync"]
            );
            assert_eq!(
                pairs
                    .iter()
                    .filter(|(key, _)| key == "future")
                    .map(|(_, value)| value.as_ref())
                    .collect::<Vec<_>>(),
                ["one", "two"]
            );
            assert!(pairs
                .iter()
                .any(|(key, value)| key == "showDeleted" && value == "true"));
            assert_eq!(
                pairs
                    .iter()
                    .find(|(key, _)| key == "pageToken")
                    .map(|(_, value)| value.as_ref()),
                [None, Some("middle +/"), Some("last")][index]
            );
        }
    }

    #[tokio::test]
    async fn sync_validation_covers_raw_duplicates_before_network_and_preserves_410() {
        let (client, observed) =
            fixture(vec![(410, r#"{"error":{"message":"expired sync token"}}"#)]).await;
        let client = AclClient::new(client);
        let mut raw = AclListOptions::default();
        raw.extra.append("syncToken".into(), "raw-sync".into());
        raw.extra.append("showDeleted".into(), "true".into());
        raw.extra.append("showDeleted".into(), "false".into());
        assert!(matches!(
            client.list_page("primary", &raw).await,
            Err(ClientError::InvalidRequest(_))
        ));
        assert!(matches!(
            client
                .sync(
                    "primary",
                    "sync",
                    AclListOptions {
                        show_deleted: Some(false),
                        ..Default::default()
                    }
                )
                .await,
            Err(ClientError::InvalidRequest(_))
        ));
        raw.show_deleted = Some(true);
        let error = client.list_page("primary", &raw).await.unwrap_err();
        assert!(error.requires_full_sync());
        let requests = observed.await.unwrap();
        assert_eq!(requests.len(), 1);
        let url = url::Url::parse(&format!("http://localhost{}", requests[0].target)).unwrap();
        assert_eq!(
            url.query_pairs()
                .filter(|(key, _)| key == "showDeleted")
                .count(),
            1
        );
    }

    #[test]
    fn safe_write_patch_and_unknown_principals_round_trip() {
        let rule = AclRuleWrite {
            scope: AclScope {
                typ: AclScopeType::Default,
                value: None,
            },
            role: AclRole::FreeBusyReader,
        };
        assert_eq!(
            serde_json::to_value(rule).unwrap(),
            serde_json::json!({
                "scope":{"type":"default"},"role":"freeBusyReader"
            })
        );
        let patch: AclRulePatch =
            serde_json::from_value(serde_json::json!({"scope":null,"role":"newRole"})).unwrap();
        assert_eq!(patch.scope, Patch::Null);
        assert_eq!(
            serde_json::to_value(patch).unwrap(),
            serde_json::json!({"scope":null,"role":"newRole"})
        );
        assert_eq!(
            serde_json::to_value(AclRulePatch::default()).unwrap(),
            serde_json::json!({})
        );
        let scope_only = AclRulePatch {
            scope: Patch::Value(AclScopePatch {
                value: Patch::Value("new@example.com".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(
            serde_json::to_value(scope_only).unwrap(),
            serde_json::json!({
                "scope":{"value":"new@example.com"}
            })
        );
    }
}
