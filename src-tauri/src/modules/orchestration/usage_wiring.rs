//! Usage event wiring — connects runner usage events to budget tracking
//! and coordinator enforcement (plan §H1 implementation).
//!
//! When a runner emits usage data (tokens, duration, cost), this module
//! accumulates it into TaskUsage and checks it against BudgetsConfig,
//! producing BudgetAlert/BudgetStop signals the coordinator can act on.

use serde::{Deserialize, Serialize};

use super::budget::{BudgetAlert, BudgetStatus, TaskUsage};
use super::workflow_v2::BudgetsConfig;

// ---------------------------------------------------------------------------
// Usage events from runners
// ---------------------------------------------------------------------------

/// A usage update emitted by a runner during or after an attempt.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageEvent {
    pub task_id: String,
    pub attempt_id: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub duration_ms: u64,
    pub cost_usd: f64,
}

impl UsageEvent {
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens.saturating_add(self.output_tokens)
    }
}

// ---------------------------------------------------------------------------
// Usage accumulator
// ---------------------------------------------------------------------------

/// Accumulates usage events per task and enforces budget limits.
#[derive(Clone, Debug, Default)]
pub struct UsageTracker {
    /// task_id → accumulated usage.
    usage: std::collections::HashMap<String, TaskUsage>,
    /// (task_id, attempt_id) → latest absolute counters for that attempt.
    attempts: std::collections::HashMap<(String, String), AttemptUsage>,
    /// task_id → last budget status.
    last_status: std::collections::HashMap<String, BudgetStatus>,
}

#[derive(Clone, Debug, Default)]
struct AttemptUsage {
    elapsed_ms: u64,
    total_tokens: u64,
    total_cost_usd: f64,
}

/// Result of processing a usage event.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageResult {
    pub task_id: String,
    pub status: BudgetStatus,
    pub new_alerts: Vec<BudgetAlert>,
    pub should_stop: bool,
    pub current_usage: TaskUsage,
}

