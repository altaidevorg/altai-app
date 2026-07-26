//! O5 — Startup recovery and crash reconciliation.
//!
//! On restart the durable ledger is the source of truth, but an attempt may
//! have been left mid-flight (runner gone) or a terminal attempt may not have
//! been reflected in the task projection (crash between attempt completion and
//! task update). This pass reconciles those without guessing: it parks
//! ambiguous tasks in [`TaskState::NeedsAttention`] and replays missed
//! terminal reactions exactly once.
//!
//! There is no persisted legacy `altai-orchestration.json` in this codebase
//! (the pre-v2 model is in-memory), so there is nothing to import; the ledger
//! is durable and is reconciled directly. See
//! `docs/AGENT_OPERATIONS_IMPLEMENTATION_PLAN.md` §A3.

use super::coordinator::{Clock, Coordinator, CoordinatorError, CoordinatorPolicy};
use super::domain::{AttemptState, TaskState};
use super::ledger::OrchestrationLedger;

/// Summary of one recovery pass (assertable in tests).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RecoveryReport {
    /// Attempts whose lease had lapsed and were parked `Stalled`.
    pub leased_reclaimed: usize,
    /// Terminal attempts whose task projection was corrected (replayed once).
    pub replays: usize,
    /// Tasks parked in `NeedsAttention` (ambiguous in-flight state).
    pub needs_attention: usize,
    /// Orphaned non-terminal attempts parked `Stalled` before operator action.
    pub orphaned_stalled: usize,
}

/// Run the startup reconciliation pass. Idempotent: a second run performs no
/// additional writes (every write is keyed by a deterministic event id).
pub fn run(
    ledger: &OrchestrationLedger,
    clock: &impl Clock,
) -> Result<RecoveryReport, CoordinatorError> {
    let coord = Coordinator::new(ledger, CoordinatorPolicy::default());
    let now = clock.now_ms();
    let mut report = RecoveryReport::default();

    // 1. Park attempts whose lease has lapsed (lost-lease recovery).
    report.leased_reclaimed += coord.reclaim_expired_leases(clock)?.len();

    // 2. Reconcile every non-terminal task against its latest attempt.
    for task in ledger.non_terminal_tasks()? {
        let Some(attempt) = ledger.latest_attempt(&task.task_id)? else {
            // No attempt yet: nothing to reconcile (the task simply has not run).
            continue;
        };

        if attempt.state.is_terminal() {
            // The attempt finished but the task may not reflect it (crash
            // between attempt completion and the task projection update).
            if let Some(target) = terminal_target(task.state, attempt.state) {
                let event_id = format!("{}:recovery:{}", attempt.attempt_id, target.name());
                let status = ledger.set_task_state(&task.task_id, target, &event_id, now)?;
                if status.is_written() {
                    report.replays += 1;
                }
            }
        } else if is_ambiguous_active(task.state) {
            // An in-flight attempt with no live runner after a restart: do not
            // guess. First park the attempt itself so resolving the task back
            // to Queued can create a fresh attempt instead of reusing a dead
            // runner handle.
            if attempt.state != AttemptState::Stalled {
                let event_id = format!("{}:recovery:stalled", attempt.attempt_id);
                let status = ledger.set_attempt_state(
                    &attempt.attempt_id,
                    AttemptState::Stalled,
                    None,
                    &event_id,
                    None,
                    now,
                )?;
                if status.is_written() {
                    report.orphaned_stalled += 1;
                }
            }
            // Park the task for attention so an operator decides whether to
            // retry or abandon it.
            let event_id = format!("{}:recovery:needs_attention", attempt.attempt_id);
            let status =
                ledger.set_task_state(&task.task_id, TaskState::NeedsAttention, &event_id, now)?;
            if status.is_written() {
                report.needs_attention += 1;
            }
        }
    }

    Ok(report)
}

