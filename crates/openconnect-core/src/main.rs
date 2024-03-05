use openconnect_core::*;
use openconnect_sys::*;
use std::env;

fn main() {
    init_global_statics();

    unsafe {
        // env::set_var("GNUTLS_SYSTEM_PRIORITY_FILE", "/dev/null");
        env::set_var("OPENSSL_CONF", "/dev/null");

        let ret = openconnect_init_ssl();
        println!("init ssl ret: {}", ret);
        let ctx = OpenconnectCtx::new();
        let vpninfo = (*ctx).vpninfo;

        openconnect_set_loglevel(vpninfo, PRG_INFO as i32);

        // ====== set proxy ======
        let proxy = env::var("https_proxy").unwrap_or("".to_string());
        if !proxy.is_empty() {
            let proxy = std::ffi::CString::new(proxy).unwrap();
            let ret = openconnect_set_http_proxy(vpninfo, proxy.as_ptr());
            println!("set http proxy ret: {}", ret);
        }
        // ====== set proxy end ======

        let cmd_fd = openconnect_setup_cmd_pipe(vpninfo);
        println!("cmd_fd: {}", cmd_fd);
        libc::fcntl(
            cmd_fd,
            libc::F_SETFD,
            libc::fcntl(cmd_fd, libc::F_GETFL) & !libc::O_NONBLOCK,
        );

        openconnect_set_stats_handler(vpninfo, Some(OpenconnectCtx::stats_fn));

        // ====== set protocol ======
        // TODO: refractor protocol selection
        let mut protos = std::ptr::null_mut::<oc_vpn_proto>();
        let ret = openconnect_get_supported_protocols(&mut protos);
        // TODO: change to Vec<SomeStruct> for saving protocol details
        let mut protocols: Vec<Option<String>> = vec![];
        if ret >= 0 {
            while !protos.is_null() && !(*protos).name.is_null() {
                let protocol = std::ffi::CStr::from_ptr((*protos).name)
                    .to_str()
                    .ok()
                    .map(|s| s.to_string());
                protocols.push(protocol);
                protos = protos.offset(1);
            }
        }

        println!("protocols: {:?}", protocols);

        let selected_protocol = protocols.first().unwrap().as_ref().unwrap();
        println!("selected protocol: {}", selected_protocol);
        let selected_protocol = std::ffi::CString::new(selected_protocol.as_str()).unwrap();
        let ret = openconnect_set_protocol(vpninfo, selected_protocol.as_ptr());
        println!("set protocol ret: {}", ret);
        // ====== set protocol end ======

        openconnect_set_setup_tun_handler(vpninfo, Some(OpenconnectCtx::setup_tun_vfn));

        let os_name = std::ffi::CString::new("linux-64").unwrap();
        let ret = openconnect_set_reported_os(vpninfo, os_name.as_ptr());
        println!("set os ret: {}", ret);

        let server = *SERVER.clone();
        let server = std::ffi::CString::new(server).unwrap();
        let ret = openconnect_parse_url(vpninfo, server.as_ptr());
        println!("parse url ret: {}", ret);

        let port = openconnect_get_port(vpninfo);
        println!("port: {}", port);

        let hostname = openconnect_get_hostname(vpninfo);
        let hostname = std::ffi::CStr::from_ptr(hostname).to_str().unwrap();
        println!("hostname: {}", hostname);

        println!();

        openconnect_set_client_cert(vpninfo, std::ptr::null(), std::ptr::null());
        openconnect_set_mca_cert(vpninfo, std::ptr::null(), std::ptr::null());

        let disable_udp = false;
        if disable_udp {
            let ret = openconnect_disable_dtls(vpninfo);
            println!("disable_dtls ret: {}", ret);
        }

        let cookie = openconnect_get_cookie(vpninfo);
        if cookie.is_null() {
            let ret = openconnect_obtain_cookie(vpninfo);
            println!("cookie ret: {}", ret);
        }

        let ret = openconnect_make_cstp_connection(vpninfo);
        println!("cstp ret: {}", ret);

        let reconnect_timeout = 300;
        'main: loop {
            let ret =
                openconnect_mainloop(vpninfo, reconnect_timeout, RECONNECT_INTERVAL_MIN as i32);
            if ret == 1 {
                break 'main;
            }
        }
    }
}
