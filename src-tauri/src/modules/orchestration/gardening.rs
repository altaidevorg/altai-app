//! Continuous repository gardening (plan §G5).
//!
//! Opt-in scheduled scans for stale documentation, architecture violations,
//! flaky tests, dead code, dependency drift, repeated agent failure patterns,
//! and evidence retention. Gardening produces small reviewable findings —
//! never auto-merges. Schedules honor budgets and quiet hours.

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::modules::workspace::{resolve_path, WorkspaceEnv, WorkspaceRegistry};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Which gardening check to run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GardeningCheck {
    StaleDocs,
    ArchitectureViolations,
    FlakyTests,
    DeadCode,
    DependencyDrift,
    StaleWorktrees,
    EvidenceRetention,
    RepeatedAgentFailures,
}

impl GardeningCheck {
    pub fn all() -> &'static [GardeningCheck] {
        &[
            Self::StaleDocs,
            Self::ArchitectureViolations,
            Self::FlakyTests,
            Self::DeadCode,
            Self::DependencyDrift,
            Self::StaleWorktrees,
            Self::EvidenceRetention,
            Self::RepeatedAgentFailures,
        ]
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::StaleDocs => "stale_docs",
            Self::ArchitectureViolations => "architecture_violations",
            Self::FlakyTests => "flaky_tests",
            Self::DeadCode => "dead_code",
            Self::DependencyDrift => "dependency_drift",
            Self::StaleWorktrees => "stale_worktrees",
            Self::EvidenceRetention => "evidence_retention",
            Self::RepeatedAgentFailures => "repeated_agent_failures",
        }
    }
}

/// Quiet hours configuration — gardening scans don't run during these times.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuietHours {
    /// Start hour (0–23, local time).
    pub start_hour: u8,
    /// End hour (0–23, local time).
    pub end_hour: u8,
}

impl QuietHours {
    fn validate(&self) -> Result<(), String> {
        if self.start_hour > 23 || self.end_hour > 23 {
            return Err("Quiet hours must use values from 0 through 23.".into());
        }
        if self.start_hour == self.end_hour {
            return Err("Quiet-hours start and end must be different.".into());
        }
        Ok(())
    }

    /// Check if the given hour falls within quiet hours.
    pub fn is_quiet(&self, hour: u8) -> bool {
        if hour > 23 || self.validate().is_err() {
            return false;
        }
        if self.start_hour <= self.end_hour {
            hour >= self.start_hour && hour < self.end_hour
        } else {
            // Wraps midnight, e.g., 22:00–06:00.
            hour >= self.start_hour || hour < self.end_hour
        }
    }
}

/// Schedule for recurring gardening.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Schedule {
    /// Interval between runs in milliseconds.
    pub interval_ms: u64,
    /// Last run timestamp (epoch ms).
    pub last_run_ms: u64,
    /// Maximum budget per run in minutes.
    pub budget_minutes: u32,
    /// Optional quiet hours.
    pub quiet_hours: Option<QuietHours>,
}

/// Full gardening configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GardeningConfig {
    pub enabled_checks: Vec<GardeningCheck>,
    pub schedule: Schedule,
    /// Stale doc threshold in days.
    pub stale_doc_days: u32,
    /// Evidence retention threshold in days.
    pub evidence_retention_days: u32,
    /// Stale worktree threshold in days.
    pub stale_worktree_days: u32,
}

impl Default for GardeningConfig {
    fn default() -> Self {
        Self {
            enabled_checks: GardeningCheck::all().to_vec(),
            schedule: Schedule {
                interval_ms: 24 * 3600 * 1000, // daily
                last_run_ms: 0,
                budget_minutes: 30,
                quiet_hours: None,
            },
            stale_doc_days: 90,
            evidence_retention_days: 30,
            stale_worktree_days: 7,
        }
    }
}

impl GardeningConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.schedule.interval_ms == 0 {
            return Err("Gardening interval must be greater than zero.".into());
        }
        if self.schedule.budget_minutes == 0 {
            return Err("Gardening budget must be at least one minute.".into());
        }
        if let Some(quiet_hours) = &self.schedule.quiet_hours {
            quiet_hours.validate()?;
        }
        if self.enabled_checks.len() > 64 {
            return Err("Gardening cannot configure more than 64 checks.".into());
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Findings
// ---------------------------------------------------------------------------

/// How urgent a finding is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    Critical,
}

/// A single gardening finding — something that needs attention.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GardeningFinding {
    pub check: GardeningCheck,
    pub severity: Severity,
    pub file: String,
    pub detail: String,
    pub recommendation: String,
    pub recoverable: bool,
}

/// The full gardening report.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GardeningReport {
    pub findings: Vec<GardeningFinding>,
    pub run_at_ms: u64,
    pub within_budget: bool,
    pub checks_run: Vec<GardeningCheck>,
    pub checks_skipped: Vec<GardeningCheck>,
    pub elapsed_ms: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GardeningRunResult {
    pub ran: bool,
    pub skip_reason: Option<String>,
    pub report: Option<GardeningReport>,
    pub proposals: Vec<GardeningTaskProposal>,
    /// The caller persists this schedule; `lastRunMs` advances only after a run.
    pub schedule: Schedule,
}

/// A bounded, review-only task proposal. Gardening never writes task files,
/// opens pull requests, or applies changes on its own.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GardeningTaskProposal {
    pub id: String,
    pub title: String,
    pub description: String,
    pub cited_files: Vec<String>,
    pub severity: Severity,
    pub status: &'static str,
}

