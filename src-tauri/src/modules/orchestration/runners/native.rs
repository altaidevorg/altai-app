//! O6a/O6b — Native runner adapter: event normalization, identity, and the
//! async/sync event bridge.
//!
//! Wraps the existing ALTAI runtime behind the O3 [`RunnerAdapter`] boundary.
//! O6a delivered normalization of native `runtime::Event`s into orchestration
//! [`RunnerEventKind`]s and the per-attempt identity/isolation model. O6b adds
//! the **concurrent event bridge**: the adapter's inbox lives behind an
//! `Arc<Mutex>`, so an async feeder (the future outbound-router tap) can push
//! events while the synchronous coordinator's `poll_event` drains them. This is
//! the mechanism that lets the runtime's async/bus model coexist with the
//! synchronous coordinator decision core without feature loss.
//!
//! Design: [`NativeRunnerAdapter`] owns the shared inbox. A bus listener obtains
//! a [`NativeFeeder`] (a clonable handle) via [`NativeRunnerAdapter::feeder`]
//! and calls [`NativeFeeder::push_event`] from an async task; the adapter's
//! [`RunnerAdapter::poll_event`] drains the same inbox synchronously.
//!
//! O6c adds the **dispatch bridge**: a [`NativeRunnerAdapter`] constructed via
//! [`NativeRunnerAdapter::with_dispatch`] routes `start_attempt`/`steer`/`cancel`
//! to an injected async [`NativeDispatch`] (which wraps the runtime's public
//! `route_send`/`route_steer`/`route_cancel`). The attempt's id doubles as the
//! native `chat_id`, so events observed for that chat feed straight back into the
//! adapter's inbox. This closes the sync→async→sync loop without touching the
//! runtime's core paths.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use futures_util::FutureExt;
use serde_json::Value;
use tokio::task::JoinHandle;

use super::{
    AttemptIdentity, AttemptSpec, RunnerAdapter, RunnerCapabilities, RunnerError, RunnerEvent,
    RunnerEventKind, RunnerResult,
};
use crate::altai::agent::runtime::Event;

/// Normalized event sequence number per attempt.
type Seq = u64;

/// Mutable state shared between the adapter and its feeders.
///
/// Held behind an `Arc<Mutex<…>>` so an async feeder task can push while the
/// synchronous adapter drains.
#[derive(Default)]
struct NativeState {
    inbox: HashMap<String, VecDeque<RunnerEvent>>,
    seq: HashMap<String, Seq>,
    finished: HashSet<String>,
    steer_log: Vec<(String, String)>,
    cancel_log: Vec<String>,
}

/// Adapter around the native ALTAI runtime.
///
/// The synchronous [`RunnerAdapter`] methods lock [`NativeState`]; the
/// [`NativeRunnerAdapter::feeder`] handle lets an async bus tap push events
/// concurrently. The single translation point for native events is
/// [`map_event`].
pub struct NativeRunnerAdapter {
    state: Arc<Mutex<NativeState>>,
    /// O6c dispatch bridge. `None` (via [`NativeRunnerAdapter::new`]) leaves the
    /// adapter in pure-observation mode (O6a/O6b); `Some` makes `start_attempt`
    /// actually launch a run and `steer`/`cancel` route to the runtime.
    bridge: Option<NativeBridge>,
}

/// Async dispatch surface the adapter drives when constructed with
/// [`NativeRunnerAdapter::with_dispatch`]. The production impl wraps
/// `AgentRuntime` (`route_send`/`route_steer`/`route_cancel`); tests use a fake.
struct NativeBridge {
    handle: tokio::runtime::Handle,
    dispatch: Arc<dyn NativeDispatch>,
    tasks: Vec<DispatchTask>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DispatchAction {
    Launch,
    Steer,
    Cancel,
}

impl DispatchAction {
    fn as_str(self) -> &'static str {
        match self {
            Self::Launch => "launch",
            Self::Steer => "steer",
            Self::Cancel => "cancel",
        }
    }
}

struct DispatchTask {
    attempt_id: String,
    action: DispatchAction,
    join: JoinHandle<Result<(), String>>,
}

struct DispatchFailure {
    attempt_id: String,
    action: DispatchAction,
    error: String,
}

impl NativeBridge {
    fn collect_failures(&mut self) -> Vec<DispatchFailure> {
        let mut pending = Vec::with_capacity(self.tasks.len());
        let mut failures = Vec::new();
        for mut task in self.tasks.drain(..) {
            if !task.join.is_finished() {
                pending.push(task);
                continue;
            }
            match (&mut task.join).now_or_never() {
                Some(Ok(Ok(()))) => {}
                Some(Ok(Err(error))) => failures.push(DispatchFailure {
                    attempt_id: task.attempt_id,
                    action: task.action,
                    error,
                }),
                Some(Err(error)) => failures.push(DispatchFailure {
                    attempt_id: task.attempt_id,
                    action: task.action,
                    error: format!("dispatch task did not complete: {error}"),
                }),
                // Preserve the handle defensively if its readiness changes
                // between the observation and the non-blocking poll.
                None => pending.push(task),
            }
        }
        self.tasks = pending;
        failures
    }

