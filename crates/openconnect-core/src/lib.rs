#![doc = include_str!("../README.md")]

//! ## Example: Password Server
//!
//! This example demonstrates how to connect to a VPN server using username + password authentication
//!
//! ```rust,no_run
#![doc = include_str!("../examples/password_server.rs")]
//! ```

mod cert;
pub mod command;
pub mod config;
pub mod elevator;
pub mod events;
mod ffi;
mod form;
pub mod ip_info;
pub mod log;
pub mod protocols;
pub mod result;
pub mod stats;
pub mod storage;

use crate::{
    cert::PeerCerts,
    command::{CmdPipe, CmdPipeFd, SIGNAL_HANDLE},
    config::{Config, Entrypoint, LogLevel},
    events::{EventHandlers, Events},
    ffi::{VpnInfoHandle, c_str_to_option_string, with_callback_context},
    form::FormManager,
    ip_info::IpInfo,
    result::{EmitError, OpenconnectError, OpenconnectResult},
    stats::Stats,
};
use openconnect_sys::{
    RECONNECT_INTERVAL_MIN, oc_ip_info, openconnect_clear_cookie, openconnect_disable_dtls,
    openconnect_get_cookie, openconnect_get_dtls_cipher, openconnect_get_hostname,
    openconnect_get_ip_info, openconnect_get_peer_cert_hash, openconnect_get_port,
    openconnect_mainloop, openconnect_make_cstp_connection, openconnect_obtain_cookie,
    openconnect_parse_url, openconnect_reset_ssl, openconnect_set_client_cert,
    openconnect_set_cookie, openconnect_set_http_proxy, openconnect_set_loglevel,
    openconnect_set_mca_cert, openconnect_set_protocol, openconnect_set_reported_os,
    openconnect_set_setup_tun_handler, openconnect_set_stats_handler, openconnect_setup_cmd_pipe,
    openconnect_setup_tun_device,
};
use std::{
    ffi::CString,
    sync::{Arc, Mutex, MutexGuard, RwLock},
};

/// Describe the connection status of the client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    /// The client is initialized.
    Initialized,

    /// The client is disconnecting from the VPN server.
    Disconnecting,

    /// The client is disconnected from the VPN server and command pipe is closed.
    Disconnected,

    /// The client is connecting to the VPN server, with several stages described in the string.
    Connecting(String),

    /// The client is connected to the VPN server and the main loop is running.
    Connected,

    /// The client is in an error state.
    Error(OpenconnectError),
}

/// Thread-safe state shared between [`VpnClient`] and C FFI callbacks via `Arc<SharedState>`.
pub(crate) struct SharedState {
    pub(crate) config: Config,
    pub(crate) cmd_fd: Mutex<Option<CmdPipeFd>>,
    pub(crate) status: RwLock<Status>,
    pub(crate) callbacks: EventHandlers,
    pub(crate) entrypoint: RwLock<Option<Entrypoint>>,
    pub(crate) form_manager: Mutex<FormManager>,
    pub(crate) peer_certs: PeerCerts,
    pub(crate) cached_hostname: RwLock<Option<String>>,
    pub(crate) cached_ip_info: RwLock<Option<IpInfo>>,
}

impl SharedState {
    pub(crate) fn handle_text_input(&self, field_name: &str) -> Option<String> {
        let entrypoint = self.entrypoint.read().ok()?;
        let entrypoint = (*entrypoint).as_ref()?;
        match field_name {
            "username" | "user" | "uname" => entrypoint.username.clone(),
            other => {
                tracing::warn!("Unsupported auth form text input field: {}", other);
                None
            }
        }
    }

    pub(crate) fn handle_password_input(&self) -> Option<String> {
        let entrypoint = self.entrypoint.read().ok()?;
        (*entrypoint).as_ref()?.password.clone()
    }

    pub(crate) fn handle_stats(&self, (dtls, stats): (Option<String>, Option<Stats>)) {
        tracing::debug!("VPN stats: dtls={:?}, stats={:?}", dtls, stats);
    }

