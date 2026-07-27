//! Command quality gates and review blocks (plan §D1-D2).
//!
//! D1: Parse required checks from config, classify results (pass/fail/timeout/
//! skipped), enforce gate (no ReadyForHandoff while checks fail).
//! D2: Structured review findings with severity, deduplication, and a bounded
//! correction loop.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// D1: Quality checks
// ---------------------------------------------------------------------------

/// A single configured check command.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckSpec {
    pub name: String,
    pub command: String,
    pub timeout_ms: u64,
    pub required: bool,
    pub retry_count: u32,
}

/// The result of executing a check.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckResult {
    pub name: String,
    /// Whether this check is required to pass the handoff gate.
    pub required: bool,
    pub status: CheckStatus,
    pub duration_ms: u64,
    pub output: String,
    pub exit_code: Option<i32>,
    pub attempts: u32,
}

/// The outcome of a check execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Passed,
    Failed,
    Timeout,
    Skipped,
    Unavailable,
}

impl CheckStatus {
    pub fn is_blocking(self) -> bool {
        matches!(self, Self::Failed | Self::Timeout | Self::Unavailable)
    }

    pub fn is_success(self) -> bool {
        matches!(self, Self::Passed | Self::Skipped)
    }
}

/// Aggregate gate result across all checks.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GateResult {
    pub passed: bool,
    pub total: usize,
    pub passed_count: usize,
    pub failed_count: usize,
    pub skipped_count: usize,
    pub blocking_failures: Vec<String>,
}

/// Evaluate whether the gate passes given a set of check results.
/// Required checks that fail, time out, or are unavailable block handoff.
pub fn evaluate_gate(results: &[CheckResult]) -> GateResult {
    let total = results.len();
    let mut passed_count = 0;
    let mut failed_count = 0;
    let mut skipped_count = 0;
    let mut blocking = Vec::new();

    for result in results {
        match result.status {
            CheckStatus::Passed => passed_count += 1,
            CheckStatus::Skipped => skipped_count += 1,
            CheckStatus::Failed | CheckStatus::Timeout => {
                failed_count += 1;
                if result.required {
                    blocking.push(result.name.clone());
                }
            }
            CheckStatus::Unavailable => {
                failed_count += 1;
                if result.required {
                    blocking.push(result.name.clone());
                }
            }
        }
    }

    let passed = blocking.is_empty();
    GateResult {
        passed,
        total,
        passed_count,
        failed_count,
        skipped_count,
        blocking_failures: blocking,
    }
}

/// Check evidence identity — links results to a specific commit/diff.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CheckEvidence {
    pub commit_sha: String,
    pub diff_hash: String,
    pub checked_at_ms: u64,
}

/// Determine if existing check evidence is still valid for the current commit.
/// A later edit invalidates prior evidence.
pub fn is_evidence_valid(
    existing: &CheckEvidence,
    current_commit: &str,
    current_diff_hash: &str,
) -> bool {
    existing.commit_sha == current_commit && existing.diff_hash == current_diff_hash
}

// ---------------------------------------------------------------------------
// D2: Automated reviewer
// ---------------------------------------------------------------------------

/// Severity of a review finding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingSeverity {
    Info,
    Style,
    Warning,
    Error,
    Blocker,
}

impl FindingSeverity {
    pub fn is_blocking(self) -> bool {
        matches!(self, Self::Error | Self::Blocker)
    }
}

/// A single review finding.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewFinding {
    pub id: String,
    pub severity: FindingSeverity,
    pub file: String,
    pub line: Option<u32>,
    pub message: String,
    pub evidence: String,
    pub suggested_fix: Option<String>,
    /// Which iteration of the correction loop produced this finding.
    pub iteration: u32,
    /// Whether this finding has been addressed.
    pub resolved: bool,
    /// Which attempt/turn resolved it.
    pub resolved_by: Option<String>,
}

/// The result of a review pass.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewResult {
    pub findings: Vec<ReviewFinding>,
    pub iteration: u32,
    pub blocking_count: usize,
    pub style_only: bool,
}

