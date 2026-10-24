//! Read-only resource/batch example. Set GOOGLE_ACCESS_TOKEN; optionally set
//! GOOGLE_CALENDAR_ID (default primary), GOOGLE_EVENT_ID, and GOOGLE_READ_ACL=1.
//! The token needs the scopes required by each requested read operation.
use chrono::{Duration, Utc};
use gcal_rs::{
    AclListOptions, BatchOperation, BatchRequest, CalendarOptions, FreeBusyRequest,
    FreeBusyRequestItem, GCalClient, OToken, QueryParams,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let token = std::env::var("GOOGLE_ACCESS_TOKEN")?;
    let calendar_id = std::env::var("GOOGLE_CALENDAR_ID").unwrap_or_else(|_| "primary".into());
    let client = GCalClient::new(OToken::new(&token, None, None), None)?;
    let calendar = client
        .clone()
        .calendars_client()
        .get(&calendar_id, CalendarOptions::default())
        .await?;
    println!("Calendar: {}", calendar.summary);
    let start = Utc::now();
    let result = client
        .clone()
        .freebusy_client()
        .query(FreeBusyRequest::new(
            start.to_rfc3339(),
            (start + Duration::hours(24)).to_rfc3339(),
            vec![FreeBusyRequestItem {
                id: calendar_id.clone(),
            }],
        ))
        .await?;
    for (id, calendar) in result.calendars {
        // An empty busy list is not proof of availability if lookup failed.
        if calendar.errors.is_empty() {
            println!("{id}: {:?}", calendar.busy);
        } else {
            eprintln!("{id}: lookup errors {:?}", calendar.errors);
        }
    }
    let colors = client.clone().colors_client().get().await?;
    println!("Calendar colors: {:?}", colors.calendar);
    if std::env::var("GOOGLE_READ_ACL").as_deref() == Ok("1") {
        let acl = client
            .clone()
            .acl_client()
            .list_all(&calendar_id, AclListOptions::default())
            .await?;
        println!("ACL: {:?}", acl.items);
    }
    if let Ok(event_id) = std::env::var("GOOGLE_EVENT_ID") {
        let mut batch = BatchRequest::new();
        batch.push(
            BatchOperation::event_get(&calendar_id, &event_id, QueryParams::new())?
                .with_id("event-read")?,
        )?;
        let response = client.batch_client().send(batch).await?;
        let item = response
            .get("event-read")
            .expect("batch parser verifies every requested ID");
        let event: gcal_rs::Event = item.json()?;
        println!("Batch event: {}", event.summary);
    }
    Ok(())
}