    fn abort_all(&mut self) {
        for task in self.tasks.drain(..) {
            task.join.abort();
        }
    }
}

impl Default for NativeRunnerAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeRunnerAdapter {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(NativeState::default())),
            bridge: None,
        }
    }

    /// Construct a live adapter whose `start_attempt`/`steer`/`cancel` route to
    /// `dispatch` via `handle` (spawned async tasks). Observed events still flow
    /// back through [`NativeRunnerAdapter::feeder`]; the dispatch impl is
    /// responsible for feeding them (a fake scripts them; the production impl
    /// relies on the `agent://event` listener tap).
    pub fn with_dispatch(
        dispatch: Arc<dyn NativeDispatch>,
        handle: tokio::runtime::Handle,
    ) -> Self {
        Self {
            state: Arc::new(Mutex::new(NativeState::default())),
            bridge: Some(NativeBridge {
                handle,
                dispatch,
                tasks: Vec::new(),
            }),
        }
    }

    /// Return a clonable handle through which an **async** task can push observed
    /// native events into this adapter's inbox. The feeder shares the same
    /// `Arc<Mutex<NativeState>>` as the adapter, so pushes become visible to the
    /// synchronous [`RunnerAdapter::poll_event`] without copying.
    pub fn feeder(&self) -> NativeFeeder {
        NativeFeeder {
            state: self.state.clone(),
        }
    }

    /// Observe a native runtime event for an attempt and queue its normalized
    /// form. Events for an unknown attempt are rejected (a runner cannot emit
    /// for another task/workspace — §B1).
    pub fn feed(&mut self, attempt_id: &str, event: &Event) -> RunnerResult<()> {
        let mut state = lock(&self.state)?;
        push_event_locked(&mut state, attempt_id, event)
    }

    pub fn steers(&self) -> Vec<(String, String)> {
        lock(&self.state)
            .map(|s| s.steer_log.clone())
            .unwrap_or_default()
    }

    pub fn cancels(&self) -> Vec<String> {
        lock(&self.state)
            .map(|s| s.cancel_log.clone())
            .unwrap_or_default()
    }
}

/// Clonable handle that lets an async task push native events into the owning
/// [`NativeRunnerAdapter`]'s inbox. Created via [`NativeRunnerAdapter::feeder`].
///
/// This is the O6b integration seam: the outbound-router tap (or, in tests, a
/// scripted async task) calls [`NativeFeeder::push_event`] from a tokio task
/// while the synchronous coordinator drains with `poll_event`. The shared
/// `Arc<Mutex<NativeState>>` is what bridges the async and sync worlds.
#[derive(Clone)]
pub struct NativeFeeder {
    state: Arc<Mutex<NativeState>>,
}

impl NativeFeeder {
    /// Push an observed native event for `attempt_id`. Thread-safe: safe to call
    /// from an async task while the adapter's synchronous `poll_event` drains.
    pub fn push_event(&self, attempt_id: &str, event: &Event) -> RunnerResult<()> {
        let mut state = lock(&self.state)?;
        push_event_locked(&mut state, attempt_id, event)
    }
}

/// Async dispatch surface the adapter drives (O6c). `attempt_id` doubles as the
/// native `chat_id`.
///
/// Production binding (needs a running app + provider to validate):
/// - `launch` → `runtime::route_send(...)` (runtime.rs:1813), `chat_id = attempt_id`.
/// - `steer`  → resolve active `run_id` via `RunCoordinator::active_run`, then
///   `runtime::route_steer(...)` (runtime.rs:2144).
/// - `cancel` → `runtime::route_cancel(...)` (runtime.rs:2116).
///
/// Observed events are fed into `feeder` by the `agent://event` listener tap
/// (the single emit chokepoint, runtime.rs:742): deserialize the envelope's
/// redacted `event` JSON back into an [`Event`] and call `feeder.push_event`.
#[async_trait]
pub trait NativeDispatch: Send + Sync {
    /// Launch a run for `attempt_id`. Observed events must be fed into `feeder`.
    /// Returns `Ok` once the launch is accepted; the run continues asynchronously.
    async fn launch(
        &self,
        attempt_id: String,
        input: String,
        feeder: NativeFeeder,
    ) -> Result<(), String>;
    /// Steer the active run for `attempt_id`.
    async fn steer(&self, attempt_id: String, message: String) -> Result<(), String>;
    /// Cancel the active run for `attempt_id`.
    async fn cancel(&self, attempt_id: String) -> Result<(), String>;
}

