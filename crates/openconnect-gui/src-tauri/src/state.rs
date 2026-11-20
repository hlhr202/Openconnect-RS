use crate::{
    cert_prompt::{CertPromptBroker, CertPromptPayload},
    system_tray::AppSystemTray,
};
use openconnect_core::{
    Connectable, Status, VpnClient,
    config::{ConfigBuilder, EntrypointBuilder, LogLevel},
    events::EventHandlers,
    storage::{StoredConfigError, StoredConfigs, StoredServer},
};
use openconnect_oidc::{
    obtain_cookie_by_oidc_token,
    oidc_token::{OIDC_REDIRECT_URI, OpenIDTokenAuth, OpenIDTokenAuthConfig, OpenIDTokenAuthError},
};
use std::{path::PathBuf, sync::Arc};
use tauri::{
    Emitter, Manager, State,
    async_runtime::{RwLock, Sender, channel},
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
    OpenIdError(#[from] OpenIDTokenAuthError),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
pub enum VpnEvent {
    Status { status: StatusPayload },
    CertPrompt { prompt: CertPromptPayload },
}

#[derive(serde::Serialize, Debug, Clone)]
pub struct StatusPayload {
    pub status: String,
    pub message: Option<String>,
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
    pub(crate) vpnc_script: String,
    pub(crate) cert_prompts: Arc<CertPromptBroker>,
}

impl AppState {
    pub async fn handle_with_vpnc_script(
        app: &mut tauri::App,
        vpnc_script: &str,
        config_file: PathBuf,
    ) -> Result<(), StateError> {
        let (event_tx, mut event_rx) = channel::<VpnEvent>(100);
        let app_state = AppState::new(event_tx, vpnc_script, config_file).await?;
        app.manage(app_state);

        let handle = app.handle().clone();

        tauri::async_runtime::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                let handle = handle.clone();
                let app_system_tray: State<'_, Arc<AppSystemTray>> = handle.state();
                match event {
                    VpnEvent::Status { status } => {
                        if let Err(e) = handle.emit("vpnStatus", Some(status)) {
                            eprintln!("Error while emitting event: {:?}", e);
                        }
                        if let Err(e) = app_system_tray.recreate(&handle).await {
                            eprintln!("Error while updating tray: {:?}", e);
                        }
                    }
                    VpnEvent::CertPrompt { prompt } => {
                        // The connection may have been started from the tray while hidden.
                        if let Some(window) = handle.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.unminimize();
                            let _ = window.set_focus();
                        }
                        if let Err(e) = handle.emit("vpnCertPrompt", Some(prompt)) {
                            eprintln!("Error while emitting event: {:?}", e);
                        }
                    }
                }
            }
        });

        Ok(())
    }

    pub async fn get_status_and_name(&self) -> Result<(StatusPayload, Option<String>), StateError> {
        let client = self.client.read().await;
        let name = client.as_ref().and_then(|c| c.get_server_name());
        let state = client
            .as_ref()
            .map_or(Status::Initialized, |c| c.get_status());
        Ok((state.into(), name))
    }

    pub async fn trigger_state_retrieve(&self) -> Result<(), StateError> {
        let (status, _server_name) = self.get_status_and_name().await?;
        Ok(self.event_tx.send(VpnEvent::Status { status }).await?)
    }

    pub async fn connect_with_server_name(&self, server_name: &str) -> Result<(), StateError> {
        let stored_server = self.stored_configs.read().await;
        let server = stored_server.servers.get(server_name);
        match server {
            Some(StoredServer::Password(_)) => self.connect_with_user_pass(server_name).await,
            Some(StoredServer::Oidc(_)) => self.connect_with_oidc(server_name).await,
            None => Err(StoredConfigError::BadInput("Server not found".to_string()).into()),
        }
    }

    pub async fn connect_with_user_pass(&self, server_name: &str) -> Result<(), StateError> {
        let stored_server = self.stored_configs.read().await;
        let password_server = stored_server.get_server_as_password_server(server_name)?;
        let password_server = password_server.decrypted_by(&stored_server.cipher);

        let config = ConfigBuilder::default()
            .vpncscript(&self.vpnc_script)
            .loglevel(LogLevel::Info)
            .build()?;

        let mut entrypoint_builder = EntrypointBuilder::new();
        entrypoint_builder
            .name(&password_server.name)
            .server(&password_server.server)
            .username(&password_server.username)
            .password(password_server.password.as_deref().unwrap_or(""))
            .accept_insecure_cert(password_server.allow_insecure.unwrap_or(false))
            .enable_udp(true);
        if let Some(server_cert) = password_server.server_cert.as_deref() {
            entrypoint_builder.server_cert(server_cert);
        }
        let entrypoint = entrypoint_builder.build()?;
        drop(stored_server);

        let event_handlers = self.create_event_handler(&password_server.name);

        let client = VpnClient::new(config, event_handlers)?;
        {
            self.client.write().await.replace(client.clone());
        }
        Self::spawn_connection(client, entrypoint);

        Ok(())
    }

    pub async fn connect_with_oidc(&self, server_name: &str) -> Result<(), StateError> {
        let stored_server = self.stored_configs.read().await;
        let oidc_server = stored_server
            .get_server_as_oidc_server(server_name)?
            .clone();
        drop(stored_server);

        let allow_insecure = oidc_server.allow_insecure.unwrap_or(false);

        let openid_config = OpenIDTokenAuthConfig {
            issuer_url: oidc_server.issuer.clone(),
            redirect_uri: OIDC_REDIRECT_URI.to_string(),
            client_id: oidc_server.client_id.clone(),
            client_secret: oidc_server.client_secret.clone(),
            use_pkce_challenge: true,
        };

        let mut openid = OpenIDTokenAuth::new(openid_config).await?;
        let (authorize_url, req_state, _) = openid.auth_request();

        open::that(authorize_url.to_string())?;
        let (code, callback_state) = openid.wait_for_callback().await?;

        if req_state.secret() != callback_state.secret() {
            return Err(OpenIDTokenAuthError::StateValidationError(
                "State validation failed".to_string(),
            )
            .into());
        }

        let token = openid.exchange_token(code).await?;
        let cookie = obtain_cookie_by_oidc_token(&oidc_server.server, &token, allow_insecure)
            .await
            .ok_or_else(|| {
                StateError::IoError(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Failed to obtain cookie",
                ))
            })?;

        let config = ConfigBuilder::default()
            .vpncscript(&self.vpnc_script)
            .loglevel(LogLevel::Info)
            .build()?;

        let mut entrypoint_builder = EntrypointBuilder::new();
        entrypoint_builder
            .name(&oidc_server.name)
            .server(&oidc_server.server)
            .cookie(&cookie)
            .accept_insecure_cert(allow_insecure);
        if let Some(server_cert) = oidc_server.server_cert.as_deref() {
            entrypoint_builder.server_cert(server_cert);
        }
        let entrypoint = entrypoint_builder.build()?;

        let event_handlers = self.create_event_handler(&oidc_server.name);

        let client = VpnClient::new(config, event_handlers)?;
        {
            self.client.write().await.replace(client.clone());
        }
        Self::spawn_connection(client, entrypoint);

        Ok(())
    }

    /// Run the (blocking) connection on a worker thread. The peer-certificate handler may block
    /// there waiting for the user's decision; failures are reported through status events.
    fn spawn_connection(client: Arc<VpnClient>, entrypoint: openconnect_core::config::Entrypoint) {
        tauri::async_runtime::spawn_blocking(move || {
            if client.init_connection(entrypoint).is_ok() {
                let _ = client.run_loop();
            }
        });
    }

    /// Apply the user's answer to a certificate prompt, persisting the fingerprint on acceptance.
    pub async fn respond_cert_prompt(&self, id: u64, accept: bool) -> Result<(), StateError> {
        let Some(resolved) = self.cert_prompts.resolve(id) else {
            return Err(StoredConfigError::BadInput(
                "Certificate prompt expired or already answered".to_string(),
            )
            .into());
        };

        if accept {
            let saved = self
                .stored_configs
                .write()
                .await
                .set_server_cert(&resolved.server_name, Some(&resolved.fingerprint))
                .await
                .map(|_| ());
            if let Err(e) = saved {
                // Still honour the decision for this session, but surface the failure.
                resolved.respond(true);
                return Err(e.into());
            }
        }

        resolved.respond(accept);
        Ok(())
    }

    pub async fn disconnect(&self) -> Result<(), StateError> {
        if let Some(client) = self.client.read().await.as_ref() {
            let client = client.clone();
            tauri::async_runtime::spawn_blocking(move || client.disconnect()).await?;
        }

        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        self.client.write().await.take();

        Ok(())
    }

    pub fn create_event_handler(
        &self,
        server_name: &str,
    ) -> openconnect_core::events::EventHandlers {
        let event_tx_for_state = self.event_tx.clone();
        let event_tx_for_cert = self.event_tx.clone();
        let cert_prompts = Arc::clone(&self.cert_prompts);
        let server_name = server_name.to_string();

        EventHandlers::default()
            .with_handle_connection_state_change(move |state| {
                let event_tx = event_tx_for_state.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = event_tx
                        .send(VpnEvent::Status {
                            status: state.into(),
                        })
                        .await;
                });
            })
            .with_handle_peer_cert_verify(move |prompt| {
                // Runs on the blocking connection thread (see `spawn_connection`).
                cert_prompts.ask_blocking(&server_name, prompt, |payload| {
                    let event_tx = event_tx_for_cert.clone();
                    tauri::async_runtime::spawn(async move {
                        let _ = event_tx
                            .send(VpnEvent::CertPrompt { prompt: payload })
                            .await;
                    });
                })
            })
    }

    pub async fn new(
        event_tx: Sender<VpnEvent>,
        vpnc_script: &str,
        config_file: PathBuf,
    ) -> Result<Self, StateError> {
        let mut stored_configs = StoredConfigs::new(None, config_file);
        stored_configs.read_from_file().await?;
        Ok(Self {
            event_tx,
            client: RwLock::new(None),
            stored_configs: RwLock::new(stored_configs),
            vpnc_script: vpnc_script.to_string(),
            cert_prompts: Arc::new(CertPromptBroker::default()),
        })
    }
}