/// The task state a terminal attempt implies, if the task is still in an
/// active (unresolved) state. `None` means the task already reflects (or has
/// moved past) the attempt outcome.
fn terminal_target(task: TaskState, attempt: AttemptState) -> Option<TaskState> {
    match attempt {
        AttemptState::Completed if is_unresolved_execution(task) => Some(TaskState::Verifying),
        AttemptState::Cancelled if is_unresolved_execution(task) => Some(TaskState::Cancelled),
        // Retrying already is the persisted reaction to a failed attempt. Do
        // not overwrite it during restart; the actor must be allowed to create
        // the next attempt.
        AttemptState::Failed if task == TaskState::Retrying => None,
        AttemptState::Failed if is_unresolved_execution(task) => Some(TaskState::NeedsAttention),
        _ => None,
    }
}

/// Task states whose latest terminal attempt has not yet been projected.
fn is_unresolved_execution(task: TaskState) -> bool {
    matches!(
        task,
        TaskState::Running
            | TaskState::AwaitingInput
            | TaskState::AwaitingApproval
            | TaskState::Retrying
            | TaskState::Paused
    )
}

/// A subset of active states that are genuinely ambiguous after a crash (the
/// attempt is non-terminal and the runner is gone). A paused task is included:
/// the user's intent remains paused, but its pre-restart runner handle is no
/// longer resumable and must be explicitly reconciled.
fn is_ambiguous_active(task: TaskState) -> bool {
    matches!(
        task,
        TaskState::Running
            | TaskState::AwaitingInput
            | TaskState::AwaitingApproval
            | TaskState::Retrying
            | TaskState::Paused
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::orchestration::coordinator::{Coordinator, CoordinatorPolicy, ManualClock};
    use crate::modules::orchestration::domain::{Lease, TaskState, TaskTrigger};
    use crate::modules::orchestration::ledger::{
        CreateAttemptRequest, OrchestrationLedger, TaskRecord, WriteStatus,
    };
    use crate::modules::orchestration::runners::mock::MockRunner;

    fn seed(task_id: &str, state: TaskState) -> OrchestrationLedger {
        let ledger = OrchestrationLedger::open_in_memory().expect("ledger");
        ledger
            .upsert_task(&TaskRecord {
                task_id: task_id.into(),
                workspace_key: "ws".into(),
                source_kind: "local".into(),
                source_ref: format!("local://{task_id}"),
                title: "Do the thing".into(),
                state,
                created_at_ms: 1_000,
                updated_at_ms: 1_000,
            })
            .expect("seed task");
        ledger
    }

    fn start_attempt(ledger: &OrchestrationLedger, task_id: &str, no: u32) {
        let attempt_id = format!("{task_id}-att-{no}");
        ledger
            .create_attempt(&CreateAttemptRequest {
                attempt_id: attempt_id.clone(),
                task_id: task_id.to_string(),
                attempt_no: no,
                runner_kind: "native".into(),
                lease: Some(Lease {
                    owner: "coordinator".into(),
                    generation: 1,
                    expires_at_ms: i64::MAX as u64, // far future: not expired
                }),
                idempotency_key: format!("{task_id}:{no}:native"),
                now_ms: 1_000,
            })
            .unwrap();
        ledger
            .set_attempt_state(
                &attempt_id,
                AttemptState::Started,
                None,
                &format!("{attempt_id}:started"),
                None,
                1_000,
            )
            .unwrap();
    }

    // --- Crash after claim, before terminal: no duplicate + needs attention --

    #[test]
    fn crash_after_claim_does_not_duplicate_and_parks_for_attention() {
        let ledger = seed("t1", TaskState::Running);
        start_attempt(&ledger, "t1", 1);
        let clock = ManualClock::new(2_000);

        // Simulate restart: a fresh coordinator re-claiming must reuse the
        // existing attempt (idempotency key), never create a second run.
        let coord = Coordinator::new(&ledger, CoordinatorPolicy::default());
        let mut runner = MockRunner::new();
        let id = coord
            .claim_and_start("t1", "native", &mut runner, &clock)
            .expect("re-claim");
        assert_eq!(id.attempt_id, "t1-att-1");
        assert_eq!(ledger.attempts_for_task("t1").unwrap().len(), 1);

        // Recovery parks the orphaned in-flight task for attention.
        let report = run(&ledger, &clock).expect("recovery");
        assert_eq!(report.needs_attention, 1);
        assert_eq!(
            ledger.task("t1").unwrap().unwrap().state,
            TaskState::NeedsAttention
        );
        assert_eq!(
            ledger.attempt("t1-att-1").unwrap().unwrap().state,
            AttemptState::Stalled
        );
    }

    // --- Crash after completion, before projection: replay once -------------

    #[test]
    fn completion_before_projection_is_replayed_once() {
        let ledger = seed("t1", TaskState::Running);
        start_attempt(&ledger, "t1", 1);
        // The attempt completed, but the task projection never advanced.
        ledger
            .set_attempt_state(
                "t1-att-1",
                AttemptState::Completed,
                Some("completed"),
                "t1-att-1:complete",
                None,
                1_500,
            )
            .unwrap();
        let clock = ManualClock::new(2_000);

        let first = run(&ledger, &clock).expect("recovery 1");
        assert_eq!(first.replays, 1);
        assert_eq!(
            ledger.task("t1").unwrap().unwrap().state,
            TaskState::Verifying
        );

        // Re-running performs no additional writes (idempotent).
        let second = run(&ledger, &clock).expect("recovery 2");
        assert_eq!(second.replays, 0);
        assert_eq!(
            ledger.task("t1").unwrap().unwrap().state,
            TaskState::Verifying
        );
    }

    // --- Ambiguous failed attempt -> needs attention ------------------------

    #[test]
    fn failed_attempt_without_projection_becomes_needs_attention() {
        let ledger = seed("t1", TaskState::Running);
        start_attempt(&ledger, "t1", 1);
        ledger
            .set_attempt_state(
                "t1-att-1",
                AttemptState::Failed,
                Some("failed"),
                "t1-att-1:fail",
                None,
                1_500,
            )
            .unwrap();
        let clock = ManualClock::new(2_000);

        let report = run(&ledger, &clock).expect("recovery");
        assert_eq!(report.replays, 1);
        assert_eq!(
            ledger.task("t1").unwrap().unwrap().state,
            TaskState::NeedsAttention
        );
    }

    #[test]
    fn persisted_retrying_state_survives_recovery_and_creates_next_attempt() {
        let ledger = seed("t1", TaskState::Retrying);
        start_attempt(&ledger, "t1", 1);
        ledger
            .set_attempt_state(
                "t1-att-1",
                AttemptState::Failed,
                Some("failed"),
                "t1-att-1:fail",
                None,
                1_500,
            )
            .unwrap();
        let clock = ManualClock::new(2_000);

        let report = run(&ledger, &clock).expect("recovery");
        assert_eq!(report.replays, 0);
        assert_eq!(
            ledger.task("t1").unwrap().unwrap().state,
            TaskState::Retrying
        );

        let coord = Coordinator::new(&ledger, CoordinatorPolicy::default());
        let mut runner = MockRunner::new();
        let identity = coord
            .claim_and_start("t1", "native", &mut runner, &clock)
            .expect("retry claim");
        assert_eq!(identity.attempt_id, "t1-att-2");
    }

    // --- Idempotent migration ------------------------------------------------

    #[test]
    fn recovery_is_idempotent_across_runs() {
        let ledger = seed("t1", TaskState::Running);
        start_attempt(&ledger, "t1", 1);
        let clock = ManualClock::new(2_000);

        let first = run(&ledger, &clock).expect("recovery 1");
        let second = run(&ledger, &clock).expect("recovery 2");
        assert_eq!(first.needs_attention, 1);
        assert_eq!(first.orphaned_stalled, 1);
        assert_eq!(second.needs_attention, 0);
        assert_eq!(second.orphaned_stalled, 0);
        // Task stays parked; no extra attempts created.
        assert_eq!(ledger.attempts_for_task("t1").unwrap().len(), 1);
    }

    #[test]
    fn resolved_orphan_dispatches_a_fresh_attempt() {
        let ledger = seed("t1", TaskState::Running);
        start_attempt(&ledger, "t1", 1);
        let clock = ManualClock::new(2_000);

        run(&ledger, &clock).expect("recovery");
        assert_eq!(
            ledger.attempt("t1-att-1").unwrap().unwrap().state,
            AttemptState::Stalled
        );
        ledger
            .set_task_state("t1", TaskState::Queued, "operator:resolve:t1", 2_100)
            .expect("resolve");

        let coord = Coordinator::new(&ledger, CoordinatorPolicy::default());
        let mut runner = MockRunner::new();
        let identity = coord
            .claim_and_start("t1", "native", &mut runner, &clock)
            .expect("fresh claim");
        assert_eq!(identity.attempt_id, "t1-att-2");
        assert!(runner.was_started("t1-att-2"));
    }

    #[test]
    fn terminal_attempt_is_replayed_when_task_was_paused() {
        let ledger = seed("t1", TaskState::Paused);
        start_attempt(&ledger, "t1", 1);
        ledger
            .set_attempt_state(
                "t1-att-1",
                AttemptState::Completed,
                Some("completed"),
                "t1-att-1:complete",
                None,
                1_500,
            )
            .unwrap();
        let clock = ManualClock::new(2_000);

        let report = run(&ledger, &clock).expect("recovery");
        assert_eq!(report.replays, 1);
        assert_eq!(
            ledger.task("t1").unwrap().unwrap().state,
            TaskState::Verifying
        );
    }

    // --- Expired lease is still parked --------------------------------------

    #[test]
    fn expired_lease_is_reclaimed_by_recovery() {
        let ledger = seed("t1", TaskState::Running);
        let attempt_id = "t1-att-1";
        ledger
            .create_attempt(&CreateAttemptRequest {
                attempt_id: attempt_id.into(),
                task_id: "t1".into(),
                attempt_no: 1,
                runner_kind: "native".into(),
                lease: Some(Lease {
                    owner: "coordinator".into(),
                    generation: 1,
                    expires_at_ms: 0, // already expired
                }),
                idempotency_key: "t1:1:native".into(),
                now_ms: 1_000,
            })
            .unwrap();
        ledger
            .set_attempt_state(
                attempt_id,
                AttemptState::Started,
                None,
                "t1-att-1:started",
                None,
                1_000,
            )
            .unwrap();
        let clock = ManualClock::new(2_000);

        let report = run(&ledger, &clock).expect("recovery");
        assert!(report.leased_reclaimed >= 1);
        assert_eq!(
            ledger.attempt(attempt_id).unwrap().unwrap().state,
            AttemptState::Stalled
        );
        // And the orphaned task is parked for attention.
        assert_eq!(
            ledger.task("t1").unwrap().unwrap().state,
            TaskState::NeedsAttention
        );
    }

    // --- NeedsAttention can be resolved/cancelled ---------------------------

    #[test]
    fn needs_attention_resolves_back_to_queue() {
        // NeedsAttention is non-terminal and resolves to the queue or can be
        // cancelled/abandoned by an operator.
        let resolved = TaskState::NeedsAttention
            .transition(TaskTrigger::Resolve)
            .unwrap();
        assert_eq!(resolved, TaskState::Queued);
        let cancelled = TaskState::NeedsAttention
            .transition(TaskTrigger::Cancel)
            .unwrap();
        assert_eq!(cancelled, TaskState::Cancelled);
        // WriteStatus::is_written helper backs the recovery accounting.
        assert!(!WriteStatus::Duplicate.is_written());
    }
}
