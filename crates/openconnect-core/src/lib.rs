#![doc = include_str!("../README.md")]

mod cert;
pub mod command;
pub mod config;
pub mod elevator;
pub mod events;
mod form;
pub mod ip_info;
pub mod log;
pub mod protocols;
pub mod result;
pub mod stats;
pub mod storage;

use crate::cert::PeerCerts;
use crate::command::{CmdPipe, SIGNAL_HANDLE};
use crate::config::{Config, Entrypoint, LogLevel};
use crate::events::{EventHandlers, Events};
use crate::form::FormManager;
use crate::ip_info::IpInfo;
use crate::log::Logger;
use crate::result::{EmitError, OpenconnectError, OpenconnectResult};
use crate::stats::Stats;

use openconnect_sys::*;
use std::{
    ffi::CString,
    sync::{
        atomic::{AtomicI32, Ordering},
        Arc, RwLock, Weak,
    },
};

/// Describe the connection status of the client
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    /// The client is initialized
    Initialized,

    /// The client is disconnecting to the VPN server
    Disconnecting,

    /// The client is disconnected from the VPN server and command pipe is closed
    Disconnected,

    /// The client is connecting to the VPN server, with several stages described in the string
    Connecting(String),

    /// The client is connected to the VPN server and the main loop is running
    Connected,

    /// The client is in an error state
    Error(OpenconnectError),
}

/// VpnClient struct
///
/// This struct is the main entrypoint for interacting with the Openconnect C library (on top of [openconnect-sys](https://crates.io/crates/openconnect-sys))
#[repr(C)]
pub struct VpnClient {
    vpninfo: *mut openconnect_info,
    config: Config,
    cmd_fd: AtomicI32,
    status: RwLock<Status>,
    callbacks: EventHandlers,
    entrypoint: RwLock<Option<Entrypoint>>,
    form_manager: RwLock<FormManager>,
    peer_certs: PeerCerts,
}

unsafe impl Send for VpnClient {}
unsafe impl Sync for VpnClient {}

impl VpnClient {
    pub(crate) extern "C" fn default_setup_tun_vfn(privdata: *mut ::std::os::raw::c_void) {
        let client = unsafe { VpnClient::ref_from_raw(privdata) };

        #[cfg(target_os = "windows")]
        {
            // currently use wintun on windows
            // https://gitlab.com/openconnect/openconnect-gui/-/blob/main/src/vpninfo.cpp?ref_type=heads#L407
            // TODO: investigate tap ip address allocation, since it works well in Openconnect-GUI
            let ifname = client
                .get_hostname()
                .map(|hostname| format!("tun_{}", hostname));

            println!("ifname: {:?}", ifname);

            // TODO: handle result
            let _result = client.setup_tun_device(None, ifname);
        }

        #[cfg(not(target_os = "windows"))]
        {
            // TODO: handle result
            let _result = client.setup_tun_device(None, None);
        }
    }

    /// Reclaim a reference from c_void
    ///
    /// SAFETY: You must ensure that the pointer is valid and points to a valid instance of `Self`
    pub(crate) unsafe fn ref_from_raw<'a>(ptr: *mut std::os::raw::c_void) -> &'a Self {
        let ptr = ptr.cast::<Self>();
        &*ptr
    }

    pub(crate) fn handle_text_input(&self, field_name: &str) -> Option<String> {
        let entrypoint = self.entrypoint.read().ok()?;
        let entrypoint = (*entrypoint).as_ref()?;
        match field_name {
            "username" | "user" | "uname" => entrypoint.username.clone(),
            _ => todo!("handle_text_input: {}", field_name),
        }
    }

    pub(crate) fn handle_password_input(&self) -> Option<String> {
        let entrypoint = self.entrypoint.read().ok()?;
        (*entrypoint).as_ref()?.password.clone()
    }

    pub(crate) fn handle_stats(&self, (dlts, stats): (Option<String>, Option<Stats>)) {
        println!("stats: {:?}, {:?}", dlts, stats);
    }

    pub(crate) fn handle_accept_insecure_cert(&self, fingerprint: &str) -> bool {
        let entrypoint = self.entrypoint.read();
        let accept_in_entrypoint_config = {
            if let Ok(entrypoint) = entrypoint {
                (*entrypoint)
                    .as_ref()
                    .map(|entrypoint| entrypoint.accept_insecure_cert)
                    .unwrap_or(false)
            } else {
                false
            }
        };

        if accept_in_entrypoint_config {
            return true;
        }

        if let Some(ref handler) = self.callbacks.handle_peer_cert_invalid {
            handler(fingerprint)
        } else {
            false
        }
    }

