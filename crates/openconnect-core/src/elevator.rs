// TODO: add support for macOS/Linux GUI escalation
#[cfg(target_os = "windows")]
pub mod windows {
    use std::os::windows::process::ExitStatusExt;
    use std::process::{Command, ExitStatus};
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_NORMAL;
    use windows::{
        core::{w, HSTRING, PCWSTR},
        Win32::{Foundation::HWND, UI::WindowsAndMessaging::SW_HIDE},
    };
    use windows_sys::Win32::{
        Foundation::HANDLE,
        Security::{
            GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_ELEVATION_TYPE, TOKEN_QUERY,
        },
        System::Threading::{GetCurrentProcess, OpenProcessToken},
    };

    pub fn is_elevated() -> bool {
        let mut current_token_ptr: HANDLE = unsafe { std::mem::zeroed() };
        let mut token_evelation: TOKEN_ELEVATION = unsafe { std::mem::zeroed() };
        let token_evelation_type_ptr: *mut TOKEN_ELEVATION = &mut token_evelation;
        let mut size: u32 = 0;

        let result =
            unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut current_token_ptr) };
        if result != 0 {
            let result = unsafe {
                GetTokenInformation(
                    current_token_ptr,
                    TokenElevation,
                    token_evelation_type_ptr as *mut std::ffi::c_void,
                    std::mem::size_of::<TOKEN_ELEVATION_TYPE>() as u32,
                    &mut size,
                )
            };
            if result != 0 {
                return token_evelation.TokenIsElevated != 0;
            }
        }

        false
    }

    pub struct Output {
        pub status: ExitStatus,
        pub stdout: Vec<u8>,
        pub stderr: Vec<u8>,
    }

    pub fn elevate(cmd: &Command, with_cmd: bool) -> std::io::Result<Output> {
        let args = cmd
            .get_args()
            .map(|c| c.to_str().expect("Invalid args").to_string())
            .collect::<Vec<_>>();

        let parameters = if args.is_empty() {
            HSTRING::new()
        } else {
            let arg_str = args.join(" ");
            HSTRING::from(arg_str)
        };

        let nshowcmd = if with_cmd { SW_NORMAL } else { SW_HIDE };

        let r = unsafe {
            ShellExecuteW(
                HWND(0),
                w!("runas"),
                &HSTRING::from(cmd.get_program()),
                &parameters,
                PCWSTR::null(),
                nshowcmd,
            )
        };

        Ok(Output {
            status: ExitStatus::from_raw(r.0 as u32),
            stdout: Vec::new(),
            stderr: Vec::new(),
        })
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_is_elevated() {
            assert!(!is_elevated());
        }

        #[test]
        fn test_elevate() {
            let mut cmd = Command::new("cmd");
            cmd.arg("/c").arg("echo hello");
            elevate(&cmd, false).expect("Failed to elevate");
        }
    }
}
