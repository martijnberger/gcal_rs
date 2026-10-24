use std::{
    fmt,
    time::{Duration, SystemTime},
};

use anyhow::{ensure, Context, Result};
use oauth2::{
    basic::{BasicClient, BasicTokenType},
    reqwest, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, EmptyExtraTokenFields,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, RefreshToken, RevocationUrl, Scope,
    StandardTokenResponse, TokenResponse, TokenUrl,
};
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    sync::Mutex,
};

#[derive(Serialize, Deserialize, Default, Clone, PartialEq, Eq)]
pub struct OToken {
    pub access: String,
    pub refresh: Option<String>,

    expires_at: Option<SystemTime>,
}

impl fmt::Debug for OToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OToken")
            .field("access", &"[REDACTED]")
            .field("refresh", &self.refresh.as_ref().map(|_| "[REDACTED]"))
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
pub struct OAuthRequest {
    pub code: String,
    pub state: String,
    pub scope: String,
}

pub type GoogleClient<
    HasAuthUrl = oauth2::EndpointSet,
    HasDeviceAuthUrl = oauth2::EndpointNotSet,
    HasIntrospectionUrl = oauth2::EndpointNotSet,
    HasRevocationUrl = oauth2::EndpointSet,
    HasTokenUrl = oauth2::EndpointSet,
> = oauth2::Client<
    oauth2::basic::BasicErrorResponse,
    oauth2::basic::BasicTokenResponse,
    oauth2::basic::BasicTokenIntrospectionResponse,
    oauth2::StandardRevocableToken,
    oauth2::basic::BasicRevocationErrorResponse,
    HasAuthUrl,
    HasDeviceAuthUrl,
    HasIntrospectionUrl,
    HasRevocationUrl,
    HasTokenUrl,
>;

#[derive(Debug)]
pub struct OAuth {
    client: GoogleClient,
    pending_authorization: Mutex<Option<(CsrfToken, PkceCodeVerifier)>>,
    http_client: reqwest::Client,
}

