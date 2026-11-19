use crate::build_oidc_http_client;
use openidconnect::{
    AuthenticationFlow, AuthorizationCode, ClientId, ClientSecret, CsrfToken, EndpointMaybeSet,
    EndpointNotSet, EndpointSet, IssuerUrl, Nonce, PkceCodeChallenge, PkceCodeVerifier,
    RedirectUrl, TokenResponse,
    core::{CoreClient, CoreProviderMetadata, CoreResponseType},
};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};
use url::Url;

pub const OIDC_LOCAL_PORT: u16 = 17175;
pub const OIDC_REDIRECT_URI: &str = "http://localhost:17175/callback";

const MAX_CALLBACK_REQUEST_LINE_BYTES: u64 = 8 * 1024;
const DEFAULT_CALLBACK_TIMEOUT: Duration = Duration::from_secs(300);

type AuthCoreClient = CoreClient<
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointMaybeSet,
    EndpointMaybeSet,
>;

pub struct OpenIDTokenAuth {
    client: AuthCoreClient,
    http_client: reqwest::Client,
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
        let http_client =
            build_oidc_http_client().map_err(|e| OpenIDTokenAuthError::InitError(e.to_string()))?;

        let provider_metadata = CoreProviderMetadata::discover_async(issuer_url, &http_client)
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
                http_client,
                pkce_challenge: Some(pkce_challenge),
                pkce_verifier: Some(pkce_verifier),
            })
        } else {
            Ok(OpenIDTokenAuth {
                client,
                http_client,
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

    pub fn parse_code_and_state(url: &Url) -> Option<(AuthorizationCode, CsrfToken)> {
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

    pub fn parse_callback_request_line(
        request_line: &str,
    ) -> Result<(AuthorizationCode, CsrfToken), OpenIDTokenAuthError> {
        let redirect_target = request_line.split_whitespace().nth(1).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid HTTP request line",
            )
        })?;

        let base = Url::parse(&format!("http://localhost:{}", OIDC_LOCAL_PORT))?;
        let url = base.join(redirect_target)?;

        Self::parse_code_and_state(&url).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Failed to parse code and state from callback URL",
            )
            .into()
        })
    }

    pub async fn exchange_token(
        &mut self,
        code: AuthorizationCode,
    ) -> Result<String, OpenIDTokenAuthError> {
        let pkce_verifier = self.pkce_verifier.take();

        let mut token_request = self
            .client
            .exchange_code(code)
            .map_err(|e| OpenIDTokenAuthError::TokenExchangeError(e.to_string()))?;

        if let Some(pkce_verifier) = pkce_verifier {
            token_request = token_request.set_pkce_verifier(pkce_verifier);
        }

        let token_response = token_request
            .request_async(&self.http_client)
            .await
            .map_err(|e| OpenIDTokenAuthError::TokenExchangeError(e.to_string()))?;

        let token = token_response
            .id_token()
            .ok_or_else(|| OpenIDTokenAuthError::TokenExchangeError("No ID token".to_string()))?
            .to_string();

        Ok(token)
    }

    pub async fn wait_for_callback(
        &self,
    ) -> Result<(AuthorizationCode, CsrfToken), OpenIDTokenAuthError> {
        let listener =
            tokio::net::TcpListener::bind(format!("127.0.0.1:{}", OIDC_LOCAL_PORT)).await?;
        Self::accept_callback_on_listener(&listener, DEFAULT_CALLBACK_TIMEOUT).await
    }

    async fn accept_callback_on_listener(
        listener: &tokio::net::TcpListener,
        timeout_duration: Duration,
    ) -> Result<(AuthorizationCode, CsrfToken), OpenIDTokenAuthError> {
        let deadline = tokio::time::Instant::now() + timeout_duration;

        loop {
            let (mut stream, _) = tokio::time::timeout_at(deadline, listener.accept())
                .await
                .map_err(|_| {
                    std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "Timed out waiting for OIDC callback",
                    )
                })??;

            let per_conn_deadline = std::cmp::min(
                deadline,
                tokio::time::Instant::now() + Duration::from_secs(10),
            );
            let mut request_line = String::new();
            let read_res = {
                let mut limited = (&mut stream).take(MAX_CALLBACK_REQUEST_LINE_BYTES);
                let mut reader = tokio::io::BufReader::new(&mut limited);
                tokio::time::timeout_at(per_conn_deadline, reader.read_line(&mut request_line))
                    .await
            };

            match read_res {
                Ok(Ok(0)) | Err(_) => {
                    // Ignore idle browser TCP preconnects that send no request line before closing/timing out.
                    if tokio::time::Instant::now() >= deadline {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::TimedOut,
                            "Timed out waiting for OIDC callback",
                        )
                        .into());
                    }
                    continue;
                }
                Ok(Err(io_err)) => return Err(io_err.into()),
                Ok(Ok(_)) => {}
            }

            if let Some(target) = request_line.split_whitespace().nth(1)
                && target.starts_with("/favicon.ico")
            {
                let _ = stream
                    .write_all(
                        b"HTTP/1.1 404 Not Found\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
                    )
                    .await;
                continue;
            }

            let (code, state) = Self::parse_callback_request_line(&request_line)?;

            let message = "Authenticated, close this window and return to the application.";
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: text/plain; charset=utf-8\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                message.len(),
                message
            );
            stream.write_all(response.as_bytes()).await?;

            return Ok((code, state));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_callback_request_line() {
        let line = "GET /callback?code=abc123&state=xyz789 HTTP/1.1\r\n";
        let (code, state) = OpenIDTokenAuth::parse_callback_request_line(line).unwrap();
        assert_eq!(code.secret(), "abc123");
        assert_eq!(state.secret(), "xyz789");

        assert!(
            OpenIDTokenAuth::parse_callback_request_line("GET /callback HTTP/1.1\r\n").is_err()
        );
        assert!(OpenIDTokenAuth::parse_callback_request_line("").is_err());
    }

    #[tokio::test]
    async fn test_accept_callback_tolerates_preconnect_and_favicon() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let client_task = tokio::spawn(async move {
            // 1. Simulated browser TCP preconnect that closes without sending data.
            let preconnect = tokio::net::TcpStream::connect(addr).await.unwrap();
            drop(preconnect);

            // 2. Simulated /favicon.ico request.
            let mut fav = tokio::net::TcpStream::connect(addr).await.unwrap();
            fav.write_all(b"GET /favicon.ico HTTP/1.1\r\nHost: localhost\r\n\r\n")
                .await
                .unwrap();

            // 3. Actual OIDC redirect callback.
            let mut cb = tokio::net::TcpStream::connect(addr).await.unwrap();
            cb.write_all(
                b"GET /callback?code=mycode&state=mystate HTTP/1.1\r\nHost: localhost\r\n\r\n",
            )
            .await
            .unwrap();
        });

        let (code, state) =
            OpenIDTokenAuth::accept_callback_on_listener(&listener, Duration::from_secs(5))
                .await
                .unwrap();
        client_task.await.unwrap();
        assert_eq!(code.secret(), "mycode");
        assert_eq!(state.secret(), "mystate");
    }
}
