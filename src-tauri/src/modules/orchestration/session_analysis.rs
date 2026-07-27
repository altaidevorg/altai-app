//! Session analysis and playbooks (plan §G4).
//!
//! Analyzes successful, failed, expensive, and abandoned attempts. Compares
//! paths, retries, and tool use. Proposes reusable playbooks — but never
//! saves anything without explicit user review. Secrets and raw sensitive
//! logs are excluded from all learning.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::domain::{AttemptState, TaskState};
use super::ledger::{AttemptRecord, LedgerResult, OrchestrationLedger, TaskRecord};

// ---------------------------------------------------------------------------
// Attempt analysis
// ---------------------------------------------------------------------------

/// The outcome category of an analyzed attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptOutcome {
    Success,
    Failure,
    Expensive,
    Abandoned,
}

/// What kind of signal the analysis detected.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalKind {
    HighRetryCount,
    SlowStart,
    RapidFailure,
    MissingContext,
    ExcessiveOutput,
    RepeatedError,
    LongRunning,
}

/// A single signal detected during analysis.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisSignal {
    pub kind: SignalKind,
    pub detail: String,
}

/// Analysis result for a single attempt (or task-level aggregation).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttemptAnalysis {
    pub task_id: String,
    pub outcome: AttemptOutcome,
    pub attempt_count: u32,
    pub duration_ms: Option<u64>,
    pub signals: Vec<AnalysisSignal>,
    pub error_summary: Option<String>,
}

/// Configuration thresholds for analysis.
#[derive(Clone, Debug)]
pub struct AnalysisConfig {
    pub high_retry_threshold: u32,
    pub slow_start_threshold_ms: u64,
    pub rapid_failure_threshold_ms: u64,
    pub long_running_threshold_ms: u64,
    #[allow(dead_code)]
    pub expensive_token_threshold: u64,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            high_retry_threshold: 3,
            slow_start_threshold_ms: 60_000,
            rapid_failure_threshold_ms: 10_000,
            long_running_threshold_ms: 1_800_000,
            expensive_token_threshold: 100_000,
        }
    }
}

/// Analyze all tasks in a workspace and classify their outcomes.
#[allow(dead_code)]
pub fn analyze_workspace(
    ledger: &OrchestrationLedger,
    workspace_key: &str,
    config: &AnalysisConfig,
) -> LedgerResult<Vec<AttemptAnalysis>> {
    let tasks = ledger.tasks_for_workspace(workspace_key)?;
    let mut analyses = Vec::with_capacity(tasks.len());
    for task in &tasks {
        analyses.push(analyze_task(ledger, task, config)?);
    }
    Ok(analyses)
}

