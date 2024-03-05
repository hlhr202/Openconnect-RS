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

unsafe extern "C" fn stats_fn(privdata: *mut ::std::os::raw::c_void, _stats: *const oc_stats) {
    let ctx = privdata.cast::<OpenconnectCtx>();
    let cipher = openconnect_get_dtls_cipher(*(*ctx));
    if !cipher.is_null() {
        let _dtls = std::ffi::CStr::from_ptr(cipher).to_str().unwrap();
    }

    // TODO: display stats, dtls
}

unsafe extern "C" fn setup_tun_vfn(privdata: *mut ::std::os::raw::c_void) {
    let ctx = privdata.cast::<OpenconnectCtx>();
    let vpnc_script = DEFAULT_VPNCSCRIPT;
    let ret = openconnect_setup_tun_device(
        *(*ctx),
        vpnc_script.as_ptr() as *const i8,
        std::ptr::null_mut(),
    );
    println!("setup_tun_device ret: {}", ret);
}

lazy_static! {
    pub static ref PROCESS_AUTH_FORM_CB: unsafe extern "C" fn(
        privdata: *mut ::std::os::raw::c_void,
        form: *mut openconnect_sys::oc_auth_form,
    ) -> ::std::os::raw::c_int = process_auth_form_cb;
    pub static ref WRITE_PROCESS: unsafe extern "C" fn(
        *mut ::std::os::raw::c_void,
        ::std::os::raw::c_int,
        *const ::std::os::raw::c_char,
        ...
    ) = write_process;
    pub static ref STATS_FN: unsafe extern "C" fn(*mut ::std::os::raw::c_void, *const oc_stats) =
        stats_fn;
    pub static ref SETUP_TUN_VFN: unsafe extern "C" fn(*mut ::std::os::raw::c_void) = setup_tun_vfn;
}

lazy_static! {
    // TODO: Optimize memory allocation or avoid using Box
    pub static ref USER: Box<String> = Box::new(env::var("USER").unwrap_or("".to_string()));
    pub static ref SERVER: Box<String> = Box::new(env::var("SERVER").unwrap_or("".to_string()));
    pub static ref PASSWORD: Box<String> = Box::new(env::var("PASSWORD").unwrap_or("".to_string()));
}

pub fn init_global_statics() {
    dotenvy::from_path(".env.local").unwrap();
    initialize(&USER);
    initialize(&SERVER);
    initialize(&PASSWORD);

    initialize(&PROCESS_AUTH_FORM_CB);
    initialize(&WRITE_PROCESS);
    initialize(&STATS_FN);
    initialize(&SETUP_TUN_VFN);
}

impl OpenconnectCtx {
    unsafe extern "C" fn validate_peer_cert(
        _privdata: *mut ::std::os::raw::c_void,
        _reason: *const ::std::os::raw::c_char,
    ) -> ::std::os::raw::c_int {
        println!("validate_peer_cert");
        0
    }

    pub fn new() -> Self {
        let useragent = "AnyConnect-compatible OpenConnect VPN Agent";

        let mut instance = Self {
            vpninfo: std::ptr::null_mut(),
        };

        let vpninfo = unsafe {
            openconnect_vpninfo_new(
                useragent.as_ptr() as *const i8,
                Some(OpenconnectCtx::validate_peer_cert),
                None,
                Some(*PROCESS_AUTH_FORM_CB),
                Some(*WRITE_PROCESS),
                &instance as *const _ as *mut _,
            )
        };

        instance.vpninfo = vpninfo;

        instance
    }
}

impl Default for OpenconnectCtx {
    fn default() -> Self {
        Self::new()
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
