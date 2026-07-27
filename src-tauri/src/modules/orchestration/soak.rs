//! Soak and integration tests for the Preview 1 exit gate (plan §10, §A3).
//!
//! These verify the critical correctness invariants currently supported by the
//! v2 orchestration core: concurrent duplicate-dispatch prevention, durable
//! ledger recovery, lease reclamation, and idempotent reconciliation. Native
//! runner reattachment after a full application-process restart is not claimed
//! here because `RunnerAdapter` does not yet expose a durable reconnect API.

#![cfg(test)]

use super::coordinator::{Coordinator, CoordinatorPolicy, ManualClock, PumpOutcome};
use super::domain::{AttemptState, Lease, TaskState};
use super::ledger::{OrchestrationLedger, TaskRecord, WriteStatus};
use super::recovery::{self, RecoveryReport};
use super::runners::mock::MockRunner;
use super::runners::{
    AttemptIdentity, AttemptSpec, RunnerAdapter, RunnerCapabilities, RunnerError, RunnerEvent,
    RunnerEventKind, RunnerResult,
};

use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, Barrier, Mutex};

fn mk_ledger_with_task(id: &str) -> OrchestrationLedger {
    let ledger = OrchestrationLedger::open_in_memory().unwrap();
    seed_task(&ledger, id);
    ledger
}

fn seed_task(ledger: &OrchestrationLedger, id: &str) {
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
}

fn short_lease_policy() -> CoordinatorPolicy {
    CoordinatorPolicy {
        lease_ttl_ms: 5_000,
        max_attempts: 3,
        ..CoordinatorPolicy::default()
    }
}

// ===========================================================================
// Coordinator restart after dispatch → reconnect without duplicate dispatch
// ===========================================================================

#[test]
fn coordinator_restart_reconnects_to_existing_run() {
    let ledger = mk_ledger_with_task("t-1");
    let clock = ManualClock::new(1_000);
    let mut runner = MockRunner::new();

    // Coordinator A dispatches the run, then disappears before it can pump
    // the completion event.
    let coord_a = Coordinator::new(&ledger, short_lease_policy());
    let identity = coord_a
        .claim_and_start("t-1", "mock", &mut runner, &clock)
        .unwrap();
    runner.enqueue_event(
        &identity.attempt_id,
        RunnerEventKind::Completed,
        json!({ "result": "success" }),
    );
    assert_eq!(runner.start_count(&identity.attempt_id), 1);
    drop(coord_a);

    // A fresh coordinator reconnects to the still-live runner identity.
    let coord_b = Coordinator::new(&ledger, short_lease_policy());
    let identity_b = coord_b
        .claim_and_start("t-1", "mock", &mut runner, &clock)
        .unwrap();

    assert_eq!(identity_b.attempt_id, identity.attempt_id);
    assert_eq!(
        runner.start_count(&identity.attempt_id),
        1,
        "reconnect must not dispatch the existing attempt again"
    );

    let outcome = coord_b.pump(&identity_b, &mut runner, &clock).unwrap();
    assert!(matches!(
        outcome,
        PumpOutcome::Terminal(AttemptState::Completed)
    ));
    assert_eq!(
        ledger.task("t-1").unwrap().unwrap().state,
        TaskState::Verifying
    );
    let attempts = ledger.attempts_for_task("t-1").unwrap();
    assert_eq!(attempts.len(), 1);
}

// ===========================================================================
// Preview 1 soak gate — 1,000 tasks, eight concurrent workers, no duplicates
// ===========================================================================

#[derive(Clone)]
struct CountingRunner {
    starts: Arc<Mutex<HashMap<String, usize>>>,
}

impl RunnerAdapter for CountingRunner {
    fn capabilities(&self) -> RunnerCapabilities {
        RunnerCapabilities::default()
    }

    fn start_attempt(&mut self, spec: &AttemptSpec) -> RunnerResult<AttemptIdentity> {
        let mut starts = self.starts.lock().expect("start counter");
        *starts.entry(spec.attempt_id.clone()).or_default() += 1;
        Ok(AttemptIdentity {
            attempt_id: spec.attempt_id.clone(),
            handle: spec.attempt_id.clone(),
        })
    }

    fn poll_event(&mut self, _identity: &AttemptIdentity) -> RunnerResult<Option<RunnerEvent>> {
        Ok(None)
    }

    fn steer(&mut self, _identity: &AttemptIdentity, _message: &str) -> RunnerResult<()> {
        Err(RunnerError::Unsupported { action: "steer" })
    }

    fn cancel(&mut self, _identity: &AttemptIdentity) -> RunnerResult<()> {
        Err(RunnerError::Unsupported { action: "cancel" })
    }