impl OAuth {
    /// Create a new OAuth client configuration.
    ///
    /// # Panics
    /// Panics if any of the hard-coded OAuth endpoints are invalid URLs or if the HTTP client
    /// fails to build.
    #[must_use]
    pub fn new(client_id: &str, client_secret: &str, redir_url: &str) -> Self {
        // Set up the config for the Google OAuth2 process.
        Self {
            client: BasicClient::new(ClientId::new(client_id.to_string()))
                .set_client_secret(ClientSecret::new(client_secret.to_string()))
                .set_auth_uri(
                    AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())
                        .expect("Invalid authorization endpoint URL"),
                )
                .set_token_uri(
                    TokenUrl::new("https://www.googleapis.com/oauth2/v3/token".to_string())
                        .expect("Invalid token endpoint URL"),
                )
                .set_redirect_uri(
                    RedirectUrl::new(redir_url.to_string()).expect("Invalid redirect URL"),
                )
                .set_revocation_url(
                    RevocationUrl::new("https://oauth2.googleapis.com/revoke".to_string())
                        .expect("Invalid revocation endpoint URL"),
                ),
            pending_authorization: Mutex::new(None),
            http_client: reqwest::ClientBuilder::new()
                // Following redirects opens the client up to SSRF vulnerabilities.
                .redirect(reqwest::redirect::Policy::none())
                .https_only(true)
                .build()
                .expect("Client should build"),
        }
    }

    /// Exchange a refresh token for a new access token.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or the token response cannot be parsed.
    /// Preserves the supplied refresh token if the server does not return a replacement.
    pub async fn exhange_refresh(&self, ref_token: impl ToString) -> Result<OToken> {
        let refresh_token = RefreshToken::new(ref_token.to_string());
        let mut token: OToken = self
            .client
            .exchange_refresh_token(&refresh_token)
            .request_async(&self.http_client)
            .await?
            .into();
        if token.refresh.is_none() {
            token.refresh = Some(refresh_token.into_secret());
        }
        Ok(token)
    }

    /// Generate an authorization URL, replacing any pending authorization flow.
    pub async fn auth_url(&self) -> String {
        // Google supports Proof Key for Code Exchange (PKCE - https://oauth.net/2/pkce/).
        // Create a PKCE code verifier and SHA-256 encode it as a code challenge.
        let (pkce_code_challenge, pkce_code_verifier) = PkceCodeChallenge::new_random_sha256();

        // Generate the authorization URL to which we'll redirect the user.
        let (authorize_url, csrf_state) = self
            .client
            .authorize_url(CsrfToken::new_random)
            // This example is requesting access to the "calendar" features and the user's profile.
            .add_scope(Scope::new(
                "https://www.googleapis.com/auth/calendar".to_string(),
            ))
            .add_scope(Scope::new(
                "https://www.googleapis.com/auth/userinfo.profile".to_string(),
            ))
            .add_scope(Scope::new(
                "https://www.googleapis.com/auth/userinfo.email".to_string(),
            ))
            .set_pkce_challenge(pkce_code_challenge)
            .url();
        *self.pending_authorization.lock().await = Some((csrf_state, pkce_code_verifier));
        authorize_url.to_string()
    }

    /// Exchange an authorization response for an access token.
    ///
    /// # Errors
    /// Returns an error if no authorization is pending, the CSRF state does not match,
    /// the token exchange fails, or the response cannot be parsed. Invalid state does not
    /// consume the pending authorization; matching responses can only be exchanged once.
    pub async fn auth(&self, request: OAuthRequest) -> Result<(String, OToken)> {
        let OAuthRequest { code, state, .. } = request;
        let state_token = CsrfToken::new(state);
        let pkce_code_verifier = {
            let mut pending = self.pending_authorization.lock().await;
            let (expected_state, _) = pending.as_ref().context("No pending OAuth authorization")?;
            // oauth2's timing-resistant-secret-traits feature makes this comparison safe.
            ensure!(
                expected_state == &state_token,
                "OAuth state does not match the pending authorization"
            );
            pending.take().context("No pending OAuth authorization")?.1
        };

        // Exchange the code with a token.
        Ok((
            state_token.into_secret(),
            self.client
                .exchange_code(AuthorizationCode::new(code))
                .set_pkce_verifier(pkce_code_verifier)
                .request_async(&self.http_client)
                .await?
                .into(),
        ))
    }

    /// Refresh the access token if it has expired.
    ///
    /// # Errors
    /// Returns an error if the refresh token is missing or if the token exchange fails.
    pub async fn refresh(&self, token: &mut OToken) -> Result<()> {
        if token.is_expired() {
            let t = self
                .exhange_refresh(
                    token
                        .refresh
                        .as_ref()
                        .context("Refresh token should exist")?,
                )
                .await?;
            token.take_over(t);
        }
        Ok(())
    }

    /// Perform an interactive OAuth flow using a loopback-only TCP listener.
    ///
    /// # Errors
    /// Returns an error if the local listener fails or the OAuth flow does not return a token.
    pub async fn naive(&mut self) -> Result<OToken> {
        async fn listener() -> Result<OAuthRequest> {
            fn query(url: &url::Url, key: &str) -> Option<String> {
                url.query_pairs()
                    .find(|(k, _)| k == key)
                    .map(|(_, state)| state.into_owned())
            }
            // A minimal loopback redirect listener.
            let listener = TcpListener::bind("127.0.0.1:8555").await?;
            let (mut stream, _) = listener.accept().await?;
            let mut reader = BufReader::new(&mut stream);
            let mut request_line = String::new();
            reader.read_line(&mut request_line).await?;

            let path = request_line
                .split_whitespace()
                .nth(1)
                .context("Missing OAuth callback request target")?;
            let url = url::Url::parse(&("http://127.0.0.1".to_string() + path))?;

            let auth = OAuthRequest {
                code: query(&url, "code").context("Missing OAuth callback code")?,
                state: query(&url, "state").context("Missing OAuth callback state")?,
                scope: query(&url, "scope").context("Missing OAuth callback scope")?,
            };

            let message = "Go back to your application!";
            stream
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\ncontent-length: {}\r\n\r\n{}",
                        message.len(),
                        message
                    )
                    .as_bytes(),
                )
                .await?;

            Ok(auth)
        }
        println!("🔗 Open this URL: {}", self.auth_url().await);

        let auth = listener().await.context("Failed to get auth response.")?;
        let (_, token) = self.auth(auth).await?;
        println!("[INFO] Successfully retrieved access token.");
        Ok(token)
    }
}

