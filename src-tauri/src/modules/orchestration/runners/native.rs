//! O6a — Native runner adapter: event normalization and identity.
//!
//! Wraps the existing ALTAI runtime behind the O3 [`RunnerAdapter`] boundary.
//! This slice delivers the **normalization** of native `runtime::Event`s into
//! orchestration [`RunnerEventKind`]s and the per-attempt identity/isolation
//! model, with conformance tests (§B1). The live dispatch bridge (starting a
//! real run and feeding bus events) lands in O6b.
//!
//! Design: the adapter owns an event inbox per attempt. A future bus listener
//! calls [`NativeRunnerAdapter::feed`] with a runtime event; the adapter
//! normalizes it (the single translation point) and queues it. `poll_event`
//! drains the inbox. This keeps the runtime's async/bus model out of the
//! synchronous coordinator decision core while preventing feature loss.

use std::collections::{HashMap, VecDeque};

use serde_json::Value;

use super::{
    AttemptIdentity, AttemptSpec, RunnerAdapter, RunnerCapabilities, RunnerError, RunnerEvent,
    RunnerEventKind, RunnerResult,
};
use crate::altai::agent::runtime::Event;

/// Normalized event sequence number per attempt.
type Seq = u64;

/// Adapter around the native ALTAI runtime.
///
/// `feed` is the integration seam for O6b: the bus listener pushes observed
/// `runtime::Event`s here and the adapter normalizes + queues them.
pub struct NativeRunnerAdapter {
    inbox: HashMap<String, VecDeque<RunnerEvent>>,
    seq: HashMap<String, Seq>,
    steer_log: Vec<(String, String)>,
    cancel_log: Vec<String>,
}

impl Default for NativeRunnerAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeRunnerAdapter {
    pub fn new() -> Self {
        Self {
            inbox: HashMap::new(),
            seq: HashMap::new(),
            steer_log: Vec::new(),
            cancel_log: Vec::new(),
        }
    }

    /// Observe a native runtime event for an attempt and queue its normalized
    /// form. Events for an unknown attempt are rejected (a runner cannot emit
    /// for another task/workspace — §B1).
    pub fn feed(&mut self, attempt_id: &str, event: &Event) -> RunnerResult<()> {
        if !self.inbox.contains_key(attempt_id) {
            return Err(RunnerError::UnknownAttempt {
                attempt_id: attempt_id.to_string(),
            });
        }
        if let Some(kind) = map_event(event) {
            let seq = self.next_seq(attempt_id);
            self.inbox
                .get_mut(attempt_id)
                .expect("checked above")
                .push_back(RunnerEvent {
                    attempt_id: attempt_id.to_string(),
                    kind,
                    seq,
                    payload: Value::Null,
                });
        }
        Ok(())
    }

    pub fn steers(&self) -> &[(String, String)] {
        &self.steer_log
    }

    pub fn cancels(&self) -> &[String] {
        &self.cancel_log
    }

    fn next_seq(&mut self, attempt_id: &str) -> Seq {
        let entry = self.seq.entry(attempt_id.to_string()).or_insert(0);
        *entry += 1;
        *entry
    }
}

impl RunnerAdapter for NativeRunnerAdapter {
    fn capabilities(&self) -> RunnerCapabilities {
        RunnerCapabilities {
            can_steer: true,
            can_cancel: true,
            can_resume: true,
        }
    }

    fn start_attempt(&mut self, spec: &AttemptSpec) -> RunnerResult<AttemptIdentity> {
        // O6b will route this to the live runtime (ensure_instance). Here we
        // establish the immutable identity and an empty inbox.
        self.inbox
            .entry(spec.attempt_id.clone())
            .or_default()
            .clear();
        Ok(AttemptIdentity {
            attempt_id: spec.attempt_id.clone(),
            handle: spec.attempt_id.clone(),
        })
    }

