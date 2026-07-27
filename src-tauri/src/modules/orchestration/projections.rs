//! Read-model projections derived from the durable event ledger (plan §4.1).
//!
//! Projections are pure queries: they read the ledger and build higher-level
//! views (per-task summaries, aggregate metrics, audit trails) without mutating
//! anything. They power dashboards/UI and feed the success metrics (§11). All
//! derive `Serialize` so they can be returned from a Tauri command directly.

use std::collections::BTreeMap;

use serde::Serialize;

use super::domain::{AttemptState, TaskState};
use super::ledger::{
    AttemptRecord, LedgerResult, OrchestrationEvent, OrchestrationLedger, TaskRecord,
};

/// Per-task read-model: the current task state plus a summary of its attempt
/// history. One row per task in a dashboard.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskProjection {
    pub task_id: String,
    pub title: String,
    pub workspace_key: String,
    pub source_kind: String,
    pub state: TaskState,
    pub attempt_count: u32,
    /// The latest attempt's state (None if the task was never attempted).
    pub last_attempt_state: Option<AttemptState>,
    /// Terminal outcome of the latest attempt, if it reached one.
    pub last_attempt_outcome: Option<String>,
    pub last_activity_ms: u64,
    pub created_at_ms: u64,
}

/// Aggregate metrics over a workspace's tasks and attempts (§11 success metrics).
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricsSnapshot {
    pub total_tasks: usize,
    /// Task counts grouped by domain state name.
    pub tasks_by_state: BTreeMap<String, usize>,
    pub total_attempts: usize,
    /// Attempt counts grouped by attempt state name.
    pub attempts_by_state: BTreeMap<String, usize>,
    /// Terminal attempt tallies for quick success/failure reads.
    pub completed: usize,
    pub failed: usize,
    pub cancelled: usize,
    pub abandoned: usize,
    /// Completed / (Completed + Failed + Cancelled + Abandoned). None when no
    /// terminal attempts exist yet.
    pub success_rate: Option<f64>,
}

/// Build the projection for a single task, enriching it with its latest attempt.
pub fn task_projection(
    ledger: &OrchestrationLedger,
    task_id: &str,
) -> LedgerResult<Option<TaskProjection>> {
    let Some(task) = ledger.task(task_id)? else {
        return Ok(None);
    };
    Ok(Some(project_task(ledger, &task)?))
}

/// Build projections for every task in a workspace, ordered by task id.
pub fn task_projections(
    ledger: &OrchestrationLedger,
    workspace_key: &str,
) -> LedgerResult<Vec<TaskProjection>> {
    let tasks = ledger.tasks_for_workspace(workspace_key)?;
    let mut out = Vec::with_capacity(tasks.len());
    for task in tasks {
        out.push(project_task(ledger, &task)?);
    }
    Ok(out)
}

/// Aggregate metrics over a workspace's tasks and their attempts.
pub fn workspace_metrics(
    ledger: &OrchestrationLedger,
    workspace_key: &str,
) -> LedgerResult<MetricsSnapshot> {
    let tasks = ledger.tasks_for_workspace(workspace_key)?;
    let mut snap = MetricsSnapshot {
        total_tasks: tasks.len(),
        ..MetricsSnapshot::default()
    };
    for task in &tasks {
        *snap
            .tasks_by_state
            .entry(task.state.name().to_string())
            .or_default() += 1;
        for attempt in ledger.attempts_for_task(&task.task_id)? {
            snap.total_attempts += 1;
            *snap
                .attempts_by_state
                .entry(attempt.state.name().to_string())
                .or_default() += 1;
            if let Some(outcome) = &attempt.terminal_outcome {
                count_terminal(&mut snap, &attempt.state, outcome);
            }
        }
    }
    snap.success_rate = success_rate(snap.completed, snap.failed, snap.cancelled, snap.abandoned);
    Ok(snap)
}

/// The chronological audit trail for a task (its full event history).
pub fn audit_trail(
    ledger: &OrchestrationLedger,
    task_id: &str,
    limit: usize,
) -> LedgerResult<Vec<OrchestrationEvent>> {
    ledger.events_for_task(task_id, 0, limit)
}

