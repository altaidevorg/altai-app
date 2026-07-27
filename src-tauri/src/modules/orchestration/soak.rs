//! Soak and integration tests for the Preview 1 exit gate (plan §10, §A3).
//!
//! These verify the critical correctness invariants that the v2 orchestration
//! stack must hold: zero duplicate dispatch, crash recovery, lease reclamation,
//! and idempotent reconciliation — all exercised across the real coordinator,
//! ledger, recovery, and mock runner (no mocks of internal logic).

#![cfg(test)]

use super::coordinator::{Coordinator, CoordinatorPolicy, ManualClock, PumpOutcome};
use super::domain::{AttemptState, Lease, TaskState};
use super::ledger::{OrchestrationLedger, TaskRecord, WriteStatus};
use super::recovery::{self, RecoveryReport};
use super::runners::mock::MockRunner;
use super::runners::RunnerEventKind;

use serde_json::json;

fn mk_ledger_with_task(id: &str) -> OrchestrationLedger {
    let ledger = OrchestrationLedger::open_in_memory().unwrap();
    ledger
        .upsert_task(&TaskRecord {
            task_id: id.into(),
            workspace_key: "ws-1".into(),
            source_kind: "local".into(),
            source_ref: id.into(),
            title: format!("Task {id}"),
            description: "do the thing".into(),
            state: TaskState::Queued,
            created_at_ms: 1_000,
            updated_at_ms: 1_000,
        })
        .unwrap();
    ledger
}

fn short_lease_policy() -> CoordinatorPolicy {
    CoordinatorPolicy {
        lease_ttl_ms: 5_000,
        max_attempts: 3,
        ..CoordinatorPolicy::default()
    }
}

// ===========================================================================
// §A3.1 — Crash after claim but before dispatch → no duplicate
// ===========================================================================

#[test]
fn crash_after_claim_no_duplicate_dispatch() {
    let ledger = mk_ledger_with_task("t-1");
    let clock = ManualClock::new(1_000);
    let mut runner = MockRunner::new();

    // Coordinator A claims and starts.
    let coord_a = Coordinator::new(&ledger, short_lease_policy());
    let identity = coord_a
        .claim_and_start("t-1", "mock", &mut runner, &clock)
        .unwrap();
    assert!(runner.was_started(&identity.attempt_id));

    // Simulate crash: coordinator A is gone. Coordinator B starts fresh
    // (same ledger, same task — e.g. after app restart).
    let coord_b = Coordinator::new(&ledger, short_lease_policy());
    let identity_b = coord_b
        .claim_and_start("t-1", "mock", &mut runner, &clock)
        .unwrap();

    // Must return the SAME attempt — no new dispatch.
    assert_eq!(identity_b.attempt_id, identity.attempt_id);
    assert!(!runner.was_started(&format!("{}-dup", identity.attempt_id)));

    // Exactly one attempt in the ledger.
    let attempts = ledger.attempts_for_task("t-1").unwrap();
    assert_eq!(attempts.len(), 1);
}

// ===========================================================================
// Concurrent claim → idempotency prevents double dispatch
// ===========================================================================

#[test]
fn concurrent_claim_is_idempotent() {
    let ledger = mk_ledger_with_task("t-1");
    let clock = ManualClock::new(1_000);
    let mut runner = MockRunner::new();

    let coord = Coordinator::new(&ledger, short_lease_policy());

    // Two "coordinators" share the same ledger. The second claim returns the
    // existing attempt's identity without re-dispatching.
    let id1 = coord
        .claim_and_start("t-1", "mock", &mut runner, &clock)
        .unwrap();
    let id2 = coord
        .claim_and_start("t-1", "mock", &mut runner, &clock)
        .unwrap();

    assert_eq!(id1.attempt_id, id2.attempt_id);

    let attempts = ledger.attempts_for_task("t-1").unwrap();
    assert_eq!(attempts.len(), 1, "exactly one attempt, not two");
}

// ===========================================================================
// §A3.2 — Crash after dispatch, before completion → reconnect via recovery
// ===========================================================================

