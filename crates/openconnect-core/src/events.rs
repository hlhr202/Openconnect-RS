use crate::{result::OpenconnectError, Status};
use std::sync::Arc;

#[allow(clippy::type_complexity)]
#[derive(Clone)]
pub struct EventHandlers {
    pub(crate) handle_connection_state_change: Option<Arc<dyn Fn(Status)>>,
    pub(crate) handle_peer_cert_invalid: Option<Arc<dyn Fn(&str) -> bool>>,
}

impl EventHandlers {
    pub fn new() -> Self {
        Self {
            handle_connection_state_change: None,
            handle_peer_cert_invalid: None,
        }
    }

    pub fn with_handle_connection_state_change<F>(mut self, handler: F) -> Self
    where
        F: Fn(Status),
        F: Send + 'static,
    {
        self.handle_connection_state_change = Some(Arc::new(handler));
        self
    }

    pub fn with_handle_peer_cert_invalid<F>(mut self, handler: F) -> Self
    where
        F: Fn(&str) -> bool,
        F: Send + 'static,
    {
        self.handle_peer_cert_invalid = Some(Arc::new(handler));
        self
    }
}

impl Default for EventHandlers {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) trait Events {
    fn emit_state_change(&self, status: Status);
    fn emit_error(&self, error: &OpenconnectError);
}
