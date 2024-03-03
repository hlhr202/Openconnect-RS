#![feature(c_variadic)]
#![feature(pointer_is_aligned)]
// #![allow(unused)]
#![allow(clippy::box_collection)]
#![allow(clippy::just_underscores_and_digits)]

use std::{
    env,
    ops::{Deref, DerefMut},
};

use form::process_auth_form_cb;
use lazy_static::{initialize, lazy_static};
use openconnect_sys::{
    oc_stats, openconnect_get_dtls_cipher, openconnect_info, openconnect_setup_tun_device,
    DEFAULT_VPNCSCRIPT,
};

mod errno;
mod form;

#[repr(C)]
pub struct OpenconnectInfo {
    pub vpninfo: *mut openconnect_info,
}

// struct AcceptCert {
//     next: Rc<AcceptCert>,
//     fingerprint: String,
//     host: String,
//     port: u16,
// }

// TODO: complete tls implementation
unsafe extern "C" fn validate_peer_cert(
    _privdata: *mut ::std::os::raw::c_void,
    _reason: *const ::std::os::raw::c_char,
) -> ::std::os::raw::c_int {
    println!("validate_peer_cert");
    // let vpninfo = privdata.cast::<openconnect_info>();
    // let fingerprint = openconnect_get_peer_cert_hash(vpninfo);
    // let this: Rc<AcceptCert>;

    // TODO: review certificate validation
    // let mut root_store = RootCertStore::empty();
    // let der = Der::from_slice(std::slice::from_raw_parts(der, der_size as usize));
    // let cert = CertificateDer::from(der.to_vec());
    // root_store.add(cert);
    // let root_store = Arc::new(root_store);

    // let client_config = ClientConfig::builder()
    //     .with_root_certificates(root_store.clone())
    //     .with_no_client_auth();

    // let verifier = WebPkiClientVerifier::builder(root_store).build().unwrap();

    // let details = openconnect_get_peer_cert_details(vpninfo);

    0
}

unsafe extern "C" fn write_process(
    _privdata: *mut ::std::os::raw::c_void,
    _level: ::std::os::raw::c_int,
    fmt: *const ::std::os::raw::c_char,
    _args: ...
) {
    let fmt = std::ffi::CStr::from_ptr(fmt).to_str().unwrap();
    let level = match _level as u32 {
        openconnect_sys::PRG_ERR => "ERR",
        openconnect_sys::PRG_INFO => "INFO",
        openconnect_sys::PRG_DEBUG => "DEBUG",
        openconnect_sys::PRG_TRACE => "TRACE",
        _ => "UNKNOWN",
    };
    print!("level: {}, ", level);
    print!("fmt: {}", fmt);
    // print args
    // let mut args = args;
    // let arg = args.arg::<*mut i8>();
    // if !arg.is_null() {
    //     let arg = std::ffi::CStr::from_ptr(arg).to_str().unwrap_or("");
    //     println!("arg: {}", arg);
    // }
}

unsafe extern "C" fn stats_fn(privdata: *mut ::std::os::raw::c_void, _stats: *const oc_stats) {
    let vpninfo = privdata.cast::<openconnect_info>();
    let cipher = openconnect_get_dtls_cipher(vpninfo);
    if !cipher.is_null() {
        let _dtls = std::ffi::CStr::from_ptr(cipher).to_str().unwrap();
    }

    // TODO: display stats, dtls
}

unsafe extern "C" fn setup_tun_vfn(privdata: *mut ::std::os::raw::c_void) {
    let vpninfo = privdata.cast::<openconnect_info>();
    let vpnc_script = DEFAULT_VPNCSCRIPT;
    let ret = openconnect_setup_tun_device(
        vpninfo,
        vpnc_script.as_ptr() as *const i8,
        std::ptr::null_mut(),
    );
    println!("setup_tun_device ret: {}", ret);
}

lazy_static! {
    pub static ref VALIDATE_PEER_CERT: unsafe extern "C" fn(
        *mut ::std::os::raw::c_void,
        *const ::std::os::raw::c_char,
    ) -> ::std::os::raw::c_int = validate_peer_cert;
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

    initialize(&VALIDATE_PEER_CERT);
    initialize(&PROCESS_AUTH_FORM_CB);
    initialize(&WRITE_PROCESS);
    initialize(&STATS_FN);
    initialize(&SETUP_TUN_VFN);
}

impl OpenconnectInfo {
    pub fn new() -> Self {
        let useragent = "AnyConnect-compatible OpenConnect VPN Agent";

        let vpninfo = unsafe {
            openconnect_sys::openconnect_vpninfo_new(
                useragent.as_ptr() as *const i8,
                Some(*VALIDATE_PEER_CERT),
                None,
                Some(*PROCESS_AUTH_FORM_CB),
                Some(*WRITE_PROCESS),
                std::ptr::null_mut(),
            )
        };

        Self { vpninfo }
    }
}

impl Default for OpenconnectInfo {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for OpenconnectInfo {
    type Target = *mut openconnect_info;

    fn deref(&self) -> &Self::Target {
        &self.vpninfo
    }
}

impl DerefMut for OpenconnectInfo {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.vpninfo
    }
}
