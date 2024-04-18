use colored::Colorize;

pub enum ForkResult {
    Parent,
    Child,
    Grandchild,
}

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
