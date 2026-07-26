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

use std::collections::{HashMap, HashSet, VecDeque};

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
    finished: HashSet<String>,
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
            finished: HashSet::new(),
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
        if self.finished.contains(attempt_id) {
            return Err(RunnerError::Finished {
                attempt_id: attempt_id.to_string(),
            });
        }
        if let Some(kind) = map_event(event) {
            let payload = serde_json::to_value(event)
                .map_err(|error| RunnerError::Other(format!("cannot normalize event: {error}")))?;
            let seq = self.next_seq(attempt_id);
            let terminal = matches!(
                kind,
                RunnerEventKind::Completed
                    | RunnerEventKind::Failed
                    | RunnerEventKind::Cancelled
                    | RunnerEventKind::Stalled
            );
            self.inbox
                .get_mut(attempt_id)
                .expect("checked above")
                .push_back(RunnerEvent {
                    attempt_id: attempt_id.to_string(),
                    kind,
                    seq,
                    payload,
                });
            if terminal {
                self.finished.insert(attempt_id.to_string());
            }
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
        if self.finished.contains(&spec.attempt_id) {
            return Err(RunnerError::Finished {
                attempt_id: spec.attempt_id.clone(),
            });
        }
        // Starting the same active identity is idempotent and must not discard
        // events already delivered by the runtime bus.
        self.inbox.entry(spec.attempt_id.clone()).or_default();
        Ok(AttemptIdentity {
            attempt_id: spec.attempt_id.clone(),
            handle: spec.attempt_id.clone(),
        })
    }

    fn poll_event(&mut self, identity: &AttemptIdentity) -> RunnerResult<Option<RunnerEvent>> {
        validate_identity(identity)?;
        let Some(queue) = self.inbox.get_mut(&identity.attempt_id) else {
            return Err(RunnerError::UnknownAttempt {
                attempt_id: identity.attempt_id.clone(),
            });
        };
        Ok(queue.pop_front())
    }

    fn steer(&mut self, identity: &AttemptIdentity, message: &str) -> RunnerResult<()> {
        validate_identity(identity)?;
        if !self.inbox.contains_key(&identity.attempt_id) {
            return Err(RunnerError::UnknownAttempt {
                attempt_id: identity.attempt_id.clone(),
            });
        }
        if self.finished.contains(&identity.attempt_id) {
            return Err(RunnerError::Finished {
                attempt_id: identity.attempt_id.clone(),
            });
        }
        // O6b routes this to `route_steer` on the owning runtime only.
        self.steer_log
            .push((identity.attempt_id.clone(), message.to_string()));
        Ok(())
    }

    fn cancel(&mut self, identity: &AttemptIdentity) -> RunnerResult<()> {
        validate_identity(identity)?;
        if !self.inbox.contains_key(&identity.attempt_id) {
            return Err(RunnerError::UnknownAttempt {
                attempt_id: identity.attempt_id.clone(),
            });
        }
        if self.finished.contains(&identity.attempt_id) {
            return Err(RunnerError::Finished {
                attempt_id: identity.attempt_id.clone(),
            });
        }
        // O6b routes this to `route_cancel` on the owning runtime only.
        self.cancel_log.push(identity.attempt_id.clone());
        Ok(())
    }

    fn shutdown(&mut self) {
        self.inbox.clear();
        self.seq.clear();
        self.finished.clear();
        self.steer_log.clear();
        self.cancel_log.clear();
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
        Event::Clarification {
            edit_diff: Some(_), ..
        } => Some(RunnerEventKind::ApprovalRequired),
        Event::Clarification {
            edit_diff: None, ..
        } => Some(RunnerEventKind::InputRequired),
        Event::ApprovalRequest { .. } => Some(RunnerEventKind::ApprovalRequired),
        // These events describe tools/jobs within an agent run, not the
        // orchestration attempt itself. Only RunTerminated is authoritative
        // for the attempt's terminal state.
        Event::ExecutionRunFinished { .. } | Event::ExecutionJobFinished { .. } => {
            Some(RunnerEventKind::Output)
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
    let discriminator = outcome
        .get("kind")
        .or_else(|| outcome.get("status"))
        .and_then(Value::as_str);
    match discriminator {
        Some("completed" | "success") => RunnerEventKind::Completed,
        Some("cancelled") => RunnerEventKind::Cancelled,
        Some("stuck") => RunnerEventKind::Stalled,
        Some("failed" | "error" | "budget_exhausted") => RunnerEventKind::Failed,
        // The runtime outcome is a tagged enum. Unknown/malformed outcomes
        // fail closed instead of silently reporting successful completion.
        _ => RunnerEventKind::Failed,
    }
}

fn validate_identity(identity: &AttemptIdentity) -> RunnerResult<()> {
    if identity.handle != identity.attempt_id {
        return Err(RunnerError::UnknownAttempt {
            attempt_id: identity.attempt_id.clone(),
        });
    }
    Ok(())
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
                outcome: serde_json::json!({ "kind": "completed" }),
            }),
            Some(RunnerEventKind::Completed)
        );
        assert_eq!(
            map_event(&Event::RunTerminated {
                run_id: "r1".into(),
                outcome: serde_json::json!({ "kind": "failed", "failure": "boom" }),
            }),
            Some(RunnerEventKind::Failed)
        );
        assert_eq!(
            map_event(&Event::RunTerminated {
                run_id: "r1".into(),
                outcome: serde_json::json!({ "kind": "cancelled" }),
            }),
            Some(RunnerEventKind::Cancelled)
        );
        assert_eq!(
            map_event(&Event::RunTerminated {
                run_id: "r1".into(),
                outcome: serde_json::json!({ "kind": "stuck" }),
            }),
            Some(RunnerEventKind::Stalled)
        );
        assert_eq!(
            map_event(&Event::RunTerminated {
                run_id: "r1".into(),
                outcome: serde_json::json!({ "kind": "budget_exhausted" }),
            }),
            Some(RunnerEventKind::Failed)
        );
        assert_eq!(
            map_event(&Event::RunTerminated {
                run_id: "r1".into(),
                outcome: Value::Null,
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
            map_event(&Event::Clarification {
                content: "apply edit?".into(),
                choices: vec![],
                edit_diff: Some(crate::altai::agent::runtime::EditDiffPayload {
                    file: "src/lib.rs".into(),
                    diff: "@@".into(),
                    truncated: false,
                }),
            }),
            Some(RunnerEventKind::ApprovalRequired)
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
    fn execution_tools_are_output_not_attempt_terminals() {
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
            Some(RunnerEventKind::Output)
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
            Some(RunnerEventKind::Output)
        );
        assert_eq!(
            map_event(&Event::ExecutionJobFinished {
                job_id: "j".into(),
                session_id: "s".into(),
                provider_id: "p".into(),
                status: "failed".into(),
                exit_code: Some(2),
                duration_ms: 10,
                stdout_len: 0,
                stderr_len: 10,
                artifact_count: 0,
                description: None,
            }),
            Some(RunnerEventKind::Output)
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
        let identity = AttemptIdentity {
            attempt_id: "att-1".into(),
            handle: "att-1".into(),
        };
        let started = a.poll_event(&identity).expect("poll").expect("started");
        assert_eq!(started.kind, RunnerEventKind::Started);
        assert_eq!(started.payload["type"], "run_started");
        let output = a.poll_event(&identity).expect("poll").expect("output");
        assert_eq!(output.kind, RunnerEventKind::Output);
        assert_eq!(output.payload["content"], "working");
        assert_eq!(output.payload["role"], "assistant");
        assert_eq!(poll_kind(&mut a, "att-1"), None);
    }

    #[test]
    fn duplicate_start_preserves_queued_events_and_sequence() {
        let mut a = adapter_with_attempt("att-1");
        a.feed(
            "att-1",
            &Event::AgentMessage {
                content: "queued".into(),
                role: "assistant".into(),
            },
        )
        .expect("feed");
        a.start_attempt(&AttemptSpec {
            task_id: "t1".into(),
            attempt_id: "att-1".into(),
            input: String::new(),
        })
        .expect("idempotent start");

        let identity = AttemptIdentity {
            attempt_id: "att-1".into(),
            handle: "att-1".into(),
        };
        let event = a.poll_event(&identity).unwrap().unwrap();
        assert_eq!(event.seq, 1);
        assert_eq!(event.payload["content"], "queued");
    }

    #[test]
    fn terminal_event_closes_the_attempt_after_it_is_queued() {
        let mut a = adapter_with_attempt("att-1");
        a.feed(
            "att-1",
            &Event::RunTerminated {
                run_id: "r1".into(),
                outcome: serde_json::json!({ "kind": "completed" }),
            },
        )
        .expect("terminal");
        assert!(matches!(
            a.feed(
                "att-1",
                &Event::AgentMessage {
                    content: "late".into(),
                    role: "assistant".into(),
                }
            ),
            Err(RunnerError::Finished { .. })
        ));
        let identity = AttemptIdentity {
            attempt_id: "att-1".into(),
            handle: "att-1".into(),
        };
        assert!(matches!(
            a.cancel(&identity),
            Err(RunnerError::Finished { .. })
        ));
        assert_eq!(poll_kind(&mut a, "att-1"), Some(RunnerEventKind::Completed));
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
        let mismatched_handle = AttemptIdentity {
            attempt_id: "att-1".into(),
            handle: "att-2".into(),
        };
        assert!(matches!(
            a.poll_event(&mismatched_handle),
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
