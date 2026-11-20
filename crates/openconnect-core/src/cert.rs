use crate::ffi::{CallbackContext, c_str_to_option_string, with_callback_context};
use openconnect_sys::{
    openconnect_check_peer_cert_hash, openconnect_get_hostname, openconnect_get_peer_cert_hash,
    openconnect_get_port,
};
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

impl PeerCerts {
    pub(crate) extern "C" fn validate_peer_cert(
        privdata: *mut ::std::os::raw::c_void,
        _reason: *const ::std::os::raw::c_char,
    ) -> ::std::os::raw::c_int {
        let body = |ctx: &CallbackContext| -> ::std::os::raw::c_int {
            let Some(vpninfo) = ctx.vpninfo_ptr() else {
                return 1;
            };
            let vpninfo_ptr = vpninfo.as_ptr();

            // SAFETY: `vpninfo_ptr` is valid for the duration of this synchronous callback.
            let raw_host = unsafe { openconnect_get_hostname(vpninfo_ptr) };
            // SAFETY: `openconnect_get_hostname` returns NULL or a valid C string.
            let host = unsafe { c_str_to_option_string(raw_host) };
            // SAFETY: `vpninfo_ptr` is valid for the duration of this synchronous callback.
            let port = unsafe { openconnect_get_port(vpninfo_ptr) };

            if let Ok(accepted_certs) = ctx.state.peer_certs.accepted_certs.lock() {
                for cert in accepted_certs.iter().rev() {
                    if (host.is_none() || cert.host == host) && (port == 0 || cert.port == port) {
                        let Ok(fingerprint_in_cstr) = CString::new(cert.fingerprint.as_str())
                        else {
                            tracing::warn!(
                                "Skipping stored certificate fingerprint with interior NUL"
                            );
                            continue;
                        };
                        // SAFETY: `vpninfo_ptr` and `fingerprint_in_cstr` are valid for this call.
                        let err = unsafe {
                            openconnect_check_peer_cert_hash(
                                vpninfo_ptr,
                                fingerprint_in_cstr.as_ptr(),
                            )
                        };
                        if err == 0 {
                            return 0;
                        }
                        if err < 0 {
                            tracing::warn!("Could not check peer cert hash: {}", cert.fingerprint);
                        }
                    }
                }
            }

            // SAFETY: `vpninfo_ptr` is valid for the duration of this synchronous callback.
            let raw_hash = unsafe { openconnect_get_peer_cert_hash(vpninfo_ptr) };
            // SAFETY: `openconnect_get_peer_cert_hash` returns NULL or a valid C string.
            let Some(fingerprint) = (unsafe { c_str_to_option_string(raw_hash) }) else {
                tracing::error!("Peer certificate hash unavailable; rejecting certificate");
                return 1;
            };

            if ctx.state.handle_accept_insecure_cert(&fingerprint) {
                let newcert = AcceptedCert {
                    fingerprint,
                    host,
                    port,
                };
                if let Ok(mut accepted_certs) = ctx.state.peer_certs.accepted_certs.lock() {
                    accepted_certs.push(newcert);
                }
                tracing::debug!("User accepted insecure certificate");
                0
            } else {
                tracing::debug!("User rejected insecure certificate");
                1
            }
        };

        // Fail-closed: return `1` (reject certificate) if privdata is null or a panic occurs.
        // SAFETY: Invoked synchronously by `libopenconnect` while `VpnInfoHandle` is alive.
        unsafe { with_callback_context(privdata, 1, body) }
    }
}