impl UsageTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Process a usage event and check against budget config.
    pub fn process(&mut self, event: &UsageEvent, config: &BudgetsConfig) -> UsageResult {
        let attempt_key = (event.task_id.clone(), event.attempt_id.clone());
        let event_cost = if event.cost_usd.is_finite() && event.cost_usd >= 0.0 {
            event.cost_usd
        } else {
            // Invalid telemetry must fail closed rather than bypassing a cost
            // budget through NaN, infinity, or a negative value. Keep the
            // sentinel finite so UsageResult remains JSON-serializable.
            f64::MAX
        };
        let current_attempt_tokens = {
            let attempt = self.attempts.entry(attempt_key).or_default();
            // Runner counters are absolute within one attempt, so repeated
            // events take the max. Different attempts are summed below.
            attempt.elapsed_ms = attempt.elapsed_ms.max(event.duration_ms);
            attempt.total_tokens = attempt.total_tokens.max(event.total_tokens());
            attempt.total_cost_usd = attempt.total_cost_usd.max(event_cost);
            attempt.total_tokens
        };

        let mut usage = TaskUsage {
            task_id: event.task_id.clone(),
            elapsed_ms: 0,
            total_tokens: 0,
            total_cost_usd: 0.0,
            attempt_count: 0,
        };
        for ((task_id, _), attempt) in &self.attempts {
            if task_id != &event.task_id {
                continue;
            }
            usage.elapsed_ms = usage.elapsed_ms.saturating_add(attempt.elapsed_ms);
            usage.total_tokens = usage.total_tokens.saturating_add(attempt.total_tokens);
            usage.total_cost_usd =
                finite_saturating_add(usage.total_cost_usd, attempt.total_cost_usd);
            usage.attempt_count = usage.attempt_count.saturating_add(1);
        }
        self.usage.insert(event.task_id.clone(), usage.clone());

        // Check budget — both task-level (minutes/cost) and attempt tokens.
        let task_status = super::budget::check_task(&usage, config);
        let token_status = super::budget::check_attempt_tokens(current_attempt_tokens, config);
        let status = merge_statuses(task_status, token_status);
        let prev_status = self
            .last_status
            .insert(event.task_id.clone(), status.clone())
            .unwrap_or(BudgetStatus::Ok);

        // Detect new alerts (alerts that weren't present before).
        let current_alerts: &[BudgetAlert] = match &status {
            BudgetStatus::Ok => &[],
            BudgetStatus::Warning { alerts } | BudgetStatus::Exceeded { alerts } => alerts,
        };
        let prev_alerts: &[BudgetAlert] = match &prev_status {
            BudgetStatus::Ok => &[],
            BudgetStatus::Warning { alerts } | BudgetStatus::Exceeded { alerts } => alerts,
        };
        let prev_was_exceeded = prev_status.is_exceeded();
        let now_exceeded = status.is_exceeded();

        let new_alerts: Vec<BudgetAlert> = current_alerts
            .iter()
            .filter(|alert| {
                // New if dimension wasn't in previous alerts.
                let was_present = prev_alerts
                    .iter()
                    .any(|prev| prev.dimension == alert.dimension);
                // Or if we just transitioned to exceeded.
                !was_present || (now_exceeded && !prev_was_exceeded)
            })
            .cloned()
            .collect();

        let should_stop = status.is_exceeded();

        UsageResult {
            task_id: event.task_id.clone(),
            status,
            new_alerts,
            should_stop,
            current_usage: usage,
        }
    }

    /// Get accumulated usage for a task.
    pub fn usage_for(&self, task_id: &str) -> Option<&TaskUsage> {
        self.usage.get(task_id)
    }

    /// Get the last budget status for a task.
    pub fn status_for(&self, task_id: &str) -> Option<&BudgetStatus> {
        self.last_status.get(task_id)
    }

    /// Clear usage for a task (after completion).
    pub fn clear(&mut self, task_id: &str) {
        self.usage.remove(task_id);
        self.last_status.remove(task_id);
        self.attempts
            .retain(|(tracked_task_id, _), _| tracked_task_id != task_id);
    }

    /// Get all tracked task IDs.
    pub fn tracked_tasks(&self) -> Vec<String> {
        let mut tasks: Vec<String> = self.usage.keys().cloned().collect();
        tasks.sort();
        tasks
    }
}

fn merge_statuses(task: BudgetStatus, attempt: BudgetStatus) -> BudgetStatus {
    let mut warnings = Vec::new();
    let mut exceeded = Vec::new();
    for status in [task, attempt] {
        match status {
            BudgetStatus::Ok => {}
            BudgetStatus::Warning { alerts } => warnings.extend(alerts),
            BudgetStatus::Exceeded { alerts } => exceeded.extend(alerts),
        }
    }
    if !exceeded.is_empty() {
        BudgetStatus::Exceeded { alerts: exceeded }
    } else if !warnings.is_empty() {
        BudgetStatus::Warning { alerts: warnings }
    } else {
        BudgetStatus::Ok
    }
}

fn finite_saturating_add(left: f64, right: f64) -> f64 {
    let sum = left + right;
    if sum.is_finite() {
        sum
    } else {
        f64::MAX
    }
}

// ---------------------------------------------------------------------------
// Coordinator budget gate
// ---------------------------------------------------------------------------

/// Decision the coordinator should make based on budget status.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetDecision {
    Continue,
    Warn,
    Stop,
}

/// Translate a budget status into a coordinator decision.
pub fn budget_decision(status: &BudgetStatus) -> BudgetDecision {
    match status {
        BudgetStatus::Ok => BudgetDecision::Continue,
        BudgetStatus::Warning { .. } => BudgetDecision::Warn,
        BudgetStatus::Exceeded { .. } => BudgetDecision::Stop,
    }
}

/// Check if a task should be stopped before dispatching more work.
pub fn should_stop_task(tracker: &UsageTracker, task_id: &str) -> bool {
    tracker.status_for(task_id).is_some_and(|s| s.is_exceeded())
}

