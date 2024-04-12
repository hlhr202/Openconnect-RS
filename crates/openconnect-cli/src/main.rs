mod daemon;
mod sock;

use crate::sock::Server;
use clap::{Parser, Subcommand};
use futures::{SinkExt, TryStreamExt};
use openconnect_core::{
    config::{ConfigBuilder, EntrypointBuilder, LogLevel},
    events::EventHandlers,
    log::Logger,
    storage::StoredConfigs,
    Connectable, Status, VpnClient,
};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{
    select,
    signal::unix::{signal, SignalKind},
};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Connect {
        server_name: String,
        config_file: Option<String>,
    },
    Info,
    Stop,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub enum JsonRequest {
    Stop,
    Info,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub enum JsonResponse {
    StopResult,
    InfoResult { uid: u32, status: String },
}

async fn connect(server_name: &str, config_file: &Path) -> Arc<VpnClient> {
    Logger::init().unwrap();

    let mut stored_server = StoredConfigs::new(None, config_file.to_path_buf());
    let config = stored_server.read_from_file().await.unwrap();
    let password_server = config.get_server_as_password_server(server_name).unwrap();
    let password_server = &password_server.decrypted_by(&config.cipher);

    let config = ConfigBuilder::default()
        .vpncscript("/opt/vpnc-scripts/vpnc-script")
        .loglevel(LogLevel::Info)
        .build()
        .unwrap();

    let entrypoint = EntrypointBuilder::new()
        .name(&password_server.name)
        .server(&password_server.server)
        .username(&password_server.username)
        .password(&password_server.password.clone().unwrap_or("".to_string()))
        .accept_insecure_cert(password_server.allow_insecure.unwrap_or(false))
        .enable_udp(true)
        .build()
        .unwrap();

    let event_handler = EventHandlers::default();

    let client = VpnClient::new(config, event_handler).unwrap();
    let client_clone = client.clone();

    tokio::task::spawn_blocking(move || {
        let _ = client_clone.connect(entrypoint);
    });

    client
}

async fn try_accept(listener: &tokio::net::UnixListener, client: Arc<VpnClient>) {
    if let Ok((stream, _)) = listener.accept().await {
        let (read, write) = stream.into_split();
        let mut framed_reader = sock::get_framed_reader::<JsonRequest>(read);
        let mut framed_writer = sock::get_framed_writer::<JsonResponse>(write);

        tokio::spawn(async move {
            while let Ok(Some(command)) = framed_reader.try_next().await {
                match command {
                    JsonRequest::Stop => {
                        println!("Stopping");
                        client.disconnect();
                        framed_writer.send(JsonResponse::StopResult).await.unwrap();
                        unsafe {
                            libc::raise(libc::SIGTERM);
                        }
                    }

                    JsonRequest::Info => {
                        let uid = unsafe { libc::getuid() };
                        let status = client.get_status();
                        let status = match status {
                            Status::Connected => "Connected",
                            Status::Connecting(_) => "Connecting",
                            Status::Disconnected => "Disconnected",
                            Status::Disconnecting => "Disconnecting",
                            Status::Error(_) => "Error",
                            Status::Initialized => "Initialized",
                        }
                        .to_string();

                        framed_writer
                            .send(JsonResponse::InfoResult { uid, status })
                            .await
                            .unwrap();
                        println!("Info");
                    }
                }
            }
        });
    }
}

fn start_daemon(server_name: &str, config_file: &Path) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let server = Server::bind().unwrap();
        let mut sigterm = signal(SignalKind::terminate()).unwrap();
        let mut sigint = signal(SignalKind::interrupt()).unwrap();
        let mut sigquit = signal(SignalKind::quit()).unwrap();

        let client = connect(server_name, config_file).await;

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

                }
            };
        }

        println!("Exiting")
    });
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Info => {
            let runtime = tokio::runtime::Runtime::new().unwrap();

            runtime.block_on(async {
                let mut client = sock::Client::connect().await.unwrap();
                client.send(JsonRequest::Info).await.unwrap();

                if let Ok(Some(response)) = client.framed_reader.try_next().await {
                    match response {
                        JsonResponse::InfoResult { uid, status } => {
                            println!("Received uid: {}", uid);
                            println!("Received status: {}", status);
                        }
                        _ => {
                            println!("Received unexpected response");
                        }
                    }
                }
            });
        }
        Commands::Stop => {
            let runtime = tokio::runtime::Runtime::new().unwrap();

            runtime.block_on(async {
                let mut client = sock::Client::connect().await.unwrap();
                client.send(JsonRequest::Stop).await.unwrap();

                if let Ok(Some(response)) = client.framed_reader.try_next().await {
                    match response {
                        JsonResponse::StopResult => {
                            println!("Received stop result");
                        }
                        _ => {
                            println!("Received unexpected response");
                        }
                    }
                }
            });
        }
        Commands::Connect {
            server_name,
            config_file,
        } => {
            if sock::exists() {
                println!("Socket already exists, exiting");
                std::process::exit(1);
            }

            let config_file = config_file
                .map(PathBuf::from)
                .unwrap_or(StoredConfigs::getorinit_config_file().unwrap());

            sudo::escalate_if_needed().expect("Failed to escalate permissions");
            daemon::daemonize();
            start_daemon(&server_name, &config_file);
        }
    }
}
