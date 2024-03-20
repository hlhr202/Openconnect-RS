use openidconnect::core::{CoreClient, CoreProviderMetadata, CoreResponseType};
use openidconnect::{reqwest, PkceCodeChallenge, PkceCodeVerifier, TokenResponse};
use openidconnect::{
    AuthenticationFlow, AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce,
    RedirectUrl,
};
use std::str::FromStr;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use url::Url;

pub const OIDC_LOCAL_PORT: u16 = 17175;
pub const OIDC_REDIRECT_URI: &str = "http://localhost:17175/callback";

pub struct OpenID {
    client: CoreClient,
    pkce_challenge: Option<PkceCodeChallenge>,
    pkce_verifier: Option<PkceCodeVerifier>,
}

pub struct OpenIDConfig {
    pub issuer_url: String,
    pub redirect_uri: String,
    pub client_id: String,
    pub use_pkce_challenge: bool,
    pub client_secret: Option<String>,
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, thiserror::Error)]
pub enum OpenIDError {
    #[error("Channel error: {0}")]
    ChannelError(String),

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

impl OpenID {
    pub async fn new(config: OpenIDConfig) -> Result<Self, OpenIDError> {
        tauri::async_runtime::spawn_blocking(move || {
            let issuer_url = IssuerUrl::new(config.issuer_url)?;
            let provider_metadata =
                CoreProviderMetadata::discover(&issuer_url, openidconnect::reqwest::http_client)
                    .map_err(|e| OpenIDError::InitError(e.to_string()))?;
            let redirect_uri = RedirectUrl::new(config.redirect_uri)?;
            let client_id = ClientId::new(config.client_id);
            let client_secret = config.client_secret.map(ClientSecret::new);

            let client =
                CoreClient::from_provider_metadata(provider_metadata, client_id, client_secret)
                    .set_redirect_uri(redirect_uri);

            if config.use_pkce_challenge {
                let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

                Ok(OpenID {
                    client,
                    pkce_challenge: Some(pkce_challenge),
                    pkce_verifier: Some(pkce_verifier),
                })
            } else {
                Ok(OpenID {
                    client,
                    pkce_challenge: None,
                    pkce_verifier: None,
                })
            }
        })
        .await
        .map_err(|e| OpenIDError::ChannelError(e.to_string()))?
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

    pub async fn exchange_token(&mut self, code: AuthorizationCode) -> Result<String, OpenIDError> {
        let client = self.client.clone();
        let pkce_verifier = self.pkce_verifier.take();

        tauri::async_runtime::spawn_blocking(move || {
            let mut token_response = client.exchange_code(code);

            if let Some(pkce_verifier) = pkce_verifier {
                token_response = token_response.set_pkce_verifier(pkce_verifier);
            }

            let token_response = token_response
                .request(reqwest::http_client)
                .map_err(|e| OpenIDError::TokenExchangeError(e.to_string()))?;

            let token = token_response
                .id_token()
                .ok_or(OpenIDError::TokenExchangeError("No ID token".to_string()))?
                .to_string();

            Ok(token)
        })
        .await
        .map_err(|e| OpenIDError::ChannelError(e.to_string()))?
    }

    pub async fn wait_for_callback(&self) -> Result<(AuthorizationCode, CsrfToken), OpenIDError> {
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

    pub async fn obtain_cookie_by_oidc(&self, server_url: &str, token: &str) -> Option<String> {
        let client = ::reqwest::ClientBuilder::new()
            .danger_accept_invalid_certs(true)
            .default_headers(::reqwest::header::HeaderMap::new())
            .http1_allow_obsolete_multiline_headers_in_responses(true)
            .http1_allow_spaces_after_header_name_in_responses(true)
            .http1_ignore_invalid_headers_in_responses(true)
            .http1_title_case_headers()
            .no_brotli()
            .no_deflate()
            .no_gzip()
            .no_proxy()
            .build()
            .ok()?;

        let mut url = ::reqwest::Url::from_str(server_url).ok()?;
        if url.path().is_empty() {
            url.set_path("auth")
        }
        let req_builder = client
            .post(url)
            .header("Accept", "*/*")
            .header("User-Agent", "AnnyConnect Compatible Client")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .bearer_auth(token);

        let req = req_builder.build().ok()?;

        let res = client.execute(req).await.ok()?;
        if !res.status().is_success() {
            eprintln!(
                "Failed to obtain cookie from server. Error code: {:?}",
                res.status()
            );
            return None;
        }

        let combined_cookie = res
            .cookies()
            .map(|c| format!("{}={}", c.name(), c.value()))
            .collect::<Vec<_>>()
            .join("; ");

        Some(combined_cookie)
    }
}
