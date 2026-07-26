//! O3 — Coordinator decision core.
//!
//! The synchronous decision core that drives a [`RunnerAdapter`] against the
//! O2 ledger using the O1 domain model. It owns claim/start, event pumping,
//! cancellation, retry scheduling, and lost-lease recovery. The async loop and
//! Tauri event emission are added in O4; here the logic is fully deterministic
//! and unit-tested with the mock runner.
//!
//! Only the coordinator changes authoritative state (§2.2).

use super::domain::{
    AttemptState, AttemptTrigger, Lease, LeaseError, TaskState, TaskTrigger, TransitionError,
};
use super::ledger::{
    CreateAttemptRequest, LedgerError, OrchestrationEvent, OrchestrationLedger, WriteStatus,
};
use super::runners::{
    event_to_trigger, AttemptIdentity, AttemptSpec, RunnerAdapter, RunnerError, RunnerEventKind,
};
use std::cell::Cell;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

const COORDINATOR_OWNER: &str = "coordinator";

// ---------------------------------------------------------------------------
// Clock
// ---------------------------------------------------------------------------

/// Injectable time source so tests are deterministic.
pub trait Clock {
    fn now_ms(&self) -> u64;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .min(u64::MAX as u128) as u64
    }
}

/// A manually advanced clock for tests.
pub struct ManualClock {
    now: Cell<u64>,
}

impl ManualClock {
    pub fn new(start_ms: u64) -> Self {
        Self {
            now: Cell::new(start_ms),
        }
    }

    pub fn advance(&self, by_ms: u64) {
        self.now.set(self.now.get().saturating_add(by_ms));
    }

    pub fn set(&self, now_ms: u64) {
        self.now.set(now_ms);
    }
}

impl Clock for ManualClock {
    fn now_ms(&self) -> u64 {
        self.now.get()
    }
}

// ---------------------------------------------------------------------------
// Retry policy
// ---------------------------------------------------------------------------

/// Coordinator tuning: retry backoff, attempt cap, and lease lifetime.
#[derive(Debug, Clone, Copy)]
pub struct CoordinatorPolicy {
    pub retry_base_ms: u64,
    pub retry_max_ms: u64,
    pub max_attempts: u32,
    pub lease_ttl_ms: u64,
}

impl Default for CoordinatorPolicy {
    fn default() -> Self {
        Self {
            retry_base_ms: 5_000,
            retry_max_ms: 300_000,
            max_attempts: 4,
            lease_ttl_ms: 60_000,
        }
    }
}

impl CoordinatorPolicy {
    /// Exponential backoff for the attempt that just failed (1-based).
    pub fn retry_delay_ms(&self, failed_attempt_no: u32) -> u64 {
        let shift = failed_attempt_no.saturating_sub(1).min(20);
        self.retry_base_ms
            .saturating_mul(1u64 << shift)
            .min(self.retry_max_ms)
    }
}

// ---------------------------------------------------------------------------
// Outcomes & errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum PumpOutcome {
    /// The runner had no event ready.
    Idle,
    /// A non-terminal event advanced the attempt.
    Progressed(AttemptState),
    /// A terminal event resolved the attempt.
    Terminal(AttemptState),
    /// The attempt failed and a retry was scheduled for `retry_at_ms`.
    RetryScheduled { retry_at_ms: u64 },
}

#[derive(Debug)]
pub enum CoordinatorError {
    UnknownTask {
        task_id: String,
    },
    UnknownAttempt {
        attempt_id: String,
    },
    /// The task is not in a state the coordinator may claim.
    NotClaimable {
        task_id: String,
        state: TaskState,
    },
    NotCancellable {
        attempt_id: String,
    },
    EventAttemptMismatch {
        expected: String,
        actual: String,
    },
    InvalidTransition(TransitionError),
    Lease(LeaseError),
    Ledger(LedgerError),
    Runner(RunnerError),
}

