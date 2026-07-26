//! O4 — Coordinator actor and event subscription.
//!
//! Wraps the O3 synchronous decision core in a long-lived actor: a
//! deterministic [`CoordinatorActor::tick_once`] for unit tests, and an async
//! [`CoordinatorActor::run`] loop with pause/resume/stop semantics for
//! production. Committed ledger events are forwarded to an [`EventSink`]
//! (Tauri `AppHandle` in production, a recording sink in tests).
//!
//! See `docs/AGENT_OPERATIONS_IMPLEMENTATION_PLAN.md` §A2.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time;

use super::coordinator::{Clock, Coordinator, CoordinatorError, CoordinatorPolicy, PumpOutcome};
use super::domain::{AttemptState, TaskState};
use super::ledger::{OrchestrationEvent, OrchestrationLedger};
use super::runners::{AttemptIdentity, RunnerAdapter};

const EVENT_CHANNEL: &str = "orchestration://event";
const DEFAULT_TICK_MS: u64 = 1_000;
const DEFAULT_MAX_EVENTS_PER_ATTEMPT: usize = 1_024;

// ---------------------------------------------------------------------------
// Event sinks
// ---------------------------------------------------------------------------

/// Receives every committed orchestration event the actor observes. The Tauri
/// implementation forwards to the renderer; tests use a recording sink.
pub trait EventSink: Send + Sync {
    fn deliver(&self, event: &OrchestrationEvent);
}

/// A sink that drops everything. Useful when event emission is disabled.
pub struct NullSink;

impl EventSink for NullSink {
    fn deliver(&self, _event: &OrchestrationEvent) {}
}

#[cfg(test)]
#[derive(Default)]
struct RecordingSink {
    events: std::sync::Mutex<Vec<OrchestrationEvent>>,
}

#[cfg(test)]
impl EventSink for RecordingSink {
    fn deliver(&self, event: &OrchestrationEvent) {
        self.events.lock().expect("sink").push(event.clone());
    }
}

/// Forwards orchestration events to the renderer via a Tauri `AppHandle`.
/// Not wired into a command yet (the native runner lands in O6); provided now
/// so the emission contract is exercised.
pub struct TauriEventSink {
    app: tauri::AppHandle,
}

impl TauriEventSink {
    pub fn new(app: tauri::AppHandle) -> Self {
        Self { app }
    }
}

impl EventSink for TauriEventSink {
    fn deliver(&self, event: &OrchestrationEvent) {
        use tauri::Emitter;
        let _ = self.app.emit(EVENT_CHANNEL, event);
    }
}

// ---------------------------------------------------------------------------
// Actor phase + commands
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorPhase {
    /// Accepting new claims and pumping attempts.
    Running,
    /// No new claims; active attempts keep running.
    Paused,
    /// Graceful shutdown requested.
    Stopped,
}

#[derive(Debug)]
pub enum ActorCommand {
    Pause,
    Resume,
    Stop,
}

/// A summary of one tick, returned for assertions and logging.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TickReport {
    pub reclaimed: usize,
    pub claimed: usize,
    pub pumped: usize,
    pub forwarded: usize,
}

/// Control handle returned by [`CoordinatorActor::spawn`]. Cloning is cheap
/// (it wraps an mpsc sender); the join handle is shared.
#[derive(Clone)]
pub struct ActorHandle {
    sender: mpsc::Sender<ActorCommand>,
    join: Arc<std::sync::Mutex<Option<JoinHandle<()>>>>,
}

impl ActorHandle {
    pub async fn pause(&self) -> Result<(), mpsc::error::SendError<ActorCommand>> {
        self.sender.send(ActorCommand::Pause).await
    }

    pub async fn resume(&self) -> Result<(), mpsc::error::SendError<ActorCommand>> {
        self.sender.send(ActorCommand::Resume).await
    }

    pub async fn stop(&self) -> Result<(), mpsc::error::SendError<ActorCommand>> {
        self.sender.send(ActorCommand::Stop).await
    }

    /// Wait for the actor task to finish after a stop. Only the first caller
    /// actually awaits the join handle.
    pub async fn join(self) {
        let join = self.join.lock().expect("join").take();
        if let Some(join) = join {
            let _ = join.await;
        }
    }
}

