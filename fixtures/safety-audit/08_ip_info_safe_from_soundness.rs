//! Fixture 08: Proves `impl From<&oc_ip_info> for IpInfo` (`crates/openconnect-core/src/ip_info.rs:35-61`)
//! is a public API soundness hole: 100% SAFE caller code can construct an `oc_ip_info` with an
//! arbitrary invalid pointer and cause SIGSEGV via the safe `From` trait.

use std::ffi::{c_char, c_int, c_void, CStr};
use std::os::unix::process::ExitStatusExt;
use std::process::Command;

#[repr(C)]
pub struct OcIpInfo {
    pub addr: *const c_char,
    pub netmask: *const c_char,
    pub addr6: *const c_char,
    pub netmask6: *const c_char,
    pub dns: [*const c_char; 3],
    pub nbns: [*const c_char; 3],
    pub domain: *const c_char,
    pub proxy_pac: *const c_char,
    pub mtu: c_int,
    pub split_dns: *mut c_void,
    pub split_includes: *mut c_void,
    pub split_excludes: *mut c_void,
    pub gateway_addr: *const c_char,
}

pub struct IpInfo {
    pub addr: Option<String>,
}

unsafe fn raw_to_string(raw: *const c_char) -> Option<String> {
    if raw.is_null() {
        None
    } else {
        Some(CStr::from_ptr(raw).to_string_lossy().into_owned())
    }
}

/// Exact reproduction of `crates/openconnect-core/src/ip_info.rs:35-61`
impl From<&OcIpInfo> for IpInfo {
    fn from(value: &OcIpInfo) -> Self {
        unsafe {
            Self {
                addr: raw_to_string(value.addr),
            }
        }
    }
}

/// Note: NO `unsafe` block anywhere in this function!
fn safe_caller_triggers_segfault() {
    let bogus = OcIpInfo {
        addr: 0xDEAD_BEEFusize as *const c_char, // Creating a raw pointer is 100% safe Rust!
        netmask: std::ptr::null(),
        addr6: std::ptr::null(),
        netmask6: std::ptr::null(),
        dns: [std::ptr::null(); 3],
        nbns: [std::ptr::null(); 3],
        domain: std::ptr::null(),
        proxy_pac: std::ptr::null(),
        mtu: 0,
        split_dns: std::ptr::null_mut(),
        split_includes: std::ptr::null_mut(),
        split_excludes: std::ptr::null_mut(),
        gateway_addr: std::ptr::null(),
    };
    let info = IpInfo::from(&bogus); // <-- Safe trait call triggers UB / SIGSEGV!
    println!("unexpected: {:?}", info.addr);
}

fn main() {
    if std::env::args().nth(1).as_deref() == Some("--child") {
        safe_caller_triggers_segfault();
        return;
    }

    let exe = std::env::current_exe().unwrap();
    let status = Command::new(exe).arg("--child").status().unwrap();
    assert_eq!(status.signal(), Some(11), "Expected SIGSEGV (11)");
    println!(
        "[08] PROVEN (Safe API soundness hole): 100% safe caller invoking `IpInfo::from(&oc_ip_info)` crashed with SIGSEGV (signal 11)!"
    );
}
