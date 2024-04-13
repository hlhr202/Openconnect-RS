use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[clap(
    name = env!("CARGO_PKG_NAME"),
    version = env!("CARGO_PKG_VERSION"),
    about = env!("CARGO_PKG_DESCRIPTION"),
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(about = "Connect to a VPN server")]
    Connect {
        /// The server name saved in local config file to connect to
        #[arg(short, long)]
        server_name: String,

        /// The path to the local config file
        #[arg(short, long)]
        config_file: Option<String>,
    },

    #[command(about = "Get the current VPN connection status")]
    Info,

    #[command(about = "Close the current connection and exit the daemon process")]
    Stop,

    #[command(
        subcommand,
        about = "Add new VPN server configuration to local config file"
    )]
    Add(ServerType),
}

#[derive(Subcommand, Debug)]
pub enum ServerType {
    #[command(about = "Add an OIDC authentication VPN server")]
    Oidc {
        #[arg(short, long)]
        name: String,

        #[arg(short, long)]
        server: String,

        #[arg(short, long)]
        issuer: String,

        #[arg(short = 'c', long)]
        client_id: String,

        #[arg(short = 'k', long)]
        client_secret: Option<String>,

        #[arg(short, long, default_value = "false")]
        allow_insecure: Option<bool>,
    },

    #[command(about = "Add a password authentication VPN server")]
    Password {
        #[arg(short, long)]
        name: String,

        #[arg(short, long)]
        server: String,

        #[arg(short, long)]
        username: String,

        #[arg(short, long)]
        password: String,

        #[arg(short, long, default_value = "false")]
        allow_insecure: Option<bool>,
    },
}
