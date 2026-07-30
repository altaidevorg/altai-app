//! Durable replay and legacy session-key compatibility shared by all hosts.

use std::fmt;

use serde::{Deserialize, Serialize};

use altai_core::journal::EventJournal;

use crate::Event;

/// A host-neutral, opaque root-session identity.
///
/// New hosts keep their own namespace, while Desktop's historical
/// `tauri:<chat_id>:` keys remain an explicit read alias rather than an
/// implicit parser scattered through host adapters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionIdentity(String);

impl SessionIdentity {
    pub fn parse(value: &str) -> Result<Self, ReplayError> {
        let value = value.trim();
        if value.is_empty() {
            return Err(ReplayError::InvalidIdentity("chat id is required"));
        }
        if value.len() > 256 {
            return Err(ReplayError::InvalidIdentity("chat id is too long"));
        }
        if value.contains(':') {
            return Err(ReplayError::InvalidIdentity("chat id contains a delimiter"));
        }
        Ok(Self(value.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn legacy_tauri_thread_id(&self) -> String {
        format!("tauri:{}:", self.0)
    }

    pub fn from_legacy_tauri_thread_id(thread_id: &str) -> Option<Self> {
        let chat_id = thread_id.strip_prefix("tauri:")?.strip_suffix(':')?;
        // `parse` accepts surrounding whitespace at the public host boundary,
        // but persisted legacy keys must round-trip exactly. Otherwise a
        // malformed database key could be projected as a chat that Desktop
        // can no longer address through `legacy_tauri_thread_id`.
        if chat_id.trim() != chat_id {
            return None;
        }
        Self::parse(chat_id).ok()
    }

    pub fn matches_legacy_tauri_thread_id(&self, thread_id: &str) -> bool {
        thread_id == self.legacy_tauri_thread_id()
    }
}

/// Owned replay form of a live event envelope. It intentionally matches the
/// Desktop webview contract so a reconnecting host can reduce replayed and
/// live events through the same sequence-aware reducer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentReplayEventEnvelope {
    pub version: u8,
    pub scope: String,
    pub run_id: String,
    pub seq: u64,
    pub chat_id: String,
    pub event: Event,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRunReplayCursor {
    pub run_id: String,
    pub last_seq: u64,
    pub terminal_seq: Option<u64>,
}

/// Query-only durable API. It does not construct an agent, touch provider
/// configuration, or depend on Tauri/stdin/webview state.
pub struct ReplayService<'a> {
    journal: &'a EventJournal,
}

impl<'a> ReplayService<'a> {
    pub fn new(journal: &'a EventJournal) -> Self {
        Self { journal }
    }

    pub fn replay_run_events(
        &self,
        chat_id: &SessionIdentity,
        run_id: &str,
        after_seq: u64,
        limit: usize,
    ) -> Result<Vec<AgentReplayEventEnvelope>, ReplayError> {
        let run_id = validate_run_id(run_id)?;
        if !(1..=1_000).contains(&limit) {
            return Err(ReplayError::InvalidLimit);
        }
        let summary = self
            .journal
            .run_summary(run_id)
            .map_err(ReplayError::Journal)?
            .filter(|summary| summary.chat_id == chat_id.as_str())
            .ok_or(ReplayError::RunNotFound)?;
        if summary.run_id != run_id {
            return Err(ReplayError::InvalidJournalIdentity);
        }
        self.journal
            .fetch_after(run_id, after_seq, limit)
            .map_err(ReplayError::Journal)?
            .into_iter()
            .map(|record| {
                if record.version != 1
                    || record.run_id != run_id
                    || record.chat_id != chat_id.as_str()
                    || record.seq <= after_seq
                {
                    return Err(ReplayError::InvalidJournalEnvelope);
                }
                let payload_kind = record
                    .payload
                    .get("type")
                    .and_then(serde_json::Value::as_str);
                if payload_kind != Some(record.kind.as_str()) {
                    return Err(ReplayError::PayloadKindMismatch);
                }
                let event =
                    serde_json::from_value(record.payload).map_err(ReplayError::InvalidEvent)?;
                Ok(AgentReplayEventEnvelope {
                    version: 1,
                    scope: "run".to_string(),
                    run_id: record.run_id,
                    seq: record.seq,
                    chat_id: record.chat_id,
                    event,
                })
            })
            .collect()
    }

    pub fn latest_run_replay_cursor(
        &self,
        chat_id: &SessionIdentity,
    ) -> Result<Option<AgentRunReplayCursor>, ReplayError> {
        self.journal
            .latest_run_summary_for_chat(chat_id.as_str())
            .map(|summary| {
                summary.map(|summary| AgentRunReplayCursor {
                    run_id: summary.run_id,
                    last_seq: summary.last_seq,
                    terminal_seq: summary.terminal_seq,
                })
            })
            .map_err(ReplayError::Journal)
    }
}

fn validate_run_id(run_id: &str) -> Result<&str, ReplayError> {
    let run_id = run_id.trim();
    if run_id.is_empty() {
        return Err(ReplayError::InvalidRunId);
    }
    if run_id.len() > 256 {
        return Err(ReplayError::RunIdTooLong);
    }
    Ok(run_id)
}

#[derive(Debug)]
pub enum ReplayError {
    InvalidIdentity(&'static str),
    InvalidRunId,
    RunIdTooLong,
    InvalidLimit,
    RunNotFound,
    InvalidJournalIdentity,
    InvalidJournalEnvelope,
    PayloadKindMismatch,
    Journal(altai_core::journal::JournalError),
    InvalidEvent(serde_json::Error),
}

impl fmt::Display for ReplayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentity(detail) => write!(f, "invalid session identity: {detail}"),
            Self::InvalidRunId => f.write_str("runId is required"),
            Self::RunIdTooLong => f.write_str("runId is too long"),
            Self::InvalidLimit => f.write_str("limit must be between 1 and 1000"),
            Self::RunNotFound => f.write_str("run was not found for this chat"),
            Self::InvalidJournalIdentity => f.write_str("journal returned an invalid run identity"),
            Self::InvalidJournalEnvelope => {
                f.write_str("journal returned an invalid event envelope")
            }
            Self::PayloadKindMismatch => {
                f.write_str("journal event type does not match its payload")
            }
            Self::Journal(error) => write!(f, "failed to inspect agent event journal: {error}"),
            Self::InvalidEvent(error) => {
                write!(f, "journal contains an invalid agent event: {error}")
            }
        }
    }
}

