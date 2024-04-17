use openidconnect::{
    core::{
        CoreAuthDisplay, CoreClaimName, CoreClaimType, CoreClient, CoreClientAuthMethod,
        CoreDeviceAuthorizationResponse, CoreGrantType, CoreJsonWebKey, CoreJsonWebKeyType,
        CoreJsonWebKeyUse, CoreJweContentEncryptionAlgorithm, CoreJweKeyManagementAlgorithm,
        CoreJwsSigningAlgorithm, CoreResponseMode, CoreResponseType, CoreSubjectIdentifierType,
    },
    reqwest::async_http_client,
    AdditionalProviderMetadata, AuthType, ClientId, ClientSecret, DeviceAuthorizationUrl,
    IssuerUrl, ProviderMetadata, TokenResponse,
};
use std::{future::Future, time::Duration};

pub struct OpenIDDeviceAuth {
    client: CoreClient,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
struct DeviceEndpointProviderMetadata {
    device_authorization_endpoint: DeviceAuthorizationUrl,
}

impl AdditionalProviderMetadata for DeviceEndpointProviderMetadata {}

type DeviceProviderMetadata = ProviderMetadata<
    DeviceEndpointProviderMetadata,
    CoreAuthDisplay,
    CoreClientAuthMethod,
    CoreClaimName,
    CoreClaimType,
    CoreGrantType,
    CoreJweContentEncryptionAlgorithm,
    CoreJweKeyManagementAlgorithm,
    CoreJwsSigningAlgorithm,
    CoreJsonWebKeyType,
    CoreJsonWebKeyUse,
    CoreJsonWebKey,
    CoreResponseMode,
    CoreResponseType,
    CoreSubjectIdentifierType,
>;

pub struct OpenIDDeviceAuthConfig {
    pub issuer_url: String,
    pub client_id: String,
    pub client_secret: Option<String>,
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, thiserror::Error)]
pub enum OpenIDDeviceAuthError {
    #[error("Failed to initialize OpenID client: {0}")]
    InitError(String),

    #[error("Failed to exchange device token: {0}")]
    ExchangeDeviceTokenError(String),

    #[error("URL parse error: {0}")]
    UrlParseError(#[from] url::ParseError),

    #[error("Token exchange error: {0}")]
    TokenExchangeError(String),
}

impl OpenIDDeviceAuth {
    pub async fn new(config: OpenIDDeviceAuthConfig) -> Result<Self, OpenIDDeviceAuthError> {
        let issuer_url = IssuerUrl::new(config.issuer_url)?;

        let provider_metadata =
            DeviceProviderMetadata::discover_async(issuer_url, async_http_client)
                .await
                .map_err(|e| OpenIDDeviceAuthError::InitError(e.to_string()))?;

        let device_authorization_endpoint = provider_metadata
            .additional_metadata()
            .device_authorization_endpoint
            .clone();

        let client_id = ClientId::new(config.client_id);
        let client_secret = config.client_secret.map(ClientSecret::new);

        let client =
            CoreClient::from_provider_metadata(provider_metadata, client_id, client_secret)
                .set_device_authorization_uri(device_authorization_endpoint)
                .set_auth_type(AuthType::RequestBody);

        Ok(OpenIDDeviceAuth { client })
    }

    /// Exchange device token
    ///
    /// Example:
    /// ```rust
    /// let device_auth_response = openid_device_auth.exchange_device_token().await.unwrap();
    /// let verification_url = device_auth_response.verification_uri();
    /// let user_code = device_auth_response.user_code();
    /// println!(
    /// "Please visit {} and enter code {}",
    ///   **verification_url,
    /// . user_code.secret()
    /// );
    /// ```
    pub async fn exchange_device_token(
        &self,
    ) -> Result<CoreDeviceAuthorizationResponse, OpenIDDeviceAuthError> {
        let device_auth_request = self
            .client
            .exchange_device_code()
            .map_err(|e| OpenIDDeviceAuthError::ExchangeDeviceTokenError(e.to_string()))?;

        let device_auth_response: CoreDeviceAuthorizationResponse = device_auth_request
            .request_async(async_http_client)
            .await
            .map_err(|e| OpenIDDeviceAuthError::ExchangeDeviceTokenError(e.to_string()))?;

        Ok(device_auth_response)
    }

    pub async fn exchange_token<S, SF>(
        &mut self,
        device_auth_response: &CoreDeviceAuthorizationResponse,
        sleep_fn: S,
        timout: Option<Duration>,
    ) -> Result<String, OpenIDDeviceAuthError>
    where
        SF: Future<Output = ()>,
        S: Fn(Duration) -> SF,
    {
        let token_response = self
            .client
            .exchange_device_access_token(device_auth_response)
            .request_async(async_http_client, sleep_fn, timout)
            .await
            .map_err(|e| OpenIDDeviceAuthError::TokenExchangeError(e.to_string()))?;

        let token = token_response
            .id_token()
            .ok_or(OpenIDDeviceAuthError::TokenExchangeError(
                "No ID token".to_string(),
            ))?
            .to_string();

        Ok(token)
    }
}
