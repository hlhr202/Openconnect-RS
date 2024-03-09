use crate::Status;

pub struct EventHandlers {
    pub(crate) handle_connection_state_change: Option<Box<dyn Fn(Status)>>,
}

impl EventHandlers {
    pub fn new() -> Self {
        Self {
            handle_connection_state_change: None,
        }
    }

    pub fn with_handle_connection_state_change<F>(mut self, handler: F) -> Self
    where
        F: Fn(Status),
        F: Send + 'static,
    {
        self.handle_connection_state_change = Some(Box::new(handler));
        self
    }
}

impl Default for EventHandlers {
    fn default() -> Self {
        Self::new()
    }
}

pub trait Events {
    fn emit_state_change(&self, status: Status);
}
