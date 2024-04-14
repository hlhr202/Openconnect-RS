use openconnect_core::{
    config::{ConfigBuilder, EntrypointBuilder, LogLevel},
    events::EventHandlers,
    storage::{PasswordServer, StoredConfigs},
    Connectable, VpnClient,
};
use std::{error::Error, sync::Arc};

pub async fn connect_password_server(
    password_server: &PasswordServer,
    stored_configs: &StoredConfigs,
) -> Result<Arc<VpnClient>, Box<dyn Error>> {
    let password_server = password_server.decrypted_by(&stored_configs.cipher);
    let homedir = home::home_dir().ok_or("Failed to get home directory")?;
    let vpncscript = homedir.join(".oidcvpn/bin/vpnc-script");
    let vpncscript = vpncscript.to_str().ok_or("Failed to get vpncscript path")?;

    let config = ConfigBuilder::default()
        .vpncscript(vpncscript)
        .loglevel(LogLevel::Info)
        .build()?;

    let entrypoint = EntrypointBuilder::new()
        .name(&password_server.name)
        .server(&password_server.server)
        .username(&password_server.username)
        .password(&password_server.password.clone().unwrap_or("".to_string()))
        .accept_insecure_cert(password_server.allow_insecure.unwrap_or(false))
        .enable_udp(true)
        .build()?;

    let event_handler = EventHandlers::default();

    let client = VpnClient::new(config, event_handler)?;
    let client_clone = client.clone();

    tokio::task::spawn_blocking(move || {
        let _ = client_clone.connect(entrypoint);
    });

    Ok(client)
}
