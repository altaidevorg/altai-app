//! O1 — Durable domain model for orchestration state transitions.
//!
//! This is the smallest authoritative model for task, attempt, and lease state
//! transitions. Only this module decides which transitions are legal; every
//! other component (coordinator, ledger, projections) must go through these
//! types. Invalid transitions are rejected with a typed [`TransitionError`].
//!
//! No SQLite or UI work lives here. See `docs/AGENT_OPERATIONS_IMPLEMENTATION_PLAN.md`
//! section 5.3 for the state machine and section 11.1 (O1) for acceptance.

use serde::{Deserialize, Serialize};
use std::fmt;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Returned when a requested state transition is not permitted.
///
/// Deliberately typed (not a `String`) so callers cannot silently ignore an
/// illegal move and so tests can assert on the exact reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransitionError {
    /// The transition is not defined for this state/trigger pair.
    Invalid {
        from: &'static str,
        trigger: &'static str,
    },
    /// The aggregate is already in a terminal state and cannot move again.
    Terminal {
        from: &'static str,
        trigger: &'static str,
    },
}

impl fmt::Display for TransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransitionError::Invalid { from, trigger } => write!(
                f,
                "illegal transition: trigger `{trigger}` is not valid from state `{from}`"
            ),
            TransitionError::Terminal { from, trigger } => write!(
                f,
                "illegal transition: `{from}` is terminal; trigger `{trigger}` has no effect"
            ),
        }
    }
}

impl std::error::Error for TransitionError {}

/// Returned when a lease operation cannot proceed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeaseError {
    /// The caller is not the current lease owner.
    NotOwner,
    /// The supplied generation does not match the active generation.
    StaleGeneration,
    /// The lease has already expired; only recovery may reclaim it.
    Expired,
}

impl fmt::Display for LeaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LeaseError::NotOwner => {
                f.write_str("lease operation rejected: caller is not the owner")
            }
            LeaseError::StaleGeneration => {
                f.write_str("lease operation rejected: stale generation")
            }
            LeaseError::Expired => f.write_str("lease operation rejected: lease has expired"),
        }
    }
}

impl std::error::Error for LeaseError {}

// ---------------------------------------------------------------------------
// Task state machine
// ---------------------------------------------------------------------------

/// Authoritative task lifecycle state (main flow plus side states).
///
/// Source: `AGENT_OPERATIONS_IMPLEMENTATION_PLAN.md` §5.3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    // Main happy path.
    Draft,
    Queued,
    Planning,
    AwaitingPlanApproval,
    Running,
    AwaitingInput,
    AwaitingApproval,
    Verifying,
    Reviewing,
    ReadyForHandoff,
    Done,
    // Side states.
    Blocked,
    Retrying,
    Paused,
    Cancelled,
    Failed,
    Abandoned,
}

