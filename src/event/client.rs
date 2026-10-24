use std::sync::Arc;

use super::{
    collection_path, item_path, Event, EventInstancesOptions, EventListOptions, EventWrite, Events,
    SendUpdates,
};
use crate::request::ApiRequest;
use crate::{ClientResult, GCalClient};

/// `EventClient` is the method of managing events from a specific calendar. Requires a Google
/// Calendar client.
#[derive(Debug, Clone)]
pub struct EventClient(Arc<GCalClient>);

fn write_request(path: String, event: EventWrite) -> ApiRequest<EventWrite> {
    let supports_attachments = event
        .attachments
        .as_ref()
        .is_some_and(|items| !items.is_empty());
    let mut request = ApiRequest::with_body(path, event);
    if supports_attachments {
        request
            .query
            .insert("supportsAttachments".to_string(), "true".to_string());
    }
    request
}

impl EventClient {
    /// Construct a new `EventClient`. Requires a Google Calendar Client.
    #[must_use]
    pub const fn new(client: Arc<GCalClient>) -> Self {
        Self(client)
    }

    /// Delete the event.
    ///
    /// # Errors
    /// Returns an error if the request fails.
    pub async fn delete(&self, event: Event) -> ClientResult<()> {
        self.0.delete(None, event).await?;
        Ok(())
    }

    /// Get an event by ID.
    ///
    /// # Errors
    /// Returns an error if the request fails or the response cannot be parsed.
    pub async fn get(&self, calendar_id: String, event_id: String) -> ClientResult<Event> {
        let request = ApiRequest::new(item_path(&calendar_id, &event_id));
        let mut event = self.0.get(None, request).await?.json::<Event>().await?;
        event.calendar_id = calendar_id;
        Ok(event)
    }

    /// Import an event into the calendar's collection, even when the payload supplies an ID.
    /// Unlike insert, import requires an `iCalUID` in the payload.
    ///
    /// # Errors
    /// Returns an error if the request fails or the response cannot be parsed.
    pub async fn import(&self, calendar_id: String, event: EventWrite) -> ClientResult<Event> {
        let request = write_request(format!("{}/import", collection_path(&calendar_id)), event);
        let mut event = self.0.post(None, request).await?.json::<Event>().await?;
        event.calendar_id = calendar_id;
        Ok(event)
    }

    /// Insert a writable event into the calendar's collection.
    ///
    /// # Errors
    /// Returns an error if the request fails or the response cannot be parsed.
    pub async fn insert(&self, calendar_id: String, event: EventWrite) -> ClientResult<Event> {
        let request = write_request(collection_path(&calendar_id), event);
        let mut event = self.0.post(None, request).await?.json::<Event>().await?;
        event.calendar_id = calendar_id;
        Ok(event)
    }

    /// Retrieve one recurring-instance page.
    /// # Errors
    /// Returns API, transport, or decoding errors.
    pub async fn instances_page(
        &self,
        calendar_id: &str,
        event_id: &str,
        options: &EventInstancesOptions,
    ) -> ClientResult<Events> {
        let mut request =
            ApiRequest::new(format!("{}/instances", item_path(calendar_id, event_id)));
        request.query = options.query();
        let mut page = self.0.get(None, request).await?.json::<Events>().await?;
        page.add_calendar(calendar_id);
        Ok(page)
    }

    /// Retrieve every recurring instance, including empty intermediate pages.
    /// # Errors
    /// Returns API, transport, or decoding errors.
    pub async fn instances_all(
        &self,
        calendar_id: &str,
        event_id: &str,
        mut options: EventInstancesOptions,
    ) -> ClientResult<Events> {
        let mut result = self.instances_page(calendar_id, event_id, &options).await?;
        while let Some(token) = result.next_page_token.take() {
            options.page_token = Some(token);
            let page = self.instances_page(calendar_id, event_id, &options).await?;
            result.items.extend(page.items);
            result.next_page_token = page.next_page_token;
            result.next_sync_token = page.next_sync_token;
        }
        Ok(result)
    }

