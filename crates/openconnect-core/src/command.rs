use crate::VpnClient;
use openconnect_sys::{OC_CMD_CANCEL, OC_CMD_DETACH, OC_CMD_PAUSE, OC_CMD_STATS};
use std::sync::{LazyLock, Mutex, Weak};

#[cfg(not(target_os = "windows"))]
pub(crate) type CmdPipeFd = std::os::fd::RawFd;

#[cfg(target_os = "windows")]
pub(crate) type CmdPipeFd = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Cancel,
    Detach,
    Pause,
    Stats,
}

impl From<Command> for u8 {
    fn from(cmd: Command) -> u8 {
        match cmd {
            Command::Cancel => OC_CMD_CANCEL,
            Command::Detach => OC_CMD_DETACH,
            Command::Pause => OC_CMD_PAUSE,
            Command::Stats => OC_CMD_STATS,
        }
    }
}

pub trait CmdPipe {
    fn set_sock_block(&self, cmd_fd: CmdPipeFd);
    fn send_command(&self, cmd: Command);
}

impl CmdPipe for VpnClient {
    fn set_sock_block(&self, cmd_fd: CmdPipeFd) {
        #[cfg(not(target_os = "windows"))]
        {
            if cmd_fd < 0 {
                tracing::warn!("Ignoring invalid cmd_fd: {}", cmd_fd);
                return;
            }
            // SAFETY: `cmd_fd` is a valid non-negative descriptor owned by `libopenconnect`.
            let flags = unsafe { libc::fcntl(cmd_fd, libc::F_GETFL) };
            if flags < 0 {
                tracing::warn!(
                    "fcntl(F_GETFL) failed for cmd_fd {}: {}",
                    cmd_fd,
                    std::io::Error::last_os_error()
                );
                return;
            }
            // SAFETY: `cmd_fd` is valid and `flags` was returned by `F_GETFL`.
            let ret = unsafe { libc::fcntl(cmd_fd, libc::F_SETFL, flags & !libc::O_NONBLOCK) };
            if ret < 0 {
                tracing::warn!(
                    "fcntl(F_SETFL) failed for cmd_fd {}: {}",
                    cmd_fd,
                    std::io::Error::last_os_error()
                );
            }
        }

        #[cfg(target_os = "windows")]
        {
            let mut mode: u32 = 0;
            // SAFETY: `cmd_fd` is a valid WinSock SOCKET returned by `openconnect_setup_cmd_pipe`.
            let ret = unsafe {
                windows_sys::Win32::Networking::WinSock::ioctlsocket(
                    cmd_fd,
                    windows_sys::Win32::Networking::WinSock::FIONBIO,
                    &mut mode,
                )
            };
            if ret != 0 {
                tracing::warn!("ioctlsocket(FIONBIO) failed with code {}", ret);
            }
        }
    }

    fn send_command(&self, cmd: Command) {
        let cmd_byte: u8 = cmd.into();
        if cmd_byte == 0 {
            return;
        }

        // Hold `cmd_fd` mutex across the write syscall so `VpnInfoHandle::drop` cannot
        // close and reuse the FD concurrently while we write to it.
        let Ok(fd_guard) = self.state.cmd_fd.lock() else {
            tracing::error!("Failed to lock cmd_fd mutex while sending command");
            return;
        };
        let Some(cmd_fd) = *fd_guard else {
            return;
        };

        #[cfg(not(target_os = "windows"))]
        {
            if cmd_fd < 0 {
                return;
            }
            loop {
                // SAFETY: `cmd_fd` is kept live by `fd_guard` (invalidated before `vpninfo_free`)
                // and `&cmd_byte` points to 1 readable byte.
                let ret = unsafe {
                    libc::write(
                        cmd_fd,
                        (&cmd_byte as *const u8).cast::<std::ffi::c_void>(),
                        1,
                    )
                };
                if ret >= 0 {
                    break;
                }
                let err = std::io::Error::last_os_error();
                if err.kind() == std::io::ErrorKind::Interrupted {
                    continue;
                }
                tracing::error!(
                    "Failed to send command {} to VPN pipe (fd={}): {}",
                    cmd_byte,
                    cmd_fd,
                    err
                );
                break;
            }
        }

        #[cfg(target_os = "windows")]
        {
            // SAFETY: `cmd_fd` is kept live by `fd_guard` and `&cmd_byte` points to 1 readable byte.
            let ret = unsafe {
                windows_sys::Win32::Networking::WinSock::send(
                    cmd_fd,
                    std::ptr::from_ref(&cmd_byte),
                    1,
                    0,
                )
            };
            if ret < 0 {
                tracing::error!(
                    "Failed to send command {} to VPN socket: ret={}",
                    cmd_byte,
                    ret
                );
            }
        }
    }
}