    fn shutdown(&mut self) {}
}

#[test]
fn thousand_tasks_eight_workers_have_zero_duplicate_dispatches() {
    const TASK_COUNT: usize = 1_000;
    const WORKER_COUNT: usize = 8;

    let ledger = Arc::new(OrchestrationLedger::open_in_memory().unwrap());
    for index in 0..TASK_COUNT {
        let task_id = format!("soak-{index}");
        ledger
            .upsert_task(&TaskRecord {
                task_id: task_id.clone(),
                workspace_key: "ws-soak".into(),
                source_kind: "local".into(),
                source_ref: task_id.clone(),
                title: task_id,
                description: "soak".into(),
                state: TaskState::Queued,
                created_at_ms: 1_000,
                updated_at_ms: 1_000,
            })
            .unwrap();
    }

    let starts = Arc::new(Mutex::new(HashMap::new()));
    let barrier = Arc::new(Barrier::new(WORKER_COUNT));
    let workers = (0..WORKER_COUNT)
        .map(|_| {
            let ledger = Arc::clone(&ledger);
            let starts = Arc::clone(&starts);
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                let coordinator = Coordinator::new(&ledger, short_lease_policy());
                let clock = ManualClock::new(1_000);
                let mut runner = CountingRunner { starts };
                for index in 0..TASK_COUNT {
                    barrier.wait();
                    coordinator
                        .claim_and_start(&format!("soak-{index}"), "counting", &mut runner, &clock)
                        .unwrap();
                }
            })
        })
        .collect::<Vec<_>>();

    for worker in workers {
        worker.join().expect("soak worker");
    }

    let starts = starts.lock().expect("start counter");
    assert_eq!(starts.len(), TASK_COUNT);
    assert_eq!(starts.values().sum::<usize>(), TASK_COUNT);
    for index in 0..TASK_COUNT {
        let task_id = format!("soak-{index}");
        let attempt_id = format!("{task_id}-att-1");
        assert_eq!(
            starts.get(&attempt_id),
            Some(&1),
            "{attempt_id} must be dispatched exactly once"
        );
        assert_eq!(
            ledger.attempts_for_task(&task_id).unwrap().len(),
            1,
            "{task_id} must have exactly one attempt"
        );
    }
}

// ===========================================================================
// Expired runner recovery → park ambiguous work for operator attention
// ===========================================================================

#[test]
fn expired_run_is_reclaimed_and_parked_for_attention() {
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
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("ledger.sqlite3");
    let clock = ManualClock::new(1_000);

    {
        let ledger = OrchestrationLedger::open(&path).unwrap();
        seed_task(&ledger, "t-1");
        // Simulate: attempt completed but task projection NOT yet updated
        // (crash between attempt terminal and task terminal).
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
        ledger
            .set_task_state("t-1", TaskState::Running, "t-1:running", 1_000)
            .unwrap();
    }

    // Reopen the durable ledger after the simulated process crash.
    let ledger = OrchestrationLedger::open(&path).unwrap();
    let report = recovery::run(&ledger, &clock).unwrap();
    assert_eq!(report.replays, 1);
    let task = ledger.task("t-1").unwrap().unwrap();
    assert_eq!(task.state, TaskState::Verifying);
    drop(ledger);

    // A second restart is idempotent.
    let reopened = OrchestrationLedger::open(&path).unwrap();
    let report2 = recovery::run(&reopened, &clock).unwrap();
    assert_eq!(report2.replays, 0);
}

// ===========================================================================
// Durable restart recovery is idempotent across database reopens
// ===========================================================================

#[test]
fn recovery_is_idempotent_across_database_reopens() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("ledger.sqlite3");
    {
        let ledger = OrchestrationLedger::open(&path).unwrap();
        seed_task(&ledger, "t-1");
        let clock = ManualClock::new(1_000);
        let mut runner = MockRunner::new();
        let coord = Coordinator::new(&ledger, short_lease_policy());
        coord
            .claim_and_start("t-1", "mock", &mut runner, &clock)
            .unwrap();
    }

    let clock = ManualClock::new(11_000);
    let ledger = OrchestrationLedger::open(&path).unwrap();
    let report1 = recovery::run(&ledger, &clock).unwrap();
    assert!(report1.leased_reclaimed > 0 || report1.needs_attention > 0);
    drop(ledger);

    let reopened = OrchestrationLedger::open(&path).unwrap();
    let report2 = recovery::run(&reopened, &clock).unwrap();
    assert_eq!(
        report2,
        RecoveryReport::default(),
        "recovery after a second process restart must be a no-op"
    );
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
