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
    #[command(about = "Connect to a VPN server and run in daemon mode", visible_aliases = ["connect", "run"])]
    Start {
        /// The server name saved in local config file to connect to
        name: String,

        /// The path to the local config file
        #[arg(short, long, value_hint = clap::ValueHint::FilePath)]
        config_file: Option<String>,
    },

    #[command(about = "Get the current VPN connection status", visible_aliases = ["info", "stat"])]
    Status,

    #[command(about = "Close the current connection and exit the daemon process", visible_aliases = ["kill", "disconnect"])]
    Stop,

    #[command(
        subcommand,
        about = "Add new VPN server configuration to local config file",
        visible_aliases = ["new", "create", "insert"]
    )]
    Add(ServerType),

    #[command(about = "Delete a VPN server configuration from local config file", visible_aliases = ["rm", "remove", "del"])]
    Delete {
        /// The server name saved in local config file to delete
        name: String,
    },

    #[command(about = "List all VPN server configurations in local config file", visible_aliases = ["ls", "l"])]
    List,

    #[command(about = "Show logs of the daemon process", visible_aliases = ["log"])]
    Logs,
}

#[derive(Subcommand, Debug)]
pub enum ServerType {
    #[command(about = "Add an OIDC authentication VPN server")]
    Oidc {
        #[arg(short, long)]
        name: String,

        #[arg(short, long, value_hint = clap::ValueHint::Url)]
        server: String,

        #[arg(short = 'I', long)]
        issuer: String,

        #[arg(short = 'i', long)]
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

        #[arg(short, long, value_hint = clap::ValueHint::Url)]
        server: String,

        #[arg(short, long)]
        username: String,

        #[arg(short, long)]
        password: String,

        #[arg(short, long, default_value = "false")]
        allow_insecure: Option<bool>,
    },
}
