use openconnect_core::{protocols::get_supported_protocols, *};
use openconnect_sys::*;
use std::env;

fn main() -> anyhow::Result<()> {
    dotenvy::from_path(".env.local").unwrap();

    unsafe {
        env::set_var("OPENSSL_CONF", "/dev/null");

        let ctx = OpenconnectCtx::new(); // TODO: optimize when all methods are implemented
        let vpninfo = **ctx;

        ctx.set_loglevel(LogLevel::Info);

        let proxy = env::var("https_proxy").unwrap_or("".to_string());
        if !proxy.is_empty() {
            ctx.set_http_proxy(&proxy)?;
        }

        ctx.setup_cmd_pipe()?;

        openconnect_set_stats_handler(vpninfo, Some(OpenconnectCtx::stats_fn));

        let protocols = get_supported_protocols();
        let selected = protocols.first();
        if let Some(selected) = selected {
            ctx.set_protocol(&selected.name)?;
        }

        // let setup_tun_vfn = Box::into_raw(Box::new(OpenconnectCtx::setup_tun_vfn));
        // openconnect_set_setup_tun_handler(vpninfo, Some(*setup_tun_vfn));

        println!("VPN server: {}", env::var("VPN_SERVER").unwrap());
        ctx.set_report_os("linux-64")?;


        let disable_udp = false;
        if disable_udp {
            ctx.disable_dtls()?;
        }

        let server = env::var("VPN_SERVER").unwrap();
        ctx.parse_url(&server)?;
        println!("port: {}", ctx.get_port());
        let hostname = ctx.get_hostname();
        if let Some(hostname) = hostname {
            println!("hostname: {}", hostname);
        }

        ctx.obtain_cookie()?;
        ctx.make_cstp_connection()?;

        // 'main: loop {
        //     if ctx.main_loop(300, RECONNECT_INTERVAL_MIN).is_err() {
        //         break 'main;
        //     }
        // }

        Ok(())
    }
}