impl OToken {
    pub fn new(access: &str, refresh: Option<&str>, expires_at: Option<SystemTime>) -> Self {
        Self {
            access: access.to_string(),
            refresh: refresh.map(str::to_string),
            expires_at,
        }
    }
    #[must_use]
    pub fn is_expired(&self) -> bool {
        if let Some(t) = self.expires_at.map(|e| e <= SystemTime::now()) {
            return t;
        }
        false
    }
    pub fn take_over(&mut self, token: Self) {
        self.access = token.access;
        self.refresh = token.refresh;
        self.expires_at = token.expires_at;
    }
}

impl From<StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>> for OToken {
    fn from(value: StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>) -> Self {
        Self {
            access: value.access_token().secret().clone(),
            refresh: value.refresh_token().map(|r| r.secret().clone()),

            expires_at: compute_expiration(value.expires_in()),
        }
    }
}

fn compute_expiration(expires_in: Option<Duration>) -> Option<SystemTime> {
    let secs_valid = expires_in
        .and_then(|dur| dur.checked_sub(Duration::from_secs(60)))
        .or_else(|| Some(Duration::from_secs(0)));
    secs_valid.map(|secs| SystemTime::now() + secs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{io::AsyncReadExt, task::JoinHandle};

    fn query_value(url: &str, name: &str) -> String {
        url::Url::parse(url)
            .unwrap()
            .query_pairs()
            .find(|(key, _)| key == name)
            .unwrap()
            .1
            .into_owned()
    }

    fn callback(state: String) -> OAuthRequest {
        OAuthRequest {
            code: "test-authorization-code".to_string(),
            state,
            scope: String::new(),
        }
    }

    async fn local_oauth() -> (OAuth, TcpListener) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut oauth = OAuth::new("test-client", "test-secret", "http://127.0.0.1:8555/auth");
        oauth.client = oauth.client.set_token_uri(
            TokenUrl::new(format!("http://{}/token", listener.local_addr().unwrap())).unwrap(),
        );
        // Only the test transport permits HTTP, and never uses an environment proxy.
        oauth.http_client = reqwest::ClientBuilder::new()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();
        (oauth, listener)
    }

    fn token_response(
        listener: TcpListener,
        status: &'static str,
        body: &'static str,
    ) -> JoinHandle<String> {
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut reader = BufReader::new(&mut stream);
            let mut content_length = 0;
            loop {
                let mut line = String::new();
                assert_ne!(reader.read_line(&mut line).await.unwrap(), 0);
                if line == "\r\n" {
                    break;
                }
                if let Some((name, value)) = line.split_once(':') {
                    if name.eq_ignore_ascii_case("content-length") {
                        content_length = value.trim().parse().unwrap();
                    }
                }
            }
            let mut request_body = vec![0; content_length];
            reader.read_exact(&mut request_body).await.unwrap();
            stream
                .write_all(
                    format!(
                        "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
            String::from_utf8(request_body).unwrap()
        })
    }

    async fn assert_rejected_state(oauth: &OAuth, state: String) {
        tokio::time::timeout(Duration::from_secs(2), oauth.auth(callback(state)))
            .await
            .expect("Invalid state must be rejected without contacting the token endpoint")
            .unwrap_err();
    }

    #[test]
    fn token_debug_redacts_secrets_without_changing_serde() {
        let token = OToken::new("secret-access-value", Some("secret-refresh-value"), None);
        let debug = format!("{token:?}");
        assert!(!debug.contains(&token.access));
        assert!(!debug.contains(token.refresh.as_ref().unwrap()));
        assert!(debug.contains("[REDACTED]"));
        let serialized = serde_json::to_value(&token).unwrap();
        assert_eq!(serialized["access"], "secret-access-value");
        assert_eq!(serialized["refresh"], "secret-refresh-value");
        assert_eq!(serde_json::from_value::<OToken>(serialized).unwrap(), token);
    }

    #[tokio::test]
    async fn invalid_state_preserves_the_pending_authorization() {
        let (oauth, _listener) = local_oauth().await;
        let url = oauth.auth_url().await;
        for state in ["wrong-state", ""] {
            assert_rejected_state(&oauth, state.to_string()).await;
        }
        let pending = oauth.pending_authorization.lock().await;
        let (state, verifier) = pending.as_ref().unwrap();
        assert_eq!(state.secret(), &query_value(&url, "state"));
        assert_eq!(
            PkceCodeChallenge::from_code_verifier_sha256(verifier).as_str(),
            query_value(&url, "code_challenge")
        );
    }

    #[tokio::test]
    async fn repeated_authorization_urls_replace_state_and_matching_pkce_atomically() {
        let (oauth, listener) = local_oauth().await;
        let first = oauth.auth_url().await;
        let second = oauth.auth_url().await;
        assert_rejected_state(&oauth, query_value(&first, "state")).await;

        let server = token_response(
            listener,
            "200 OK",
            r#"{"access_token":"new-access","token_type":"Bearer"}"#,
        );
        let state = query_value(&second, "state");
        let (first_result, second_result) = tokio::join!(
            oauth.auth(callback(state.clone())),
            oauth.auth(callback(state.clone()))
        );
        let accepted = if let Ok(accepted) = first_result {
            assert!(second_result.is_err());
            accepted
        } else {
            second_result.unwrap()
        };
        assert_eq!(accepted.0, state);
        assert_eq!(accepted.1.access, "new-access");
        assert!(oauth.pending_authorization.lock().await.is_none());

        let body = server.await.unwrap();
        let verifier = url::form_urlencoded::parse(body.as_bytes())
            .find(|(key, _)| key == "code_verifier")
            .unwrap()
            .1
            .into_owned();
        assert_eq!(
            PkceCodeChallenge::from_code_verifier_sha256(&PkceCodeVerifier::new(verifier)).as_str(),
            query_value(&second, "code_challenge")
        );
    }

    #[tokio::test]
    async fn refresh_preserves_the_entire_token_on_failure() {
        for (status, body) in [
            ("400 Bad Request", r#"{"error":"invalid_grant"}"#),
            ("200 OK", r#"{"invalid":"token response"}"#),
        ] {
            let (oauth, listener) = local_oauth().await;
            let server = token_response(listener, status, body);
            let mut token = OToken::new(
                "old-access",
                Some("original-refresh"),
                Some(SystemTime::UNIX_EPOCH),
            );
            let original = token.clone();
            assert!(oauth.refresh(&mut token).await.is_err());
            assert_eq!(token, original);
            assert!(server
                .await
                .unwrap()
                .contains("refresh_token=original-refresh"));
        }
    }

    #[tokio::test]
    async fn refresh_preserves_or_rotates_refresh_tokens_as_returned() {
        for (body, expected_refresh) in [
            (
                r#"{"access_token":"new-access","token_type":"Bearer","expires_in":3600}"#,
                "original-refresh",
            ),
            (
                r#"{"access_token":"new-access","token_type":"Bearer","refresh_token":"rotated-refresh","expires_in":3600}"#,
                "rotated-refresh",
            ),
        ] {
            let (oauth, listener) = local_oauth().await;
            let server = token_response(listener, "200 OK", body);
            let mut token = OToken::new(
                "old-access",
                Some("original-refresh"),
                Some(SystemTime::UNIX_EPOCH),
            );
            oauth.refresh(&mut token).await.unwrap();
            assert_eq!(token.access, "new-access");
            assert_eq!(token.refresh.as_deref(), Some(expected_refresh));
            assert!(!token.is_expired());
            assert!(server
                .await
                .unwrap()
                .contains("refresh_token=original-refresh"));
        }
    }
}