/// A redacted, stable failure fingerprint supplied by the orchestration
/// projection. Raw logs are deliberately not accepted or emitted.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentFailureSample {
    pub task_id: String,
    pub fingerprint: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GardeningTickRequest {
    pub repo_path: String,
    pub config: GardeningConfig,
    pub now_ms: u64,
    pub now_hour: u8,
    pub force: bool,
    #[serde(default)]
    pub recent_failures: Vec<AgentFailureSample>,
}

// ---------------------------------------------------------------------------
// Schedule logic
// ---------------------------------------------------------------------------

/// Determine whether gardening should run now based on the schedule.
pub fn should_run_now(schedule: &Schedule, now_ms: u64, now_hour: u8) -> bool {
    if now_hour > 23 {
        return false;
    }
    // Check quiet hours.
    if let Some(ref quiet) = schedule.quiet_hours {
        if quiet.is_quiet(now_hour) {
            return false;
        }
    }
    // Check interval.
    now_ms >= schedule.last_run_ms.saturating_add(schedule.interval_ms)
}

/// Public tick endpoint for manual and scheduled callers. The command is
/// intentionally side-effect free apart from reading the selected repository:
/// it returns the advanced schedule so the existing settings layer can persist
/// it only after a completed run.
#[tauri::command]
pub async fn orchestration_gardening_tick(
    request: GardeningTickRequest,
    workspace: WorkspaceEnv,
    registry: State<'_, WorkspaceRegistry>,
) -> Result<GardeningRunResult, String> {
    let resolved = resolve_path(request.repo_path.trim(), &workspace);
    let repo_path = registry
        .canonicalize_cached(&resolved)
        .map_err(|error| format!("Cannot resolve gardening repository: {error}"))?;
    if !repo_path.is_dir() || !registry.is_authorized(&repo_path) {
        return Err("Gardening repository is outside the authorized workspace.".into());
    }
    gardening_tick_at(
        repo_path,
        request.config,
        request.now_ms,
        request.now_hour,
        request.force,
        request.recent_failures,
    )
    .await
}

async fn gardening_tick_at(
    repo_path: PathBuf,
    config: GardeningConfig,
    now_ms: u64,
    now_hour: u8,
    force: bool,
    recent_failures: Vec<AgentFailureSample>,
) -> Result<GardeningRunResult, String> {
    config.validate()?;
    if recent_failures.len() > 10_000
        || recent_failures
            .iter()
            .any(|sample| sample.task_id.len() > 512 || sample.fingerprint.len() > 256)
    {
        return Err("Recent failure samples exceed the gardening input limits.".into());
    }
    if now_hour > 23 {
        return Err("Current local hour must be from 0 through 23.".into());
    }

    if !force && !should_run_now(&config.schedule, now_ms, now_hour) {
        return Ok(GardeningRunResult {
            ran: false,
            skip_reason: Some("The interval has not elapsed or quiet hours are active.".into()),
            report: None,
            proposals: Vec::new(),
            schedule: config.schedule,
        });
    }

    let mut schedule = config.schedule.clone();
    let report = tauri::async_runtime::spawn_blocking(move || {
        run_gardening_with_failures(&repo_path, &config, now_ms, &recent_failures)
    })
    .await
    .map_err(|error| format!("Gardening worker failed: {error}"))?;
    schedule.last_run_ms = now_ms;
    let proposals = propose_gardening_tasks(&report, 5);
    Ok(GardeningRunResult {
        ran: true,
        skip_reason: None,
        report: Some(report),
        proposals,
        schedule,
    })
}

// ---------------------------------------------------------------------------
// Gardening checks
// ---------------------------------------------------------------------------

/// Run all enabled gardening checks against a repository.
pub fn run_gardening(repo_path: &Path, config: &GardeningConfig, now_ms: u64) -> GardeningReport {
    run_gardening_with_failures(repo_path, config, now_ms, &[])
}

pub fn run_gardening_with_failures(
    repo_path: &Path,
    config: &GardeningConfig,
    now_ms: u64,
    recent_failures: &[AgentFailureSample],
) -> GardeningReport {
    let start = Instant::now();
    let deadline = start
        .checked_add(Duration::from_secs(
            u64::from(config.schedule.budget_minutes).saturating_mul(60),
        ))
        .unwrap_or(start);
    let mut findings = Vec::new();
    let mut checks_run = Vec::new();
    let mut seen = HashSet::new();
    let enabled_checks: Vec<_> = config
        .enabled_checks
        .iter()
        .copied()
        .filter(|check| seen.insert(*check))
        .collect();

    for &check in &enabled_checks {
        if Instant::now() >= deadline {
            break;
        }
        let check_findings = match check {
            GardeningCheck::StaleDocs => check_stale_docs(repo_path, config, now_ms, deadline),
            GardeningCheck::ArchitectureViolations => check_architecture(repo_path, deadline),
            GardeningCheck::FlakyTests => check_flaky_tests(repo_path, deadline),
            GardeningCheck::DeadCode => check_dead_code(repo_path, deadline),
            GardeningCheck::DependencyDrift => check_dependency_drift(repo_path, deadline),
            GardeningCheck::StaleWorktrees => {
                check_stale_worktrees(repo_path, config, now_ms, deadline)
            }
            GardeningCheck::EvidenceRetention => {
                check_evidence_retention(repo_path, config, now_ms, deadline)
            }
            GardeningCheck::RepeatedAgentFailures => check_repeated_agent_failures(recent_failures),
        };
        findings.extend(check_findings);
        checks_run.push(check);
    }

    let elapsed = start.elapsed().as_millis() as u64;
    let budget_ms = config.schedule.budget_minutes as u64 * 60_000;
    let checks_skipped = enabled_checks
        .iter()
        .copied()
        .filter(|check| !checks_run.contains(check))
        .collect::<Vec<_>>();

    GardeningReport {
        findings,
        run_at_ms: now_ms,
        within_budget: checks_skipped.is_empty()
            && Instant::now() < deadline
            && elapsed <= budget_ms,
        checks_run,
        checks_skipped,
        elapsed_ms: elapsed,
    }
}

