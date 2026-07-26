//! O3 — Runner adapter contract.
//!
//! Defines the boundary between the orchestrator and any execution backend
//! (the native ALTAI runtime, Codex App Server, or a future CLI runner). The
//! coordinator (O3/O4) only ever talks to [`RunnerAdapter`]; provider-specific
//! state never leaks past this trait.
//!
//! Runner events are normalized into [`RunnerEventKind`] and then mapped to O1
//! domain triggers (see [`event_to_trigger`]). See
//! `docs/AGENT_OPERATIONS_IMPLEMENTATION_PLAN.md` §4.2.

use super::domain::AttemptTrigger;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

/// What a runner can do. Capabilities are checked before dispatching an action
/// the runner cannot satisfy.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunnerCapabilities {
    pub can_steer: bool,
    pub can_cancel: bool,
    pub can_resume: bool,
}

/// Input handed to a runner when an attempt starts. The orchestrator keeps the
/// authoritative identity; the runner returns its own opaque handle.
#[derive(Debug, Clone)]
pub struct AttemptSpec {
    pub task_id: String,
    pub attempt_id: String,
    /// Immutable effective input for this attempt (prompt/instructions).
    pub input: String,
}

/// A runner-side handle for one attempt. `handle` is opaque to the orchestrator
/// and only meaningful to the runner that produced it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AttemptIdentity {
    pub attempt_id: String,
    pub handle: String,
}

/// Normalized runner event kinds. Provider-specific payloads are kept as a
/// bounded JSON value; scheduler/UI logic consumes only these kinds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunnerEventKind {
    Started,
    Heartbeat,
    /// Progress/output that does not change attempt state.
    Output,
    InputRequired,
    ApprovalRequired,
    CancelRequested,
    Completed,
    Failed,
    Cancelled,
    Stalled,
}

/// A single normalized runner event for one attempt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunnerEvent {
    pub attempt_id: String,
    pub kind: RunnerEventKind,
    /// Per-attempt monotonically increasing sequence (1-based).
    pub seq: u64,
    pub payload: Value,
}

#[derive(Debug)]
pub enum RunnerError {
    UnknownAttempt { attempt_id: String },
    Unsupported { action: &'static str },
    Finished { attempt_id: String },
    Io(std::io::Error),
    Other(String),
}

impl fmt::Display for RunnerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunnerError::UnknownAttempt { attempt_id } => {
                write!(f, "runner has no attempt {attempt_id}")
            }
            RunnerError::Unsupported { action } => {
                write!(f, "runner does not support `{action}`")
            }
            RunnerError::Finished { attempt_id } => {
                write!(f, "runner attempt {attempt_id} is already finished")
            }
            RunnerError::Io(error) => write!(f, "runner I/O error: {error}"),
            RunnerError::Other(message) => write!(f, "runner error: {message}"),
        }
    }
}

impl std::error::Error for RunnerError {}

impl From<std::io::Error> for RunnerError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

pub type RunnerResult<T> = Result<T, RunnerError>;

/// The single boundary the orchestrator uses. Implementations may bridge an
/// async backend internally; the trait itself is synchronous so the coordinator
/// decision core can be tested deterministically without a runtime.
pub trait RunnerAdapter {
    fn capabilities(&self) -> RunnerCapabilities;

    /// Begin an attempt and return the runner-side identity.
    fn start_attempt(&mut self, spec: &AttemptSpec) -> RunnerResult<AttemptIdentity>;

    /// Poll for the next normalized event for this attempt, or `None` if the
    /// runner has nothing ready right now.
    fn poll_event(&mut self, identity: &AttemptIdentity) -> RunnerResult<Option<RunnerEvent>>;

    /// Steer a running attempt with a new message. Errors if unsupported.
    fn steer(&mut self, identity: &AttemptIdentity, message: &str) -> RunnerResult<()>;

    /// Request cancellation of an attempt. The runner should emit a terminal
    /// `Cancelled` (or `Completed`/`Failed` if it raced ahead).
    fn cancel(&mut self, identity: &AttemptIdentity) -> RunnerResult<()>;

    /// Release all runner-held resources. Called on shutdown.
    fn shutdown(&mut self);
}

/// Map a normalized runner event to the O1 domain trigger that advances the
/// attempt state machine. `Output` events advance no state and map to `None`.
pub fn event_to_trigger(kind: &RunnerEventKind) -> Option<AttemptTrigger> {
    Some(match kind {
        RunnerEventKind::Started => AttemptTrigger::Start,
        RunnerEventKind::Heartbeat => AttemptTrigger::Heartbeat,
        RunnerEventKind::Output => return None,
        RunnerEventKind::InputRequired => AttemptTrigger::NeedInput,
        RunnerEventKind::ApprovalRequired => AttemptTrigger::NeedApproval,
        RunnerEventKind::CancelRequested => AttemptTrigger::RequestCancel,
        RunnerEventKind::Completed => AttemptTrigger::Complete,
        RunnerEventKind::Failed => AttemptTrigger::Fail,
        RunnerEventKind::Cancelled => AttemptTrigger::Cancel,
        RunnerEventKind::Stalled => AttemptTrigger::Stall,
    })
}

pub mod mock;
pub mod native;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_events_do_not_advance_state() {
        assert_eq!(event_to_trigger(&RunnerEventKind::Output), None);
    }

    #[test]
    fn terminal_events_map_to_terminal_triggers() {
        assert_eq!(
            event_to_trigger(&RunnerEventKind::Completed),
            Some(AttemptTrigger::Complete)
        );
        assert_eq!(
            event_to_trigger(&RunnerEventKind::Failed),
            Some(AttemptTrigger::Fail)
        );
        assert_eq!(
            event_to_trigger(&RunnerEventKind::Cancelled),
            Some(AttemptTrigger::Cancel)
        );
    }
}