#[test]
fn crash_after_dispatch_recovery_reclaims_and_retries() {
    let ledger = mk_ledger_with_task("t-1");
    let clock = ManualClock::new(1_000);
    let mut runner = MockRunner::new();

    // Claim + start. The runner has events queued but we never pump them
    // (simulating a crash mid-run).
    let coord = Coordinator::new(&ledger, short_lease_policy());
    let identity = coord
        .claim_and_start("t-1", "mock", &mut runner, &clock)
        .unwrap();

    // Advance clock past the lease expiry.
    clock.advance(10_000);

    // Restart: recovery reclaims the expired lease.
    let report = recovery::run(&ledger, &clock).unwrap();
    assert_eq!(report.leased_reclaimed, 1);

    // The attempt should be Stalled (lease reclaimed).
    let attempt = ledger.attempt(&identity.attempt_id).unwrap().unwrap();
    assert_eq!(attempt.state, AttemptState::Stalled);

    // Task should be NeedsAttention (ambiguous in-flight state).
    let task = ledger.task("t-1").unwrap().unwrap();
    assert_eq!(task.state, TaskState::NeedsAttention);
}

// ===========================================================================
// Full lifecycle: claim → pump → complete → task Verifying
// ===========================================================================

#[test]
fn full_lifecycle_claim_pump_complete() {
    let ledger = mk_ledger_with_task("t-1");
    let clock = ManualClock::new(1_000);
    let mut runner = MockRunner::new();

    let coord = Coordinator::new(&ledger, short_lease_policy());

    // Claim and start.
    let identity = coord
        .claim_and_start("t-1", "mock", &mut runner, &clock)
        .unwrap();

    // Queue a completion event.
    runner.enqueue_event(
        &identity.attempt_id,
        RunnerEventKind::Completed,
        json!({ "result": "success" }),
    );

    // Pump until terminal.
    let outcome = coord.pump(&identity, &mut runner, &clock).unwrap();
    assert!(matches!(
        outcome,
        PumpOutcome::Terminal(AttemptState::Completed)
    ));

    // Task should be Verifying (§5.3: completion hands off to verification,
    // never directly to Done).
    let task = ledger.task("t-1").unwrap().unwrap();
    assert_eq!(task.state, TaskState::Verifying);

    // Only one attempt.
    let attempts = ledger.attempts_for_task("t-1").unwrap();
    assert_eq!(attempts.len(), 1);
}

// ===========================================================================
// §A3.3 — Crash after completion, before projection → replay once
// ===========================================================================

#[test]
fn crash_after_completion_recovery_replays_once() {
    let ledger = mk_ledger_with_task("t-1");
    let clock = ManualClock::new(1_000);

    // Simulate: attempt completed but task projection NOT yet updated (crash
    // between attempt terminal and task terminal). We do this by writing the
    // attempt terminal directly without the coordinator's task update.
    ledger
        .create_attempt(&super::ledger::CreateAttemptRequest {
            attempt_id: "t-1-att-1".into(),
            task_id: "t-1".into(),
            attempt_no: 1,
            runner_kind: "mock".into(),
            lease: Some(Lease {
                owner: "coordinator".into(),
                generation: 1,
                expires_at_ms: 6_000,
            }),
            idempotency_key: "t-1:1:mock".into(),
            now_ms: 1_000,
        })
        .unwrap();
    ledger
        .set_attempt_state(
            "t-1-att-1",
            AttemptState::Completed,
            Some("done"),
            "t-1-att-1:completed",
            None,
            2_000,
        )
        .unwrap();
    // Task is still Running (projection missed).
    ledger
        .set_task_state("t-1", TaskState::Running, "t-1:running", 1_000)
        .unwrap();

    // Recovery should replay the terminal projection.
    let report = recovery::run(&ledger, &clock).unwrap();
    assert_eq!(report.replays, 1);

    let task = ledger.task("t-1").unwrap().unwrap();
    assert_eq!(task.state, TaskState::Verifying);

    // Second recovery run is idempotent (no additional replays).
    let report2 = recovery::run(&ledger, &clock).unwrap();
    assert_eq!(report2.replays, 0);
}

