use crate::sock;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Spawn the current CLI executable in detached `__daemon` mode with stdio redirected to `/dev/null`
/// (avoiding unsafe `fork()` and avoiding closing raw FDs 0/1/2 without `/dev/null` redirection).
pub fn spawn_detached_daemon() -> std::io::Result<()> {
    let exe = std::env::current_exe()?;
    let mut child = Command::new(exe)
        .arg("__daemon")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    let sock_path = sock::get_sock();
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if sock_path.exists() {
            return Ok(());
        }
        if let Some(status) = child.try_wait()? {
            return Err(std::io::Error::other(format!(
                "Daemon process exited prematurely with status: {}",
                status
            )));
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::TimedOut,
        format!(
            "Timed out waiting for daemon socket at {}",
            sock_path.display()
        ),
    ))
}

/// Detach from the controlling terminal and apply a restrictive umask (`0o077`)
/// inside the newly exec'd single-threaded `__daemon` process before starting Tokio.
pub fn prepare_daemon_process() {
    #[cfg(unix)]
    {
        // SAFETY: Called once in a fresh single-threaded daemon process to start a new session.
        let sid = unsafe { libc::setsid() };
        if sid < 0 {
            let err = std::io::Error::last_os_error();
            // Ignore EPERM if already a session leader.
            if err.raw_os_error() != Some(libc::EPERM) {
                eprintln!("setsid warning: {}", err);
            }
        }
        // SAFETY: `umask` is always safe per POSIX and restricts newly created files/sockets to owner-only.
        unsafe {
            libc::umask(0o077);
        }
    }
}
