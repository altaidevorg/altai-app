//! Per-task and per-attempt budget accounting (plan §H1).
//!
//! Checks accumulated usage (elapsed time, tokens, cost) against configurable
//! limits from [`BudgetsConfig`]. Returns a structured status so the coordinator
//! can warn the user, pause, or abort before resources are wasted.

use serde::Serialize;

use super::workflow_v2::BudgetsConfig;

/// Accumulated resource usage for a task across all its attempts.
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskUsage {
    pub task_id: String,
    /// Wall-clock time spent across all attempts (including stalled/retried).
    pub elapsed_ms: u64,
    /// Total input + output tokens across all attempts.
    pub total_tokens: u64,
    /// Estimated cost in USD across all attempts.
    pub total_cost_usd: f64,
    pub attempt_count: u32,
}

/// Which budget dimension triggered an alert.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetDimension {
    TaskMinutes,
    AttemptTokens,
    TaskCostUsd,
}

/// A single threshold event — either a warning (approaching limit) or an
/// exceedance (over limit).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BudgetAlert {
    pub dimension: BudgetDimension,
    pub current: f64,
    pub limit: f64,
    /// `current / limit * 100`, capped at 255 (u8).
    pub percent: u8,
}

/// Overall budget status.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum BudgetStatus {
    /// Within limits, no warnings.
    Ok,
    /// One or more dimensions are at or above `warn_at_percent` but below 100%.
    Warning { alerts: Vec<BudgetAlert> },
    /// One or more dimensions have reached or exceeded 100%.
    Exceeded { alerts: Vec<BudgetAlert> },
}

impl BudgetStatus {
    pub fn is_exceeded(&self) -> bool {
        matches!(self, Self::Exceeded { .. })
    }

    pub fn is_ok(&self) -> bool {
        matches!(self, Self::Ok)
    }
}

/// Check accumulated task usage against all configured budget limits.
pub fn check_task(usage: &TaskUsage, config: &BudgetsConfig) -> BudgetStatus {
    let mut warnings = Vec::new();
    let mut exceeded = Vec::new();
    let warn_pct = config.warn_at_percent;

    if let Some(max_minutes) = config.max_task_minutes {
        let minutes = usage.elapsed_ms as f64 / 60_000.0;
        classify(
            BudgetDimension::TaskMinutes,
            minutes,
            max_minutes as f64,
            warn_pct,
            &mut warnings,
            &mut exceeded,
        );
    }

    if let Some(max_cost) = config.max_task_cost_usd {
        classify(
            BudgetDimension::TaskCostUsd,
            usage.total_cost_usd,
            max_cost,
            warn_pct,
            &mut warnings,
            &mut exceeded,
        );
    }

    finalize(warnings, exceeded)
}

/// Check per-attempt token usage against the configured limit.
pub fn check_attempt_tokens(tokens: u64, config: &BudgetsConfig) -> BudgetStatus {
    let Some(max_tokens) = config.max_attempt_tokens else {
        return BudgetStatus::Ok;
    };
    let mut warnings = Vec::new();
    let mut exceeded = Vec::new();
    classify(
        BudgetDimension::AttemptTokens,
        tokens as f64,
        max_tokens as f64,
        config.warn_at_percent,
        &mut warnings,
        &mut exceeded,
    );
    finalize(warnings, exceeded)
}

fn classify(
    dimension: BudgetDimension,
    current: f64,
    limit: f64,
    warn_pct: u8,
    warnings: &mut Vec<BudgetAlert>,
    exceeded: &mut Vec<BudgetAlert>,
) {
    if limit <= 0.0 {
        return;
    }
    let ratio = (current / limit).clamp(0.0, 1.0);
    let percent = (ratio * 100.0).round() as u8;
    let alert = BudgetAlert {
        dimension,
        current,
        limit,
        percent,
    };
    if ratio >= 1.0 {
        exceeded.push(alert);
    } else if percent >= warn_pct {
        warnings.push(alert);
    }
}

