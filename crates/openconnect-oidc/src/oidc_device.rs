use crate::build_oidc_http_client;
use openidconnect::{
    AdditionalProviderMetadata, AuthType, ClientId, ClientSecret, DeviceAuthorizationUrl,
    EndpointMaybeSet, EndpointNotSet, EndpointSet, IssuerUrl, ProviderMetadata, TokenResponse,
    core::{
        CoreAuthDisplay, CoreClaimName, CoreClaimType, CoreClient, CoreClientAuthMethod,
        CoreDeviceAuthorizationResponse, CoreGrantType, CoreJsonWebKey,
        CoreJweContentEncryptionAlgorithm, CoreJweKeyManagementAlgorithm, CoreResponseMode,
        CoreResponseType, CoreSubjectIdentifierType,
    },
};
use std::{future::Future, time::Duration};

type DeviceCoreClient = CoreClient<
    EndpointSet,
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointMaybeSet,
    EndpointMaybeSet,
>;

pub struct OpenIDDeviceAuth {
    client: DeviceCoreClient,
    http_client: reqwest::Client,
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
        let http_client = build_oidc_http_client()
            .map_err(|e| OpenIDDeviceAuthError::InitError(e.to_string()))?;

        let provider_metadata = DeviceProviderMetadata::discover_async(issuer_url, &http_client)
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
                .set_device_authorization_url(device_authorization_endpoint)
                .set_auth_type(AuthType::RequestBody);

        Ok(OpenIDDeviceAuth {
            client,
            http_client,
        })
    }

    pub async fn exchange_device_token(
        &self,
    ) -> Result<CoreDeviceAuthorizationResponse, OpenIDDeviceAuthError> {
        let device_auth_request = self.client.exchange_device_code();

        let device_auth_response: CoreDeviceAuthorizationResponse = device_auth_request
            .request_async(&self.http_client)
            .await
            .map_err(|e| OpenIDDeviceAuthError::ExchangeDeviceTokenError(e.to_string()))?;

        Ok(device_auth_response)
    }

    pub async fn exchange_token<S, SF>(
        &mut self,
        device_auth_response: &CoreDeviceAuthorizationResponse,
        sleep_fn: S,
        timeout: Option<Duration>,
    ) -> Result<String, OpenIDDeviceAuthError>
    where
        SF: Future<Output = ()>,
        S: Fn(Duration) -> SF,
    {
        let token_request = self
            .client
            .exchange_device_access_token(device_auth_response)
            .map_err(|e| OpenIDDeviceAuthError::TokenExchangeError(e.to_string()))?;

        let token_response = token_request
            .request_async(&self.http_client, sleep_fn, timeout)
            .await
            .map_err(|e| OpenIDDeviceAuthError::TokenExchangeError(e.to_string()))?;

        let token = token_response
            .id_token()
            .ok_or_else(|| OpenIDDeviceAuthError::TokenExchangeError("No ID token".to_string()))?
            .to_string();

        Ok(token)
    }
}
