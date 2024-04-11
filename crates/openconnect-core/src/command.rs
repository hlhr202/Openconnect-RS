use crate::VpnClient;
use lazy_static::lazy_static;
use openconnect_sys::{OC_CMD_CANCEL, OC_CMD_DETACH, OC_CMD_PAUSE, OC_CMD_STATS};
use std::sync::{atomic::Ordering, Mutex, Weak};

#[derive(Debug, Clone, Copy)]
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
    fn set_sock_block(&self, cmd_fd: i32);
    fn send_command(&self, cmd: Command);
}

impl CmdPipe for VpnClient {
    fn set_sock_block(&self, cmd_fd: i32) {
        #[cfg(not(target_os = "windows"))]
        {
            unsafe {
                libc::fcntl(
                    cmd_fd,
                    libc::F_SETFL,
                    libc::fcntl(cmd_fd, libc::F_GETFL) & !libc::O_NONBLOCK,
                );
            }
        }

        #[cfg(target_os = "windows")]
        {
            let mut mode: u32 = 0;
            unsafe {
                windows_sys::Win32::Networking::WinSock::ioctlsocket(
                    cmd_fd as usize,
                    windows_sys::Win32::Networking::WinSock::FIONBIO,
                    &mut mode,
                );
            }
        }
    }
    fn send_command(&self, cmd: Command) {
        let cmd: u8 = cmd.into();

        #[cfg(not(target_os = "windows"))]
        {
            let cmd_fd = self.cmd_fd.load(Ordering::SeqCst);
            if cmd != 0 && cmd_fd >= 0 {
                let ret = unsafe { libc::write(cmd_fd, std::ptr::from_ref(&cmd) as *const _, 1) };

                if ret < 0 {
                    // TODO: log error
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            let cmd_fd = self.cmd_fd.load(Ordering::SeqCst);
            if cmd_fd >= 0 {
                let ret = {
                    unsafe {
                        windows_sys::Win32::Networking::WinSock::send(
                            cmd_fd as usize,
                            std::ptr::from_ref(&cmd) as *const _,
                            1,
                            0,
                        )
                    }
                };

                if ret < 0 {
                    // TODO: log error
                }
            }
        }
    }
}

pub struct SignalHandle {
    client: Mutex<Weak<VpnClient>>,
}

lazy_static! {
    pub static ref SIGNAL_HANDLE: SignalHandle = {
        let sig_handle = SignalHandle {
            client: Mutex::new(Weak::new()),
        };
        sig_handle.set_sig_handler();
        sig_handle
    };
}

impl SignalHandle {
    /// Set the current client singleton to the given client.
    /// This is used when signal handler is called to send command to the client.
    pub fn update_client_singleton(&self, client: Weak<VpnClient>) {
        let saved_client = self.client.lock();
        if let Ok(mut saved_client) = saved_client {
            *saved_client = client;
        }
    }

    /// Set the signal handler for the current process.
    fn set_sig_handler(&self) {
        #[cfg(not(target_os = "windows"))]
        {
            use signal_hook::{
                consts::{SIGHUP, SIGINT, SIGTERM, SIGUSR1, SIGUSR2},
                iterator::Signals,
            };

            let mut signals = Signals::new([SIGINT, SIGTERM, SIGHUP, SIGUSR1, SIGUSR2])
                .expect("Failed to register signal handler");

            std::thread::spawn(move || {
                println!("Signal handler thread started");

                for sig in signals.forever() {
                    let cmd = match sig {
                        SIGINT | SIGTERM => {
                            println!("Received SIGINT or SIGTERM");
                            Command::Cancel
                        }
                        SIGHUP => {
                            println!("Received SIGHUP");
                            Command::Detach
                        }
                        SIGUSR2 => {
                            println!("Received SIGUSR2");
                            Command::Pause
                        }
                        SIGUSR1 => {
                            println!("Received SIGUSR1");
                            Command::Stats
                        }
                        _ => {
                            println!("Received unknown signal");
                            unreachable!()
                        }
                    };

                    {
                        let this = SIGNAL_HANDLE
                            .client
                            .lock()
                            .ok()
                            .and_then(|this| this.upgrade());

                        if let Some(this) = this {
                            this.send_command(cmd);
                        }
                    }

                    if sig == SIGINT || sig == SIGTERM {
                        // Exit the signal handler thread since the process is going to exit
                        break;
                    }
                }
            });
        }

        #[cfg(target_os = "windows")]
        {
            use windows_sys::Win32::System::Console::{
                SetConsoleCtrlHandler, CTRL_BREAK_EVENT, CTRL_CLOSE_EVENT, CTRL_C_EVENT,
                CTRL_LOGOFF_EVENT, CTRL_SHUTDOWN_EVENT,
            };

            unsafe extern "system" fn console_control_handle(dw_ctrl_type: u32) -> i32 {
                let cmd = match dw_ctrl_type {
                    CTRL_C_EVENT | CTRL_CLOSE_EVENT | CTRL_LOGOFF_EVENT | CTRL_SHUTDOWN_EVENT => {
                        Command::Cancel
                    }
                    CTRL_BREAK_EVENT => Command::Detach,
                    _ => unreachable!(),
                };

                {
                    let this = SIGNAL_HANDLE
                        .client
                        .lock()
                        .ok()
                        .and_then(|this| this.upgrade());

                    if let Some(this) = this {
                        this.send_command(cmd);
                    }
                }

                1
            }

            unsafe {
                SetConsoleCtrlHandler(Some(console_control_handle), 1);
            }
        }
    }
}