fn lock(state: &Arc<Mutex<NativeState>>) -> RunnerResult<std::sync::MutexGuard<'_, NativeState>> {
    state
        .lock()
        .map_err(|_| RunnerError::Other("native adapter inbox poisoned".to_string()))
}

/// Core normalization + enqueue, operating on a held guard. Shared by
/// [`NativeRunnerAdapter::feed`] (sync) and [`NativeFeeder::push_event`] (async).
fn push_event_locked(state: &mut NativeState, attempt_id: &str, event: &Event) -> RunnerResult<()> {
    if !state.inbox.contains_key(attempt_id) {
        return Err(RunnerError::UnknownAttempt {
            attempt_id: attempt_id.to_string(),
        });
    }
    if state.finished.contains(attempt_id) {
        return Err(RunnerError::Finished {
            attempt_id: attempt_id.to_string(),
        });
    }
    if let Some(kind) = map_event(event) {
        let payload = serde_json::to_value(event)
            .map_err(|error| RunnerError::Other(format!("cannot normalize event: {error}")))?;
        let seq = next_seq(state, attempt_id);
        let terminal = matches!(
            kind,
            RunnerEventKind::Completed
                | RunnerEventKind::Failed
                | RunnerEventKind::Cancelled
                | RunnerEventKind::Stalled
        );
        state
            .inbox
            .get_mut(attempt_id)
            .expect("checked above")
            .push_back(RunnerEvent {
                attempt_id: attempt_id.to_string(),
                kind,
                seq,
                payload,
            });
        if terminal {
            state.finished.insert(attempt_id.to_string());
        }
    }
    Ok(())
}

fn push_dispatch_failure_locked(state: &mut NativeState, failure: DispatchFailure) {
    // A native terminal event may have won the race before the dispatch task
    // itself failed. Preserve that first terminal outcome.
    if state.finished.contains(&failure.attempt_id) {
        return;
    }
    if !state.inbox.contains_key(&failure.attempt_id) {
        return;
    }
    let action = failure.action.as_str();
    let payload = serde_json::json!({
        "kind": "native_dispatch_failed",
        "action": action,
        "error": failure.error,
    });
    let attempt_id = failure.attempt_id;
    let seq = next_seq(state, &attempt_id);
    state
        .inbox
        .get_mut(&attempt_id)
        .expect("checked above")
        .push_back(RunnerEvent {
            attempt_id: attempt_id.clone(),
            kind: RunnerEventKind::Stalled,
            seq,
            payload,
        });
    state.finished.insert(attempt_id);
}

