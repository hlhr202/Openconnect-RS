//! Fixture 05: Proves `get_cookie`, `get_hostname`, `get_peer_cert_hash`, and
//! `raw_handle_process_log` dereference NULL pointers via `CStr::from_ptr` -> immediate SIGSEGV.

use std::ffi::{c_char, c_void, CStr};
use std::os::unix::process::ExitStatusExt;
use std::process::Command;

extern "C" {
    fn mock_vpninfo_new() -> *mut c_void;
    fn mock_vpninfo_free(v: *mut c_void);
    fn mock_openconnect_get_cookie(vpninfo: *mut c_void) -> *const c_char;
    fn mock_openconnect_get_hostname(vpninfo: *mut c_void) -> *const c_char;
    fn mock_openconnect_get_peer_cert_hash(vpninfo: *mut c_void) -> *const c_char;
}

fn run_child(case: &str) {
    unsafe {
        let vpninfo = mock_vpninfo_new();
        let ptr = match case {
            "get_cookie" => mock_openconnect_get_cookie(vpninfo),
            "get_hostname" => mock_openconnect_get_hostname(vpninfo),
            "get_peer_cert_hash" => mock_openconnect_get_peer_cert_hash(vpninfo),
            _ => std::ptr::null(),
        };
        // Exact buggy pattern in lib.rs:264, 317, 369 and log.rs:50:
        let s = CStr::from_ptr(ptr).to_string_lossy().into_owned();
        println!("unexpectedly survived: {}", s);
        mock_vpninfo_free(vpninfo);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 3 && args[1] == "--child" {
        run_child(&args[2]);
        return;
    }

    let exe = std::env::current_exe().unwrap();
    for case in ["get_cookie", "get_hostname", "get_peer_cert_hash", "raw_handle_process_log"] {
        let status = Command::new(&exe)
            .args(["--child", case])
            .status()
            .expect("spawn child");
        let sig = status.signal();
        assert_eq!(
            sig,
            Some(11), // SIGSEGV
            "Expected SIGSEGV (11) for {}, got {:?}",
            case,
            status
        );
        println!("[05] PROVEN (NULL CStr::from_ptr): `{}` crashed with SIGSEGV (signal 11)!", case);
    }
}
