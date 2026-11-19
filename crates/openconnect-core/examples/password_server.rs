use openconnect_core::{
    Connectable, VpnClient,
    config::{ConfigBuilder, EntrypointBuilder, LogLevel},
    events::EventHandlers,
    protocols::get_anyconnect_protocol,
};
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::from_path(".env.local")?;

    let protocol = get_anyconnect_protocol()?;

    let config = ConfigBuilder::default().loglevel(LogLevel::Info).build()?;
    let event_handlers = EventHandlers::default();
    let client = VpnClient::new(config, event_handlers)?;

    let entrypoint = EntrypointBuilder::new()
        .server(&env::var("VPN_SERVER")?)
        .username(&env::var("VPN_USERNAME")?)
        .password(&env::var("VPN_PASSWORD")?)
        .protocol(protocol)
        .enable_udp(true)
        .accept_insecure_cert(true)
        .build()?;

    client.init_connection(entrypoint)?;
    client.run_loop()?;

    Ok(())
}