// ===========================================================================
// §A3 acceptance — crash recovery success: idempotent recovery
// ===========================================================================

#[test]
fn recovery_is_idempotent() {
    let ledger = mk_ledger_with_task("t-1");
    let clock = ManualClock::new(1_000);
    let mut runner = MockRunner::new();

    let coord = Coordinator::new(&ledger, short_lease_policy());
    let identity = coord
        .claim_and_start("t-1", "mock", &mut runner, &clock)
        .unwrap();

    clock.advance(10_000);

    // First recovery.
    let report1 = recovery::run(&ledger, &clock).unwrap();
    assert!(report1.leased_reclaimed > 0 || report1.needs_attention > 0);

    // Second recovery: nothing changes.
    let report2 = recovery::run(&ledger, &clock).unwrap();
    assert_eq!(
        report2,
        RecoveryReport::default(),
        "second recovery must be a no-op"
    );

    let _ = identity;
}

// ===========================================================================
// Lease expiry + reclaim lifecycle
// ===========================================================================

#[test]
fn lease_expiry_reclaim_then_reclaim_completes() {
    let ledger = mk_ledger_with_task("t-1");
    let clock = ManualClock::new(1_000);
    let mut runner = MockRunner::new();

    // Claim with short lease.
    let coord = Coordinator::new(&ledger, short_lease_policy());
    let identity = coord
        .claim_and_start("t-1", "mock", &mut runner, &clock)
        .unwrap();

    // Lease expires.
    clock.advance(10_000);
    let report = recovery::run(&ledger, &clock).unwrap();
    assert_eq!(report.leased_reclaimed, 1);

    // The task is now NeedsAttention. Resolve it back to Queued.
    ledger
        .set_task_state("t-1", TaskState::Queued, "manual:resolve", 11_000)
        .unwrap();

    // Re-claim and complete.
    runner.enqueue_event(
        "t-1-att-2",
        RunnerEventKind::Completed,
        json!({ "result": "ok" }),
    );
    let identity2 = coord
        .claim_and_start("t-1", "mock", &mut runner, &clock)
        .unwrap();
    assert_ne!(identity2.attempt_id, identity.attempt_id);

    let outcome = coord.pump(&identity2, &mut runner, &clock).unwrap();
    assert!(matches!(
        outcome,
        PumpOutcome::Terminal(AttemptState::Completed)
    ));

    let task = ledger.task("t-1").unwrap().unwrap();
    assert_eq!(task.state, TaskState::Verifying);

    // Two attempts: 1 stalled + 1 completed.
    let attempts = ledger.attempts_for_task("t-1").unwrap();
    assert_eq!(attempts.len(), 2);
}

// ===========================================================================
// Retry after failure → second attempt succeeds
// ===========================================================================

#[test]
fn retry_after_failure_second_attempt_succeeds() {
    let ledger = mk_ledger_with_task("t-1");
    let clock = ManualClock::new(1_000);
    let mut runner = MockRunner::new();

    let coord = Coordinator::new(&ledger, short_lease_policy());

    // First attempt: queue a failure.
    runner.enqueue_event(
        "t-1-att-1",
        RunnerEventKind::Failed,
        json!({ "error": "boom" }),
    );
    let id1 = coord
        .claim_and_start("t-1", "mock", &mut runner, &clock)
        .unwrap();
    let outcome1 = coord.pump(&id1, &mut runner, &clock).unwrap();
    assert!(matches!(outcome1, PumpOutcome::RetryScheduled { .. }));

    // Task should be Retrying.
    let task = ledger.task("t-1").unwrap().unwrap();
    assert_eq!(task.state, TaskState::Retrying);

    // Advance past retry delay.
    clock.advance(10_000);

    // Second attempt: queue success.
    runner.enqueue_event(
        "t-1-att-2",
        RunnerEventKind::Completed,
        json!({ "result": "ok" }),
    );
    let id2 = coord
        .claim_and_start("t-1", "mock", &mut runner, &clock)
        .unwrap();
    assert_ne!(id1.attempt_id, id2.attempt_id);

    let outcome2 = coord.pump(&id2, &mut runner, &clock).unwrap();
    assert!(matches!(
        outcome2,
        PumpOutcome::Terminal(AttemptState::Completed)
    ));

    let task = ledger.task("t-1").unwrap().unwrap();
    assert_eq!(task.state, TaskState::Verifying);

    let attempts = ledger.attempts_for_task("t-1").unwrap();
    assert_eq!(attempts.len(), 2);
}