/// Analyze a single task's attempt history.
pub fn analyze_task(
    ledger: &OrchestrationLedger,
    task: &TaskRecord,
    config: &AnalysisConfig,
) -> LedgerResult<AttemptAnalysis> {
    let attempts = ledger.attempts_for_task(&task.task_id)?;
    let attempt_count = attempts.len() as u32;

    let mut signals = Vec::new();
    let mut error_summary: Option<String> = None;

    // Determine outcome.
    let outcome = classify_outcome(task, &attempts);

    // --- high retry count ---
    if attempt_count >= config.high_retry_threshold {
        signals.push(AnalysisSignal {
            kind: SignalKind::HighRetryCount,
            detail: format!(
                "Task required {} attempts (threshold: {})",
                attempt_count, config.high_retry_threshold
            ),
        });
    }

    // --- duration analysis ---
    let duration_ms = compute_duration(task, &attempts);
    if let Some(dur) = duration_ms {
        if dur >= config.long_running_threshold_ms {
            signals.push(AnalysisSignal {
                kind: SignalKind::LongRunning,
                detail: format!(
                    "Task ran for {}ms (threshold: {}ms)",
                    dur, config.long_running_threshold_ms
                ),
            });
        }
    }

    // --- slow start ---
    if let Some(first) = attempts.iter().min_by_key(|a| a.attempt_no) {
        if let (Some(created), Some(started)) = (Some(task.created_at_ms), first.started_at_ms) {
            if started > created && started - created >= config.slow_start_threshold_ms {
                signals.push(AnalysisSignal {
                    kind: SignalKind::SlowStart,
                    detail: format!(
                        "First attempt started {}ms after task creation (threshold: {}ms)",
                        started - created,
                        config.slow_start_threshold_ms
                    ),
                });
            }
        }
    }

    // --- rapid failure ---
    for att in &attempts {
        if att.state == AttemptState::Failed {
            if let (Some(start), Some(term)) = (att.started_at_ms, att.terminal_at_ms) {
                if term > start && term - start <= config.rapid_failure_threshold_ms {
                    signals.push(AnalysisSignal {
                        kind: SignalKind::RapidFailure,
                        detail: format!(
                            "Attempt {} failed within {}ms",
                            att.attempt_no,
                            term - start
                        ),
                    });
                    if error_summary.is_none() {
                        error_summary = att.terminal_outcome.clone();
                    }
                    break;
                }
            }
        }
    }

    // --- repeated errors ---
    let error_counts: HashMap<&str, u32> = attempts
        .iter()
        .filter(|a| a.terminal_outcome.is_some())
        .fold(HashMap::new(), |mut acc, a| {
            let key = a.terminal_outcome.as_deref().unwrap_or("");
            *acc.entry(key).or_default() += 1;
            acc
        });
    for (error, count) in &error_counts {
        if *count >= 2 && !error.is_empty() {
            signals.push(AnalysisSignal {
                kind: SignalKind::RepeatedError,
                detail: format!("Same error '{error}' appeared {count} times"),
            });
            if error_summary.is_none() {
                error_summary = Some(error.to_string());
            }
        }
    }

    // --- missing context (task had no description) ---
    if task.description.trim().is_empty() {
        signals.push(AnalysisSignal {
            kind: SignalKind::MissingContext,
            detail: "Task had no description — agent may have lacked context".into(),
        });
    }

    Ok(AttemptAnalysis {
        task_id: task.task_id.clone(),
        outcome,
        attempt_count,
        duration_ms,
        signals,
        error_summary,
    })
}

fn classify_outcome(task: &TaskRecord, attempts: &[AttemptRecord]) -> AttemptOutcome {
    match task.state {
        TaskState::Done => {
            // Check if it was expensive (many retries or long duration).
            if attempts.len() as u32 >= 3 {
                AttemptOutcome::Expensive
            } else {
                AttemptOutcome::Success
            }
        }
        TaskState::Abandoned => AttemptOutcome::Abandoned,
        TaskState::Failed | TaskState::Cancelled => AttemptOutcome::Failure,
        _ => {
            // Non-terminal: classify by latest attempt state.
            if attempts.iter().any(|a| a.state == AttemptState::Failed) {
                AttemptOutcome::Failure
            } else if attempts.is_empty() {
                AttemptOutcome::Abandoned
            } else {
                AttemptOutcome::Success
            }
        }
    }
}

fn compute_duration(task: &TaskRecord, attempts: &[AttemptRecord]) -> Option<u64> {
    let earliest_start = attempts.iter().filter_map(|a| a.started_at_ms).min()?;
    let latest_end = attempts
        .iter()
        .filter_map(|a| a.terminal_at_ms)
        .max()
        .unwrap_or(task.updated_at_ms);
    if latest_end > earliest_start {
        Some(latest_end - earliest_start)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Playbook proposals
// ---------------------------------------------------------------------------

/// What kind of action a playbook proposes.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum PlaybookAction {
    RetryPolicy {
        max_retries: u32,
        backoff_base_ms: u64,
    },
    PrecheckRule {
        rule: String,
    },
    DocumentationUpdate {
        file: String,
        summary: String,
    },
    HookSuggestion {
        event: String,
        command: String,
    },
    QualityRule {
        rule: String,
    },
}

/// Whether a proposal has been reviewed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProposalStatus {
    #[default]
    Pending,
    Approved,
    Rejected,
    Applied,
}

/// A proposed playbook derived from session analysis.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybookProposal {
    pub id: String,
    pub title: String,
    pub trigger: String,
    pub action: PlaybookAction,
    pub cited_task_ids: Vec<String>,
    pub rationale: String,
    pub status: ProposalStatus,
}

