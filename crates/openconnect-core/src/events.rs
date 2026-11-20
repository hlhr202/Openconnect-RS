use crate::{Status, cert::PeerCertPrompt, result::OpenconnectError};
use std::sync::Arc;

/// Peer certificate verification handler. Returns `true` to trust the certificate.
pub(crate) type PeerCertVerifyHandler =
    Arc<dyn Fn(&PeerCertPrompt) -> bool + Send + Sync + 'static>;

/// Thread-safe event handlers for [`crate::VpnClient`].
#[allow(clippy::type_complexity)]
#[derive(Clone, Default)]
pub struct EventHandlers {
    pub(crate) handle_connection_state_change: Option<Arc<dyn Fn(Status) + Send + Sync + 'static>>,
    pub(crate) handle_peer_cert_verify: Option<PeerCertVerifyHandler>,
}

impl EventHandlers {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_handle_connection_state_change<F>(mut self, handler: F) -> Self
    where
        F: Fn(Status) + Send + Sync + 'static,
    {
        self.handle_connection_state_change = Some(Arc::new(handler));
        self
    }

    /// Register a handler asked to trust a server certificate that failed CA/hostname
    /// verification and does not match the stored fingerprint
    /// ([`crate::config::Entrypoint::server_cert`]).
    ///
    /// Return `true` to accept the certificate for this session. Callers implementing
    /// trust-on-first-use should persist [`PeerCertPrompt::fingerprint`] on acceptance and pass it
    /// as `server_cert` for future connections.
    ///
    /// The handler runs synchronously on the thread driving `libopenconnect` (inside
    /// `connect_for_cookie` / `init_connection` / reconnects) while the client's internal
    /// handle lock is held. It may block (e.g. waiting for a UI dialog), but it **must not**
    /// call methods of the same [`crate::VpnClient`], or it will deadlock.
    pub fn with_handle_peer_cert_verify<F>(mut self, handler: F) -> Self
    where
        F: Fn(&PeerCertPrompt) -> bool + Send + Sync + 'static,
    {
        self.handle_peer_cert_verify = Some(Arc::new(handler));
        self
    }

    /// Legacy handler receiving only the certificate fingerprint.
    #[deprecated(
        since = "0.2.2",
        note = "use `with_handle_peer_cert_verify`, which also reports host, reason, details and whether the stored fingerprint mismatches"
    )]
    pub fn with_handle_peer_cert_invalid<F>(self, handler: F) -> Self
    where
        F: Fn(&str) -> bool + Send + Sync + 'static,
    {
        self.with_handle_peer_cert_verify(move |prompt| handler(&prompt.fingerprint))
    }
}

pub(crate) trait Events {
    fn emit_state_change(&self, status: Status);
    fn emit_error(&self, error: &OpenconnectError);
}
