pub mod cert;
pub mod config;
pub mod events;
pub mod form;
pub mod protocols;
pub mod result;
pub mod stats;

use config::{Config, Entrypoint, LogLevel};
use events::{EventHandlers, Events};
use form::FormContext;
pub use openconnect_sys::*;
use result::{EmitError, OpenconnectError, OpenconnectResult};
use stats::Stats;
use std::{
    ffi::CString,
    sync::{
        atomic::{AtomicI32, Ordering},
        Arc, RwLock,
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    Initialized,
    Disconnecting,
    Disconnected,
    Connecting,
    Connected,
    Error(OpenconnectError),
}

#[repr(C)]
pub struct VpnClient {
    pub(crate) config: Config,
    vpninfo: *mut openconnect_info,
    cmd_fd: AtomicI32,
    status: RwLock<Status>,
    callbacks: EventHandlers,
    entrypoint: RwLock<Option<Entrypoint>>,
    form_context: FormContext,
}

// make openconnect_info thread shareable/tranferable
// TODO: check if it's safe to share openconnect_info between threads
unsafe impl Send for VpnClient {}
unsafe impl Sync for VpnClient {}

impl VpnClient {
    pub(crate) unsafe extern "C" fn handle_process_log(
        _privdata: *mut ::std::os::raw::c_void,
        level: ::std::os::raw::c_int,
        buf: *const ::std::os::raw::c_char,
    ) {
        let buf = std::ffi::CStr::from_ptr(buf).to_str().ok();
        let level = level as u32;
        let level = match level {
            PRG_ERR => "ERR",
            PRG_INFO => "INFO",
            PRG_DEBUG => "DEBUG",
            PRG_TRACE => "TRACE",
            _ => "UNKNOWN",
        };
        if buf.is_some() {
            println!("{}: {}", level, buf.unwrap());
        }
    }

    pub(crate) extern "C" fn validate_peer_cert(
        _privdata: *mut ::std::os::raw::c_void,
        _reason: *const ::std::os::raw::c_char,
    ) -> ::std::os::raw::c_int {
        println!("validate_peer_cert");
        0
    }

    pub(crate) extern "C" fn default_setup_tun_vfn(privdata: *mut ::std::os::raw::c_void) {
        let client = VpnClient::from_c_void(privdata);
        if !client.is_null() {
            unsafe {
                let vpnc_script = (*client)
                    .config
                    .vpncscript
                    .clone()
                    .map(|s| CString::new(s).unwrap())
                    .map_or_else(|| DEFAULT_VPNCSCRIPT.as_ptr() as *const i8, |s| s.as_ptr());

                let ret = openconnect_setup_tun_device(
                    (*client).vpninfo,
                    vpnc_script,
                    std::ptr::null_mut(),
                );

                // TODO: handle ret
                println!("setup_tun_device ret: {}", ret);
            }
        }
    }

    pub(crate) fn from_c_void(ptr: *mut std::os::raw::c_void) -> *mut Self {
        ptr.cast()
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

    pub fn set_loglevel(&self, level: LogLevel) {
        unsafe {
            openconnect_set_loglevel(self.vpninfo, level as i32);
        }
    }

    pub fn set_protocol(&self, protocol: &str) -> OpenconnectResult<()> {
        let protocol = CString::new(protocol).unwrap();
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

    pub fn set_setup_tun_handler(&self) {
        unsafe {
            openconnect_set_setup_tun_handler(self.vpninfo, Some(VpnClient::default_setup_tun_vfn));
        }
    }

    pub fn set_report_os(&self, os: &str) -> OpenconnectResult<()> {
        let os = CString::new(os).unwrap();
        let ret = unsafe { openconnect_set_reported_os(self.vpninfo, os.as_ptr()) };
        match ret {
            0 => Ok(()),
            _ => Err(OpenconnectError::SetReportOSError(ret)),
        }
    }

    pub fn obtain_cookie(&self) -> OpenconnectResult<()> {
        let ret = unsafe {
            println!();
            println!("obtain_cookie");
            println!();
            openconnect_obtain_cookie(self.vpninfo)
        };
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
        unsafe {
            let cmd_fd = openconnect_setup_cmd_pipe(self.vpninfo);
            self.cmd_fd.store(cmd_fd, Ordering::Relaxed);
            if cmd_fd < 0 {
                return Err(result::OpenconnectError::CmdPipeError(cmd_fd));
            }

            #[cfg(not(target_os = "windows"))]
            {
                libc::fcntl(
                    cmd_fd,
                    libc::F_SETFL,
                    libc::fcntl(cmd_fd, libc::F_GETFL) & !libc::O_NONBLOCK,
                );
            }

            #[cfg(target_os = "windows")]
            {
                let mut mode: u32 = 0;
                windows_sys::Win32::Networking::WinSock::ioctlsocket(
                    cmd_fd as usize,
                    windows_sys::Win32::Networking::WinSock::FIONBIO,
                    &mut mode,
                );
            }
        }
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
        println!("free context");
    }
}

impl Drop for VpnClient {
    fn drop(&mut self) {
        self.disconnect();
        self.free();
    }
}

pub trait Connectable {
    fn new(config: Config, callbacks: EventHandlers) -> OpenconnectResult<Arc<Self>>;
    fn connect(&self, entrypoint: Entrypoint) -> OpenconnectResult<()>;
    fn disconnect(&self);
    fn get_state(&self) -> Status;
}

impl Connectable for VpnClient {
    fn new(config: Config, callbacks: EventHandlers) -> OpenconnectResult<Arc<Self>> {
        let useragent =
            std::ffi::CString::new("AnyConnect-compatible OpenConnect VPN Agent").unwrap();

        let instance = Arc::new(Self {
            vpninfo: std::ptr::null_mut(),
            config,
            cmd_fd: (-1).into(),
            status: RwLock::new(Status::Initialized),
            callbacks,
            entrypoint: RwLock::new(None),
            form_context: FormContext::default(),
        });

        let instance = Arc::into_raw(instance) as *mut VpnClient; // dangerous, leak for assign to vpninfo

        let instance = unsafe {
            let ret = openconnect_init_ssl();
            if ret != 0 {
                panic!("openconnect_init_ssl failed");
            }

            // format args on C side
            helper_set_global_progress_vfn(Some(Self::handle_process_log));

            let vpninfo = openconnect_vpninfo_new(
                useragent.as_ptr(),
                Some(Self::validate_peer_cert),
                None,
                Some(FormContext::process_auth_form_cb),
                Some(helper_format_vargs), // format args on C side
                instance as *mut ::std::os::raw::c_void,
            );

            if vpninfo.is_null() {
                panic!("openconnect_vpninfo_new failed");
            }

            (*instance).vpninfo = vpninfo;
            Arc::from_raw(instance) // reclaim ownership
        };

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

    fn connect(&self, entrypoint: Entrypoint) -> OpenconnectResult<()> {
        self.emit_state_change(Status::Connecting);
        self.form_context.reset();

        self.set_protocol(&entrypoint.protocol.name)
            .emit_error(self)?;
        self.setup_cmd_pipe().emit_error(self)?;
        self.set_stats_handler();
        self.set_report_os("linux-64").emit_error(self)?;

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

        self.parse_url(&entrypoint.server).emit_error(self)?;
        let hostname = self.get_hostname();
        if let Some(hostname) = hostname {
            println!("connecting: {}", hostname);
        }

        if let Some(cookie) = entrypoint.cookie.clone() {
            self.set_cookie(&cookie).emit_error(self)?;
        } else {
            self.obtain_cookie().emit_error(self)?;
        }
        self.make_cstp_connection().emit_error(self)?;

        self.emit_state_change(Status::Connected);

        loop {
            if self.main_loop(300, RECONNECT_INTERVAL_MIN).is_err() {
                break;
            }
        }

        self.reset_ssl();
        self.clear_cookie();

        Ok(())
    }

    /// Gracefully stop the main loop
    fn disconnect(&self) {
        if self.get_state() != Status::Connected {
            return;
        }

        self.emit_state_change(Status::Disconnecting);

        let cmd = OC_CMD_CANCEL;
        unsafe {
            let cmd_fd = self.cmd_fd.load(Ordering::Relaxed);
            if cmd_fd != -1 {
                // TODO: implement under windows, should use libc::send
                let ret = libc::write(cmd_fd, std::ptr::from_ref(&cmd) as *const _, 1);
                if ret < 0 {
                    println!("write cmd_fd failed");
                }
                self.cmd_fd.store(-1, Ordering::Relaxed);
            }
        }

        self.emit_state_change(Status::Disconnected);
    }

    fn get_state(&self) -> Status {
        self.status
            .read()
            .ok()
            .map_or(Status::Initialized, |r| r.clone())
    }
}

pub trait Shutdown {
    fn with_ctrlc_shutdown(self) -> OpenconnectResult<Self>
    where
        Self: std::marker::Sized;
}

impl Shutdown for Arc<VpnClient> {
    fn with_ctrlc_shutdown(self) -> OpenconnectResult<Self> {
        let cloned_client = self.clone();

        ctrlc::set_handler(move || {
            cloned_client.disconnect();
        })
        .map_err(|e| OpenconnectError::SetupShutdownError(e.to_string()))?;

        Ok(self)
    }
}

impl Events for VpnClient {
    fn emit_state_change(&self, status: Status) {
        {
            let status_write_guard = self.status.write();
            if let Ok(mut write) = status_write_guard {
                *write = status.clone();
            } else {
                // FIXME: handle error?
                return;
            }
        }

        if let Some(ref handler) = self.callbacks.handle_connection_state_change {
            handler(status);
        }
    }

    /// Change state and emit error to state change handler
    fn emit_error(&self, error: &OpenconnectError) {
        self.emit_state_change(Status::Error(error.clone()));
    }
}
