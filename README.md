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
