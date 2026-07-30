//! Desktop adapter for the host-neutral agent event sink.

use altai_agent_service::{AgentEventEnvelope, AgentEventSink, AgentEventSinkError};
use tauri::{AppHandle, Emitter};

/// Delivers the unchanged public event envelope to the Desktop renderer.
///
/// Tauri emits synchronously to its event bus, so this adapter has no hidden
/// unbounded queue: once `try_send` returns `Ok`, the host accepted it.
#[derive(Clone)]
pub struct TauriEventSink {
    app: AppHandle,
}

impl TauriEventSink {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl AgentEventSink for TauriEventSink {
    fn try_send(&self, envelope: AgentEventEnvelope) -> Result<(), AgentEventSinkError> {
        self.app
            .emit("agent://event", envelope)
            .map_err(|error| AgentEventSinkError::Unavailable(error.to_string()))
    }
}
