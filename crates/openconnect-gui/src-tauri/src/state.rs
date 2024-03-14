use crate::oidc::{OpenID, OpenIDConfig};
use openconnect_core::{
    config::{ConfigBuilder, EntrypointBuilder, LogLevel},
    events::EventHandlers,
    Connectable, Status, VpnClient,
};
use std::env;
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
            Status::Connecting => ("connecting".to_string(), None),
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
}

impl AppState {
    pub fn handle(app: &mut tauri::App) {
        let (event_tx, mut event_rx) = channel::<VpnEvent>(100);
        let app_state = AppState::new(event_tx);
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

    pub async fn connect_with_user_pass(
        &self,
        server: &str,
        username: &str,
        password: &str,
    ) -> anyhow::Result<()> {
        let config = ConfigBuilder::default().loglevel(LogLevel::Info).build()?;

        let entrypoint = EntrypointBuilder::new()
            .server(server)
            .username(username)
            .password(password)
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

    pub async fn connect_with_oidc(&self, server: &str) -> anyhow::Result<()> {
        let openid_config = OpenIDConfig {
            issuer_url: env::var("OIDC_ISSUER")?,
            redirect_uri: env::var("OIDC_REDIRECT_URI")?,
            client_id: env::var("OIDC_CLIENT_ID")?,
            client_secret: Some(env::var("OIDC_CLIENT_SECRET")?),
        };

        let openid = OpenID::new(openid_config).await?;
        let (authorize_url, req_state, _) = openid.auth_request()?;
        println!("Connecting with OpenID");

        println!();

        open::that(authorize_url.to_string())?;
        let (code, callback_state) = openid.wait_for_callback().await?;

        if req_state.secret() != callback_state.secret() {
            return Err(anyhow::anyhow!("Invalid state"));
        }

        let token = openid.exchange_token(code).await?;
        let cookie = openid
            .obtain_cookie_by_oidc(server, &token)
            .await
            .ok_or(anyhow::anyhow!("Failed to obtain cookie from server"))?;

        let config = ConfigBuilder::default().loglevel(LogLevel::Info).build()?;

        let entrypoint = EntrypointBuilder::new()
            .server(server)
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
            client.disconnect();
        }

        self.client.write().await.take(); // drop the client

        Ok(())
    }

    pub fn new(event_tx: Sender<VpnEvent>) -> Self {
        Self {
            event_tx,
            client: RwLock::new(None),
        }
    }
}
