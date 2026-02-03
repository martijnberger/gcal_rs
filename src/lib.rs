//! `gcal_rs`: Another Google Calendar API library for rust-lang
//!
//! I wrote this by hand because I found other clients hard to use for my use-cases. This provides an API layer into the Google Calendar API that is very minimal but also mostly complete. Types are fully represented.
//!
//! ## Example
//!
//! ```ignore
//! use gcal_rs::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), anyhow::Error> {
//!     let access_key = std::env::args().nth(1).expect("Provide an access key");
//!     let now = chrono::Local::now();
//!     let client = Client::new(access_key);
//!     let client = EventClient::new(client);
//!     let list = client.list(now - chrono::Duration::days(1), now).await?;
//!
//!     for event in &list {
//!         eprintln!("{} {}", event.id, event.summary);
//!     }
//! }
//! ```

/// Core client, used to construct other clients.
mod client;
pub use client::*;

pub mod oauth;
pub use oauth::*;

/// Calendar types.
mod calendar;
pub use calendar::*;

/// Events, the method you will work with most; events in a single calendar.
mod event;
pub use event::*;

/// User, includes information about the user/account.
mod user;
pub use user::*;

/// Settings, user preferences like timezone, locale, etc.
mod settings;
pub use settings::*;

/// Sendable trait for constructing your own queries to Google Calendar through the client.
mod sendable;
pub use sendable::*;

mod error;
pub use error::{ClientError, ClientResult};