    pub fn set_loglevel(&self, level: LogLevel) {
        unsafe {
            openconnect_set_loglevel(self.vpninfo, level as i32);
        }
    }

    pub fn set_protocol(&self, protocol: &str) -> OpenconnectResult<()> {
        let protocol =
            CString::new(protocol).map_err(|_| OpenconnectError::SetProtocolError(libc::EIO))?;
        let ret = unsafe { openconnect_set_protocol(self.vpninfo, protocol.as_ptr()) };
        match ret {
            0 => Ok(()),
            _ => Err(OpenconnectError::SetProtocolError(ret)),
        }
    }

    pub fn set_stats_handler(&self) {
        unsafe {
            openconnect_set_stats_handler(self.vpninfo, Some(stats::stats_fn));
        }
    }

    pub fn setup_tun_device(
        &self,
        vpnc_script: Option<String>,
        ifname: Option<String>,
    ) -> OpenconnectResult<()> {
        let vpnc_script_from_config = vpnc_script.or_else(|| self.config.vpncscript.clone());

        let vpnc_script = {
            if let Some(vpnc_script) = vpnc_script_from_config {
                CString::new(vpnc_script)
                    .map_err(|_| OpenconnectError::SetupTunDeviceEror(libc::EIO))?
            } else {
                #[cfg(not(target_os = "windows"))]
                const DEFAULT_SCRIPT: &str = "./vpnc-script";

                #[cfg(target_os = "windows")]
                const DEFAULT_SCRIPT: &str = "./vpnc-script-win.js";

                CString::new(DEFAULT_SCRIPT)
                    .map_err(|_| OpenconnectError::SetupTunDeviceEror(libc::EIO))?
            }
        };

        let ifname = ifname.and_then(|s| CString::new(s).ok());

        let ret = unsafe {
            openconnect_setup_tun_device(
                self.vpninfo,
                vpnc_script.as_ptr(),
                ifname.as_ref().map_or_else(std::ptr::null, |s| s.as_ptr()),
            )
        };

        let _manually_dropped = ifname; // SAFETY: dont remove this line, ifname's lifetime should be extended

        match ret {
            0 => Ok(()),
            _ => Err(OpenconnectError::SetupTunDeviceEror(ret)),
        }
    }

    pub fn set_setup_tun_handler(&self) {
        unsafe {
            openconnect_set_setup_tun_handler(self.vpninfo, Some(VpnClient::default_setup_tun_vfn));
        }
    }

    pub fn set_report_os(&self, os: &str) -> OpenconnectResult<()> {
        let os = CString::new(os).map_err(|_| OpenconnectError::SetReportOSError(libc::EIO))?;
        let ret = unsafe { openconnect_set_reported_os(self.vpninfo, os.as_ptr()) };
        match ret {
            0 => Ok(()),
            _ => Err(OpenconnectError::SetReportOSError(ret)),
        }
    }

    pub fn obtain_cookie(&self) -> OpenconnectResult<()> {
        let ret = unsafe { openconnect_obtain_cookie(self.vpninfo) };
        match ret {
            0 => Ok(()),
            _ => Err(result::OpenconnectError::ObtainCookieError(ret)),
        }
    }

    pub fn set_cookie(&self, cookie: &str) -> OpenconnectResult<()> {
        let cookie =
            CString::new(cookie).map_err(|_| OpenconnectError::SetCookieError(libc::EIO))?;
        let ret = unsafe { openconnect_set_cookie(self.vpninfo, cookie.as_ptr()) };
        match ret {
            0 => Ok(()),
            _ => Err(OpenconnectError::SetCookieError(ret)),
        }
    }

    pub fn clear_cookie(&self) {
        unsafe {
            openconnect_clear_cookie(self.vpninfo);
        }
    }

    pub fn get_cookie(&self) -> Option<String> {
        unsafe {
            let cookie = openconnect_get_cookie(self.vpninfo);
            std::ffi::CStr::from_ptr(cookie)
                .to_str()
                .map(|s| s.to_string())
                .ok()
        }
    }

