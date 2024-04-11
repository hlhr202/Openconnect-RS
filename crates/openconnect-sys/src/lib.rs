#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
extern crate openssl_sys;

#[cfg(target_os = "macos")]
#[cfg(target_arch = "aarch64")]
include!("bindings_aarch64_macos.rs");

#[cfg(target_os = "macos")]
#[cfg(target_arch = "x86_64")]
include!("bindings_x86_64_macos.rs");

#[cfg(target_os = "linux")]
#[cfg(target_arch = "x86_64")]
#[cfg(target_env = "gnu")]
include!("bindings_x86_64_linux_gnu.rs");

#[cfg(target_os = "linux")]
#[cfg(target_arch = "x86_64")]
#[cfg(target_env = "musl")]
include!("bindings_x86_64_linux_musl.rs");

#[cfg(target_os = "windows")]
#[cfg(target_arch = "x86_64")]
#[cfg(target_env = "gnu")]
include!("bindings_x86_64_windows_gnu.rs");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openconnect_version() {
        let version = unsafe {
            let raw_version = openconnect_get_version();
            std::ffi::CStr::from_ptr(raw_version).to_str().ok()
        };
        println!("OpenConnect version: {:?}", version.unwrap());
    }
}