    pub(crate) fn handle_accept_insecure_cert(&self, fingerprint: &str) -> bool {
        let accept_in_entrypoint_config = self
            .entrypoint
            .read()
            .ok()
            .and_then(|ep| ep.as_ref().map(|e| e.accept_insecure_cert))
            .unwrap_or(false);

        if accept_in_entrypoint_config {
            return true;
        }

        if let Some(ref handler) = self.callbacks.handle_peer_cert_invalid {
            handler(fingerprint)
        } else {
            false
        }
    }

    pub(crate) fn resolve_vpnc_script(
        &self,
        vpnc_script: Option<String>,
    ) -> OpenconnectResult<CString> {
        let script = vpnc_script.or_else(|| self.config.vpncscript.clone());
        let script_str = match script {
            Some(s) => s,
            None => {
                #[cfg(not(target_os = "windows"))]
                const DEFAULT_SCRIPT: &str = "./vpnc-script";
                #[cfg(target_os = "windows")]
                const DEFAULT_SCRIPT: &str = "./vpnc-script-win.js";
                DEFAULT_SCRIPT.to_string()
            }
        };
        CString::new(script_str).map_err(|_| OpenconnectError::SetupTunDeviceError(libc::EINVAL))
    }
}

/// Main safe entrypoint for interacting with the OpenConnect C library.
///
/// `VpnClient` automatically derives `Send + Sync` with zero `unsafe impl Send/Sync`:
/// - `handle` serializes all non-thread-safe `libopenconnect` calls behind a `Mutex`.
/// - `state` holds thread-safe metadata, snapshots, callbacks, and the command pipe FD.
pub struct VpnClient {
    handle: Mutex<VpnInfoHandle>,
    pub(crate) state: Arc<SharedState>,
}

// Compile-time guarantee that `VpnClient` is soundly `Send + Sync`.
const _: () = {
    const fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<VpnClient>();
};

