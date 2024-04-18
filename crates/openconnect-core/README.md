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

- Use the library in your code:

  ```rust
  use openconnect_core::{
      config::{ConfigBuilder, EntrypointBuilder, LogLevel},
      events::EventHandlers,
      protocols::get_anyconnect_protocol,
      Connectable, VpnClient,
  };
  use std::env;

  fn main() -> Result<(), Box<dyn std::error::Error>> {
      let protocol = get_anyconnect_protocol();
      let config = ConfigBuilder::default().loglevel(LogLevel::Info).build()?;
      let event_handlers = EventHandlers::default();
      let client = VpnClient::new(config, event_handlers)?;

      let entrypoint = EntrypointBuilder::new()
          .server("vpn.example.com")
          .username("your_username")
          .password("your_password")
          .protocol(protocol)
          .enable_udp(true)
          .accept_insecure_cert(true)
          .build()?;

      client.connect(entrypoint)?;

      Ok(())
  }
  ```

- For more use cases, you can checkout our CLI application [openconnect-cli](https://github.com/hlhr202/Openconnect-RS/tree/main/crates/openconnect-cli).

- For GUI/CLI applications, you can checkout our github repository [Openconnect-RS](https://github.com/hlhr202/Openconnect-RS/)
