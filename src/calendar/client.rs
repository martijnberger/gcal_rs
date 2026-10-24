use std::sync::Arc;

use super::{CalendarAccessRole, CalendarList, CalendarListItem};
use crate::{ClientResult, GCalClient, QueryParams};

/// `CalendarListClient` is the method of accessing the calendar list. You must provide it with a
/// Google Calendar client.
#[derive(Debug, Clone)]
pub struct CalendarListClient(pub(crate) Arc<GCalClient>);

impl CalendarListClient {
    /// Construct a `CalendarListClient`. Requires a Google Calendar Client.
    #[must_use]
    pub const fn new(client: Arc<GCalClient>) -> Self {
        Self(client)
    }

    /// List every page of calendars, including empty intermediate pages.
    ///
    /// # Errors
    /// Returns an error if the request fails or the response cannot be parsed.
    pub async fn list(
        &self,
        hidden: bool,
        access_role: CalendarAccessRole,
    ) -> ClientResult<Vec<CalendarListItem>> {
        Ok(self
            .list_all(CalendarListOptions {
                show_hidden: Some(hidden),
                min_access_role: Some(access_role),
                ..CalendarListOptions::default()
            })
            .await?
            .items)
    }

    /// Fetch a single page without consuming or modifying reusable options.
    ///
    /// # Errors
    /// Returns an error if the request fails or the response cannot be parsed.
    pub async fn list_page(&self, options: &CalendarListOptions) -> ClientResult<CalendarList> {
        let mut request = CalendarList::default();
        request.set_query(options.query());
        Ok(self
            .0
            .get(None, request)
            .await?
            .json::<CalendarList>()
            .await?)
    }

    /// Collect all continuation pages, preserving the final synchronization token.
    ///
    /// # Errors
    /// Returns an error if any request fails or a response cannot be parsed.
    pub async fn list_all(&self, mut options: CalendarListOptions) -> ClientResult<CalendarList> {
        let mut result = self.list_page(&options).await?;
        while let Some(token) = result.next_page_token.take() {
            options.page_token = Some(token);
            let mut page = self.list_page(&options).await?;
            result.items.append(&mut page.items);
            result.next_page_token = page.next_page_token;
            result.next_sync_token = page.next_sync_token;
        }
        Ok(result)
    }
}

/// Parameters for `calendarList.list`. Typed fields override matching raw keys.
///
/// Google restricts which filters can be combined with a synchronization token.
/// These options expose the wire parameters; they do not manage synchronization.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CalendarListOptions {
    pub max_results: Option<u32>,
    pub min_access_role: Option<CalendarAccessRole>,
    pub page_token: Option<String>,
    pub show_deleted: Option<bool>,
    pub show_hidden: Option<bool>,
    pub show_own_organization_only: Option<bool>,
    pub sync_token: Option<String>,
    /// Additional query pairs, including repeated keys and global API parameters.
    pub extra: QueryParams,
}

