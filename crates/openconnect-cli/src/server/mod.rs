use crate::{
    JsonRequest, JsonResponse,
    client::state::{StateError, get_vpnc_script},
    sock::{self, UnixDomainServer},
};
use futures::{SinkExt, TryStreamExt};
use openconnect_core::{
    Connectable, Status, VpnClient,
    config::{ConfigBuilder, EntrypointBuilder, LogLevel},
    events::EventHandlers,
};
use std::sync::Arc;
use tokio::{
    select,
    signal::unix::{SignalKind, signal},
    sync::RwLock,
};
use tokio_util::sync::CancellationToken;

struct State {
    client: RwLock<Option<Arc<VpnClient>>>,
    server: UnixDomainServer,
    shutdown: CancellationToken,
}

impl State {
    pub fn new(server: UnixDomainServer) -> Arc<Self> {
        Arc::new(State {
            client: RwLock::new(None),
            server,
            shutdown: CancellationToken::new(),
        })
    }
}

trait Acceptable {
    async fn try_accept(self);
}

async fn connect_to_vpn_server(
    name: &str,
    server: &str,
    allow_insecure: bool,
    cookie: &str,
) -> Result<Arc<VpnClient>, StateError> {
    let vpncscript = get_vpnc_script()?;

    let config = ConfigBuilder::default()
        .vpncscript(&vpncscript)
        .loglevel(LogLevel::Info)
        .build()?;

    let entrypoint = EntrypointBuilder::new()
        .name(name)
        .server(server)
        .accept_insecure_cert(allow_insecure)
        .cookie(cookie)
        .enable_udp(true)
        .build()?;

    let event_handler = EventHandlers::default();

    let client = VpnClient::new(config, event_handler)?;
    client.init_connection(entrypoint)?;

    let client_cloned = client.clone();
    tokio::task::spawn_blocking(move || {
        let _ = client_cloned.run_loop();
    });

    Ok(client)
}

impl Acceptable for Arc<State> {
    async fn try_accept(self) {
        if let Ok((stream, _)) = self.server.listener.accept().await {
            // Enforce peer credential authorization: only root (uid 0) or the invoking user may control the daemon.
            match stream.peer_cred() {
                Ok(cred) => {
                    let peer_uid = cred.uid();
                    if peer_uid != 0 && peer_uid != self.server.allowed_uid {
                        tracing::warn!(
                            "Rejected unauthorized Unix socket connection from uid {}",
                            peer_uid
                        );
                        return;
                    }
                }
                Err(err) => {
                    tracing::warn!(
                        "Failed to read peer credentials; rejecting connection: {}",
                        err
                    );
                    return;
                }
            }

            let (read, write) = stream.into_split();
            let mut framed_reader = sock::get_framed_reader::<JsonRequest>(read);
            let mut framed_writer = sock::get_framed_writer::<JsonResponse>(write);

            tokio::spawn(async move {
                while let Ok(Some(command)) = framed_reader.try_next().await {
                    match command {
                        JsonRequest::Start {
                            name,
                            server,
                            allow_insecure,
                            cookie,
                        } => {
                            tracing::debug!("Received start command, name: {}", name);
                            let connection_result =
                                connect_to_vpn_server(&name, &server, allow_insecure, &cookie)
                                    .await;

                            match connection_result {
                                Ok(client) => {
                                    {
                                        let mut client_to_write = self.client.write().await;
                                        *client_to_write = Some(client);
                                    }
                                    let _ = framed_writer
                                        .send(JsonResponse::StartResult {
                                            name,
                                            success: true,
                                            err_message: None,
                                        })
                                        .await;
                                }
                                Err(e) => {
                                    let _ = framed_writer
                                        .send(JsonResponse::StartResult {
                                            name,
                                            success: false,
                                            err_message: Some(e.to_string()),
                                        })
                                        .await;

                                    self.shutdown.cancel();
                                    break;
                                }
                            }
                        }

                        JsonRequest::Stop => {
                            tracing::debug!("Received stop command");
                            {
                                let client = self.client.read().await;
                                if let Some(ref client) = *client {
                                    let server_name = client.get_server_name().unwrap_or_default();
                                    client.disconnect();

                                    let _ = framed_writer
                                        .send(JsonResponse::StopResult { name: server_name })
                                        .await;
                                }
                            }
                            {
                                self.client.write().await.take();
                            }
                            self.shutdown.cancel();
                            break;
                        }

                        JsonRequest::Info => {
                            tracing::debug!("Received info command");
                            {
                                let client = self.client.read().await;
                                if let Some(client) = (*client).clone() {
                                    let server_name = client.get_server_name().unwrap_or_default();
                                    let server_url = client.get_server_url().unwrap_or_default();
                                    let hostname = client.get_hostname().unwrap_or_default();
                                    let status = client.get_status();
                                    let info = client.get_info().ok().flatten().map(Box::new);
                                    let status = match status {
                                        Status::Connected => "Connected",
                                        Status::Connecting(_) => "Connecting",
                                        Status::Disconnected => "Disconnected",
                                        Status::Disconnecting => "Disconnecting",
                                        Status::Error(_) => "Error",
                                        Status::Initialized => "Initialized",
                                    }
                                    .to_string();

                                    let _ = framed_writer
                                        .send(JsonResponse::InfoResult {
                                            server_name,
                                            server_url,
                                            hostname,
                                            status,
                                            info,
                                        })
                                        .await;
                                }
                            }
                        }
                    }
                }
            });
        }
    }
}

pub async fn start_daemon() -> anyhow::Result<()> {
    let server = UnixDomainServer::bind()?;
    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sigint = signal(SignalKind::interrupt())?;
    let mut sigquit = signal(SignalKind::quit())?;
    let state = State::new(server);

    loop {
        let state_cloned = state.clone();
        select! {
            _ = state.shutdown.cancelled() => {
                break;
            }
            _ = sigquit.recv() => {
                break;
            }
            _ = sigint.recv() => {
                break;
            }
            _ = sigterm.recv() => {
                break;
            }
            _ = state_cloned.try_accept() => {
                // noop
            }
        };
    }

    // Ensure any active client disconnects cleanly before exiting.
    if let Some(client) = state.client.write().await.take() {
        let _ = tokio::task::spawn_blocking(move || client.disconnect()).await;
    }

    Ok(())
}