impl TaskState {
    /// Terminal states are immutable: no trigger may move them. First-terminal
    /// wins and every later transition is rejected.
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            TaskState::Done | TaskState::Cancelled | TaskState::Failed | TaskState::Abandoned
        )
    }

    /// Returns the snake_case string name of the state (for diagnostics and
    /// projection payloads).
    pub fn name(self) -> &'static str {
        match self {
            TaskState::Draft => "draft",
            TaskState::Queued => "queued",
            TaskState::Planning => "planning",
            TaskState::AwaitingPlanApproval => "awaiting_plan_approval",
            TaskState::Running => "running",
            TaskState::AwaitingInput => "awaiting_input",
            TaskState::AwaitingApproval => "awaiting_approval",
            TaskState::Verifying => "verifying",
            TaskState::Reviewing => "reviewing",
            TaskState::ReadyForHandoff => "ready_for_handoff",
            TaskState::Done => "done",
            TaskState::Blocked => "blocked",
            TaskState::Retrying => "retrying",
            TaskState::Paused => "paused",
            TaskState::Cancelled => "cancelled",
            TaskState::Failed => "failed",
            TaskState::Abandoned => "abandoned",
        }
    }

    /// Apply `trigger` and return the resulting state, or a typed error if the
    /// move is illegal. This is the single source of truth for task transitions.
    pub fn transition(self, trigger: TaskTrigger) -> Result<TaskState, TransitionError> {
        use TaskState::*;

        if self.is_terminal() {
            // Abandon is the single permitted move out of terminal Failed or
            // Cancelled: it archives the task as permanently given up. Every
            // other trigger on a terminal task is rejected (first-terminal-wins).
            return match (self, trigger) {
                (TaskState::Failed | TaskState::Cancelled, TaskTrigger::Abandon) => {
                    Ok(TaskState::Abandoned)
                }
                _ => Err(TransitionError::Terminal {
                    from: self.name(),
                    trigger: trigger.name(),
                }),
            };
        }

        let next = match (self, trigger) {
            // Draft -> Queued.
            (Draft, TaskTrigger::Queue) => Queued,
            // Queued enters planning or is blocked.
            (Queued, TaskTrigger::StartPlanning) => Planning,
            (Queued, TaskTrigger::Block) => Blocked,
            // Planning may request approval or run directly.
            (Planning, TaskTrigger::RequestPlanApproval) => AwaitingPlanApproval,
            (Planning, TaskTrigger::StartRun) => Running,
            (Planning, TaskTrigger::Block) => Blocked,
            // Plan approval resolves to run or back to planning.
            (AwaitingPlanApproval, TaskTrigger::ApprovePlan) => Running,
            (AwaitingPlanApproval, TaskTrigger::RevisePlan) => Planning,
            (AwaitingPlanApproval, TaskTrigger::Block) => Blocked,
            // Running is the execution hub.
            (Running, TaskTrigger::NeedInput) => AwaitingInput,
            (Running, TaskTrigger::NeedApproval) => AwaitingApproval,
            (Running, TaskTrigger::Retry) => Retrying,
            (Running, TaskTrigger::StartVerify) => Verifying,
            (Running, TaskTrigger::Pause) => Paused,
            (Running, TaskTrigger::Block) => Blocked,
            (Running, TaskTrigger::Cancel) => Cancelled,
            (Running, TaskTrigger::Fail) => Failed,
            // Awaiting* states resume back to running or are cancelled.
            (AwaitingInput, TaskTrigger::Resume) => Running,
            (AwaitingInput, TaskTrigger::Cancel) => Cancelled,
            (AwaitingInput, TaskTrigger::Pause) => Paused,
            (AwaitingApproval, TaskTrigger::Resume) => Running,
            (AwaitingApproval, TaskTrigger::Cancel) => Cancelled,
            (AwaitingApproval, TaskTrigger::Pause) => Paused,
            // Retrying resumes execution or fails out.
            (Retrying, TaskTrigger::Resume) => Running,
            (Retrying, TaskTrigger::Fail) => Failed,
            (Retrying, TaskTrigger::Cancel) => Cancelled,
            // Verification leads to review or failure.
            (Verifying, TaskTrigger::StartReview) => Reviewing,
            (Verifying, TaskTrigger::Rework) => Running,
            (Verifying, TaskTrigger::Fail) => Failed,
            // Review produces a handoff-ready result or sends work back.
            (Reviewing, TaskTrigger::ReadyForHandoff) => ReadyForHandoff,
            (Reviewing, TaskTrigger::Rework) => Running,
            (Reviewing, TaskTrigger::Fail) => Failed,
            // Handoff resolves to Done (only after completion gates) or rework.
            (ReadyForHandoff, TaskTrigger::Complete) => Done,
            (ReadyForHandoff, TaskTrigger::Rework) => Running,
            // Paused resumes to running.
            (Paused, TaskTrigger::Resume) => Running,
            // Blocked returns to the queue.
            (Blocked, TaskTrigger::Unblock) => Queued,
            // A retrying or blocked task may be given up on directly.
            (Retrying, TaskTrigger::Abandon) => Abandoned,
            (Blocked, TaskTrigger::Abandon) => Abandoned,
            // Everything else is undefined.
            (from, t) => {
                return Err(TransitionError::Invalid {
                    from: from.name(),
                    trigger: t.name(),
                })
            }
        };
        Ok(next)
    }
}

impl Default for TaskState {
    fn default() -> Self {
        TaskState::Draft
    }
}

/// Events that drive task state transitions. Only the coordinator emits these
/// against authoritative state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskTrigger {
    Queue,
    StartPlanning,
    RequestPlanApproval,
    ApprovePlan,
    RevisePlan,
    StartRun,
    NeedInput,
    NeedApproval,
    Resume,
    Retry,
    StartVerify,
    StartReview,
    ReadyForHandoff,
    Rework,
    Complete,
    Pause,
    Block,
    Unblock,
    Cancel,
    Fail,
    Abandon,
}

