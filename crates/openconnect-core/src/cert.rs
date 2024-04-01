use crate::VpnClient;
use openconnect_sys::*;
use std::{ffi::CString, sync::Mutex};

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct AcceptedCert {
    pub fingerprint: String,
    pub host: Option<String>,
    pub port: i32,
}

#[derive(Debug, Default)]
pub(crate) struct PeerCerts {
    pub accepted_certs: Mutex<Vec<AcceptedCert>>,
}

pub(crate) extern "C" fn validate_peer_cert(
    privdata: *mut ::std::os::raw::c_void,
    _reason: *const ::std::os::raw::c_char,
) -> ::std::os::raw::c_int {
    let client = unsafe { VpnClient::from_raw(privdata) };
    let vpninfo = client.vpninfo;
    let host = client.get_hostname();
    let port = client.get_port();

    let openssl_cert_guard = client.peer_certs.accepted_certs.lock();
    if let Ok(openssl_cert) = openssl_cert_guard {
        for cert in openssl_cert.iter().rev() {
            if (host.is_none() || cert.host == host) && (port == 0 || cert.port == port) {
                let fingerprint_in_cstr =
                    CString::new(cert.fingerprint.as_str()).expect("Invalid fingerprint");
                let err = unsafe {
                    openconnect_check_peer_cert_hash(vpninfo, fingerprint_in_cstr.as_ptr())
                };
                if err == 0 {
                    return 0;
                }
                if err < 0 {
                    // TODO: log error
                    println!("Could not check peer cert hash: {}", cert.fingerprint);
                }
            }
        }
    }

    let fingerprint = client.get_peer_cert_hash();

    if client.handle_accept_insecure_cert(&fingerprint) {
        let newcert = AcceptedCert {
            fingerprint,
            host,
            port,
        };
        let openssl_cert_guard = client.peer_certs.accepted_certs.lock();
        if let Ok(mut openssl_cert) = openssl_cert_guard {
            openssl_cert.push(newcert);
        }
        println!("User accepted insecure certificate");
        0
    } else {
        println!("User rejected insecure certificate");
        1
    }
}