    /// Retrieve all instances for the supplied recurring event.
    /// # Errors
    /// Returns API, transport, or decoding errors.
    pub async fn instances(&self, event: Event) -> ClientResult<Events> {
        self.instances_all(
            &event.calendar_id,
            &event.id,
            EventInstancesOptions {
                extra: event.query_string,
                ..Default::default()
            },
        )
        .await
    }

    /// Retrieve one page with its continuation and synchronization tokens.
    /// # Errors
    /// Returns API, transport, or decoding errors.
    pub async fn list_page(
        &self,
        calendar_id: &str,
        options: &EventListOptions,
    ) -> ClientResult<Events> {
        let mut request = ApiRequest::new(collection_path(calendar_id));
        request.query = options.query();
        let mut page = self.0.get(None, request).await?.json::<Events>().await?;
        page.add_calendar(calendar_id);
        Ok(page)
    }

    /// Retrieve all pages without losing the final synchronization token.
    /// # Errors
    /// Returns API, transport, or decoding errors.
    pub async fn list_all(
        &self,
        calendar_id: &str,
        mut options: EventListOptions,
    ) -> ClientResult<Events> {
        let mut result = self.list_page(calendar_id, &options).await?;
        while let Some(token) = result.next_page_token.take() {
            options.page_token = Some(token);
            let page = self.list_page(calendar_id, &options).await?;
            result.items.extend(page.items);
            result.next_page_token = page.next_page_token;
            result.next_sync_token = page.next_sync_token;
        }
        Ok(result)
    }

    /// List every event in a time window, expanding recurring events.
    /// # Errors
    /// Returns API, transport, or decoding errors.
    pub async fn list(
        &self,
        calendar_id: String,
        start_time: chrono::DateTime<chrono::Local>,
        end_time: chrono::DateTime<chrono::Local>,
    ) -> ClientResult<Vec<Event>> {
        Ok(self
            .list_all(
                &calendar_id,
                EventListOptions {
                    time_min: Some(start_time.to_rfc3339()),
                    time_max: Some(end_time.to_rfc3339()),
                    single_events: Some(true),
                    order_by: Some("startTime".into()),
                    ..Default::default()
                },
            )
            .await?
            .items)
    }

    /// Move an event to another calendar and return it with the destination calendar context.
    /// Omitting `send_updates` leaves Google's notification default unchanged.
    ///
    /// # Errors
    /// Returns an error if the request fails or the response cannot be parsed.
    pub async fn move_to_calendar(
        &self,
        event: Event,
        destination: String,
        send_updates: Option<SendUpdates>,
    ) -> ClientResult<Event> {
        let mut request =
            ApiRequest::new(format!("{}/move", item_path(&event.calendar_id, &event.id)));
        request.query = event.query_string;
        request
            .query
            .insert("destination".to_string(), destination.clone());
        if let Some(send_updates) = send_updates {
            request
                .query
                .insert("sendUpdates".to_string(), send_updates.to_string());
        } else {
            request.query.remove("sendUpdates");
        }
        let mut event = self.0.post(None, request).await?.json::<Event>().await?;
        event.calendar_id = destination;
        Ok(event)
    }

    /// Quick-add an event to the selected calendar using natural-language text.
    ///
    /// # Errors
    /// Returns an error if the request fails or the response cannot be parsed.
    pub async fn add(&self, calendar_id: String, text: String) -> ClientResult<Event> {
        let mut request = ApiRequest::new(format!("{}/quickAdd", collection_path(&calendar_id)));
        request.query.insert("text".to_string(), text);
        let mut event = self.0.post(None, request).await?.json::<Event>().await?;
        event.calendar_id = calendar_id;
        Ok(event)
    }

    /// Replace an event's writable properties at the explicitly selected event ID.
    /// Omitted properties follow Google's full-update semantics, not PATCH semantics.
    ///
    /// # Errors
    /// Returns an error if the request fails or the response cannot be parsed.
    pub async fn update(
        &self,
        calendar_id: String,
        event_id: String,
        event: EventWrite,
    ) -> ClientResult<Event> {
        let request = write_request(item_path(&calendar_id, &event_id), event);
        let mut event = self.0.put(None, request).await?.json::<Event>().await?;
        event.calendar_id = calendar_id;
        Ok(event)
    }
}

