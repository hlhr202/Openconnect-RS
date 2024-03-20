use crate::oidc::{OpenID, OpenIDConfig, OpenIDError, OIDC_REDIRECT_URI};
use openconnect_core::{
    config::{ConfigBuilder, EntrypointBuilder, LogLevel},
    events::EventHandlers,
    storage::StoredConfigs,
    Connectable, Status, VpnClient,
};
use std::sync::Arc;
use tauri::{
    async_runtime::{channel, RwLock, Sender},
    Manager,
};
use tokio::sync::mpsc::error::SendError;

#[allow(clippy::enum_variant_names)]
#[derive(Debug, thiserror::Error)]
pub enum StateError {
    #[error("Config error: {0}")]
    ConfigError(#[from] openconnect_core::storage::StoredConfigError),

    #[error("Openconnect error: {0}")]
    OpenconnectError(#[from] openconnect_core::result::OpenconnectError),

    #[error("Channel error: {0}")]
    ChannelError(#[from] SendError<VpnEvent>),

    #[error("Tauri error: {0}")]
    TauriError(#[from] tauri::Error),

    #[error("OpenID error: {0}")]
    OpenIdError(#[from] crate::oidc::OpenIDError),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
pub enum VpnEvent {
    Status(StatusPayload),
}

#[derive(serde::Serialize, Debug, Clone)]
pub struct StatusPayload {
    status: String,
    message: Option<String>,
}

impl From<Status> for StatusPayload {
    fn from(status: Status) -> Self {
        let (status, message) = match status {
            Status::Initialized => ("INITIALIZED".to_string(), None),
            Status::Connecting(msg) => ("CONNECTING".to_string(), Some(msg)),
            Status::Connected => ("CONNECTED".to_string(), None),
            Status::Disconnecting => ("DISCONNECTING".to_string(), None),
            Status::Disconnected => ("DISCONNECTED".to_string(), None),
            Status::Error(err) => ("ERROR".to_string(), Some(err.to_string())),
        };

        Self { status, message }
    }
}

pub struct AppState {
    pub(crate) event_tx: Sender<VpnEvent>,
    pub(crate) client: RwLock<Option<Arc<VpnClient>>>,
    pub(crate) stored_configs: RwLock<StoredConfigs>,
    #[allow(dead_code)]
    pub(crate) vpnc_sciprt: String,
}

impl AppState {
    pub async fn handle_with_vpnc_script(
        app: &mut tauri::App,
        vpnc_scipt: &str,
    ) -> Result<(), StateError> {
        let (event_tx, mut event_rx) = channel::<VpnEvent>(100);
        let app_state = AppState::new(event_tx, vpnc_scipt).await?;
        app.manage(app_state);

        let handle = app.app_handle();

        tauri::async_runtime::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                let handle = handle.clone();
                match event {
                    VpnEvent::Status(status) => {
                        let result = handle.emit_all("vpnStatus", Some(status));
                        if let Err(e) = result {
                            eprintln!("Error while emitting event: {:?}", e);
                        }
                    }
                }
            }
        });

        Ok(())
    }

    pub async fn trigger_state_retrieve(&self) -> Result<(), StateError> {
        let client = self.client.read().await;
        match client.as_ref() {
            Some(client) => {
                let state = client.get_state();
                Ok(self.event_tx.send(VpnEvent::Status(state.into())).await?)
            }
            None => Ok(self
                .event_tx
                .send(VpnEvent::Status(Status::Initialized.into()))
                .await?),
        }
    }

    pub async fn connect_with_user_pass(&self, server_name: &str) -> Result<(), StateError> {
        let stored_server = self.stored_configs.read().await;
        let password_server = stored_server.get_server_as_password_server(server_name)?;
        let password_server = &password_server.decrypted_by(&stored_server.cipher);

        let mut config = ConfigBuilder::default();

        #[cfg(not(target_os = "windows"))]
        let config = config.vpncscript(&self.vpnc_sciprt);

        let config = config.loglevel(LogLevel::Info).build()?;

        let entrypoint = EntrypointBuilder::new()
            .server(&password_server.server)
            .username(&password_server.username)
            .password(&password_server.password.clone().unwrap_or("".to_string()))
            .enable_udp(true)
            .build()?;

        let event_tx = self.event_tx.clone();

        let event_handlers =
            EventHandlers::default().with_handle_connection_state_change(move |state| {
                let event_tx = event_tx.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = event_tx.send(VpnEvent::Status(state.into())).await;
                    // ignore the result
                });
            });

        let client = VpnClient::new(config, event_handlers)?;
        {
            self.client.write().await.replace(client.clone());
        }
        tauri::async_runtime::spawn_blocking(move || {
            let _ = client.connect(entrypoint); // ignore the result
        });

        Ok(())
    }

    pub async fn connect_with_oidc(&self, server_name: &str) -> Result<(), StateError> {
        let stored_server = self.stored_configs.read().await;
        let oidc_server = stored_server.get_server_as_oidc_server(server_name)?;

        let openid_config = OpenIDConfig {
            issuer_url: oidc_server.issuer.clone(),
            redirect_uri: OIDC_REDIRECT_URI.to_string(),
            client_id: oidc_server.client_id.clone(),
            client_secret: oidc_server.client_secret.clone(),
        };

        let openid = OpenID::new(openid_config).await?;
        let (authorize_url, req_state, _) = openid.auth_request();

        open::that(authorize_url.to_string())?;
        let (code, callback_state) = openid.wait_for_callback().await?;

        if req_state.secret() != callback_state.secret() {
            return Err(OpenIDError::StateValidationError(
                "State validation failed".to_string(),
            ))?;
        }

        let token = openid.exchange_token(code).await?;
        let cookie = openid
            .obtain_cookie_by_oidc(&oidc_server.server, &token)
            .await
            .ok_or(StateError::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Failed to obtain cookie",
            )))?;

        let mut config = ConfigBuilder::default();

        #[cfg(not(target_os = "windows"))]
        let config = config.vpncscript(&self.vpnc_sciprt);

        let config = config.loglevel(LogLevel::Info).build()?;

        let entrypoint = EntrypointBuilder::new()
            .server(&oidc_server.server)
            .cookie(&cookie)
            .build()?;

        let event_tx = self.event_tx.clone();

        let event_handlers =
            EventHandlers::default().with_handle_connection_state_change(move |state| {
                let event_tx = event_tx.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = event_tx.send(VpnEvent::Status(state.into())).await;
                    // ignore the result
                });
            });

        let client = VpnClient::new(config, event_handlers)?;
        {
            self.client.write().await.replace(client.clone());
        }
        tauri::async_runtime::spawn_blocking(move || {
            let _ = client.connect(entrypoint); // ignore the result
        });

        Ok(())
    }

    pub async fn disconnect(&self) -> Result<(), StateError> {
        if let Some(client) = self.client.read().await.as_ref() {
            let client = client.clone();
            tauri::async_runtime::spawn_blocking(move || client.disconnect()).await?;
        }

        // self.client.write().await.take(); // TODO: wait a few seconds and drop the client

        Ok(())
    }

    pub async fn new(event_tx: Sender<VpnEvent>, vpnc_scipt: &str) -> Result<Self, StateError> {
        let mut stored_configs = StoredConfigs::default();
        stored_configs.read_from_file().await?;
        Ok(Self {
            event_tx,
            client: RwLock::new(None),
            stored_configs: RwLock::new(stored_configs),
            vpnc_sciprt: vpnc_scipt.to_string(),
        })
    }
}