fn check_repeated_agent_failures(samples: &[AgentFailureSample]) -> Vec<GardeningFinding> {
    const REPEAT_THRESHOLD: usize = 3;
    let mut grouped: HashMap<&str, HashSet<&str>> = HashMap::new();
    for sample in samples {
        let fingerprint = sample.fingerprint.trim();
        let task_id = sample.task_id.trim();
        if fingerprint.is_empty() || task_id.is_empty() {
            continue;
        }
        grouped.entry(fingerprint).or_default().insert(task_id);
    }

    let mut findings = grouped
        .into_values()
        .filter(|task_ids| task_ids.len() >= REPEAT_THRESHOLD)
        .map(|task_ids| {
            let mut task_ids = task_ids.into_iter().collect::<Vec<_>>();
            task_ids.sort_unstable();
            GardeningFinding {
                check: GardeningCheck::RepeatedAgentFailures,
                severity: Severity::Warning,
                file: ".altai/tasks".into(),
                detail: format!(
                    "The same redacted failure fingerprint affected {} tasks: {}",
                    task_ids.len(),
                    task_ids.join(", ")
                ),
                recommendation: "Create a reviewable task to address the shared failure pattern"
                    .into(),
                recoverable: true,
            }
        })
        .collect::<Vec<_>>();
    findings.sort_by(|left, right| left.detail.cmp(&right.detail));
    findings
}

/// Convert findings into bounded, reviewable task proposals. At most one task
/// is proposed per check, so a large repository cannot flood the project board
/// with one task per file.
pub fn propose_gardening_tasks(
    report: &GardeningReport,
    max_proposals: usize,
) -> Vec<GardeningTaskProposal> {
    let mut proposals = Vec::new();
    for &check in GardeningCheck::all() {
        if proposals.len() >= max_proposals {
            break;
        }
        let matching = report
            .findings
            .iter()
            .filter(|finding| finding.check == check)
            .collect::<Vec<_>>();
        if matching.is_empty() {
            continue;
        }
        let severity = if matching
            .iter()
            .any(|finding| finding.severity == Severity::Critical)
        {
            Severity::Critical
        } else if matching
            .iter()
            .any(|finding| finding.severity == Severity::Warning)
        {
            Severity::Warning
        } else {
            Severity::Info
        };
        let mut cited_files = matching
            .iter()
            .map(|finding| finding.file.clone())
            .collect::<Vec<_>>();
        cited_files.sort();
        cited_files.dedup();
        proposals.push(GardeningTaskProposal {
            id: format!("gardening-{}-{}", check.name(), report.run_at_ms),
            title: format!("Repository gardening: {}", check.name().replace('_', " ")),
            description: format!(
                "Review and address {} {} finding(s). No changes have been applied.",
                matching.len(),
                check.name()
            ),
            cited_files,
            severity,
            status: "pending",
        });
    }
    proposals
}

const SKIPPED_DIRECTORIES: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    ".next",
    ".turbo",
];

fn relative_display(repo: &Path, path: &Path) -> String {
    path.strip_prefix(repo)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Walk regular files without following symlinks. Returning `false` means the
/// budget expired before the walk completed.
fn walk_files(root: &Path, deadline: Instant, mut visit: impl FnMut(&Path)) -> bool {
    if !root.is_dir() {
        return true;
    }
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        if Instant::now() >= deadline {
            return false;
        }
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            if Instant::now() >= deadline {
                return false;
            }
            let path = entry.path();
            let Ok(metadata) = std::fs::symlink_metadata(&path) else {
                continue;
            };
            if metadata.file_type().is_symlink() {
                continue;
            }
            if metadata.is_dir() {
                let skipped = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| SKIPPED_DIRECTORIES.contains(&name));
                if !skipped {
                    pending.push(path);
                }
            } else if metadata.is_file() {
                visit(&path);
            }
        }
    }
    true
}

fn collect_named_files(repo: &Path, file_name: &str, deadline: Instant) -> Vec<PathBuf> {
    let mut files = Vec::new();
    walk_files(repo, deadline, |path| {
        if path.file_name().and_then(|name| name.to_str()) == Some(file_name) {
            files.push(path.to_path_buf());
        }
    });
    files.sort();
    files.dedup();
    files
}

fn read_source_text(path: &Path) -> Option<String> {
    const MAX_SOURCE_BYTES: u64 = 4 * 1024 * 1024;
    let metadata = std::fs::metadata(path).ok()?;
    if metadata.len() > MAX_SOURCE_BYTES {
        return None;
    }
    std::fs::read_to_string(path).ok()
}

fn rust_roots(repo: &Path, deadline: Instant) -> Vec<PathBuf> {
    collect_named_files(repo, "Cargo.toml", deadline)
        .into_iter()
        .filter_map(|manifest| manifest.parent().map(Path::to_path_buf))
        .collect()
}