#[cfg(test)]
mod tests {
    use super::{Event, EventClient, EventWrite, SendUpdates};
    use crate::event::types::EventAttachmentWrite;
    use crate::test_support::fixture;

    #[tokio::test]
    async fn writes_use_collection_and_item_paths_with_only_explicit_payload_fields() {
        let (client, observed) = fixture(vec![(200, "{}"), (200, "{}"), (200, "{}")]).await;
        let client = EventClient::new(client);
        let calendar = "team/a+b@example.com";
        let payload = EventWrite {
            id: Some("supplied/id".to_string()),
            guests_can_invite_others: Some(false),
            summary: Some("Meeting".to_string()),
            attachments: Some(vec![EventAttachmentWrite {
                file_url: "https://drive.google.com/file/d/attachment".to_string(),
                ..Default::default()
            }]),
            ..Default::default()
        };
        let inserted = client
            .insert(calendar.to_string(), payload.clone())
            .await
            .unwrap();
        let mut imported_payload = payload.clone();
        imported_payload.ical_uid = Some("uid@example.com".to_string());
        let imported = client
            .import(calendar.to_string(), imported_payload)
            .await
            .unwrap();
        let updated = client
            .update(calendar.to_string(), "event/id?%".to_string(), payload)
            .await
            .unwrap();
        for event in [inserted, imported, updated] {
            assert_eq!(event.calendar_id, calendar);
        }
        let requests = observed.await.unwrap();
        for request in &requests {
            assert_eq!(
                request.headers.get("content-type").map(String::as_str),
                Some("application/json")
            );
            let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
            assert_eq!(body["id"], "supplied/id");
            assert_eq!(body["guestsCanInviteOthers"], false);
            assert_eq!(
                body["attachments"],
                serde_json::json!([{
                    "fileUrl": "https://drive.google.com/file/d/attachment"
                }])
            );
            for omitted in [
                "kind",
                "etag",
                "created",
                "updated",
                "creator",
                "organizer",
                "source",
                "reminders",
                "workingLocationProperties",
                "start",
                "end",
            ] {
                assert!(body.get(omitted).is_none(), "{omitted} was injected");
            }
        }
        assert_eq!(requests[0].method, "POST");
        assert_eq!(
            requests[0].target,
            "/calendar/v3/calendars/team%2Fa%2Bb%40example.com/events?supportsAttachments=true"
        );
        assert_eq!(requests[1].method, "POST");
        assert_eq!(requests[1].target, "/calendar/v3/calendars/team%2Fa%2Bb%40example.com/events/import?supportsAttachments=true");
        let imported_body: serde_json::Value = serde_json::from_slice(&requests[1].body).unwrap();
        assert_eq!(imported_body["iCalUID"], "uid@example.com");
        assert_eq!(requests[2].method, "PUT");
        assert_eq!(requests[2].target, "/calendar/v3/calendars/team%2Fa%2Bb%40example.com/events/event%2Fid%3F%25?supportsAttachments=true");
    }

