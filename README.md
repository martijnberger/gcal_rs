</div>

<div align="center">

<br>

# 🗓️ gcal_rs: Google Calendar API 🗓️

> A blazingly fast, hand written Google calendar API in Rust.

<a href="https://docs.rs/gcal_rs/latest/gcal_rs/"> ![Docs](https://img.shields.io/docsrs/gcal_rs?color=37d4a7&logo=rust&style=for-the-badge)</a>
<a href="https://crates.io/crates/gcal_rs"> ![Crate](https://img.shields.io/crates/v/gcal_rs?color=ff4971&style=for-the-badge)</a>
<a href="/LICENSE"> ![License](https://img.shields.io/badge/license-GPL%20v3-blueviolet?style=for-the-badge)</a>
<a href="#development"> ![Status](https://img.shields.io/badge/status-WIP-informational?style=for-the-badge&color=ff69b4) </a>

[Usage](#usage)
•
[Notes](#notes)
•
[Examples](#examples)
•
[Development](#development)
<br>
[Docs](https://docs.rs/shadocal/latest/shadocal/)

</div>

## Summary

This is intended to be a solution to the current state of Google Calendar API's for Rust out there currently.
There are a few out there but either are for specific projects usecases or just are horribly generated.

I'm not saying this is perfect but it attempts to be better and have some solidity.

## Example

```rust
use chrono::{Duration, Local};
use gcal_rs::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client_id = std::env::var("GOOGLE_CLIENT_ID")?;
    let client_secret = std::env::var("GOOGLE_CLIENT_SECRET")?;

    let token = OAuth::new(&client_id, &client_secret, "http://127.0.0.1:8555/auth")
        .naive()
        .await?;

    let (calendar_client, event_client) = GCalClient::new(token, None)?.clients();

    let list = calendar_client
        .list(true, CalendarAccessRole::Reader)
        .await?;

    let start = Local::now();
    let end = Local::now().checked_add_signed(Duration::days(7)).unwrap();

    let mut event_list = Vec::new();
    for calendar in list {
        event_list.extend(
            event_client
                .list(calendar.id.clone(), start, end)
                .await?,
        );
    }

    println!("Events: ");
    for event in &event_list {
        println!("  - {} : {}", event.summary, event.calendar_id);
    }
}
```

## Typed writes and paginated reads

Use `EventWrite`, not the response `Event`, for creation and full updates.
`None` omits a property; `Some(false)` and empty collections remain explicit.
`insert(calendar_id, payload)`, `import(calendar_id, payload)`, and
`update(calendar_id, event_id, payload)` select the target separately from the body.
`add(calendar_id, text)` performs a bodyless quick-add.

```rust
use gcal_rs::{EventListOptions, EventWrite};

let payload = EventWrite {
    summary: Some("Planning".into()),
    guests_can_invite_others: Some(false),
    ..Default::default()
};
let filters = EventListOptions {
    event_types: vec!["default".into(), "focusTime".into()],
    ..Default::default()
};
```

`EventClient::list_page` exposes one page; `list_all` collects every page and
retains the final synchronization token. The time-window `list` convenience method
also collects all pages. Recurring instances have equivalent `instances_page` and
`instances_all` methods. Calendar-list and settings clients provide `list_page`
and `list_all` with `CalendarListOptions` and `SettingsListOptions`.

Use `QueryParams::append` for repeated raw filters in an options struct's `extra`
field. `insert` replaces a single-valued key. Response enums retain future values
as `Unknown(String)`; sparse nested fields are optional. Calendar access roles are
optional on deleted entries, and recurrence lines retain their original order.

## Synchronization and conditional patches

Start with `list_all` and persist its `next_sync_token`. Subsequent
`EventClient::sync(calendar_id, token, options)`, `CalendarListClient::sync`, and
`SettingsClient::sync` collect the delta, including every continuation page.
Keep other filters consistent; incompatible event/calendar sync filters are rejected
before transport. Apply deletions to your store and persist the final token only
after applying the complete delta.

If `ClientError::requires_full_sync()` is true, Google returned HTTP 410. Rebuild
your application's snapshot with a fresh full read; the library never erases storage
or silently retries an expired token.

Use `EventPatch` with `Patch::Unset` (omit), `Patch::Null` (clear), and
`Patch::Value(value)` (set). Arrays replace the whole array.
`patch_with_options`, `update_with_options`, and `delete_with_options` accept
`EventWriteOptions::if_match` for ETag concurrency checks; HTTP 412 is returned
without retrying a stale write.

`EventWriteOptions` also controls notifications, attachment support, conference
version, and label version. Conference writes automatically use version 1 and reject
an explicitly conflicting version; writing or clearing attachments enables support.
Default-options convenience methods remain available alongside `*_with_options`.

## Resource coverage

The typed clients cover all 38 Calendar v3 discovery methods (revision 20261021).
`calendar_client()` retains its existing meaning: the user's **calendar list**.
Use `calendars_client()` for calendar metadata and calendar lifecycle operations.

| Factory | Operations |
| --- | --- |
| `calendars_client()` | get, insert, update, patch, delete, clear, transfer ownership |
| `calendar_client()` | get, insert, update, patch, delete, list/page/sync, watch |
| `event_client()` | get, insert, import, update, patch, delete, list/page/sync, instances, move, quick-add, watch |
| `acl_client()` | get, insert, update, patch, delete, list/page/sync, watch |
| `settings_client()` | get, list/page/sync, watch |
| `freebusy_client()` | query, including per-calendar/group partial errors |
| `colors_client()` | get calendar and event palettes |
| `channels_client()` | stop a notification channel |
| `batch_client()` | multipart operations with correlated per-item results |

Use `CalendarWrite`/`CalendarPatch`, `CalendarListEntryWrite`/`CalendarListEntryPatch`,
and `AclRuleWrite`/`AclRulePatch` rather than serializing response resources.
Patches use the same `Patch` omission/null/value semantics as events; nested ACL
scope patches preserve omitted scope properties. Conditional resource options carry
`if_match`. Calendar deletion, clearing, and ownership transfer are destructive;
they are distinct explicit calls, never side effects of removing a calendar-list entry.

`examples/resources.rs` demonstrates read-only calendar, availability, colors, optional
ACL, and batch reads:

```sh
GOOGLE_ACCESS_TOKEN=... cargo run --example resources
# Optional: GOOGLE_CALENDAR_ID, GOOGLE_EVENT_ID, GOOGLE_READ_ACL=1.
```

Provide an access token with the scopes required by the selected operations.
Inspect `FreeBusyResponse` calendar/group `errors` before treating an empty busy
list as availability. HTTP success does not mean every lookup succeeded.

## Notifications

Create a `ChannelRequest::new(unique_id, https_webhook_url)`, set an unguessable
`token`, then call the resource client's `watch` with its list options. The returned
`Channel` supplies `id`, `resource_id`, and `expiration`; persist those for lifecycle
management. Stop it with `channels_client().stop(ChannelStop::from(&channel))`.
The `ChannelRequest` rustdoc includes a compiling subscribe/stop example.

The library does not host or authenticate a webhook. Validate inbound
`X-Goog-Channel-ID`, `X-Goog-Channel-Token`, and `X-Goog-Resource-ID` against stored
subscriptions, handle initial synchronization notifications and registration races,
and re-fetch changes through the API. Notifications are invalidation signals, not
event payloads; tolerate duplicates and ordering differences. Renew before expiry
with a new unique channel ID and stop obsolete subscriptions. Debug output redacts
channel tokens, but serialized requests contain them.

## Batch requests

```rust
use std::sync::Arc;
use gcal_rs::{
    BatchOperation, BatchRequest, ClientResult, Event, GCalClient, QueryParams,
};

async fn read_event(client: Arc<GCalClient>) -> ClientResult<Event> {
    let mut batch = BatchRequest::new();
    batch.push(
        BatchOperation::event_get("primary", "event-id", QueryParams::new())?
            .with_id("read-event")?,
    )?;
    let response = client.batch_client().send(batch).await?;
    response.get("read-event")
        .expect("the parser verifies every requested ID")
        .json()
}
```

Safe event constructors share ordinary write DTOs, conference/attachment semantics,
and ETag options. `BatchOperation::raw` is an explicit escape hatch for other Calendar
operations, not a replacement for typed clients. Requests contain 1–1000 operations.
Multipart responses preserve each item's status, headers, body, and structured error;
use `BatchItem::json` or `result` to surface per-item failures. An outer successful
response can contain failed items. Results are correlated by ID and returned in
request order, but server execution order is not guaranteed. Batches are not atomic;
do not combine operations that depend on each other's successful completion.

## Security and dependency maintenance

- API and OAuth requests require HTTPS and do not follow redirects. Callers receive
  redirect responses rather than having credentials or request bodies forwarded.
- `OAuth::auth` validates CSRF state before exchanging a code. Each `OAuth` instance
  supports one pending authorization flow; calling `auth_url` again invalidates the
  previous URL. Matching callbacks consume their state and PKCE verifier once.
- `OAuth::naive` is a minimal desktop helper listening on `127.0.0.1:8555`. Use
  `http://127.0.0.1:8555/auth` as its redirect URI. Applications hosting their own
  callback handler should pass the parsed response to `OAuth::auth`.
- Token debug output is redacted, but token fields and serialized JSON still contain
  credentials. Do not log serialized tokens; protect persisted tokens with restrictive
  storage permissions.

The OAuth dependency uses a revision-pinned fork over HTTPS for reqwest 0.13 support.
Published oauth2 5.0 uses reqwest 0.12; switching to it would introduce a second
HTTP/TLS dependency stack. Review the fork when upgrading OAuth.

To review updates and advisories:

```sh
cargo outdated --root-deps-only
cargo update
cargo audit --deny warnings
cargo test --locked --all-targets
cargo clippy --locked --all-targets --all-features -- -D warnings
```

`cargo-outdated` and `cargo-audit` are separate Cargo tools. Commit `Cargo.lock`
updates alongside manifest changes. Downstream library consumers resolve their own
lockfiles and should run their own advisory checks.

## Status

Currently working on updating documentation for each part of the code and structuring the best API.

## Author

Shadorain <shadorain7517@gmail.com>

### Original Authors

Erik Hollensbe <erik+github@hollensbe.org> : [gcal](https://github.com/erikh/gcal)
