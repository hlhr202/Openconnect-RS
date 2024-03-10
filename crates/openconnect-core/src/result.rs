use thiserror::Error;

use crate::{events::Events, VpnClient};

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum OpenconnectError {
    #[error("Failed to create new VPN config: {0}")]
    ConfigError(String),

    #[error("Failed to create new VPN entry point: {0}")]
    EntrypointConfigError(String),

    #[error("Failed to setup shutdown error: {0}")]
    SetupShutdownError(String),

    #[error("Failed to obtain cookie from server. Error code: {0}")]
    ObtainCookieError(i32),

    #[error("Failed to set protocol. Error code: {0}")]
    SetProtocolError(i32),

    #[error("Failed to set reported OS. Error code: {0}")]
    SetReportOSError(i32),

    #[error("Failed to setup command pipe. Error code: {0}")]
    CmdPipeError(i32),

    #[error("Failed to set HTTP proxy. Error code: {0}")]
    SetProxyError(i32),

    #[error("Failed to make CSTP connection. Error code: {0}")]
    MakeCstpError(i32),

    #[error("Failed to disable DTLS. Error code: {0}")]
    DisableDTLSError(i32),

    #[error("Failed to parse URL. Error code: {0}")]
    ParseUrlError(i32),

    #[error("Failed to set client certificate. Error code: {0}")]
    SetClientCertError(i32),

    #[error("Failed to set MCA certificate. Error code: {0}")]
    SetMCACertError(i32),

    #[error("Failed to set MCA private key. Error code: {0}")]
    MainLoopError(i32),
}

pub type OpenconnectResult<T> = std::result::Result<T, OpenconnectError>;

pub trait EmitError<T> {
    fn emit_error(self, client: &VpnClient) -> OpenconnectResult<T>;
}

impl<T> EmitError<T> for OpenconnectResult<T> {
    fn emit_error(self, client: &VpnClient) -> OpenconnectResult<T> {
        self.inspect_err(|e| client.emit_error(e))
    }
}