impl fmt::Display for CoordinatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoordinatorError::UnknownTask { task_id } => {
                write!(f, "coordinator: unknown task {task_id}")
            }
            CoordinatorError::UnknownAttempt { attempt_id } => {
                write!(f, "coordinator: unknown attempt {attempt_id}")
            }
            CoordinatorError::NotClaimable { task_id, state } => write!(
                f,
                "coordinator: task {task_id} is not claimable from state `{}`",
                state.name()
            ),
            CoordinatorError::NotCancellable { attempt_id } => {
                write!(f, "coordinator: attempt {attempt_id} cannot be cancelled")
            }
            CoordinatorError::EventAttemptMismatch { expected, actual } => write!(
                f,
                "coordinator: runner event for attempt {actual} was returned while polling {expected}"
            ),
            CoordinatorError::InvalidTransition(error) => {
                write!(f, "coordinator: invalid transition: {error}")
            }
            CoordinatorError::Lease(error) => write!(f, "coordinator: {error}"),
            CoordinatorError::Ledger(error) => write!(f, "coordinator: {error}"),
            CoordinatorError::Runner(error) => write!(f, "coordinator: {error}"),
        }
    }
}

impl std::error::Error for CoordinatorError {}

impl From<LedgerError> for CoordinatorError {
    fn from(value: LedgerError) -> Self {
        Self::Ledger(value)
    }
}

impl From<RunnerError> for CoordinatorError {
    fn from(value: RunnerError) -> Self {
        Self::Runner(value)
    }
}

impl From<TransitionError> for CoordinatorError {
    fn from(value: TransitionError) -> Self {
        Self::InvalidTransition(value)
    }
}

impl From<LeaseError> for CoordinatorError {
    fn from(value: LeaseError) -> Self {
        Self::Lease(value)
    }
}

pub type CoordinatorResult<T> = Result<T, CoordinatorError>;

// ---------------------------------------------------------------------------
// Coordinator
// ---------------------------------------------------------------------------

/// Synchronous coordinator core. Borrows the ledger; the runner and clock are
/// passed per call so a single coordinator can drive many attempts.
pub struct Coordinator<'a> {
    ledger: &'a OrchestrationLedger,
    policy: CoordinatorPolicy,
}

impl<'a> Coordinator<'a> {
    pub fn new(ledger: &'a OrchestrationLedger, policy: CoordinatorPolicy) -> Self {
        Self { ledger, policy }
    }

    pub fn ledger(&self) -> &OrchestrationLedger {
        self.ledger
    }

    /// Claim an eligible task, create an attempt with a fresh lease, and start
    /// the runner. Returns the runner-side identity to pump later.
    pub fn claim_and_start<R, C>(
        &self,
        task_id: &str,
        runner_kind: &str,
        runner: &mut R,
        clock: &C,
    ) -> CoordinatorResult<AttemptIdentity>
    where
        R: RunnerAdapter,
        C: Clock,
    {
        let task = self
            .ledger
            .task(task_id)?
            .ok_or(CoordinatorError::UnknownTask {
                task_id: task_id.to_string(),
            })?;
        let now = clock.now_ms();
        // Resolve the authoritative task state into Running via the documented
        // path (Queued -> Planning -> Running, or Retrying -> Running).
        let running = advance_to_running(task.state).ok_or(CoordinatorError::NotClaimable {
            task_id: task_id.to_string(),
            state: task.state,
        })?;

        let latest = self.ledger.latest_attempt(task_id)?;
        // An active (non-terminal) attempt already owns this task: re-claim is
        // idempotent and must not create a second attempt or re-dispatch.
        if let Some(active) = latest.as_ref() {
            if !active.state.is_terminal() && active.state != AttemptState::Stalled {
                return Ok(AttemptIdentity {
                    attempt_id: active.attempt_id.clone(),
                    handle: active.attempt_id.clone(),
                });
            }
        }

        let attempt_no = latest
            .as_ref()
            .map(|attempt| attempt.attempt_no.saturating_add(1))
            .unwrap_or(1);

        let attempt_id = format!("{task_id}-att-{attempt_no}");
        let outcome = self.ledger.create_attempt(&CreateAttemptRequest {
            attempt_id: attempt_id.clone(),
            task_id: task_id.to_string(),
            attempt_no,
            runner_kind: runner_kind.to_string(),
            lease: Some(Lease {
                owner: COORDINATOR_OWNER.to_string(),
                generation: 1,
                expires_at_ms: now.saturating_add(self.policy.lease_ttl_ms),
            }),
            idempotency_key: format!("{task_id}:{attempt_no}:{runner_kind}"),
            now_ms: now,
        })?;
        // A duplicate idempotency key means a concurrent coordinator already
        // dispatched this attempt; return its identity without re-dispatching
        // to the runner (no duplicate work).
        if outcome.status == WriteStatus::Duplicate {
            return Ok(AttemptIdentity {
                attempt_id: outcome.attempt_id.clone(),
                handle: outcome.attempt_id.clone(),
            });
        }
        let attempt_id = outcome.attempt_id;

        self.ledger
            .set_task_state(task_id, running, &format!("{attempt_id}:task:running"), now)?;
        let identity = match runner.start_attempt(&AttemptSpec {
            task_id: task_id.to_string(),
            attempt_id: attempt_id.clone(),
            input: task.title.clone(),
        }) {
            Ok(identity) => identity,
            Err(error) => {
                let message = error.to_string();
                self.ledger.set_attempt_state(
                    &attempt_id,
                    AttemptState::Failed,
                    Some(&message),
                    &format!("{attempt_id}:start_failed"),
                    None,
                    now,
                )?;
                let task_state = if attempt_no < self.policy.max_attempts {
                    TaskState::Retrying
                } else {
                    TaskState::Failed
                };
                self.ledger.set_task_state(
                    task_id,
                    task_state,
                    &format!("{attempt_id}:task:start_failed"),
                    now,
                )?;
                return Err(CoordinatorError::Runner(error));
            }
        };
        self.ledger.set_attempt_state(
            &attempt_id,
            AttemptState::Started,
            None,
            &format!("{attempt_id}:started"),
            None,
            now,
        )?;
        Ok(identity)
    }

