use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};
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

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
pub struct OToken {
    pub access: String,
    pub refresh: Option<String>,

    expires_at: Option<SystemTime>,
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
    pkce_code_verifier: Mutex<Option<PkceCodeVerifier>>,
    http_client: reqwest::Client,
}

impl OAuth {
    pub fn new(
        client_id: impl ToString,
        client_secret: impl ToString,
        redir_url: impl ToString,
    ) -> Self {
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
            pkce_code_verifier: Mutex::new(None),
            http_client: reqwest::ClientBuilder::new()
                // Following redirects opens the client up to SSRF vulnerabilities.
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("Client should build"),
        }
    }

    pub async fn exhange_refresh(&self, ref_token: impl ToString) -> Result<OToken> {
        Ok(self
            .client
            .exchange_refresh_token(&RefreshToken::new(ref_token.to_string()))
            .request_async(&self.http_client)
            .await?
            .into())
    }

    pub async fn auth_url(&self) -> String {
        // Google supports Proof Key for Code Exchange (PKCE - https://oauth.net/2/pkce/).
        // Create a PKCE code verifier and SHA-256 encode it as a code challenge.
        let (pkce_code_challenge, pkce_code_verifier) = PkceCodeChallenge::new_random_sha256();
        self.pkce_code_verifier
            .lock()
            .await
            .get_or_insert(pkce_code_verifier);

        // Generate the authorization URL to which we'll redirect the user.
        let (authorize_url, _csrf_state) = self
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
        authorize_url.to_string()
    }

    pub async fn auth(&self, request: OAuthRequest) -> Result<(String, OToken)> {
        let _scope = request.scope;

        // Exchange the code with a token.
        Ok((
            CsrfToken::new(request.state).secret().clone(),
            self.client
                .exchange_code(AuthorizationCode::new(request.code))
                .set_pkce_verifier(
                    self.pkce_code_verifier
                        .lock()
                        .await
                        .take()
                        .context("PKCE code verifier should exist at this point")?,
                )
                .request_async(&self.http_client)
                .await?
                .into(),
        ))
    }

    pub async fn refresh(&self, token: &mut OToken) -> Result<()> {
        if token.is_expired() {
            let t = self
                .exhange_refresh(token.refresh.take().context("Refresh token should exist")?)
                .await?;
            token.take_over(t);
        }
        Ok(())
    }

    pub async fn naive(&mut self) -> Result<OToken> {
        async fn listener() -> Option<OAuthRequest> {
            fn query(url: &url::Url, key: &str) -> Option<String> {
                url.query_pairs()
                    .find(|(k, _)| k == key)
                    .map(|(_, state)| state.into_owned())
            }
            // // A very naive implementation of the redirect server.
            let listener = TcpListener::bind("0.0.0.0:8555").await.unwrap();
            loop {
                if let Ok((mut stream, _)) = listener.accept().await {
                    let mut reader = BufReader::new(&mut stream);
                    let mut request_line = String::new();
                    reader.read_line(&mut request_line).await.unwrap();

                    let url = url::Url::parse(
                        &("http://127.0.0.1".to_string()
                            + request_line.split_whitespace().nth(1)?),
                    )
                    .ok()?;

                    let auth = OAuthRequest {
                        code: query(&url, "code")?,
                        state: query(&url, "state")?,
                        scope: query(&url, "scope")?,
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
                        .await
                        .ok()?;

                    // The server will terminate itself after collecting the first code.
                    break Some(auth);
                }
            }
        }
        println!("🔗 Open this URL: {}", self.auth_url().await);

        let auth = listener().await.context("Failed to get auth response.")?;
        let (_, token) = self.auth(auth).await?;
        println!("[INFO] Successfully retrieved access token.");
        Ok(token)
    }
}

impl OToken {
    pub fn new(
        access: impl ToString,
        refresh: Option<impl ToString>,
        expires_at: Option<SystemTime>,
    ) -> Self {
        Self {
            access: access.to_string(),
            refresh: refresh.map(|r| r.to_string()),
            expires_at,
        }
    }
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
