use std::collections::BTreeMap;
use url::Url;

use super::ClientResult;

const BASE_URL: &str = "https://www.googleapis.com/calendar/v3";

pub type QueryParams = BTreeMap<String, String>;
pub type AdditionalProperties = BTreeMap<String, String>;

/// Sendable is the trait you must implement to interact with the Client. This object is received
/// by the client and is used to construct the request URL as well as manage the (de)serialization
/// of the object.
pub trait Sendable
where
    Self: serde::Serialize,
{
    fn path(&self, action: Option<String>) -> String;

    fn query(&self) -> BTreeMap<String, String>;

    /// Build the full request URL.
    ///
    /// # Errors
    /// Returns an error if the generated URL is invalid.
    fn url(&self, action: Option<String>) -> ClientResult<Url> {
        Ok(Url::parse_with_params(
            &format!("{}/{}", BASE_URL, self.path(action)),
            self.query(),
        )?)
    }

    /// Serialize the request body.
    ///
    /// # Errors
    /// Returns an error if the payload cannot be serialized to JSON.
    fn body_bytes(&self) -> ClientResult<Vec<u8>> {
        Ok(serde_json::to_vec(self)?)
    }
}
