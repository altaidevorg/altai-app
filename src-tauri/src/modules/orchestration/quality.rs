//! Quality dashboard metrics (plan §H4).
//!
//! Computes the full set of quality metrics from committed ledger data.
//! All metrics are read-only — dashboards never affect orchestration
//! correctness. No raw prompts or credentials are included in metrics.

use std::collections::HashMap;

use serde::Serialize;

use super::domain::TaskState;
use super::ledger::{ApprovalState, AttemptRecord, LedgerResult, OrchestrationLedger, TaskRecord};

// ---------------------------------------------------------------------------
// Quality metrics
// ---------------------------------------------------------------------------

/// The full quality dashboard snapshot for a workspace.
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QualityMetrics {
    /// Total number of tasks considered.
    pub total_tasks: usize,
    /// Tasks that reached Done on the first attempt (no retries).
    pub first_attempt_success: usize,
    /// Tasks that required at least one retry.
    pub tasks_with_retries: usize,
    /// Tasks that ended in Abandoned state.
    pub abandoned: usize,
    /// Percentage helpers.
    pub first_attempt_success_rate: Option<f64>,
    pub retry_rate: Option<f64>,
    pub abandonment_rate: Option<f64>,

    /// Duplicate dispatch attempts detected (same idempotency key replayed).
    pub duplicate_dispatch_count: usize,

    /// Recovery: tasks that were in NeedsAttention and later reached a
    /// terminal state (Done, Cancelled, Failed, Abandoned).
    pub recovery_attempts: usize,
    pub recovery_successes: usize,
    pub recovery_success_rate: Option<f64>,

    /// Median time from task creation to first attempt start (ms).
    pub median_time_to_first_activity_ms: Option<u64>,
    /// Median time from task creation to handoff (terminal state) (ms).
    pub median_time_to_handoff_ms: Option<u64>,

    /// Attempts that reached Verifying but then went backwards (failed verify).
    pub verification_failures: usize,
    pub verification_failure_rate: Option<f64>,

    /// Human approvals requested / granted / denied.
    pub approvals_requested: usize,
    pub approvals_granted: usize,
    pub approvals_denied: usize,
    pub approvals_expired: usize,
    pub steering_frequency: usize,

    /// Stale workspace: tasks that are non-terminal and haven't been updated
    /// in the given threshold.
    pub stale_task_count: usize,
    /// Threshold used for staleness (ms).
    pub stale_threshold_ms: u64,

    /// Computed at timestamp.
    pub computed_at_ms: u64,
}

// ---------------------------------------------------------------------------
// Compute
// ---------------------------------------------------------------------------

