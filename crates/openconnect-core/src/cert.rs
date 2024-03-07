use crate::VpnClient;
use openconnect_sys::*;

pub static mut ACCEPTED_CERTS: *mut AcceptedCert = std::ptr::null_mut();

pub struct AcceptedCert {
    next: *mut AcceptedCert,
    fingerprint: *mut ::std::os::raw::c_char,
    host: *const ::std::os::raw::c_char,
    port: i32,
}

pub extern "C" fn validate_peer_cert(
    _privdata: *mut ::std::os::raw::c_void,
    _reason: *const ::std::os::raw::c_char,
) -> ::std::os::raw::c_int {
    let ctx = VpnClient::from_c_void(_privdata);

    unsafe {
        let vpninfo = (*ctx).vpninfo;
        let _fingerprint = openconnect_get_peer_cert_hash(vpninfo);
        let mut this = ACCEPTED_CERTS;

        while !this.is_null() {
            if (!(*this).host.is_null() || (*this).host == openconnect_get_hostname(vpninfo))
                && ((*this).port == 0 || (*this).port == openconnect_get_port(vpninfo))
            {
                let err = openconnect_check_peer_cert_hash(vpninfo, (*this).fingerprint);
                if err == 0 {
                    return 0;
                }
                if err < 0 {
                    println!("Certificate hash check failed");
                }
            }
            this = (*this).next;
        }
    }

    0
}