    /// Pump one runner event through the domain model and persist the result.
    pub fn pump<R, C>(
        &self,
        identity: &AttemptIdentity,
        runner: &mut R,
        clock: &C,
    ) -> CoordinatorResult<PumpOutcome>
    where
        R: RunnerAdapter,
        C: Clock,
    {
        let now = clock.now_ms();
        let Some(event) = runner.poll_event(identity)? else {
            return Ok(PumpOutcome::Idle);
        };
        if event.attempt_id != identity.attempt_id {
            return Err(CoordinatorError::EventAttemptMismatch {
                expected: identity.attempt_id.clone(),
                actual: event.attempt_id,
            });
        }
        let attempt =
            self.ledger
                .attempt(&identity.attempt_id)?
                .ok_or(CoordinatorError::UnknownAttempt {
                    attempt_id: identity.attempt_id.clone(),
                })?;

        if event.kind == RunnerEventKind::Output {
            self.ledger.record_event(&OrchestrationEvent {
                event_id: format!("{}:event:{}", identity.attempt_id, event.seq),
                task_id: attempt.task_id,
                seq: 0,
                kind: "attempt.output".to_string(),
                payload: serde_json::json!({
                    "attempt_id": identity.attempt_id,
                    "runner_seq": event.seq,
                    "payload": event.payload,
                }),
                recorded_at_ms: now,
            })?;
            return Ok(PumpOutcome::Progressed(attempt.state));
        }

        let new_state = match (&event.kind, event_to_trigger(&event.kind)) {
            (RunnerEventKind::Started, _) if attempt.state == AttemptState::Started => {
                AttemptState::Started
            }
            (_, Some(trigger)) => attempt.state.transition(trigger)?,
            (_, None) => attempt.state,
        };
        let outcome = outcome_payload(&event.kind);
        let renewed_lease = if event.kind == RunnerEventKind::Heartbeat {
            let lease = attempt.lease.as_ref().ok_or_else(|| {
                CoordinatorError::Ledger(LedgerError::LeaseMismatch {
                    attempt_id: identity.attempt_id.clone(),
                })
            })?;
            Some(lease.renew(
                COORDINATOR_OWNER,
                lease.generation,
                now,
                self.policy.lease_ttl_ms,
            )?)
        } else {
            None
        };
        self.ledger.set_attempt_state(
            &identity.attempt_id,
            new_state,
            outcome,
            &format!("{}:event:{}", identity.attempt_id, event.seq),
            renewed_lease.as_ref(),
            now,
        )?;

        if !new_state.is_terminal() {
            return Ok(PumpOutcome::Progressed(new_state));
        }

        // Terminal: react at the task level.
        match new_state {
            AttemptState::Completed => {
                // An attempt completion never directly produces Done (§5.3);
                // hand off to verification.
                self.ledger.set_task_state(
                    &attempt.task_id,
                    TaskState::Verifying,
                    &format!("{}:task:verifying", identity.attempt_id),
                    now,
                )?;
                Ok(PumpOutcome::Terminal(new_state))
            }
            AttemptState::Cancelled => {
                self.ledger.set_task_state(
                    &attempt.task_id,
                    TaskState::Cancelled,
                    &format!("{}:task:cancelled", identity.attempt_id),
                    now,
                )?;
                Ok(PumpOutcome::Terminal(new_state))
            }
            AttemptState::Failed => {
                if attempt.attempt_no < self.policy.max_attempts {
                    self.ledger.set_task_state(
                        &attempt.task_id,
                        TaskState::Retrying,
                        &format!("{}:task:retrying", identity.attempt_id),
                        now,
                    )?;
                    let retry_at =
                        now.saturating_add(self.policy.retry_delay_ms(attempt.attempt_no));
                    Ok(PumpOutcome::RetryScheduled {
                        retry_at_ms: retry_at,
                    })
                } else {
                    self.ledger.set_task_state(
                        &attempt.task_id,
                        TaskState::Failed,
                        &format!("{}:task:failed", identity.attempt_id),
                        now,
                    )?;
                    Ok(PumpOutcome::Terminal(new_state))
                }
            }
            // Stalled is non-terminal; reached only when the runner stalls, but
            // guard defensively.
            other => Ok(PumpOutcome::Terminal(other)),
        }
    }