fn next_seq(state: &mut NativeState, attempt_id: &str) -> Seq {
    let entry = state.seq.entry(attempt_id.to_string()).or_insert(0);
    *entry += 1;
    *entry
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
        // Only a *first* start launches: O6a made re-start idempotent so already-
        // delivered bus events are preserved, and relaunching would start a
        // second real run for one attempt. Detect vacancy atomically with the
        // inbox insert so the spawn decision matches creation.
        let is_new = {
            let mut state = lock(&self.state)?;
            if state.finished.contains(&spec.attempt_id) {
                return Err(RunnerError::Finished {
                    attempt_id: spec.attempt_id.clone(),
                });
            }
            let is_new = match state.inbox.entry(spec.attempt_id.clone()) {
                std::collections::hash_map::Entry::Vacant(v) => {
                    v.insert(VecDeque::new());
                    true
                }
                std::collections::hash_map::Entry::Occupied(_) => false,
            };
            is_new
        };
        // O6c: route to the live runtime via the async dispatch. The run launches
        // asynchronously; observed events flow back through the feeder. The
        // attempt's id doubles as the native chat_id, so the adapter can poll
        // events before the launched task has produced any.
        if is_new {
            let feeder = self.feeder();
            if let Some(bridge) = &mut self.bridge {
                let dispatch = bridge.dispatch.clone();
                let handle = bridge.handle.clone();
                let attempt_id = spec.attempt_id.clone();
                let input = spec.input.clone();
                let task_attempt_id = attempt_id.clone();
                let join =
                    handle.spawn(async move { dispatch.launch(attempt_id, input, feeder).await });
                bridge.tasks.push(DispatchTask {
                    attempt_id: task_attempt_id,
                    action: DispatchAction::Launch,
                    join,
                });
            }
        }
        Ok(AttemptIdentity {
            attempt_id: spec.attempt_id.clone(),
            handle: spec.attempt_id.clone(),
        })
    }

    fn poll_event(&mut self, identity: &AttemptIdentity) -> RunnerResult<Option<RunnerEvent>> {
        validate_identity(identity)?;
        let failures = self
            .bridge
            .as_mut()
            .map(NativeBridge::collect_failures)
            .unwrap_or_default();
        let mut state = lock(&self.state)?;
        for failure in failures {
            push_dispatch_failure_locked(&mut state, failure);
        }
        let Some(queue) = state.inbox.get_mut(&identity.attempt_id) else {
            return Err(RunnerError::UnknownAttempt {
                attempt_id: identity.attempt_id.clone(),
            });
        };
        Ok(queue.pop_front())
    }

    fn steer(&mut self, identity: &AttemptIdentity, message: &str) -> RunnerResult<()> {
        validate_identity(identity)?;
        {
            let mut state = lock(&self.state)?;
            if !state.inbox.contains_key(&identity.attempt_id) {
                return Err(RunnerError::UnknownAttempt {
                    attempt_id: identity.attempt_id.clone(),
                });
            }
            if state.finished.contains(&identity.attempt_id) {
                return Err(RunnerError::Finished {
                    attempt_id: identity.attempt_id.clone(),
                });
            }
            // Record the steering intent; the async dispatch executes it.
            state
                .steer_log
                .push((identity.attempt_id.clone(), message.to_string()));
        }
        // O6c: route to the runtime's `route_steer` asynchronously.
        if let Some(bridge) = &mut self.bridge {
            let dispatch = bridge.dispatch.clone();
            let handle = bridge.handle.clone();
            let attempt_id = identity.attempt_id.clone();
            let message = message.to_string();
            let task_attempt_id = attempt_id.clone();
            let join = handle.spawn(async move { dispatch.steer(attempt_id, message).await });
            bridge.tasks.push(DispatchTask {
                attempt_id: task_attempt_id,
                action: DispatchAction::Steer,
                join,
            });
        }
        Ok(())
    }

    fn cancel(&mut self, identity: &AttemptIdentity) -> RunnerResult<()> {
        validate_identity(identity)?;
        {
            let mut state = lock(&self.state)?;
            if !state.inbox.contains_key(&identity.attempt_id) {
                return Err(RunnerError::UnknownAttempt {
                    attempt_id: identity.attempt_id.clone(),
                });
            }
            if state.finished.contains(&identity.attempt_id) {
                return Err(RunnerError::Finished {
                    attempt_id: identity.attempt_id.clone(),
                });
            }
            // Record the cancel intent; the async dispatch executes it.
            state.cancel_log.push(identity.attempt_id.clone());
        }
        // O6c: route to the runtime's `route_cancel` asynchronously.
        if let Some(bridge) = &mut self.bridge {
            let dispatch = bridge.dispatch.clone();
            let handle = bridge.handle.clone();
            let attempt_id = identity.attempt_id.clone();
            let task_attempt_id = attempt_id.clone();
            let join = handle.spawn(async move { dispatch.cancel(attempt_id).await });
            bridge.tasks.push(DispatchTask {
                attempt_id: task_attempt_id,
                action: DispatchAction::Cancel,
                join,
            });
        }
        Ok(())
    }

    fn shutdown(&mut self) {
        if let Some(bridge) = &mut self.bridge {
            bridge.abort_all();
        }
        // Shutdown cannot report an error through RunnerAdapter, so recover a
        // poisoned guard and still honor the cleanup contract. Clear the poison
        // marker only after every resource has been released.
        let mut state = match self.state.lock() {
            Ok(state) => state,
            Err(poisoned) => poisoned.into_inner(),
        };
        state.inbox.clear();
        state.seq.clear();
        state.finished.clear();
        state.steer_log.clear();
        state.cancel_log.clear();
        drop(state);
        self.state.clear_poison();
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
        assert_eq!(a.steers(), vec![("att-1".into(), "focus".into())]);
        assert_eq!(a.cancels(), vec!["att-1".to_string()]);

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

    // --- O6b: async/sync bridge ---------------------------------------------

    #[tokio::test]
    async fn async_feeder_feeds_sync_poll() {
        // The adapter is driven synchronously; a feeder (an async-task handle)
        // pushes events into the shared inbox. `poll_event` drains them without
        // copying — proving the Arc<Mutex> bridge between the async runtime and
        // the synchronous coordinator.
        let mut a = adapter_with_attempt("att-1");
        let feeder = a.feeder();

        let task = tokio::spawn(async move {
            feeder
                .push_event(
                    "att-1",
                    &Event::RunStarted {
                        run_id: "r1".into(),
                    },
                )
                .expect("push started");
            // Yield so the synchronous drainer (below) can observe ordering.
            tokio::task::yield_now().await;
            feeder
                .push_event(
                    "att-1",
                    &Event::AgentMessage {
                        content: "step".into(),
                        role: "assistant".into(),
                    },
                )
                .expect("push output");
            tokio::task::yield_now().await;
            feeder
                .push_event(
                    "att-1",
                    &Event::RunTerminated {
                        run_id: "r1".into(),
                        outcome: serde_json::json!({ "kind": "completed" }),
                    },
                )
                .expect("push terminal");
        });

        // Drain synchronously, yielding between sweeps so the feeder task runs.
        let id = AttemptIdentity {
            attempt_id: "att-1".into(),
            handle: "att-1".into(),
        };
        let mut kinds = Vec::new();
        for _ in 0..200 {
            while let Some(e) = a.poll_event(&id).expect("poll") {
                kinds.push(e.kind);
            }
            if kinds
                .iter()
                .any(|k| matches!(k, RunnerEventKind::Completed))
            {
                break;
            }
            tokio::task::yield_now().await;
        }
        task.await.expect("feeder task");

        assert!(kinds.contains(&RunnerEventKind::Started));
        assert!(kinds.contains(&RunnerEventKind::Output));
        assert!(kinds.contains(&RunnerEventKind::Completed));
        // Ordering is preserved across the async/sync boundary.
        let pos = |k: RunnerEventKind| kinds.iter().position(|x| *x == k).unwrap();
        assert!(pos(RunnerEventKind::Started) < pos(RunnerEventKind::Completed));
    }

    #[tokio::test]
    async fn feeder_is_isolated_by_attempt() {
        let mut a = adapter_with_attempt("att-1");
        let feeder = a.feeder();
        // Pushing to an attempt this adapter never started is rejected, even
        // from an async task.
        let err = tokio::spawn(async move {
            feeder.push_event(
                "att-other",
                &Event::RunStarted {
                    run_id: "r2".into(),
                },
            )
        })
        .await
        .expect("task");
        assert!(matches!(err, Err(RunnerError::UnknownAttempt { .. })));
        // att-1 remains untouched.
        assert_eq!(poll_kind(&mut a, "att-1"), None);
    }

    #[tokio::test]
    async fn feeder_shares_terminal_state_with_adapter() {
        // A terminal event pushed by the async feeder must be observed by the
        // synchronous adapter's `finished` guard: subsequent feeds are rejected.
        let mut a = adapter_with_attempt("att-1");
        let feeder = a.feeder();
        feeder
            .push_event(
                "att-1",
                &Event::RunTerminated {
                    run_id: "r1".into(),
                    outcome: serde_json::json!({ "kind": "failed" }),
                },
            )
            .expect("push terminal");
        // The adapter (sync side) sees the same terminal state.
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
    }

    #[test]
    fn feeder_can_be_cloned_and_shared() {
        // Multiple feeders (e.g. a bus tap + a replay task) share one inbox.
        let mut a = adapter_with_attempt("att-1");
        let f1 = a.feeder();
        let f2 = f1.clone();
        f1.push_event(
            "att-1",
            &Event::AgentMessage {
                content: "from-f1".into(),
                role: "assistant".into(),
            },
        )
        .expect("push");
        f2.push_event(
            "att-1",
            &Event::AgentMessage {
                content: "from-f2".into(),
                role: "assistant".into(),
            },
        )
        .expect("push");
        let id = AttemptIdentity {
            attempt_id: "att-1".into(),
            handle: "att-1".into(),
        };
        let e1 = a.poll_event(&id).unwrap().unwrap();
        let e2 = a.poll_event(&id).unwrap().unwrap();
        assert_eq!(e1.payload["content"], "from-f1");
        assert_eq!(e2.payload["content"], "from-f2");
        assert!(e1.seq < e2.seq);
    }

    #[test]
    fn shutdown_recovers_poisoned_state_and_cleans_every_resource() {
        let mut adapter = adapter_with_attempt("att-1");
        let identity = AttemptIdentity {
            attempt_id: "att-1".into(),
            handle: "att-1".into(),
        };
        adapter
            .feed(
                "att-1",
                &Event::AgentMessage {
                    content: "queued".into(),
                    role: "assistant".into(),
                },
            )
            .expect("feed");
        adapter.steer(&identity, "focus").expect("steer");
        adapter.cancel(&identity).expect("cancel");

        let shared = adapter.state.clone();
        let panic = std::thread::spawn(move || {
            let _guard = shared.lock().expect("lock before poison");
            panic!("poison native adapter state");
        })
        .join();
        assert!(panic.is_err());
        assert!(adapter.state.is_poisoned());

        adapter.shutdown();

        assert!(!adapter.state.is_poisoned());
        let state = adapter.state.lock().expect("recovered state");
        assert!(state.inbox.is_empty());
        assert!(state.seq.is_empty());
        assert!(state.finished.is_empty());
        assert!(state.steer_log.is_empty());
        assert!(state.cancel_log.is_empty());
    }

    // --- O6c: dispatch bridge -----------------------------------------------

    /// A `NativeDispatch` that scripts a normal run lifecycle (Started → Output →
    /// Completed) into the feeder and records steer/cancel calls. Stands in for
    /// the production `AgentRuntime` binding, which needs a running app + provider.
    struct FakeNativeDispatch {
        launches: Arc<Mutex<Vec<String>>>,
        steers: Arc<Mutex<Vec<(String, String)>>>,
        cancels: Arc<Mutex<Vec<String>>>,
    }

    impl FakeNativeDispatch {
        fn new() -> (Self, DispatchCalls) {
            let launches = Arc::new(Mutex::new(Vec::new()));
            let steers = Arc::new(Mutex::new(Vec::new()));
            let cancels = Arc::new(Mutex::new(Vec::new()));
            let calls = DispatchCalls {
                launches: Arc::clone(&launches),
                steers: Arc::clone(&steers),
                cancels: Arc::clone(&cancels),
            };
            let dispatch = Self {
                launches,
                steers,
                cancels,
            };
            (dispatch, calls)
        }
    }

    struct DispatchCalls {
        launches: Arc<Mutex<Vec<String>>>,
        steers: Arc<Mutex<Vec<(String, String)>>>,
        cancels: Arc<Mutex<Vec<String>>>,
    }

    #[async_trait]
    impl NativeDispatch for FakeNativeDispatch {
        async fn launch(
            &self,
            attempt_id: String,
            _input: String,
            feeder: NativeFeeder,
        ) -> Result<(), String> {
            self.launches.lock().unwrap().push(attempt_id.clone());
            // Script a normal lifecycle: started → one output → completed.
            feeder
                .push_event(
                    &attempt_id,
                    &Event::RunStarted {
                        run_id: attempt_id.clone(),
                    },
                )
                .map_err(|error| error.to_string())?;
            feeder
                .push_event(
                    &attempt_id,
                    &Event::AgentMessage {
                        content: "working".into(),
                        role: "assistant".into(),
                    },
                )
                .map_err(|error| error.to_string())?;
            feeder
                .push_event(
                    &attempt_id,
                    &Event::RunTerminated {
                        run_id: attempt_id.clone(),
                        outcome: serde_json::json!({ "kind": "completed" }),
                    },
                )
                .map_err(|error| error.to_string())?;
            Ok(())
        }

        async fn steer(&self, attempt_id: String, message: String) -> Result<(), String> {
            self.steers.lock().unwrap().push((attempt_id, message));
            Ok(())
        }

        async fn cancel(&self, attempt_id: String) -> Result<(), String> {
            self.cancels.lock().unwrap().push(attempt_id);
            Ok(())
        }
    }

    struct FailingNativeDispatch {
        action: DispatchAction,
    }

    impl FailingNativeDispatch {
        fn result(&self, action: DispatchAction) -> Result<(), String> {
            if self.action == action {
                Err(format!("{} route unavailable", action.as_str()))
            } else {
                Ok(())
            }
        }
    }

    #[async_trait]
    impl NativeDispatch for FailingNativeDispatch {
        async fn launch(
            &self,
            _attempt_id: String,
            _input: String,
            _feeder: NativeFeeder,
        ) -> Result<(), String> {
            self.result(DispatchAction::Launch)
        }

        async fn steer(&self, _attempt_id: String, _message: String) -> Result<(), String> {
            self.result(DispatchAction::Steer)
        }

        async fn cancel(&self, _attempt_id: String) -> Result<(), String> {
            self.result(DispatchAction::Cancel)
        }
    }

    struct PanickingNativeDispatch;

    #[async_trait]
    impl NativeDispatch for PanickingNativeDispatch {
        async fn launch(
            &self,
            _attempt_id: String,
            _input: String,
            _feeder: NativeFeeder,
        ) -> Result<(), String> {
            panic!("launch dispatch panic");
        }

        async fn steer(&self, _attempt_id: String, _message: String) -> Result<(), String> {
            Ok(())
        }

        async fn cancel(&self, _attempt_id: String) -> Result<(), String> {
            Ok(())
        }
    }

    struct PendingNativeDispatch {
        started: Arc<std::sync::atomic::AtomicBool>,
        dropped: Arc<std::sync::atomic::AtomicBool>,
    }

    struct DropSignal(Arc<std::sync::atomic::AtomicBool>);

    impl Drop for DropSignal {
        fn drop(&mut self) {
            self.0.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }

    #[async_trait]
    impl NativeDispatch for PendingNativeDispatch {
        async fn launch(
            &self,
            _attempt_id: String,
            _input: String,
            _feeder: NativeFeeder,
        ) -> Result<(), String> {
            self.started
                .store(true, std::sync::atomic::Ordering::SeqCst);
            let _drop_signal = DropSignal(self.dropped.clone());
            std::future::pending().await
        }

        async fn steer(&self, _attempt_id: String, _message: String) -> Result<(), String> {
            Ok(())
        }

        async fn cancel(&self, _attempt_id: String) -> Result<(), String> {
            Ok(())
        }
    }

    async fn next_dispatch_event(
        adapter: &mut NativeRunnerAdapter,
        identity: &AttemptIdentity,
    ) -> RunnerEvent {
        for _ in 0..500 {
            if let Some(event) = adapter.poll_event(identity).expect("poll") {
                return event;
            }
            tokio::task::yield_now().await;
        }
        panic!("dispatch task did not produce an observable event");
    }

    async fn drain_until_terminal(
        a: &mut NativeRunnerAdapter,
        id: &AttemptIdentity,
        terminal: RunnerEventKind,
    ) -> Vec<RunnerEventKind> {
        let mut kinds = Vec::new();
        for _ in 0..500 {
            while let Some(e) = a.poll_event(id).expect("poll") {
                kinds.push(e.kind);
            }
            if kinds.contains(&terminal) {
                break;
            }
            tokio::task::yield_now().await;
        }
        kinds
    }

    #[tokio::test]
    async fn dispatch_launch_feeds_events_to_completion() {
        // End-to-end O6c round trip: the synchronous coordinator starts an
        // attempt; the async dispatch launches and feeds events; the sync poll
        // drains them to a terminal — proving sync→async→sync works.
        let handle = tokio::runtime::Handle::current();
        let (dispatch, calls) = FakeNativeDispatch::new();
        let mut adapter = NativeRunnerAdapter::with_dispatch(Arc::new(dispatch), handle);

        let identity = adapter
            .start_attempt(&AttemptSpec {
                task_id: "t1".into(),
                attempt_id: "att-1".into(),
                input: "do the thing".into(),
            })
            .expect("start");

        let kinds = drain_until_terminal(&mut adapter, &identity, RunnerEventKind::Completed).await;

        assert!(kinds.contains(&RunnerEventKind::Started));
        assert!(kinds.contains(&RunnerEventKind::Output));
        assert!(kinds.contains(&RunnerEventKind::Completed));
        // Ordering: Started precedes Completed across the async/sync boundary.
        let pos = |k: RunnerEventKind| kinds.iter().position(|x| *x == k).unwrap();
        assert!(pos(RunnerEventKind::Started) < pos(RunnerEventKind::Completed));
        // The dispatch observed exactly one launch with the attempt's id.
        assert_eq!(*calls.launches.lock().unwrap(), vec!["att-1".to_string()]);
    }

    #[tokio::test]
    async fn duplicate_start_does_not_double_launch() {
        // start_attempt is idempotent: a re-start must not launch a second run.
        // O6a preserved already-delivered events on re-start; O6c must not
        // relaunch the runtime either.
        let handle = tokio::runtime::Handle::current();
        let (dispatch, calls) = FakeNativeDispatch::new();
        let mut adapter = NativeRunnerAdapter::with_dispatch(Arc::new(dispatch), handle);

        let spec = AttemptSpec {
            task_id: "t1".into(),
            attempt_id: "att-1".into(),
            input: "do the thing".into(),
        };
        adapter.start_attempt(&spec).expect("first start");
        adapter.start_attempt(&spec).expect("idempotent re-start");

        // Let any spawned launches run.
        for _ in 0..50 {
            tokio::task::yield_now().await;
        }

        // Exactly one launch, despite two start_attempt calls.
        assert_eq!(*calls.launches.lock().unwrap(), vec!["att-1".to_string()]);
    }

    #[tokio::test]
    async fn dispatch_steer_and_cancel_route_async() {
        let handle = tokio::runtime::Handle::current();
        let (dispatch, calls) = FakeNativeDispatch::new();
        let mut adapter = NativeRunnerAdapter::with_dispatch(Arc::new(dispatch), handle);

        let id = AttemptIdentity {
            attempt_id: "att-1".into(),
            handle: "att-1".into(),
        };
        adapter
            .start_attempt(&AttemptSpec {
                task_id: "t1".into(),
                attempt_id: "att-1".into(),
                input: String::new(),
            })
            .expect("start");

        adapter.steer(&id, "focus on tests").expect("steer");
        adapter.cancel(&id).expect("cancel");

        // Let the spawned dispatch tasks run.
        for _ in 0..50 {
            tokio::task::yield_now().await;
        }

        assert_eq!(
            *calls.steers.lock().unwrap(),
            vec![("att-1".to_string(), "focus on tests".to_string())]
        );
        assert_eq!(*calls.cancels.lock().unwrap(), vec!["att-1".to_string()]);
    }

    #[tokio::test]
    async fn pure_observation_adapter_never_dispatches() {
        // A `new()` adapter (no bridge) never launches: events arrive only via
        // the explicit `feed`/feeder seam. This is the O6a/O6b contract.
        let (dispatch, calls) = FakeNativeDispatch::new();
        // The dispatch is intentionally never wired; ensure no launch happens
        // when the bridge is absent.
        let _ = dispatch;
        let mut adapter = NativeRunnerAdapter::new();
        adapter
            .start_attempt(&AttemptSpec {
                task_id: "t1".into(),
                attempt_id: "att-1".into(),
                input: "ignored".into(),
            })
            .expect("start");
        // No async work was spawned: nothing drains without an explicit feed.
        for _ in 0..10 {
            assert_eq!(poll_kind(&mut adapter, "att-1"), None);
            tokio::task::yield_now().await;
        }
        assert!(calls.launches.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn dispatch_terminal_closes_attempt_before_poll() {
        // The async feeder marks the attempt terminal; a later sync cancel is
        // rejected with Finished even though the coordinator has not yet polled.
        let handle = tokio::runtime::Handle::current();
        let (dispatch, _) = FakeNativeDispatch::new();
        let mut adapter = NativeRunnerAdapter::with_dispatch(Arc::new(dispatch), handle);

        let id = AttemptIdentity {
            attempt_id: "att-1".into(),
            handle: "att-1".into(),
        };
        adapter
            .start_attempt(&AttemptSpec {
                task_id: "t1".into(),
                attempt_id: "att-1".into(),
                input: String::new(),
            })
            .expect("start");

        // Drain until the dispatch's terminal lands.
        drain_until_terminal(&mut adapter, &id, RunnerEventKind::Completed).await;

        // The attempt is now closed: cancel is rejected.
        assert!(matches!(
            adapter.cancel(&id),
            Err(RunnerError::Finished { .. })
        ));
    }

    #[tokio::test]
    async fn dispatch_errors_are_observable_for_every_action() {
        for action in [
            DispatchAction::Launch,
            DispatchAction::Steer,
            DispatchAction::Cancel,
        ] {
            let handle = tokio::runtime::Handle::current();
            let dispatch = FailingNativeDispatch { action };
            let mut adapter = NativeRunnerAdapter::with_dispatch(Arc::new(dispatch), handle);
            let identity = adapter
                .start_attempt(&AttemptSpec {
                    task_id: "t1".into(),
                    attempt_id: "att-1".into(),
                    input: String::new(),
                })
                .expect("start");

            match action {
                DispatchAction::Launch => {}
                DispatchAction::Steer => adapter.steer(&identity, "focus").expect("steer"),
                DispatchAction::Cancel => adapter.cancel(&identity).expect("cancel"),
            }

            let event = next_dispatch_event(&mut adapter, &identity).await;
            assert_eq!(event.kind, RunnerEventKind::Stalled);
            assert_eq!(event.payload["kind"], "native_dispatch_failed");
            assert_eq!(event.payload["action"], action.as_str());
            assert_eq!(
                event.payload["error"],
                format!("{} route unavailable", action.as_str())
            );
        }
    }

    #[tokio::test]
    async fn dispatch_panics_are_observable() {
        let handle = tokio::runtime::Handle::current();
        let mut adapter =
            NativeRunnerAdapter::with_dispatch(Arc::new(PanickingNativeDispatch), handle);
        let identity = adapter
            .start_attempt(&AttemptSpec {
                task_id: "t1".into(),
                attempt_id: "att-1".into(),
                input: String::new(),
            })
            .expect("start");

        let event = next_dispatch_event(&mut adapter, &identity).await;
        assert_eq!(event.kind, RunnerEventKind::Stalled);
        assert_eq!(event.payload["action"], "launch");
        assert!(event.payload["error"]
            .as_str()
            .expect("error string")
            .contains("panicked"));
    }

    #[tokio::test]
    async fn shutdown_aborts_pending_dispatch_tasks() {
        let started = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let dropped = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let dispatch = PendingNativeDispatch {
            started: started.clone(),
            dropped: dropped.clone(),
        };
        let handle = tokio::runtime::Handle::current();
        let mut adapter = NativeRunnerAdapter::with_dispatch(Arc::new(dispatch), handle);
        adapter
            .start_attempt(&AttemptSpec {
                task_id: "t1".into(),
                attempt_id: "att-1".into(),
                input: String::new(),
            })
            .expect("start");

        for _ in 0..500 {
            if started.load(std::sync::atomic::Ordering::SeqCst) {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert!(started.load(std::sync::atomic::Ordering::SeqCst));

        adapter.shutdown();

        for _ in 0..500 {
            if dropped.load(std::sync::atomic::Ordering::SeqCst) {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert!(dropped.load(std::sync::atomic::Ordering::SeqCst));
        assert!(adapter
            .bridge
            .as_ref()
            .expect("live bridge")
            .tasks
            .is_empty());
    }
}
