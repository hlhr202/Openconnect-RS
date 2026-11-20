use crate::{Status, result::OpenconnectError};
use std::sync::Arc;

/// Thread-safe event handlers for [`crate::VpnClient`].
#[allow(clippy::type_complexity)]
#[derive(Clone, Default)]
pub struct EventHandlers {
    pub(crate) handle_connection_state_change: Option<Arc<dyn Fn(Status) + Send + Sync + 'static>>,
    pub(crate) handle_peer_cert_invalid: Option<Arc<dyn Fn(&str) -> bool + Send + Sync + 'static>>,
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

    pub fn with_handle_peer_cert_invalid<F>(mut self, handler: F) -> Self
    where
        F: Fn(&str) -> bool + Send + Sync + 'static,
    {
        self.handle_peer_cert_invalid = Some(Arc::new(handler));
        self
    }
}

pub(crate) trait Events {
    fn emit_state_change(&self, status: Status);
    fn emit_error(&self, error: &OpenconnectError);
}
