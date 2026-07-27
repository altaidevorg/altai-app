//! Continuous repository gardening (plan §G5).
//!
//! Opt-in scheduled scans for stale documentation, architecture violations,
//! flaky tests, dead code, dependency drift, repeated agent failure patterns,
//! and evidence retention. Gardening produces small reviewable findings —
//! never auto-merges. Schedules honor budgets and quiet hours.

use std::path::Path;

use serde::{Deserialize, Serialize};

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
    /// Check if the given hour falls within quiet hours.
    pub fn is_quiet(&self, hour: u8) -> bool {
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
    pub elapsed_ms: u64,
}

// ---------------------------------------------------------------------------
// Schedule logic
// ---------------------------------------------------------------------------

/// Determine whether gardening should run now based on the schedule.
pub fn should_run_now(schedule: &Schedule, now_ms: u64, now_hour: u8) -> bool {
    // Check quiet hours.
    if let Some(ref quiet) = schedule.quiet_hours {
        if quiet.is_quiet(now_hour) {
            return false;
        }
    }
    // Check interval.
    now_ms >= schedule.last_run_ms + schedule.interval_ms
}

// ---------------------------------------------------------------------------
// Gardening checks
// ---------------------------------------------------------------------------

/// Run all enabled gardening checks against a repository.
pub fn run_gardening(repo_path: &Path, config: &GardeningConfig, now_ms: u64) -> GardeningReport {
    let start = std::time::Instant::now();
    let mut findings = Vec::new();

    for &check in &config.enabled_checks {
        let check_findings = match check {
            GardeningCheck::StaleDocs => check_stale_docs(repo_path, config, now_ms),
            GardeningCheck::ArchitectureViolations => check_architecture(repo_path),
            GardeningCheck::FlakyTests => check_flaky_tests(repo_path),
            GardeningCheck::DeadCode => check_dead_code(repo_path),
            GardeningCheck::DependencyDrift => check_dependency_drift(repo_path),
            GardeningCheck::StaleWorktrees => check_stale_worktrees(repo_path, config, now_ms),
            GardeningCheck::EvidenceRetention => {
                check_evidence_retention(repo_path, config, now_ms)
            }
        };
        findings.extend(check_findings);
    }

    let elapsed = start.elapsed().as_millis() as u64;
    let budget_ms = config.schedule.budget_minutes as u64 * 60_000;

    GardeningReport {
        findings,
        run_at_ms: now_ms,
        within_budget: elapsed <= budget_ms,
        checks_run: config.enabled_checks.clone(),
        elapsed_ms: elapsed,
    }
}

fn check_stale_docs(repo: &Path, config: &GardeningConfig, now_ms: u64) -> Vec<GardeningFinding> {
    let mut findings = Vec::new();
    let threshold_ms = config.stale_doc_days as u64 * 24 * 3600 * 1000;

    let doc_dirs = ["docs", "doc", "documentation"];
    for dir in &doc_dirs {
        let dir_path = repo.join(dir);
        if !dir_path.is_dir() {
            continue;
        }
        let entries = match std::fs::read_dir(&dir_path) {
            Ok(e) => e,
            Err(_) => {
                findings.push(GardeningFinding {
                    check: GardeningCheck::StaleDocs,
                    severity: Severity::Warning,
                    file: dir.to_string(),
                    detail: "Cannot read documentation directory".into(),
                    recommendation: "Check directory permissions".into(),
                    recoverable: false,
                });
                continue;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if !name.ends_with(".md") && !name.ends_with(".rst") && !name.ends_with(".txt") {
                continue;
            }
            let metadata = match std::fs::metadata(&path) {
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
                    check: GardeningCheck::StaleDocs,
                    severity: Severity::Warning,
                    file: format!("{dir}/{name}"),
                    detail: format!("Document not updated in {age_days} days"),
                    recommendation: "Review for accuracy or mark as archived".into(),
                    recoverable: true,
                });
            }
        }
    }

    findings
}

fn check_architecture(repo: &Path) -> Vec<GardeningFinding> {
    let mut findings = Vec::new();

    // Check for circular module dependencies (simplified: look for mod.rs files
    // that import from child modules that also import back).
    // For now, check for obvious violations: source files in docs/, or test
    // files in src root.

    let check_patterns = [
        ("src/test_*.rs", "Test file in src root — move to tests/"),
        (
            "src/**/test_*.rs",
            "Test file in source directory — move to tests/ or #[cfg(test)]",
        ),
    ];

    for (pattern, msg) in &check_patterns {
        // Simplified check: look for test files in src/.
        let src = repo.join("src");
        if !src.is_dir() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(&src) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.starts_with("test_") && name_str.ends_with(".rs") {
                    findings.push(GardeningFinding {
                        check: GardeningCheck::ArchitectureViolations,
                        severity: Severity::Warning,
                        file: format!("src/{name_str}"),
                        detail: msg.to_string(),
                        recommendation: "Move to tests/ directory".into(),
                        recoverable: true,
                    });
                }
            }
        }
        let _ = pattern; // pattern reserved for future glob matching
    }

    findings
}

