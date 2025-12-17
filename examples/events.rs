//! This example showcases the Google OAuth2 process for requesting access to the Google Calendar features
//! and the user's events.
//!
//! Before running it, you'll need to generate your own Google OAuth2 credentials.
//!
//! In order to run the example call:
//!
//! ```sh
//! GOOGLE_CLIENT_ID=xxx GOOGLE_CLIENT_SECRET=yyy cargo run --example events
//! ```
//!
//! ...and follow the instructions.
use chrono::{Duration, Local};
use gcal_rs::*;

#[tokio::main]
async fn main() {
    let client_id = std::env::var("GOOGLE_CLIENT_ID")
        .expect("[ERR] Missing the GOOGLE_CLIENT_ID environment variable.");
    let client_secret = std::env::var("GOOGLE_CLIENT_SECRET")
        .expect("[ERR] Missing the GOOGLE_CLIENT_SECRET environment variable.");

    let token = OAuth::new(client_id, client_secret, "http://127.0.0.1:8555/auth")
        .naive()
        .await
        .expect("[ERR] Failed to get access key.");
    println!("Refresh: {:?}", token.refresh);

    // # Mini example showing how to refresh the access token.
    //
    // println!("Ref: {}", token.refresh.unwrap());
    // let token = OAuth::new(client_id, client_secret, "http://localhost:8555".to_string())
    //     .exhange_refresh("REF TOKEN HERE".to_string())
    //     .await
    //     .unwrap();

    let (calendar_client, event_client) = GCalClient::new(token, None).unwrap().clients();

    let list = calendar_client
        .list(true, CalendarAccessRole::Reader)
        .await
        .unwrap();

    let start = Local::now();
    let end = Local::now().checked_add_signed(Duration::days(7)).unwrap();

    let mut event_list = Vec::new();
    for calendar in list {
        event_list.extend(
            event_client
                .list(calendar.id.clone(), start, end)
                .await
                .unwrap(),
        );
    }

    println!("Events: ");
    for event in &event_list {
        println!("  - {} : {}", event.summary, event.calendar_id);
    }
}
