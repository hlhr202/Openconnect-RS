use crate::oidc::{OpenID, OpenIDConfig, OIDC_REDIRECT_URI};
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
            Status::Initialized => ("initialized".to_string(), None),
            Status::Connecting(msg) => ("connecting".to_string(), Some(msg)),
            Status::Connected => ("connected".to_string(), None),
            Status::Disconnecting => ("disconnecting".to_string(), None),
            Status::Disconnected => ("disconnected".to_string(), None),
            Status::Error(err) => ("error".to_string(), Some(err.to_string())),
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
    ) -> anyhow::Result<()> {
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

    pub async fn trigger_state_retrieve(&self) -> anyhow::Result<()> {
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

    pub async fn connect_with_user_pass(&self, server_name: &str) -> anyhow::Result<()> {
        let mut stored_server = StoredConfigs::default();
        let stored_server = stored_server
            .read_from_file()
            .await?
            .get_server_as_password(server_name)
            .ok_or(anyhow::anyhow!("Server not found"))?;

        let mut config = ConfigBuilder::default();

        #[cfg(not(target_os = "windows"))]
        let mut config = config.vpncscript(&self.vpnc_sciprt);

        let config = config.loglevel(LogLevel::Info).build()?;

        let entrypoint = EntrypointBuilder::new()
            .server(&stored_server.server)
            .username(&stored_server.username)
            .password(&stored_server.password)
            .enable_udp(true)
            .build()?;

        let event_tx = self.event_tx.clone();

        let event_handlers =
            EventHandlers::default().with_handle_connection_state_change(move |state| {
                let event_tx = event_tx.clone();
                tauri::async_runtime::spawn(async move {
                    event_tx
                        .send(VpnEvent::Status(state.into()))
                        .await
                        .unwrap_or_default();
                });
            });

        let client = VpnClient::new(config, event_handlers)?;
        self.client.write().await.replace(client.clone());

        tauri::async_runtime::spawn_blocking(move || {
            client.connect(entrypoint).unwrap();
        });

        Ok(())
    }

    pub async fn connect_with_oidc(&self, server_name: &str) -> anyhow::Result<()> {
        let mut stored_server = StoredConfigs::default();
        let stored_server = stored_server
            .read_from_file()
            .await?
            .get_server_as_oidc(server_name)
            .ok_or(anyhow::anyhow!("Server not found"))?;

        let openid_config = OpenIDConfig {
            issuer_url: stored_server.issuer.clone(),
            redirect_uri: OIDC_REDIRECT_URI.to_string(),
            client_id: stored_server.client_id.clone(),
            client_secret: stored_server.client_secret.clone(),
        };

        let openid = OpenID::new(openid_config).await?;
        let (authorize_url, req_state, _) = openid.auth_request()?;

        open::that(authorize_url.to_string())?;
        let (code, callback_state) = openid.wait_for_callback().await?;

        if req_state.secret() != callback_state.secret() {
            return Err(anyhow::anyhow!("Invalid state"));
        }

        let token = openid.exchange_token(code).await?;
        let cookie = openid
            .obtain_cookie_by_oidc(&stored_server.server, &token)
            .await
            .ok_or(anyhow::anyhow!("Failed to obtain cookie from server"))?;

        let mut config = ConfigBuilder::default();

        #[cfg(not(target_os = "windows"))]
        let mut config = config.vpncscript(&self.vpnc_sciprt);

        let config = config.loglevel(LogLevel::Info).build()?;

        let entrypoint = EntrypointBuilder::new()
            .server(&stored_server.server)
            .cookie(&cookie)
            .build()?;

        let event_tx = self.event_tx.clone();

        let event_handlers =
            EventHandlers::default().with_handle_connection_state_change(move |state| {
                let event_tx = event_tx.clone();
                tauri::async_runtime::spawn(async move {
                    event_tx
                        .send(VpnEvent::Status(state.into()))
                        .await
                        .unwrap_or_default();
                });
            });

        let client = VpnClient::new(config, event_handlers)?;
        self.client.write().await.replace(client.clone());

        tauri::async_runtime::spawn_blocking(move || {
            client.connect(entrypoint).unwrap();
        });

        Ok(())
    }

    pub async fn disconnect(&self) -> anyhow::Result<()> {
        if let Some(client) = self.client.read().await.as_ref() {
            let client = client.clone();
            tauri::async_runtime::spawn_blocking(move || client.disconnect()).await?;
        }

        // self.client.write().await.take(); // TODO: wait a few seconds and drop the client

        Ok(())
    }

    pub async fn new(event_tx: Sender<VpnEvent>, vpnc_scipt: &str) -> anyhow::Result<Self> {
        let mut stored_configs = StoredConfigs::new();
        stored_configs.read_from_file().await?;
        Ok(Self {
            event_tx,
            client: RwLock::new(None),
            stored_configs: RwLock::new(stored_configs),
            vpnc_sciprt: vpnc_scipt.to_string(),
        })
    }
}