impl TaskTrigger {
    pub fn name(self) -> &'static str {
        match self {
            TaskTrigger::Queue => "queue",
            TaskTrigger::StartPlanning => "start_planning",
            TaskTrigger::RequestPlanApproval => "request_plan_approval",
            TaskTrigger::ApprovePlan => "approve_plan",
            TaskTrigger::RevisePlan => "revise_plan",
            TaskTrigger::StartRun => "start_run",
            TaskTrigger::NeedInput => "need_input",
            TaskTrigger::NeedApproval => "need_approval",
            TaskTrigger::Resume => "resume",
            TaskTrigger::Retry => "retry",
            TaskTrigger::StartVerify => "start_verify",
            TaskTrigger::StartReview => "start_review",
            TaskTrigger::ReadyForHandoff => "ready_for_handoff",
            TaskTrigger::Rework => "rework",
            TaskTrigger::Complete => "complete",
            TaskTrigger::Pause => "pause",
            TaskTrigger::Block => "block",
            TaskTrigger::Unblock => "unblock",
            TaskTrigger::Cancel => "cancel",
            TaskTrigger::Fail => "fail",
            TaskTrigger::Abandon => "abandon",
        }
    }
}

// ---------------------------------------------------------------------------
// Attempt state machine
// ---------------------------------------------------------------------------

/// Authoritative attempt lifecycle state.
///
/// An attempt completion never directly produces a task `Done`; that requires
/// the configured completion gates at the task level (§5.3 rule).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptState {
    Created,
    Started,
    Heartbeat,
    InputRequired,
    ApprovalRequired,
    Steered,
    CancelRequested,
    Completed,
    Failed,
    Cancelled,
    Stalled,
}

impl AttemptState {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            AttemptState::Completed | AttemptState::Failed | AttemptState::Cancelled
        )
    }

    pub fn name(self) -> &'static str {
        match self {
            AttemptState::Created => "created",
            AttemptState::Started => "started",
            AttemptState::Heartbeat => "heartbeat",
            AttemptState::InputRequired => "input_required",
            AttemptState::ApprovalRequired => "approval_required",
            AttemptState::Steered => "steered",
            AttemptState::CancelRequested => "cancel_requested",
            AttemptState::Completed => "completed",
            AttemptState::Failed => "failed",
            AttemptState::Cancelled => "cancelled",
            AttemptState::Stalled => "stalled",
        }
    }

    /// Apply `trigger` and return the resulting state, or a typed error.
    pub fn transition(self, trigger: AttemptTrigger) -> Result<AttemptState, TransitionError> {
        use AttemptState::*;

        if self.is_terminal() {
            return Err(TransitionError::Terminal {
                from: self.name(),
                trigger: trigger.name(),
            });
        }

        let next = match (self, trigger) {
            (Created, AttemptTrigger::Start) => Started,
            (Created, AttemptTrigger::Cancel) => Cancelled,
            (Started, AttemptTrigger::Heartbeat) => Heartbeat,
            (Started, AttemptTrigger::NeedInput) => InputRequired,
            (Started, AttemptTrigger::NeedApproval) => ApprovalRequired,
            (Started, AttemptTrigger::Steer) => Steered,
            (Started, AttemptTrigger::RequestCancel) => CancelRequested,
            (Started, AttemptTrigger::Complete) => Completed,
            (Started, AttemptTrigger::Fail) => Failed,
            (Started, AttemptTrigger::Stall) => Stalled,
            (Heartbeat, AttemptTrigger::NeedInput) => InputRequired,
            (Heartbeat, AttemptTrigger::NeedApproval) => ApprovalRequired,
            (Heartbeat, AttemptTrigger::Steer) => Steered,
            (Heartbeat, AttemptTrigger::RequestCancel) => CancelRequested,
            (Heartbeat, AttemptTrigger::Complete) => Completed,
            (Heartbeat, AttemptTrigger::Fail) => Failed,
            (Heartbeat, AttemptTrigger::Stall) => Stalled,
            (InputRequired, AttemptTrigger::Resume) => Started,
            (InputRequired, AttemptTrigger::RequestCancel) => CancelRequested,
            (InputRequired, AttemptTrigger::Cancel) => Cancelled,
            (ApprovalRequired, AttemptTrigger::Resume) => Started,
            (ApprovalRequired, AttemptTrigger::RequestCancel) => CancelRequested,
            (ApprovalRequired, AttemptTrigger::Cancel) => Cancelled,
            (Steered, AttemptTrigger::Heartbeat) => Heartbeat,
            (Steered, AttemptTrigger::Complete) => Completed,
            (Steered, AttemptTrigger::Fail) => Failed,
            (Steered, AttemptTrigger::RequestCancel) => CancelRequested,
            // CancelRequested resolves to cancelled, or to completed/failed if
            // the terminal event raced ahead (first-terminal-wins).
            (CancelRequested, AttemptTrigger::Cancel) => Cancelled,
            (CancelRequested, AttemptTrigger::Complete) => Completed,
            (CancelRequested, AttemptTrigger::Fail) => Failed,
            // Stalled may resume or fail out.
            (Stalled, AttemptTrigger::Resume) => Started,
            (Stalled, AttemptTrigger::Fail) => Failed,
            (from, t) => {
                return Err(TransitionError::Invalid {
                    from: from.name(),
                    trigger: t.name(),
                })
            }
        };
        Ok(next)
    }
}