fn check_stale_docs(
    repo: &Path,
    config: &GardeningConfig,
    now_ms: u64,
    deadline: Instant,
) -> Vec<GardeningFinding> {
    let mut findings = Vec::new();
    let threshold_ms = config.stale_doc_days as u64 * 24 * 3600 * 1000;

    let doc_dirs = ["docs", "doc", "documentation"];
    for dir in &doc_dirs {
        if Instant::now() >= deadline {
            break;
        }
        let dir_path = repo.join(dir);
        if !dir_path.is_dir() {
            continue;
        }
        walk_files(&dir_path, deadline, |path| {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if !name.ends_with(".md") && !name.ends_with(".rst") && !name.ends_with(".txt") {
                return;
            }
            let metadata = match std::fs::metadata(path) {
                Ok(m) => m,
                Err(_) => return,
            };
            let modified = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            if modified > 0 && now_ms > modified && now_ms - modified > threshold_ms {
                let age_days = (now_ms - modified) / (24 * 3600 * 1000);
                findings.push(GardeningFinding {
                    check: GardeningCheck::StaleDocs,
                    severity: Severity::Warning,
                    file: relative_display(repo, path),
                    detail: format!("Document not updated in {age_days} days"),
                    recommendation: "Review for accuracy or mark as archived".into(),
                    recoverable: true,
                });
            }
        });
    }

    findings
}

fn check_architecture(repo: &Path, deadline: Instant) -> Vec<GardeningFinding> {
    let mut findings = Vec::new();
    for root in rust_roots(repo, deadline) {
        let source_root = root.join("src");
        walk_files(&source_root, deadline, |path| {
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                return;
            };
            if name.starts_with("test_") && name.ends_with(".rs") {
                findings.push(GardeningFinding {
                    check: GardeningCheck::ArchitectureViolations,
                    severity: Severity::Warning,
                    file: relative_display(repo, path),
                    detail: "Test file in a source directory".into(),
                    recommendation: "Move it to tests/ or use an inline #[cfg(test)] module".into(),
                    recoverable: true,
                });
            }
        });
    }
    findings.sort_by(|left, right| left.file.cmp(&right.file));
    findings.dedup_by(|left, right| left.file == right.file);
    findings
}

fn check_flaky_tests(repo: &Path, deadline: Instant) -> Vec<GardeningFinding> {
    let mut findings = Vec::new();

    // Look for common flaky test indicators: sleeps, random without seed,
    // time-dependent assertions.
    let flaky_indicators = [
        (
            "thread::sleep",
            "Possible flaky test: uses thread::sleep (timing-dependent)",
        ),
        (
            "SystemTime::now()",
            "Possible flaky test: depends on system clock",
        ),
        (
            "rand::random()",
            "Possible flaky test: non-deterministic randomness",
        ),
    ];

    for root in rust_roots(repo, deadline) {
        for dir in [root.join("src"), root.join("tests")] {
            walk_files(&dir, deadline, |path| {
                if path.extension().is_none_or(|extension| extension != "rs") {
                    return;
                }
                let Some(content) = read_source_text(path) else {
                    return;
                };
                if !content.contains("#[test]") {
                    return;
                }
                for (indicator, msg) in &flaky_indicators {
                    if content.contains(indicator) {
                        findings.push(GardeningFinding {
                            check: GardeningCheck::FlakyTests,
                            severity: Severity::Warning,
                            file: relative_display(repo, path),
                            detail: msg.to_string(),
                            recommendation: "Use deterministic time/seed or mock".into(),
                            recoverable: true,
                        });
                    }
                }
            });
        }
    }

    findings
}

fn check_dead_code(repo: &Path, deadline: Instant) -> Vec<GardeningFinding> {
    let mut findings = Vec::new();
    for root in rust_roots(repo, deadline) {
        walk_files(&root.join("src"), deadline, |path| {
            if path.extension().is_some_and(|extension| extension == "rs") {
                let Some(content) = read_source_text(path) else {
                    return;
                };
                let allow_count = content.matches("#[allow(dead_code)]").count();
                if allow_count >= 3 {
                    findings.push(GardeningFinding {
                        check: GardeningCheck::DeadCode,
                        severity: Severity::Info,
                        file: relative_display(repo, path),
                        detail: format!("{allow_count} #[allow(dead_code)] annotations"),
                        recommendation: "Run cargo clippy and remove genuinely dead code".into(),
                        recoverable: true,
                    });
                }
            }
        });
    }
    findings
}

fn ancestor_has_file(start: &Path, repo: &Path, names: &[&str]) -> bool {
    let mut current = Some(start);
    while let Some(directory) = current {
        if names.iter().any(|name| directory.join(name).is_file()) {
            return true;
        }
        if directory == repo {
            break;
        }
        current = directory.parent();
    }
    false
}

fn check_dependency_drift(repo: &Path, deadline: Instant) -> Vec<GardeningFinding> {
    let mut findings = Vec::new();
    for manifest in collect_named_files(repo, "Cargo.toml", deadline) {
        let Some(package_root) = manifest.parent() else {
            continue;
        };
        if ancestor_has_file(package_root, repo, &["Cargo.lock"]) {
            continue;
        }
        findings.push(GardeningFinding {
            check: GardeningCheck::DependencyDrift,
            severity: Severity::Critical,
            file: relative_display(repo, &package_root.join("Cargo.lock")),
            detail: format!("No Cargo.lock covers {}", relative_display(repo, &manifest)),
            recommendation: "Run cargo generate-lockfile".into(),
            recoverable: true,
        });
    }

    let pkg_locks = ["package-lock.json", "yarn.lock", "pnpm-lock.yaml"];
    for manifest in collect_named_files(repo, "package.json", deadline) {
        let Some(package_root) = manifest.parent() else {
            continue;
        };
        if ancestor_has_file(package_root, repo, &pkg_locks) {
            continue;
        }
        findings.push(GardeningFinding {
            check: GardeningCheck::DependencyDrift,
            severity: Severity::Critical,
            file: relative_display(repo, &package_root.join("package-lock.json")),
            detail: format!(
                "No JavaScript lock file covers {}",
                relative_display(repo, &manifest)
            ),
            recommendation: "Generate the lock file with the repository's package manager".into(),
            recoverable: true,
        });
    }

    findings
}

