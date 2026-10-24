use serde::{Deserialize, Serialize};
use url::Url;

use super::{ClientResult, QueryParams, Sendable};

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(default)]
pub struct UserInfo {
    pub id: String,
    pub email: String,
    pub verified_email: bool,
    pub name: String,
    pub given_name: String,
    pub family_name: String,

    pub picture: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hd: Option<String>,

    #[serde(skip)]
    query_string: QueryParams,
}

impl Sendable for UserInfo {
    fn path(&self, _action: Option<String>) -> String {
        String::new()
    }

    fn query(&self) -> QueryParams {
        self.query_string.clone()
    }

    fn url(&self, _action: Option<String>) -> ClientResult<Url> {
        let mut url = Url::parse("https://www.googleapis.com/oauth2/v2/userinfo")?;
        if !self.query_string.is_empty() {
            url.query_pairs_mut().extend_pairs(self.query());
        }
        Ok(url)
    }
}