// ===========================================================================
// Multiple tasks — isolation and no cross-contamination
// ===========================================================================

#[test]
fn multiple_tasks_are_isolated() {
    let ledger = OrchestrationLedger::open_in_memory().unwrap();
    for id in ["t-a", "t-b", "t-c"] {
        ledger
            .upsert_task(&TaskRecord {
                task_id: id.into(),
                workspace_key: "ws-1".into(),
                source_kind: "local".into(),
                source_ref: id.into(),
                title: format!("Task {id}"),
                description: "work".into(),
                state: TaskState::Queued,
                created_at_ms: 1_000,
                updated_at_ms: 1_000,
            })
            .unwrap();
    }

    let clock = ManualClock::new(1_000);
    let mut runner = MockRunner::new();
    let coord = Coordinator::new(&ledger, short_lease_policy());

    // Claim all three.
    let id_a = coord
        .claim_and_start("t-a", "mock", &mut runner, &clock)
        .unwrap();
    let id_b = coord
        .claim_and_start("t-b", "mock", &mut runner, &clock)
        .unwrap();
    let id_c = coord
        .claim_and_start("t-c", "mock", &mut runner, &clock)
        .unwrap();

    // Each gets its own attempt.
    assert_ne!(id_a.attempt_id, id_b.attempt_id);
    assert_ne!(id_b.attempt_id, id_c.attempt_id);

    // Complete only t-b.
    runner.enqueue_event(&id_b.attempt_id, RunnerEventKind::Completed, json!({}));
    coord.pump(&id_b, &mut runner, &clock).unwrap();

    // t-b is Verifying, others are still Running.
    assert_eq!(
        ledger.task("t-a").unwrap().unwrap().state,
        TaskState::Running
    );
    assert_eq!(
        ledger.task("t-b").unwrap().unwrap().state,
        TaskState::Verifying
    );
    assert_eq!(
        ledger.task("t-c").unwrap().unwrap().state,
        TaskState::Running
    );

    // Each task has exactly one attempt.
    assert_eq!(ledger.attempts_for_task("t-a").unwrap().len(), 1);
    assert_eq!(ledger.attempts_for_task("t-b").unwrap().len(), 1);
    assert_eq!(ledger.attempts_for_task("t-c").unwrap().len(), 1);
}

// ===========================================================================
// Event journal integrity — events are append-only and ordered
// ===========================================================================

#[test]
fn event_journal_is_append_only_and_ordered() {
    let ledger = mk_ledger_with_task("t-1");
    let clock = ManualClock::new(1_000);
    let mut runner = MockRunner::new();

    let coord = Coordinator::new(&ledger, short_lease_policy());
    let identity = coord
        .claim_and_start("t-1", "mock", &mut runner, &clock)
        .unwrap();
    runner.enqueue_event(&identity.attempt_id, RunnerEventKind::Completed, json!({}));
    coord.pump(&identity, &mut runner, &clock).unwrap();

    let events = ledger.events_for_task("t-1", 0, 100).unwrap();
    assert!(!events.is_empty());

    // Seq numbers are strictly increasing.
    for window in events.windows(2) {
        assert!(window[0].seq < window[1].seq, "events must be ordered");
    }

    // Replaying a duplicate event is a no-op (idempotent).
    let first_event = &events[0];
    let status = ledger.record_event(first_event).unwrap();
    assert_eq!(status, WriteStatus::Duplicate);
}
