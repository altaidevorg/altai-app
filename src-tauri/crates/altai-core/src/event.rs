//! Versioned machine-readable event primitives for `altai run --output jsonl`.

use serde::Serialize;

/// The first stable JSONL event schema for the ALTAI terminal product.
pub const EVENT_SCHEMA_VERSION: u16 = 1;

/// A single JSONL record emitted by the CLI.
///
/// The payload is generic so command-specific event types can stay structured
/// without weakening the envelope contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EventEnvelope<T> {
    pub schema_version: u16,
    pub sequence: u64,
    pub timestamp_ms: u64,
    pub workspace: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    #[serde(rename = "type")]
    pub event_type: String,
    pub data: T,
}

impl<T> EventEnvelope<T> {
    pub fn new(
        sequence: u64,
        timestamp_ms: u64,
        workspace: impl Into<String>,
        event_type: impl Into<String>,
        data: T,
    ) -> Self {
        Self {
            schema_version: EVENT_SCHEMA_VERSION,
            sequence,
            timestamp_ms,
            workspace: workspace.into(),
            chat_id: None,
            run_id: None,
            event_type: event_type.into(),
            data,
        }
    }

    pub fn with_chat_id(mut self, chat_id: impl Into<String>) -> Self {
        self.chat_id = Some(chat_id.into());
        self
    }

    pub fn with_run_id(mut self, run_id: impl Into<String>) -> Self {
        self.run_id = Some(run_id.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_serializes_to_the_public_contract() {
        let event = EventEnvelope::new(
            12,
            1_785_300_000_000,
            "/workspace",
            "run_started",
            serde_json::json!({ "model": "test" }),
        )
        .with_chat_id("chat-1")
        .with_run_id("run-1");

        let value = serde_json::to_value(event).expect("event serializes");
        assert_eq!(value["schema_version"], EVENT_SCHEMA_VERSION);
        assert_eq!(value["type"], "run_started");
        assert_eq!(value["chat_id"], "chat-1");
        assert_eq!(value["run_id"], "run-1");
    }
}
