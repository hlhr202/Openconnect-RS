use crate::{
    client::state::{get_vpnc_script, StateError},
    sock::UnixDomainServer,
    JsonRequest, JsonResponse,
};
use futures::{SinkExt, TryStreamExt};
use openconnect_core::{
    config::{ConfigBuilder, EntrypointBuilder, LogLevel},
    events::EventHandlers,
    Connectable, Status, VpnClient,
};
use std::sync::Arc;
use tokio::sync::RwLock;

#[cfg(not(target_os = "windows"))]
use tokio::signal::unix::{signal, SignalKind};

struct State {
    client: RwLock<Option<Arc<VpnClient>>>,
    server: UnixDomainServer,
}

impl State {
    pub fn new(server: UnixDomainServer) -> Arc<Self> {
        Arc::new(State {
            client: RwLock::new(None),
            server,
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
        if let Ok((mut framed_reader, mut framed_writer)) = self.server.accept().await {
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

                                    unsafe {
                                        libc::raise(libc::SIGTERM);
                                    }
                                }
                            }
                        }

                        JsonRequest::Stop => {
                            tracing::debug!("Received stop command");
                            {
                                let client = self.client.read().await;
                                if let Some(ref client) = *client {
                                    let server_name =
                                        client.get_server_name().unwrap_or("".to_string());
                                    client.disconnect();

                                    // ignore send error
                                    let _ = framed_writer
                                        .send(JsonResponse::StopResult { name: server_name })
                                        .await;
                                }
                            }
                            {
                                self.client.write().await.take();
                            }
                            unsafe {
                                libc::raise(libc::SIGTERM);
                            }
                        }

                        JsonRequest::Info => {
                            tracing::debug!("Received info command");
                            {
                                let client = self.client.read().await;
                                if let Some(client) = (*client).clone() {
                                    let server_name =
                                        client.get_server_name().unwrap_or("".to_string());
                                    let server_url =
                                        client.get_server_url().unwrap_or("".to_string());
                                    let hostname = client.get_hostname().unwrap_or("".to_string());
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

                                    // ignore send error
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

#[cfg(not(target_os = "windows"))]
pub async fn start_daemon() -> anyhow::Result<()> {
    let server = UnixDomainServer::bind()?;
    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sigint = signal(SignalKind::interrupt())?;
    let mut sigquit = signal(SignalKind::quit())?;
    let state = State::new(server);

    loop {
        let state = state.clone();
        select! {
            _ = sigquit.recv() => {
                break;
            }
            _ = sigint.recv() => {
                break;
            }
            _ = sigterm.recv() => {
                break;
            }
            _ = state.try_accept() => {
                // noop
            }
        };
    }

    Ok(())
}

// TODO: Implement this function for Windows
#[cfg(target_os = "windows")]
pub async fn start_daemon() -> anyhow::Result<()> {
    let server = UnixDomainServer::bind()?;
    let state = State::new(server);

    state.try_accept().await;

    Ok(())
}