    pub fn setup_cmd_pipe(&self) -> OpenconnectResult<()> {
        let cmd_fd = unsafe {
            let cmd_fd = openconnect_setup_cmd_pipe(self.vpninfo);
            self.cmd_fd.store(cmd_fd, Ordering::Relaxed);
            if cmd_fd < 0 {
                return Err(result::OpenconnectError::CmdPipeError(cmd_fd));
            }
            cmd_fd
        };
        self.set_sock_block(cmd_fd);
        Ok(())
    }

    pub fn reset_ssl(&self) {
        unsafe {
            openconnect_reset_ssl(self.vpninfo);
        }
    }

    pub fn make_cstp_connection(&self) -> OpenconnectResult<()> {
        let ret = unsafe { openconnect_make_cstp_connection(self.vpninfo) };
        match ret {
            0 => Ok(()),
            _ => Err(OpenconnectError::MakeCstpError(ret)),
        }
    }

    pub fn get_dlts_cipher(&self) -> Option<String> {
        unsafe {
            let cipher = openconnect_get_dtls_cipher(self.vpninfo);
            if !cipher.is_null() {
                Some(
                    std::ffi::CStr::from_ptr(cipher)
                        .to_str()
                        .unwrap()
                        .to_string(),
                )
            } else {
                None
            }
        }
    }

    pub fn get_peer_cert_hash(&self) -> String {
        // SAFETY: we should not use CString::from_raw(peer_fingerprint)
        // because peer_fingerprint will be deallocated in rust and cause a double free
        unsafe { std::ffi::CStr::from_ptr(openconnect_get_peer_cert_hash(self.vpninfo)) }
            .to_string_lossy()
            .to_string()
    }

    pub fn disable_dtls(&self) -> OpenconnectResult<()> {
        let ret = unsafe { openconnect_disable_dtls(self.vpninfo) };
        match ret {
            0 => Ok(()),
            _ => Err(OpenconnectError::DisableDTLSError(ret)),
        }
    }

    pub fn set_http_proxy(&self, proxy: &str) -> OpenconnectResult<()> {
        let proxy = CString::new(proxy).map_err(|_| OpenconnectError::SetProxyError(libc::EIO))?;
        let ret = unsafe { openconnect_set_http_proxy(self.vpninfo, proxy.as_ptr()) };
        match ret {
            0 => Ok(()),
            _ => Err(OpenconnectError::SetProxyError(ret)),
        }
    }

    pub fn parse_url(&self, url: &str) -> OpenconnectResult<()> {
        let url = CString::new(url).map_err(|_| OpenconnectError::ParseUrlError(libc::EIO))?;
        let ret = unsafe { openconnect_parse_url(self.vpninfo, url.as_ptr()) };
        match ret {
            0 => Ok(()),
            _ => Err(OpenconnectError::ParseUrlError(ret)),
        }
    }

    pub fn get_server_name(&self) -> Option<String> {
        {
            let entrypoint = self.entrypoint.read().ok()?;
            (*entrypoint).as_ref()?.name.clone()
        }
    }

    pub fn get_server_url(&self) -> Option<String> {
        {
            let entrypoint = self.entrypoint.read().ok()?;
            Some((*entrypoint).as_ref()?.server.clone())
        }
    }

    pub fn get_port(&self) -> i32 {
        unsafe { openconnect_get_port(self.vpninfo) }
    }

    pub fn get_hostname(&self) -> Option<String> {
        unsafe {
            let hostname = openconnect_get_hostname(self.vpninfo);
            std::ffi::CStr::from_ptr(hostname)
                .to_str()
                .map(|s| s.to_string())
                .ok()
        }
    }

    pub fn set_client_cert(&self, cert: &str, sslkey: &str) -> OpenconnectResult<()> {
        let cert =
            CString::new(cert).map_err(|_| OpenconnectError::SetClientCertError(libc::EIO))?;
        let sslkey =
            CString::new(sslkey).map_err(|_| OpenconnectError::SetClientCertError(libc::EIO))?;
        let ret =
            unsafe { openconnect_set_client_cert(self.vpninfo, cert.as_ptr(), sslkey.as_ptr()) };
        match ret {
            0 => Ok(()),
            _ => Err(OpenconnectError::SetClientCertError(ret)),
        }
    }

    pub fn set_mca_cert(&self, cert: &str, key: &str) -> OpenconnectResult<()> {
        let cert = CString::new(cert).map_err(|_| OpenconnectError::SetMCACertError(libc::EIO))?;
        let key = CString::new(key).map_err(|_| OpenconnectError::SetMCACertError(libc::EIO))?;
        let ret = unsafe { openconnect_set_mca_cert(self.vpninfo, cert.as_ptr(), key.as_ptr()) };
        match ret {
            0 => Ok(()),
            _ => Err(OpenconnectError::SetMCACertError(ret)),
        }
    }

