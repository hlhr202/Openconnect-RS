use anyhow::Ok;
use openidconnect::core::{CoreClient, CoreProviderMetadata, CoreResponseType};
use openidconnect::{reqwest, TokenResponse};
use openidconnect::{
    AuthenticationFlow, AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce,
    RedirectUrl,
};
use std::str::FromStr;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use url::Url;

pub const OIDC_REDIRECT_URI: &str = "http://localhost:8080";

pub struct OpenID {
    client: CoreClient,
}

pub struct OpenIDConfig {
    pub issuer_url: String,
    pub redirect_uri: String,
    pub client_id: String,
    pub client_secret: Option<String>,
}

impl OpenID {
    pub async fn new(config: OpenIDConfig) -> anyhow::Result<Self> {
        tauri::async_runtime::spawn_blocking(|| {
            let issuer_url = IssuerUrl::new(config.issuer_url)?;
            let provider_metadata =
                CoreProviderMetadata::discover(&issuer_url, openidconnect::reqwest::http_client)?;
            let redirect_uri = RedirectUrl::new(config.redirect_uri)?;
            let client_id = ClientId::new(config.client_id);
            let client_secret = config.client_secret.map(ClientSecret::new);

            let client =
                CoreClient::from_provider_metadata(provider_metadata, client_id, client_secret)
                    .set_redirect_uri(redirect_uri);

            Ok(OpenID { client })
        })
        .await?
    }

    pub fn auth_request(&self) -> anyhow::Result<(Url, CsrfToken, Nonce)> {
        let (authorize_url, csrf_state, nonce) = self
            .client
            .authorize_url(
                AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            .url();

        Ok((authorize_url, csrf_state, nonce))
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

    pub async fn exchange_token(&self, code: AuthorizationCode) -> anyhow::Result<String> {
        let client = self.client.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let token_response = client.exchange_code(code).request(reqwest::http_client)?;

            let token = token_response
                .id_token()
                .ok_or(anyhow::anyhow!("Response token is empty"))?
                .to_string();

            Ok(token)
        })
        .await?
    }

    pub async fn wait_for_callback(&self) -> anyhow::Result<(AuthorizationCode, CsrfToken)> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
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

        let (code, state) = self.parse_code_and_state(url).ok_or(anyhow::anyhow!(
            "Failed to parse code and state from redirect URL"
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