impl Default for AttemptState {
    fn default() -> Self {
        AttemptState::Created
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptTrigger {
    Start,
    Heartbeat,
    NeedInput,
    NeedApproval,
    Steer,
    Resume,
    RequestCancel,
    Cancel,
    Complete,
    Fail,
    Stall,
}

impl AttemptTrigger {
    pub fn name(self) -> &'static str {
        match self {
            AttemptTrigger::Start => "start",
            AttemptTrigger::Heartbeat => "heartbeat",
            AttemptTrigger::NeedInput => "need_input",
            AttemptTrigger::NeedApproval => "need_approval",
            AttemptTrigger::Steer => "steer",
            AttemptTrigger::Resume => "resume",
            AttemptTrigger::RequestCancel => "request_cancel",
            AttemptTrigger::Cancel => "cancel",
            AttemptTrigger::Complete => "complete",
            AttemptTrigger::Fail => "fail",
            AttemptTrigger::Stall => "stall",
        }
    }
}

// ---------------------------------------------------------------------------
// Lease
// ---------------------------------------------------------------------------

/// A dispatch lease. Claim and lease creation happen in one transaction at the
/// ledger layer (O2); this type only encodes the ownership/expiry invariants so
/// the coordinator can renew and reclaim deterministically.
///
/// Rules (§5.4):
/// - Heartbeats renew only the current generation.
/// - Recovery may reclaim an expired lease only after inspecting the runner and
///   workspace (that path bumps the generation; it is not a `renew`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Lease {
    pub owner: String,
    pub generation: u64,
    pub expires_at_ms: u64,
}

impl Lease {
    /// True when `now_ms` has reached or passed the expiry.
    pub fn is_expired(&self, now_ms: u64) -> bool {
        now_ms >= self.expires_at_ms
    }

