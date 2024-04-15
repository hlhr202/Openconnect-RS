mod cli;
mod config;
mod daemon;
mod sock;
mod state;

use crate::sock::UnixDomainServer;
use clap::Parser;
use cli::{Cli, Commands};
use futures::{SinkExt, TryStreamExt};
use openconnect_core::{
    ip_info::IpInfo,
    log::Logger,
    storage::{StoredConfigs, StoredServer},
    Connectable, Status, VpnClient,
};
use std::{error::Error, io::BufRead, path::PathBuf, sync::Arc};
use tokio::{
    select,
    signal::unix::{signal, SignalKind},
};

#[derive(serde::Serialize, serde::Deserialize)]
pub enum JsonRequest {
    Stop,
    Info,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub enum JsonResponse {
    StopResult {
        server_name: String,
    },
    InfoResult {
        server_name: String,
        server_url: String,
        hostname: String,
        status: String,
        info: Option<Box<IpInfo>>,
    },
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

async fn start_daemon(
    stored_server: &StoredServer,
    stored_configs: &StoredConfigs,
) -> Result<(), Box<dyn Error>> {
    let server = UnixDomainServer::bind()?;
    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sigint = signal(SignalKind::interrupt())?;
    let mut sigquit = signal(SignalKind::quit())?;

    let client = match stored_server {
        StoredServer::Password(password_server) => {
            crate::state::connect_password_server(password_server, stored_configs).await?
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

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Add(server_config) => {
            crate::config::request_add_server(server_config);
        }

        Commands::Import { base64 } => {
            crate::config::request_import_server(&base64);
        }

        Commands::Export { name } => {
            crate::config::request_export_server(&name);
        }

        Commands::Delete { name } => {
            crate::config::request_delete_server(&name);
        }

        Commands::List => {
            crate::config::request_list_servers();
        }

        Commands::Status => {
            crate::state::request_get_status();
        }

        Commands::Logs => {
            let log_path = Logger::get_log_path();
            let files = std::fs::read_dir(log_path)
                .expect("Failed to read log directory")
                .flatten()
                .filter(|f| f.metadata().unwrap().is_file())
                .max_by_key(|f| f.metadata().unwrap().modified().unwrap());

            if let Some(file) = files {
                let file = std::fs::File::open(file.path()).expect("Failed to open log file");
                let reader = std::io::BufReader::new(file);
                for line in reader.lines() {
                    println!("{}", line.unwrap());
                }
            }
        }

        Commands::Stop => {
            crate::state::request_stop_server();
        }

        Commands::Start { name, config_file } => {
            if sock::exists() {
                eprintln!("Socket already exists. You may have a connected VPN session or a stale socket file. You may solve by:");
                eprintln!("1. Stopping the connection by sending stop command.");
                eprintln!(
                    "2. Manually deleting the socket file which located at: {}",
                    sock::get_sock().display()
                );
                std::process::exit(1);
            }

            let config_file = config_file.map(PathBuf::from).unwrap_or(
                StoredConfigs::getorinit_config_file().expect("Failed to get config file"),
            );

            sudo::escalate_if_needed().expect("Failed to escalate permissions");
            match daemon::daemonize() {
                daemon::ForkResult::Parent => {
                    let runtime = tokio::runtime::Runtime::new().expect("Failed to create runtime");
                    runtime.block_on(async {
                        match crate::config::read_server_config_from_fs(&name, config_file).await {
                            Ok(_) => {}
                            Err(e) => {
                                eprintln!("Failed to get server: {}", e);
                                std::process::exit(1);
                            }
                        }
                    });
                    println!("The process will be running in the background, you should use cli to interact with it.");
                    std::process::exit(0);
                }
                daemon::ForkResult::Child => {
                    std::process::exit(0);
                }
                daemon::ForkResult::Grandchild => {
                    // Daemon process
                }
            }

            let runtime = tokio::runtime::Runtime::new().expect("Failed to create runtime");

            let (server, configs) = runtime.block_on(async {
                crate::config::read_server_config_from_fs(&name, config_file)
                    .await
                    .expect("Failed to get server")
            });

            runtime.block_on(async {
                Logger::init().expect("Failed to initialize logger");
                let _ = start_daemon(&server, &configs).await.inspect_err(|e| {
                    tracing::error!("Failed to start daemon: {}", e);
                });
            });
        }
    }
}
