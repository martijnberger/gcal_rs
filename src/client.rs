use std::sync::Arc;

use reqwest::{ClientBuilder, Method, RequestBuilder, Response};
use tokio::sync::RwLock;

use super::{
    ApiError, CalendarListClient, ClientError, ClientResult, EventClient, OAuth, OToken, Sendable,
};

/// Authenticated Google Calendar transport. Requests never follow redirects.
#[derive(Debug, Clone)]
pub struct GCalClient {
    client: reqwest::Client,
    token: Arc<RwLock<OToken>>,
    oauth: Option<Arc<OAuth>>,
    debug: bool,
    #[cfg(test)]
    base_url: Option<url::Url>,
}

impl GCalClient {
    /// Create an HTTPS-only client with an existing token.
    ///
    /// # Errors
    /// Returns an error if the HTTP client cannot be constructed.
    pub fn new(token: OToken, oauth: Option<Arc<OAuth>>) -> ClientResult<Arc<Self>> {
        let client = ClientBuilder::new()
            .gzip(true)
            .https_only(true)
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        Ok(Arc::new(Self {
            client,
            token: Arc::new(token.into()),
            oauth,
            debug: false,
            #[cfg(test)]
            base_url: None,
        }))
    }

    #[must_use]
    pub const fn calendar_client(self: Arc<Self>) -> CalendarListClient {
        CalendarListClient::new(self)
    }
    #[must_use]
    pub const fn event_client(self: Arc<Self>) -> EventClient {
        EventClient::new(self)
    }
    #[must_use]
    pub fn clients(self: Arc<Self>) -> (CalendarListClient, EventClient) {
        (
            CalendarListClient::new(self.clone()),
            EventClient::new(self),
        )
    }

    /// Log method and path only, excluding tokens, query values, and event bodies.
    pub const fn set_debug(&mut self) {
        self.debug = true;
    }

    /// Send a GET request.
    /// # Errors
    /// Returns transport, API, or URL errors.
    pub async fn get(
        &self,
        action: Option<String>,
        target: impl Sendable,
    ) -> ClientResult<Response> {
        self.request(Method::GET, action, target).await
    }
    /// Send a JSON POST or bodyless action.
    /// # Errors
    /// Returns transport, API, URL, or serialization errors.
    pub async fn post(
        &self,
        action: Option<String>,
        target: impl Sendable,
    ) -> ClientResult<Response> {
        self.request(Method::POST, action, target).await
    }
    /// Send a JSON PUT request.
    /// # Errors
    /// Returns transport, API, URL, or serialization errors.
    pub async fn put(
        &self,
        action: Option<String>,
        target: impl Sendable,
    ) -> ClientResult<Response> {
        self.request(Method::PUT, action, target).await
    }
    /// Send a JSON PATCH request.
    /// # Errors
    /// Returns transport, API, URL, or serialization errors.
    pub async fn patch(
        &self,
        action: Option<String>,
        target: impl Sendable,
    ) -> ClientResult<Response> {
        self.request(Method::PATCH, action, target).await
    }
    /// Send a DELETE request, accepting successful empty responses.
    /// # Errors
    /// Returns transport, API, or URL errors.
    pub async fn delete(
        &self,
        action: Option<String>,
        target: impl Sendable,
    ) -> ClientResult<Response> {
        self.request(Method::DELETE, action, target).await
    }

    async fn request(
        &self,
        method: Method,
        action: Option<String>,
        target: impl Sendable,
    ) -> ClientResult<Response> {
        let url = self.get_url(&target, action)?;
        if self.debug {
            eprintln!("[{}] {}", method, url.path());
        }
        let carries_body =
            matches!(method, Method::POST | Method::PUT | Method::PATCH) && target.has_body();
        let mut request = self.client.request(method, url).headers(target.headers()?);
        if carries_body {
            request = request
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(target.body_bytes()?);
        }
        self.send(request).await
    }

    pub(crate) async fn send(&self, req: RequestBuilder) -> ClientResult<Response> {
        if let Some(oauth) = &self.oauth {
            oauth.refresh(&mut *(self.token.write().await)).await?;
        }
        let response = req
            .bearer_auth(&self.token.read().await.access)
            .send()
            .await?;
        Self::check_response(response).await
    }

    pub(crate) async fn check_response(response: Response) -> ClientResult<Response> {
        let status = response.status();
        if status.is_success() {
            return Ok(response);
        }
        Err(ClientError::Api(Box::new(ApiError::from_response(
            status,
            &response.bytes().await?,
        ))))
    }

    fn get_url(&self, target: &impl Sendable, action: Option<String>) -> ClientResult<url::Url> {
        let url = target.url(action)?;
        #[cfg(test)]
        if let Some(base) = &self.base_url {
            let mut local = base.join(url.path().trim_start_matches("/calendar/v3/"))?;
            local.set_query(url.query());
            return Ok(local);
        }
        Ok(url)
    }

    #[cfg(test)]
    pub(crate) fn for_test(base_url: url::Url) -> Arc<Self> {
        Arc::new(Self {
            client: ClientBuilder::new()
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .unwrap(),
            token: Arc::new(OToken::new("test-token", None, None).into()),
            oauth: None,
            debug: false,
            base_url: Some(base_url),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{request::ApiRequest, test_support::fixture, ClientError};

    #[tokio::test]
    async fn google_errors_preserve_status_reason_and_details() {
        let (client, server) = fixture(vec![(403, r#"{"error":{"message":"Denied","errors":[{"reason":"forbidden","domain":"calendar"}],"details":[{"permission":"write"}]}}"#)]).await;
        let error = client
            .event_client()
            .delete(crate::Event::default())
            .await
            .unwrap_err();
        assert_eq!(error.status(), Some(reqwest::StatusCode::FORBIDDEN));
        match error {
            ClientError::Api(api) => {
                assert_eq!(api.errors[0].reason, "forbidden");
                assert_eq!(api.details[0]["permission"], "write");
            }
            other => panic!("expected API error: {other:?}"),
        }
        server.await.unwrap();
    }

    #[tokio::test]
    async fn empty_success_and_non_json_failures_are_handled() {
        let (client, server) = fixture(vec![(204, ""), (503, "upstream unavailable")]).await;
        client
            .delete(
                None,
                ApiRequest::new("calendars/primary/events/event".into()),
            )
            .await
            .unwrap();
        let error = client
            .get(None, ApiRequest::new("calendars/primary".into()))
            .await
            .unwrap_err();
        assert_eq!(
            error.status(),
            Some(reqwest::StatusCode::SERVICE_UNAVAILABLE)
        );
        server.await.unwrap();
    }
}