/// Deduplicate findings across review iterations. Findings at the same
/// file+line with the same message are merged (keeping the latest iteration).
pub fn deduplicate_findings(findings: Vec<ReviewFinding>) -> Vec<ReviewFinding> {
    let mut seen: std::collections::HashMap<(String, Option<u32>, String), ReviewFinding> =
        std::collections::HashMap::new();
    for finding in findings {
        let key = (finding.file.clone(), finding.line, finding.message.clone());
        seen.entry(key)
            .and_modify(|existing| {
                let was_resolved = existing.resolved;
                let previous_resolved_by = existing.resolved_by.clone();
                // Keep the higher iteration number.
                if finding.iteration > existing.iteration {
                    *existing = finding.clone();
                }
                // Resolution is monotonic across review iterations.
                existing.resolved = was_resolved || finding.resolved;
                if existing.resolved_by.is_none() {
                    existing.resolved_by =
                        previous_resolved_by.or_else(|| finding.resolved_by.clone());
                }
            })
            .or_insert(finding);
    }
    let mut result: Vec<ReviewFinding> = seen.into_values().collect();
    result.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then(a.line.cmp(&b.line))
            .then(a.message.cmp(&b.message))
    });
    result
}

/// Evaluate review findings for handoff readiness.
/// Blocking findings (Error/Blocker) prevent handoff unless configured to allow.
pub fn evaluate_review(findings: &[ReviewFinding], allow_style_blocking: bool) -> ReviewResult {
    let mut blocking_count = 0;
    let mut has_non_style = false;

    for finding in findings {
        if finding.resolved {
            continue;
        }
        if finding.severity.is_blocking() {
            blocking_count += 1;
            has_non_style = true;
        } else if finding.severity == FindingSeverity::Style {
            if allow_style_blocking {
                blocking_count += 1;
            }
        } else {
            has_non_style = true;
        }
    }

    let style_only = !has_non_style
        && findings
            .iter()
            .any(|f| !f.resolved && f.severity == FindingSeverity::Style);

    ReviewResult {
        findings: findings.to_vec(),
        iteration: findings.iter().map(|f| f.iteration).max().unwrap_or(0),
        blocking_count,
        style_only,
    }
}

/// Whether the correction loop should continue (unresolved blocking findings remain
/// and iterations haven't exceeded the limit).
pub fn should_continue_loop(
    findings: &[ReviewFinding],
    current_iteration: u32,
    max_iterations: u32,
) -> bool {
    let has_blocking = findings
        .iter()
        .any(|f| !f.resolved && f.severity.is_blocking());
    has_blocking && current_iteration < max_iterations
}

