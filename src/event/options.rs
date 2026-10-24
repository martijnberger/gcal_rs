use crate::QueryParams;

/// Event list filters. Repeated properties and event types retain all values.
#[derive(Debug, Clone, Default)]
pub struct EventListOptions {
    pub max_results: Option<u32>,
    pub max_attendees: Option<u32>,
    pub page_token: Option<String>,
    pub sync_token: Option<String>,
    pub time_min: Option<String>,
    pub time_max: Option<String>,
    pub updated_min: Option<String>,
    pub time_zone: Option<String>,
    pub q: Option<String>,
    pub ical_uid: Option<String>,
    pub order_by: Option<String>,
    pub single_events: Option<bool>,
    pub show_deleted: Option<bool>,
    pub show_hidden_invitations: Option<bool>,
    pub always_include_email: Option<bool>,
    pub event_types: Vec<String>,
    pub private_extended_property: Vec<String>,
    pub shared_extended_property: Vec<String>,
    pub extra: QueryParams,
}

impl EventListOptions {
    pub(crate) fn query(&self) -> QueryParams {
        let mut query = self.extra.clone();
        macro_rules! add {
            ($field:ident, $name:literal) => {
                if let Some(value) = &self.$field {
                    query.insert($name.into(), value.to_string());
                }
            };
        }
        add!(max_results, "maxResults");
        add!(max_attendees, "maxAttendees");
        add!(page_token, "pageToken");
        add!(sync_token, "syncToken");
        add!(time_min, "timeMin");
        add!(time_max, "timeMax");
        add!(updated_min, "updatedMin");
        add!(time_zone, "timeZone");
        add!(q, "q");
        add!(ical_uid, "iCalUID");
        add!(order_by, "orderBy");
        add!(single_events, "singleEvents");
        add!(show_deleted, "showDeleted");
        add!(show_hidden_invitations, "showHiddenInvitations");
        add!(always_include_email, "alwaysIncludeEmail");
        for (key, values) in [
            ("eventTypes", &self.event_types),
            ("privateExtendedProperty", &self.private_extended_property),
            ("sharedExtendedProperty", &self.shared_extended_property),
        ] {
            for value in values {
                query.append(key.into(), value.clone());
            }
        }
        query
    }
}

/// Recurring-instance page filters; the original event ID is supplied to the method.
#[derive(Debug, Clone, Default)]
pub struct EventInstancesOptions {
    pub max_results: Option<u32>,
    pub max_attendees: Option<u32>,
    pub page_token: Option<String>,
    pub time_min: Option<String>,
    pub time_max: Option<String>,
    pub time_zone: Option<String>,
    pub original_start: Option<String>,
    pub show_deleted: Option<bool>,
    pub always_include_email: Option<bool>,
    pub extra: QueryParams,
}

impl EventInstancesOptions {
    pub(crate) fn query(&self) -> QueryParams {
        let mut query = self.extra.clone();
        macro_rules! add {
            ($field:ident, $name:literal) => {
                if let Some(value) = &self.$field {
                    query.insert($name.into(), value.to_string());
                }
            };
        }
        add!(max_results, "maxResults");
        add!(max_attendees, "maxAttendees");
        add!(page_token, "pageToken");
        add!(time_min, "timeMin");
        add!(time_max, "timeMax");
        add!(time_zone, "timeZone");
        add!(original_start, "originalStart");
        add!(show_deleted, "showDeleted");
        add!(always_include_email, "alwaysIncludeEmail");
        query
    }
}
