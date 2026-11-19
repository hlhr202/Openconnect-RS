#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::undocumented_unsafe_blocks)]
#![allow(clippy::multiple_unsafe_ops_per_block)]

#[cfg(all(feature = "native", not(feature = "mock-ffi")))]
extern crate openssl_sys;

#[cfg(openconnect_generated_bindings)]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(all(
    not(openconnect_generated_bindings),
    target_os = "macos",
    target_arch = "aarch64"
))]
include!("bindings_aarch64_macos.rs");

#[cfg(all(
    not(openconnect_generated_bindings),
    target_os = "macos",
    target_arch = "x86_64"
))]
include!("bindings_x86_64_macos.rs");

#[cfg(all(
    not(openconnect_generated_bindings),
    target_os = "linux",
    target_arch = "x86_64",
    target_env = "gnu"
))]
include!("bindings_x86_64_linux_gnu.rs");

#[cfg(all(
    not(openconnect_generated_bindings),
    target_os = "linux",
    target_arch = "x86_64",
    target_env = "musl"
))]
include!("bindings_x86_64_linux_musl.rs");

#[cfg(all(
    not(openconnect_generated_bindings),
    target_os = "windows",
    target_arch = "x86_64",
    target_env = "gnu"
))]
include!("bindings_x86_64_windows_gnu.rs");

#[cfg(feature = "mock-ffi")]
pub mod mock {
    use super::openconnect_info;
    use std::ffi::{c_char, c_int};

    unsafe extern "C" {
        pub fn mock_oc_reset_counters();
        pub fn mock_oc_live_vpninfo_count() -> c_int;
        pub fn mock_oc_live_proto_alloc_count() -> c_int;
        pub fn mock_oc_set_return_nulls(vpninfo: *mut openconnect_info, enabled: c_int) -> c_int;
        pub fn mock_oc_trigger_progress_empty(vpninfo: *mut openconnect_info);
        pub fn mock_oc_trigger_progress_unknown_level(vpninfo: *mut openconnect_info);
        pub fn mock_oc_trigger_validate_peer_cert(
            vpninfo: *mut openconnect_info,
            reason: *const c_char,
        ) -> c_int;
        pub fn mock_oc_trigger_stats(vpninfo: *mut openconnect_info, include_stats: c_int);
        pub fn mock_oc_trigger_setup_tun(vpninfo: *mut openconnect_info);
        pub fn mock_oc_trigger_auth_form_ignored_opt(vpninfo: *mut openconnect_info) -> c_int;
        pub fn mock_oc_trigger_auth_form_select(
            vpninfo: *mut openconnect_info,
            choice_name: *const c_char,
            choice_label: *const c_char,
            opt_name: *const c_char,
        ) -> c_int;
        pub fn mock_oc_trigger_auth_form_text_and_password(
            vpninfo: *mut openconnect_info,
            text_field_name: *const c_char,
        ) -> c_int;
        pub fn mock_oc_trigger_auth_form_null_cases(vpninfo: *mut openconnect_info) -> c_int;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openconnect_version() {
        // SAFETY: `openconnect_get_version` returns a static NUL-terminated C string (or NULL).
        let version = unsafe {
            let raw_version = openconnect_get_version();
            assert!(
                !raw_version.is_null(),
                "openconnect_get_version returned NULL"
            );
            std::ffi::CStr::from_ptr(raw_version).to_str().ok()
        };
        assert!(version.is_some());
    }
}