/// Mark a finding as resolved by a specific attempt.
pub fn resolve_finding(
    findings: &mut [ReviewFinding],
    finding_id: &str,
    resolved_by: &str,
) -> bool {
    let mut resolved = false;
    for f in findings.iter_mut() {
        if f.id == finding_id && !f.resolved {
            f.resolved = true;
            f.resolved_by = Some(resolved_by.to_string());
            resolved = true;
        }
    }
    resolved
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(name: &str, status: CheckStatus) -> CheckResult {
        CheckResult {
            name: name.into(),
            required: true,
            status,
            duration_ms: 100,
            output: "".into(),
            exit_code: Some(0),
            attempts: 1,
        }
    }

    fn make_finding(
        id: &str,
        severity: FindingSeverity,
        file: &str,
        line: Option<u32>,
        msg: &str,
        iter: u32,
    ) -> ReviewFinding {
        ReviewFinding {
            id: id.into(),
            severity,
            file: file.into(),
            line,
            message: msg.into(),
            evidence: "".into(),
            suggested_fix: None,
            iteration: iter,
            resolved: false,
            resolved_by: None,
        }
    }

    // ---- D1: gate evaluation ----

    #[test]
    fn all_passed_gate_passes() {
        let results = vec![
            make_result("lint", CheckStatus::Passed),
            make_result("test", CheckStatus::Passed),
        ];
        let gate = evaluate_gate(&results);
        assert!(gate.passed);
        assert_eq!(gate.blocking_failures.len(), 0);
    }

    #[test]
    fn failed_check_blocks_gate() {
        let results = vec![
            make_result("lint", CheckStatus::Passed),
            make_result("test", CheckStatus::Failed),
        ];
        let gate = evaluate_gate(&results);
        assert!(!gate.passed);
        assert_eq!(gate.blocking_failures, vec!["test"]);
    }

    #[test]
    fn timeout_blocks_gate() {
        let results = vec![make_result("e2e", CheckStatus::Timeout)];
        let gate = evaluate_gate(&results);
        assert!(!gate.passed);
    }

    #[test]
    fn skipped_does_not_block() {
        let results = vec![
            make_result("lint", CheckStatus::Passed),
            make_result("slow_test", CheckStatus::Skipped),
        ];
        let gate = evaluate_gate(&results);
        assert!(gate.passed);
        assert_eq!(gate.skipped_count, 1);
    }

    #[test]
    fn unavailable_blocks() {
        let results = vec![make_result("coverage", CheckStatus::Unavailable)];
        let gate = evaluate_gate(&results);
        assert!(!gate.passed);
    }

    #[test]
    fn optional_failure_is_reported_but_does_not_block() {
        let mut optional = make_result("coverage", CheckStatus::Failed);
        optional.required = false;

        let gate = evaluate_gate(&[optional]);
        assert!(gate.passed);
        assert_eq!(gate.failed_count, 1);
        assert!(gate.blocking_failures.is_empty());
    }

    // ---- D1: evidence validity ----

    #[test]
    fn evidence_valid_for_same_commit() {
        let evidence = CheckEvidence {
            commit_sha: "abc123".into(),
            diff_hash: "hash1".into(),
            checked_at_ms: 1000,
        };
        assert!(is_evidence_valid(&evidence, "abc123", "hash1"));
    }

    #[test]
    fn evidence_invalidated_by_new_commit() {
        let evidence = CheckEvidence {
            commit_sha: "abc123".into(),
            diff_hash: "hash1".into(),
            checked_at_ms: 1000,
        };
        assert!(!is_evidence_valid(&evidence, "def456", "hash1"));
    }

    #[test]
    fn evidence_invalidated_by_new_diff() {
        let evidence = CheckEvidence {
            commit_sha: "abc123".into(),
            diff_hash: "hash1".into(),
            checked_at_ms: 1000,
        };
        assert!(!is_evidence_valid(&evidence, "abc123", "hash2"));
    }

    // ---- D2: review findings ----

    #[test]
    fn blocking_findings_prevent_handoff() {
        let findings = vec![
            make_finding(
                "f1",
                FindingSeverity::Error,
                "src/main.rs",
                Some(42),
                "Bug",
                1,
            ),
            make_finding("f2", FindingSeverity::Info, "README.md", None, "Typo", 1),
        ];
        let review = evaluate_review(&findings, false);
        assert_eq!(review.blocking_count, 1);
    }

    #[test]
    fn resolved_findings_dont_block() {
        let mut findings = vec![make_finding(
            "f1",
            FindingSeverity::Blocker,
            "src/main.rs",
            Some(42),
            "Bug",
            1,
        )];
        resolve_finding(&mut findings, "f1", "att-2");
        let review = evaluate_review(&findings, false);
        assert_eq!(review.blocking_count, 0);
    }

    #[test]
    fn style_only_does_not_block_by_default() {
        let findings = vec![
            make_finding(
                "f1",
                FindingSeverity::Style,
                "src/a.rs",
                Some(1),
                "Indent",
                1,
            ),
            make_finding(
                "f2",
                FindingSeverity::Style,
                "src/b.rs",
                Some(2),
                "Naming",
                1,
            ),
        ];
        let review = evaluate_review(&findings, false);
        assert_eq!(review.blocking_count, 0);
        assert!(review.style_only);
    }

    #[test]
    fn style_can_block_when_configured() {
        let findings = vec![make_finding(
            "f1",
            FindingSeverity::Style,
            "src/a.rs",
            Some(1),
            "Indent",
            1,
        )];
        let review = evaluate_review(&findings, true);
        assert_eq!(review.blocking_count, 1);
    }

    // ---- D2: dedup ----

    #[test]
    fn dedup_merges_same_finding_across_iterations() {
        let findings = vec![
            make_finding("f1", FindingSeverity::Warning, "a.rs", Some(10), "Issue", 1),
            make_finding("f1", FindingSeverity::Warning, "a.rs", Some(10), "Issue", 2),
            make_finding("f1", FindingSeverity::Warning, "a.rs", Some(10), "Issue", 3),
        ];
        let deduped = deduplicate_findings(findings);
        assert_eq!(deduped.len(), 1);
        assert_eq!(deduped[0].iteration, 3); // latest iteration kept
    }

    #[test]
    fn dedup_keeps_different_findings() {
        let findings = vec![
            make_finding("f1", FindingSeverity::Error, "a.rs", Some(10), "Bug A", 1),
            make_finding("f2", FindingSeverity::Error, "a.rs", Some(20), "Bug B", 1),
            make_finding("f3", FindingSeverity::Warning, "b.rs", Some(5), "Warn", 1),
        ];
        let deduped = deduplicate_findings(findings);
        assert_eq!(deduped.len(), 3);
    }

    #[test]
    fn dedup_preserves_resolved_status() {
        let f1 = make_finding("f1", FindingSeverity::Error, "a.rs", Some(10), "Bug", 1);
        let mut f2 = make_finding("f1", FindingSeverity::Error, "a.rs", Some(10), "Bug", 2);
        f2.resolved = true;
        let deduped = deduplicate_findings(vec![f1, f2]);
        assert_eq!(deduped.len(), 1);
        assert!(deduped[0].resolved);
    }

    #[test]
    fn dedup_does_not_reopen_a_finding_resolved_in_an_older_iteration() {
        let mut resolved = make_finding("f1", FindingSeverity::Error, "a.rs", Some(10), "Bug", 1);
        resolved.resolved = true;
        resolved.resolved_by = Some("attempt-1".into());
        let newer = make_finding("f2", FindingSeverity::Error, "a.rs", Some(10), "Bug", 2);

        let deduped = deduplicate_findings(vec![resolved, newer]);
        assert_eq!(deduped.len(), 1);
        assert!(deduped[0].resolved);
        assert_eq!(deduped[0].resolved_by.as_deref(), Some("attempt-1"));
    }

    // ---- D2: correction loop ----

    #[test]
    fn loop_continues_with_unresolved_blocking() {
        let findings = vec![make_finding(
            "f1",
            FindingSeverity::Error,
            "a.rs",
            Some(1),
            "Bug",
            1,
        )];
        assert!(should_continue_loop(&findings, 1, 3));
    }

    #[test]
    fn loop_stops_when_all_resolved() {
        let mut findings = vec![make_finding(
            "f1",
            FindingSeverity::Error,
            "a.rs",
            Some(1),
            "Bug",
            1,
        )];
        resolve_finding(&mut findings, "f1", "att-2");
        assert!(!should_continue_loop(&findings, 1, 3));
    }

    #[test]
    fn loop_stops_at_max_iterations() {
        let findings = vec![make_finding(
            "f1",
            FindingSeverity::Blocker,
            "a.rs",
            Some(1),
            "Bug",
            1,
        )];
        assert!(!should_continue_loop(&findings, 3, 3));
    }

    // ---- severity ordering ----

    #[test]
    fn severity_ordering() {
        assert!(FindingSeverity::Blocker > FindingSeverity::Error);
        assert!(FindingSeverity::Error > FindingSeverity::Warning);
        assert!(FindingSeverity::Warning > FindingSeverity::Style);
        assert!(FindingSeverity::Style > FindingSeverity::Info);
    }

    #[test]
    fn only_error_and_blocker_are_blocking() {
        assert!(!FindingSeverity::Info.is_blocking());
        assert!(!FindingSeverity::Style.is_blocking());
        assert!(!FindingSeverity::Warning.is_blocking());
        assert!(FindingSeverity::Error.is_blocking());
        assert!(FindingSeverity::Blocker.is_blocking());
        assert!(CheckStatus::Unavailable.is_blocking());
    }
}
