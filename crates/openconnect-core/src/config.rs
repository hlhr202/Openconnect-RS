use crate::{
    protocols::{get_anyconnect_protocol, Protocol},
    result::{OpenConnectError, OpenConnectResult},
};
use openconnect_sys::{PRG_DEBUG, PRG_ERR, PRG_INFO, PRG_TRACE};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Err = PRG_ERR as isize,
    Info = PRG_INFO as isize,
    Debug = PRG_DEBUG as isize,
    Trace = PRG_TRACE as isize,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub vpncscript: Option<String>,
    pub http_proxy: Option<String>,
    pub loglevel: LogLevel,
}

pub struct ConfigBuilder {
    vpncscript: Option<String>,
    http_proxy: Option<String>,
    loglevel: Option<LogLevel>,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Self {
            vpncscript: None,
            http_proxy: None,
            loglevel: None,
        }
    }

    pub fn vpncscript(&mut self, vpncscript: &str) -> &mut Self {
        self.vpncscript = Some(vpncscript.to_string());
        self
    }

    pub fn loglevel(&mut self, loglevel: LogLevel) -> &mut Self {
        self.loglevel = Some(loglevel);
        self
    }

    pub fn http_proxy(&mut self, http_proxy: &str) -> &mut Self {
        self.http_proxy = Some(http_proxy.to_string());
        self
    }

    pub fn build(&self) -> OpenConnectResult<Config> {
        Ok(Config {
            http_proxy: self.http_proxy.clone(),
            vpncscript: self.vpncscript.clone(),
            loglevel: self.loglevel.unwrap_or(LogLevel::Info),
        })
    }
}

impl Config {
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::new()
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct Entrypoint {
    pub server: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub protocol: Protocol,
    pub enable_udp: bool,
}

pub struct EntrypointBuilder {
    server: Option<String>,
    username: Option<String>,
    password: Option<String>,
    protocol: Option<Protocol>,
    enable_udp: bool,
}

impl EntrypointBuilder {
    pub fn new() -> Self {
        Self {
            server: None,
            username: None,
            password: None,
            protocol: None,
            enable_udp: true,
        }
    }

    pub fn server(&mut self, server: &str) -> &mut Self {
        self.server = Some(server.to_string());
        self
    }

    pub fn username(&mut self, username: &str) -> &mut Self {
        self.username = Some(username.to_string());
        self
    }

    pub fn password(&mut self, password: &str) -> &mut Self {
        self.password = Some(password.to_string());
        self
    }

    pub fn protocol(&mut self, protocol: Protocol) -> &mut Self {
        self.protocol = Some(protocol);
        self
    }

    pub fn enable_udp(&mut self, enable_udp: bool) -> &mut Self {
        self.enable_udp = enable_udp;
        self
    }

    pub fn build(&self) -> OpenConnectResult<Entrypoint> {
        let server = self
            .server
            .clone()
            .ok_or(OpenConnectError::EntrypointConfigError(
                "Server is required".to_string(),
            ))?;

        let protocol = self
            .protocol
            .clone()
            .unwrap_or_else(get_anyconnect_protocol);

        Ok(Entrypoint {
            server,
            username: self.username.clone(),
            password: self.password.clone(),
            protocol,
            enable_udp: self.enable_udp,
        })
    }
}

impl Default for EntrypointBuilder {
    fn default() -> Self {
        Self::new()
    }
}