pub struct SignalHandle {
    client: Mutex<Weak<VpnClient>>,
}

pub static SIGNAL_HANDLE: LazyLock<SignalHandle> = LazyLock::new(|| {
    let sig_handle = SignalHandle {
        client: Mutex::new(Weak::new()),
    };
    sig_handle.set_sig_handler();
    sig_handle
});

impl SignalHandle {
    /// Set the current client singleton to the given client.
    /// This is used when signal handler is called to send command to the client.
    pub fn update_client_singleton(&self, client: Weak<VpnClient>) {
        if let Ok(mut saved_client) = self.client.lock() {
            *saved_client = client;
        }
    }

    fn set_sig_handler(&self) {
        #[cfg(not(target_os = "windows"))]
        {
            use signal_hook::{
                consts::{SIGHUP, SIGINT, SIGTERM, SIGUSR1, SIGUSR2},
                iterator::Signals,
            };

            let Ok(mut signals) = Signals::new([SIGINT, SIGTERM, SIGHUP, SIGUSR1, SIGUSR2]) else {
                tracing::error!("Failed to register OS signal handler");
                return;
            };

            std::thread::spawn(move || {
                for sig in signals.forever() {
                    let cmd = match sig {
                        SIGINT | SIGTERM => Command::Cancel,
                        SIGHUP => Command::Detach,
                        SIGUSR2 => Command::Pause,
                        SIGUSR1 => Command::Stats,
                        _ => continue,
                    };

                    let client = SIGNAL_HANDLE
                        .client
                        .lock()
                        .ok()
                        .and_then(|weak| weak.upgrade());

                    if let Some(client) = client {
                        client.send_command(cmd);
                    }

                    if sig == SIGINT || sig == SIGTERM {
                        break;
                    }
                }
            });
        }

        #[cfg(target_os = "windows")]
        {
            use windows_sys::Win32::System::Console::{
                CTRL_BREAK_EVENT, CTRL_C_EVENT, CTRL_CLOSE_EVENT, CTRL_LOGOFF_EVENT,
                CTRL_SHUTDOWN_EVENT, SetConsoleCtrlHandler,
            };

            unsafe extern "system" fn console_control_handle(dw_ctrl_type: u32) -> i32 {
                crate::ffi::ffi_catch_unwind(0, || {
                    let cmd = match dw_ctrl_type {
                        CTRL_C_EVENT | CTRL_CLOSE_EVENT | CTRL_LOGOFF_EVENT
                        | CTRL_SHUTDOWN_EVENT => Command::Cancel,
                        CTRL_BREAK_EVENT => Command::Detach,
                        _ => return 0,
                    };

                    let client = SIGNAL_HANDLE
                        .client
                        .try_lock()
                        .ok()
                        .and_then(|weak| weak.upgrade());

                    if let Some(client) = client {
                        client.send_command(cmd);
                    }

                    1
                })
            }

            // SAFETY: `console_control_handle` has the required `PHANDLER_ROUTINE` ABI
            // and never unwinds across FFI.
            let ok = unsafe { SetConsoleCtrlHandler(Some(console_control_handle), 1) };
            if ok == 0 {
                tracing::warn!(
                    "SetConsoleCtrlHandler failed: {}",
                    std::io::Error::last_os_error()
                );
            }
        }
    }
}