    /// Renew the lease, extending its expiry by `ttl_ms`. Only the current owner
    /// operating on the current generation may renew. Renewing an expired lease
    /// is rejected: that requires a recovery reclaim (a new generation).
    pub fn renew(
        &self,
        owner: &str,
        generation: u64,
        now_ms: u64,
        ttl_ms: u64,
    ) -> Result<Lease, LeaseError> {
        if self.is_expired(now_ms) {
            return Err(LeaseError::Expired);
        }
        if self.owner != owner {
            return Err(LeaseError::NotOwner);
        }
        if self.generation != generation {
            return Err(LeaseError::StaleGeneration);
        }
        Ok(Lease {
            owner: self.owner.clone(),
            generation: self.generation,
            expires_at_ms: now_ms.saturating_add(ttl_ms),
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Task: every allowed terminal transition ----------------------------

    #[test]
    fn task_done_via_handoff() {
        // ReadyForHandoff -> Complete -> Done.
        let s = TaskState::ReadyForHandoff
            .transition(TaskTrigger::Complete)
            .unwrap();
        assert_eq!(s, TaskState::Done);
        assert!(s.is_terminal());
    }

    #[test]
    fn task_failed_from_running() {
        let s = TaskState::Running.transition(TaskTrigger::Fail).unwrap();
        assert_eq!(s, TaskState::Failed);
        assert!(s.is_terminal());
    }

    #[test]
    fn task_cancelled_from_running() {
        let s = TaskState::Running.transition(TaskTrigger::Cancel).unwrap();
        assert_eq!(s, TaskState::Cancelled);
        assert!(s.is_terminal());
    }

    #[test]
    fn task_abandoned_from_failed() {
        let s = TaskState::Failed.transition(TaskTrigger::Abandon).unwrap();
        assert_eq!(s, TaskState::Abandoned);
        assert!(s.is_terminal());
    }

    #[test]
    fn task_abandoned_from_cancelled() {
        let s = TaskState::Cancelled
            .transition(TaskTrigger::Abandon)
            .unwrap();
        assert_eq!(s, TaskState::Abandoned);
    }

    // --- Task: representative invalid transitions ---------------------------

    #[test]
    fn task_complete_is_rejected_outside_handoff() {
        // Completing from Draft is undefined.
        let err = TaskState::Draft
            .transition(TaskTrigger::Complete)
            .unwrap_err();
        assert_eq!(
            err,
            TransitionError::Invalid {
                from: "draft",
                trigger: "complete"
            }
        );
    }

    #[test]
    fn task_terminal_blocks_all_triggers() {
        // First-terminal wins: once Done, nothing else may move it.
        let err = TaskState::Done.transition(TaskTrigger::Fail).unwrap_err();
        assert!(matches!(err, TransitionError::Terminal { .. }));
        let err = TaskState::Done.transition(TaskTrigger::Cancel).unwrap_err();
        assert!(matches!(err, TransitionError::Terminal { .. }));
    }

    #[test]
    fn task_blocked_cannot_run() {
        let err = TaskState::Blocked
            .transition(TaskTrigger::StartRun)
            .unwrap_err();
        assert!(matches!(err, TransitionError::Invalid { .. }));
    }

    #[test]
    fn task_running_cannot_jump_to_done() {
        let err = TaskState::Running
            .transition(TaskTrigger::Complete)
            .unwrap_err();
        assert!(matches!(err, TransitionError::Invalid { .. }));
    }

    // --- Task: happy-path coverage ------------------------------------------

    #[test]
    fn task_full_happy_path() {
        let mut s = TaskState::Draft;
        for (trigger, expected) in [
            (TaskTrigger::Queue, TaskState::Queued),
            (TaskTrigger::StartPlanning, TaskState::Planning),
            (
                TaskTrigger::RequestPlanApproval,
                TaskState::AwaitingPlanApproval,
            ),
            (TaskTrigger::ApprovePlan, TaskState::Running),
            (TaskTrigger::StartVerify, TaskState::Verifying),
            (TaskTrigger::StartReview, TaskState::Reviewing),
            (TaskTrigger::ReadyForHandoff, TaskState::ReadyForHandoff),
            (TaskTrigger::Complete, TaskState::Done),
        ] {
            s = s.transition(trigger).unwrap();
            assert_eq!(s, expected, "after {trigger:?}");
        }
        assert!(s.is_terminal());
    }

    #[test]
    fn task_pause_resume_roundtrip() {
        let s = TaskState::Running.transition(TaskTrigger::Pause).unwrap();
        assert_eq!(s, TaskState::Paused);
        let s = s.transition(TaskTrigger::Resume).unwrap();
        assert_eq!(s, TaskState::Running);
    }

    #[test]
    fn task_retry_creates_retrying_state() {
        // Note: retries create a new attempt identity (coordinator concern);
        // at the task level they move into Retrying then resume.
        let s = TaskState::Running.transition(TaskTrigger::Retry).unwrap();
        assert_eq!(s, TaskState::Retrying);
        let s = s.transition(TaskTrigger::Resume).unwrap();
        assert_eq!(s, TaskState::Running);
    }

    #[test]
    fn task_block_unblock_returns_to_queue() {
        let s = TaskState::Queued.transition(TaskTrigger::Block).unwrap();
        assert_eq!(s, TaskState::Blocked);
        let s = s.transition(TaskTrigger::Unblock).unwrap();
        assert_eq!(s, TaskState::Queued);
    }

    // --- Attempt: terminal transitions --------------------------------------

    #[test]
    fn attempt_completed() {
        let s = AttemptState::Started
            .transition(AttemptTrigger::Complete)
            .unwrap();
        assert_eq!(s, AttemptState::Completed);
        assert!(s.is_terminal());
    }

    #[test]
    fn attempt_failed() {
        let s = AttemptState::Started
            .transition(AttemptTrigger::Fail)
            .unwrap();
        assert_eq!(s, AttemptState::Failed);
        assert!(s.is_terminal());
    }

    #[test]
    fn attempt_cancelled_after_request() {
        let s = AttemptState::Started
            .transition(AttemptTrigger::RequestCancel)
            .unwrap();
        assert_eq!(s, AttemptState::CancelRequested);
        let s = s.transition(AttemptTrigger::Cancel).unwrap();
        assert_eq!(s, AttemptState::Cancelled);
        assert!(s.is_terminal());
    }

    #[test]
    fn attempt_first_terminal_wins() {
        // Completed attempt cannot be cancelled afterwards.
        let err = AttemptState::Completed
            .transition(AttemptTrigger::Cancel)
            .unwrap_err();
        assert!(matches!(err, TransitionError::Terminal { .. }));
    }

    #[test]
    fn attempt_cancel_race_completes() {
        // A complete that races ahead of cancellation still wins.
        let s = AttemptState::CancelRequested
            .transition(AttemptTrigger::Complete)
            .unwrap();
        assert_eq!(s, AttemptState::Completed);
    }

    // --- Attempt: representative invalid transitions -------------------------

    #[test]
    fn attempt_created_cannot_complete() {
        let err = AttemptState::Created
            .transition(AttemptTrigger::Complete)
            .unwrap_err();
        assert!(matches!(err, TransitionError::Invalid { .. }));
    }

    #[test]
    fn attempt_completed_cannot_start() {
        let err = AttemptState::Completed
            .transition(AttemptTrigger::Start)
            .unwrap_err();
        assert!(matches!(err, TransitionError::Terminal { .. }));
    }

    #[test]
    fn attempt_input_required_cannot_complete() {
        let err = AttemptState::InputRequired
            .transition(AttemptTrigger::Complete)
            .unwrap_err();
        assert!(matches!(err, TransitionError::Invalid { .. }));
    }

    #[test]
    fn attempt_happy_path() {
        let mut s = AttemptState::Created;
        s = s.transition(AttemptTrigger::Start).unwrap();
        s = s.transition(AttemptTrigger::Heartbeat).unwrap();
        s = s.transition(AttemptTrigger::NeedInput).unwrap();
        assert_eq!(s, AttemptState::InputRequired);
        s = s.transition(AttemptTrigger::Resume).unwrap();
        s = s.transition(AttemptTrigger::Complete).unwrap();
        assert_eq!(s, AttemptState::Completed);
    }

    // --- Lease --------------------------------------------------------------

    #[test]
    fn lease_renew_extends_expiry_for_owner() {
        let lease = Lease {
            owner: "coord-1".into(),
            generation: 1,
            expires_at_ms: 1_000,
        };
        let renewed = lease.renew("coord-1", 1, 900, 500).unwrap();
        assert_eq!(renewed.owner, "coord-1");
        assert_eq!(renewed.generation, 1);
        assert_eq!(renewed.expires_at_ms, 1_400);
        assert!(!renewed.is_expired(900));
    }

    #[test]
    fn lease_renew_rejects_wrong_owner() {
        let lease = Lease {
            owner: "coord-1".into(),
            generation: 1,
            expires_at_ms: 1_000,
        };
        assert_eq!(
            lease.renew("coord-2", 1, 900, 500).unwrap_err(),
            LeaseError::NotOwner
        );
    }

    #[test]
    fn lease_renew_rejects_stale_generation() {
        let lease = Lease {
            owner: "coord-1".into(),
            generation: 2,
            expires_at_ms: 1_000,
        };
        assert_eq!(
            lease.renew("coord-1", 1, 900, 500).unwrap_err(),
            LeaseError::StaleGeneration
        );
    }

    #[test]
    fn lease_renew_rejects_expired() {
        let lease = Lease {
            owner: "coord-1".into(),
            generation: 1,
            expires_at_ms: 1_000,
        };
        assert!(lease.is_expired(1_000));
        assert_eq!(
            lease.renew("coord-1", 1, 2_000, 500).unwrap_err(),
            LeaseError::Expired
        );
    }

    // --- Default sanity ------------------------------------------------------

    #[test]
    fn defaults_are_initial_states() {
        assert_eq!(TaskState::default(), TaskState::Draft);
        assert_eq!(AttemptState::default(), AttemptState::Created);
    }
}