    /// Request cancellation of an attempt. The runner is asked to cancel; the
    /// attempt moves to CancelRequested (unless already terminal).
    pub fn request_cancel<R, C>(
        &self,
        identity: &AttemptIdentity,
        runner: &mut R,
        clock: &C,
    ) -> CoordinatorResult<()>
    where
        R: RunnerAdapter,
        C: Clock,
    {
        let attempt =
            self.ledger
                .attempt(&identity.attempt_id)?
                .ok_or(CoordinatorError::UnknownAttempt {
                    attempt_id: identity.attempt_id.clone(),
                })?;
        if attempt.state.is_terminal() {
            return Err(CoordinatorError::NotCancellable {
                attempt_id: identity.attempt_id.clone(),
            });
        }
        runner.cancel(identity)?;
        let new_state = attempt.state.transition(AttemptTrigger::RequestCancel)?;
        self.ledger.set_attempt_state(
            &identity.attempt_id,
            new_state,
            None,
            &format!("{}:cancel_requested", identity.attempt_id),
            None,
            clock.now_ms(),
        )?;
        Ok(())
    }

    /// Reclaim attempts whose lease has lapsed without a terminal event
    /// (lost-lease recovery, §5.4). Each reclaimed attempt is parked in
    /// `Stalled` so the coordinator can retry or fail it on the next tick.
    pub fn reclaim_expired_leases<C>(&self, clock: &C) -> CoordinatorResult<Vec<String>>
    where
        C: Clock,
    {
        let now = clock.now_ms();
        let expired = self.ledger.expired_lease_attempts(now)?;
        let mut reclaimed = Vec::with_capacity(expired.len());
        for attempt in expired {
            // A lapsed lease is no longer authoritative; mark the attempt
            // stalled so a fresh claim/attempt can take over.
            self.ledger.set_attempt_state(
                &attempt.attempt_id,
                AttemptState::Stalled,
                None,
                &format!("{}:stalled:{now}", attempt.attempt_id),
                None,
                now,
            )?;
            reclaimed.push(attempt.attempt_id);
        }
        Ok(reclaimed)
    }
}

fn advance_to_running(state: TaskState) -> Option<TaskState> {
    use TaskState::*;
    use TaskTrigger::*;
    let mut s = state;
    loop {
        match s {
            Queued => s = s.transition(StartPlanning).ok()?,
            Planning => s = s.transition(StartRun).ok()?,
            Retrying => s = s.transition(TaskTrigger::Resume).ok()?,
            Running => return Some(Running),
            _ => return None,
        }
    }
}