impl CalendarListOptions {
    /// Build query pairs without modifying the options.
    #[must_use]
    pub fn query(&self) -> QueryParams {
        let mut query = self.extra.clone();
        if let Some(value) = self.max_results {
            query.insert("maxResults".into(), value.to_string());
        }
        if let Some(value) = &self.min_access_role {
            query.insert("minAccessRole".into(), value.to_string());
        }
        if let Some(value) = &self.page_token {
            query.insert("pageToken".into(), value.clone());
        }
        if let Some(value) = self.show_deleted {
            query.insert("showDeleted".into(), value.to_string());
        }
        if let Some(value) = self.show_hidden {
            query.insert("showHidden".into(), value.to_string());
        }
        if let Some(value) = self.show_own_organization_only {
            query.insert("showOwnOrganizationOnly".into(), value.to_string());
        }
        if let Some(value) = &self.sync_token {
            query.insert("syncToken".into(), value.clone());
        }
        query
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::fixture;

    #[tokio::test]
    async fn all_pages_preserve_filters_repeated_extras_and_final_sync_token() {
        let (client, observed) = fixture(vec![
            (200, r#"{"items":[{"id":"first","accessRole":"writerWithoutPrivateAccess"}],"nextPageToken":"middle +/"}"#),
            (200, r#"{"nextPageToken":"last"}"#),
            (200, r#"{"items":[{"id":"removed","deleted":true}],"nextSyncToken":"final-sync"}"#),
        ]).await;
        let mut options = CalendarListOptions {
            max_results: Some(2),
            min_access_role: Some(CalendarAccessRole::WriterWithoutPrivateAccess),
            show_deleted: Some(true),
            show_hidden: Some(true),
            show_own_organization_only: Some(false),
            ..CalendarListOptions::default()
        };
        options.extra.append("futureFilter".into(), "one".into());
        options.extra.append("futureFilter".into(), "two".into());
        options
            .extra
            .append("minAccessRole".into(), "reader".into());
        options.extra.append("minAccessRole".into(), "owner".into());
        let list = CalendarListClient::new(client)
            .list_all(options)
            .await
            .unwrap();
        assert_eq!(list.items.len(), 2);
        assert_eq!(list.items[1].deleted, Some(true));
        assert_eq!(list.items[1].access_role, None);
        assert_eq!(list.next_sync_token.as_deref(), Some("final-sync"));
        assert_eq!(list.next_page_token, None);
        for (index, request) in observed.await.unwrap().iter().enumerate() {
            assert_eq!(request.method, "GET");
            let url = url::Url::parse(&format!("http://localhost{}", request.target)).unwrap();
            assert_eq!(url.path(), "/calendar/v3/users/me/calendarList");
            let query: Vec<_> = url.query_pairs().collect();
            for (key, value) in [
                ("maxResults", "2"),
                ("minAccessRole", "writerWithoutPrivateAccess"),
                ("showDeleted", "true"),
                ("showHidden", "true"),
                ("showOwnOrganizationOnly", "false"),
            ] {
                assert!(query.iter().any(|pair| pair.0 == key && pair.1 == value));
            }
            assert_eq!(
                query
                    .iter()
                    .filter(|pair| pair.0 == "minAccessRole")
                    .count(),
                1
            );
            assert_eq!(
                query
                    .iter()
                    .filter(|pair| pair.0 == "futureFilter")
                    .map(|pair| pair.1.as_ref())
                    .collect::<Vec<_>>(),
                ["one", "two"]
            );
            let token = query
                .iter()
                .find(|pair| pair.0 == "pageToken")
                .map(|pair| pair.1.as_ref());
            assert_eq!(token, [None, Some("middle +/"), Some("last")][index]);
        }
    }

    #[tokio::test]
    async fn page_options_are_reusable_and_preserve_response_tokens() {
        let (client, observed) = fixture(vec![
            (200, r#"{"nextPageToken":"next"}"#),
            (200, r#"{"nextSyncToken":"done"}"#),
        ])
        .await;
        let client = CalendarListClient::new(client);
        let options = CalendarListOptions {
            page_token: Some("initial".into()),
            sync_token: Some("previous-sync".into()),
            ..CalendarListOptions::default()
        };
        let original = options.clone();
        let first = client.list_page(&options).await.unwrap();
        let second = client.list_page(&options).await.unwrap();
        assert_eq!(first.next_page_token.as_deref(), Some("next"));
        assert_eq!(second.next_sync_token.as_deref(), Some("done"));
        assert_eq!(options, original);
        let requests = observed.await.unwrap();
        assert_eq!(requests[0].target, requests[1].target);
        assert!(requests[0].target.contains("syncToken=previous-sync"));
    }

    #[tokio::test]
    async fn convenience_list_does_not_stop_at_an_empty_page() {
        let (client, observed) = fixture(vec![
            (
                200,
                r#"{"items":[{"id":"first"}],"nextPageToken":"middle"}"#,
            ),
            (200, r#"{"items":[],"nextPageToken":"last"}"#),
            (200, r#"{"items":[{"id":"last"}]}"#),
        ])
        .await;
        let items = CalendarListClient::new(client)
            .list(false, CalendarAccessRole::Reader)
            .await
            .unwrap();
        assert_eq!(
            items
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            ["first", "last"]
        );
        assert_eq!(observed.await.unwrap().len(), 3);
    }
}