fn check_stale_worktrees(
    repo: &Path,
    config: &GardeningConfig,
    now_ms: u64,
    deadline: Instant,
) -> Vec<GardeningFinding> {
    let mut findings = Vec::new();
    let threshold_ms = config.stale_worktree_days as u64 * 24 * 3600 * 1000;

    let Some(common_git_dir) = resolve_common_git_dir(repo) else {
        return findings;
    };
    let worktrees_dir = common_git_dir.join("worktrees");
    if !worktrees_dir.is_dir() {
        return findings;
    }

    let Ok(entries) = std::fs::read_dir(&worktrees_dir) else {
        return findings;
    };

    for entry in entries.flatten() {
        if Instant::now() >= deadline {
            break;
        }
        let path = entry.path();
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let worktree_path = read_git_path(&path.join("gitdir"))
            .and_then(|git_file| git_file.parent().map(Path::to_path_buf));
        if worktree_path
            .as_ref()
            .and_then(|worktree| std::fs::canonicalize(worktree).ok())
            .zip(std::fs::canonicalize(repo).ok())
            .is_some_and(|(worktree, current)| worktree == current)
        {
            continue;
        }

        // HEAD mtime measures worktree metadata activity, not filesystem access.
        let head = path.join("HEAD");
        let metadata = match std::fs::metadata(&head) {
            Ok(m) => m,
            Err(_) => continue,
        };
        let modified = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        if modified > 0 && now_ms > modified && now_ms - modified > threshold_ms {
            let age_days = (now_ms - modified) / (24 * 3600 * 1000);
            findings.push(GardeningFinding {
                check: GardeningCheck::StaleWorktrees,
                severity: Severity::Warning,
                file: worktree_path
                    .as_ref()
                    .map_or(name, |worktree| worktree.to_string_lossy().to_string()),
                detail: format!("Worktree metadata has not changed in {age_days} days"),
                recommendation:
                    "Inspect for uncommitted work, then remove with `git worktree remove <path>`"
                        .into(),
                recoverable: false,
            });
        }
    }

    findings
}

fn check_evidence_retention(
    repo: &Path,
    config: &GardeningConfig,
    now_ms: u64,
    deadline: Instant,
) -> Vec<GardeningFinding> {
    let mut findings = Vec::new();
    let threshold_ms = config.evidence_retention_days as u64 * 24 * 3600 * 1000;

    // Check for old artifacts in .altai/artifacts/.
    let artifacts_dir = repo.join(".altai").join("artifacts");
    if !artifacts_dir.is_dir() {
        return findings;
    }

    walk_files(&artifacts_dir, deadline, |path| {
        let metadata = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(_) => return,
        };
        let modified = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        if modified > 0 && now_ms > modified && now_ms - modified > threshold_ms {
            let age_days = (now_ms - modified) / (24 * 3600 * 1000);
            findings.push(GardeningFinding {
                check: GardeningCheck::EvidenceRetention,
                severity: Severity::Info,
                file: relative_display(repo, path),
                detail: format!("Artifact older than retention policy ({age_days} days)"),
                recommendation: "Consider cleanup or archival".into(),
                recoverable: true,
            });
        }
    });

    findings
}

fn read_git_path(path: &Path) -> Option<PathBuf> {
    let value = std::fs::read_to_string(path).ok()?;
    let value = value
        .trim()
        .strip_prefix("gitdir: ")
        .unwrap_or(value.trim());
    let candidate = PathBuf::from(value);
    Some(if candidate.is_absolute() {
        candidate
    } else {
        path.parent()?.join(candidate)
    })
}