fn finalize(warnings: Vec<BudgetAlert>, exceeded: Vec<BudgetAlert>) -> BudgetStatus {
    if !exceeded.is_empty() {
        BudgetStatus::Exceeded { alerts: exceeded }
    } else if !warnings.is_empty() {
        BudgetStatus::Warning { alerts: warnings }
    } else {
        BudgetStatus::Ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(
        max_minutes: Option<u64>,
        max_tokens: Option<u64>,
        max_cost: Option<f64>,
        warn_pct: u8,
    ) -> BudgetsConfig {
        BudgetsConfig {
            max_task_minutes: max_minutes,
            max_attempt_tokens: max_tokens,
            max_task_cost_usd: max_cost,
            warn_at_percent: warn_pct,
        }
    }

    fn usage(elapsed_ms: u64, tokens: u64, cost: f64) -> TaskUsage {
        TaskUsage {
            task_id: "t-1".into(),
            elapsed_ms,
            total_tokens: tokens,
            total_cost_usd: cost,
            attempt_count: 1,
        }
    }

    // ---- no limits configured → always Ok ----

    #[test]
    fn no_limits_is_always_ok() {
        let cfg = config(None, None, None, 80);
        let status = check_task(&usage(9_999_999, 9_999_999, 999.0), &cfg);
        assert_eq!(status, BudgetStatus::Ok);
    }

    // ---- well under limits → Ok ----

    #[test]
    fn well_under_limits_is_ok() {
        let cfg = config(Some(120), Some(200_000), Some(10.0), 80);
        // 10 minutes, 10k tokens, $0.50
        let status = check_task(&usage(600_000, 10_000, 0.5), &cfg);
        assert_eq!(status, BudgetStatus::Ok);
    }

    // ---- task minutes: warning at threshold ----

    #[test]
    fn task_minutes_warning_at_threshold() {
        let cfg = config(Some(100), None, None, 80);
        // 82 minutes of 100 → 82% → warning
        let status = check_task(&usage(82 * 60_000, 0, 0.0), &cfg);
        assert!(matches!(status, BudgetStatus::Warning { alerts } if alerts.len() == 1));
    }

    #[test]
    fn task_minutes_exceeded() {
        let cfg = config(Some(100), None, None, 80);
        // 100 minutes of 100 → 100% → exceeded
        let status = check_task(&usage(100 * 60_000, 0, 0.0), &cfg);
        match status {
            BudgetStatus::Exceeded { alerts } => {
                assert_eq!(alerts.len(), 1);
                assert_eq!(alerts[0].dimension, BudgetDimension::TaskMinutes);
                assert_eq!(alerts[0].percent, 100);
            }
            other => panic!("expected Exceeded, got {other:?}"),
        }
    }

    #[test]
    fn task_minutes_below_warning_threshold_is_ok() {
        let cfg = config(Some(100), None, None, 80);
        // 79 minutes of 100 → 79% → ok (below 80% threshold)
        let status = check_task(&usage(79 * 60_000, 0, 0.0), &cfg);
        assert_eq!(status, BudgetStatus::Ok);
    }

    // ---- attempt tokens ----

    #[test]
    fn attempt_tokens_warning() {
        let cfg = config(None, Some(100_000), None, 80);
        // 85k of 100k → 85% → warning
        let status = check_attempt_tokens(85_000, &cfg);
        assert!(matches!(status, BudgetStatus::Warning { .. }));
    }

    #[test]
    fn attempt_tokens_exceeded() {
        let cfg = config(None, Some(100_000), None, 80);
        let status = check_attempt_tokens(100_000, &cfg);
        assert!(matches!(status, BudgetStatus::Exceeded { .. }));
    }

    #[test]
    fn attempt_tokens_no_limit_is_ok() {
        let cfg = config(None, None, None, 80);
        let status = check_attempt_tokens(999_999_999, &cfg);
        assert_eq!(status, BudgetStatus::Ok);
    }

    // ---- cost ----

    #[test]
    fn cost_warning() {
        let cfg = config(None, None, Some(5.0), 80);
        // $4.20 of $5.00 → 84% → warning
        let status = check_task(&usage(0, 0, 4.2), &cfg);
        assert!(matches!(status, BudgetStatus::Warning { .. }));
    }

    #[test]
    fn cost_exceeded() {
        let cfg = config(None, None, Some(5.0), 80);
        let status = check_task(&usage(0, 0, 5.5), &cfg);
        assert!(matches!(status, BudgetStatus::Exceeded { .. }));
    }

    // ---- multiple dimensions simultaneously ----

    #[test]
    fn multiple_dimensions_exceeded_takes_priority() {
        let cfg = config(Some(100), None, Some(5.0), 80);
        // Minutes exceeded, cost only warning.
        let status = check_task(&usage(110 * 60_000, 0, 4.2), &cfg);
        assert!(matches!(status, BudgetStatus::Exceeded { alerts } if alerts.len() == 1));
    }

    #[test]
    fn multiple_warnings_collected() {
        let cfg = config(Some(100), None, Some(5.0), 80);
        // Both at warning level.
        let status = check_task(&usage(85 * 60_000, 0, 4.3), &cfg);
        match status {
            BudgetStatus::Warning { alerts } => assert_eq!(alerts.len(), 2),
            other => panic!("expected Warning, got {other:?}"),
        }
    }

    // ---- edge cases ----

    #[test]
    fn exactly_at_warning_threshold() {
        let cfg = config(Some(100), None, None, 80);
        // 80 minutes of 100 → exactly 80% → warning
        let status = check_task(&usage(80 * 60_000, 0, 0.0), &cfg);
        assert!(matches!(status, BudgetStatus::Warning { .. }));
    }

    #[test]
    fn zero_usage_is_ok() {
        let cfg = config(Some(120), Some(200_000), Some(10.0), 80);
        let status = check_task(&usage(0, 0, 0.0), &cfg);
        assert_eq!(status, BudgetStatus::Ok);
    }

    // ---- status helpers ----

    #[test]
    fn status_helpers() {
        assert!(BudgetStatus::Ok.is_ok());
        assert!(!BudgetStatus::Ok.is_exceeded());

        let warn = BudgetStatus::Warning {
            alerts: vec![BudgetAlert {
                dimension: BudgetDimension::TaskMinutes,
                current: 85.0,
                limit: 100.0,
                percent: 85,
            }],
        };
        assert!(!warn.is_ok());
        assert!(!warn.is_exceeded());

        let exc = BudgetStatus::Exceeded {
            alerts: vec![BudgetAlert {
                dimension: BudgetDimension::TaskMinutes,
                current: 110.0,
                limit: 100.0,
                percent: 100,
            }],
        };
        assert!(!exc.is_ok());
        assert!(exc.is_exceeded());
    }
}
