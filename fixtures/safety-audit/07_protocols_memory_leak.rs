//! Fixture 07: Proves `crates/openconnect-core/src/protocols.rs:11-40` leaks the C-allocated
//! `oc_vpn_proto` array on EVERY call because `openconnect_free_supported_protocols` is never called.

use std::ffi::{c_char, c_int, c_uint, CStr};

#[repr(C)]
struct OcVpnProto {
    name: *const c_char,
    pretty_name: *const c_char,
    description: *const c_char,
    flags: c_uint,
}

extern "C" {
    fn mock_openconnect_get_supported_protocols(protos: *mut *mut OcVpnProto) -> c_int;
    fn mock_openconnect_free_supported_protocols(protos: *mut OcVpnProto);
    fn mock_get_unfreed_proto_count() -> c_int;
}

/// Exact reproduction of `crates/openconnect-core/src/protocols.rs:11-40`
fn buggy_get_supported_protocols() -> Vec<String> {
    let mut raw_protocols = std::ptr::null_mut::<OcVpnProto>();
    let mut protocols = vec![];
    unsafe {
        let n = mock_openconnect_get_supported_protocols(&mut raw_protocols);
        assert!(n >= 0);
        while !raw_protocols.is_null() && !(*raw_protocols).name.is_null() {
            let name = CStr::from_ptr((*raw_protocols).name)
                .to_string_lossy()
                .to_string();
            protocols.push(name);
            raw_protocols = raw_protocols.offset(1); // <-- also overwrites base pointer!
        }
        // BUG: openconnect_free_supported_protocols is NEVER called!
    }
    protocols
}

fn fixed_get_supported_protocols() -> Vec<String> {
    let mut base_ptr = std::ptr::null_mut::<OcVpnProto>();
    let mut protocols = vec![];
    unsafe {
        let n = mock_openconnect_get_supported_protocols(&mut base_ptr);
        if n > 0 && !base_ptr.is_null() {
            let slice = std::slice::from_raw_parts(base_ptr, n as usize);
            for p in slice {
                if !p.name.is_null() {
                    protocols.push(CStr::from_ptr(p.name).to_string_lossy().into_owned());
                }
            }
        }
        if !base_ptr.is_null() {
            mock_openconnect_free_supported_protocols(base_ptr);
        }
    }
    protocols
}

fn main() {
    for _ in 0..5 {
        let _ = buggy_get_supported_protocols();
    }
    let leaked = unsafe { mock_get_unfreed_proto_count() };
    assert_eq!(leaked, 5);
    println!(
        "[07] PROVEN (C heap leak): 5 calls to buggy `get_supported_protocols()` leaked {} C allocations!",
        leaked
    );

    let before = leaked;
    for _ in 0..5 {
        let _ = fixed_get_supported_protocols();
    }
    let after = unsafe { mock_get_unfreed_proto_count() };
    assert_eq!(after, before);
    println!(
        "[07] VERIFIED (Fixed): 5 calls to fixed version leaked 0 additional C allocations (net delta = 0)."
    );
}
