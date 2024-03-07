use openconnect_core::*;
use openconnect_sys::*;
use std::env;

fn main() -> anyhow::Result<()> {
    dotenvy::from_path(".env.local").unwrap();
    env::set_var("OPENSSL_CONF", "/dev/null");

    let ctx = OpenconnectCtx::new();

    ctx.set_loglevel(LogLevel::Info);

    let proxy = env::var("https_proxy").unwrap_or("".to_string());
    if !proxy.is_empty() {
        ctx.set_http_proxy(&proxy)?;
    }

    ctx.setup_cmd_pipe()?;

    ctx.set_stats_handler();

    let protocols = protocols::get_supported_protocols();
    let selected = protocols.first();
    if let Some(selected) = selected {
        ctx.set_protocol(&selected.name)?;
    }

    ctx.set_setup_tun_handler();

    println!("VPN server: {}", env::var("VPN_SERVER").unwrap());
    ctx.set_report_os("linux-64")?;

    let disable_udp = false;
    if disable_udp {
        ctx.disable_dtls()?;
    }

    ctx.parse_url(&ctx.server)?;
    let hostname = ctx.get_hostname();
    if let Some(hostname) = hostname {
        println!("connecting: {}", hostname);
    }

    ctx.obtain_cookie()?;
    ctx.make_cstp_connection()?;

    let ctx_clone = ctx.clone();

    // setup graceful shutdown
    ctrlc::set_handler(move || {
        ctx_clone.stop_main_loop();
    })?;

    loop {
        if ctx.main_loop(300, RECONNECT_INTERVAL_MIN).is_err() {
            break;
        }
    }

    Ok(())
}
