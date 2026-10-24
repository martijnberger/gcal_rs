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
    /// Validate the final query, including raw parameters, before synchronization.
    pub(crate) fn validate(query: &QueryParams) -> crate::ClientResult<()> {
        if query.get("syncToken").is_none() {
            return Ok(());
        }
        for (key, value) in query.iter() {
            if matches!(
                key.as_str(),
                "iCalUID"
                    | "orderBy"
                    | "privateExtendedProperty"
                    | "q"
                    | "sharedExtendedProperty"
                    | "timeMin"
                    | "timeMax"
                    | "updatedMin"
            ) || (key == "showDeleted" && value != "true")
            {
                return Err(crate::ClientError::InvalidRequest(format!(
                    "{key} is incompatible with event syncToken"
                )));
            }
        }
        Ok(())
    }

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

/// Event retrieval options. Raw parameters remain available for newer API fields.
#[derive(Debug, Clone, Default)]
pub struct EventGetOptions {
    pub max_attendees: Option<u32>,
    pub time_zone: Option<String>,
    pub always_include_email: Option<bool>,
    pub extra: QueryParams,
}

impl EventGetOptions {
    pub(crate) fn query(&self) -> QueryParams {
        let mut query = self.extra.clone();
        if let Some(value) = self.max_attendees {
            query.insert("maxAttendees".into(), value.to_string());
        }
        if let Some(value) = &self.time_zone {
            query.insert("timeZone".into(), value.clone());
        }
        if let Some(value) = self.always_include_email {
            query.insert("alwaysIncludeEmail".into(), value.to_string());
        }
        query
    }
}

/// Options shared by event writes; only parameters supported by each endpoint are sent.
///
/// Writes carrying conference data automatically use version 1 and reject an explicit
/// conflicting version. Writes carrying attachments (including a clear) always enable
/// attachment support. `if_match` supplies an ETag without retrying stale writes.
#[derive(Debug, Clone, Default)]
pub struct EventWriteOptions {
    pub send_updates: Option<super::types::SendUpdates>,
    /// Deprecated by Google in favor of `send_updates`.
    pub send_notifications: Option<bool>,
    pub conference_data_version: Option<u32>,
    pub supports_attachments: Option<bool>,
    pub max_attendees: Option<u32>,
    /// Deprecated Google response option, supported by update and patch.
    pub always_include_email: Option<bool>,
    pub event_label_version: Option<u32>,
    pub if_match: Option<String>,
    pub extra: QueryParams,
}

#[derive(Clone, Copy)]
pub(crate) enum EventWriteMethod {
    Insert,
    Import,
    Update,
    Patch,
    Delete,
    Move,
    QuickAdd,
}

impl EventWriteOptions {
    pub(crate) fn query_for(&self, method: EventWriteMethod) -> QueryParams {
        let mut query = self.extra.clone();
        macro_rules! add {
            ($field:ident, $name:literal) => {
                if let Some(value) = &self.$field {
                    query.insert($name.into(), value.to_string());
                }
            };
        }
        if !matches!(method, EventWriteMethod::Import) {
            add!(send_updates, "sendUpdates");
            add!(send_notifications, "sendNotifications");
        }
        if matches!(
            method,
            EventWriteMethod::Insert
                | EventWriteMethod::Import
                | EventWriteMethod::Update
                | EventWriteMethod::Patch
        ) {
            add!(conference_data_version, "conferenceDataVersion");
            add!(supports_attachments, "supportsAttachments");
            add!(event_label_version, "eventLabelVersion");
        }
        if matches!(
            method,
            EventWriteMethod::Insert | EventWriteMethod::Update | EventWriteMethod::Patch
        ) {
            add!(max_attendees, "maxAttendees");
        }
        if matches!(method, EventWriteMethod::Update | EventWriteMethod::Patch) {
            add!(always_include_email, "alwaysIncludeEmail");
        }
        query
    }

    pub(crate) fn payload_query(
        &self,
        method: EventWriteMethod,
        has_conference_data: bool,
        has_attachments: bool,
    ) -> crate::ClientResult<QueryParams> {
        let mut query = self.query_for(method);
        if has_conference_data {
            if query
                .iter()
                .any(|(key, value)| key == "conferenceDataVersion" && value != "1")
            {
                return Err(crate::ClientError::InvalidRequest(
                    "writing conferenceData requires conferenceDataVersion=1".into(),
                ));
            }
            query.insert("conferenceDataVersion".into(), "1".into());
        }
        if has_attachments {
            query.insert("supportsAttachments".into(), "true".into());
        }
        Ok(query)
    }
}