    #[tokio::test]
    async fn quick_add_and_move_are_bodyless_and_use_valid_notifications() {
        let (client, observed) = fixture(vec![(200, "{}"); 5]).await;
        let client = EventClient::new(client);
        let calendar = "team/a@example.com";
        let added = client
            .add(calendar.to_string(), "Meeting tomorrow".to_string())
            .await
            .unwrap();
        assert_eq!(added.calendar_id, calendar);
        for send_updates in [
            None,
            Some(SendUpdates::All),
            Some(SendUpdates::ExternalOnly),
            Some(SendUpdates::None),
        ] {
            let event = Event {
                calendar_id: calendar.to_string(),
                id: "event/id".to_string(),
                ..Default::default()
            };
            let moved = client
                .move_to_calendar(event, "destination@example.com".to_string(), send_updates)
                .await
                .unwrap();
            assert_eq!(moved.calendar_id, "destination@example.com");
        }
        let requests = observed.await.unwrap();
        for request in &requests {
            assert_eq!(request.method, "POST");
            assert!(request.body.is_empty());
        }
        assert_eq!(
            requests[0].target,
            "/calendar/v3/calendars/team%2Fa%40example.com/events/quickAdd?text=Meeting+tomorrow"
        );
        for (request, expected) in
            requests[1..]
                .iter()
                .zip([None, Some("all"), Some("externalOnly"), Some("none")])
        {
            let url = url::Url::parse(&format!("http://fixture{}", request.target)).unwrap();
            assert_eq!(
                url.path(),
                "/calendar/v3/calendars/team%2Fa%40example.com/events/event%2Fid/move"
            );
            let query: std::collections::BTreeMap<_, _> = url.query_pairs().collect();
            assert_eq!(
                query.get("destination").map(|value| value.as_ref()),
                Some("destination@example.com")
            );
            assert_eq!(
                query.get("sendUpdates").map(|value| value.as_ref()),
                expected
            );
        }
    }

    #[tokio::test]
    async fn reads_retain_calendar_context() {
        let (client, observed) = fixture(vec![
            (200, r#"{"id":"one"}"#),
            (200, r#"{"items":[{"id":"two"}]}"#),
            (200, r#"{"items":[{"id":"three"}]}"#),
        ])
        .await;
        let client = EventClient::new(client);
        let calendar = "team/a@example.com";
        let event = client
            .get(calendar.to_string(), "one".to_string())
            .await
            .unwrap();
        assert_eq!(event.calendar_id, calendar);
        let instances = client.instances(event).await.unwrap();
        assert_eq!(instances.items[0].calendar_id, calendar);
        let now = chrono::Local::now();
        let listed = client
            .list(calendar.to_string(), now, now + chrono::Duration::days(1))
            .await
            .unwrap();
        assert_eq!(listed[0].calendar_id, calendar);
        let requests = observed.await.unwrap();
        assert_eq!(
            requests[0].target,
            "/calendar/v3/calendars/team%2Fa%40example.com/events/one"
        );
        assert_eq!(
            requests[1].target,
            "/calendar/v3/calendars/team%2Fa%40example.com/events/one/instances"
        );
        assert!(requests[2]
            .target
            .starts_with("/calendar/v3/calendars/team%2Fa%40example.com/events?"));
    }

    #[tokio::test]
    async fn pagination_crosses_empty_pages_and_retains_repeated_filters_and_final_token() {
        let (transport, observed) = fixture(vec![
            (200, r#"{"items":[{"id":"first"}],"nextPageToken":"page2"}"#),
            (200, r#"{"items":[],"nextPageToken":"page3"}"#),
            (200, r#"{"items":[{"id":"last"}],"nextSyncToken":"final"}"#),
        ])
        .await;
        let result = transport
            .event_client()
            .list_all(
                "primary",
                crate::EventListOptions {
                    event_types: vec!["default".into(), "focusTime".into()],
                    private_extended_property: vec!["a=1".into(), "b=2".into()],
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(
            result
                .items
                .iter()
                .map(|e| e.id.as_str())
                .collect::<Vec<_>>(),
            ["first", "last"]
        );
        assert!(result.items.iter().all(|e| e.calendar_id == "primary"));
        assert_eq!(result.next_sync_token.as_deref(), Some("final"));
        assert!(result.next_page_token.is_none());
        let requests = observed.await.unwrap();
        for (index, request) in requests.iter().enumerate() {
            let url = url::Url::parse(&format!("http://fixture{}", request.target)).unwrap();
            let pairs: Vec<_> = url.query_pairs().collect();
            let values: Vec<_> = pairs
                .iter()
                .filter(|(key, _)| key == "eventTypes")
                .map(|(_, value)| value.as_ref())
                .collect();
            assert_eq!(values, ["default", "focusTime"]);
            let expected = [None, Some("page2"), Some("page3")][index];
            assert_eq!(
                pairs
                    .iter()
                    .find(|(key, _)| key == "pageToken")
                    .map(|(_, v)| v.as_ref()),
                expected
            );
        }
    }
}
