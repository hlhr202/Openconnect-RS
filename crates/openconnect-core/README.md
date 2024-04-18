# Openconnect Core Library

This library provides a safe Rust API for interacting with underlying Openconnect C library. The unsafe bindings are provided by the [openconnect-sys](https://crates.io/crates/openconnect-sys) crate.

## Prerequisites

Read the [openconnect-sys](https://crates.io/crates/openconnect-sys) crate documentation for installing prerequisites including native system libraries and headers.

## Usage

- Add `openconnect-core` to your `Cargo.toml`:

  ```toml
  [dependencies]
  openconnect-core = "0.1"
  ```

- For simple use cases, please refer to [openconnect-core docs](https://docs.rs/openconnect-core/).

- For more use cases, you can checkout our CLI application [openconnect-cli](https://github.com/hlhr202/Openconnect-RS/tree/main/crates/openconnect-cli).

- For GUI/CLI applications, you can checkout our github repository [Openconnect-RS](https://github.com/hlhr202/Openconnect-RS/)
