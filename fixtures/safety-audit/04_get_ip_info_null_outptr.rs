//! Fixture 04: Proves `VpnClient::get_info` (`crates/openconnect-core/src/lib.rs:399-417`)
//! passes a NULL `*mut *const oc_ip_info` instead of `&mut info_ptr`, causing `get_info()`
//! to ALWAYS return `Ok(None)` even when connected with valid IP info.

use std::ffi::{c_char, c_int, c_void, CStr};

#[repr(C)]
struct OcIpInfo {
    addr: *const c_char,
    netmask: *const c_char,
    addr6: *const c_char,
    netmask6: *const c_char,
    dns: [*const c_char; 3],
    nbns: [*const c_char; 3],
    domain: *const c_char,
    proxy_pac: *const c_char,
    mtu: c_int,
    split_dns: *mut c_void,
    split_includes: *mut c_void,
    split_excludes: *mut c_void,
    gateway_addr: *const c_char,
}

extern "C" {
    fn mock_vpninfo_new() -> *mut c_void;
    fn mock_vpninfo_free(v: *mut c_void);
    fn mock_openconnect_get_ip_info(
        vpninfo: *mut c_void,
        info: *mut *const OcIpInfo,
        cstp_options: *mut *const c_void,
        dtls_options: *mut *const c_void,
    ) -> c_int;
}

/// Exact reproduction of `crates/openconnect-core/src/lib.rs:399-417`
fn buggy_get_info(vpninfo: *mut c_void) -> Option<String> {
    unsafe {
        let info: *mut *const OcIpInfo = std::ptr::null_mut(); // <-- BUG: NULL out-pointer!
        let ret = mock_openconnect_get_ip_info(
            vpninfo,
            info,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        assert_eq!(ret, 0);
        info.as_ref()
            .and_then(|p| p.as_ref())
            .map(|ip| CStr::from_ptr(ip.addr).to_string_lossy().into_owned())
    }
}

/// Fixed version passing `&mut info_ptr`
fn fixed_get_info(vpninfo: *mut c_void) -> Option<String> {
    unsafe {
        let mut info: *const OcIpInfo = std::ptr::null();
        let ret = mock_openconnect_get_ip_info(
            vpninfo,
            &mut info,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        assert_eq!(ret, 0);
        info.as_ref()
            .filter(|ip| !ip.addr.is_null())
            .map(|ip| CStr::from_ptr(ip.addr).to_string_lossy().into_owned())
    }
}

fn main() {
    unsafe {
        let vpninfo = mock_vpninfo_new();
        let buggy = buggy_get_info(vpninfo);
        let fixed = fixed_get_info(vpninfo);
        mock_vpninfo_free(vpninfo);

        assert_eq!(buggy, None, "Buggy version must silently lose IP info");
        assert_eq!(fixed.as_deref(), Some("10.8.0.2"));
        println!(
            "[04] PROVEN (Null out-pointer): buggy_get_info() = {:?}, whereas fixed_get_info() = {:?}!",
            buggy, fixed
        );
    }
}