// ---------------------------------------------------------------------------
// Actor
// ---------------------------------------------------------------------------

/// One coordinator actor for a single workspace. Generic over the runner so
/// tests use [`MockRunner`](super::runners::mock::MockRunner) and production
/// uses the native runner (O6).
pub struct CoordinatorActor<R: RunnerAdapter + Send + 'static> {
    workspace_key: String,
    runner_kind: String,
    ledger: Arc<OrchestrationLedger>,
    runner: R,
    policy: CoordinatorPolicy,
    clock: super::coordinator::SystemClock,
    sink: Arc<dyn EventSink>,
    phase: ActorPhase,
    /// Stored runner handles for attempts started this session. For attempts
    /// recovered on restart the attempt id is used as the handle (the native
    /// runner reconciles its own handles in O6).
    handles: HashMap<String, String>,
    /// Per-task last-forwarded event seq, so events are emitted exactly once.
    cursors: HashMap<String, u64>,
    tick_ms: u64,
    max_events_per_attempt: usize,
}

impl<R: RunnerAdapter + Send + 'static> CoordinatorActor<R> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        workspace_key: impl Into<String>,
        runner_kind: impl Into<String>,
        ledger: Arc<OrchestrationLedger>,
        runner: R,
        policy: CoordinatorPolicy,
        sink: Arc<dyn EventSink>,
    ) -> Self {
        Self {
            workspace_key: workspace_key.into(),
            runner_kind: runner_kind.into(),
            ledger,
            runner,
            policy,
            clock: super::coordinator::SystemClock,
            sink,
            phase: ActorPhase::Stopped,
            handles: HashMap::new(),
            cursors: HashMap::new(),
            tick_ms: DEFAULT_TICK_MS,
            max_events_per_attempt: DEFAULT_MAX_EVENTS_PER_ATTEMPT,
        }
    }

    pub fn with_tick_interval(mut self, ms: u64) -> Self {
        self.tick_ms = ms.max(1);
        self
    }

    pub fn phase(&self) -> ActorPhase {
        self.phase
    }

    /// One deterministic tick. Pure function of ledger + runner state; safe to
    /// call repeatedly in tests.
    pub fn tick_once(&mut self) -> Result<TickReport, CoordinatorError> {
        // Destructure into disjoint field borrows so the ledger (held by `coord`)
        // and the runner (mutably pumped) can coexist.
        let Self {
            workspace_key,
            runner_kind,
            ledger,
            runner,
            policy,
            clock,
            sink,
            phase,
            handles,
            cursors,
            tick_ms: _,
            max_events_per_attempt,
        } = self;
        let ledger: &OrchestrationLedger = ledger;
        let coord = Coordinator::new(ledger, *policy);
        let mut report = TickReport::default();

        // 1. Reclaim lapsed leases (active-attempt management; runs even when
        //    paused so a stalled attempt is parked).
        report.reclaimed += coord.reclaim_expired_leases(&*clock)?.len();

        // 2. New claims are gated by the phase.
        let tasks = ledger.active_tasks(workspace_key)?;
        for task in &tasks {
            if *phase != ActorPhase::Running {
                break;
            }
            if is_claimable(task.state) {
                let identity =
                    coord.claim_and_start(&task.task_id, runner_kind, runner, &*clock)?;
                handles.insert(identity.attempt_id.clone(), identity.handle.clone());
                report.claimed += 1;
                report.pumped +=
                    pump_until_idle(runner, &coord, &*clock, &identity, *max_events_per_attempt)?;
            }
        }

        // 3. Pump any active attempt for tasks that are already Running.
        let active = ledger.active_tasks(workspace_key)?;
        for task in &active {
            if task.state == TaskState::Running {
                if let Some(attempt) = ledger.latest_attempt(&task.task_id)? {
                    // Pump only actively-running attempts; Stalled attempts are
                    // parked pending recovery (O5) and have no runner handle.
                    if !attempt.state.is_terminal() && attempt.state != AttemptState::Stalled {
                        let handle = handles
                            .get(&attempt.attempt_id)
                            .cloned()
                            .unwrap_or_else(|| attempt.attempt_id.clone());
                        let identity = AttemptIdentity {
                            attempt_id: attempt.attempt_id.clone(),
                            handle,
                        };
                        report.pumped += pump_until_idle(
                            runner,
                            &coord,
                            &*clock,
                            &identity,
                            *max_events_per_attempt,
                        )?;
                    }
                }
            }
        }

        // 4. Forward committed events to the sink (exactly once per event).
        for task in &active {
            let cursor = cursors.entry(task.task_id.clone()).or_insert(0);
            let after = *cursor;
            let events = ledger.events_for_task(&task.task_id, after, 512)?;
            for event in &events {
                sink.deliver(event);
                *cursor = (*cursor).max(event.seq);
                report.forwarded += 1;
            }
        }

        Ok(report)
    }

    /// Spawn the async drive loop. Returns a control handle; the actor runs
    /// until [`ActorHandle::stop`] is sent and awaited.
    pub fn spawn(mut self) -> ActorHandle {
        let (sender, mut receiver) = mpsc::channel::<ActorCommand>(16);
        let tick_ms = self.tick_ms;
        self.phase = ActorPhase::Running;
        let join = tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_millis(tick_ms));
            interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if self.phase == ActorPhase::Stopped {
                            break;
                        }
                        if self.phase == ActorPhase::Running {
                            let _ = self.tick_once();
                        }
                    }
                    cmd = receiver.recv() => {
                        match cmd {
                            Some(ActorCommand::Pause) => self.phase = ActorPhase::Paused,
                            Some(ActorCommand::Resume) => {
                                if self.phase == ActorPhase::Paused {
                                    self.phase = ActorPhase::Running;
                                }
                            }
                            Some(ActorCommand::Stop) | None => {
                                break;
                            }
                        }
                    }
                }
            }
        });
        ActorHandle {
            sender,
            join: Arc::new(std::sync::Mutex::new(Some(join))),
        }
    }
}

