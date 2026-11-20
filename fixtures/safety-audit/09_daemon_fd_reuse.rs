//! Fixture 09: Proves `crates/openconnect-cli/src/daemon.rs:36-38` closing fd 0/1/2 without
//! redirecting to `/dev/null` causes newly opened fds to receive fd 1 (stdout), so any
//! `println!` corrupts the newly opened file/socket!

use std::fs::File;
use std::io::Read;
use std::os::unix::io::AsRawFd;
use std::process::Command;

extern "C" {
    fn close(fd: i32) -> i32;
}

fn child_run(tmp_path: &str) {
    // Exact reproduction of `crates/openconnect-cli/src/daemon.rs:36-38`
    unsafe {
        close(0);
        close(1);
        close(2);
    }

    // Next opened file/socket gets fd 0, then fd 1 (STDOUT_FILENO)!
    let f0 = File::create(format!("{}.0", tmp_path)).unwrap();
    let f1 = File::create(format!("{}.1", tmp_path)).unwrap();
    assert_eq!(f0.as_raw_fd(), 0);
    assert_eq!(f1.as_raw_fd(), 1);

    // Now any Rust `println!` (e.g., from openconnect-core's `println!("stats_fn")`) writes into `f1`!
    println!("CORRUPTED_SOCKET_PAYLOAD");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 3 && args[1] == "--child" {
        child_run(&args[2]);
        return;
    }

    let tmp_base = "/tmp/openconnect_audit_fd_reuse";
    let exe = std::env::current_exe().unwrap();
    let status = Command::new(exe)
        .args(["--child", tmp_base])
        .status()
        .unwrap();
    assert!(status.success());

    let mut contents = String::new();
    File::open(format!("{}.1", tmp_base))
        .unwrap()
        .read_to_string(&mut contents)
        .unwrap();
    let _ = std::fs::remove_file(format!("{}.0", tmp_base));
    let _ = std::fs::remove_file(format!("{}.1", tmp_base));

    assert_eq!(contents.trim(), "CORRUPTED_SOCKET_PAYLOAD");
    println!(
        "[09] PROVEN (FD 0/1/2 reuse): After `close(0/1/2)` without `/dev/null` dup2, `println!` wrote `{}` directly into newly opened fd 1!",
        contents.trim()
    );
}