fn project_task(ledger: &OrchestrationLedger, task: &TaskRecord) -> LedgerResult<TaskProjection> {
    let attempts = ledger.attempts_for_task(&task.task_id)?;
    let attempt_count = u32::try_from(attempts.len()).unwrap_or(u32::MAX);
    let (last_attempt_state, last_attempt_outcome, last_activity_ms) = attempts
        .iter()
        .max_by_key(|a| a.attempt_no)
        .map(|a| {
            (
                Some(a.state),
                a.terminal_outcome.clone(),
                activity_ms(task, a),
            )
        })
        .unwrap_or_else(|| (None, None, task.updated_at_ms));
    Ok(TaskProjection {
        task_id: task.task_id.clone(),
        title: task.title.clone(),
        workspace_key: task.workspace_key.clone(),
        source_kind: task.source_kind.clone(),
        state: task.state,
        attempt_count,
        last_attempt_state,
        last_attempt_outcome,
        last_activity_ms,
        created_at_ms: task.created_at_ms,
    })
}

fn activity_ms(task: &TaskRecord, attempt: &AttemptRecord) -> u64 {
    attempt
        .terminal_at_ms
        .or(attempt.heartbeat_ms)
        .or(attempt.started_at_ms)
        .unwrap_or(task.updated_at_ms)
}

fn count_terminal(snap: &mut MetricsSnapshot, state: &AttemptState, outcome: &str) {
    // Count by the authoritative attempt state; the outcome string is kept for
    // diagnostics but the tallies use the domain state so they sum cleanly.
    match state {
        AttemptState::Completed => snap.completed += 1,
        AttemptState::Failed => snap.failed += 1,
        AttemptState::Cancelled => snap.cancelled += 1,
        _ => {
            // A terminal_outcome on a non-terminal-named state is unexpected;
            // attribute it to failed so the denominator stays honest.
            let _ = outcome;
            snap.failed += 1;
        }
    }
}