/// Drain runner events for one attempt until idle/terminal, capped to avoid
/// runaway loops on misbehaving runners.
fn pump_until_idle<R, C>(
    runner: &mut R,
    coord: &Coordinator<'_>,
    clock: &C,
    identity: &AttemptIdentity,
    max: usize,
) -> Result<usize, CoordinatorError>
where
    R: RunnerAdapter,
    C: Clock,
{
    let mut processed = 0usize;
    loop {
        if processed >= max {
            break;
        }
        match coord.pump(identity, runner, clock)? {
            PumpOutcome::Idle => break,
            PumpOutcome::Terminal(_) => {
                processed += 1;
                break;
            }
            PumpOutcome::Progressed(_) | PumpOutcome::RetryScheduled { .. } => processed += 1,
        }
    }
    Ok(processed)
}

fn is_claimable(state: TaskState) -> bool {
    matches!(state, TaskState::Queued | TaskState::Retrying)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::orchestration::coordinator::CoordinatorPolicy;
    use crate::modules::orchestration::domain::Lease;
    use crate::modules::orchestration::ledger::{CreateAttemptRequest, TaskRecord};
    use crate::modules::orchestration::runners::mock::MockRunner;
    use crate::modules::orchestration::runners::RunnerEventKind;

    fn seed_ledger() -> Arc<OrchestrationLedger> {
        let ledger = Arc::new(OrchestrationLedger::open_in_memory().expect("ledger"));
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

    fn actor(
        ledger: Arc<OrchestrationLedger>,
        runner: MockRunner,
        sink: Arc<RecordingSink>,
    ) -> CoordinatorActor<MockRunner> {
        let mut act = CoordinatorActor::new(
            "ws",
            "native",
            ledger,
            runner,
            CoordinatorPolicy::default(),
            sink,
        );
        // tick_once respects the phase; default to Running for deterministic
        // tests (spawn() does the same in production).
        act.phase = ActorPhase::Running;
        act
    }

    // --- Deterministic tick_once -------------------------------------------

    #[test]
    fn tick_once_claims_and_drains_a_task_to_completion() {
        let ledger = seed_ledger();
        let mut runner = MockRunner::new();
        runner.enqueue("t1-att-1", [RunnerEventKind::Completed]);
        let sink = Arc::new(RecordingSink::default());
        let mut act = actor(ledger.clone(), runner, sink.clone());

        let report = act.tick_once().expect("tick");
        assert!(report.claimed >= 1);
        // Completion moves the task to Verifying (never Done directly).
        assert_eq!(
            ledger.task("t1").unwrap().unwrap().state,
            TaskState::Verifying
        );
        assert_eq!(
            ledger.attempt("t1-att-1").unwrap().unwrap().state,
            AttemptState::Completed
        );
        // Events forwarded to the sink.
        assert!(!sink.events.lock().unwrap().is_empty());
    }

    #[test]
    fn paused_phase_blocks_new_claims() {
        let ledger = seed_ledger();
        let mut runner = MockRunner::new();
        runner.enqueue("t1-att-1", [RunnerEventKind::Completed]);
        let sink = Arc::new(RecordingSink::default());
        let mut act = actor(ledger.clone(), runner, sink);
        act.phase = ActorPhase::Paused;

        let report = act.tick_once().expect("tick");
        assert_eq!(report.claimed, 0);
        assert_eq!(ledger.task("t1").unwrap().unwrap().state, TaskState::Queued);
    }

    #[test]
    fn events_are_forwarded_exactly_once_across_ticks() {
        let ledger = seed_ledger();
        let mut runner = MockRunner::new();
        runner.enqueue("t1-att-1", [RunnerEventKind::Completed]);
        let sink = Arc::new(RecordingSink::default());
        let mut act = actor(ledger.clone(), runner, sink.clone());

        act.tick_once().expect("tick 1");
        let after_first = sink.events.lock().unwrap().len();
        // Second tick: nothing new (task is now Verifying, cursor advanced).
        act.tick_once().expect("tick 2");
        let after_second = sink.events.lock().unwrap().len();
        assert_eq!(after_first, after_second);
    }

    #[test]
    fn lapsed_lease_is_reclaimed_on_tick() {
        let ledger = seed_ledger();
        // Insert an attempt with an already-expired lease and a Running task.
        ledger
            .set_task_state("t1", TaskState::Running, "t1:running", 1_000)
            .unwrap();
        ledger
            .create_attempt(&CreateAttemptRequest {
                attempt_id: "t1-att-1".into(),
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
                "t1-att-1",
                AttemptState::Started,
                None,
                "t1-att-1:started",
                None,
                1_000,
            )
            .unwrap();

        let runner = MockRunner::new();
        let sink = Arc::new(RecordingSink::default());
        let mut act = actor(ledger.clone(), runner, sink);

        let report = act.tick_once().expect("tick");
        assert!(report.reclaimed >= 1, "{report:?}");
        assert_eq!(
            ledger.attempt("t1-att-1").unwrap().unwrap().state,
            AttemptState::Stalled
        );
    }

    // --- Async lifecycle ----------------------------------------------------

    async fn becomes(ledger: Arc<OrchestrationLedger>, task_id: &str, want: TaskState) {
        for _ in 0..200 {
            if ledger.task(task_id).expect("task").expect("present").state == want {
                return;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        let got = ledger.task(task_id).unwrap().unwrap().state;
        panic!("task {task_id} did not reach {want:?} (got {got:?})");
    }

    #[tokio::test]
    async fn spawn_drains_work_and_shuts_down_gracefully() {
        let ledger = seed_ledger();
        let mut runner = MockRunner::new();
        runner.enqueue(
            "t1-att-1",
            [RunnerEventKind::Started, RunnerEventKind::Completed],
        );
        let sink = Arc::new(RecordingSink::default());
        let act = actor(ledger.clone(), runner, sink).with_tick_interval(5);
        let handle = act.spawn();

        // The loop claims t1 and drains it to completion (Verifying).
        becomes(ledger.clone(), "t1", TaskState::Verifying).await;

        // Lifecycle commands are accepted and the task terminates on stop.
        // Pause blocking new claims is asserted deterministically by
        // `paused_phase_blocks_new_claims`.
        handle.pause().await.unwrap();
        handle.resume().await.unwrap();
        handle.stop().await.unwrap();
        handle.join().await;
    }
}