    pub fn get_info(&self) -> OpenconnectResult<Option<IpInfo>> {
        unsafe {
            let info = std::ptr::null_mut();
            let ret = openconnect_get_ip_info(
                self.vpninfo,
                info,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );

            match ret {
                0 => Ok(info
                    .as_ref()
                    .and_then(|info| info.as_ref())
                    .map(IpInfo::from)),
                _ => Err(OpenconnectError::GetIpInfoError(ret)),
            }
        }
    }

    pub(crate) fn main_loop(
        &self,
        reconnect_timeout: i32,
        reconnect_interval: u32,
    ) -> OpenconnectResult<()> {
        let ret = unsafe {
            openconnect_mainloop(self.vpninfo, reconnect_timeout, reconnect_interval as i32)
        };
        match ret {
            0 => Ok(()),
            _ => Err(OpenconnectError::MainLoopError(ret)),
        }
    }

    pub(crate) fn free(&self) {
        unsafe {
            openconnect_vpninfo_free(self.vpninfo);
        }
        tracing::debug!("Client instance is dropped");
    }
}

impl Drop for VpnClient {
    fn drop(&mut self) {
        self.disconnect();
        self.free();
    }
}

/// Trait for creating a new instance of VpnClient and connecting to the VPN server
///
/// This trait is implemented for the lifecycle of the VpnClient
pub trait Connectable {
    fn new(config: Config, callbacks: EventHandlers) -> OpenconnectResult<Arc<Self>>;
    fn connect_for_cookie(&self, entrypoint: Entrypoint) -> OpenconnectResult<Option<String>>;
    fn init_connection(&self, entrypoint: Entrypoint) -> OpenconnectResult<()>;
    fn run_loop(&self) -> OpenconnectResult<()>;
    fn disconnect(&self);
    fn get_status(&self) -> Status;
    fn get_server_name(&self) -> Option<String>;
}

impl Connectable for VpnClient {
    /// Create a new instance of VpnClient
    ///
    /// config can be created using [config::ConfigBuilder]
    ///
    /// callbacks can be created using [events::EventHandlers]
    fn new(config: Config, callbacks: EventHandlers) -> OpenconnectResult<Arc<Self>> {
        let useragent = std::ffi::CString::new("AnyConnect-compatible OpenConnect VPN Agent")
            .map_err(|_| OpenconnectError::OtherError("useragent is not valid".to_string()))?;

        let instance = Arc::new(Self {
            vpninfo: std::ptr::null_mut(),
            config,
            cmd_fd: (-1).into(),
            status: RwLock::new(Status::Initialized),
            callbacks,
            entrypoint: RwLock::new(None),
            form_manager: RwLock::new(FormManager::default()),
            peer_certs: PeerCerts::default(),
        });

        unsafe {
            let weak_instance = Arc::downgrade(&instance);
            let raw_instance = Weak::into_raw(weak_instance) as *mut VpnClient; // dangerous, leak for assign to vpninfo
            let ret = openconnect_init_ssl();
            if ret != 0 {
                panic!("openconnect_init_ssl failed");
            }

            // format args on C side
            helper_set_global_progress_vfn(Some(Logger::raw_handle_process_log));

            let vpninfo = openconnect_vpninfo_new(
                useragent.as_ptr(),
                Some(PeerCerts::validate_peer_cert),
                None,
                Some(FormManager::process_auth_form_cb),
                Some(helper_format_vargs), // format args on C side
                raw_instance as *mut ::std::os::raw::c_void,
            );

            if vpninfo.is_null() {
                panic!("openconnect_vpninfo_new failed");
            }

            (*raw_instance).vpninfo = vpninfo;
        };

        SIGNAL_HANDLE.update_client_singleton(Arc::downgrade(&instance));
        instance.set_loglevel(instance.config.loglevel);
        instance.set_setup_tun_handler();

        if let Some(proxy) = &instance.config.http_proxy {
            instance
                .set_http_proxy(proxy.as_str())
                .emit_error(&instance)?;
        }

        instance.emit_state_change(Status::Initialized);

        Ok(instance)
    }