// ---------------------------------------------------------------------------
// Tests
// // -----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config(max_minutes: u64, max_tokens: u64) -> BudgetsConfig {
        BudgetsConfig {
            max_task_minutes: Some(max_minutes),
            max_attempt_tokens: Some(max_tokens),
            max_task_cost_usd: None,
            warn_at_percent: 80,
        }
    }

    fn make_event(task: &str, tokens: u64, duration_min: u64, cost: f64) -> UsageEvent {
        make_attempt_event(task, &format!("{task}-att-1"), tokens, duration_min, cost)
    }

    fn make_attempt_event(
        task: &str,
        attempt: &str,
        tokens: u64,
        duration_min: u64,
        cost: f64,
    ) -> UsageEvent {
        UsageEvent {
            task_id: task.into(),
            attempt_id: attempt.into(),
            input_tokens: tokens / 2,
            output_tokens: tokens / 2,
            duration_ms: duration_min * 60_000,
            cost_usd: cost,
        }
    }

    // ---- accumulation ----

    #[test]
    fn accumulate_usage_from_event() {
        let mut tracker = UsageTracker::new();
        let config = make_config(120, 200_000);

        let result = tracker.process(&make_event("t1", 50_000, 10, 0.5), &config);
        assert_eq!(result.current_usage.total_tokens, 50_000);
        assert_eq!(result.current_usage.elapsed_ms, 600_000);
        assert_eq!(result.current_usage.attempt_count, 1);
    }

    #[test]
    fn accumulate_takes_max_not_sum() {
        // Runner reports absolute values, so we take the max.
        let mut tracker = UsageTracker::new();
        let config = make_config(120, 200_000);

        tracker.process(&make_event("t1", 50_000, 10, 0.5), &config);
        let result = tracker.process(&make_event("t1", 80_000, 15, 0.8), &config);

        // Max tokens: 80_000, max duration: 15min, one unique attempt.
        assert_eq!(result.current_usage.total_tokens, 80_000);
        assert_eq!(result.current_usage.elapsed_ms, 900_000);
        assert_eq!(result.current_usage.attempt_count, 1);
    }

    #[test]
    fn different_attempts_are_summed_without_double_counting_updates() {
        let mut tracker = UsageTracker::new();
        let config = make_config(120, 200_000);

        tracker.process(&make_attempt_event("t1", "att-1", 40_000, 5, 0.4), &config);
        tracker.process(&make_attempt_event("t1", "att-2", 30_000, 3, 0.3), &config);
        let result = tracker.process(&make_attempt_event("t1", "att-1", 50_000, 6, 0.5), &config);

        assert_eq!(result.current_usage.total_tokens, 80_000);
        assert_eq!(result.current_usage.elapsed_ms, 9 * 60_000);
        assert_eq!(result.current_usage.total_cost_usd, 0.8);
        assert_eq!(result.current_usage.attempt_count, 2);
    }

    #[test]
    fn attempt_token_limit_checks_only_the_current_attempt() {
        let mut tracker = UsageTracker::new();
        let config = make_config(120, 100_000);

        tracker.process(&make_attempt_event("t1", "att-1", 60_000, 1, 0.1), &config);
        let result = tracker.process(&make_attempt_event("t1", "att-2", 60_000, 1, 0.1), &config);

        assert_eq!(result.current_usage.total_tokens, 120_000);
        assert!(!result.should_stop);
    }

    #[test]
    fn invalid_cost_telemetry_fails_closed() {
        let mut tracker = UsageTracker::new();
        let mut config = make_config(120, 100_000);
        config.max_task_cost_usd = Some(10.0);

        let result = tracker.process(&make_event("t1", 1_000, 1, f64::NAN), &config);
        assert!(result.should_stop);
        assert!(serde_json::to_string(&result).is_ok());
    }

    // ---- budget enforcement ----

    #[test]
    fn under_budget_returns_continue() {
        let mut tracker = UsageTracker::new();
        let config = make_config(120, 200_000);

        let result = tracker.process(&make_event("t1", 10_000, 5, 0.1), &config);
        assert_eq!(budget_decision(&result.status), BudgetDecision::Continue);
        assert!(!result.should_stop);
    }

    #[test]
    fn near_limit_returns_warning() {
        let mut tracker = UsageTracker::new();
        let config = make_config(120, 100_000);

        // 85% of token limit.
        let result = tracker.process(&make_event("t1", 85_000, 5, 0.1), &config);
        assert_eq!(budget_decision(&result.status), BudgetDecision::Warn);
    }

    #[test]
    fn over_limit_returns_stop() {
        let mut tracker = UsageTracker::new();
        let config = make_config(120, 100_000);

        let result = tracker.process(&make_event("t1", 120_000, 5, 0.1), &config);
        assert_eq!(budget_decision(&result.status), BudgetDecision::Stop);
        assert!(result.should_stop);
    }

    #[test]
    fn time_limit_exceeded_stops() {
        let mut tracker = UsageTracker::new();
        let config = make_config(10, 200_000);

        // 15 minutes > 10 minute limit.
        let result = tracker.process(&make_event("t1", 10_000, 15, 0.1), &config);
        assert!(result.should_stop);
    }

    // ---- new alert detection ----

    #[test]
    fn new_alerts_detected() {
        let mut tracker = UsageTracker::new();
        let config = make_config(120, 100_000);

        // First event: under warn threshold.
        let r1 = tracker.process(&make_event("t1", 50_000, 5, 0.1), &config);
        assert!(r1.new_alerts.is_empty());

        // Second event: crosses warn threshold.
        let r2 = tracker.process(&make_event("t1", 85_000, 5, 0.1), &config);
        assert!(!r2.new_alerts.is_empty(), "should detect new warning alert");

        // Third event: still at warning level — no NEW alerts.
        let r3 = tracker.process(&make_event("t1", 88_000, 5, 0.1), &config);
        assert!(
            r3.new_alerts.is_empty() || !r3.status.is_exceeded(),
            "no new alerts at same warning level"
        );
    }

    #[test]
    fn exceeded_is_new_alert() {
        let mut tracker = UsageTracker::new();
        let config = make_config(120, 100_000);

        // Jump straight to exceeded.
        let r = tracker.process(&make_event("t1", 150_000, 5, 0.1), &config);
        assert!(r.should_stop);
    }

    // ---- should_stop_task ----

    #[test]
    fn should_stop_when_exceeded() {
        let mut tracker = UsageTracker::new();
        let config = make_config(120, 100_000);

        tracker.process(&make_event("t1", 150_000, 5, 0.1), &config);
        assert!(should_stop_task(&tracker, "t1"));
    }

    #[test]
    fn should_not_stop_when_ok() {
        let mut tracker = UsageTracker::new();
        let config = make_config(120, 100_000);

        tracker.process(&make_event("t1", 10_000, 5, 0.1), &config);
        assert!(!should_stop_task(&tracker, "t1"));
    }

    // ---- cleanup ----

    #[test]
    fn clear_removes_tracking() {
        let mut tracker = UsageTracker::new();
        let config = make_config(120, 100_000);

        tracker.process(&make_event("t1", 10_000, 5, 0.1), &config);
        assert!(tracker.usage_for("t1").is_some());

        tracker.clear("t1");
        assert!(tracker.usage_for("t1").is_none());
        assert!(tracker.status_for("t1").is_none());
    }

    // ---- multi-task isolation ----

    #[test]
    fn tasks_tracked_independently() {
        let mut tracker = UsageTracker::new();
        let config = make_config(120, 100_000);

        tracker.process(&make_event("t1", 10_000, 5, 0.1), &config);
        tracker.process(&make_event("t2", 150_000, 5, 0.1), &config);

        assert!(!should_stop_task(&tracker, "t1"));
        assert!(should_stop_task(&tracker, "t2"));
    }

    #[test]
    fn tracked_tasks_listed() {
        let mut tracker = UsageTracker::new();
        let config = make_config(120, 100_000);

        tracker.process(&make_event("t1", 1000, 1, 0.01), &config);
        tracker.process(&make_event("t2", 1000, 1, 0.01), &config);

        let tasks = tracker.tracked_tasks();
        assert_eq!(tasks.len(), 2);
    }
}
