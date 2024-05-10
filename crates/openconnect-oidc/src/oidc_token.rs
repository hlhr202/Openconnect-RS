use openidconnect::{
    core::{CoreClient, CoreProviderMetadata, CoreResponseType},
    reqwest::async_http_client,
    AuthenticationFlow, AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, TokenResponse,
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use url::Url;

pub const OIDC_LOCAL_PORT: u16 = 17175;
pub const OIDC_REDIRECT_URI: &str = "http://localhost:17175/callback";

pub struct OpenIDTokenAuth {
    client: CoreClient,
    pkce_challenge: Option<PkceCodeChallenge>,
    pkce_verifier: Option<PkceCodeVerifier>,
}

pub struct OpenIDTokenAuthConfig {
    pub issuer_url: String,
    pub redirect_uri: String,
    pub client_id: String,
    pub use_pkce_challenge: bool,
    pub client_secret: Option<String>,
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, thiserror::Error)]
pub enum OpenIDTokenAuthError {
    #[error("Failed to initialize OpenID client: {0}")]
    InitError(String),

    #[error("IO error: {0}")]
    IoError(#[from] tokio::io::Error),

    #[error("URL parse error: {0}")]
    UrlParseError(#[from] url::ParseError),

    #[error("OpenID state validation error: {0}")]
    StateValidationError(String),

    #[error("Token exchange error: {0}")]
    TokenExchangeError(String),
}

impl OpenIDTokenAuth {
    pub async fn new(config: OpenIDTokenAuthConfig) -> Result<Self, OpenIDTokenAuthError> {
        let issuer_url = IssuerUrl::new(config.issuer_url)?;
        let provider_metadata = CoreProviderMetadata::discover_async(issuer_url, async_http_client)
            .await
            .map_err(|e| OpenIDTokenAuthError::InitError(e.to_string()))?;
        let redirect_uri = RedirectUrl::new(config.redirect_uri)?;
        let client_id = ClientId::new(config.client_id);
        let client_secret = config.client_secret.and_then(|s| {
            if config.use_pkce_challenge || s.trim().is_empty() {
                None
            } else {
                Some(ClientSecret::new(s))
            }
        });

        let client =
            CoreClient::from_provider_metadata(provider_metadata, client_id, client_secret)
                .set_redirect_uri(redirect_uri);

        if config.use_pkce_challenge {
            let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

            Ok(OpenIDTokenAuth {
                client,
                pkce_challenge: Some(pkce_challenge),
                pkce_verifier: Some(pkce_verifier),
            })
        } else {
            Ok(OpenIDTokenAuth {
                client,
                pkce_challenge: None,
                pkce_verifier: None,
            })
        }
    }

    pub fn auth_request(&mut self) -> (Url, CsrfToken, Nonce) {
        let mut auth_request = self.client.authorize_url(
            AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        );

        if let Some(pkce_challenge) = self.pkce_challenge.take() {
            auth_request = auth_request.set_pkce_challenge(pkce_challenge);
        }

        auth_request.url()
    }

    pub fn parse_code_and_state(&self, url: Url) -> Option<(AuthorizationCode, CsrfToken)> {
        let code = url
            .query_pairs()
            .find(|(key, _)| key == "code")
            .map(|(_, code)| AuthorizationCode::new(code.into_owned()))?;

        let state = url
            .query_pairs()
            .find(|(key, _)| key == "state")
            .map(|(_, state)| CsrfToken::new(state.into_owned()))?;

        Some((code, state))
    }

    pub async fn exchange_token(
        &mut self,
        code: AuthorizationCode,
    ) -> Result<String, OpenIDTokenAuthError> {
        let client = self.client.clone();
        let pkce_verifier = self.pkce_verifier.take();

        let mut token_response = client.exchange_code(code);

        if let Some(pkce_verifier) = pkce_verifier {
            token_response = token_response.set_pkce_verifier(pkce_verifier);
        }

        let token_response = token_response
            .request_async(async_http_client)
            .await
            .map_err(|e| OpenIDTokenAuthError::TokenExchangeError(e.to_string()))?;

        let token = token_response
            .id_token()
            .ok_or(OpenIDTokenAuthError::TokenExchangeError(
                "No ID token".to_string(),
            ))?
            .to_string();

        Ok(token)
    }

    pub async fn wait_for_callback(
        &self,
    ) -> Result<(AuthorizationCode, CsrfToken), OpenIDTokenAuthError> {
        let listener =
            tokio::net::TcpListener::bind(format!("127.0.0.1:{}", OIDC_LOCAL_PORT)).await?;
        let (mut stream, _) = listener.accept().await?;
        let mut reader = tokio::io::BufReader::new(&mut stream);
        let mut request_line = String::new();
        reader.read_line(&mut request_line).await?;
        let redirect_url = request_line
            .split_whitespace()
            .nth(1)
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid request",
            ))?;
        let url = Url::parse(&format!("{},{}", OIDC_REDIRECT_URI, redirect_url))?;

        let (code, state) = self.parse_code_and_state(url).ok_or(tokio::io::Error::new(
            tokio::io::ErrorKind::InvalidInput,
            "Failed to parse code and state",
        ))?;
        let message = "Authenticated, close this window and return to the application.";
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-length: {}\r\n\r\n{}",
            message.len(),
            message
        );

        stream.write_all(response.as_bytes()).await?;

        Ok((code, state))
    }
}
