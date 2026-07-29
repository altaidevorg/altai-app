//! Bridges `altai run` into the same durable SQLite event journal ALTAI
//! Desktop uses, so restart-discovery and `altai journal` tooling see CLI
//! runs regardless of which surface produced them.
//!
//! This is intentionally a minimal subset: only a `run_started` event (once
//! the bus reports one) and a single `run_terminated` event once the oneshot
//! host returns a final outcome. Failures to append are logged to stderr and
//! never fail the run itself.

use crate::run_output::describe_oneshot_outcome;
use altai_core::{AppendStatus, EventJournal, JournalEvent, WorkspacePaths};
use isanagent::bus::{BusMessage, RunLifecycleEvent};
use isanagent::host::OneshotResult;
use serde_json::json;

const JOURNAL_EVENT_VERSION: u32 = 1;

pub struct JournalSink {
    journal: EventJournal,
    chat_id: Option<String>,
    run_id: Option<String>,
    next_seq: u64,
    terminated: bool,
}

impl JournalSink {
    /// Opens the workspace's event journal. Returns `None` (after logging to
    /// stderr) if the journal cannot be prepared; callers proceed without
    /// journaling rather than failing the run.
    pub fn open(workspace: &WorkspacePaths) -> Option<Self> {
        let path = workspace.agent_event_journal_db();
        if let Some(parent) = path.parent() {
            if let Err(error) = std::fs::create_dir_all(parent) {
                eprintln!("altai-cli: could not prepare event journal directory: {error}");
                return None;
            }
        }
        match EventJournal::open(&path) {
            Ok(journal) => Some(Self {
                journal,
                chat_id: None,
                run_id: None,
                next_seq: 1,
                terminated: false,
            }),
            Err(error) => {
                eprintln!("altai-cli: could not open event journal: {error}");
                None
            }
        }
    }

    /// Records a `run_started` event the first time the bus reports one.
    pub fn observe_bus_message(&mut self, message: &BusMessage) {
        if let BusMessage::RunLifecycle(RunLifecycleEvent::Started { run_id, chat_id }) = message {
            if self.run_id.is_some() {
                return;
            }
            self.chat_id = Some(chat_id.clone());
            self.run_id = Some(run_id.clone());
            self.append(
                "run_started",
                json!({ "type": "run_started", "run_id": run_id }),
                false,
            );
        }
    }

    /// Ensures a terminal event is committed once the oneshot host returns.
    /// Synthesizes the run identity from the final result when no
    /// `run_started` bus message was ever observed (for example, the host
    /// failed before the run began).
    pub fn finalize(&mut self, result: &OneshotResult) {
        if self.terminated {
            return;
        }
        let run_id = self.run_id.clone().or_else(|| result.run_id.clone());
        let chat_id = self
            .chat_id
            .clone()
            .or_else(|| (!result.chat_id.is_empty()).then(|| result.chat_id.clone()));
        let (Some(run_id), Some(chat_id)) = (run_id, chat_id) else {
            return;
        };
        if self.run_id.is_none() {
            self.chat_id = Some(chat_id);
            self.run_id = Some(run_id.clone());
            self.append(
                "run_started",
                json!({ "type": "run_started", "run_id": run_id }),
                false,
            );
        }

        let (kind, detail) = describe_oneshot_outcome(&result.outcome);
        let mut outcome = json!({ "kind": kind });
        if let Some(detail) = detail {
            outcome["detail"] = json!(detail);
        }
        self.append(
            "run_terminated",
            json!({ "type": "run_terminated", "run_id": run_id, "outcome": outcome }),
            true,
        );
    }

    fn append(&mut self, kind: &str, payload: serde_json::Value, terminal: bool) {
        let (Some(run_id), Some(chat_id)) = (self.run_id.clone(), self.chat_id.clone()) else {
            return;
        };
        let seq = self.next_seq;
        let event = JournalEvent::now(JOURNAL_EVENT_VERSION, run_id, seq, chat_id, kind, payload);
        let outcome = if terminal {
            self.journal.append_terminal(&event)
        } else {
            self.journal.append(&event)
        };
        match outcome {
            Ok(AppendStatus::Appended) => {
                self.next_seq += 1;
                if terminal {
                    self.terminated = true;
                }
            }
            Ok(AppendStatus::Duplicate) => {
                if terminal {
                    self.terminated = true;
                }
            }
            Err(error) => {
                eprintln!("altai-cli: failed to append event journal entry ({kind}): {error}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use isanagent::host::OneshotOutcome;

    fn workspace(root: &std::path::Path) -> WorkspacePaths {
        WorkspacePaths {
            root: root.to_path_buf(),
            isanagent_state: root.join(".isanagent"),
        }
    }

    #[test]
    fn started_then_finalize_writes_two_ordered_events() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = workspace(temp.path());
        let mut sink = JournalSink::open(&workspace).expect("journal opens");

        sink.observe_bus_message(&BusMessage::RunLifecycle(RunLifecycleEvent::Started {
            run_id: "run-1".to_string(),
            chat_id: "chat-1".to_string(),
        }));
        sink.finalize(&OneshotResult {
            chat_id: "chat-1".to_string(),
            run_id: Some("run-1".to_string()),
            outcome: OneshotOutcome::Completed,
            final_text: Some("done".to_string()),
        });

        let journal = EventJournal::open(workspace.agent_event_journal_db()).expect("reopen");
        let summary = journal
            .run_summary("run-1")
            .expect("summary query")
            .expect("run summary");
        assert_eq!(summary.last_seq, 2);
        assert_eq!(summary.terminal_kind.as_deref(), Some("run_terminated"));
        assert_eq!(
            summary
                .terminal_payload
                .as_ref()
                .and_then(|value| value["outcome"]["kind"].as_str()),
            Some("completed")
        );
    }

    #[test]
    fn finalize_without_bus_message_synthesizes_run_started() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = workspace(temp.path());
        let mut sink = JournalSink::open(&workspace).expect("journal opens");

        sink.finalize(&OneshotResult {
            chat_id: "chat-2".to_string(),
            run_id: Some("run-2".to_string()),
            outcome: OneshotOutcome::Failed("boom".to_string()),
            final_text: None,
        });

        let journal = EventJournal::open(workspace.agent_event_journal_db()).expect("reopen");
        let events = journal.fetch_after("run-2", 0, 10).expect("fetch events");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].kind, "run_started");
        assert_eq!(events[1].kind, "run_terminated");
        assert_eq!(events[1].payload["outcome"]["detail"], json!("boom"));
    }

    #[test]
    fn finalize_without_any_run_identity_is_a_no_op() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = workspace(temp.path());
        let mut sink = JournalSink::open(&workspace).expect("journal opens");

        sink.finalize(&OneshotResult {
            chat_id: String::new(),
            run_id: None,
            outcome: OneshotOutcome::Cancelled,
            final_text: None,
        });

        let journal = EventJournal::open(workspace.agent_event_journal_db()).expect("reopen");
        assert!(journal
            .incomplete_run_summaries()
            .expect("query")
            .is_empty());
    }
}