/// Generate playbook proposals from attempt analyses.
/// Proposals are always Pending — they require user review before any
/// workflow change is applied.
pub fn propose_playbooks(analyses: &[AttemptAnalysis]) -> Vec<PlaybookProposal> {
    let mut proposals = Vec::new();

    // --- high retry count → retry policy ---
    let high_retry: Vec<_> = analyses
        .iter()
        .filter(|a| {
            a.signals
                .iter()
                .any(|s| s.kind == SignalKind::HighRetryCount)
        })
        .collect();
    if let Some(high_retry_count) = std::num::NonZeroUsize::new(high_retry.len()) {
        let total_retries = high_retry
            .iter()
            .map(|analysis| u64::from(analysis.attempt_count))
            .fold(0_u64, u64::saturating_add);
        let divisor = u64::try_from(high_retry_count.get()).unwrap_or(u64::MAX);
        let avg_retries = u32::try_from(total_retries / divisor).unwrap_or(u32::MAX);
        proposals.push(PlaybookProposal {
            id: "pb-retry-policy".into(),
            title: "Adjust retry policy for frequently-retried tasks".into(),
            trigger: "Task requires 3+ attempts".into(),
            action: PlaybookAction::RetryPolicy {
                max_retries: avg_retries.saturating_add(2),
                backoff_base_ms: 10_000,
            },
            cited_task_ids: high_retry.iter().map(|a| a.task_id.clone()).collect(),
            rationale: format!(
                "{} tasks averaged {} attempts. Consider increasing max_retries or adjusting backoff.",
                high_retry.len(),
                avg_retries
            ),
            status: ProposalStatus::Pending,
        });
    }

    // --- rapid failure → precheck rule ---
    let rapid_fail: Vec<_> = analyses
        .iter()
        .filter(|a| a.signals.iter().any(|s| s.kind == SignalKind::RapidFailure))
        .collect();
    if !rapid_fail.is_empty() {
        let error = rapid_fail
            .iter()
            .filter_map(|a| a.error_summary.as_ref())
            .next()
            .cloned()
            .unwrap_or_else(|| "Unknown error".into());
        proposals.push(PlaybookProposal {
            id: "pb-precheck".into(),
            title: "Add precheck to catch rapid-failure conditions".into(),
            trigger: format!("Attempt fails within 10s with: {error}"),
            action: PlaybookAction::PrecheckRule {
                rule: format!(
                    "Verify environment and dependencies before starting (error: {error})"
                ),
            },
            cited_task_ids: rapid_fail.iter().map(|a| a.task_id.clone()).collect(),
            rationale: format!(
                "{} tasks failed rapidly. A precheck could prevent wasted attempts.",
                rapid_fail.len()
            ),
            status: ProposalStatus::Pending,
        });
    }

    // --- repeated error → documentation update ---
    let repeated: Vec<_> = analyses
        .iter()
        .filter(|a| {
            a.signals
                .iter()
                .any(|s| s.kind == SignalKind::RepeatedError)
        })
        .collect();
    if !repeated.is_empty() {
        let error = repeated
            .iter()
            .filter_map(|a| a.error_summary.as_ref())
            .next()
            .cloned()
            .unwrap_or_else(|| "recurring error".into());
        proposals.push(PlaybookProposal {
            id: "pb-doc-update".into(),
            title: "Document recurring failure and resolution".into(),
            trigger: format!("Same error appears across multiple attempts: {error}"),
            action: PlaybookAction::DocumentationUpdate {
                file: "docs/TROUBLESHOOTING.md".into(),
                summary: format!("Add section for: {error}"),
            },
            cited_task_ids: repeated.iter().map(|a| a.task_id.clone()).collect(),
            rationale: format!(
                "{} tasks hit the same error. Documenting the resolution would help future tasks.",
                repeated.len()
            ),
            status: ProposalStatus::Pending,
        });
    }

    // --- missing context → hook suggestion ---
    let missing_ctx: Vec<_> = analyses
        .iter()
        .filter(|a| {
            a.signals
                .iter()
                .any(|s| s.kind == SignalKind::MissingContext)
        })
        .collect();
    if !missing_ctx.is_empty() {
        proposals.push(PlaybookProposal {
            id: "pb-context-hook".into(),
            title: "Add pre-dispatch context validation hook".into(),
            trigger: "Task created without description".into(),
            action: PlaybookAction::HookSuggestion {
                event: "pre_dispatch".into(),
                command: "test -n \"$TASK_DESCRIPTION\" || { echo 'Task description required'; exit 1; }".into(),
            },
            cited_task_ids: missing_ctx.iter().map(|a| a.task_id.clone()).collect(),
            rationale: format!(
                "{} tasks lacked descriptions. A pre-dispatch hook could enforce context requirements.",
                missing_ctx.len()
            ),
            status: ProposalStatus::Pending,
        });
    }

    // --- long running → quality rule ---
    let long_running: Vec<_> = analyses
        .iter()
        .filter(|a| a.signals.iter().any(|s| s.kind == SignalKind::LongRunning))
        .collect();
    if !long_running.is_empty() {
        proposals.push(PlaybookProposal {
            id: "pb-timeout-rule".into(),
            title: "Add timeout quality rule for long-running tasks".into(),
            trigger: "Task runs longer than 30 minutes".into(),
            action: PlaybookAction::QualityRule {
                rule: "Flag tasks exceeding 30min for manual review or decomposition".into(),
            },
            cited_task_ids: long_running.iter().map(|a| a.task_id.clone()).collect(),
            rationale: format!(
                "{} tasks ran for a very long time. Consider decomposition or timeout rules.",
                long_running.len()
            ),
            status: ProposalStatus::Pending,
        });
    }

    proposals
}

