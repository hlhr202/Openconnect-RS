use crate::{
    sock::{self, UnixDomainServer},
    JsonRequest, JsonResponse,
};
use futures::{SinkExt, TryStreamExt};
use openconnect_core::{
    storage::{StoredConfigs, StoredServer},
    Connectable, Status, VpnClient,
};
use std::{error::Error, sync::Arc};
use tokio::{
    select,
    signal::unix::{signal, SignalKind},
};

pub async fn try_accept(listener: &tokio::net::UnixListener, client: Arc<VpnClient>) {
    if let Ok((stream, _)) = listener.accept().await {
        let (read, write) = stream.into_split();
        let mut framed_reader = sock::get_framed_reader::<JsonRequest>(read);
        let mut framed_writer = sock::get_framed_writer::<JsonResponse>(write);

        tokio::spawn(async move {
            while let Ok(Some(command)) = framed_reader.try_next().await {
                match command {
                    JsonRequest::Stop => {
                        let server_name = client.get_server_name().unwrap_or("".to_string());
                        client.disconnect();

                        // ignore send error
                        let _ = framed_writer
                            .send(JsonResponse::StopResult { server_name })
                            .await;
                        unsafe {
                            libc::raise(libc::SIGTERM);
                        }
                    }

                    JsonRequest::Info => {
                        let server_name = client.get_server_name().unwrap_or("".to_string());
                        let server_url = client.get_server_url().unwrap_or("".to_string());
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
        });
    }
}

pub async fn start_daemon(
    stored_server: &StoredServer,
    stored_configs: &StoredConfigs,
) -> Result<(), Box<dyn Error>> {
    let server = UnixDomainServer::bind()?;
    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sigint = signal(SignalKind::interrupt())?;
    let mut sigquit = signal(SignalKind::quit())?;

    let client = match stored_server {
        StoredServer::Password(password_server) => {
            crate::client::state::connect_password_server(password_server, stored_configs).await?
        }
        StoredServer::Oidc(_) => {
            panic!("OIDC server not implemented");
        }
    };

    loop {
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
            _ = try_accept(&server.listener, client.clone()) => {
                // noop
            }
        };
    }

    Ok(())
}