impl std::error::Error for ReplayError {}

#[cfg(test)]
mod tests {
    use super::*;
    use altai_core::journal::JournalEvent;

    #[test]
    fn legacy_desktop_session_key_is_an_explicit_compatibility_alias() {
        let identity = SessionIdentity::parse("chat-1").unwrap();
        assert_eq!(identity.legacy_tauri_thread_id(), "tauri:chat-1:");
        assert!(identity.matches_legacy_tauri_thread_id("tauri:chat-1:"));
        assert!(!identity.matches_legacy_tauri_thread_id("vscode:chat-1:"));
        assert_eq!(
            SessionIdentity::from_legacy_tauri_thread_id("tauri:chat-1:").unwrap(),
            identity
        );
        assert!(SessionIdentity::from_legacy_tauri_thread_id("tauri: chat-1 :").is_none());
        assert!(SessionIdentity::from_legacy_tauri_thread_id("tauri:chat-1").is_none());
    }

    #[test]
    fn replay_is_chat_scoped_and_strictly_after_cursor() {
        let dir = tempfile::tempdir().unwrap();
        let journal = EventJournal::open(dir.path().join("events.db")).unwrap();
        let payload = serde_json::json!({"type":"run_started", "run_id":"run-1"});
        journal
            .append(&JournalEvent::now(
                1,
                "run-1",
                1,
                "chat-1",
                "run_started",
                payload,
            ))
            .unwrap();
        let service = ReplayService::new(&journal);
        let chat = SessionIdentity::parse("chat-1").unwrap();
        assert_eq!(
            service.replay_run_events(&chat, "run-1", 0, 10).unwrap()[0].seq,
            1
        );
        assert!(service
            .replay_run_events(&chat, "run-1", 1, 10)
            .unwrap()
            .is_empty());
        assert!(service
            .replay_run_events(&SessionIdentity::parse("chat-2").unwrap(), "run-1", 0, 10)
            .is_err());
    }

    #[test]
    fn latest_cursor_is_scoped_to_its_session_identity() {
        let dir = tempfile::tempdir().unwrap();
        let journal = EventJournal::open(dir.path().join("events.db")).unwrap();
        for (run_id, chat_id) in [
            ("run-1", "chat-1"),
            ("run-2", "chat-1"),
            ("run-3", "chat-2"),
        ] {
            journal
                .append(&JournalEvent::now(
                    1,
                    run_id,
                    1,
                    chat_id,
                    "run_started",
                    serde_json::json!({"type":"run_started", "run_id": run_id}),
                ))
                .unwrap();
        }

        let service = ReplayService::new(&journal);
        let cursor = service
            .latest_run_replay_cursor(&SessionIdentity::parse("chat-1").unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(cursor.run_id, "run-2");
        assert_eq!(cursor.last_seq, 1);
        assert_eq!(cursor.terminal_seq, None);
        assert!(service
            .latest_run_replay_cursor(&SessionIdentity::parse("chat-unknown").unwrap())
            .unwrap()
            .is_none());
    }
}