// ---------------------------------------------------------------------------
// Sanitization
// ---------------------------------------------------------------------------

/// Remove sensitive data from an analysis (error summaries may contain secrets).
pub fn sanitize_analysis(analysis: &mut AttemptAnalysis) {
    for signal in &mut analysis.signals {
        sanitize_string(&mut signal.detail);
    }
    if let Some(ref mut error) = analysis.error_summary {
        sanitize_string(error);
    }
}

/// Redact common secret patterns from a string.
fn sanitize_string(text: &mut String) {
    let patterns = ["sk-", "ghp_", "gho_", "AKIA", "password=", "secret="];
    for pat in &patterns {
        if text.contains(pat) {
            *text = "[REDACTED: contains potential secret]".to_string();
            return;
        }
    }
}

// ---------------------------------------------------------------------------
// Playbook serialization (for .altai/playbooks/)
// ---------------------------------------------------------------------------

/// Serialize playbooks to YAML-like format for `.altai/playbooks/`.
pub fn proposals_to_yaml(proposals: &[PlaybookProposal]) -> String {
    let mut out = String::new();
    for p in proposals {
        out.push_str(&format!(
            "# Playbook: {}\n# Trigger: {}\n# Status: {:?}\n#\n",
            p.title, p.trigger, p.status
        ));
        out.push_str(&format!("# Cited tasks: {}\n", p.cited_task_ids.join(", ")));
        out.push_str(&format!("# Rationale: {}\n", p.rationale));
        out.push_str(&format!("id: {}\n", p.id));
        out.push_str(&format!("title: {:?}\n", p.title));
        out.push_str(&format!("trigger: {:?}\n", p.trigger));
        match &p.action {
            PlaybookAction::RetryPolicy {
                max_retries,
                backoff_base_ms,
            } => {
                out.push_str("action:\n");
                out.push_str("  type: retry_policy\n");
                out.push_str(&format!("  max_retries: {max_retries}\n"));
                out.push_str(&format!("  backoff_base_ms: {backoff_base_ms}\n"));
            }
            PlaybookAction::PrecheckRule { rule } => {
                out.push_str("action:\n");
                out.push_str("  type: precheck_rule\n");
                out.push_str(&format!("  rule: {rule:?}\n"));
            }
            PlaybookAction::DocumentationUpdate { file, summary } => {
                out.push_str("action:\n");
                out.push_str("  type: documentation_update\n");
                out.push_str(&format!("  file: {file:?}\n"));
                out.push_str(&format!("  summary: {summary:?}\n"));
            }
            PlaybookAction::HookSuggestion { event, command } => {
                out.push_str("action:\n");
                out.push_str("  type: hook_suggestion\n");
                out.push_str(&format!("  event: {event:?}\n"));
                out.push_str(&format!("  command: {command:?}\n"));
            }
            PlaybookAction::QualityRule { rule } => {
                out.push_str("action:\n");
                out.push_str("  type: quality_rule\n");
                out.push_str(&format!("  rule: {rule:?}\n"));
            }
        }
        out.push('\n');
    }
    out
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
        state: TaskState,
        created: u64,
        desc: &str,
    ) -> TaskRecord {
        TaskRecord {
            task_id: task_id.into(),
            workspace_key: ws.into(),
            source_kind: "local".into(),
            source_ref: format!("local:{task_id}"),
            title: format!("Task {task_id}"),
            description: desc.into(),
            state,
            created_at_ms: created,
            updated_at_ms: created + 1,
        }
    }

    fn seed_task(
        ledger: &OrchestrationLedger,
        task_id: &str,
        ws: &str,
        state: TaskState,
        created: u64,
        desc: &str,
    ) -> TaskRecord {
        let task = make_task(task_id, ws, state, created, desc);
        ledger.upsert_task(&task).unwrap();
        task
    }

    #[allow(clippy::too_many_arguments)]
    fn add_attempt(
        ledger: &OrchestrationLedger,
        attempt_id: &str,
        task_id: &str,
        attempt_no: u32,
        started: u64,
        terminal: Option<u64>,
        state: AttemptState,
        outcome: Option<&str>,
    ) {
        ledger
            .create_attempt(&CreateAttemptRequest {
                attempt_id: attempt_id.into(),
                task_id: task_id.into(),
                attempt_no,
                runner_kind: "mock".into(),
                lease: None,
                idempotency_key: format!("{task_id}-{attempt_no}"),
                now_ms: started,
            })
            .unwrap();
        ledger
            .set_attempt_state(
                attempt_id,
                AttemptState::Started,
                None,
                &format!("evt-{attempt_id}-start"),
                None,
                started,
            )
            .unwrap();
        if let Some(term) = terminal {
            ledger
                .set_attempt_state(
                    attempt_id,
                    state,
                    outcome,
                    &format!("evt-{attempt_id}-term"),
                    None,
                    term,
                )
                .unwrap();
        }
    }

    // ---- outcome classification ----

    #[test]
    fn done_single_attempt_is_success() {
        let ledger = fresh_ledger();
        let task = seed_task(&ledger, "t1", "ws", TaskState::Done, 1000, "desc");
        add_attempt(
            &ledger,
            "a1",
            "t1",
            1,
            1100,
            Some(2000),
            AttemptState::Completed,
            Some("ok"),
        );
        let analysis = analyze_task(&ledger, &task, &AnalysisConfig::default()).unwrap();
        assert_eq!(analysis.outcome, AttemptOutcome::Success);
    }

    #[test]
    fn done_many_retries_is_expensive() {
        let ledger = fresh_ledger();
        let task = seed_task(&ledger, "t1", "ws", TaskState::Done, 1000, "desc");
        add_attempt(
            &ledger,
            "a1",
            "t1",
            1,
            1100,
            Some(1200),
            AttemptState::Failed,
            Some("err"),
        );
        add_attempt(
            &ledger,
            "a2",
            "t1",
            2,
            1300,
            Some(1400),
            AttemptState::Failed,
            Some("err"),
        );
        add_attempt(
            &ledger,
            "a3",
            "t1",
            3,
            1500,
            Some(1600),
            AttemptState::Completed,
            Some("ok"),
        );
        let analysis = analyze_task(&ledger, &task, &AnalysisConfig::default()).unwrap();
        assert_eq!(analysis.outcome, AttemptOutcome::Expensive);
    }

    #[test]
    fn abandoned_task_classified() {
        let ledger = fresh_ledger();
        let task = seed_task(&ledger, "t1", "ws", TaskState::Abandoned, 1000, "desc");
        let analysis = analyze_task(&ledger, &task, &AnalysisConfig::default()).unwrap();
        assert_eq!(analysis.outcome, AttemptOutcome::Abandoned);
    }

    // ---- signals ----

    #[test]
    fn high_retry_detected() {
        let ledger = fresh_ledger();
        let task = seed_task(&ledger, "t1", "ws", TaskState::Done, 1000, "desc");
        add_attempt(
            &ledger,
            "a1",
            "t1",
            1,
            1100,
            Some(1200),
            AttemptState::Failed,
            Some("err"),
        );
        add_attempt(
            &ledger,
            "a2",
            "t1",
            2,
            1300,
            Some(1400),
            AttemptState::Failed,
            Some("err"),
        );
        add_attempt(
            &ledger,
            "a3",
            "t1",
            3,
            1500,
            Some(1600),
            AttemptState::Completed,
            Some("ok"),
        );
        let analysis = analyze_task(&ledger, &task, &AnalysisConfig::default()).unwrap();
        assert!(analysis
            .signals
            .iter()
            .any(|s| s.kind == SignalKind::HighRetryCount));
    }

    #[test]
    fn rapid_failure_detected() {
        let ledger = fresh_ledger();
        let task = seed_task(&ledger, "t1", "ws", TaskState::Failed, 1000, "desc");
        add_attempt(
            &ledger,
            "a1",
            "t1",
            1,
            1100,
            Some(1105),
            AttemptState::Failed,
            Some("crash"),
        );
        let analysis = analyze_task(&ledger, &task, &AnalysisConfig::default()).unwrap();
        assert!(analysis
            .signals
            .iter()
            .any(|s| s.kind == SignalKind::RapidFailure));
    }

    #[test]
    fn missing_context_detected() {
        let ledger = fresh_ledger();
        let task = seed_task(&ledger, "t1", "ws", TaskState::Done, 1000, "");
        let analysis = analyze_task(&ledger, &task, &AnalysisConfig::default()).unwrap();
        assert!(analysis
            .signals
            .iter()
            .any(|s| s.kind == SignalKind::MissingContext));
    }

    #[test]
    fn repeated_error_detected() {
        let ledger = fresh_ledger();
        let task = seed_task(&ledger, "t1", "ws", TaskState::Done, 1000, "desc");
        add_attempt(
            &ledger,
            "a1",
            "t1",
            1,
            1100,
            Some(2000),
            AttemptState::Failed,
            Some("OOM"),
        );
        add_attempt(
            &ledger,
            "a2",
            "t1",
            2,
            2100,
            Some(3000),
            AttemptState::Failed,
            Some("OOM"),
        );
        add_attempt(
            &ledger,
            "a3",
            "t1",
            3,
            3100,
            Some(4000),
            AttemptState::Completed,
            Some("ok"),
        );
        let analysis = analyze_task(&ledger, &task, &AnalysisConfig::default()).unwrap();
        assert!(analysis
            .signals
            .iter()
            .any(|s| s.kind == SignalKind::RepeatedError));
    }

    // ---- playbook proposals ----

    #[test]
    fn high_retry_proposes_retry_policy() {
        let analyses = vec![AttemptAnalysis {
            task_id: "t1".into(),
            outcome: AttemptOutcome::Expensive,
            attempt_count: 4,
            duration_ms: Some(100_000),
            signals: vec![AnalysisSignal {
                kind: SignalKind::HighRetryCount,
                detail: "4 attempts".into(),
            }],
            error_summary: None,
        }];
        let proposals = propose_playbooks(&analyses);
        assert!(proposals.iter().any(|p| p.id == "pb-retry-policy"));
        let retry = proposals
            .iter()
            .find(|p| p.id == "pb-retry-policy")
            .unwrap();
        assert!(retry.cited_task_ids.contains(&"t1".to_string()));
        assert_eq!(retry.status, ProposalStatus::Pending);
    }

    #[test]
    fn rapid_failure_proposes_precheck() {
        let analyses = vec![AttemptAnalysis {
            task_id: "t1".into(),
            outcome: AttemptOutcome::Failure,
            attempt_count: 1,
            duration_ms: Some(500),
            signals: vec![AnalysisSignal {
                kind: SignalKind::RapidFailure,
                detail: "failed in 5ms".into(),
            }],
            error_summary: Some("OOM".into()),
        }];
        let proposals = propose_playbooks(&analyses);
        assert!(proposals.iter().any(|p| p.id == "pb-precheck"));
    }

    #[test]
    fn missing_context_proposes_hook() {
        let analyses = vec![AttemptAnalysis {
            task_id: "t1".into(),
            outcome: AttemptOutcome::Success,
            attempt_count: 1,
            duration_ms: None,
            signals: vec![AnalysisSignal {
                kind: SignalKind::MissingContext,
                detail: "no description".into(),
            }],
            error_summary: None,
        }];
        let proposals = propose_playbooks(&analyses);
        assert!(proposals.iter().any(|p| p.id == "pb-context-hook"));
    }

    #[test]
    fn clean_session_produces_no_proposals() {
        let analyses = vec![AttemptAnalysis {
            task_id: "t1".into(),
            outcome: AttemptOutcome::Success,
            attempt_count: 1,
            duration_ms: Some(100),
            signals: vec![],
            error_summary: None,
        }];
        let proposals = propose_playbooks(&analyses);
        assert!(proposals.is_empty());
    }

    #[test]
    fn all_proposals_are_pending() {
        let analyses = vec![
            AttemptAnalysis {
                task_id: "t1".into(),
                outcome: AttemptOutcome::Expensive,
                attempt_count: 4,
                duration_ms: None,
                signals: vec![AnalysisSignal {
                    kind: SignalKind::HighRetryCount,
                    detail: "test".into(),
                }],
                error_summary: None,
            },
            AttemptAnalysis {
                task_id: "t2".into(),
                outcome: AttemptOutcome::Failure,
                attempt_count: 1,
                duration_ms: None,
                signals: vec![AnalysisSignal {
                    kind: SignalKind::MissingContext,
                    detail: "test".into(),
                }],
                error_summary: None,
            },
        ];
        let proposals = propose_playbooks(&analyses);
        assert!(proposals
            .iter()
            .all(|p| p.status == ProposalStatus::Pending));
    }

    // ---- sanitization ----

    #[test]
    fn sanitize_redacts_secret_in_error() {
        let mut analysis = AttemptAnalysis {
            task_id: "t1".into(),
            outcome: AttemptOutcome::Failure,
            attempt_count: 1,
            duration_ms: None,
            signals: vec![AnalysisSignal {
                kind: SignalKind::RapidFailure,
                detail: "Failed with sk-abc123 in config".into(),
            }],
            error_summary: Some("Token: ghp_xyz".into()),
        };
        sanitize_analysis(&mut analysis);
        assert!(analysis.signals[0].detail.contains("[REDACTED"));
        assert!(analysis
            .error_summary
            .as_ref()
            .unwrap()
            .contains("[REDACTED"));
    }

    // ---- serialization ----

    #[test]
    fn proposals_yaml_is_valid() {
        let proposals = vec![PlaybookProposal {
            id: "pb-test".into(),
            title: "Test playbook".into(),
            trigger: "test trigger".into(),
            action: PlaybookAction::RetryPolicy {
                max_retries: 5,
                backoff_base_ms: 10_000,
            },
            cited_task_ids: vec!["t1".into()],
            rationale: "test rationale".into(),
            status: ProposalStatus::Pending,
        }];
        let yaml = proposals_to_yaml(&proposals);
        assert!(yaml.contains("id: pb-test"));
        assert!(yaml.contains("type: retry_policy"));
        assert!(yaml.contains("max_retries: 5"));
        assert!(yaml.contains("# Cited tasks: t1"));
    }

    // ---- G4 acceptance: proposals cite motivating runs ----

    #[test]
    fn every_proposal_cites_task_ids() {
        let analyses = vec![
            AttemptAnalysis {
                task_id: "t1".into(),
                outcome: AttemptOutcome::Expensive,
                attempt_count: 5,
                duration_ms: None,
                signals: vec![AnalysisSignal {
                    kind: SignalKind::HighRetryCount,
                    detail: "test".into(),
                }],
                error_summary: None,
            },
            AttemptAnalysis {
                task_id: "t2".into(),
                outcome: AttemptOutcome::Failure,
                attempt_count: 1,
                duration_ms: None,
                signals: vec![AnalysisSignal {
                    kind: SignalKind::RepeatedError,
                    detail: "test".into(),
                }],
                error_summary: Some("OOM".into()),
            },
        ];
        let proposals = propose_playbooks(&analyses);
        for p in &proposals {
            assert!(
                !p.cited_task_ids.is_empty(),
                "proposal {} must cite motivating task IDs",
                p.id
            );
        }
    }
}
