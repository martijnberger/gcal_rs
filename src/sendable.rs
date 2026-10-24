use std::collections::BTreeMap;
use url::Url;

use super::ClientResult;

const BASE_URL: &str = "https://www.googleapis.com/calendar/v3";

/// Ordered query pairs. Use `append` for Google's repeatable filters.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QueryParams(Vec<(String, String)>);

impl QueryParams {
    #[must_use]
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    pub fn insert(&mut self, key: String, value: String) -> Option<String> {
        let previous = self.remove(&key);
        self.0.push((key, value));
        previous
    }

    pub fn append(&mut self, key: String, value: String) {
        self.0.push((key, value));
    }

    pub fn remove(&mut self, key: &str) -> Option<String> {
        let mut previous = None;
        self.0.retain_mut(|(existing, value)| {
            if existing == key {
                previous = Some(std::mem::take(value));
                false
            } else {
                true
            }
        });
        previous
    }

    #[must_use]
    pub fn get(&self, key: &str) -> Option<&String> {
        self.0
            .iter()
            .find(|(existing, _)| existing == key)
            .map(|(_, value)| value)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, (String, String)> {
        self.0.iter()
    }
}

impl IntoIterator for QueryParams {
    type Item = (String, String);
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl Extend<(String, String)> for QueryParams {
    fn extend<T: IntoIterator<Item = (String, String)>>(&mut self, pairs: T) {
        self.0.extend(pairs);
    }
}

impl FromIterator<(String, String)> for QueryParams {
    fn from_iter<T: IntoIterator<Item = (String, String)>>(pairs: T) -> Self {
        Self(pairs.into_iter().collect())
    }
}
pub type AdditionalProperties = BTreeMap<String, String>;

/// Sendable is the trait you must implement to interact with the Client. This object is received
/// by the client and is used to construct the request URL as well as manage the (de)serialization
/// of the object.
pub trait Sendable
where
    Self: serde::Serialize,
{
    fn path(&self, action: Option<String>) -> String;

    fn query(&self) -> QueryParams;

    /// Whether a mutation carries a JSON body. Bodyless POST actions override this.
    fn has_body(&self) -> bool {
        true
    }

    /// Additional request headers, such as an optimistic concurrency precondition.
    ///
    /// # Errors
    /// Returns an error if a header cannot be encoded.
    fn headers(&self) -> ClientResult<reqwest::header::HeaderMap> {
        Ok(reqwest::header::HeaderMap::new())
    }

    /// Build the full request URL.
    ///
    /// # Errors
    /// Returns an error if the generated URL is invalid.
    fn url(&self, action: Option<String>) -> ClientResult<Url> {
        let mut url = Url::parse(&format!("{}/{}", BASE_URL, self.path(action)))?;
        let query = self.query();
        if !query.is_empty() {
            url.query_pairs_mut().extend_pairs(query);
        }
        Ok(url)
    }

    /// Serialize the request body.
    ///
    /// # Errors
    /// Returns an error if the payload cannot be serialized to JSON.
    fn body_bytes(&self) -> ClientResult<Vec<u8>> {
        Ok(serde_json::to_vec(self)?)
    }
}