    /// Connect to the VPN server and obtain a cookie
    ///
    /// This function will not keep the connection, it will only connect and obtain a cookie. This function will not block the thread
    ///
    /// The cookie can be used to connect to the VPN server later by passing it to another [config::EntrypointBuilder]
    ///
    /// entrypoint can be created using [config::EntrypointBuilder]
    fn connect_for_cookie(&self, entrypoint: Entrypoint) -> OpenconnectResult<Option<String>> {
        self.emit_state_change(Status::Connecting("Initializing connection".to_string()));
        {
            if let Ok(mut form_context) = self.form_manager.try_write() {
                form_context.reset();
            }
        }
        self.set_protocol(&entrypoint.protocol.name)
            .emit_error(self)?;
        self.emit_state_change(Status::Connecting("Setting up system pipe".to_string()));
        self.setup_cmd_pipe().emit_error(self)?;
        self.set_stats_handler();

        #[cfg(target_os = "windows")]
        const OS_NAME: &str = "win";

        #[cfg(target_os = "macos")]
        const OS_NAME: &str = "mac-intel";

        #[cfg(target_os = "linux")]
        const OS_NAME: &str = "linux-64";

        self.set_report_os(OS_NAME).emit_error(self)?;

        {
            let mut entrypoint_write_guard = self
                .entrypoint
                .write()
                .map_err(|_| {
                    OpenconnectError::EntrypointConfigError(
                        "write entrypoint lock failed".to_string(),
                    )
                })
                .emit_error(self)?;

            *entrypoint_write_guard = Some(entrypoint.clone());
            // drop entrypoint_write_guard
        }

        if !entrypoint.enable_udp {
            self.disable_dtls().emit_error(self)?;
        }

        self.emit_state_change(Status::Connecting("Parsing URL".to_string()));
        self.parse_url(&entrypoint.server).emit_error(self)?;
        let hostname = self.get_hostname();

        self.emit_state_change(Status::Connecting(format!(
            "Obtaining cookie from: {}",
            hostname.unwrap_or("".to_string())
        )));
        if let Some(cookie) = entrypoint.cookie.clone() {
            self.set_cookie(&cookie).emit_error(self)?;
        } else {
            self.obtain_cookie().emit_error(self)?;
        }

        Ok(self.get_cookie())
    }

    /// Initialize the connection to the VPN server, this function will not block the thread and only make a CSTP connection
    ///
    /// entrypoint can be created using [config::EntrypointBuilder]
    fn init_connection(&self, entrypoint: Entrypoint) -> OpenconnectResult<()> {
        self.emit_state_change(Status::Connecting("Make CSTP connection".to_string()));
        self.connect_for_cookie(entrypoint)?;
        self.make_cstp_connection().emit_error(self)?;
        self.emit_state_change(Status::Connected);

        Ok(())
    }

    /// Run main loop and block until the connection is closed
    fn run_loop(&self) -> OpenconnectResult<()> {
        loop {
            if self.main_loop(300, RECONNECT_INTERVAL_MIN).is_err() {
                break;
            }
        }

        // TODO: check if the following should be invoke?
        // self.reset_ssl();
        // self.clear_cookie();
        self.emit_state_change(Status::Disconnected);

        Ok(())
    }

    /// Gracefully stop the main loop
    ///
    /// This function will send a cancel command to the main loop and wait for the main loop to stop
    fn disconnect(&self) {
        if self.get_status() != Status::Connected {
            return;
        }

        self.emit_state_change(Status::Disconnecting);
        self.send_command(command::Command::Cancel);
        self.cmd_fd.store(-1, Ordering::SeqCst);

        std::thread::sleep(std::time::Duration::from_millis(200));
    }

    fn get_server_name(&self) -> Option<String> {
        self.entrypoint
            .read()
            .ok()
            .and_then(|r| r.as_ref().and_then(|e| e.name.clone()))
    }

    fn get_status(&self) -> Status {
        self.status
            .read()
            .ok()
            .map_or(Status::Initialized, |r| r.clone())
    }
}

impl Events for VpnClient {
    fn emit_state_change(&self, status: Status) {
        if let Some(ref handler) = self.callbacks.handle_connection_state_change {
            handler(status.clone());
        }

        {
            let status_write_guard = self.status.write();
            if let Ok(mut write) = status_write_guard {
                *write = status;
            } else {
                // FIXME: handle error?
            }
        }
    }

    /// Change state and emit error to state change handler
    fn emit_error(&self, error: &OpenconnectError) {
        self.emit_state_change(Status::Error(error.clone()));
    }
}