    fn poll_event(&mut self, identity: &AttemptIdentity) -> RunnerResult<Option<RunnerEvent>> {
        let Some(queue) = self.inbox.get_mut(&identity.attempt_id) else {
            return Err(RunnerError::UnknownAttempt {
                attempt_id: identity.attempt_id.clone(),
            });
        };
        Ok(queue.pop_front())
    }

    fn steer(&mut self, identity: &AttemptIdentity, message: &str) -> RunnerResult<()> {
        if !self.inbox.contains_key(&identity.attempt_id) {
            return Err(RunnerError::UnknownAttempt {
                attempt_id: identity.attempt_id.clone(),
            });
        }
        // O6b routes this to `route_steer` on the owning runtime only.
        self.steer_log
            .push((identity.attempt_id.clone(), message.to_string()));
        Ok(())
    }

    fn cancel(&mut self, identity: &AttemptIdentity) -> RunnerResult<()> {
        if !self.inbox.contains_key(&identity.attempt_id) {
            return Err(RunnerError::UnknownAttempt {
                attempt_id: identity.attempt_id.clone(),
            });
        }
        // O6b routes this to `route_cancel` on the owning runtime only.
        self.cancel_log.push(identity.attempt_id.clone());
        Ok(())
    }

    fn shutdown(&mut self) {
        self.inbox.clear();
    }
}

/// Translate a native `runtime::Event` into the orchestration
/// [`RunnerEventKind`]. Returns `None` for events that do not advance attempt
/// state (warnings, notifications, background jobs, subagent bookkeeping).
///
/// This is the single place native events become domain triggers; keeping it
/// exhaustive prevents feature loss (§B1).
pub fn map_event(event: &Event) -> Option<RunnerEventKind> {
    match event {
        Event::RunStarted { .. } => Some(RunnerEventKind::Started),
        Event::RunTerminated { outcome, .. } => Some(terminal_kind(outcome)),
        Event::AgentMessage { .. } | Event::Thinking { .. } => Some(RunnerEventKind::Output),
        Event::ToolCallStart { .. } | Event::ToolCallEnd { .. } => Some(RunnerEventKind::Output),
        Event::EditDiff { .. } => Some(RunnerEventKind::Output),
        Event::Usage { .. } => Some(RunnerEventKind::Output),
        Event::Clarification { .. } => Some(RunnerEventKind::InputRequired),
        Event::ApprovalRequest { .. } => Some(RunnerEventKind::ApprovalRequired),
        Event::ExecutionRunFinished { exit_code, .. } => {
            let code = exit_code.as_ref().copied().unwrap_or(0);
            Some(if code == 0 {
                RunnerEventKind::Completed
            } else {
                RunnerEventKind::Failed
            })
        }
        Event::ExecutionJobFinished {
            status, exit_code, ..
        } => {
            let code = exit_code.as_ref().copied().unwrap_or(0);
            Some(execution_job_kind(status, code))
        }
        // Events with no orchestration meaning.
        Event::RunWarning { .. }
        | Event::RunWarningCleared { .. }
        | Event::BackgroundJobUpdated { .. }
        | Event::NotificationCreated { .. }
        | Event::NotificationUpdated { .. }
        | Event::SubagentSpawned { .. }
        | Event::SubagentFinished { .. }
        | Event::NotebookOutput { .. }
        | Event::ExperimentResult { .. } => None,
    }
}

fn terminal_kind(outcome: &Value) -> RunnerEventKind {
    if is_failure_outcome(outcome) {
        RunnerEventKind::Failed
    } else {
        RunnerEventKind::Completed
    }
}

fn execution_job_kind(status: &str, exit_code: i32) -> RunnerEventKind {
    match status {
        "completed" => RunnerEventKind::Completed,
        "cancelled" => RunnerEventKind::Cancelled,
        _ if exit_code != 0 => RunnerEventKind::Failed,
        _ => RunnerEventKind::Completed,
    }
}

