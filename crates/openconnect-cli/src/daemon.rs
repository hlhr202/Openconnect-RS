pub fn daemonize() {
    unsafe {
        let pid = libc::fork();

        if pid < 0 {
            eprintln!("Failed to fork");
            std::process::exit(1);
        } else if pid != 0 {
            // Parent process
            std::process::exit(0);
        }

        libc::setsid();

        let pid = libc::fork();

        if pid < 0 {
            eprintln!("Failed to fork");
            std::process::exit(1);
        } else if pid != 0 {
            // Child process
            std::process::exit(0);
        }

        libc::umask(0);
        libc::close(libc::STDIN_FILENO);
        libc::close(libc::STDOUT_FILENO);
        libc::close(libc::STDERR_FILENO);
    }

    // Daemon process
}
