use std::sync::Arc;

use super::options::EventWriteMethod;
use super::{
    collection_path, item_path, Event, EventGetOptions, EventInstancesOptions, EventListOptions,
    EventPatch, EventWrite, EventWriteOptions, Events, SendUpdates,
};
use crate::request::ApiRequest;
use crate::{ClientResult, GCalClient};

/// `EventClient` is the method of managing events from a specific calendar. Requires a Google
/// Calendar client.
#[derive(Debug, Clone)]
pub struct EventClient(pub(crate) Arc<GCalClient>);

fn write_request(
    path: String,
    event: EventWrite,
    options: &EventWriteOptions,
    method: EventWriteMethod,
) -> ClientResult<ApiRequest<EventWrite>> {
    let query = options.payload_query(
        method,
        event.conference_data.is_some(),
        event.attachments.is_some(),
    )?;
    let mut request = ApiRequest::with_body(path, event);
    request.query = query;
    request.if_match.clone_from(&options.if_match);
    Ok(request)
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
        self.delete_with_options(
            &event.calendar_id,
            &event.id,
            EventWriteOptions {
                extra: event.query_string,
                ..Default::default()
            },
        )
        .await
    }

    /// Delete by ID, optionally sending notifications and an `If-Match` ETag.
    /// # Errors
    /// Returns transport and API errors, including stale-ETag HTTP 412.
    pub async fn delete_with_options(
        &self,
        calendar_id: &str,
        event_id: &str,
        options: EventWriteOptions,
    ) -> ClientResult<()> {
        let mut request = ApiRequest::new(item_path(calendar_id, event_id));
        request.query = options.query_for(EventWriteMethod::Delete);
        request.if_match = options.if_match;
        self.0.delete(None, request).await?;
        Ok(())
    }

    /// Get an event by ID.
    ///
    /// # Errors
    /// Returns an error if the request fails or the response cannot be parsed.
    pub async fn get(&self, calendar_id: String, event_id: String) -> ClientResult<Event> {
        self.get_with_options(calendar_id, event_id, EventGetOptions::default())
            .await
    }

    /// Retrieve an event with attendee and time-zone response options.
    /// # Errors
    /// Returns API, transport, or decoding errors.
    pub async fn get_with_options(
        &self,
        calendar_id: String,
        event_id: String,
        options: EventGetOptions,
    ) -> ClientResult<Event> {
        let mut request = ApiRequest::new(item_path(&calendar_id, &event_id));
        request.query = options.query();
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
        self.import_with_options(calendar_id, event, EventWriteOptions::default())
            .await
    }

    /// Import with conference, attachment, and event-label API options.
    /// # Errors
    /// Returns validation, API, transport, or decoding errors.
    pub async fn import_with_options(
        &self,
        calendar_id: String,
        event: EventWrite,
        options: EventWriteOptions,
    ) -> ClientResult<Event> {
        let request = write_request(
            format!("{}/import", collection_path(&calendar_id)),
            event,
            &options,
            EventWriteMethod::Import,
        )?;
        let mut event = self.0.post(None, request).await?.json::<Event>().await?;
        event.calendar_id = calendar_id;
        Ok(event)
    }

    /// Insert a writable event into the calendar's collection.
    ///
    /// # Errors
    /// Returns an error if the request fails or the response cannot be parsed.
    pub async fn insert(&self, calendar_id: String, event: EventWrite) -> ClientResult<Event> {
        self.insert_with_options(calendar_id, event, EventWriteOptions::default())
            .await
    }

    /// Insert with explicit notification and conference options.
    /// # Errors
    /// Returns validation, API, transport, or decoding errors.
    pub async fn insert_with_options(
        &self,
        calendar_id: String,
        event: EventWrite,
        options: EventWriteOptions,
    ) -> ClientResult<Event> {
        let request = write_request(
            collection_path(&calendar_id),
            event,
            &options,
            EventWriteMethod::Insert,
        )?;
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
        EventListOptions::validate(&request.query)?;
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

    /// Collect every incremental page, retaining deleted events and the final token.
    ///
    /// Keep all other options identical to the full sync that produced `sync_token`.
    /// HTTP 410 is returned unchanged; the caller owns full-sync recovery and storage.
    /// # Errors
    /// Returns incompatible-filter validation, API, transport, or decoding errors.
    pub async fn sync(
        &self,
        calendar_id: &str,
        sync_token: impl Into<String>,
        mut options: EventListOptions,
    ) -> ClientResult<Events> {
        options.sync_token = Some(sync_token.into());
        self.list_all(calendar_id, options).await
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
        let mut extra = event.query_string;
        extra.remove("sendUpdates");
        self.move_to_calendar_with_options(
            &event.calendar_id,
            &event.id,
            destination,
            EventWriteOptions {
                send_updates,
                extra,
                ..Default::default()
            },
        )
        .await
    }

    /// Move an event with notification options and optional ETag.
    /// # Errors
    /// Returns API, transport, or decoding errors.
    pub async fn move_to_calendar_with_options(
        &self,
        calendar_id: &str,
        event_id: &str,
        destination: String,
        options: EventWriteOptions,
    ) -> ClientResult<Event> {
        let mut request = ApiRequest::new(format!("{}/move", item_path(calendar_id, event_id)));
        request.query = options.query_for(EventWriteMethod::Move);
        request
            .query
            .insert("destination".into(), destination.clone());
        request.if_match = options.if_match;
        let mut event = self.0.post(None, request).await?.json::<Event>().await?;
        event.calendar_id = destination;
        Ok(event)
    }

    /// Quick-add an event to the selected calendar using natural-language text.
    ///
    /// # Errors
    /// Returns an error if the request fails or the response cannot be parsed.
    pub async fn add(&self, calendar_id: String, text: String) -> ClientResult<Event> {
        self.add_with_options(calendar_id, text, EventWriteOptions::default())
            .await
    }

    /// Quick-add natural-language text with explicit notification options.
    /// # Errors
    /// Returns API, transport, or decoding errors.
    pub async fn add_with_options(
        &self,
        calendar_id: String,
        text: String,
        options: EventWriteOptions,
    ) -> ClientResult<Event> {
        let mut request = ApiRequest::new(format!("{}/quickAdd", collection_path(&calendar_id)));
        request.query = options.query_for(EventWriteMethod::QuickAdd);
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
        self.update_with_options(calendar_id, event_id, event, EventWriteOptions::default())
            .await
    }

    /// Replace writable properties, optionally conditional on the supplied ETag.
    /// # Errors
    /// Returns validation, API (including HTTP 412), transport, or decoding errors.
    pub async fn update_with_options(
        &self,
        calendar_id: String,
        event_id: String,
        event: EventWrite,
        options: EventWriteOptions,
    ) -> ClientResult<Event> {
        let request = write_request(
            item_path(&calendar_id, &event_id),
            event,
            &options,
            EventWriteMethod::Update,
        )?;
        let mut event = self.0.put(None, request).await?.json::<Event>().await?;
        event.calendar_id = calendar_id;
        Ok(event)
    }

    /// Apply only explicitly supplied changes to the selected event.
    /// # Errors
    /// Returns validation, API, transport, or decoding errors.
    pub async fn patch(
        &self,
        calendar_id: String,
        event_id: String,
        event: EventPatch,
    ) -> ClientResult<Event> {
        self.patch_with_options(calendar_id, event_id, event, EventWriteOptions::default())
            .await
    }

    /// Apply partial changes, optionally conditional on an `If-Match` ETag.
    ///
    /// Explicit conference clears also require conference data version 1.
    /// # Errors
    /// Returns validation, API (including HTTP 412), transport, or decoding errors.
    pub async fn patch_with_options(
        &self,
        calendar_id: String,
        event_id: String,
        event: EventPatch,
        options: EventWriteOptions,
    ) -> ClientResult<Event> {
        let query = options.payload_query(
            EventWriteMethod::Patch,
            !event.conference_data.is_unset(),
            !event.attachments.is_unset(),
        )?;
        let mut request = ApiRequest::with_body(item_path(&calendar_id, &event_id), event);
        request.query = query;
        request.if_match = options.if_match;
        let mut event = self.0.patch(None, request).await?.json::<Event>().await?;
        event.calendar_id = calendar_id;
        Ok(event)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Event, EventClient, EventGetOptions, EventListOptions, EventPatch, EventWrite,
        EventWriteOptions, SendUpdates,
    };
    use crate::event::types::{EventAttachmentWrite, EventConferenceDataWrite, EventStatus};
    use crate::test_support::fixture;
    use crate::{ClientError, Patch, QueryParams};

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

    #[tokio::test]
    async fn incremental_sync_collects_deleted_items_and_the_final_token() {
        let (transport, observed) = fixture(vec![
            (200, r#"{"items":[{"id":"changed"}],"nextPageToken":"second"}"#),
            (200, r#"{"items":[],"nextPageToken":"third"}"#),
            (200, r#"{"items":[{"id":"deleted","status":"cancelled"}],"nextSyncToken":"replacement"}"#),
        ])
        .await;
        let result = transport
            .event_client()
            .sync(
                "team/a@example.com",
                "previous",
                EventListOptions {
                    max_results: Some(10),
                    show_deleted: Some(true),
                    single_events: Some(true),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(result.items.len(), 2);
        assert_eq!(result.items[1].id, "deleted");
        assert_eq!(result.items[1].status, EventStatus::Cancelled);
        assert!(result
            .items
            .iter()
            .all(|event| event.calendar_id == "team/a@example.com"));
        assert_eq!(result.next_sync_token.as_deref(), Some("replacement"));
        assert!(result.next_page_token.is_none());
        let requests = observed.await.unwrap();
        for (index, request) in requests.iter().enumerate() {
            let url = url::Url::parse(&format!("http://fixture{}", request.target)).unwrap();
            let query: std::collections::BTreeMap<_, _> = url.query_pairs().collect();
            assert_eq!(query.get("syncToken").unwrap(), "previous");
            assert_eq!(query.get("maxResults").unwrap(), "10");
            assert_eq!(query.get("showDeleted").unwrap(), "true");
            assert_eq!(
                query.get("pageToken").map(|value| value.as_ref()),
                [None, Some("second"), Some("third")][index]
            );
        }
    }

    #[tokio::test]
    async fn sync_rejects_typed_and_raw_incompatible_filters_before_network() {
        let (transport, observed) = fixture(vec![(200, "{}")]).await;
        let client = transport.event_client();
        for options in [
            EventListOptions {
                time_min: Some("2026-01-01T00:00:00Z".into()),
                ..Default::default()
            },
            EventListOptions {
                time_max: Some("2027-01-01T00:00:00Z".into()),
                ..Default::default()
            },
            EventListOptions {
                updated_min: Some("2026-01-01T00:00:00Z".into()),
                ..Default::default()
            },
            EventListOptions {
                q: Some("meeting".into()),
                ..Default::default()
            },
            EventListOptions {
                ical_uid: Some("uid".into()),
                ..Default::default()
            },
            EventListOptions {
                order_by: Some("updated".into()),
                ..Default::default()
            },
            EventListOptions {
                private_extended_property: vec!["a=b".into()],
                ..Default::default()
            },
            EventListOptions {
                shared_extended_property: vec!["a=b".into()],
                ..Default::default()
            },
            EventListOptions {
                show_deleted: Some(false),
                ..Default::default()
            },
        ] {
            assert!(matches!(
                client.sync("primary", "old", options).await,
                Err(ClientError::InvalidRequest(_))
            ));
        }
        for (key, value) in [
            ("timeMin", "2026-01-01T00:00:00Z"),
            ("timeMax", "2027-01-01T00:00:00Z"),
            ("updatedMin", "2026-01-01T00:00:00Z"),
            ("q", "meeting"),
            ("iCalUID", "uid"),
            ("orderBy", "updated"),
            ("privateExtendedProperty", "a=b"),
            ("sharedExtendedProperty", "a=b"),
            ("showDeleted", "false"),
        ] {
            let mut extra = QueryParams::new();
            extra.append("syncToken".into(), "raw-token".into());
            extra.append(key.into(), value.into());
            let options = EventListOptions {
                extra,
                ..Default::default()
            };
            assert!(matches!(
                client.list_page("primary", &options).await,
                Err(ClientError::InvalidRequest(_))
            ));
        }
        client
            .list_page("primary", &EventListOptions::default())
            .await
            .unwrap();
        assert_eq!(observed.await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn expired_sync_tokens_are_returned_without_retry() {
        let (transport, observed) = fixture(vec![
            (410, r#"{"error":{"message":"Sync token is no longer valid","errors":[{"reason":"fullSyncRequired"}]}}"#),
        ]).await;
        let error = transport
            .event_client()
            .sync("primary", "expired", EventListOptions::default())
            .await
            .unwrap_err();
        assert!(error.requires_full_sync());
        assert_eq!(error.status(), Some(reqwest::StatusCode::GONE));
        assert_eq!(observed.await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn patch_preserves_omission_null_false_and_empty_and_enables_capabilities() {
        let (transport, observed) = fixture(vec![(200, r#"{"id":"event"}"#)]).await;
        let result = transport
            .event_client()
            .patch_with_options(
                "team/a".into(),
                "event/id".into(),
                EventPatch {
                    description: Patch::Null,
                    summary: Patch::Value(String::new()),
                    guests_can_invite_others: Patch::Value(false),
                    attendees: Patch::Value(Vec::new()),
                    attachments: Patch::Value(Vec::new()),
                    conference_data: Patch::Null,
                    ..Default::default()
                },
                EventWriteOptions {
                    send_updates: Some(SendUpdates::All),
                    supports_attachments: Some(false),
                    if_match: Some("\"current\"".into()),
                    event_label_version: Some(1),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(result.calendar_id, "team/a");
        let requests = observed.await.unwrap();
        assert_eq!(requests[0].method, "PATCH");
        assert_eq!(requests[0].headers.get("if-match").unwrap(), "\"current\"");
        let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
        assert_eq!(
            body,
            serde_json::json!({
                "description": null, "summary": "", "guestsCanInviteOthers": false,
                "attendees": [], "attachments": [], "conferenceData": null
            })
        );
        let url = url::Url::parse(&format!("http://fixture{}", requests[0].target)).unwrap();
        assert_eq!(
            url.path(),
            "/calendar/v3/calendars/team%2Fa/events/event%2Fid"
        );
        let query: std::collections::BTreeMap<_, _> = url.query_pairs().collect();
        assert_eq!(query.get("supportsAttachments").unwrap(), "true");
        assert_eq!(query.get("conferenceDataVersion").unwrap(), "1");
        assert_eq!(query.get("sendUpdates").unwrap(), "all");
        assert_eq!(query.get("eventLabelVersion").unwrap(), "1");
    }

    #[tokio::test]
    async fn conditional_patch_update_and_delete_preserve_http_412() {
        let (transport, observed) =
            fixture(vec![(412, r#"{"error":{"message":"stale etag"}}"#); 3]).await;
        let client = transport.event_client();
        let options = EventWriteOptions {
            if_match: Some("\"stale\"".into()),
            send_updates: Some(SendUpdates::ExternalOnly),
            ..Default::default()
        };
        let errors = [
            client
                .patch_with_options(
                    "primary".into(),
                    "event".into(),
                    EventPatch::default(),
                    options.clone(),
                )
                .await
                .unwrap_err(),
            client
                .update_with_options(
                    "primary".into(),
                    "event".into(),
                    EventWrite::default(),
                    options.clone(),
                )
                .await
                .unwrap_err(),
            client
                .delete_with_options("primary", "event", options)
                .await
                .unwrap_err(),
        ];
        assert!(errors
            .iter()
            .all(|error| error.status() == Some(reqwest::StatusCode::PRECONDITION_FAILED)));
        let requests = observed.await.unwrap();
        assert_eq!(
            requests
                .iter()
                .map(|request| request.method.as_str())
                .collect::<Vec<_>>(),
            ["PATCH", "PUT", "DELETE"]
        );
        for request in &requests {
            assert_eq!(request.headers.get("if-match").unwrap(), "\"stale\"");
            assert!(request.target.contains("sendUpdates=externalOnly"));
        }
        assert!(requests[2].body.is_empty());
    }

    #[tokio::test]
    async fn conference_version_conflicts_fail_before_network_and_defaults_use_version_one() {
        let (transport, observed) = fixture(vec![(200, "{}"); 2]).await;
        let client = transport.event_client();
        let write = EventWrite {
            conference_data: Some(EventConferenceDataWrite::default()),
            ..Default::default()
        };
        let mut extra = QueryParams::new();
        extra.append("conferenceDataVersion".into(), "0".into());
        for options in [
            EventWriteOptions {
                conference_data_version: Some(0),
                ..Default::default()
            },
            EventWriteOptions {
                extra,
                ..Default::default()
            },
        ] {
            assert!(matches!(
                client
                    .insert_with_options("primary".into(), write.clone(), options.clone())
                    .await,
                Err(ClientError::InvalidRequest(_))
            ));
            assert!(matches!(
                client
                    .patch_with_options(
                        "primary".into(),
                        "event".into(),
                        EventPatch {
                            conference_data: Patch::Null,
                            ..Default::default()
                        },
                        options,
                    )
                    .await,
                Err(ClientError::InvalidRequest(_))
            ));
        }
        client
            .insert("primary".into(), write.clone())
            .await
            .unwrap();
        client
            .update("primary".into(), "event".into(), write)
            .await
            .unwrap();
        let requests = observed.await.unwrap();
        assert_eq!(requests.len(), 2);
        for request in requests {
            assert!(request.target.ends_with("?conferenceDataVersion=1"));
        }
    }

    #[tokio::test]
    async fn endpoint_options_only_send_supported_typed_parameters() {
        let (transport, observed) = fixture(vec![(200, "{}"); 6]).await;
        let client = transport.event_client();
        let options = EventWriteOptions {
            send_updates: Some(SendUpdates::None),
            send_notifications: Some(false),
            conference_data_version: Some(1),
            supports_attachments: Some(true),
            max_attendees: Some(7),
            always_include_email: Some(false),
            event_label_version: Some(1),
            ..Default::default()
        };
        client
            .import_with_options("primary".into(), EventWrite::default(), options.clone())
            .await
            .unwrap();
        client
            .add_with_options("primary".into(), "Lunch".into(), options.clone())
            .await
            .unwrap();
        client
            .move_to_calendar_with_options(
                "primary",
                "event",
                "destination".into(),
                options.clone(),
            )
            .await
            .unwrap();
        client
            .delete_with_options("primary", "event", options.clone())
            .await
            .unwrap();
        client
            .update_with_options(
                "primary".into(),
                "event".into(),
                EventWrite::default(),
                options,
            )
            .await
            .unwrap();
        client
            .get_with_options(
                "primary".into(),
                "event".into(),
                EventGetOptions {
                    max_attendees: Some(3),
                    time_zone: Some("Europe/Copenhagen".into()),
                    always_include_email: Some(false),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        let requests = observed.await.unwrap();
        for (index, request) in requests.iter().enumerate() {
            let url = url::Url::parse(&format!("http://fixture{}", request.target)).unwrap();
            let query: std::collections::BTreeMap<_, _> = url.query_pairs().collect();
            if index == 0 {
                assert_eq!(query.len(), 3);
                for key in [
                    "conferenceDataVersion",
                    "supportsAttachments",
                    "eventLabelVersion",
                ] {
                    assert_eq!(
                        query.get(key).unwrap(),
                        if key == "supportsAttachments" {
                            "true"
                        } else {
                            "1"
                        }
                    );
                }
            } else if index < 4 {
                assert_eq!(query.get("sendUpdates").unwrap(), "none");
                assert_eq!(query.get("sendNotifications").unwrap(), "false");
                assert_eq!(query.len(), if index == 1 || index == 2 { 3 } else { 2 });
            } else if index == 4 {
                assert_eq!(query.get("maxAttendees").unwrap(), "7");
                assert_eq!(query.get("alwaysIncludeEmail").unwrap(), "false");
                assert_eq!(query.len(), 7);
            } else {
                assert_eq!(request.method, "GET");
                assert!(request.body.is_empty());
                assert_eq!(query.get("maxAttendees").unwrap(), "3");
                assert_eq!(query.get("timeZone").unwrap(), "Europe/Copenhagen");
                assert_eq!(query.get("alwaysIncludeEmail").unwrap(), "false");
                assert_eq!(query.len(), 3);
            }
        }
    }
}
