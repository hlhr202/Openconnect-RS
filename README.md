# OpenConnect for Rust

<img alt="GitHub License" src="https://img.shields.io/github/license/hlhr202/Openconnect-RS"> <img alt="Crates.io Version" src="https://img.shields.io/crates/v/openconnect-core?label=crates.io%20openconnect-core"> <img alt="GitHub Release" src="https://img.shields.io/github/v/release/hlhr202/Openconnect-RS?include_prereleases">

<img alt="GitHub Actions Workflow Status" src="https://img.shields.io/github/actions/workflow/status/hlhr202/Openconnect-RS/mingw64.yml?label=win-x86_64%20build"> <img alt="GitHub Actions Workflow Status" src="https://img.shields.io/github/actions/workflow/status/hlhr202/Openconnect-RS/mac-aarch64.yml?label=mac-aarch64%20build" /> <img alt="GitHub Actions Workflow Status" src="https://img.shields.io/github/actions/workflow/status/hlhr202/Openconnect-RS/mac-x64.yml?label=mac-x86_64%20build">

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

## Installation

- GUI:

  - Supports Windows(x64), Linux(x64), and macOS(aarch64, x64)

  - Download can be found in [Releases](https://github.com/hlhr202/Openconnect-RS/releases)

- CLI install:

  - Only supports Linux(x64) and macOS(aarch64, x64)

  - Run the following command in your terminal:

    ```bash
    curl -sL https://raw.githubusercontent.com/hlhr202/Openconnect-RS/main/install-cli.sh | sh
    ```

## Build

- Read the [System Requirements](./crates/openconnect-sys/README.md) for environment setup
- Modify it to fit your environment (For automatic setup, its WIP)
- For windows, since openconnect provides GNU automake, we currently only support MSYS2-MINGW64 and `x86_64-pc-windows-gnu` toolchain
  - Install MSYS2
  - Install `x86_64-pc-windows-gnu` toolchain with command `rustup default stable-x86_64-pc-windows-gnu`
  - Run cargo under MINGW64 shell

## License

This project is licensed under the GNU Lesser General Public License (LGPL). See the [LICENSE](./LICENSE-LGPL) file for details.

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

### Openconnect core

- [x] implement safe ffi
- [x] implement password login
- [x] implement cookie login
- [x] implement ssl certificate validation
- [ ] implement public key login

### Client

- [x] implement password login
- [x] implement oidc login
- [x] implement logs
  - [x] tracing file rotation
- [x] implement CLI
  - [x] Add/Remove configurations
  - [x] Daemon mode
  - [x] Password login
  - [ ] OIDC login
