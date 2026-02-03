use std::sync::Arc;

use super::{CalendarAccessRole, CalendarList, CalendarListItem};
use crate::{ClientResult, GCalClient};

/// `CalendarListClient` is the method of accessing the calendar list. You must provide it with a
/// Google Calendar client.
#[derive(Debug, Clone)]
pub struct CalendarListClient(Arc<GCalClient>);

impl CalendarListClient {
    /// Construct a `CalendarListClient`. Requires a Google Calendar Client.
    #[must_use]
    pub const fn new(client: Arc<GCalClient>) -> Self {
        Self(client)
    }

    /// List the calendars. Currently only returns the first page of results.
    ///
    /// # Errors
    /// Returns an error if the request fails or the response cannot be parsed.
    pub async fn list(
        &self,
        hidden: bool,
        access_role: CalendarAccessRole,
    ) -> ClientResult<Vec<CalendarListItem>> {
        // FIXME get all the results lol
        let mut cl = CalendarList::default();
        cl.add_query("minAccessRole".to_string(), access_role.to_string());
        cl.add_query("showHidden".to_string(), hidden.to_string());

        Ok(self
            .0
            .get(None, cl)
            .await?
            .json::<CalendarList>()
            .await?
            .items)
    }
}
