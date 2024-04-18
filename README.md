# OpenConnect for Rust

<a target="_blank" href="https://github.com/hlhr202/Openconnect-RS/blob/main/LICENSE-LGPL"><img alt="GitHub License" src="https://img.shields.io/github/license/hlhr202/Openconnect-RS"></a> <a target="_blank" href="https://crates.io/crates/openconnect-core"><img alt="Crates.io Version" src="https://img.shields.io/crates/v/openconnect-core?label=crates.io%20openconnect-core"></a> <a target="_blank" href="https://github.com/hlhr202/Openconnect-RS/releases"><img alt="GitHub Release" src="https://img.shields.io/github/v/release/hlhr202/Openconnect-RS?include_prereleases"></a>

<img alt="GitHub Actions Workflow Status" src="https://img.shields.io/github/actions/workflow/status/hlhr202/Openconnect-RS/mingw64.yml?label=win-x86_64%20build"> <img alt="GitHub Actions Workflow Status" src="https://img.shields.io/github/actions/workflow/status/hlhr202/Openconnect-RS/mac-aarch64.yml?label=mac-aarch64%20build" /> <img alt="GitHub Actions Workflow Status" src="https://img.shields.io/github/actions/workflow/status/hlhr202/Openconnect-RS/mac-x64.yml?label=mac-x86_64%20build"> <img alt="GitHub Actions Workflow Status" src="https://img.shields.io/github/actions/workflow/status/hlhr202/Openconnect-RS/linux-x64.yml?label=linux-x86_64%20build">

This is a cross-platform GUI client for OpenConnect, written in Rust and designed to work seamlessly on Windows, Linux, and macOS desktop systems. The program utilizes various technologies including MSYS2, Tauri, React, and NextUI. It provides a user-friendly interface for connecting to VPN servers using both password and OIDC authentication methods.

## Features

- Cross-platform compatibility (Windows, Linux, macOS)
- Easy-to-use GUI interface
- Support for both password and OIDC authentication
- Built with MSYS2, Tauri, React, and NextUI

## Screenshots

<img src="./doc/screenshot/openconnect-1.png" width="360px" height="320px" alt="Main">
<img src="./doc/screenshot/openconnect-2.png" width="360px" height="320px" alt="Edit">
<img src="./doc/screenshot/openconnect-3.png" width="360px" height="320px" alt="Connect">

## Installation of Client

- GUI:

  - Supports Windows(x64), Linux(x64), and macOS(aarch64, x64)

  - Download can be found in [Releases](https://github.com/hlhr202/Openconnect-RS/releases)

- CLI install:

  - Only supports Linux(x64) and macOS(aarch64, x64)

  - Run the following command in your terminal:

    ```bash
    curl -sL https://raw.githubusercontent.com/hlhr202/Openconnect-RS/main/install-cli.sh | bash
    ```

## Usage of CLI client

- Run the following command in your terminal:

  ```bash
  openconnect --help
  ```

  This will print the following help message:

  ```plaintext
  A CLI client to connect to VPN using OpenConnect

  Usage: openconnect <COMMAND>

  Commands:
    start         Connect to a VPN server and run in daemon mode [aliases: connect, run]
    status        Get the current VPN connection status [aliases: info, stat]
    stop          Close the current connection and exit the daemon process [aliases: kill, disconnect]
    add           Add new VPN server configuration to local config file [aliases: new, create, insert]
    import        Import VPN server configurations from a base64 encoded string
    export        Export VPN server configurations to a base64 encoded string
    delete        Delete a VPN server configuration from local config file [aliases: rm, remove, del]
    list          List all VPN server configurations in local config file [aliases: ls, l]
    logs          Show logs of the daemon process [aliases: log]
    gen-complete  Generate shell completion script
    help          Print this message or the help of the given subcommand(s)

  Options:
    -h, --help     Print help
    -V, --version  Print version
  ```

- For each subcommand, you can run `openconnect <COMMAND> --help` to get more information

  For example:

  ```bash
  openconnect start --help
  ```

  This will print the following help message:

  ```plaintext
  Connect to a VPN server and run in daemon mode

  Usage: openconnect start [OPTIONS] <NAME>

  Arguments:
    <NAME>  The server name saved in local config file to connect to

  Options:
    -c, --config-file <CONFIG_FILE>  The path to the local config file
    -h, --help                       Print help
  ```

### Generate shell completion script

- ZSH (Oh My Zsh!)

  ```bash
  mkdir -p ~/.oh-my-zsh/custom/plugins/openconnect
  openconnect gen-complete zsh > ~/.oh-my-zsh/custom/plugins/openconnect/openconnect.plugin.zsh
  ```

  Then add `openconnect` to the `plugins` array in your `~/.zshrc` file:

- Bash

  ```bash
  mkdir -p ~/.bash_completion
  openconnect gen-complete bash > ~/.bash_completion/openconnect
  echo "source ~/.bash_completion/openconnect" >> ~/.bashrc
  ```

## Build

- Read the [System Requirements](./crates/openconnect-sys/README.md) for environment setup
- Modify it to fit your environment (For automatic setup, its WIP)
- For windows, since openconnect provides GNU automake, we currently only support MSYS2-MINGW64 and `x86_64-pc-windows-gnu` toolchain
  - Install MSYS2
  - Install `x86_64-pc-windows-gnu` toolchain with command `rustup default stable-x86_64-pc-windows-gnu`
  - Run cargo under MINGW64 shell

## License

Since Openconnect is released under LGPL license, the core libraries (openconnect-core and openconnect-sys) of this project is licensed under the GNU Lesser General Public License (LGPL). See the [LICENSE](./LICENSE-LGPL) file for details.

For some part of this library (openconnect-oidc), it is licensed under the MIT license.

## Acknowledgements

Special thanks to (MORE THAN) the following projects and technologies for making this project possible:

- [OpenConnect](https://www.infradead.org/openconnect/)
- [MSYS2](https://www.msys2.org/)
- [Tauri](https://tauri.app/)
- [Tokio](https://tokio.rs/)
- [Windows-rs](https://github.com/microsoft/windows-rs)
- [OpenIDConnect](https://github.com/ramosbugs/openidconnect-rs)
- [React](https://reactjs.org/)
- [NextUI](https://nextui.org/)
- [Vite](https://vitejs.dev/)

## Roadmap

### Openconnect sys

- [x] Automatically build openconnect
- [x] Automatically search library path
  - [ ] Optimize search path for more cases
- [ ] better docs

### Openconnect core

- [x] implement safe ffi
- [x] implement username + password login
- [x] implement cookie login
- [x] implement ssl certificate validation
- [ ] implement public key login
- [ ] implement various auth methods
- [ ] better docs

### Client

- [x] implement username + password login
- [x] implement oidc login
- [x] implement logs
  - [x] tracing file rotation
  - [ ] optimize log search
- [x] implement CLI
  - [x] Add/Remove configurations
  - [x] Daemon mode
  - [x] Password login
  - [x] OIDC login