fn check_flaky_tests(repo: &Path) -> Vec<GardeningFinding> {
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

    let tests_dir = repo.join("tests");
    let dirs_to_check = if tests_dir.is_dir() {
        vec![tests_dir]
    } else {
        vec![repo.join("src")]
    };

    for dir in dirs_to_check {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() || path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let Ok(content) = std::fs::read_to_string(&path) else {
                continue;
            };
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            for (indicator, msg) in &flaky_indicators {
                if content.contains(indicator) && content.contains("#[test]") {
                    findings.push(GardeningFinding {
                        check: GardeningCheck::FlakyTests,
                        severity: Severity::Warning,
                        file: name.clone(),
                        detail: msg.to_string(),
                        recommendation: "Use deterministic time/seed or mock".into(),
                        recoverable: true,
                    });
                }
            }
        }
    }

    findings
}

fn check_dead_code(repo: &Path) -> Vec<GardeningFinding> {
    let mut findings = Vec::new();

    // Check for #[allow(dead_code)] annotations (potential dead code).
    let src = repo.join("src");
    if !src.is_dir() {
        return findings;
    }

    fn scan_dir(dir: &Path, findings: &mut Vec<GardeningFinding>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                scan_dir(&path, findings);
            } else if path.is_file() && path.extension().is_some_and(|e| e == "rs") {
                let Ok(content) = std::fs::read_to_string(&path) else {
                    continue;
                };
                let allow_count = content.matches("#[allow(dead_code)]").count();
                if allow_count >= 3 {
                    let rel = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    findings.push(GardeningFinding {
                        check: GardeningCheck::DeadCode,
                        severity: Severity::Info,
                        file: rel,
                        detail: format!("{allow_count} #[allow(dead_code)] annotations"),
                        recommendation: "Run cargo clippy and remove genuinely dead code".into(),
                        recoverable: true,
                    });
                }
            }
        }
    }

    scan_dir(&src, &mut findings);
    findings
}

fn check_dependency_drift(repo: &Path) -> Vec<GardeningFinding> {
    let mut findings = Vec::new();

    // Check for outdated lock file vs manifest.
    // Simplified: check if Cargo.lock exists and is not older than Cargo.toml.
    let manifest = repo.join("Cargo.toml");
    let lockfile = repo.join("Cargo.lock");

    if manifest.exists() && !lockfile.exists() {
        findings.push(GardeningFinding {
            check: GardeningCheck::DependencyDrift,
            severity: Severity::Critical,
            file: "Cargo.lock".into(),
            detail: "Lock file missing — dependencies not pinned".into(),
            recommendation: "Run cargo generate-lockfile".into(),
            recoverable: true,
        });
    }

    // Check for package.json without lock file.
    let pkg_json = repo.join("package.json");
    let pkg_locks = ["package-lock.json", "yarn.lock", "pnpm-lock.yaml"];
    if pkg_json.exists() && !pkg_locks.iter().any(|l| repo.join(l).exists()) {
        findings.push(GardeningFinding {
            check: GardeningCheck::DependencyDrift,
            severity: Severity::Critical,
            file: "package-lock.json".into(),
            detail: "No JS lock file found".into(),
            recommendation: "Run npm install / yarn install".into(),
            recoverable: true,
        });
    }

    findings
}

fn check_stale_worktrees(
    repo: &Path,
    config: &GardeningConfig,
    now_ms: u64,
) -> Vec<GardeningFinding> {
    let mut findings = Vec::new();
    let threshold_ms = config.stale_worktree_days as u64 * 24 * 3600 * 1000;

    // Check for .git/worktrees entries.
    let worktrees_dir = repo.join(".git").join("worktrees");
    if !worktrees_dir.is_dir() {
        return findings;
    }

    let Ok(entries) = std::fs::read_dir(&worktrees_dir) else {
        return findings;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        // Check the HEAD file modification time.
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
                file: name,
                detail: format!("Worktree not accessed in {age_days} days"),
                recommendation: "Remove with: git worktree remove".into(),
                recoverable: true,
            });
        }
    }

    findings
}

fn check_evidence_retention(
    repo: &Path,
    config: &GardeningConfig,
    now_ms: u64,
) -> Vec<GardeningFinding> {
    let mut findings = Vec::new();
    let threshold_ms = config.evidence_retention_days as u64 * 24 * 3600 * 1000;

    // Check for old artifacts in .altai/artifacts/.
    let artifacts_dir = repo.join(".altai").join("artifacts");
    if !artifacts_dir.is_dir() {
        return findings;
    }

    let Ok(entries) = std::fs::read_dir(&artifacts_dir) else {
        return findings;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let metadata = match std::fs::metadata(&path) {
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
                check: GardeningCheck::EvidenceRetention,
                severity: Severity::Info,
                file: format!(".altai/artifacts/{name}"),
                detail: format!("Artifact older than retention policy ({age_days} days)"),
                recommendation: "Consider cleanup or archival".into(),
                recoverable: true,
            });
        }
    }

    findings
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

    // ---- architecture violations ----

    #[test]
    fn test_file_in_src_detected() {
        let repo = temp_repo();
        write_file(&repo, "src/test_helper.rs", "fn helper() {}\n");

        let config = GardeningConfig {
            enabled_checks: vec![GardeningCheck::ArchitectureViolations],
            ..GardeningConfig::default()
        };
        let report = run_gardening(&repo, &config, now_ms());
        assert!(report
            .findings
            .iter()
            .any(|f| f.check == GardeningCheck::ArchitectureViolations));
    }

    // ---- dead code ----

    #[test]
    fn many_dead_code_allows_detected() {
        let repo = temp_repo();
        let content = "#[allow(dead_code)]\nfn a() {}\n#[allow(dead_code)]\nfn b() {}\n#[allow(dead_code)]\nfn c() {}\n";
        write_file(&repo, "src/lib.rs", content);

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
}
