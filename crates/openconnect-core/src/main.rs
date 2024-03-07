use openconnect_core::{
    config::ConfigBuilder, protocols, LogLevel, VpnClient, RECONNECT_INTERVAL_MIN,
};
use std::env;

fn main() -> anyhow::Result<()> {
    dotenvy::from_path(".env.local").unwrap();
    env::set_var("OPENSSL_CONF", "/dev/null");

    let config = ConfigBuilder::default()
        .username(&env::var("VPN_USERNAME").unwrap())
        .password(&env::var("VPN_PASSWORD").unwrap())
        .server(&env::var("VPN_SERVER").unwrap())
        .build();

    let client = VpnClient::new(config);

    client.set_loglevel(LogLevel::Info);

    let proxy = env::var("https_proxy").unwrap_or("".to_string());
    if !proxy.is_empty() {
        client.set_http_proxy(&proxy)?;
    }

    client.setup_cmd_pipe()?;

    client.set_stats_handler();

    let protocols = protocols::get_supported_protocols();
    let selected = protocols.first();
    if let Some(selected) = selected {
        client.set_protocol(&selected.name)?;
    }

    client.set_setup_tun_handler();
    client.set_report_os("linux-64")?;

    let disable_udp = false;
    if disable_udp {
        client.disable_dtls()?;
    }

    client.parse_url(client.config.server.clone())?;
    let hostname = client.get_hostname();
    if let Some(hostname) = hostname {
        println!("connecting: {}", hostname);
    }

    client.obtain_cookie()?;
    client.make_cstp_connection()?;

    let cloned_client = client.clone();

    // setup graceful shutdown
    ctrlc::set_handler(move || {
        cloned_client.stop_main_loop();
    })?;

    loop {
        if client.main_loop(300, RECONNECT_INTERVAL_MIN).is_err() {
            break;
        }
    }

    Ok(())
}
