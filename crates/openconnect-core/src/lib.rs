#![feature(c_variadic)]
#![feature(pointer_is_aligned)]
// #![allow(unused)]
#![allow(clippy::just_underscores_and_digits)]

use std::{
    env,
    ops::{Deref, DerefMut},
};

use form::process_auth_form_cb;
use lazy_static::lazy_static;
use openconnect_sys::openconnect_info;

mod errno;
mod form;

#[repr(C)]
pub struct OpenconnectInfo {
    pub info: *mut openconnect_info,
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
    _fmt: *const ::std::os::raw::c_char,
    ...
) {
}

lazy_static! {
    static ref VALIDATE_PEER_CERT: unsafe extern "C" fn(
        *mut ::std::os::raw::c_void,
        *const ::std::os::raw::c_char,
    ) -> ::std::os::raw::c_int = validate_peer_cert;
    static ref PROCESS_AUTH_FORM_CB: unsafe extern "C" fn(
        privdata: *mut ::std::os::raw::c_void,
        form: *mut openconnect_sys::oc_auth_form,
    ) -> ::std::os::raw::c_int = process_auth_form_cb;
    static ref WRITE_PROCESS: unsafe extern "C" fn(
        *mut ::std::os::raw::c_void,
        ::std::os::raw::c_int,
        *const ::std::os::raw::c_char,
        ...
    ) = write_process;
}

lazy_static! {
    pub static ref USER: String = env::var("USER").unwrap_or("".to_string());
    pub static ref SERVER: String = env::var("SERVER").unwrap_or("".to_string());
    pub static ref PASSWORD: String = env::var("PASSWORD").unwrap_or("".to_string());
}

pub fn init() {
    dotenvy::from_path(".env.local").unwrap();
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

        Self { info: vpninfo }
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
        &self.info
    }
}

impl DerefMut for OpenconnectInfo {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.info
    }
}

#[test]
fn test_openconnect_info() {
    use openconnect_sys::{
        openconnect_get_cookie, openconnect_get_hostname, openconnect_get_port,
        openconnect_init_ssl, openconnect_make_cstp_connection, openconnect_obtain_cookie,
        openconnect_parse_url, openconnect_set_loglevel, PRG_DEBUG,
    };

    init();

    println!("PASSWORD: {}", *PASSWORD);
    println!("USER: {}", *USER);
    println!("SERVER: {}", *SERVER);

    unsafe {
        openconnect_init_ssl();
        let vpninfo = OpenconnectInfo::new();

        openconnect_set_loglevel(*vpninfo, PRG_DEBUG as i32);

        openconnect_parse_url(*vpninfo, SERVER.as_ptr() as *const i8);

        let port = openconnect_get_port(*vpninfo);
        println!("port: {}", port);

        let hostname = openconnect_get_hostname(*vpninfo);
        let hostname = std::ffi::CStr::from_ptr(hostname).to_str().unwrap();
        println!("hostname: {}", hostname);

        let cookie = openconnect_get_cookie(*vpninfo);
        if cookie.is_null() {
            let ret = openconnect_obtain_cookie(*vpninfo);
            println!("ret: {}", ret);
        }

        let ret = openconnect_make_cstp_connection(*vpninfo);
        println!("ret: {}", ret);
    }
}
