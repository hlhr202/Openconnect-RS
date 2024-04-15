use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};

#[derive(Parser, Debug)]
#[clap(
    name = env!("CARGO_PKG_NAME"),
    version = env!("CARGO_PKG_VERSION"),
    long_about = env!("CARGO_PKG_DESCRIPTION"),
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
    Add(SeverConfigArgs),

    #[command(about = "Import VPN server configurations from a base64 encoded string")]
    Import {
        /// The base64 encoded string of the VPN server configurations
        base64: String,
    },

    #[command(about = "Export VPN server configurations to a base64 encoded string")]
    Export {
        /// The name of the VPN server configuration to export
        name: String,
    },

    #[command(about = "Delete a VPN server configuration from local config file", visible_aliases = ["rm", "remove", "del"])]
    Delete {
        /// The server name saved in local config file to delete
        name: String,
    },

    #[command(about = "List all VPN server configurations in local config file", visible_aliases = ["ls", "l"])]
    List,

    #[command(about = "Show logs of the daemon process", visible_aliases = ["log"])]
    Logs,

    #[command(about = "Generate shell completion script")]
    GenComplete { generator: Shell },
}

#[derive(Subcommand, Debug)]
pub enum SeverConfigArgs {
    #[command(long_about = "Add an OIDC authentication VPN server")]
    Oidc {
        /// The unique name of the VPN server configuration
        #[arg(short, long)]
        name: String,

        /// The VPN server URL
        #[arg(short, long, value_hint = clap::ValueHint::Url)]
        server: String,

        /// The OIDC issuer URL
        #[arg(short = 'I', long)]
        issuer: String,

        /// The OIDC client ID
        #[arg(short = 'i', long)]
        client_id: String,

        /// The OIDC client secret
        #[arg(short = 'k', long)]
        client_secret: Option<String>,

        /// Allow insecure peer certificate verification
        #[arg(short, long, default_value = "false")]
        allow_insecure: Option<bool>,
    },

    #[command(
        long_about = "Add a password authentication VPN server. For safty reason, the password input will be prompted in terminal later"
    )]
    Password {
        /// The unique name of the VPN server configuration
        #[arg(short, long)]
        name: String,

        /// The VPN server URL
        #[arg(short, long, value_hint = clap::ValueHint::Url)]
        server: String,

        /// The username for password authentication
        #[arg(short, long)]
        username: String,

        /// Allow insecure peer certificate verification
        #[arg(short, long, default_value = "false")]
        allow_insecure: Option<bool>,
    },
}

pub fn print_completions(generator: Shell) {
    let mut cmd = Cli::command();
    let cmd = &mut cmd;

    generate(
        generator,
        cmd,
        cmd.get_name().to_string(),
        &mut std::io::stdout(),
    );
}
