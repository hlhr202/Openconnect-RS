#![feature(c_variadic)]
#![allow(clippy::box_collection)]

mod cert;
mod errno;
mod form;

use form::{Form, FormTrait};
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
        let ctx = privdata.cast::<OpenconnectCtx>();
        unsafe {
            let cipher = openconnect_get_dtls_cipher(*(*ctx));
            if !cipher.is_null() {
                let _dtls = std::ffi::CStr::from_ptr(cipher).to_str().unwrap();
            }
            // TODO: display stats, dtls
        }
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

        let instance = Box::into_raw(instance);

        let process_auth_form_cb = Form::process_auth_form_cb as *const ();
        let validate_peer_cert = validate_peer_cert as *const ();
        let write_process = write_process as *const ();

        let vpninfo = unsafe {
            // TODO: these pointers are weird
            openconnect_vpninfo_new(
                useragent.as_ptr(),
                std::mem::transmute(validate_peer_cert),
                None,
                std::mem::transmute(process_auth_form_cb),
                std::mem::transmute(write_process),
                instance as *mut ::std::os::raw::c_void,
            )
        };

        unsafe {
            (*instance).vpninfo = vpninfo;
            if (*instance).vpninfo.is_null() {
                panic!("openconnect_vpninfo_new failed");
            }
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
