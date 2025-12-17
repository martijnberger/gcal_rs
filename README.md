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
use gcal::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client_id = std::env::var("GOOGLE_CLIENT_ID")?;
    let client_secret = std::env::var("GOOGLE_CLIENT_SECRET")?;

    let token = OAuth::new(client_id, client_secret, "http://localhost:8555/auth")
        .naive()
        .await?;

    let (calendar_client, event_client) = GCalClient::new(token.access)?.clients();

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

## Status

Currently working on updating documentation for each part of the code and structuring the best API.

## Author

Shadorain <shadorain7517@gmail.com>

### Original Authors

Erik Hollensbe <erik+github@hollensbe.org> : [gcal](https://github.com/erikh/gcal)
