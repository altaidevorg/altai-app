//! Bridges `altai run` into the same durable SQLite event journal ALTAI
//! Desktop uses, so restart-discovery and `altai journal` tooling see CLI
//! runs regardless of which surface produced them.
//!
//! Lifecycle identity comes from `run_started` / oneshot finalize. After the
//! run is known, selected IsanAgent telemetry and outbound messages are
//! mirrored with Desktop's payload vocabulary (`tool_call_*`, `thinking`,
//! `usage`, `agent_message`, `clarification`). Failures to append are logged
//! to stderr and never fail the run itself.

use crate::run_output::describe_oneshot_outcome;
use altai_core::{AppendStatus, EventJournal, JournalEvent, WorkspacePaths};
use isanagent::bus::{BusMessage, OutboundMessage, RunLifecycleEvent, TelemetryEvent};
use isanagent::host::OneshotResult;
use serde_json::{json, Value};

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

    /// Records `run_started` once, then mirrors selected run-scoped telemetry
    /// and outbound assistant / clarification messages.
    pub fn observe_bus_message(&mut self, message: &BusMessage) {
        if self.terminated {
            return;
        }
        match message {
            BusMessage::RunLifecycle(RunLifecycleEvent::Started { run_id, chat_id }) => {
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
            BusMessage::RunLifecycle(RunLifecycleEvent::Warning {
                run_id,
                chat_id,
                warning,
            }) => {
                if !self.matches_run(chat_id, run_id) {
                    return;
                }
                let warning_value = serde_json::to_value(warning).unwrap_or_else(|_| json!({}));
                self.append_redacted(
                    "run_warning",
                    json!({
                        "type": "run_warning",
                        "run_id": run_id,
                        "warning": warning_value,
                    }),
                );
            }
            BusMessage::RunLifecycle(RunLifecycleEvent::WarningCleared { run_id, chat_id }) => {
                if !self.matches_run(chat_id, run_id) {
                    return;
                }
                self.append_redacted(
                    "run_warning_cleared",
                    json!({
                        "type": "run_warning_cleared",
                        "run_id": run_id,
                    }),
                );
            }
            // Terminated is ignored: oneshot `finalize` is authoritative for the
            // terminal journal row (same as the existing M5 contract).
            BusMessage::RunLifecycle(RunLifecycleEvent::Terminated { .. }) => {}
            BusMessage::Telemetry(telemetry) => {
                if !self.matches_run_chat(telemetry_scope_chat_id(telemetry)) {
                    return;
                }
                if let Some((kind, payload)) = map_telemetry_payload(telemetry) {
                    self.append_redacted(kind, payload);
                }
            }
            BusMessage::Outbound(outbound) => {
                // Keep child-chat outbound (subagent traffic) out of the parent
                // run journal — Desktop scopes outbound by outbound.chat_id.
                if !self.matches_run_chat(Some(outbound.chat_id.as_str())) {
                    return;
                }
                let (kind, payload) = map_outbound_payload(outbound);
                self.append_redacted(kind, payload);
            }
            _ => {}
        }
    }

    fn matches_run(&self, chat_id: &str, run_id: &str) -> bool {
        match (&self.run_id, &self.chat_id) {
            (Some(active_run), Some(active_chat)) => {
                active_run == run_id && active_chat == chat_id
            }
            _ => false,
        }
    }

    fn matches_run_chat(&self, chat_id: Option<&str>) -> bool {
        match (&self.run_id, &self.chat_id, chat_id) {
            (Some(_), Some(run_chat), Some(event_chat)) => run_chat == event_chat,
            _ => false,
        }
    }

    fn append_redacted(&mut self, kind: &str, mut payload: Value) {
        isanagent::redact::shared().redact_json(&mut payload);
        self.append(kind, payload, false);
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

    fn append(&mut self, kind: &str, payload: Value, terminal: bool) {
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

/// Maps IsanAgent outbound onto Desktop's durable event kinds/payloads.
/// Clarification detection matches Desktop: metadata key presence
/// (`METADATA_CLARIFICATION`), not a boolean true check.
fn map_outbound_payload(outbound: &OutboundMessage) -> (&'static str, Value) {
    let is_clarification = outbound
        .metadata
        .contains_key(isanagent::clarification::METADATA_CLARIFICATION);
    if !is_clarification {
        return (
            "agent_message",
            json!({
                "type": "agent_message",
                "content": outbound.content,
                "role": "assistant",
            }),
        );
    }

    let choices = outbound
        .metadata
        .get(isanagent::clarification::METADATA_CLARIFICATION_CHOICES)
        .and_then(|value| value.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let mut payload = json!({
        "type": "clarification",
        "content": outbound.content,
        "choices": choices,
    });
    if let Some(edit_diff) = outbound.metadata.get("edit_diff").and_then(parse_edit_diff) {
        payload["edit_diff"] = edit_diff;
    }
    ("clarification", payload)
}

fn parse_edit_diff(value: &Value) -> Option<Value> {
    let obj = value.as_object()?;
    let file = obj.get("file").and_then(|v| v.as_str())?.to_string();
    let diff = obj.get("diff").and_then(|v| v.as_str())?.to_string();
    let truncated = obj
        .get("truncated")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    Some(json!({
        "file": file,
        "diff": diff,
        "truncated": truncated,
    }))
}

/// Chat scope used to decide whether telemetry belongs on the active run.
/// Subagent lifecycle events are attributed to the parent chat (Desktop parity).
fn telemetry_scope_chat_id(telemetry: &TelemetryEvent) -> Option<&str> {
    match telemetry {
        TelemetryEvent::ToolCall { chat_id, .. }
        | TelemetryEvent::ToolResult { chat_id, .. }
        | TelemetryEvent::AgentThought { chat_id, .. }
        | TelemetryEvent::AgentUsage { chat_id, .. }
        | TelemetryEvent::ToolProgress { chat_id, .. }
        | TelemetryEvent::ExecutionRunFinished { chat_id, .. }
        | TelemetryEvent::ExecutionJobFinished { chat_id, .. } => Some(chat_id.as_str()),
        TelemetryEvent::SubagentSpawned { parent_chat_id, .. }
        | TelemetryEvent::SubagentFinished { parent_chat_id, .. } => Some(parent_chat_id.as_str()),
        _ => None,
    }
}

/// Maps IsanAgent telemetry onto Desktop's durable event kinds/payloads.
/// Legacy `ToolCallStarted` / `ToolCallFinished` are intentionally ignored so
/// each logical tool invocation is journaled once (matching Desktop).
fn map_telemetry_payload(telemetry: &TelemetryEvent) -> Option<(&'static str, Value)> {
    match telemetry {
        TelemetryEvent::ToolCall {
            tool_name,
            tool_call_id,
            args,
            ..
        } => {
            let id = tool_call_id
                .clone()
                .unwrap_or_else(|| uuid_fallback(tool_name));
            let input = serde_json::from_str(args).unwrap_or(Value::String(args.clone()));
            Some((
                "tool_call_start",
                json!({
                    "type": "tool_call_start",
                    "id": id,
                    "name": tool_name,
                    "input": input,
                }),
            ))
        }
        TelemetryEvent::ToolResult {
            tool_name,
            tool_call_id,
            result,
            is_error,
            ..
        } => {
            let id = tool_call_id
                .clone()
                .unwrap_or_else(|| tool_name.clone());
            let mut payload = json!({
                "type": "tool_call_end",
                "id": id,
                "name": tool_name,
                "output": result,
            });
            if *is_error {
                payload["error"] = json!(result);
            }
            Some(("tool_call_end", payload))
        }
        TelemetryEvent::AgentThought { thought, .. } => Some((
            "thinking",
            json!({ "type": "thinking", "content": thought }),
        )),
        TelemetryEvent::ToolProgress {
            tool_name, message, ..
        } => Some((
            "thinking",
            json!({
                "type": "thinking",
                "content": format!("[{tool_name}] {message}"),
            }),
        )),
        TelemetryEvent::AgentUsage {
            prompt_tokens,
            completion_tokens,
            total_tokens,
            cache_read_tokens,
            cache_creation_tokens,
            ..
        } => Some((
            "usage",
            json!({
                "type": "usage",
                "prompt_tokens": prompt_tokens,
                "completion_tokens": completion_tokens,
                "total_tokens": total_tokens,
                "cache_read_tokens": cache_read_tokens,
                "cache_creation_tokens": cache_creation_tokens,
            }),
        )),
        TelemetryEvent::ExecutionRunFinished {
            provider_id,
            session_id,
            exit_code,
            duration_ms,
            stdout_len,
            stderr_len,
            artifact_count,
            git_head,
            description,
            ..
        } => Some((
            "execution_run_finished",
            json!({
                "type": "execution_run_finished",
                "provider_id": provider_id,
                "session_id": session_id,
                "exit_code": exit_code,
                "duration_ms": duration_ms,
                "stdout_len": stdout_len,
                "stderr_len": stderr_len,
                "artifact_count": artifact_count,
                "git_head": git_head,
                "description": description,
            }),
        )),
        TelemetryEvent::ExecutionJobFinished {
            job_id,
            session_id,
            provider_id,
            status,
            exit_code,
            duration_ms,
            stdout_len,
            stderr_len,
            artifact_count,
            description,
            ..
        } => Some((
            "execution_job_finished",
            json!({
                "type": "execution_job_finished",
                "job_id": job_id,
                "session_id": session_id,
                "provider_id": provider_id,
                "status": status,
                "exit_code": exit_code,
                "duration_ms": duration_ms,
                "stdout_len": stdout_len,
                "stderr_len": stderr_len,
                "artifact_count": artifact_count,
                "description": description,
            }),
        )),
        TelemetryEvent::SubagentSpawned {
            child_chat_id,
            task_id,
            display_name,
            agent_name,
            background_job_id,
            ..
        } => Some((
            "subagent_spawned",
            json!({
                "type": "subagent_spawned",
                "task_id": task_id,
                "child_chat_id": child_chat_id,
                "display_name": display_name,
                "agent_name": agent_name,
                "background_job_id": background_job_id,
            }),
        )),
        TelemetryEvent::SubagentFinished {
            child_chat_id,
            task_id,
            status,
            agent_name,
            ..
        } => Some((
            "subagent_finished",
            json!({
                "type": "subagent_finished",
                "task_id": task_id,
                "child_chat_id": child_chat_id,
                "status": status,
                "agent_name": agent_name,
            }),
        )),
        _ => None,
    }
}

fn uuid_fallback(tool_name: &str) -> String {
    // Prefer a stable-looking id when IsanAgent omits tool_call_id. Desktop
    // uses Uuid::new_v4(); CLI avoids a uuid crate dep and still keeps ids
    // unique enough for journal inspection.
    format!("{tool_name}-{}", next_seq())
}

fn next_seq() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, Ordering::Relaxed)
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

    fn start_sink(root: &std::path::Path) -> JournalSink {
        let workspace = workspace(root);
        let mut sink = JournalSink::open(&workspace).expect("journal opens");
        sink.observe_bus_message(&BusMessage::RunLifecycle(RunLifecycleEvent::Started {
            run_id: "run-1".to_string(),
            chat_id: "chat-1".to_string(),
        }));
        sink
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

    #[test]
    fn mirrors_tool_thought_usage_and_progress_in_desktop_shape() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = workspace(temp.path());
        let mut sink = start_sink(temp.path());

        sink.observe_bus_message(&BusMessage::Telemetry(TelemetryEvent::ToolCall {
            chat_id: "chat-1".into(),
            channel: "altai-cli".into(),
            tool_name: "read_file".into(),
            args: r#"{"path":"/x"}"#.into(),
            tool_call_id: Some("tc1".into()),
            background_job_id: None,
        }));
        sink.observe_bus_message(&BusMessage::Telemetry(TelemetryEvent::ToolResult {
            chat_id: "chat-1".into(),
            channel: "altai-cli".into(),
            tool_name: "read_file".into(),
            result: "hello".into(),
            is_error: false,
            tool_call_id: Some("tc1".into()),
            background_job_id: None,
        }));
        sink.observe_bus_message(&BusMessage::Telemetry(TelemetryEvent::AgentThought {
            chat_id: "chat-1".into(),
            thought: "hmm".into(),
            background_job_id: None,
        }));
        sink.observe_bus_message(&BusMessage::Telemetry(TelemetryEvent::ToolProgress {
            chat_id: "chat-1".into(),
            channel: "altai-cli".into(),
            tool_name: "execution_run".into(),
            tool_call_id: Some("tc2".into()),
            message: "installing deps".into(),
            background_job_id: None,
        }));
        sink.observe_bus_message(&BusMessage::Telemetry(TelemetryEvent::AgentUsage {
            chat_id: "chat-1".into(),
            model: "gpt-4".into(),
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
            cache_read_tokens: 10,
            cache_creation_tokens: 2,
            background_job_id: None,
        }));
        // Legacy duplicates must not create extra journal rows.
        sink.observe_bus_message(&BusMessage::Telemetry(TelemetryEvent::ToolCallStarted {
            chat_id: "chat-1".into(),
            tool_name: "read_file".into(),
            args: r#"{"path":"/x"}"#.into(),
            tool_call_id: Some("tc1".into()),
            background_job_id: None,
        }));
        sink.observe_bus_message(&BusMessage::Telemetry(TelemetryEvent::ToolCallFinished {
            chat_id: "chat-1".into(),
            tool_name: "read_file".into(),
            result: "hello".into(),
            is_error: false,
            tool_call_id: Some("tc1".into()),
            background_job_id: None,
        }));

        sink.finalize(&OneshotResult {
            chat_id: "chat-1".to_string(),
            run_id: Some("run-1".to_string()),
            outcome: OneshotOutcome::Completed,
            final_text: Some("done".to_string()),
        });

        let journal = EventJournal::open(workspace.agent_event_journal_db()).expect("reopen");
        let events = journal.fetch_after("run-1", 0, 20).expect("fetch");
        let kinds: Vec<&str> = events.iter().map(|e| e.kind.as_str()).collect();
        assert_eq!(
            kinds,
            [
                "run_started",
                "tool_call_start",
                "tool_call_end",
                "thinking",
                "thinking",
                "usage",
                "run_terminated",
            ]
        );
        assert_eq!(events[1].payload["input"], json!({"path": "/x"}));
        assert_eq!(events[2].payload["output"], json!("hello"));
        assert!(events[2].payload.get("error").is_none());
        assert_eq!(events[3].payload["content"], json!("hmm"));
        assert_eq!(
            events[4].payload["content"],
            json!("[execution_run] installing deps")
        );
        assert_eq!(events[5].payload["prompt_tokens"], 100);
        assert_eq!(events[5].payload["cache_read_tokens"], 10);
        assert_eq!(events[5].payload["cache_creation_tokens"], 2);
        assert_eq!(events.last().map(|e| e.seq), Some(7));
    }

    #[test]
    fn failed_tool_result_records_error_field() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = workspace(temp.path());
        let mut sink = start_sink(temp.path());

        sink.observe_bus_message(&BusMessage::Telemetry(TelemetryEvent::ToolResult {
            chat_id: "chat-1".into(),
            channel: "altai-cli".into(),
            tool_name: "edit_file".into(),
            result: "Error: old_text not found".into(),
            is_error: true,
            tool_call_id: Some("tc-err".into()),
            background_job_id: None,
        }));

        let journal = EventJournal::open(workspace.agent_event_journal_db()).expect("reopen");
        let events = journal.fetch_after("run-1", 0, 10).expect("fetch");
        assert_eq!(events[1].kind, "tool_call_end");
        assert_eq!(
            events[1].payload["error"],
            json!("Error: old_text not found")
        );
        assert_eq!(
            events[1].payload["output"],
            json!("Error: old_text not found")
        );
    }

    #[test]
    fn telemetry_before_run_started_is_ignored() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = workspace(temp.path());
        let mut sink = JournalSink::open(&workspace).expect("journal opens");

        sink.observe_bus_message(&BusMessage::Telemetry(TelemetryEvent::AgentThought {
            chat_id: "chat-1".into(),
            thought: "too early".into(),
            background_job_id: None,
        }));
        sink.finalize(&OneshotResult {
            chat_id: "chat-1".to_string(),
            run_id: Some("run-late".to_string()),
            outcome: OneshotOutcome::Completed,
            final_text: None,
        });

        let journal = EventJournal::open(workspace.agent_event_journal_db()).expect("reopen");
        let events = journal.fetch_after("run-late", 0, 10).expect("fetch");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].kind, "run_started");
        assert_eq!(events[1].kind, "run_terminated");
    }

    #[test]
    fn redacts_sensitive_tool_arguments_before_persist() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = workspace(temp.path());
        let mut sink = start_sink(temp.path());

        sink.observe_bus_message(&BusMessage::Telemetry(TelemetryEvent::ToolCall {
            chat_id: "chat-1".into(),
            channel: "altai-cli".into(),
            tool_name: "shell".into(),
            args: r#"{"command":"export OPENAI_API_KEY=sk-secret-token-123456"}"#.into(),
            tool_call_id: Some("tc-secret".into()),
            background_job_id: None,
        }));

        let journal = EventJournal::open(workspace.agent_event_journal_db()).expect("reopen");
        let events = journal.fetch_after("run-1", 0, 10).expect("fetch");
        let encoded = events[1].payload.to_string();
        assert!(
            !encoded.contains("sk-secret-token-123456"),
            "expected redaction, got {encoded}"
        );
    }

    fn outbound_from(value: Value) -> OutboundMessage {
        serde_json::from_value(value).expect("outbound message")
    }

    #[test]
    fn outbound_assistant_message_is_journaled() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = workspace(temp.path());
        let mut sink = start_sink(temp.path());

        sink.observe_bus_message(&BusMessage::Outbound(outbound_from(json!({
            "channel": "altai-cli",
            "chat_id": "chat-1",
            "content": "hello from agent",
            "metadata": {},
        }))));
        sink.finalize(&OneshotResult {
            chat_id: "chat-1".into(),
            run_id: Some("run-1".into()),
            outcome: OneshotOutcome::Completed,
            final_text: Some("hello from agent".into()),
        });

        let journal = EventJournal::open(workspace.agent_event_journal_db()).expect("reopen");
        let events = journal.fetch_after("run-1", 0, 10).expect("fetch");
        assert_eq!(
            events.iter().map(|e| e.kind.as_str()).collect::<Vec<_>>(),
            ["run_started", "agent_message", "run_terminated"]
        );
        assert_eq!(events[1].payload["content"], json!("hello from agent"));
        assert_eq!(events[1].payload["role"], json!("assistant"));
    }

    #[test]
    fn clarification_outbound_keeps_choices_and_valid_edit_diff() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = workspace(temp.path());
        let mut sink = start_sink(temp.path());

        sink.observe_bus_message(&BusMessage::Outbound(outbound_from(json!({
            "channel": "altai-cli",
            "chat_id": "chat-1",
            "content": "Approve edit?",
            "metadata": {
                "isanagent_clarification": true,
                "isanagent_clarification_choices": ["approve", "deny", 42, "abort"],
                "edit_diff": {
                    "file": "src/main.rs",
                    "diff": "--- a\n+++ b\n",
                    "truncated": false
                }
            },
        }))));

        let journal = EventJournal::open(workspace.agent_event_journal_db()).expect("reopen");
        let events = journal.fetch_after("run-1", 0, 10).expect("fetch");
        assert_eq!(events[1].kind, "clarification");
        assert_eq!(
            events[1].payload["choices"],
            json!(["approve", "deny", "abort"])
        );
        assert_eq!(events[1].payload["edit_diff"]["file"], json!("src/main.rs"));
        assert_eq!(events[1].payload["edit_diff"]["truncated"], json!(false));
    }

    #[test]
    fn malformed_edit_diff_is_omitted_from_clarification() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = workspace(temp.path());
        let mut sink = start_sink(temp.path());

        sink.observe_bus_message(&BusMessage::Outbound(outbound_from(json!({
            "channel": "altai-cli",
            "chat_id": "chat-1",
            "content": "Approve?",
            "metadata": {
                "isanagent_clarification": true,
                "edit_diff": { "file": 1, "diff": null }
            },
        }))));

        let journal = EventJournal::open(workspace.agent_event_journal_db()).expect("reopen");
        let events = journal.fetch_after("run-1", 0, 10).expect("fetch");
        assert_eq!(events[1].kind, "clarification");
        assert!(events[1].payload.get("edit_diff").is_none());
    }

    #[test]
    fn outbound_before_run_started_is_ignored() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = workspace(temp.path());
        let mut sink = JournalSink::open(&workspace).expect("journal opens");

        sink.observe_bus_message(&BusMessage::Outbound(outbound_from(json!({
            "channel": "altai-cli",
            "chat_id": "chat-1",
            "content": "too early",
            "metadata": {},
        }))));
        sink.finalize(&OneshotResult {
            chat_id: "chat-1".into(),
            run_id: Some("run-late".into()),
            outcome: OneshotOutcome::Completed,
            final_text: None,
        });

        let journal = EventJournal::open(workspace.agent_event_journal_db()).expect("reopen");
        let events = journal.fetch_after("run-late", 0, 10).expect("fetch");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].kind, "run_started");
        assert_eq!(events[1].kind, "run_terminated");
    }

    #[test]
    fn execution_and_subagent_telemetry_are_journaled() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = workspace(temp.path());
        let mut sink = start_sink(temp.path());

        sink.observe_bus_message(&BusMessage::Telemetry(
            TelemetryEvent::ExecutionRunFinished {
                chat_id: "chat-1".into(),
                channel: "altai-cli".into(),
                provider_id: "local".into(),
                session_id: "s1".into(),
                exit_code: Some(0),
                duration_ms: 12,
                stdout_len: 3,
                stderr_len: 0,
                artifact_count: 1,
                git_head: Some("abc".into()),
                description: Some("train".into()),
            },
        ));
        sink.observe_bus_message(&BusMessage::Telemetry(TelemetryEvent::SubagentSpawned {
            parent_chat_id: "chat-1".into(),
            child_chat_id: "child-1".into(),
            task_id: "t1".into(),
            display_name: Some("Research".into()),
            agent_name: Some("researcher".into()),
            background_job_id: None,
        }));
        sink.observe_bus_message(&BusMessage::Telemetry(TelemetryEvent::SubagentFinished {
            parent_chat_id: "chat-1".into(),
            child_chat_id: "child-1".into(),
            task_id: "t1".into(),
            status: "completed".into(),
            agent_name: Some("researcher".into()),
        }));

        let journal = EventJournal::open(workspace.agent_event_journal_db()).expect("reopen");
        let events = journal.fetch_after("run-1", 0, 10).expect("fetch");
        assert_eq!(
            events.iter().map(|e| e.kind.as_str()).collect::<Vec<_>>(),
            [
                "run_started",
                "execution_run_finished",
                "subagent_spawned",
                "subagent_finished"
            ]
        );
        assert_eq!(events[1].payload["provider_id"], json!("local"));
        assert_eq!(events[2].payload["task_id"], json!("t1"));
        assert_eq!(events[3].payload["status"], json!("completed"));
    }

    #[test]
    fn child_chat_outbound_is_not_written_to_parent_run() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = workspace(temp.path());
        let mut sink = start_sink(temp.path());

        sink.observe_bus_message(&BusMessage::Outbound(outbound_from(json!({
            "channel": "altai-cli",
            "chat_id": "child-subagent",
            "content": "subagent says hi",
            "metadata": {},
        }))));
        sink.observe_bus_message(&BusMessage::Telemetry(TelemetryEvent::AgentThought {
            chat_id: "child-subagent".into(),
            thought: "child thought".into(),
            background_job_id: None,
        }));
        // Parent-scoped subagent lifecycle still journals.
        sink.observe_bus_message(&BusMessage::Telemetry(TelemetryEvent::SubagentSpawned {
            parent_chat_id: "chat-1".into(),
            child_chat_id: "child-subagent".into(),
            task_id: "t-child".into(),
            display_name: None,
            agent_name: None,
            background_job_id: None,
        }));

        let journal = EventJournal::open(workspace.agent_event_journal_db()).expect("reopen");
        let events = journal.fetch_after("run-1", 0, 10).expect("fetch");
        assert_eq!(
            events.iter().map(|e| e.kind.as_str()).collect::<Vec<_>>(),
            ["run_started", "subagent_spawned"]
        );
    }

    #[test]
    fn lifecycle_warning_and_clear_are_journaled() {
        use isanagent::bus::{RunBudgetSnapshot, RunBudgetWarning, RunBudgetWarningReason};

        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = workspace(temp.path());
        let mut sink = start_sink(temp.path());

        sink.observe_bus_message(&BusMessage::RunLifecycle(RunLifecycleEvent::Warning {
            run_id: "run-1".into(),
            chat_id: "chat-1".into(),
            warning: RunBudgetWarning {
                reason: RunBudgetWarningReason::NoProgress { turns: 6 },
                budget: RunBudgetSnapshot::default(),
            },
        }));
        sink.observe_bus_message(&BusMessage::RunLifecycle(
            RunLifecycleEvent::WarningCleared {
                run_id: "run-1".into(),
                chat_id: "chat-1".into(),
            },
        ));
        // Foreign run warnings must not land on this journal.
        sink.observe_bus_message(&BusMessage::RunLifecycle(RunLifecycleEvent::Warning {
            run_id: "other-run".into(),
            chat_id: "chat-1".into(),
            warning: RunBudgetWarning {
                reason: RunBudgetWarningReason::NoProgress { turns: 9 },
                budget: RunBudgetSnapshot::default(),
            },
        }));

        let journal = EventJournal::open(workspace.agent_event_journal_db()).expect("reopen");
        let events = journal.fetch_after("run-1", 0, 10).expect("fetch");
        assert_eq!(
            events.iter().map(|e| e.kind.as_str()).collect::<Vec<_>>(),
            ["run_started", "run_warning", "run_warning_cleared"]
        );
        assert_eq!(events[1].payload["warning"]["reason"]["kind"], "no_progress");
        assert_eq!(events[1].payload["warning"]["reason"]["turns"], 6);
    }
}