fn resolve_common_git_dir(repo: &Path) -> Option<PathBuf> {
    let dot_git = repo.join(".git");
    let git_dir = if dot_git.is_dir() {
        dot_git
    } else {
        read_git_path(&dot_git)?
    };
    let common_dir_file = git_dir.join("commondir");
    let common_dir = if !common_dir_file.is_file() {
        git_dir
    } else {
        let value = std::fs::read_to_string(common_dir_file).ok()?;
        let candidate = PathBuf::from(value.trim());
        if candidate.is_absolute() {
            candidate
        } else {
            git_dir.join(candidate)
        }
    };
    Some(std::fs::canonicalize(&common_dir).unwrap_or(common_dir))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn temp_repo() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "altai-garden-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_file(repo: &Path, rel: &str, content: &str) {
        let path = repo.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, content).unwrap();
    }

    fn now_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }

    // ---- schedule logic ----

    #[test]
    fn should_run_after_interval() {
        let schedule = Schedule {
            interval_ms: 1000,
            last_run_ms: 5000,
            budget_minutes: 10,
            quiet_hours: None,
        };
        assert!(should_run_now(&schedule, 6001, 12));
        assert!(!should_run_now(&schedule, 5999, 12));
    }

    #[test]
    fn schedule_math_saturates_instead_of_overflowing() {
        let schedule = Schedule {
            interval_ms: 10,
            last_run_ms: u64::MAX - 5,
            budget_minutes: 10,
            quiet_hours: None,
        };
        assert!(should_run_now(&schedule, u64::MAX, 12));
    }

    #[test]
    fn invalid_quiet_hours_are_rejected() {
        let config = GardeningConfig {
            schedule: Schedule {
                quiet_hours: Some(QuietHours {
                    start_hour: 24,
                    end_hour: 6,
                }),
                ..GardeningConfig::default().schedule
            },
            ..GardeningConfig::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn quiet_hours_block_runs() {
        let schedule = Schedule {
            interval_ms: 0,
            last_run_ms: 0,
            budget_minutes: 10,
            quiet_hours: Some(QuietHours {
                start_hour: 22,
                end_hour: 6,
            }),
        };
        assert!(
            !should_run_now(&schedule, 100_000, 23),
            "23:00 should be quiet"
        );
        assert!(
            !should_run_now(&schedule, 100_000, 2),
            "02:00 should be quiet"
        );
        assert!(
            should_run_now(&schedule, 100_000, 10),
            "10:00 should be active"
        );
    }

    #[test]
    fn quiet_hours_same_day() {
        let quiet = QuietHours {
            start_hour: 9,
            end_hour: 17,
        };
        assert!(quiet.is_quiet(10));
        assert!(quiet.is_quiet(16));
        assert!(!quiet.is_quiet(8));
        assert!(!quiet.is_quiet(17));
    }

    #[test]
    fn quiet_hours_wrap_midnight() {
        let quiet = QuietHours {
            start_hour: 22,
            end_hour: 6,
        };
        assert!(quiet.is_quiet(23));
        assert!(quiet.is_quiet(0));
        assert!(quiet.is_quiet(5));
        assert!(!quiet.is_quiet(6));
        assert!(!quiet.is_quiet(12));
    }

    // ---- stale docs ----

    #[test]
    fn stale_docs_detected() {
        let repo = temp_repo();
        write_file(&repo, "docs/guide.md", "# Old guide\n");
        // Manually set the file's modification time to the past.
        let path = repo.join("docs/guide.md");
        let old_time =
            std::time::SystemTime::now() - std::time::Duration::from_secs(100 * 24 * 3600);
        let _ = filetime::set_file_mtime(&path, filetime::FileTime::from_system_time(old_time));

        let config = GardeningConfig {
            stale_doc_days: 90,
            ..GardeningConfig::default()
        };
        let report = run_gardening(&repo, &config, now_ms());
        assert!(report
            .findings
            .iter()
            .any(|f| f.check == GardeningCheck::StaleDocs));
    }

    #[test]
    fn fresh_docs_not_flagged() {
        let repo = temp_repo();
        write_file(&repo, "docs/guide.md", "# Fresh guide\n");

        let config = GardeningConfig {
            stale_doc_days: 90,
            ..GardeningConfig::default()
        };
        let report = run_gardening(&repo, &config, now_ms());
        assert!(!report
            .findings
            .iter()
            .any(|f| f.check == GardeningCheck::StaleDocs));
    }

    #[test]
    fn nested_stale_docs_are_detected() {
        let repo = temp_repo();
        write_file(&repo, "docs/guides/old.md", "# Old guide\n");
        let path = repo.join("docs/guides/old.md");
        let old_time =
            std::time::SystemTime::now() - std::time::Duration::from_secs(100 * 24 * 3600);
        filetime::set_file_mtime(&path, filetime::FileTime::from_system_time(old_time)).unwrap();

        let report = run_gardening(
            &repo,
            &GardeningConfig {
                enabled_checks: vec![GardeningCheck::StaleDocs],
                ..GardeningConfig::default()
            },
            now_ms(),
        );
        assert_eq!(report.findings[0].file, "docs/guides/old.md");
    }

    // ---- architecture violations ----

    #[test]
    fn test_file_in_src_detected() {
        let repo = temp_repo();
        write_file(&repo, "Cargo.toml", "[package]\nname = \"test\"\n");
        write_file(&repo, "src/test_helper.rs", "fn helper() {}\n");

        let config = GardeningConfig {
            enabled_checks: vec![GardeningCheck::ArchitectureViolations],
            ..GardeningConfig::default()
        };
        let report = run_gardening(&repo, &config, now_ms());
        let architecture_findings = report
            .findings
            .iter()
            .filter(|finding| finding.check == GardeningCheck::ArchitectureViolations)
            .collect::<Vec<_>>();
        assert_eq!(architecture_findings.len(), 1);
        assert_eq!(architecture_findings[0].file, "src/test_helper.rs");
    }

    #[test]
    fn nested_rust_source_is_scanned_once() {
        let repo = temp_repo();
        write_file(
            &repo,
            "src-tauri/Cargo.toml",
            "[package]\nname = \"test\"\n",
        );
        write_file(
            &repo,
            "src-tauri/src/nested/test_helper.rs",
            "fn helper() {}\n",
        );
        let report = run_gardening(
            &repo,
            &GardeningConfig {
                enabled_checks: vec![
                    GardeningCheck::ArchitectureViolations,
                    GardeningCheck::ArchitectureViolations,
                ],
                ..GardeningConfig::default()
            },
            now_ms(),
        );
        assert_eq!(report.findings.len(), 1);
        assert_eq!(
            report.findings[0].file,
            "src-tauri/src/nested/test_helper.rs"
        );
        assert_eq!(
            report.checks_run,
            vec![GardeningCheck::ArchitectureViolations]
        );
    }

    // ---- dead code ----

    #[test]
    fn many_dead_code_allows_detected() {
        let repo = temp_repo();
        let content = "#[allow(dead_code)]\nfn a() {}\n#[allow(dead_code)]\nfn b() {}\n#[allow(dead_code)]\nfn c() {}\n";
        write_file(
            &repo,
            "src-tauri/Cargo.toml",
            "[package]\nname = \"test\"\n",
        );
        write_file(&repo, "src-tauri/src/lib.rs", content);

        let config = GardeningConfig {
            enabled_checks: vec![GardeningCheck::DeadCode],
            ..GardeningConfig::default()
        };
        let report = run_gardening(&repo, &config, now_ms());
        assert!(report
            .findings
            .iter()
            .any(|f| f.check == GardeningCheck::DeadCode));
    }

    #[test]
    fn flaky_tests_scan_nested_src_and_tests_directories() {
        let repo = temp_repo();
        write_file(
            &repo,
            "src-tauri/Cargo.toml",
            "[package]\nname = \"test\"\n",
        );
        write_file(
            &repo,
            "src-tauri/src/nested.rs",
            "#[test]\nfn unit() { std::thread::sleep(std::time::Duration::ZERO); }\n",
        );
        write_file(
            &repo,
            "src-tauri/tests/integration.rs",
            "#[test]\nfn integration() { let _ = std::time::SystemTime::now(); }\n",
        );
        let report = run_gardening(
            &repo,
            &GardeningConfig {
                enabled_checks: vec![GardeningCheck::FlakyTests],
                ..GardeningConfig::default()
            },
            now_ms(),
        );
        assert_eq!(report.findings.len(), 2);
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.file == "src-tauri/src/nested.rs"));
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.file == "src-tauri/tests/integration.rs"));
    }

    // ---- dependency drift ----

    #[test]
    fn missing_lock_file_detected() {
        let repo = temp_repo();
        write_file(&repo, "Cargo.toml", "[package]\nname = \"test\"\n");

        let config = GardeningConfig {
            enabled_checks: vec![GardeningCheck::DependencyDrift],
            ..GardeningConfig::default()
        };
        let report = run_gardening(&repo, &config, now_ms());
        assert!(report
            .findings
            .iter()
            .any(|f| f.check == GardeningCheck::DependencyDrift));
    }

    #[test]
    fn nested_cargo_manifest_uses_nested_lock_file() {
        let repo = temp_repo();
        write_file(
            &repo,
            "src-tauri/Cargo.toml",
            "[package]\nname = \"test\"\n",
        );
        write_file(&repo, "src-tauri/Cargo.lock", "version = 3\n");
        let report = run_gardening(
            &repo,
            &GardeningConfig {
                enabled_checks: vec![GardeningCheck::DependencyDrift],
                ..GardeningConfig::default()
            },
            now_ms(),
        );
        assert!(report.findings.is_empty());
    }

    #[test]
    fn lock_file_present_no_drift() {
        let repo = temp_repo();
        write_file(&repo, "Cargo.toml", "[package]\n");
        write_file(&repo, "Cargo.lock", "version = 3\n");

        let config = GardeningConfig {
            enabled_checks: vec![GardeningCheck::DependencyDrift],
            ..GardeningConfig::default()
        };
        let report = run_gardening(&repo, &config, now_ms());
        assert!(!report
            .findings
            .iter()
            .any(|f| f.check == GardeningCheck::DependencyDrift));
    }

    #[test]
    fn missing_js_lock_detected() {
        let repo = temp_repo();
        write_file(&repo, "package.json", "{\"name\":\"test\"}\n");

        let config = GardeningConfig {
            enabled_checks: vec![GardeningCheck::DependencyDrift],
            ..GardeningConfig::default()
        };
        let report = run_gardening(&repo, &config, now_ms());
        assert!(report
            .findings
            .iter()
            .any(|f| f.check == GardeningCheck::DependencyDrift));
    }

    // ---- evidence retention ----

    #[test]
    fn old_artifact_detected() {
        let repo = temp_repo();
        write_file(&repo, ".altai/artifacts/old.log", "data\n");
        let path = repo.join(".altai/artifacts/old.log");
        let old_time =
            std::time::SystemTime::now() - std::time::Duration::from_secs(60 * 24 * 3600);
        let _ = filetime::set_file_mtime(&path, filetime::FileTime::from_system_time(old_time));

        let config = GardeningConfig {
            enabled_checks: vec![GardeningCheck::EvidenceRetention],
            evidence_retention_days: 30,
            ..GardeningConfig::default()
        };
        let report = run_gardening(&repo, &config, now_ms());
        assert!(report
            .findings
            .iter()
            .any(|f| f.check == GardeningCheck::EvidenceRetention));
    }

    #[test]
    fn nested_artifact_is_detected() {
        let repo = temp_repo();
        write_file(&repo, ".altai/artifacts/task/attempt/old.log", "data\n");
        let path = repo.join(".altai/artifacts/task/attempt/old.log");
        let old_time =
            std::time::SystemTime::now() - std::time::Duration::from_secs(60 * 24 * 3600);
        filetime::set_file_mtime(&path, filetime::FileTime::from_system_time(old_time)).unwrap();
        let report = run_gardening(
            &repo,
            &GardeningConfig {
                enabled_checks: vec![GardeningCheck::EvidenceRetention],
                ..GardeningConfig::default()
            },
            now_ms(),
        );
        assert_eq!(
            report.findings[0].file,
            ".altai/artifacts/task/attempt/old.log"
        );
    }

    #[test]
    fn linked_worktree_resolves_common_git_directory() {
        let root = temp_repo();
        let repo = root.join("linked");
        let stale_worktree = root.join("stale");
        let common_git = root.join("main/.git");
        let current_admin = common_git.join("worktrees/current");
        let stale_admin = common_git.join("worktrees/stale");
        fs::create_dir_all(&repo).unwrap();
        fs::create_dir_all(&stale_worktree).unwrap();
        fs::create_dir_all(&current_admin).unwrap();
        fs::create_dir_all(&stale_admin).unwrap();
        write_file(
            &root,
            "linked/.git",
            &format!("gitdir: {}\n", current_admin.display()),
        );
        write_file(&root, "main/.git/worktrees/current/commondir", "../..\n");
        write_file(
            &root,
            "main/.git/worktrees/current/gitdir",
            &format!("{}\n", repo.join(".git").display()),
        );
        write_file(&root, "main/.git/worktrees/current/HEAD", "ref: current\n");
        write_file(
            &root,
            "main/.git/worktrees/stale/gitdir",
            &format!("{}\n", stale_worktree.join(".git").display()),
        );
        write_file(&root, "main/.git/worktrees/stale/HEAD", "ref: stale\n");
        let old_time =
            std::time::SystemTime::now() - std::time::Duration::from_secs(30 * 24 * 3600);
        filetime::set_file_mtime(
            stale_admin.join("HEAD"),
            filetime::FileTime::from_system_time(old_time),
        )
        .unwrap();

        assert_eq!(
            resolve_common_git_dir(&repo).unwrap(),
            fs::canonicalize(&common_git).unwrap()
        );
        let report = run_gardening(
            &repo,
            &GardeningConfig {
                enabled_checks: vec![GardeningCheck::StaleWorktrees],
                ..GardeningConfig::default()
            },
            now_ms(),
        );
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].file, stale_worktree.to_string_lossy());
        assert!(!report.findings[0].recoverable);
    }

    // ---- general report structure ----

    #[test]
    fn empty_repo_no_findings() {
        let repo = temp_repo();
        let report = run_gardening(&repo, &GardeningConfig::default(), now_ms());
        // Should produce no findings for an empty repo (no docs, no src, etc.).
        assert!(report
            .findings
            .iter()
            .all(|f| f.severity != Severity::Critical));
    }

    #[test]
    fn all_findings_are_recoverable_or_info() {
        let repo = temp_repo();
        write_file(&repo, "Cargo.toml", "[package]\n");
        let report = run_gardening(&repo, &GardeningConfig::default(), now_ms());
        for f in &report.findings {
            // Critical findings should be recoverable (cleanup is recoverable).
            if f.severity == Severity::Critical {
                assert!(
                    f.recoverable,
                    "Critical finding should be recoverable: {}",
                    f.file
                );
            }
        }
    }

    #[test]
    fn budget_not_exceeded_for_small_repos() {
        let repo = temp_repo();
        write_file(&repo, "docs/test.md", "# Test\n");
        let report = run_gardening(&repo, &GardeningConfig::default(), now_ms());
        assert!(report.within_budget, "small repo should be within budget");
    }

    #[test]
    fn zero_budget_skips_every_check() {
        let repo = temp_repo();
        let config = GardeningConfig {
            schedule: Schedule {
                budget_minutes: 0,
                ..GardeningConfig::default().schedule
            },
            ..GardeningConfig::default()
        };
        let report = run_gardening(&repo, &config, now_ms());
        assert!(!report.within_budget);
        assert!(report.checks_run.is_empty());
        assert_eq!(report.checks_skipped.len(), GardeningCheck::all().len());
    }

    #[test]
    fn repeated_failures_use_redacted_fingerprints_without_raw_logs() {
        let repo = temp_repo();
        let samples = ["task-a", "task-b", "task-c"]
            .into_iter()
            .map(|task_id| AgentFailureSample {
                task_id: task_id.into(),
                fingerprint: "redacted:dependency-missing".into(),
            })
            .collect::<Vec<_>>();
        let report = run_gardening_with_failures(
            &repo,
            &GardeningConfig {
                enabled_checks: vec![GardeningCheck::RepeatedAgentFailures],
                ..GardeningConfig::default()
            },
            now_ms(),
            &samples,
        );
        assert_eq!(report.findings.len(), 1);
        assert!(report.findings[0].detail.contains("3 tasks"));
        assert!(!report.findings[0].detail.contains("dependency-missing"));
    }

    #[test]
    fn findings_become_bounded_pending_task_proposals() {
        let report = GardeningReport {
            findings: vec![
                GardeningFinding {
                    check: GardeningCheck::StaleDocs,
                    severity: Severity::Warning,
                    file: "docs/a.md".into(),
                    detail: "stale".into(),
                    recommendation: "review".into(),
                    recoverable: true,
                },
                GardeningFinding {
                    check: GardeningCheck::StaleDocs,
                    severity: Severity::Warning,
                    file: "docs/b.md".into(),
                    detail: "stale".into(),
                    recommendation: "review".into(),
                    recoverable: true,
                },
                GardeningFinding {
                    check: GardeningCheck::DeadCode,
                    severity: Severity::Info,
                    file: "src/lib.rs".into(),
                    detail: "dead".into(),
                    recommendation: "review".into(),
                    recoverable: true,
                },
            ],
            run_at_ms: 42,
            within_budget: true,
            checks_run: vec![GardeningCheck::StaleDocs, GardeningCheck::DeadCode],
            checks_skipped: Vec::new(),
            elapsed_ms: 1,
        };
        let proposals = propose_gardening_tasks(&report, 1);
        assert_eq!(proposals.len(), 1);
        assert_eq!(proposals[0].status, "pending");
        assert_eq!(
            proposals[0].cited_files,
            vec!["docs/a.md".to_string(), "docs/b.md".to_string()]
        );
    }

    #[tokio::test]
    async fn tick_exposes_manual_run_and_advances_schedule() {
        let repo = temp_repo();
        let result = gardening_tick_at(repo, GardeningConfig::default(), 42, 12, true, Vec::new())
            .await
            .unwrap();
        assert!(result.ran);
        assert!(result.report.is_some());
        assert_eq!(result.schedule.last_run_ms, 42);
    }
}
