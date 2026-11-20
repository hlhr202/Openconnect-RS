#[cfg(target_os = "windows")]
pub mod windows {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::process::ExitStatusExt;
    use std::process::{Command, ExitStatus};
    use windows_sys::Win32::{
        Foundation::{CloseHandle, HANDLE},
        Security::{GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation},
        System::Threading::{GetCurrentProcess, OpenProcessToken},
        UI::{
            Shell::ShellExecuteW,
            WindowsAndMessaging::{SW_HIDE, SW_NORMAL},
        },
    };

    /// RAII wrapper ensuring Windows `HANDLE` is always closed via `CloseHandle`.
    struct OwnedTokenHandle(HANDLE);

    impl Drop for OwnedTokenHandle {
        fn drop(&mut self) {
            if !self.0.is_null() {
                // SAFETY: `self.0` is a valid token handle opened by `OpenProcessToken`.
                unsafe {
                    CloseHandle(self.0);
                }
            }
        }
    }

    pub fn is_elevated() -> bool {
        let mut raw_token: HANDLE = std::ptr::null_mut();
        let mut token_elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut return_len: u32 = 0;

        // SAFETY: `GetCurrentProcess()` returns a pseudo-handle and `&mut raw_token` is valid.
        let opened = unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut raw_token) };
        if opened == 0 {
            return false;
        }
        let token = OwnedTokenHandle(raw_token);

        // SAFETY: `token.0` is valid and `token_elevation` matches `size_of::<TOKEN_ELEVATION>()`.
        let ok = unsafe {
            GetTokenInformation(
                token.0,
                TokenElevation,
                (&mut token_elevation as *mut TOKEN_ELEVATION).cast::<std::ffi::c_void>(),
                std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut return_len,
            )
        };

        ok != 0 && token_elevation.TokenIsElevated != 0
    }

    pub struct Output {
        pub status: ExitStatus,
        pub stdout: Vec<u8>,
        pub stderr: Vec<u8>,
    }

    fn to_wide_null(s: &OsStr) -> Vec<u16> {
        s.encode_wide().chain(std::iter::once(0)).collect()
    }

    /// Quote a single Windows command-line argument according to `CommandLineToArgvW` rules.
    fn quote_windows_arg(arg: &str) -> String {
        if !arg.is_empty() && !arg.chars().any(|c| c == ' ' || c == '\t' || c == '"') {
            return arg.to_string();
        }
        let mut quoted = String::with_capacity(arg.len() + 2);
        quoted.push('"');
        let mut backslashes = 0usize;
        for c in arg.chars() {
            if c == '\\' {
                backslashes += 1;
            } else if c == '"' {
                for _ in 0..(backslashes * 2 + 1) {
                    quoted.push('\\');
                }
                quoted.push('"');
                backslashes = 0;
            } else {
                for _ in 0..backslashes {
                    quoted.push('\\');
                }
                backslashes = 0;
                quoted.push(c);
            }
        }
        for _ in 0..(backslashes * 2) {
            quoted.push('\\');
        }
        quoted.push('"');
        quoted
    }

    pub fn elevate(cmd: &Command, with_cmd: bool) -> std::io::Result<Output> {
        let mut args = Vec::new();
        for arg in cmd.get_args() {
            let arg_str = arg.to_str().ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Command argument contains non-UTF-8 characters",
                )
            })?;
            args.push(quote_windows_arg(arg_str));
        }

        let verb_w = to_wide_null(OsStr::new("runas"));
        let program_w = to_wide_null(cmd.get_program());
        let params_str = args.join(" ");
        let params_w = to_wide_null(OsStr::new(&params_str));
        let nshowcmd = if with_cmd { SW_NORMAL } else { SW_HIDE };

        // SAFETY: All wide-string pointers are NUL-terminated and live for the duration of the call.
        let r = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                verb_w.as_ptr(),
                program_w.as_ptr(),
                if args.is_empty() {
                    std::ptr::null()
                } else {
                    params_w.as_ptr()
                },
                std::ptr::null(),
                nshowcmd,
            )
        };

        // Per Win32 docs, ShellExecuteW returns a value > 32 on success, <= 32 on error.
        if (r as isize) <= 32 {
            return Err(std::io::Error::last_os_error());
        }

        Ok(Output {
            status: ExitStatus::from_raw(0),
            stdout: Vec::new(),
            stderr: Vec::new(),
        })
    }
}

#[cfg(unix)]
pub mod unix {
    pub fn is_elevated() -> bool {
        // SAFETY: `libc::geteuid()` is always safe and cannot fail per POSIX.
        unsafe { libc::geteuid() == 0 }
    }
}

#[cfg(target_os = "macos")]
pub mod macos {
    use security_framework::{
        authorization::{Authorization, Flags},
        base::Result,
    };
    use std::process::Command;

    pub fn is_elevated() -> bool {
        crate::elevator::unix::is_elevated()
    }

    pub fn elevate(cmd: &Command) -> Result<()> {
        let args = cmd
            .get_args()
            .filter_map(|c| c.to_str().map(str::to_string))
            .collect::<Vec<_>>();

        let flags = Flags::default();
        let auth = Authorization::default()?;
        auth.execute_with_privileges(cmd.get_program(), args, flags)
    }
}
