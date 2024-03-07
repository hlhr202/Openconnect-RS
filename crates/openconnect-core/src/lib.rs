#![feature(c_variadic)]
#![allow(clippy::box_collection)]

pub mod cert;
pub mod config;
pub mod form;
pub mod protocols;
pub mod result;

use config::Config;
use form::Form;
pub use openconnect_sys::*;
use result::{OpenConnectError, OpenConnectResult};
use std::{
    ffi::CString,
    ops::{Deref, DerefMut},
    sync::Arc,
};

#[repr(C)]
pub struct VpnClient {
    pub vpninfo: *mut openconnect_info,
    pub config: Config,
    pub cmd_fd: i32,
}

// make openconnect_info thread shareable/tranferable
unsafe impl Send for VpnClient {}
unsafe impl Sync for VpnClient {}

pub enum LogLevel {
    Err = PRG_ERR as isize,
    Info = PRG_INFO as isize,
    Debug = PRG_DEBUG as isize,
    Trace = PRG_TRACE as isize,
}

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

    pub(crate) extern "C" fn stats_fn(
        privdata: *mut ::std::os::raw::c_void,
        _stats: *const oc_stats,
    ) {
        println!("stats_fn");
        let client = VpnClient::from_c_void(privdata);
        unsafe {
            let cipher = openconnect_get_dtls_cipher((*client).vpninfo);
            if !cipher.is_null() {
                let _dtls = std::ffi::CStr::from_ptr(cipher).to_str().unwrap();
            }
            // TODO: display stats, dtls
        }
    }

    pub(crate) extern "C" fn default_setup_tun_vfn(privdata: *mut ::std::os::raw::c_void) {
        let client = VpnClient::from_c_void(privdata);
        let vpnc_script = DEFAULT_VPNCSCRIPT;
        unsafe {
            let ret = openconnect_setup_tun_device(
                (*client).vpninfo,
                vpnc_script.as_ptr() as *const i8,
                std::ptr::null_mut(),
            );
            println!("setup_tun_device ret: {}", ret);
        }
    }

    pub(crate) fn from_c_void(ptr: *mut std::os::raw::c_void) -> *mut Self {
        ptr.cast()
    }

    pub(crate) fn to_raw_ptr(&self) -> *mut Self {
        self as *const Self as *mut Self
    }

    pub fn new(config: Config) -> Arc<Self> {
        let useragent =
            std::ffi::CString::new("AnyConnect-compatible OpenConnect VPN Agent").unwrap();

        let instance = Arc::new(Self {
            vpninfo: std::ptr::null_mut(),
            config,
            cmd_fd: -1,
        });

        let instance = Arc::into_raw(instance) as *mut VpnClient; // dangerous, leak for assign to vpninfo

        unsafe {
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
                Some(Form::process_auth_form_cb),
                Some(helper_format_vargs), // format args on C side
                instance as *mut ::std::os::raw::c_void,
            );

            if vpninfo.is_null() {
                panic!("openconnect_vpninfo_new failed");
            }

            (*instance).vpninfo = vpninfo;
            Arc::from_raw(instance) // reclaim ownership
        }
    }

    pub(crate) fn handle_text_input(&self, field_name: &str) -> Option<String> {
        match field_name {
            "username" | "user" | "uname" => self.config.username.clone(),
            _ => todo!("handle_text_input: {}", field_name),
        }
    }

    pub(crate) fn handle_password_input(&self) -> Option<String> {
        self.config.password.clone()
    }

    pub fn set_loglevel(&self, level: LogLevel) {
        unsafe {
            openconnect_set_loglevel(self.vpninfo, level as i32);
        }
    }

    pub fn set_protocol(&self, protocol: &str) -> OpenConnectResult<()> {
        let protocol = CString::new(protocol).unwrap();
        let ret = unsafe { openconnect_set_protocol(self.vpninfo, protocol.as_ptr()) };
        match ret {
            0 => Ok(()),
            _ => Err(OpenConnectError::SetProtocolError(ret)),
        }
    }

    pub fn set_stats_handler(&self) {
        unsafe {
            openconnect_set_stats_handler(self.vpninfo, Some(VpnClient::stats_fn));
        }
    }

    pub fn set_setup_tun_handler(&self) {
        unsafe {
            openconnect_set_setup_tun_handler(self.vpninfo, Some(VpnClient::default_setup_tun_vfn));
        }
    }

    pub fn set_report_os(&self, os: &str) -> OpenConnectResult<()> {
        let os = CString::new(os).unwrap();
        let ret = unsafe { openconnect_set_reported_os(self.vpninfo, os.as_ptr()) };
        match ret {
            0 => Ok(()),
            _ => Err(OpenConnectError::SetReportOSError(ret)),
        }
    }

    pub fn obtain_cookie(&self) -> OpenConnectResult<()> {
        let ret = unsafe { openconnect_obtain_cookie(self.vpninfo) };
        match ret {
            0 => Ok(()),
            _ => Err(result::OpenConnectError::ObtainCookieError(ret)),
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

    pub fn setup_cmd_pipe(&self) -> OpenConnectResult<()> {
        // mutates self but keep &self immutable
        let this = self.to_raw_ptr();
        unsafe {
            (*this).cmd_fd = openconnect_setup_cmd_pipe((*this).vpninfo);
            if (*this).cmd_fd < 0 {
                return Err(result::OpenConnectError::CmdPipeError((*this).cmd_fd));
            }
            libc::fcntl(
                (*this).cmd_fd,
                libc::F_SETFL,
                libc::fcntl((*this).cmd_fd, libc::F_GETFL) & !libc::O_NONBLOCK,
            );
        }
        Ok(())
    }

    pub fn make_cstp_connection(&self) -> OpenConnectResult<()> {
        let ret = unsafe { openconnect_make_cstp_connection(self.vpninfo) };
        match ret {
            0 => Ok(()),
            _ => Err(result::OpenConnectError::MakeCstpError(ret)),
        }
    }

    pub fn disable_dtls(&self) -> OpenConnectResult<()> {
        let ret = unsafe { openconnect_disable_dtls(self.vpninfo) };
        match ret {
            0 => Ok(()),
            _ => Err(result::OpenConnectError::DisableDTLSError(ret)),
        }
    }

    pub fn set_http_proxy(&self, proxy: &str) -> OpenConnectResult<()> {
        let proxy = CString::new(proxy).map_err(|_| OpenConnectError::SetProxyError(libc::EIO))?;
        let ret = unsafe { openconnect_set_http_proxy(self.vpninfo, proxy.as_ptr()) };
        match ret {
            0 => Ok(()),
            _ => Err(OpenConnectError::SetProxyError(ret)),
        }
    }

    pub fn parse_url(&self, url: Option<String>) -> OpenConnectResult<()> {
        match url {
            None => Err(OpenConnectError::ParseUrlError(libc::EIO)),
            Some(url) => {
                let url =
                    CString::new(url).map_err(|_| OpenConnectError::ParseUrlError(libc::EIO))?;
                let ret = unsafe { openconnect_parse_url(self.vpninfo, url.as_ptr()) };
                match ret {
                    0 => Ok(()),
                    _ => Err(OpenConnectError::ParseUrlError(ret)),
                }
            }
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

    pub fn set_client_cert(&self, cert: &str, sslkey: &str) -> OpenConnectResult<()> {
        let cert =
            CString::new(cert).map_err(|_| OpenConnectError::SetClientCertError(libc::EIO))?;
        let sslkey =
            CString::new(sslkey).map_err(|_| OpenConnectError::SetClientCertError(libc::EIO))?;
        let ret =
            unsafe { openconnect_set_client_cert(self.vpninfo, cert.as_ptr(), sslkey.as_ptr()) };
        match ret {
            0 => Ok(()),
            _ => Err(OpenConnectError::SetClientCertError(ret)),
        }
    }

    pub fn set_mca_cert(&self, cert: &str, key: &str) -> OpenConnectResult<()> {
        let cert = CString::new(cert).map_err(|_| OpenConnectError::SetMCACertError(libc::EIO))?;
        let key = CString::new(key).map_err(|_| OpenConnectError::SetMCACertError(libc::EIO))?;
        let ret = unsafe { openconnect_set_mca_cert(self.vpninfo, cert.as_ptr(), key.as_ptr()) };
        match ret {
            0 => Ok(()),
            _ => Err(OpenConnectError::SetMCACertError(ret)),
        }
    }

    pub fn main_loop(
        &self,
        reconnect_timeout: i32,
        reconnect_interval: u32,
    ) -> OpenConnectResult<()> {
        let ret = unsafe {
            openconnect_mainloop(self.vpninfo, reconnect_timeout, reconnect_interval as i32)
        };
        match ret {
            0 => Ok(()),
            _ => Err(OpenConnectError::MainLoopError(ret)),
        }
    }

    /// Gracefully stop the main loop
    pub fn stop_main_loop(&self) {
        let cmd = OC_CMD_CANCEL;
        let this = self.to_raw_ptr();
        unsafe {
            if (*this).cmd_fd != -1 {
                // TODO: implement under windows, should use libc::send
                let ret = libc::write((*this).cmd_fd, std::ptr::from_ref(&cmd) as *const _, 1);
                if ret < 0 {
                    println!("write cmd_fd failed");
                }
                (*this).cmd_fd = -1;
            }
        }
        println!("terminate");
    }

    pub fn free(&self) {
        unsafe {
            openconnect_vpninfo_free(self.vpninfo);
        }
        println!("free context");
    }
}

impl Deref for VpnClient {
    type Target = *mut openconnect_info;

    fn deref(&self) -> &Self::Target {
        &self.vpninfo
    }
}

impl DerefMut for VpnClient {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.vpninfo
    }
}

impl Drop for VpnClient {
    fn drop(&mut self) {
        self.free();
    }
}
