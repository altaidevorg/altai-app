//! Output contracts for `altai run`.

use altai_core::EventEnvelope;
use isanagent::bus::{BusMessage, RunLifecycleEvent, RunOutcome, TelemetryEvent};
use isanagent::host::{OneshotOutcome, OneshotResult};
use serde::Serialize;
use serde_json::json;
use std::io::{self, Write};
use std::time::{SystemTime, UNIX_EPOCH};

/// Stable process exit codes from the ALTAI CLI contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RunExitCode {
    Success = 0,
    Failed = 1,
    Validation = 2,
    Config = 3,
    ApprovalRequired = 4,
    Workspace = 5,
    Provider = 6,
    Cancelled = 7,
    Timeout = 8,
    Internal = 10,
}

impl From<RunExitCode> for u8 {
    fn from(value: RunExitCode) -> Self {
        value as u8
    }
}

impl RunExitCode {
    pub fn from_oneshot_outcome(outcome: &OneshotOutcome) -> Self {
        match outcome {
            OneshotOutcome::Completed => Self::Success,
            OneshotOutcome::Cancelled => Self::Cancelled,
            OneshotOutcome::TimedOut => Self::Timeout,
            OneshotOutcome::ApprovalRequired { .. }
            | OneshotOutcome::ClarificationRequired { .. } => Self::ApprovalRequired,
            OneshotOutcome::Failed(message) => {
                let lower = message.to_lowercase();
                if lower.contains("provider") || lower.contains("api") || lower.contains("network")
                {
                    Self::Provider
                } else {
                    Self::Failed
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct FinalRunResult {
    pub ok: bool,
    pub exit_code: u8,
    pub workspace: String,
    pub chat_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    pub outcome: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl FinalRunResult {
    pub fn from_oneshot(workspace: &str, result: &OneshotResult) -> Self {
        let exit_code = RunExitCode::from_oneshot_outcome(&result.outcome);
        let (outcome, detail) = match &result.outcome {
            OneshotOutcome::Completed => ("completed".to_string(), None),
            OneshotOutcome::Cancelled => ("cancelled".to_string(), None),
            OneshotOutcome::TimedOut => ("timeout".to_string(), None),
            OneshotOutcome::Failed(message) => ("failed".to_string(), Some(message.clone())),
            OneshotOutcome::ApprovalRequired { detail } => {
                ("approval_required".to_string(), Some(detail.clone()))
            }
            OneshotOutcome::ClarificationRequired { detail } => {
                ("clarification_required".to_string(), Some(detail.clone()))
            }
        };
        Self {
            ok: matches!(exit_code, RunExitCode::Success),
            exit_code: exit_code.into(),
            workspace: workspace.to_string(),
            chat_id: result.chat_id.clone(),
            run_id: result.run_id.clone(),
            outcome,
            final_text: result.final_text.clone(),
            detail,
        }
    }
}

#[derive(Debug)]
pub struct JsonlEmitter {
    workspace: String,
    sequence: u64,
    chat_id: Option<String>,
    run_id: Option<String>,
}

impl JsonlEmitter {
    pub fn new(workspace: impl Into<String>) -> Self {
        Self {
            workspace: workspace.into(),
            sequence: 0,
            chat_id: None,
            run_id: None,
        }
    }

    fn next_sequence(&mut self) -> u64 {
        self.sequence += 1;
        self.sequence
    }

    fn timestamp_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis() as u64)
            .unwrap_or(0)
    }

    pub fn emit(
        &mut self,
        event_type: &str,
        data: serde_json::Value,
        out: &mut dyn Write,
    ) -> io::Result<()> {
        let mut envelope = EventEnvelope::new(
            self.next_sequence(),
            Self::timestamp_ms(),
            self.workspace.clone(),
            event_type,
            data,
        );
        if let Some(chat_id) = &self.chat_id {
            envelope = envelope.with_chat_id(chat_id.clone());
        }
        if let Some(run_id) = &self.run_id {
            envelope = envelope.with_run_id(run_id.clone());
        }
        writeln!(out, "{}", serde_json::to_string(&envelope).map_err(io::Error::other)?)
    }

    pub fn observe_bus_message(
        &mut self,
        message: &BusMessage,
        out: &mut dyn Write,
    ) -> io::Result<()> {
        match message {
            BusMessage::RunLifecycle(RunLifecycleEvent::Started { run_id, chat_id }) => {
                self.chat_id = Some(chat_id.clone());
                self.run_id = Some(run_id.clone());
                self.emit(
                    "run_started",
                    json!({ "run_id": run_id, "chat_id": chat_id }),
                    out,
                )
            }
            BusMessage::RunLifecycle(RunLifecycleEvent::Terminated {
                run_id,
                chat_id,
                outcome,
            }) => {
                self.chat_id = Some(chat_id.clone());
                self.run_id = Some(run_id.clone());
                self.emit(
                    "run_finished",
                    json!({
                        "run_id": run_id,
                        "chat_id": chat_id,
                        "outcome": run_outcome_label(outcome),
                    }),
                    out,
                )
            }
            BusMessage::RunLifecycle(RunLifecycleEvent::Warning {
                run_id,
                chat_id,
                warning,
            }) => {
                self.chat_id = Some(chat_id.clone());
                self.run_id = Some(run_id.clone());
                self.emit(
                    "run_warning",
                    json!({
                        "run_id": run_id,
                        "chat_id": chat_id,
                        "warning": format!("{warning:?}"),
                    }),
                    out,
                )
            }
            BusMessage::Outbound(outbound) => self.emit(
                "assistant_message",
                json!({ "content": outbound.content }),
                out,
            ),
            BusMessage::Telemetry(TelemetryEvent::AgentThought { thought, .. }) => {
                self.emit("thinking", json!({ "content": thought }), out)
            }
            BusMessage::Telemetry(TelemetryEvent::ToolCallStarted {
                tool_name,
                args,
                tool_call_id,
                ..
            }) => self.emit(
                "tool_call_started",
                json!({
                    "tool": tool_name,
                    "args": args,
                    "tool_call_id": tool_call_id,
                }),
                out,
            ),
            BusMessage::Telemetry(TelemetryEvent::ToolCallFinished {
                tool_name,
                result,
                is_error,
                tool_call_id,
                ..
            }) => self.emit(
                "tool_call_finished",
                json!({
                    "tool": tool_name,
                    "result": result,
                    "is_error": is_error,
                    "tool_call_id": tool_call_id,
                }),
                out,
            ),
            BusMessage::Telemetry(TelemetryEvent::ToolProgress {
                tool_name,
                message,
                tool_call_id,
                ..
            }) => self.emit(
                "tool_call_progress",
                json!({
                    "tool": tool_name,
                    "message": message,
                    "tool_call_id": tool_call_id,
                }),
                out,
            ),
            BusMessage::Telemetry(TelemetryEvent::ShellPolicyDecision {
                decision,
                command_preview,
                mode,
                ..
            }) if decision == "approval_requested" => self.emit(
                "approval_requested",
                json!({
                    "mode": mode,
                    "command_preview": command_preview,
                }),
                out,
            ),
            BusMessage::Telemetry(TelemetryEvent::AgentUsage {
                model,
                prompt_tokens,
                completion_tokens,
                total_tokens,
                ..
            }) => self.emit(
                "usage",
                json!({
                    "model": model,
                    "prompt_tokens": prompt_tokens,
                    "completion_tokens": completion_tokens,
                    "total_tokens": total_tokens,
                }),
                out,
            ),
            _ => Ok(()),
        }
    }

    #[allow(dead_code)]
    pub fn emit_error(&mut self, message: &str, out: &mut dyn Write) -> io::Result<()> {
        self.emit("error", json!({ "message": message }), out)
    }

    pub fn emit_final_result(
        &mut self,
        result: &FinalRunResult,
        out: &mut dyn Write,
    ) -> io::Result<()> {
        self.chat_id = Some(result.chat_id.clone());
        self.run_id = result.run_id.clone();
        self.emit("run_finished", serde_json::to_value(result).unwrap_or(json!({})), out)
    }
}

fn run_outcome_label(outcome: &RunOutcome) -> String {
    match outcome {
        RunOutcome::Completed => "completed".to_string(),
        RunOutcome::Cancelled => "cancelled".to_string(),
        RunOutcome::Failed { failure, .. } => format!("failed:{failure:?}"),
        RunOutcome::Stuck { reason } => format!("stuck:{reason:?}"),
        RunOutcome::BudgetExhausted { .. } => "budget_exhausted".to_string(),
    }
}

pub fn render_pretty(result: &FinalRunResult, out: &mut dyn Write) -> io::Result<()> {
    match (&result.final_text, &result.detail) {
        (Some(text), _) if !text.is_empty() => writeln!(out, "{text}"),
        (_, Some(detail)) => writeln!(out, "altai run {}: {detail}", result.outcome),
        _ => writeln!(out, "altai run {}", result.outcome),
    }
}

pub fn parse_timeout(raw: &str) -> Result<std::time::Duration, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err("timeout must not be empty".into());
    }
    if let Ok(seconds) = raw.parse::<u64>() {
        return Ok(std::time::Duration::from_secs(seconds));
    }
    let (digits, unit) = raw.split_at(
        raw.find(|c: char| !c.is_ascii_digit())
            .ok_or_else(|| format!("invalid timeout: {raw}"))?,
    );
    let amount: u64 = digits
        .parse()
        .map_err(|_| format!("invalid timeout: {raw}"))?;
    let multiplier = match unit {
        "s" | "sec" | "secs" | "second" | "seconds" => 1,
        "m" | "min" | "mins" | "minute" | "minutes" => 60,
        "h" | "hr" | "hour" | "hours" => 3600,
        _ => return Err(format!("unsupported timeout unit in {raw}")),
    };
    Ok(std::time::Duration::from_secs(
        amount.saturating_mul(multiplier),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeout_parser_accepts_common_suffixes() {
        assert_eq!(parse_timeout("30").unwrap(), std::time::Duration::from_secs(30));
        assert_eq!(parse_timeout("10m").unwrap(), std::time::Duration::from_secs(600));
        assert_eq!(parse_timeout("1h").unwrap(), std::time::Duration::from_secs(3600));
    }

    #[test]
    fn exit_codes_map_oneshot_outcomes() {
        assert_eq!(
            RunExitCode::from_oneshot_outcome(&OneshotOutcome::Completed),
            RunExitCode::Success
        );
        assert_eq!(
            RunExitCode::from_oneshot_outcome(&OneshotOutcome::Cancelled),
            RunExitCode::Cancelled
        );
        assert_eq!(
            RunExitCode::from_oneshot_outcome(&OneshotOutcome::TimedOut),
            RunExitCode::Timeout
        );
        assert_eq!(
            RunExitCode::from_oneshot_outcome(&OneshotOutcome::ApprovalRequired {
                detail: "rm -rf".into()
            }),
            RunExitCode::ApprovalRequired
        );
    }

    #[test]
    fn jsonl_emitter_writes_versioned_envelope() {
        let mut emitter = JsonlEmitter::new("/workspace");
        let mut buffer = Vec::new();
        emitter
            .emit("run_started", json!({ "model": "test" }), &mut buffer)
            .unwrap();
        let line = String::from_utf8(buffer).unwrap();
        let value: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(value["schema_version"], 1);
        assert_eq!(value["type"], "run_started");
        assert_eq!(value["sequence"], 1);
        assert_eq!(value["workspace"], "/workspace");
    }
}