fn outcome_payload(kind: &super::runners::RunnerEventKind) -> Option<&'static str> {
    use super::runners::RunnerEventKind::*;
    match kind {
        Completed => Some("completed"),
        Failed => Some("failed"),
        Cancelled => Some("cancelled"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::orchestration::ledger::TaskRecord;
    use crate::modules::orchestration::runners::mock::MockRunner;
    use crate::modules::orchestration::runners::RunnerEventKind;

    fn ledger() -> OrchestrationLedger {
        let ledger = OrchestrationLedger::open_in_memory().expect("ledger");
        ledger
            .upsert_task(&TaskRecord {
                task_id: "t1".into(),
                workspace_key: "ws".into(),
                source_kind: "local".into(),
                source_ref: "local://t1".into(),
                title: "Do the thing".into(),
                state: TaskState::Queued,
                created_at_ms: 1_000,
                updated_at_ms: 1_000,
            })
            .expect("seed task");
        ledger
    }

    fn coord<'a>(ledger: &'a OrchestrationLedger) -> Coordinator<'a> {
        Coordinator::new(ledger, CoordinatorPolicy::default())
    }

    // --- Success path -------------------------------------------------------

    #[test]
    fn claim_pump_to_completion_advances_task_to_verifying() {
        let ledger = ledger();
        let coord = coord(&ledger);
        let clock = ManualClock::new(2_000);
        let mut runner = MockRunner::new();
        runner.enqueue("t1-att-1", [RunnerEventKind::Completed]);

        let id = coord
            .claim_and_start("t1", "native", &mut runner, &clock)
            .expect("claim");
        assert!(runner.was_started("t1-att-1"));
        assert_eq!(
            ledger.task("t1").unwrap().unwrap().state,
            TaskState::Running
        );

        let outcome = coord.pump(&id, &mut runner, &clock).expect("pump");
        assert_eq!(outcome, PumpOutcome::Terminal(AttemptState::Completed));
        // Attempt completion does NOT produce Done; it hands off to Verifying.
        assert_eq!(
            ledger.task("t1").unwrap().unwrap().state,
            TaskState::Verifying
        );
        let attempt = ledger.attempt("t1-att-1").unwrap().unwrap();
        assert_eq!(attempt.state, AttemptState::Completed);
        assert_eq!(attempt.terminal_outcome.as_deref(), Some("completed"));
    }

    #[test]
    fn runner_started_event_is_an_idempotent_progress_signal() {
        let ledger = ledger();
        let coord = coord(&ledger);
        let clock = ManualClock::new(2_000);
        let mut runner = MockRunner::new();
        runner.enqueue("t1-att-1", [RunnerEventKind::Started]);

        let id = coord
            .claim_and_start("t1", "native", &mut runner, &clock)
            .expect("claim");
        assert_eq!(
            coord.pump(&id, &mut runner, &clock).expect("pump"),
            PumpOutcome::Progressed(AttemptState::Started)
        );
    }

    #[test]
    fn output_event_preserves_payload_without_changing_state() {
        let ledger = ledger();
        let coord = coord(&ledger);
        let clock = ManualClock::new(2_000);
        let mut runner = MockRunner::new();
        runner.enqueue("t1-att-1", [RunnerEventKind::Output]);

        let id = coord
            .claim_and_start("t1", "native", &mut runner, &clock)
            .expect("claim");
        assert_eq!(
            coord.pump(&id, &mut runner, &clock).expect("pump"),
            PumpOutcome::Progressed(AttemptState::Started)
        );
        let events = ledger.events_for_task("t1", 0, 20).expect("events");
        assert!(events.iter().any(|event| event.kind == "attempt.output"));
        assert_eq!(
            ledger.attempt("t1-att-1").unwrap().unwrap().state,
            AttemptState::Started
        );
    }

    #[test]
    fn heartbeat_renews_the_lease_and_may_repeat() {
        let ledger = ledger();
        let coord = coord(&ledger);
        let clock = ManualClock::new(2_000);
        let mut runner = MockRunner::new();
        runner.enqueue(
            "t1-att-1",
            [RunnerEventKind::Heartbeat, RunnerEventKind::Heartbeat],
        );

        let id = coord
            .claim_and_start("t1", "native", &mut runner, &clock)
            .expect("claim");
        clock.set(3_000);
        coord.pump(&id, &mut runner, &clock).expect("heartbeat 1");
        assert_eq!(
            ledger
                .attempt("t1-att-1")
                .unwrap()
                .unwrap()
                .lease
                .unwrap()
                .expires_at_ms,
            63_000
        );
        clock.set(4_000);
        coord.pump(&id, &mut runner, &clock).expect("heartbeat 2");
        assert_eq!(
            ledger
                .attempt("t1-att-1")
                .unwrap()
                .unwrap()
                .lease
                .unwrap()
                .expires_at_ms,
            64_000
        );
    }

    // --- Failure + retry ----------------------------------------------------

    #[test]
    fn failure_with_retries_left_schedules_retry() {
        let ledger = ledger();
        let coord = coord(&ledger);
        let clock = ManualClock::new(2_000);
        let mut runner = MockRunner::new();
        runner.enqueue("t1-att-1", [RunnerEventKind::Failed]);

        let id = coord
            .claim_and_start("t1", "native", &mut runner, &clock)
            .expect("claim");
        let outcome = coord.pump(&id, &mut runner, &clock).expect("pump");
        let PumpOutcome::RetryScheduled { retry_at_ms } = outcome else {
            panic!("expected retry scheduled, got {outcome:?}");
        };
        // default base 5s for the first failure.
        assert_eq!(retry_at_ms, 2_000 + 5_000);
        assert_eq!(
            ledger.task("t1").unwrap().unwrap().state,
            TaskState::Retrying
        );
        assert_eq!(
            ledger.attempt("t1-att-1").unwrap().unwrap().state,
            AttemptState::Failed
        );

        // Re-claiming creates a NEW attempt identity (retry).
        runner.enqueue("t1-att-2", [RunnerEventKind::Completed]);
        let id2 = coord
            .claim_and_start("t1", "native", &mut runner, &clock)
            .expect("re-claim");
        assert_eq!(id2.attempt_id, "t1-att-2");
        let outcome = coord.pump(&id2, &mut runner, &clock).expect("pump 2");
        assert_eq!(outcome, PumpOutcome::Terminal(AttemptState::Completed));
        assert_eq!(
            ledger.task("t1").unwrap().unwrap().state,
            TaskState::Verifying
        );
        // History retained.
        assert_eq!(ledger.attempts_for_task("t1").unwrap().len(), 2);
    }

    #[test]
    fn failure_after_max_attempts_fails_the_task() {
        let ledger = ledger();
        let coord = Coordinator::new(
            &ledger,
            CoordinatorPolicy {
                max_attempts: 1,
                ..CoordinatorPolicy::default()
            },
        );
        let clock = ManualClock::new(2_000);
        let mut runner = MockRunner::new();
        runner.enqueue("t1-att-1", [RunnerEventKind::Failed]);

        let id = coord
            .claim_and_start("t1", "native", &mut runner, &clock)
            .expect("claim");
        let outcome = coord.pump(&id, &mut runner, &clock).expect("pump");
        assert_eq!(outcome, PumpOutcome::Terminal(AttemptState::Failed));
        assert_eq!(ledger.task("t1").unwrap().unwrap().state, TaskState::Failed);
    }

    #[test]
    fn runner_start_failure_is_persisted_and_can_be_retried() {
        let ledger = ledger();
        let coord = coord(&ledger);
        let clock = ManualClock::new(2_000);
        let mut runner = MockRunner::new();
        runner.fail_next_start("runner unavailable");

        assert!(matches!(
            coord.claim_and_start("t1", "native", &mut runner, &clock),
            Err(CoordinatorError::Runner(RunnerError::Other(message)))
                if message == "runner unavailable"
        ));
        let failed = ledger.attempt("t1-att-1").unwrap().unwrap();
        assert_eq!(failed.state, AttemptState::Failed);
        assert_eq!(
            failed.terminal_outcome.as_deref(),
            Some("runner error: runner unavailable")
        );
        assert_eq!(
            ledger.task("t1").unwrap().unwrap().state,
            TaskState::Retrying
        );

        let retry = coord
            .claim_and_start("t1", "native", &mut runner, &clock)
            .expect("retry");
        assert_eq!(retry.attempt_id, "t1-att-2");
    }

    // --- Cancellation -------------------------------------------------------

    #[test]
    fn cancellation_request_then_runner_cancelled() {
        let ledger = ledger();
        let coord = coord(&ledger);
        let clock = ManualClock::new(2_000);
        let mut runner = MockRunner::new();
        runner.enqueue("t1-att-1", [RunnerEventKind::Cancelled]);

        let id = coord
            .claim_and_start("t1", "native", &mut runner, &clock)
            .expect("claim");
        coord
            .request_cancel(&id, &mut runner, &clock)
            .expect("cancel request");
        assert!(runner.was_cancelled("t1-att-1"));
        assert_eq!(
            ledger.attempt("t1-att-1").unwrap().unwrap().state,
            AttemptState::CancelRequested
        );

        let outcome = coord.pump(&id, &mut runner, &clock).expect("pump");
        assert_eq!(outcome, PumpOutcome::Terminal(AttemptState::Cancelled));
        assert_eq!(
            ledger.task("t1").unwrap().unwrap().state,
            TaskState::Cancelled
        );
    }

    #[test]
    fn cannot_cancel_terminal_attempt() {
        let ledger = ledger();
        let coord = coord(&ledger);
        let clock = ManualClock::new(2_000);
        let mut runner = MockRunner::new();
        runner.enqueue("t1-att-1", [RunnerEventKind::Completed]);
        let id = coord
            .claim_and_start("t1", "native", &mut runner, &clock)
            .expect("claim");
        coord.pump(&id, &mut runner, &clock).expect("complete");
        assert!(matches!(
            coord.request_cancel(&id, &mut runner, &clock),
            Err(CoordinatorError::NotCancellable { .. })
        ));
        assert!(!runner.was_cancelled("t1-att-1"));
    }

    // --- Lost-lease recovery ------------------------------------------------

    #[test]
    fn expired_lease_is_reclaimed_as_stalled() {
        let ledger = ledger();
        let coord = coord(&ledger);
        let clock = ManualClock::new(1_000);
        let mut runner = MockRunner::new();
        // Started but never emits a terminal event.
        runner.enqueue("t1-att-1", []);

        let id = coord
            .claim_and_start("t1", "native", &mut runner, &clock)
            .expect("claim");
        let _ = id;

        // Reaching the exact lease expiry is enough to reclaim it.
        clock.set(1_000 + 60_000);
        let reclaimed = coord.reclaim_expired_leases(&clock).expect("reclaim");
        assert_eq!(reclaimed, vec!["t1-att-1".to_string()]);
        assert_eq!(
            ledger.attempt("t1-att-1").unwrap().unwrap().state,
            AttemptState::Stalled
        );
        assert!(ledger.attempt("t1-att-1").unwrap().unwrap().lease.is_none());
        assert!(coord
            .reclaim_expired_leases(&clock)
            .expect("second reclaim")
            .is_empty());

        runner.enqueue("t1-att-2", []);
        let retry = coord
            .claim_and_start("t1", "native", &mut runner, &clock)
            .expect("fresh claim");
        assert_eq!(retry.attempt_id, "t1-att-2");
    }

    // --- Idempotent re-claim does not double-dispatch -----------------------

    #[test]
    fn duplicate_claim_returns_existing_attempt_identity() {
        let ledger = ledger();
        let coord = coord(&ledger);
        let clock = ManualClock::new(2_000);
        let mut runner = MockRunner::new();
        runner.enqueue("t1-att-1", [RunnerEventKind::Completed]);

        let first = coord
            .claim_and_start("t1", "native", &mut runner, &clock)
            .expect("claim 1");
        // Same idempotency key (task+no+runner) -> ledger returns Duplicate.
        let second = coord
            .claim_and_start("t1", "native", &mut runner, &clock)
            .expect("claim 2");
        assert_eq!(first.attempt_id, "t1-att-1");
        assert_eq!(second.attempt_id, "t1-att-1");
        assert_eq!(ledger.attempts_for_task("t1").unwrap().len(), 1);
    }

    // --- Determinism helpers ------------------------------------------------

    #[test]
    fn retry_backoff_is_bounded_and_exponential() {
        let policy = CoordinatorPolicy::default();
        assert_eq!(policy.retry_delay_ms(1), 5_000);
        assert_eq!(policy.retry_delay_ms(2), 10_000);
        assert_eq!(policy.retry_delay_ms(3), 20_000);
        assert_eq!(policy.retry_delay_ms(99), policy.retry_max_ms);
    }

    #[test]
    fn non_claimable_task_is_rejected() {
        let ledger = ledger();
        // Move task to Done so it is not claimable.
        ledger
            .set_task_state("t1", TaskState::Verifying, "t1:verifying", 1_500)
            .unwrap();
        ledger
            .set_task_state("t1", TaskState::Reviewing, "t1:reviewing", 1_600)
            .unwrap();
        ledger
            .set_task_state(
                "t1",
                TaskState::ReadyForHandoff,
                "t1:ready_for_handoff",
                1_700,
            )
            .unwrap();
        ledger
            .set_task_state("t1", TaskState::Done, "t1:done", 1_800)
            .unwrap();
        let coord = coord(&ledger);
        let clock = ManualClock::new(2_000);
        let mut runner = MockRunner::new();
        assert!(matches!(
            coord.claim_and_start("t1", "native", &mut runner, &clock),
            Err(CoordinatorError::NotClaimable { .. })
        ));
    }
}