/// Compute the full quality dashboard for a workspace.
///
/// `stale_threshold_ms` defines how long a non-terminal task can go without
/// activity before it's counted as stale (e.g., 24 * 3600 * 1000 for 24h).
pub fn compute_quality_metrics(
    ledger: &OrchestrationLedger,
    workspace_key: &str,
    stale_threshold_ms: u64,
) -> LedgerResult<QualityMetrics> {
    let tasks = ledger.tasks_for_workspace(workspace_key)?;
    let now = now_ms();

    let mut metrics = QualityMetrics {
        total_tasks: tasks.len(),
        stale_threshold_ms,
        computed_at_ms: now,
        ..QualityMetrics::default()
    };

    let terminal_states = [
        TaskState::Done,
        TaskState::Failed,
        TaskState::Cancelled,
        TaskState::Abandoned,
    ];

    let mut first_activity_durations: Vec<u64> = Vec::new();
    let mut handoff_durations: Vec<u64> = Vec::new();
    let mut seen_idempotency_keys: HashMap<String, usize> = HashMap::new();
    let mut recovery_attempted = false;

    for task in &tasks {
        let attempts = ledger.attempts_for_task(&task.task_id)?;

        // --- first attempt success ---
        let terminal = terminal_states.contains(&task.state);
        if task.state == TaskState::Done && attempts.len() == 1 {
            metrics.first_attempt_success += 1;
        }

        // --- retries ---
        if attempts.len() > 1 {
            metrics.tasks_with_retries += 1;
        }

        // --- abandoned ---
        if task.state == TaskState::Abandoned {
            metrics.abandoned += 1;
        }

        // --- duplicate dispatch ---
        for att in &attempts {
            *seen_idempotency_keys
                .entry(att.idempotency_key.clone())
                .or_default() += 1;
        }

        // --- time to first activity ---
        if let Some(first) = attempts.iter().min_by_key(|a| a.attempt_no) {
            if let Some(started) = first.started_at_ms {
                if started >= task.created_at_ms {
                    first_activity_durations.push(started - task.created_at_ms);
                }
            }
        }

        // --- time to handoff ---
        if terminal {
            if let Some(last) = attempts.iter().max_by_key(|a| a.attempt_no) {
                if let Some(terminal_at) = last.terminal_at_ms {
                    if terminal_at >= task.created_at_ms {
                        handoff_durations.push(terminal_at - task.created_at_ms);
                    }
                }
            }
        }

        // --- verification failures ---
        // An attempt that reached Completed but the task didn't end in Done
        // indicates a verification failure.
        let had_completed_attempt = attempts
            .iter()
            .any(|a| a.state == super::domain::AttemptState::Completed);
        if had_completed_attempt && task.state != TaskState::Done && terminal {
            metrics.verification_failures += 1;
        }

        // --- recovery tracking ---
        // Look at events to detect NeedsAttention in the history.
        let events = ledger.events_for_task(&task.task_id, 0, 500)?;
        let had_needs_attention = events
            .iter()
            .any(|e| e.kind.contains("needs_attention") || e.kind.contains("NeedsAttention"));
        if had_needs_attention {
            metrics.recovery_attempts += 1;
            recovery_attempted = true;
            if terminal {
                metrics.recovery_successes += 1;
            }
        }

        // --- steering frequency ---
        // Count CancelRequested events as steering interventions.
        for e in &events {
            if e.kind.contains("cancel") || e.kind.contains("steer") {
                metrics.steering_frequency += 1;
            }
        }

        // --- approvals ---
        let approvals = ledger.approvals_for_task(&task.task_id)?;
        for ap in &approvals {
            metrics.approvals_requested += 1;
            match ap.state {
                ApprovalState::Approved => metrics.approvals_granted += 1,
                ApprovalState::Denied => metrics.approvals_denied += 1,
                ApprovalState::Expired => metrics.approvals_expired += 1,
                _ => {}
            }
        }

        // --- stale workspace ---
        if !terminal && now > task.updated_at_ms && now - task.updated_at_ms > stale_threshold_ms {
            metrics.stale_task_count += 1;
        }
    }

    // --- duplicate dispatch count ---
    for count in seen_idempotency_keys.values() {
        if *count > 1 {
            metrics.duplicate_dispatch_count += count - 1;
        }
    }

    // --- rates ---
    let total = metrics.total_tasks;
    if total > 0 {
        metrics.first_attempt_success_rate =
            Some(metrics.first_attempt_success as f64 / total as f64);
        metrics.retry_rate = Some(metrics.tasks_with_retries as f64 / total as f64);
        metrics.abandonment_rate = Some(metrics.abandoned as f64 / total as f64);
    }

    if metrics.recovery_attempts > 0 {
        metrics.recovery_success_rate =
            Some(metrics.recovery_successes as f64 / metrics.recovery_attempts as f64);
    }

    if recovery_attempted || metrics.verification_failures > 0 {
        let verified_total = metrics.first_attempt_success
            + metrics.tasks_with_retries
            + metrics.verification_failures;
        if verified_total > 0 {
            metrics.verification_failure_rate =
                Some(metrics.verification_failures as f64 / verified_total as f64);
        }
    }

    // --- medians ---
    metrics.median_time_to_first_activity_ms = median(&mut first_activity_durations);
    metrics.median_time_to_handoff_ms = median(&mut handoff_durations);

    Ok(metrics)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn median(values: &mut [u64]) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    let mid = values.len() / 2;
    if values.len().is_multiple_of(2) {
        Some((values[mid - 1] + values[mid]) / 2)
    } else {
        Some(values[mid])
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Check whether an attempt has retries (attempt_no > 1).
pub fn attempt_had_retries(attempt: &AttemptRecord) -> bool {
    attempt.attempt_no > 1
}

/// Classify a task record as terminal.
pub fn is_terminal(task: &TaskRecord) -> bool {
    matches!(
        task.state,
        TaskState::Done | TaskState::Failed | TaskState::Cancelled | TaskState::Abandoned
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::orchestration::domain::AttemptState;
    use crate::modules::orchestration::ledger::{CreateAttemptRequest, OrchestrationLedger};

    fn fresh_ledger() -> OrchestrationLedger {
        OrchestrationLedger::open_in_memory().unwrap()
    }

    fn make_task(
        task_id: &str,
        ws: &str,
        title: &str,
        state: TaskState,
        created_at: u64,
    ) -> TaskRecord {
        TaskRecord {
            task_id: task_id.to_string(),
            workspace_key: ws.to_string(),
            source_kind: "local".to_string(),
            source_ref: format!("local:{task_id}"),
            title: title.to_string(),
            description: "test".to_string(),
            state,
            created_at_ms: created_at,
            updated_at_ms: created_at + 1,
        }
    }

    fn seed_task(
        ledger: &OrchestrationLedger,
        task_id: &str,
        ws: &str,
        title: &str,
        state: TaskState,
        created_at: u64,
    ) -> TaskRecord {
        let task = make_task(task_id, ws, title, state, created_at);
        ledger.upsert_task(&task).unwrap();
        task
    }

    #[allow(clippy::too_many_arguments)]
    fn seed_attempt(
        ledger: &OrchestrationLedger,
        attempt_id: &str,
        task_id: &str,
        attempt_no: u32,
        runner: &str,
        started_at: u64,
        terminal_at: Option<u64>,
        idempotency_key: &str,
    ) {
        ledger
            .create_attempt(&CreateAttemptRequest {
                attempt_id: attempt_id.to_string(),
                task_id: task_id.to_string(),
                attempt_no,
                runner_kind: runner.to_string(),
                lease: None,
                idempotency_key: idempotency_key.to_string(),
                now_ms: started_at,
            })
            .unwrap();
        // Mark started.
        ledger
            .set_attempt_state(
                attempt_id,
                AttemptState::Started,
                None,
                &format!("evt-{attempt_id}-start"),
                None,
                started_at,
            )
            .unwrap();
        if let Some(term) = terminal_at {
            ledger
                .set_attempt_state(
                    attempt_id,
                    AttemptState::Completed,
                    Some("ok"),
                    &format!("evt-{attempt_id}-done"),
                    None,
                    term,
                )
                .unwrap();
        }
    }

    // ---- empty workspace ----

    #[test]
    fn empty_workspace_zero_metrics() {
        let ledger = fresh_ledger();
        let m = compute_quality_metrics(&ledger, "ws-1", 3_600_000).unwrap();
        assert_eq!(m.total_tasks, 0);
        assert_eq!(m.first_attempt_success_rate, None);
    }

    // ---- first attempt success ----

    #[test]
    fn single_attempt_done_counts_as_first_success() {
        let ledger = fresh_ledger();
        seed_task(&ledger, "t1", "ws", "Task 1", TaskState::Done, 1000);
        seed_attempt(&ledger, "a1", "t1", 1, "mock", 1100, Some(2000), "k1");

        let m = compute_quality_metrics(&ledger, "ws", 3_600_000).unwrap();
        assert_eq!(m.first_attempt_success, 1);
        assert_eq!(m.first_attempt_success_rate, Some(1.0));
    }

    #[test]
    fn retried_task_not_first_success() {
        let ledger = fresh_ledger();
        seed_task(&ledger, "t1", "ws", "Task 1", TaskState::Done, 1000);
        seed_attempt(&ledger, "a1", "t1", 1, "mock", 1100, Some(1500), "k1");
        seed_attempt(&ledger, "a2", "t1", 2, "mock", 1600, Some(2000), "k2");

        let m = compute_quality_metrics(&ledger, "ws", 3_600_000).unwrap();
        assert_eq!(m.first_attempt_success, 0);
        assert_eq!(m.tasks_with_retries, 1);
        assert_eq!(m.retry_rate, Some(1.0));
    }

    // ---- abandonment ----

    #[test]
    fn abandoned_task_counted() {
        let ledger = fresh_ledger();
        seed_task(&ledger, "t1", "ws", "Task 1", TaskState::Abandoned, 1000);

        let m = compute_quality_metrics(&ledger, "ws", 3_600_000).unwrap();
        assert_eq!(m.abandoned, 1);
        assert_eq!(m.abandonment_rate, Some(1.0));
    }

    // ---- median times ----

    #[test]
    fn median_time_to_first_activity() {
        let ledger = fresh_ledger();
        seed_task(&ledger, "t1", "ws", "T1", TaskState::Done, 1000);
        seed_attempt(&ledger, "a1", "t1", 1, "mock", 1500, Some(2000), "k1");

        let m = compute_quality_metrics(&ledger, "ws", 3_600_000).unwrap();
        assert_eq!(m.median_time_to_first_activity_ms, Some(500));
    }

    #[test]
    fn median_time_to_handoff() {
        let ledger = fresh_ledger();
        seed_task(&ledger, "t1", "ws", "T1", TaskState::Done, 1000);
        seed_attempt(&ledger, "a1", "t1", 1, "mock", 1100, Some(5000), "k1");

        let m = compute_quality_metrics(&ledger, "ws", 3_600_000).unwrap();
        assert_eq!(m.median_time_to_handoff_ms, Some(4000));
    }

    #[test]
    fn median_handles_even_count() {
        let mut vals = vec![100, 300];
        assert_eq!(median(&mut vals), Some(200));
        let mut vals = vec![100, 200, 300, 400];
        assert_eq!(median(&mut vals), Some(250));
    }

    // ---- stale workspace ----

    #[test]
    fn stale_task_detected() {
        let ledger = fresh_ledger();
        // Task created very far in the past, still in Running state.
        let old_time = 1_000_000_000_000u64; // well in the past
        seed_task(&ledger, "t1", "ws", "T1", TaskState::Running, old_time);

        let threshold = 3_600_000u64; // 1 hour
        let m = compute_quality_metrics(&ledger, "ws", threshold).unwrap();
        assert!(m.stale_task_count >= 1, "should detect stale task");
    }

    #[test]
    fn terminal_task_not_stale() {
        let ledger = fresh_ledger();
        let old_time = 1_000_000_000_000u64;
        seed_task(&ledger, "t1", "ws", "T1", TaskState::Done, old_time);

        let m = compute_quality_metrics(&ledger, "ws", 3_600_000).unwrap();
        assert_eq!(m.stale_task_count, 0);
    }

    // ---- helpers ----

    #[test]
    fn is_terminal_classification() {
        let make = |state| TaskRecord {
            task_id: "t".into(),
            workspace_key: "ws".into(),
            source_kind: "local".into(),
            source_ref: "r".into(),
            title: "T".into(),
            description: "".into(),
            state,
            created_at_ms: 0,
            updated_at_ms: 0,
        };
        assert!(is_terminal(&make(TaskState::Done)));
        assert!(is_terminal(&make(TaskState::Failed)));
        assert!(is_terminal(&make(TaskState::Abandoned)));
        assert!(!is_terminal(&make(TaskState::Running)));
        assert!(!is_terminal(&make(TaskState::Queued)));
    }

    #[test]
    fn metrics_are_read_only() {
        let ledger = fresh_ledger();
        seed_task(&ledger, "t1", "ws", "T1", TaskState::Done, 1000);
        let before = ledger.task("t1").unwrap();
        compute_quality_metrics(&ledger, "ws", 3_600_000).unwrap();
        let after = ledger.task("t1").unwrap();
        assert_eq!(before, after, "metrics should not modify ledger data");
    }

    #[test]
    fn no_sensitive_data_in_metrics() {
        let ledger = fresh_ledger();
        let m = compute_quality_metrics(&ledger, "ws", 3_600_000).unwrap();
        let json = serde_json::to_string(&m).unwrap();
        // Metrics should not contain any task content, prompts, or secrets.
        assert!(!json.contains("password"));
        assert!(!json.contains("api_key"));
        assert!(!json.contains("prompt"));
        assert!(!json.contains("secret"));
    }
}
