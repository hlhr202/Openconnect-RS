#![feature(c_variadic)]
#![allow(clippy::box_collection)]

use std::{
    env,
    ops::{Deref, DerefMut},
};

use form::process_auth_form_cb;
use lazy_static::{initialize, lazy_static};
use openconnect_sys::*;

mod cert;
mod errno;
mod form;

#[repr(C)]
pub struct OpenconnectCtx {
    pub vpninfo: *mut openconnect_info,
    pub user: String,
    pub server: String,
    pub password: String,
}

// struct AcceptCert {
//     next: Rc<AcceptCert>,
//     fingerprint: String,
//     host: String,
//     port: u16,
// }

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

lazy_static! {
    pub static ref WRITE_PROCESS: unsafe extern "C" fn(
        *mut ::std::os::raw::c_void,
        ::std::os::raw::c_int,
        *const ::std::os::raw::c_char,
        ...
    ) = write_process;
}

lazy_static! {
    // TODO: Optimize memory allocation or avoid using Box
    pub static ref SERVER: Box<String> = Box::new(env::var("SERVER").unwrap_or("".to_string()));
}

pub fn init_global_statics() {
    dotenvy::from_path(".env.local").unwrap();
    initialize(&SERVER);
    initialize(&WRITE_PROCESS);
}

impl OpenconnectCtx {
    pub extern "C" fn stats_fn(privdata: *mut ::std::os::raw::c_void, _stats: *const oc_stats) {
        let ctx = privdata.cast::<OpenconnectCtx>();
        unsafe {
            let cipher = openconnect_get_dtls_cipher(*(*ctx));
            if !cipher.is_null() {
                let _dtls = std::ffi::CStr::from_ptr(cipher).to_str().unwrap();
            }
            // TODO: display stats, dtls
        }
    }

    pub extern "C" fn validate_peer_cert(
        _privdata: *mut ::std::os::raw::c_void,
        _reason: *const ::std::os::raw::c_char,
    ) -> ::std::os::raw::c_int {
        println!("validate_peer_cert");
        0
    }

    pub extern "C" fn setup_tun_vfn(privdata: *mut ::std::os::raw::c_void) {
        let ctx = privdata.cast::<OpenconnectCtx>();
        let vpnc_script = DEFAULT_VPNCSCRIPT;
        unsafe {
            let ret = openconnect_setup_tun_device(
                *(*ctx),
                vpnc_script.as_ptr() as *const i8,
                std::ptr::null_mut(),
            );
            println!("setup_tun_device ret: {}", ret);
        }
    }

    pub fn new() -> *mut Self {
        let useragent = "AnyConnect-compatible OpenConnect VPN Agent";
        let user = env::var("USER").unwrap_or("".to_string());
        let server = env::var("SERVER").unwrap_or("".to_string());
        let password = env::var("PASSWORD").unwrap_or("".to_string());

        let instance = Box::new(Self {
            vpninfo: std::ptr::null_mut(),
            user,
            server,
            password,
        });

        let instance = Box::into_raw(instance);

        let vpninfo = unsafe {
            openconnect_vpninfo_new(
                useragent.as_ptr() as *const i8,
                Some(OpenconnectCtx::validate_peer_cert),
                None,
                Some(process_auth_form_cb),
                Some(*WRITE_PROCESS),
                instance as *mut ::std::os::raw::c_void,
            )
        };

        unsafe {
            (*instance).vpninfo = vpninfo;
        }

        instance
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
        unsafe {
            openconnect_vpninfo_free(self.vpninfo);
        }
    }
}
