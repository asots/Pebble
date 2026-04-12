mod pkce;
pub mod redirect;
mod tokens;

pub use pkce::PkceState;
pub use redirect::OAuthRedirect;
pub use tokens::TokenPair;

use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, RefreshToken,
    Scope, TokenUrl,
};

/// Configuration for an OAuth2 provider.
#[derive(Debug, Clone)]
pub struct OAuthConfig {
    pub client_id: String,
    pub client_secret: Option<String>,
    pub auth_url: String,
    pub token_url: String,
    pub scopes: Vec<String>,
    pub redirect_port: u16,
}

/// Errors that can occur during the OAuth flow.
#[derive(thiserror::Error, Debug)]
pub enum OAuthError {
    #[error("OAuth request failed: {0}")]
    Request(String),
    #[error("Token exchange failed: {0}")]
    TokenExchange(String),
    #[error("Redirect listener failed: {0}")]
    Redirect(String),
    #[error("Invalid configuration: {0}")]
    Config(String),
    #[error("Token expired and no refresh token available")]
    NoRefreshToken,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Manages the full OAuth2 PKCE authentication flow.
pub struct OAuthManager {
    config: OAuthConfig,
}

/// Parsed and validated URL components, ready to build an oauth2 client.
#[derive(Debug)]
struct ParsedUrls {
    auth_url: AuthUrl,
    token_url: TokenUrl,
    redirect_url: RedirectUrl,
}

impl OAuthManager {
    pub fn new(config: OAuthConfig) -> Self {
        Self { config }
    }

    /// Parse and validate the URLs from the config.
    fn parse_urls(&self) -> Result<ParsedUrls, OAuthError> {
        let auth_url = AuthUrl::new(self.config.auth_url.clone())
            .map_err(|e| OAuthError::Config(format!("Invalid auth URL: {}", e)))?;
        let token_url = TokenUrl::new(self.config.token_url.clone())
            .map_err(|e| OAuthError::Config(format!("Invalid token URL: {}", e)))?;
        let redirect_url =
            RedirectUrl::new(format!("http://127.0.0.1:{}/callback", self.config.redirect_port))
                .map_err(|e| OAuthError::Config(format!("Invalid redirect URL: {}", e)))?;
        Ok(ParsedUrls {
            auth_url,
            token_url,
            redirect_url,
        })
    }

    /// Start the OAuth flow.
    ///
    /// Returns the authorization URL (to open in the system browser) and a
    /// [`PkceState`] that must be passed to [`complete_auth`](Self::complete_auth)
    /// after the user authorises the application.
    pub async fn start_auth(&self) -> Result<(String, PkceState), OAuthError> {
        let urls = self.parse_urls()?;
        let (challenge, verifier) = pkce::generate_pkce();

        let client = oauth2::basic::BasicClient::new(ClientId::new(self.config.client_id.clone()))
            .set_client_secret(ClientSecret::new(
                self.config.client_secret.clone().unwrap_or_default(),
            ))
            .set_auth_uri(urls.auth_url)
            .set_token_uri(urls.token_url)
            .set_redirect_uri(urls.redirect_url);

        let mut auth_request = client
            .authorize_url(CsrfToken::new_random)
            .set_pkce_challenge(challenge);

        for scope in &self.config.scopes {
            auth_request = auth_request.add_scope(Scope::new(scope.clone()));
        }

        let (auth_url, csrf_token) = auth_request.url();

        let state = PkceState {
            verifier,
            csrf_token,
        };

        Ok((auth_url.to_string(), state))
    }

    /// Complete the OAuth flow by exchanging the authorization code for tokens.
    pub async fn complete_auth(
        &self,
        code: &str,
        pkce_state: PkceState,
    ) -> Result<TokenPair, OAuthError> {
        let urls = self.parse_urls()?;
        let client = oauth2::basic::BasicClient::new(ClientId::new(self.config.client_id.clone()))
            .set_client_secret(ClientSecret::new(
                self.config.client_secret.clone().unwrap_or_default(),
            ))
            .set_auth_uri(urls.auth_url)
            .set_token_uri(urls.token_url)
            .set_redirect_uri(urls.redirect_url);

        let http_client = reqwest::ClientBuilder::new()
            .build()
            .map_err(|e| OAuthError::Request(format!("Failed to build HTTP client: {}", e)))?;

        let token_result = client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .set_pkce_verifier(pkce_state.verifier)
            .request_async(&http_client)
            .await
            .map_err(|e| OAuthError::TokenExchange(format!("{}", e)))?;

        Ok(token_response_to_pair(&token_result, None))
    }