impl VpnClient {
    fn lock_handle(&self) -> OpenconnectResult<MutexGuard<'_, VpnInfoHandle>> {
        self.handle
            .lock()
            .map_err(|_| OpenconnectError::LockError("vpninfo handle mutex poisoned".to_string()))
    }

    pub(crate) extern "C" fn default_setup_tun_vfn(privdata: *mut ::std::os::raw::c_void) {
        let body = |ctx: &crate::ffi::CallbackContext| {
            let Some(vpninfo) = ctx.vpninfo_ptr() else {
                return;
            };
            let vpninfo_ptr = vpninfo.as_ptr();

            let Ok(vpnc_script) = ctx.state.resolve_vpnc_script(None) else {
                tracing::error!("Invalid vpnc-script path in default_setup_tun_vfn");
                return;
            };

            #[cfg(target_os = "windows")]
            let ifname_cstr = {
                // SAFETY: `vpninfo_ptr` is valid during this synchronous callback.
                let raw_host = unsafe { openconnect_get_hostname(vpninfo_ptr) };
                // SAFETY: `openconnect_get_hostname` returns NULL or a valid C string.
                let hostname = unsafe { c_str_to_option_string(raw_host) };
                hostname.and_then(|h| CString::new(format!("tun_{}", h)).ok())
            };

            #[cfg(not(target_os = "windows"))]
            let ifname_cstr: Option<CString> = None;

            let ifname_ptr = ifname_cstr
                .as_ref()
                .map_or(std::ptr::null(), |s| s.as_ptr());

            // SAFETY: `vpninfo_ptr`, `vpnc_script`, and `ifname_ptr` are valid for this call.
            let ret = unsafe {
                openconnect_setup_tun_device(vpninfo_ptr, vpnc_script.as_ptr(), ifname_ptr)
            };
            drop(ifname_cstr);
            if ret != 0 {
                tracing::error!("openconnect_setup_tun_device failed with code {}", ret);
            }
        };

        // SAFETY: Invoked synchronously by `libopenconnect` while `VpnInfoHandle` is alive.
        unsafe { with_callback_context(privdata, (), body) };
    }

    pub fn set_loglevel(&self, level: LogLevel) {
        if let Ok(handle) = self.lock_handle() {
            // SAFETY: `handle.as_ptr()` is a valid, exclusively locked `openconnect_info`.
            unsafe {
                openconnect_set_loglevel(handle.as_ptr(), level as i32);
            }
        }
    }

    pub fn set_protocol(&self, protocol: &str) -> OpenconnectResult<()> {
        let protocol =
            CString::new(protocol).map_err(|_| OpenconnectError::SetProtocolError(libc::EINVAL))?;
        let handle = self.lock_handle()?;
        // SAFETY: `handle.as_ptr()` is exclusively locked; `libopenconnect` copies `protocol`.
        let ret = unsafe { openconnect_set_protocol(handle.as_ptr(), protocol.as_ptr()) };
        match ret {
            0 => Ok(()),
            _ => Err(OpenconnectError::SetProtocolError(ret)),
        }
    }

    pub fn set_stats_handler(&self) {
        if let Ok(handle) = self.lock_handle() {
            // SAFETY: `handle.as_ptr()` is exclusively locked and `stats::stats_fn` has matching C ABI.
            unsafe {
                openconnect_set_stats_handler(handle.as_ptr(), Some(stats::stats_fn));
            }
        }
    }

    pub fn setup_tun_device(
        &self,
        vpnc_script: Option<String>,
        ifname: Option<String>,
    ) -> OpenconnectResult<()> {
        let vpnc_script = self.state.resolve_vpnc_script(vpnc_script)?;
        let ifname = ifname
            .map(|s| {
                CString::new(s).map_err(|_| OpenconnectError::SetupTunDeviceError(libc::EINVAL))
            })
            .transpose()?;

        let handle = self.lock_handle()?;
        let ifname_ptr = ifname.as_ref().map_or(std::ptr::null(), |s| s.as_ptr());

        // SAFETY: `handle.as_ptr()`, `vpnc_script`, and `ifname_ptr` are valid for this synchronous call.
        let ret = unsafe {
            openconnect_setup_tun_device(handle.as_ptr(), vpnc_script.as_ptr(), ifname_ptr)
        };
        drop(ifname);

        match ret {
            0 => Ok(()),
            _ => Err(OpenconnectError::SetupTunDeviceError(ret)),
        }
    }

    pub fn set_setup_tun_handler(&self) {
        if let Ok(handle) = self.lock_handle() {
            // SAFETY: `handle.as_ptr()` is exclusively locked and callback has matching C ABI.
            unsafe {
                openconnect_set_setup_tun_handler(
                    handle.as_ptr(),
                    Some(VpnClient::default_setup_tun_vfn),
                );
            }
        }
    }

    pub fn set_report_os(&self, os: &str) -> OpenconnectResult<()> {
        let os = CString::new(os).map_err(|_| OpenconnectError::SetReportOSError(libc::EINVAL))?;
        let handle = self.lock_handle()?;
        // SAFETY: `handle.as_ptr()` is exclusively locked; `libopenconnect` copies `os`.
        let ret = unsafe { openconnect_set_reported_os(handle.as_ptr(), os.as_ptr()) };
        match ret {
            0 => Ok(()),
            _ => Err(OpenconnectError::SetReportOSError(ret)),
        }
    }

    pub fn obtain_cookie(&self) -> OpenconnectResult<()> {
        let handle = self.lock_handle()?;
        // SAFETY: `handle.as_ptr()` is exclusively locked.
        let ret = unsafe { openconnect_obtain_cookie(handle.as_ptr()) };
        match ret {
            0 => Ok(()),
            _ => Err(OpenconnectError::ObtainCookieError(ret)),
        }
    }

    pub fn set_cookie(&self, cookie: &str) -> OpenconnectResult<()> {
        let cookie =
            CString::new(cookie).map_err(|_| OpenconnectError::SetCookieError(libc::EINVAL))?;
        let handle = self.lock_handle()?;
        // SAFETY: `handle.as_ptr()` is exclusively locked; `libopenconnect` copies `cookie`.
        let ret = unsafe { openconnect_set_cookie(handle.as_ptr(), cookie.as_ptr()) };
        match ret {
            0 => Ok(()),
            _ => Err(OpenconnectError::SetCookieError(ret)),
        }
    }

    pub fn clear_cookie(&self) {
        if let Ok(handle) = self.lock_handle() {
            // SAFETY: `handle.as_ptr()` is exclusively locked.
            unsafe {
                openconnect_clear_cookie(handle.as_ptr());
            }
        }
    }

    pub fn get_cookie(&self) -> Option<String> {
        let handle = self.lock_handle().ok()?;
        // SAFETY: `handle.as_ptr()` is exclusively locked.
        let cookie = unsafe { openconnect_get_cookie(handle.as_ptr()) };
        // SAFETY: `openconnect_get_cookie` returns NULL or a valid C string while `handle` is locked.
        unsafe { c_str_to_option_string(cookie) }
    }

    pub fn setup_cmd_pipe(&self) -> OpenconnectResult<()> {
        let handle = self.lock_handle()?;
        // SAFETY: `handle.as_ptr()` is exclusively locked.
        let raw_fd = unsafe { openconnect_setup_cmd_pipe(handle.as_ptr()) };
        #[cfg(not(target_os = "windows"))]
        if raw_fd < 0 {
            return Err(OpenconnectError::CmdPipeError(raw_fd));
        }

        #[cfg(not(target_os = "windows"))]
        let cmd_fd: CmdPipeFd = raw_fd;
        #[cfg(target_os = "windows")]
        let cmd_fd: CmdPipeFd = {
            let sock = raw_fd as usize;
            if sock == windows_sys::Win32::Networking::WinSock::INVALID_SOCKET {
                return Err(OpenconnectError::CmdPipeError(-1));
            }
            sock
        };

        {
            let mut guard =
                self.state.cmd_fd.lock().map_err(|_| {
                    OpenconnectError::LockError("cmd_fd mutex poisoned".to_string())
                })?;
            *guard = Some(cmd_fd);
        }
        self.set_sock_block(cmd_fd);
        Ok(())
    }

    pub fn reset_ssl(&self) {
        if let Ok(handle) = self.lock_handle() {
            // SAFETY: `handle.as_ptr()` is exclusively locked.
            unsafe {
                openconnect_reset_ssl(handle.as_ptr());
            }
        }
    }

    pub fn make_cstp_connection(&self) -> OpenconnectResult<()> {
        let handle = self.lock_handle()?;
        // SAFETY: `handle.as_ptr()` is exclusively locked.
        let ret = unsafe { openconnect_make_cstp_connection(handle.as_ptr()) };
        if ret != 0 {
            return Err(OpenconnectError::MakeCstpError(ret));
        }

        // Refresh cached IP info snapshot before `run_loop` holds `handle` exclusively.
        let info = Self::query_ip_info_locked(&handle).ok().flatten();
        if let Ok(mut cached) = self.state.cached_ip_info.write() {
            *cached = info;
        }
        Ok(())
    }

    pub fn get_dtls_cipher(&self) -> Option<String> {
        let handle = self.lock_handle().ok()?;
        // SAFETY: `handle.as_ptr()` is exclusively locked.
        let cipher = unsafe { openconnect_get_dtls_cipher(handle.as_ptr()) };
        // SAFETY: `openconnect_get_dtls_cipher` returns NULL or a valid C string.
        unsafe { c_str_to_option_string(cipher) }
    }

    pub fn get_peer_cert_hash(&self) -> Option<String> {
        let handle = self.lock_handle().ok()?;
        // SAFETY: `handle.as_ptr()` is exclusively locked.
        let hash = unsafe { openconnect_get_peer_cert_hash(handle.as_ptr()) };
        // SAFETY: `openconnect_get_peer_cert_hash` returns NULL or a valid C string owned by C.
        unsafe { c_str_to_option_string(hash) }
    }

    pub fn disable_dtls(&self) -> OpenconnectResult<()> {
        let handle = self.lock_handle()?;
        // SAFETY: `handle.as_ptr()` is exclusively locked.
        let ret = unsafe { openconnect_disable_dtls(handle.as_ptr()) };
        match ret {
            0 => Ok(()),
            _ => Err(OpenconnectError::DisableDTLSError(ret)),
        }
    }

    pub fn set_http_proxy(&self, proxy: &str) -> OpenconnectResult<()> {
        let proxy =
            CString::new(proxy).map_err(|_| OpenconnectError::SetProxyError(libc::EINVAL))?;
        let handle = self.lock_handle()?;
        // SAFETY: `handle.as_ptr()` is exclusively locked; `libopenconnect` copies `proxy`.
        let ret = unsafe { openconnect_set_http_proxy(handle.as_ptr(), proxy.as_ptr()) };
        match ret {
            0 => Ok(()),
            _ => Err(OpenconnectError::SetProxyError(ret)),
        }
    }

    pub fn parse_url(&self, url: &str) -> OpenconnectResult<()> {
        let url = CString::new(url).map_err(|_| OpenconnectError::ParseUrlError(libc::EINVAL))?;
        let handle = self.lock_handle()?;
        // SAFETY: `handle.as_ptr()` is exclusively locked; `libopenconnect` copies `url`.
        let ret = unsafe { openconnect_parse_url(handle.as_ptr(), url.as_ptr()) };
        if ret != 0 {
            return Err(OpenconnectError::ParseUrlError(ret));
        }

        // SAFETY: `handle.as_ptr()` is exclusively locked.
        let raw_host = unsafe { openconnect_get_hostname(handle.as_ptr()) };
        // SAFETY: `openconnect_get_hostname` returns NULL or a valid C string.
        let host = unsafe { c_str_to_option_string(raw_host) };
        if let Ok(mut cached) = self.state.cached_hostname.write() {
            *cached = host;
        }
        Ok(())
    }

    pub fn get_server_name(&self) -> Option<String> {
        let entrypoint = self.state.entrypoint.read().ok()?;
        (*entrypoint).as_ref()?.name.clone()
    }

    pub fn get_server_url(&self) -> Option<String> {
        let entrypoint = self.state.entrypoint.read().ok()?;
        Some((*entrypoint).as_ref()?.server.clone())
    }

    pub fn get_port(&self) -> i32 {
        let Ok(handle) = self.lock_handle() else {
            return 0;
        };
        // SAFETY: `handle.as_ptr()` is exclusively locked.
        unsafe { openconnect_get_port(handle.as_ptr()) }
    }

    /// Returns the VPN hostname. If `run_loop` is currently holding the C handle lock,
    /// returns the cached hostname snapshot without racing on `openconnect_info`.
    pub fn get_hostname(&self) -> Option<String> {
        if let Ok(handle) = self.handle.try_lock() {
            // SAFETY: `handle.as_ptr()` is exclusively locked.
            let hostname = unsafe { openconnect_get_hostname(handle.as_ptr()) };
            // SAFETY: `openconnect_get_hostname` returns NULL or a valid C string.
            let host = unsafe { c_str_to_option_string(hostname) };
            if let Ok(mut cached) = self.state.cached_hostname.write() {
                *cached = host.clone();
            }
            return host;
        }
        self.state.cached_hostname.read().ok()?.clone()
    }

    pub fn set_client_cert(&self, cert: &str, sslkey: &str) -> OpenconnectResult<()> {
        let cert =
            CString::new(cert).map_err(|_| OpenconnectError::SetClientCertError(libc::EINVAL))?;
        let sslkey =
            CString::new(sslkey).map_err(|_| OpenconnectError::SetClientCertError(libc::EINVAL))?;
        let handle = self.lock_handle()?;
        // SAFETY: `handle.as_ptr()` is exclusively locked; `libopenconnect` copies both strings.
        let ret =
            unsafe { openconnect_set_client_cert(handle.as_ptr(), cert.as_ptr(), sslkey.as_ptr()) };
        match ret {
            0 => Ok(()),
            _ => Err(OpenconnectError::SetClientCertError(ret)),
        }
    }

    pub fn set_mca_cert(&self, cert: &str, key: &str) -> OpenconnectResult<()> {
        let cert =
            CString::new(cert).map_err(|_| OpenconnectError::SetMCACertError(libc::EINVAL))?;
        let key = CString::new(key).map_err(|_| OpenconnectError::SetMCACertError(libc::EINVAL))?;
        let handle = self.lock_handle()?;
        // SAFETY: `handle.as_ptr()` is exclusively locked; `libopenconnect` copies both strings.
        let ret = unsafe { openconnect_set_mca_cert(handle.as_ptr(), cert.as_ptr(), key.as_ptr()) };
        match ret {
            0 => Ok(()),
            _ => Err(OpenconnectError::SetMCACertError(ret)),
        }
    }

    fn query_ip_info_locked(handle: &VpnInfoHandle) -> OpenconnectResult<Option<IpInfo>> {
        let mut info_ptr: *const oc_ip_info = std::ptr::null();
        // SAFETY: `handle.as_ptr()` is exclusively locked and `&mut info_ptr` is a valid out-pointer.
        let ret = unsafe {
            openconnect_get_ip_info(
                handle.as_ptr(),
                &mut info_ptr,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if ret != 0 {
            return Err(OpenconnectError::GetIpInfoError(ret));
        }
        // SAFETY: `info_ptr`, if non-null, points to a valid `oc_ip_info` owned by `handle`.
        Ok(unsafe { IpInfo::from_raw(info_ptr) })
    }

    /// Returns IP configuration info. If `run_loop` holds the C handle lock, returns the
    /// cached snapshot captured after CSTP setup without racing on `openconnect_info`.
    pub fn get_info(&self) -> OpenconnectResult<Option<IpInfo>> {
        if let Ok(handle) = self.handle.try_lock() {
            let info = Self::query_ip_info_locked(&handle)?;
            if let Ok(mut cached) = self.state.cached_ip_info.write() {
                *cached = info.clone();
            }
            return Ok(info);
        }
        Ok(self
            .state
            .cached_ip_info
            .read()
            .ok()
            .and_then(|c| c.clone()))
    }

    pub(crate) fn main_loop(
        &self,
        reconnect_timeout: i32,
        reconnect_interval: u32,
    ) -> OpenconnectResult<()> {
        let reconnect_interval = i32::try_from(reconnect_interval).unwrap_or(i32::MAX);
        let handle = self.lock_handle()?;
        // SAFETY: `handle.as_ptr()` is exclusively locked for the duration of `openconnect_mainloop`.
        let ret =
            unsafe { openconnect_mainloop(handle.as_ptr(), reconnect_timeout, reconnect_interval) };
        match ret {
            0 => Ok(()),
            _ => Err(OpenconnectError::MainLoopError(ret)),
        }
    }

    /// Test-only helpers for exercising C mock fault injection & callback triggers under `mock-ffi`.
    #[cfg(feature = "mock-ffi")]
    pub fn with_mock_vpninfo<R>(
        &self,
        f: impl FnOnce(*mut openconnect_sys::openconnect_info) -> R,
    ) -> R {
        let handle = self.lock_handle().expect("vpninfo lock poisoned");
        f(handle.as_ptr())
    }

    #[cfg(feature = "mock-ffi")]
    pub fn save_mock_form_field(&self, form_id: &str, opt_id: &str, value: Option<&str>) {
        if let Ok(mut fm) = self.state.form_manager.lock() {
            fm.save_field(
                form_id.to_string(),
                opt_id.to_string(),
                value.map(str::to_string),
            );
        }
    }
}

impl Drop for VpnClient {
    fn drop(&mut self) {
        self.disconnect();
        // `self.handle` (`VpnInfoHandle`) drops automatically after `VpnClient::drop`,
        // clearing `cmd_fd`, calling `openconnect_vpninfo_free`, and freeing `CallbackContext`.
    }
}

/// Trait for creating a new instance of [`VpnClient`] and connecting to the VPN server.
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
    fn new(config: Config, callbacks: EventHandlers) -> OpenconnectResult<Arc<Self>> {
        let useragent = CString::new("AnyConnect-compatible OpenConnect VPN Agent")
            .map_err(|_| OpenconnectError::OtherError("useragent is not valid".to_string()))?;

        let state = Arc::new(SharedState {
            config,
            cmd_fd: Mutex::new(None),
            status: RwLock::new(Status::Initialized),
            callbacks,
            entrypoint: RwLock::new(None),
            form_manager: Mutex::new(FormManager::default()),
            peer_certs: PeerCerts::default(),
            cached_hostname: RwLock::new(None),
            cached_ip_info: RwLock::new(None),
        });

        let handle = VpnInfoHandle::new(&useragent, Arc::clone(&state))?;

        let instance = Arc::new(Self {
            handle: Mutex::new(handle),
            state,
        });

        SIGNAL_HANDLE.update_client_singleton(Arc::downgrade(&instance));
        instance.set_loglevel(instance.state.config.loglevel);
        instance.set_setup_tun_handler();

        if let Some(proxy) = &instance.state.config.http_proxy {
            instance
                .set_http_proxy(proxy.as_str())
                .emit_error(&instance)?;
        }

        instance.emit_state_change(Status::Initialized);

        Ok(instance)
    }

    fn connect_for_cookie(&self, entrypoint: Entrypoint) -> OpenconnectResult<Option<String>> {
        self.emit_state_change(Status::Connecting("Initializing connection".to_string()));
        if let Ok(mut form_context) = self.state.form_manager.lock() {
            form_context.reset();
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
                .state
                .entrypoint
                .write()
                .map_err(|_| {
                    OpenconnectError::EntrypointConfigError(
                        "write entrypoint lock failed".to_string(),
                    )
                })
                .emit_error(self)?;

            *entrypoint_write_guard = Some(entrypoint.clone());
        }

        if !entrypoint.enable_udp {
            self.disable_dtls().emit_error(self)?;
        }

        self.emit_state_change(Status::Connecting("Parsing URL".to_string()));
        self.parse_url(&entrypoint.server).emit_error(self)?;
        let hostname = self.get_hostname();

        self.emit_state_change(Status::Connecting(format!(
            "Obtaining cookie from: {}",
            hostname.unwrap_or_default()
        )));
        if let Some(cookie) = entrypoint.cookie.as_deref() {
            self.set_cookie(cookie).emit_error(self)?;
        } else {
            self.obtain_cookie().emit_error(self)?;
        }

        Ok(self.get_cookie())
    }

    fn init_connection(&self, entrypoint: Entrypoint) -> OpenconnectResult<()> {
        self.emit_state_change(Status::Connecting("Make CSTP connection".to_string()));
        self.connect_for_cookie(entrypoint)?;
        self.make_cstp_connection().emit_error(self)?;
        self.emit_state_change(Status::Connected);

        Ok(())
    }

    fn run_loop(&self) -> OpenconnectResult<()> {
        loop {
            if let Err(err) = self.main_loop(300, RECONNECT_INTERVAL_MIN) {
                tracing::event!(tracing::Level::ERROR, "Main loop failed: {}", err);
                break;
            }
        }

        self.emit_state_change(Status::Disconnected);
        Ok(())
    }

    fn disconnect(&self) {
        if self.get_status() != Status::Connected {
            return;
        }

        self.emit_state_change(Status::Disconnecting);
        self.send_command(command::Command::Cancel);
        if let Ok(mut fd_guard) = self.state.cmd_fd.lock() {
            *fd_guard = None;
        }

        std::thread::sleep(std::time::Duration::from_millis(200));
    }

    fn get_server_name(&self) -> Option<String> {
        self.state
            .entrypoint
            .read()
            .ok()
            .and_then(|r| r.as_ref().and_then(|e| e.name.clone()))
    }

    fn get_status(&self) -> Status {
        self.state
            .status
            .read()
            .ok()
            .map_or(Status::Initialized, |r| r.clone())
    }
}

impl Events for VpnClient {
    fn emit_state_change(&self, status: Status) {
        if let Ok(mut write) = self.state.status.write() {
            *write = status.clone();
        }

        if let Some(ref handler) = self.state.callbacks.handle_connection_state_change {
            handler(status);
        }
    }

    fn emit_error(&self, error: &OpenconnectError) {
        self.emit_state_change(Status::Error(error.clone()));
    }
}