/// Heuristic failure detection for an opaque serialized run outcome. The native
/// outcome schema is provider-dependent; this catches the common signals
/// (an `error` field or an explicit error/failed status) and defaults to
/// success. O6b refines this once the adapter owns the typed outcome.
fn is_failure_outcome(outcome: &Value) -> bool {
    let Some(obj) = outcome.as_object() else {
        return false;
    };
    if obj.get("error").map(|v| !v.is_null()).unwrap_or(false) {
        return true;
    }
    matches!(
        obj.get("status").and_then(|v| v.as_str()),
        Some("error") | Some("failed")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::altai::agent::runtime::Event;
    use crate::modules::orchestration::runners::AttemptSpec;

    fn adapter_with_attempt(id: &str) -> NativeRunnerAdapter {
        let mut a = NativeRunnerAdapter::new();
        let identity = a
            .start_attempt(&AttemptSpec {
                task_id: "t1".into(),
                attempt_id: id.into(),
                input: "".into(),
            })
            .expect("start");
        assert_eq!(identity.attempt_id, id);
        a
    }

    fn poll_kind(a: &mut NativeRunnerAdapter, id: &str) -> Option<RunnerEventKind> {
        a.poll_event(&AttemptIdentity {
            attempt_id: id.into(),
            handle: id.into(),
        })
        .expect("poll")
        .map(|e| e.kind)
    }

    // --- Normalization conformance (§B1) ------------------------------------

    #[test]
    fn lifecycle_maps_to_started_and_terminal() {
        assert_eq!(
            map_event(&Event::RunStarted {
                run_id: "r1".into()
            }),
            Some(RunnerEventKind::Started)
        );
        assert_eq!(
            map_event(&Event::RunTerminated {
                run_id: "r1".into(),
                outcome: serde_json::json!({ "ok": true }),
            }),
            Some(RunnerEventKind::Completed)
        );
        assert_eq!(
            map_event(&Event::RunTerminated {
                run_id: "r1".into(),
                outcome: serde_json::json!({ "error": "boom" }),
            }),
            Some(RunnerEventKind::Failed)
        );
    }

    #[test]
    fn progress_events_map_to_output_without_state_change() {
        assert_eq!(
            map_event(&Event::AgentMessage {
                content: "hi".into(),
                role: "assistant".into(),
            }),
            Some(RunnerEventKind::Output)
        );
        assert_eq!(
            map_event(&Event::Thinking {
                content: "...".into(),
            }),
            Some(RunnerEventKind::Output)
        );
        assert_eq!(
            map_event(&Event::ToolCallStart {
                id: "t".into(),
                name: "ls".into(),
                input: serde_json::Value::Null,
            }),
            Some(RunnerEventKind::Output)
        );
    }

    #[test]
    fn interaction_events_map_to_input_and_approval() {
        assert_eq!(
            map_event(&Event::Clarification {
                content: "which?".into(),
                choices: vec![],
                edit_diff: None,
            }),
            Some(RunnerEventKind::InputRequired)
        );
        assert_eq!(
            map_event(&Event::ApprovalRequest {
                id: "a1".into(),
                action: "run".into(),
                payload: serde_json::Value::Null,
            }),
            Some(RunnerEventKind::ApprovalRequired)
        );
    }

    #[test]
    fn execution_runs_map_terminal_by_exit_code() {
        assert_eq!(
            map_event(&Event::ExecutionRunFinished {
                provider_id: "p".into(),
                session_id: "s".into(),
                exit_code: Some(0),
                duration_ms: 10,
                stdout_len: 0,
                stderr_len: 0,
                artifact_count: 0,
                git_head: None,
                description: None,
            }),
            Some(RunnerEventKind::Completed)
        );
        assert_eq!(
            map_event(&Event::ExecutionRunFinished {
                exit_code: Some(2),
                provider_id: "p".into(),
                session_id: "s".into(),
                duration_ms: 10,
                stdout_len: 0,
                stderr_len: 0,
                artifact_count: 0,
                git_head: None,
                description: None,
            }),
            Some(RunnerEventKind::Failed)
        );
    }

    #[test]
    fn non_orchestration_events_are_dropped() {
        assert_eq!(
            map_event(&Event::RunWarning {
                run_id: "r1".into(),
                warning: serde_json::Value::Null,
            }),
            None
        );
        assert_eq!(
            map_event(&Event::NotificationCreated {
                notification_id: "n1".into(),
                kind: "x".into(),
                title: "t".into(),
            }),
            None
        );
    }

    #[test]
    fn edit_diff_maps_to_output() {
        assert_eq!(
            map_event(&Event::EditDiff {
                file: "f".into(),
                before: "a".into(),
                after: "b".into(),
                hunk_id: "h".into(),
            }),
            Some(RunnerEventKind::Output)
        );
    }

    // --- Identity / isolation (§B1) -----------------------------------------

    #[test]
    fn feed_drains_normalized_events_in_order() {
        let mut a = adapter_with_attempt("att-1");
        a.feed(
            "att-1",
            &Event::RunStarted {
                run_id: "r1".into(),
            },
        )
        .expect("feed");
        a.feed(
            "att-1",
            &Event::AgentMessage {
                content: "working".into(),
                role: "assistant".into(),
            },
        )
        .expect("feed");
        assert_eq!(poll_kind(&mut a, "att-1"), Some(RunnerEventKind::Started));
        assert_eq!(poll_kind(&mut a, "att-1"), Some(RunnerEventKind::Output));
        assert_eq!(poll_kind(&mut a, "att-1"), None);
    }

    #[test]
    fn runner_cannot_emit_for_another_attempt() {
        let mut a = adapter_with_attempt("att-1");
        // att-2 was never started by this runner.
        assert!(matches!(
            a.feed(
                "att-2",
                &Event::RunStarted {
                    run_id: "r2".into()
                }
            ),
            Err(RunnerError::UnknownAttempt { .. })
        ));
        // att-1 inbox is unaffected.
        assert_eq!(poll_kind(&mut a, "att-1"), None);
    }

    #[test]
    fn steer_and_cancel_are_routed_only_to_known_attempts() {
        let mut a = adapter_with_attempt("att-1");
        let id = AttemptIdentity {
            attempt_id: "att-1".into(),
            handle: "att-1".into(),
        };
        a.steer(&id, "focus").expect("steer");
        a.cancel(&id).expect("cancel");
        assert_eq!(a.steers(), &[("att-1".into(), "focus".into())]);
        assert_eq!(a.cancels(), &["att-1".to_string()]);

        let ghost = AttemptIdentity {
            attempt_id: "ghost".into(),
            handle: "ghost".into(),
        };
        assert!(matches!(
            a.steer(&ghost, "x"),
            Err(RunnerError::UnknownAttempt { .. })
        ));
        assert!(matches!(
            a.cancel(&ghost),
            Err(RunnerError::UnknownAttempt { .. })
        ));
    }

    #[test]
    fn seq_is_monotonic_per_attempt() {
        let mut a = adapter_with_attempt("att-1");
        for _ in 0..3 {
            a.feed(
                "att-1",
                &Event::AgentMessage {
                    content: ".".into(),
                    role: "assistant".into(),
                },
            )
            .expect("feed");
        }
        let id = AttemptIdentity {
            attempt_id: "att-1".into(),
            handle: "att-1".into(),
        };
        let e1 = a.poll_event(&id).expect("poll").expect("event");
        let e2 = a.poll_event(&id).expect("poll").expect("event");
        let e3 = a.poll_event(&id).expect("poll").expect("event");
        assert!((e1.seq, e2.seq, e3.seq) == (1, 2, 3));
    }
}
