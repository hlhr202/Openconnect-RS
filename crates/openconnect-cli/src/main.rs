mod cli;
mod client;
mod daemon;
mod server;
mod sock;

use clap::Parser;
use cli::{Cli, Commands};
use openconnect_core::{ip_info::IpInfo, log::Logger, storage::StoredConfigs};
use std::{io::BufRead, path::PathBuf};

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub enum JsonRequest {
    Start {
        name: String,
        server: String,
        allow_insecure: bool,
        cookie: String,
    },
    Stop,
    Info,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub enum JsonResponse {
    StartResult {
        name: String,
        success: bool,
        err_message: Option<String>,
    },
    StopResult {
        name: String,
    },
    InfoResult {
        server_name: String,
        server_url: String,
        hostname: String,
        status: String,
        info: Option<Box<IpInfo>>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::GenComplete {
            generator,
            binary_name,
        } => {
            crate::cli::print_completions(generator, binary_name);
        }
        Commands::Add(server_config) => {
            crate::client::config::request_add_server(server_config);
        }

        Commands::Import { base64 } => {
            crate::client::config::request_import_server(&base64);
        }

        Commands::Export { name } => {
            crate::client::config::request_export_server(&name);
        }

        Commands::Delete { name } => {
            crate::client::config::request_delete_server(&name);
        }

        Commands::List => {
            crate::client::config::request_list_servers();
        }

        Commands::Status => {
            crate::client::state::request_get_status();
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
            } else {
                println!("No log files found");
            }
        }

        Commands::Stop => {
            crate::client::state::request_stop_server();
        }

        Commands::Start { name, config_file } => {
            sock::exit_when_socket_exists();

            #[cfg(target_os = "macos")]
            sudo::escalate_if_needed().expect("Failed to escalate permissions");

            #[cfg(target_os = "linux")]
            sudo::with_env(&["HOME"]).expect("Failed to escalate permissions"); // keep HOME env so that we can find the config file and vpnc script

            let config_file = config_file.map(PathBuf::from).unwrap_or(
                StoredConfigs::getorinit_config_file().expect("Failed to get config file"),
            );

            match daemon::daemonize() {
                daemon::ForkResult::Parent => {
                    println!();
                    println!("===============================\n");
                    println!("OpenConnect VPN CLI Client\n");
                    println!("===============================\n");
                    println!("Using Config file: {:?}", config_file);
                    crate::client::state::request_start_server(name, config_file);
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

            runtime.block_on(async {
                Logger::init().expect("Failed to initialize logger");
                let start_result = crate::server::start_daemon().await;
                if let Err(e) = start_result {
                    tracing::error!("Failed to start daemon: {}", e);
                }
            });
        }
    }
}