    /// Refresh an expired access token using the provided refresh token.
    pub async fn refresh_token(&self, refresh_token: &str) -> Result<TokenPair, OAuthError> {
        let urls = self.parse_urls()?;
        let client = oauth2::basic::BasicClient::new(ClientId::new(self.config.client_id.clone()))
            .set_client_secret(ClientSecret::new(
                self.config.client_secret.clone().unwrap_or_default(),
            ))
            .set_auth_uri(urls.auth_url)
            .set_token_uri(urls.token_url)
            .set_redirect_uri(urls.redirect_url);

        let http_client = reqwest::ClientBuilder::new()
            .build()
            .map_err(|e| OAuthError::Request(format!("Failed to build HTTP client: {}", e)))?;

        let token_result = client
            .exchange_refresh_token(&RefreshToken::new(refresh_token.to_string()))
            .request_async(&http_client)
            .await
            .map_err(|e| OAuthError::TokenExchange(format!("Refresh failed: {}", e)))?;

        Ok(token_response_to_pair(&token_result, Some(refresh_token)))
    }

    /// Bind the redirect listener. Returns the bound listener with the actual port.
    pub async fn bind_redirect_listener(&self) -> Result<redirect::BoundRedirectListener, OAuthError> {
        redirect::bind_redirect_listener(self.config.redirect_port).await
    }

    /// Wait for the OAuth redirect on the configured port and return the
    /// authorization code together with the callback state.
    pub async fn wait_for_redirect(&self) -> Result<OAuthRedirect, OAuthError> {
        redirect::wait_for_redirect(self.config.redirect_port).await
    }
}

/// Convert an oauth2 token response into our [`TokenPair`].
fn token_response_to_pair(
    resp: &oauth2::basic::BasicTokenResponse,
    fallback_refresh: Option<&str>,
) -> TokenPair {
    use oauth2::TokenResponse;

    let expires_at = resp.expires_in().map(|d| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
            + d.as_secs() as i64
    });

    let scopes: Vec<String> = resp
        .scopes()
        .map(|s| s.iter().map(|scope| scope.to_string()).collect())
        .unwrap_or_default();

    let refresh_token = resp
        .refresh_token()
        .map(|t| t.secret().clone())
        .or_else(|| fallback_refresh.map(String::from));

    TokenPair {
        access_token: resp.access_token().secret().clone(),
        refresh_token,
        expires_at,
        scopes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> OAuthConfig {
        OAuthConfig {
            client_id: "test-client-id".into(),
            client_secret: None,
            auth_url: "https://accounts.google.com/o/oauth2/v2/auth".into(),
            token_url: "https://oauth2.googleapis.com/token".into(),
            scopes: vec!["https://mail.google.com/".into()],
            redirect_port: 8765,
        }
    }

    #[test]
    fn parse_urls_with_valid_config() {
        let mgr = OAuthManager::new(test_config());
        assert!(mgr.parse_urls().is_ok());
    }

    #[test]
    fn parse_urls_rejects_invalid_auth_url() {
        let mut cfg = test_config();
        cfg.auth_url = "not a url".into();
        let mgr = OAuthManager::new(cfg);
        let result = mgr.parse_urls();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), OAuthError::Config(_)));
    }

    #[test]
    fn parse_urls_rejects_invalid_token_url() {
        let mut cfg = test_config();
        cfg.token_url = ":::bad".into();
        let mgr = OAuthManager::new(cfg);
        let result = mgr.parse_urls();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn start_auth_returns_url_and_state() {
        let mgr = OAuthManager::new(test_config());
        let (url, state) = mgr.start_auth().await.unwrap();
        assert!(url.starts_with("https://accounts.google.com/"));
        assert!(url.contains("code_challenge"));
        assert!(!state.verifier.secret().is_empty());
        assert!(!state.csrf_token.secret().is_empty());
    }
}
