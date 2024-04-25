use colored::Colorize;

#[derive(Debug)]
pub enum ForkResult {
    Parent,
    Child,
    Grandchild,
}

#[cfg(not(target_os = "windows"))]
pub fn daemonize() -> ForkResult {
    unsafe {
        let pid = libc::fork();

        if pid < 0 {
            eprintln!("{}", "\nFailed to fork child process".red());
            std::process::exit(1);
        } else if pid != 0 {
            // Parent process
            // std::process::exit(0);
            return ForkResult::Parent;
        }

        libc::setsid();

        let pid = libc::fork();

        if pid < 0 {
            eprintln!("{}", "\nFailed to fork grand-child process".red());
            std::process::exit(1);
        } else if pid != 0 {
            // Child process
            // std::process::exit(0);
            return ForkResult::Child;
        }

        libc::umask(0);
        libc::close(libc::STDIN_FILENO);
        libc::close(libc::STDOUT_FILENO);
        libc::close(libc::STDERR_FILENO);

        ForkResult::Grandchild
    }

    // Daemon process
}

#[cfg(target_os = "windows")]
pub fn daemonize() -> ForkResult {
    use ::windows::Win32::System::Threading::CreateProcessW;

    let mut startup_info = ::windows::Win32::System::Threading::STARTUPINFOW::default();
    let mut process_info = ::windows::Win32::System::Threading::PROCESS_INFORMATION::default();
    let lapplicationname = ::windows::core::HSTRING::from("openconnect-cli-child.exe");

    unsafe {
        let result = CreateProcessW(
            &lapplicationname,
            ::windows::core::PWSTR::null(),
            None,
            None,
            false,
            ::windows::Win32::System::Threading::PROCESS_CREATION_FLAGS(0),
            None,
            ::windows::core::PCWSTR::null(),
            &startup_info as *const _,
            &mut process_info,
        );

        if result.is_err() {
            eprintln!("{}", "\nFailed to create child process".red());
            std::process::exit(1);
        } else {
            println!("{}", "\nChild process created".green());
        }
    };

    ForkResult::Parent
}


#[test]
fn test_windows_daemonize() {
    let result = daemonize();
    println!("{:?}", result)
}