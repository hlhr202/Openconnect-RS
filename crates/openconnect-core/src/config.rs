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

pub struct Config {
    pub server: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub protocol: Protocol,
    pub vpncscript: Option<String>,
    pub http_proxy: Option<String>,
    pub enable_udp: bool,
    pub loglevel: LogLevel,
}

pub struct ConfigBuilder {
    username: Option<String>,
    password: Option<String>,
    server: Option<String>,
    protocol: Option<Protocol>,
    vpncscript: Option<String>,
    http_proxy: Option<String>,
    enable_udp: bool,
    loglevel: Option<LogLevel>,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Self {
            username: None,
            password: None,
            server: None,
            protocol: None,
            vpncscript: None,
            http_proxy: None,
            enable_udp: true,
            loglevel: None,
        }
    }

    pub fn username(&mut self, username: &str) -> &mut Self {
        self.username = Some(username.to_string());
        self
    }

    pub fn password(&mut self, password: &str) -> &mut Self {
        self.password = Some(password.to_string());
        self
    }

    pub fn server(&mut self, server: &str) -> &mut Self {
        self.server = Some(server.to_string());
        self
    }

    pub fn vpncscript(&mut self, vpncscript: &str) -> &mut Self {
        self.vpncscript = Some(vpncscript.to_string());
        self
    }

    pub fn loglevel(&mut self, loglevel: LogLevel) -> &mut Self {
        self.loglevel = Some(loglevel);
        self
    }

    pub fn protocol(&mut self, protocol: Protocol) -> &mut Self {
        self.protocol = Some(protocol);
        self
    }

    pub fn http_proxy(&mut self, http_proxy: &str) -> &mut Self {
        self.http_proxy = Some(http_proxy.to_string());
        self
    }

    pub fn enable_udp(&mut self, enable_udp: bool) -> &mut Self {
        self.enable_udp = enable_udp;
        self
    }

    pub fn build(&self) -> OpenConnectResult<Config> {
        let server = self.server.clone().ok_or(OpenConnectError::ConfigError(
            "Server is required".to_string(),
        ))?;

        let protocol = self
            .protocol
            .clone()
            .unwrap_or_else(get_anyconnect_protocol);

        Ok(Config {
            username: self.username.clone(),
            password: self.password.clone(),
            server,
            protocol,
            http_proxy: self.http_proxy.clone(),
            vpncscript: self.vpncscript.clone(),
            enable_udp: self.enable_udp,
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
