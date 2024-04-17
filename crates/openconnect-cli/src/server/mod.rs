use crate::{
    client::state::get_vpnc_script,
    sock::{self, UnixDomainServer},
    JsonRequest, JsonResponse,
};
use futures::{SinkExt, TryStreamExt};
use openconnect_core::{
    config::{ConfigBuilder, EntrypointBuilder, LogLevel},
    events::EventHandlers,
    Connectable, Status, VpnClient,
};
use std::sync::Arc;
use tokio::{
    select,
    signal::unix::{signal, SignalKind},
    sync::RwLock,
};

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

impl Acceptable for Arc<State> {
    async fn try_accept(self) {
        if let Ok((stream, _)) = self.server.listener.accept().await {
            let (read, write) = stream.into_split();
            let mut framed_reader = sock::get_framed_reader::<JsonRequest>(read);
            let mut framed_writer = sock::get_framed_writer::<JsonResponse>(write);

            tokio::spawn(async move {
                while let Ok(Some(command)) = framed_reader.try_next().await {
                    match command {
                        JsonRequest::Start {
                            name: server_name,
                            server: server_url,
                            allow_insecure,
                            cookie,
                        } => {
                            let vpncscript = get_vpnc_script().unwrap();

                            let config = ConfigBuilder::default()
                                .vpncscript(&vpncscript)
                                .loglevel(LogLevel::Info)
                                .build()
                                .unwrap();

                            let entrypoint = EntrypointBuilder::new()
                                .name(&server_name)
                                .server(&server_url)
                                .accept_insecure_cert(allow_insecure)
                                .cookie(&cookie)
                                .enable_udp(true)
                                .build()
                                .unwrap();

                            let event_handler = EventHandlers::default();

                            let client = VpnClient::new(config, event_handler).unwrap();
                            let client_cloned = client.clone();

                            tokio::task::spawn_blocking(move || {
                                let _ = client.connect(entrypoint);
                            });

                            {
                                let mut client_to_write = self.client.write().await;
                                *client_to_write = Some(client_cloned);
                            }
                        }

                        JsonRequest::Stop => {
                            {
                                let client = self.client.read().await;
                                if let Some(ref client) = *client {
                                    let server_name =
                                        client.get_server_name().unwrap_or("".to_string());
                                    client.disconnect();

                                    // ignore send error
                                    let _ = framed_writer
                                        .send(JsonResponse::StopResult { server_name })
                                        .await;
                                    unsafe {
                                        libc::raise(libc::SIGTERM);
                                    }
                                }
                                self.client.write().await.take();
                            }
                        }

                        JsonRequest::Info => {
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
