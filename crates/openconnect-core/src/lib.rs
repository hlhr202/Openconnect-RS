#![feature(c_variadic)]
#![allow(clippy::box_collection)]

mod cert;
mod errno;
mod form;

use form::Form;
use openconnect_sys::*;
use std::{
    env,
    ffi::CString,
    ops::{Deref, DerefMut},
};

#[repr(C)]
pub struct OpenconnectCtx {
    pub vpninfo: *mut openconnect_info,
    pub user: CString,
    pub server: CString,
    pub password: CString,
}

pub enum LogLevel {
    Err = PRG_ERR as isize,
    Info = PRG_INFO as isize,
    Debug = PRG_DEBUG as isize,
    Trace = PRG_TRACE as isize,
}

#[no_mangle]
unsafe extern "C" fn write_process(
    _privdata: *mut ::std::os::raw::c_void,
    _level: ::std::os::raw::c_int,
    fmt: *const ::std::os::raw::c_char,
    _args: ...
) {
    let fmt_str = std::ffi::CStr::from_ptr(fmt).to_str().unwrap();
    let level = match _level as u32 {
        PRG_ERR => "ERR",
        PRG_INFO => "INFO",
        PRG_DEBUG => "DEBUG",
        PRG_TRACE => "TRACE",
        _ => "UNKNOWN",
    };
    print!("level: {}, ", level);
    print!("fmt: {}", fmt_str);

    // libc::printf(fmt, _args);
}

#[no_mangle]
pub extern "C" fn validate_peer_cert(
    _privdata: *mut ::std::os::raw::c_void,
    _reason: *const ::std::os::raw::c_char,
) -> ::std::os::raw::c_int {
    println!("validate_peer_cert");
    0
}

pub fn init_global_statics() {
    dotenvy::from_path(".env.local").unwrap();
}

impl OpenconnectCtx {
    pub extern "C" fn stats_fn(privdata: *mut ::std::os::raw::c_void, _stats: *const oc_stats) {
        println!("stats_fn");
        let ctx = OpenconnectCtx::from_c_void(privdata);
        unsafe {
            let cipher = openconnect_get_dtls_cipher((*ctx).vpninfo);
            if !cipher.is_null() {
                let _dtls = std::ffi::CStr::from_ptr(cipher).to_str().unwrap();
            }
            // TODO: display stats, dtls
        }
    }

    pub extern "C" fn setup_tun_vfn(privdata: *mut ::std::os::raw::c_void) {
        let ctx = OpenconnectCtx::from_c_void(privdata);
        let vpnc_script = DEFAULT_VPNCSCRIPT;
        unsafe {
            let ret = openconnect_setup_tun_device(
                (*ctx).vpninfo,
                vpnc_script.as_ptr() as *const i8,
                std::ptr::null_mut(),
            );
            println!("setup_tun_device ret: {}", ret);
        }
    }

    pub fn from_c_void(ptr: *mut std::os::raw::c_void) -> *mut Self {
        unsafe { Box::leak(Box::from_raw(ptr.cast())) }
    }

    pub fn new() -> Box<Self> {
        let useragent =
            std::ffi::CString::new("AnyConnect-compatible OpenConnect VPN Agent").unwrap();
        let user = CString::new(env::var("USER").unwrap_or("".to_string())).unwrap();
        let server = CString::new(env::var("SERVER").unwrap_or("".to_string())).unwrap();
        let password = CString::new(env::var("PASSWORD").unwrap_or("".to_string())).unwrap();

        let instance = Box::new(Self {
            vpninfo: std::ptr::null_mut(),
            user,
            server,
            password,
        });

        let instance = Box::into_raw(instance); // leak for assign to vpninfo
        let process_auth_form_cb = Box::into_raw(Box::new(Form::process_auth_form_cb));
        let validate_peer_cert = Box::into_raw(Box::new(validate_peer_cert));
        let write_process = Box::into_raw(Box::new(write_process));

        unsafe {
            let ret = openconnect_init_ssl();
            if ret != 0 {
                panic!("openconnect_init_ssl failed");
            }

            let vpninfo = openconnect_vpninfo_new(
                // TODO: these pointers are weird
                useragent.as_ptr(),
                Some(*validate_peer_cert),
                None,
                Some(*process_auth_form_cb),
                Some(*write_process),
                instance as *mut ::std::os::raw::c_void,
            );

            if vpninfo.is_null() {
                panic!("openconnect_vpninfo_new failed");
            }

            let mut instance = Box::from_raw(instance); // reclaim ownership
            instance.vpninfo = vpninfo;
            instance
        }
    }

    pub fn set_loglevel(&self, level: LogLevel) {
        unsafe {
            openconnect_set_loglevel(self.vpninfo, level as i32);
        }
    }

    pub fn obtain_cookie(&self) {
        unsafe {
            let ret = openconnect_obtain_cookie(self.vpninfo);
            println!("cookie ret: {}", ret);
        }
    }

    pub fn setup_cmd_pipe(&self) {
        unsafe {
            let cmd_fd = openconnect_setup_cmd_pipe(self.vpninfo);
            println!("cmd_fd: {}", cmd_fd);
            libc::fcntl(
                cmd_fd,
                libc::F_SETFD,
                libc::fcntl(cmd_fd, libc::F_GETFL) & !libc::O_NONBLOCK,
            );
        }
    }

    pub fn make_cstp_connection(&self) {
        unsafe {
            let ret = openconnect_make_cstp_connection(self.vpninfo);
            println!("cstp ret: {}", ret);
        }
    }

    pub fn set_http_proxy(&self, proxy: &str) -> Result<(), std::ffi::NulError> {
        let proxy = std::ffi::CString::new(proxy)?;
        unsafe {
            let ret = openconnect_set_http_proxy(self.vpninfo, proxy.as_ptr());
            println!("set_http_proxy ret: {}", ret);
        }
        Ok(())
    }
}

impl Deref for OpenconnectCtx {
    type Target = *mut openconnect_info;

    fn deref(&self) -> &Self::Target {
        &self.vpninfo
    }
}

impl DerefMut for OpenconnectCtx {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.vpninfo
    }
}

impl Drop for OpenconnectCtx {
    fn drop(&mut self) {
        println!("drop OpenconnectCtx");
        unsafe {
            openconnect_vpninfo_free(self.vpninfo);
        }
    }
}
