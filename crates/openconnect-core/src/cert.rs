use crate::VpnClient;
use openconnect_sys::*;
use std::ffi::{CStr, CString};

#[derive(Debug, PartialEq, Eq)]
pub struct AcceptedCert {
    pub fingerprint: String,
    pub host: Option<String>,
    pub port: i32,
}

#[derive(Debug, Default)]
pub struct OpenSSLCert {
    pub accepted_certs: Vec<AcceptedCert>,
}

pub extern "C" fn validate_peer_cert(
    _privdata: *mut ::std::os::raw::c_void,
    _reason: *const ::std::os::raw::c_char,
) -> ::std::os::raw::c_int {
    let ctx = VpnClient::from_c_void(_privdata);
    let openssl_cert = unsafe { &mut (*ctx).certs.accepted_certs };

    unsafe {
        let vpninfo = (*ctx).vpninfo;
        let host = (*ctx).get_hostname();
        let port = (*ctx).get_port();
        let peer_fingerprint = openconnect_get_peer_cert_hash(vpninfo);

        for cert in openssl_cert.iter_mut().rev() {
            if (host.is_none() || cert.host == host) && (port == 0 || cert.port == port) {
                let fingerprint_in_cstr =
                    CString::new(cert.fingerprint.as_str()).expect("Invalid fingerprint");
                let err = openconnect_check_peer_cert_hash(vpninfo, fingerprint_in_cstr.as_ptr());
                if err == 0 {
                    return 0;
                }
                if err < 0 {
                    // TODO: log error
                    println!("Could not check peer cert hash: {}", cert.fingerprint);
                }
            }
        }

        // SAFETY: we should not use CString::from_raw(peer_fingerprint)
        // because peer_fingerprint will be deallocated in rust and cause a double free
        let fingerprint = CStr::from_ptr(peer_fingerprint)
            .to_string_lossy()
            .to_string();

        if (*ctx).handle_accept_insecure_cert(&fingerprint) {
            let newcert = AcceptedCert {
                fingerprint,
                host,
                port,
            };
            openssl_cert.push(newcert);
            println!("User accepted insecure certificate");
            0
        } else {
            println!("User rejected insecure certificate");
            1
        }
    }
}