fn success_rate(
    completed: usize,
    failed: usize,
    cancelled: usize,
    abandoned: usize,
) -> Option<f64> {
    let denom = completed + failed + cancelled + abandoned;
    if denom == 0 {
        None
    } else {
        Some(completed as f64 / denom as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::orchestration::domain::{AttemptState, Lease, TaskState};
    use crate::modules::orchestration::ledger::{
        CreateAttemptRequest, OrchestrationLedger, TaskRecord,
    };

    fn mk_task(id: &str, ws: &str, title: &str, state: TaskState) -> TaskRecord {
        TaskRecord {
            task_id: id.into(),
            workspace_key: ws.into(),
            source_kind: "local".into(),
            source_ref: id.into(),
            title: title.into(),
            description: String::new(),
            state,
            created_at_ms: 1_000,
            updated_at_ms: 1_000,
        }
    }

    fn ledger_with_task(state: TaskState) -> OrchestrationLedger {
        let ledger = OrchestrationLedger::open_in_memory().unwrap();
        ledger
            .upsert_task(&mk_task("t-1", "ws-1", "Fix login", state))
            .unwrap();
        ledger
    }

    fn start_attempt(ledger: &OrchestrationLedger, no: u32) {
        ledger
            .create_attempt(&CreateAttemptRequest {
                attempt_id: format!("t-1-att-{no}"),
                task_id: "t-1".into(),
                attempt_no: no,
                runner_kind: "native".into(),
                lease: Some(Lease {
                    owner: "coordinator".into(),
                    generation: 1,
                    expires_at_ms: 9_999,
                }),
                idempotency_key: format!("t-1:{no}"),
                now_ms: 2_000,
            })
            .unwrap();
    }

    #[test]
    fn task_projection_summarizes_history() {
        let ledger = ledger_with_task(TaskState::Queued);
        // Advance to Planning, start an attempt.
        ledger
            .set_task_state("t-1", TaskState::Planning, "e1", 1_100)
            .unwrap();
        start_attempt(&ledger, 1);

        let proj = task_projection(&ledger, "t-1").unwrap().expect("present");
        assert_eq!(proj.task_id, "t-1");
        assert_eq!(proj.title, "Fix login");
        assert_eq!(proj.attempt_count, 1);
        assert!(proj.last_attempt_state.is_some());
    }

    #[test]
    fn task_projection_none_for_unknown_task() {
        let ledger = OrchestrationLedger::open_in_memory().unwrap();
        assert!(task_projection(&ledger, "nope").unwrap().is_none());
    }

    #[test]
    fn task_projections_lists_workspace_tasks() {
        let ledger = OrchestrationLedger::open_in_memory().unwrap();
        ledger
            .upsert_task(&mk_task("t-1", "ws-1", "A", TaskState::Queued))
            .unwrap();
        ledger
            .upsert_task(&mk_task("t-2", "ws-1", "B", TaskState::Queued))
            .unwrap();
        // A different workspace is excluded.
        ledger
            .upsert_task(&mk_task("other", "ws-2", "C", TaskState::Queued))
            .unwrap();

        let projs = task_projections(&ledger, "ws-1").unwrap();
        assert_eq!(projs.len(), 2);
        // Ordered by task id.
        assert_eq!(projs[0].task_id, "t-1");
        assert_eq!(projs[1].task_id, "t-2");
    }

    #[test]
    fn metrics_aggregate_terminal_attempts() {
        let ledger = ledger_with_task(TaskState::Queued);
        // Attempt 1: completes.
        start_attempt(&ledger, 1);
        ledger
            .set_attempt_state(
                "t-1-att-1",
                AttemptState::Completed,
                Some("ok"),
                "e-c1",
                None,
                3_000,
            )
            .unwrap();
        // Attempt 2: fails.
        start_attempt(&ledger, 2);
        ledger
            .set_attempt_state(
                "t-1-att-2",
                AttemptState::Failed,
                Some("boom"),
                "e-f2",
                None,
                4_000,
            )
            .unwrap();

        let snap = workspace_metrics(&ledger, "ws-1").unwrap();
        assert_eq!(snap.total_tasks, 1);
        assert_eq!(snap.total_attempts, 2);
        assert_eq!(snap.completed, 1);
        assert_eq!(snap.failed, 1);
        // success_rate = 1 / (1 + 1) = 0.5
        assert_eq!(snap.success_rate, Some(0.5));
    }

    #[test]
    fn metrics_empty_workspace_has_no_success_rate() {
        let ledger = OrchestrationLedger::open_in_memory().unwrap();
        let snap = workspace_metrics(&ledger, "ws-empty").unwrap();
        assert_eq!(snap.total_tasks, 0);
        assert_eq!(snap.success_rate, None);
    }

    #[test]
    fn audit_trail_returns_events_in_order() {
        let ledger = ledger_with_task(TaskState::Queued);
        ledger
            .set_task_state("t-1", TaskState::Planning, "e1", 1_100)
            .unwrap();
        ledger
            .set_task_state("t-1", TaskState::Running, "e2", 1_200)
            .unwrap();

        let trail = audit_trail(&ledger, "t-1", 100).unwrap();
        assert_eq!(trail.len(), 2);
        assert_eq!(trail[0].seq, 1);
        assert_eq!(trail[1].seq, 2);
    }

    #[test]
    fn tasks_by_state_grouping() {
        let ledger = OrchestrationLedger::open_in_memory().unwrap();
        ledger
            .upsert_task(&mk_task("a", "ws", "A", TaskState::Queued))
            .unwrap();
        ledger
            .upsert_task(&mk_task("b", "ws", "B", TaskState::Queued))
            .unwrap();
        ledger
            .upsert_task(&mk_task("c", "ws", "C", TaskState::Done))
            .unwrap();

        let snap = workspace_metrics(&ledger, "ws").unwrap();
        assert_eq!(snap.tasks_by_state.get("queued"), Some(&2));
        assert_eq!(snap.tasks_by_state.get("done"), Some(&1));
    }
}
