//! Agent Readiness scan (plan §G1).
//!
//! Scores a repository's readiness for autonomous agent work across nine
//! dimensions. Every score links to evidence (file paths found or missing).
//! Scans are strictly read-only and never modify the repository.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde::Serialize;

// ---------------------------------------------------------------------------
// Report types
// ---------------------------------------------------------------------------

/// Which dimension of agent readiness this score measures.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessCategory {
    Instructions,
    Architecture,
    TestBuild,
    Environment,
    Dependencies,
    Security,
    StaleInstructions,
    Worktree,
    Browser,
}

impl ReadinessCategory {
    pub fn name(self) -> &'static str {
        match self {
            Self::Instructions => "instructions",
            Self::Architecture => "architecture",
            Self::TestBuild => "test_build",
            Self::Environment => "environment",
            Self::Dependencies => "dependencies",
            Self::Security => "security",
            Self::StaleInstructions => "stale_instructions",
            Self::Worktree => "worktree",
            Self::Browser => "browser",
        }
    }

    pub fn all() -> &'static [ReadinessCategory] {
        &[
            Self::Instructions,
            Self::Architecture,
            Self::TestBuild,
            Self::Environment,
            Self::Dependencies,
            Self::Security,
            Self::StaleInstructions,
            Self::Worktree,
            Self::Browser,
        ]
    }
}

/// Evidence backing a score — a file or pattern that was found (or noted absent).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Evidence {
    pub path: String,
    pub detail: String,
}

/// Score for one readiness dimension.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryScore {
    pub category: ReadinessCategory,
    /// 0–100. Higher is better.
    pub score: u8,
    pub evidence: Vec<Evidence>,
    pub notes: Vec<String>,
}

/// The full readiness report for a repository.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadinessReport {
    pub repo_path: String,
    pub overall_score: u8,
    pub categories: Vec<CategoryScore>,
    pub recommendations: Vec<String>,
}

impl ReadinessReport {
    pub fn score_for(&self, category: ReadinessCategory) -> Option<u8> {
        self.categories
            .iter()
            .find(|c| c.category == category)
            .map(|c| c.score)
    }
}

// ---------------------------------------------------------------------------
// Scan
// ---------------------------------------------------------------------------

/// Scan a repository for agent readiness. Read-only — never writes.
pub fn scan(repo_path: &Path) -> ReadinessReport {
    let mut categories = Vec::new();
    let mut recommendations = Vec::new();

    let instructions = check_instructions(repo_path);
    let arch = check_architecture(repo_path);
    let test_build = check_test_build(repo_path);
    let environment = check_environment(repo_path);
    let dependencies = check_dependencies(repo_path);
    let security = check_security(repo_path);
    let stale = check_stale_instructions(repo_path);
    let worktree = check_worktree(repo_path);
    let browser = check_browser(repo_path);

    for score in [
        &instructions,
        &arch,
        &test_build,
        &environment,
        &dependencies,
        &security,
    ] {
        if score.score < 50 {
            recommendations.push(format!(
                "Improve {}: currently {}/100",
                score.category.name(),
                score.score
            ));
        }
    }
    if stale.score > 50 {
        recommendations.push(format!(
            "Resolve stale or conflicting instructions: {}/100 conflicts detected",
            stale.score
        ));
    }

    categories.push(instructions);
    categories.push(arch);
    categories.push(test_build);
    categories.push(environment);
    categories.push(dependencies);
    categories.push(security);
    categories.push(stale);
    categories.push(worktree);
    categories.push(browser);

    // Ensure every category has at least one evidence entry.
    for cat in &mut categories {
        if cat.evidence.is_empty() {
            cat.evidence.push(Evidence {
                path: "(scan)".into(),
                detail: "No matching files or patterns found".into(),
            });
        }
    }

    let overall = compute_overall(&categories);

    ReadinessReport {
        repo_path: repo_path.to_string_lossy().to_string(),
        overall_score: overall,
        categories,
        recommendations,
    }
}

fn compute_overall(categories: &[CategoryScore]) -> u8 {
    if categories.is_empty() {
        return 0;
    }
    let sum: u32 = categories
        .iter()
        .map(|c| {
            // StaleInstructions is inverted: 0 conflicts = good (100), high = bad.
            if c.category == ReadinessCategory::StaleInstructions {
                100 - c.score as u32
            } else {
                c.score as u32
            }
        })
        .sum();
    (sum / categories.len() as u32).min(100) as u8
}

// ---------------------------------------------------------------------------
// Individual checks
// ---------------------------------------------------------------------------

fn check_instructions(repo: &Path) -> CategoryScore {
    let mut score: u32 = 0;
    let mut evidence = Vec::new();
    let mut notes = Vec::new();

    if repo.join("AGENTS.md").exists() {
        score += 50;
        evidence.push(Evidence {
            path: "AGENTS.md".into(),
            detail: "Agent instructions file present".into(),
        });
    } else {
        notes.push("No AGENTS.md found. Agents will lack repository-specific guidance.".into());
    }

    if repo.join("README.md").exists() {
        score += 25;
        evidence.push(Evidence {
            path: "README.md".into(),
            detail: "Project README present".into(),
        });
    } else {
        notes.push("No README.md found.".into());
    }

    // Alternative instruction files.
    for alt in ["CLAUDE.md", ".kilo/", ".cursorrules"] {
        if repo.join(alt).exists() {
            score += 25;
            evidence.push(Evidence {
                path: alt.into(),
                detail: "Additional agent config present".into(),
            });
            break;
        }
    }

    CategoryScore {
        category: ReadinessCategory::Instructions,
        score: score.min(100) as u8,
        evidence,
        notes,
    }
}

fn check_architecture(repo: &Path) -> CategoryScore {
    let mut score: u32 = 0;
    let mut evidence = Vec::new();
    let mut notes = Vec::new();

    if repo.join("docs").is_dir() {
        score += 40;
        evidence.push(Evidence {
            path: "docs/".into(),
            detail: "Documentation directory present".into(),
        });
    } else {
        notes.push("No docs/ directory. Architecture context is harder to discover.".into());
    }

    for f in [
        "ARCHITECTURE.md",
        "docs/ARCHITECTURE.md",
        "docs/architecture.md",
    ] {
        if repo.join(f).exists() {
            score += 30;
            evidence.push(Evidence {
                path: f.into(),
                detail: "Architecture document present".into(),
            });
            break;
        }
    }

    // Implementation plans or design docs.
    if repo.join("docs").join("IMPLEMENTATION_PLAN.md").exists()
        || !glob_docs(repo, "docs/*PLAN*.md").is_empty()
    {
        score += 30;
        evidence.push(Evidence {
            path: "docs/*PLAN*.md".into(),
            detail: "Implementation plan found".into(),
        });
    }

    CategoryScore {
        category: ReadinessCategory::Architecture,
        score: score.min(100) as u8,
        evidence,
        notes,
    }
}

fn check_test_build(repo: &Path) -> CategoryScore {
    let mut score: u32 = 0;
    let mut evidence = Vec::new();
    let mut notes = Vec::new();

    let manifest_found = [
        "Cargo.toml",
        "package.json",
        "pyproject.toml",
        "go.mod",
        "pom.xml",
    ]
    .iter()
    .find(|f| repo.join(f).exists());

    if let Some(manifest) = manifest_found {
        score += 40;
        evidence.push(Evidence {
            path: manifest.to_string(),
            detail: "Build manifest present".into(),
        });
    } else {
        notes.push("No recognizable build manifest found.".into());
    }

    // Test files or directories.
    let test_indicators = [
        "tests/",
        "test/",
        "src/test/",
        "__tests__/",
        "spec/",
        "src-tauri/tests/",
    ];
    let test_found = test_indicators
        .iter()
        .filter(|d| repo.join(d).exists())
        .count();
    if test_found > 0 {
        score += 30;
        evidence.push(Evidence {
            path: format!("{test_found} test director(y/ies)"),
            detail: "Test structure detected".into(),
        });
    } else {
        notes.push("No test directories detected.".into());
    }

    // CI configuration.
    for ci in [".github/workflows/", ".gitlab-ci.yml", ".circleci/"] {
        if repo.join(ci).exists() {
            score += 30;
            evidence.push(Evidence {
                path: ci.into(),
                detail: "CI configuration present".into(),
            });
            break;
        }
    }

    CategoryScore {
        category: ReadinessCategory::TestBuild,
        score: score.min(100) as u8,
        evidence,
        notes,
    }
}

fn check_environment(repo: &Path) -> CategoryScore {
    let mut score: u32 = 0;
    let mut evidence = Vec::new();
    let mut notes = Vec::new();

    for f in [".env.example", ".env.template", ".env.sample"] {
        if repo.join(f).exists() {
            score += 40;
            evidence.push(Evidence {
                path: f.into(),
                detail: "Environment template present".into(),
            });
            break;
        }
    }
    if score == 0 {
        notes.push(
            "No .env.example or equivalent. Agents cannot discover required env vars.".into(),
        );
    }

    for f in [
        "docker-compose.yml",
        "docker-compose.yaml",
        "Dockerfile",
        "flake.nix",
    ] {
        if repo.join(f).exists() {
            score += 30;
            evidence.push(Evidence {
                path: f.into(),
                detail: "Container or reproducible environment present".into(),
            });
            break;
        }
    }

    for f in ["Makefile", "justfile", "Taskfile.yml"] {
        if repo.join(f).exists() {
            score += 30;
            evidence.push(Evidence {
                path: f.into(),
                detail: "Task runner present".into(),
            });
            break;
        }
    }

    CategoryScore {
        category: ReadinessCategory::Environment,
        score: score.min(100) as u8,
        evidence,
        notes,
    }
}

fn check_dependencies(repo: &Path) -> CategoryScore {
    let mut score: u32 = 0;
    let mut evidence = Vec::new();
    let mut notes = Vec::new();

    for f in [
        "Cargo.lock",
        "package-lock.json",
        "yarn.lock",
        "pnpm-lock.yaml",
        "poetry.lock",
    ] {
        if repo.join(f).exists() {
            score += 50;
            evidence.push(Evidence {
                path: f.into(),
                detail: "Lock file present (reproducible dependencies)".into(),
            });
            break;
        }
    }
    if score == 0 {
        notes.push("No lock file found. Dependency versions are not pinned.".into());
    }

    // Dependency documentation.
    for f in ["DEPENDENCIES.md", "docs/dependencies.md"] {
        if repo.join(f).exists() {
            score += 50;
            evidence.push(Evidence {
                path: f.into(),
                detail: "Dependency documentation present".into(),
            });
            break;
        }
    }

    CategoryScore {
        category: ReadinessCategory::Dependencies,
        score: score.min(100) as u8,
        evidence,
        notes,
    }
}

fn check_security(repo: &Path) -> CategoryScore {
    let mut score: u32 = 0;
    let mut evidence = Vec::new();
    let mut notes = Vec::new();

    if repo.join(".gitignore").exists() {
        score += 30;
        evidence.push(Evidence {
            path: ".gitignore".into(),
            detail: "Gitignore present".into(),
        });
    } else {
        notes.push("No .gitignore. Secrets and build artifacts may be committed.".into());
    }

    if repo.join("SECURITY.md").exists() {
        score += 40;
        evidence.push(Evidence {
            path: "SECURITY.md".into(),
            detail: "Security policy present".into(),
        });
    }

    // Check for obvious secret files that are not covered by gitignore.
    let risky = [".env", "secrets.json", "credentials.json"];
    let mut risky_found = false;
    for f in &risky {
        if repo.join(f).exists() && !is_gitignored(repo, f) {
            risky_found = true;
            evidence.push(Evidence {
                path: f.to_string(),
                detail: "Potential unignored secret file detected".into(),
            });
        }
    }
    if risky_found {
        notes.push(
            "Potential secret files found in the repo root. Verify .gitignore coverage.".into(),
        );
    } else {
        score += 30;
        evidence.push(Evidence {
            path: "(root)".into(),
            detail: "No obvious unignored secret files detected".into(),
        });
    }

    CategoryScore {
        category: ReadinessCategory::Security,
        score: score.min(100) as u8,
        evidence,
        notes,
    }
}

fn check_stale_instructions(repo: &Path) -> CategoryScore {
    // For stale instructions, a LOW score means few conflicts detected (good).
    // A HIGH score means many potential conflicts (bad).
    let mut conflicts: u32 = 0;
    let mut evidence = Vec::new();
    let mut notes = Vec::new();

    // Multiple competing instruction files.
    let instruction_files = ["AGENTS.md", "CLAUDE.md", ".cursorrules", "COPILOT.md"];
    let found: Vec<&str> = instruction_files
        .iter()
        .filter(|f| repo.join(f).exists())
        .copied()
        .collect();
    if found.len() > 1 {
        conflicts += 30;
        evidence.push(Evidence {
            path: found.join(", "),
            detail: format!("{} competing instruction files", found.len()),
        });
        notes.push("Multiple instruction files may conflict. Consolidate into AGENTS.md.".into());
    }

    // Both Makefile and justfile.
    if repo.join("Makefile").exists() && repo.join("justfile").exists() {
        conflicts += 20;
        evidence.push(Evidence {
            path: "Makefile + justfile".into(),
            detail: "Multiple task runners".into(),
        });
    }

    // Both docker-compose and flake.nix.
    if repo.join("docker-compose.yml").exists() && repo.join("flake.nix").exists() {
        conflicts += 20;
        evidence.push(Evidence {
            path: "docker-compose.yml + flake.nix".into(),
            detail: "Multiple environment definitions".into(),
        });
    }

    if conflicts == 0 {
        evidence.push(Evidence {
            path: "(scan)".into(),
            detail: "No conflicting instructions detected".into(),
        });
    }

    CategoryScore {
        category: ReadinessCategory::StaleInstructions,
        score: conflicts.min(100) as u8,
        evidence,
        notes,
    }
}

fn check_worktree(repo: &Path) -> CategoryScore {
    let mut score: u32 = 0;
    let mut evidence = Vec::new();
    let mut notes = Vec::new();

    if repo.join(".git").exists() {
        score += 50;
        evidence.push(Evidence {
            path: ".git/".into(),
            detail: "Git repository detected (worktree-compatible)".into(),
        });
    } else {
        notes.push("Not a git repository. Worktree isolation requires git.".into());
    }

    // Monorepo structure (common dirs suggest multi-package).
    let common_dirs = ["packages/", "apps/", "services/", "crates/"];
    let monorepo_count = common_dirs.iter().filter(|d| repo.join(d).is_dir()).count();
    if monorepo_count > 0 {
        score += 25;
        evidence.push(Evidence {
            path: format!("{monorepo_count} monorepo workspace dirs"),
            detail: "Monorepo structure detected".into(),
        });
    }

    // Workspace config.
    for f in ["Cargo.toml", "pnpm-workspace.yaml", "turbo.json"] {
        if repo.join(f).exists() {
            score += 25;
            evidence.push(Evidence {
                path: f.into(),
                detail: "Workspace configuration present".into(),
            });
            break;
        }
    }

    CategoryScore {
        category: ReadinessCategory::Worktree,
        score: score.min(100) as u8,
        evidence,
        notes,
    }
}

fn check_browser(repo: &Path) -> CategoryScore {
    let mut score: u32 = 0;
    let mut evidence = Vec::new();
    let mut notes = Vec::new();

    for f in [
        "playwright.config.ts",
        "playwright.config.js",
        "cypress.config.ts",
        "cypress.config.js",
    ] {
        if repo.join(f).exists() {
            score += 60;
            evidence.push(Evidence {
                path: f.into(),
                detail: "Browser testing framework configured".into(),
            });
            break;
        }
    }

    for d in ["e2e/", "cypress/", "tests/e2e/"] {
        if repo.join(d).is_dir() {
            score += 40;
            evidence.push(Evidence {
                path: format!("{d}/"),
                detail: "E2E test directory present".into(),
            });
            break;
        }
    }

    if score == 0 {
        notes.push(
            "No browser testing framework detected. UI changes cannot be verified automatically."
                .into(),
        );
    }

    CategoryScore {
        category: ReadinessCategory::Browser,
        score: score.min(100) as u8,
        evidence,
        notes,
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Glob for files matching a pattern in a directory. Returns matched paths.
fn glob_docs(repo: &Path, pattern: &str) -> Vec<PathBuf> {
    let parts: Vec<&str> = pattern.split('/').collect();
    if parts.len() != 2 {
        return Vec::new();
    }
    let dir = repo.join(parts[0]);
    let glob_pattern = parts[1];

    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };

    entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            let Ok(file_type) = e.file_type() else {
                return false;
            };
            if !file_type.is_file() || file_type.is_symlink() {
                return false;
            }
            let name = e.file_name();
            let name = name.to_string_lossy();
            glob_match(&name, glob_pattern)
        })
        .map(|e| e.path())
        .collect()
}

/// Simple glob: '*' matches any characters, '?' matches one.
fn glob_match(text: &str, pattern: &str) -> bool {
    let text: Vec<char> = text.chars().collect();
    let mut previous = vec![false; text.len() + 1];
    previous[0] = true;

    for token in pattern.chars() {
        let mut current = vec![false; text.len() + 1];
        if token == '*' {
            current[0] = previous[0];
        }
        for index in 1..=text.len() {
            current[index] = match token {
                '*' => previous[index] || current[index - 1],
                '?' => previous[index - 1],
                literal => previous[index - 1] && text[index - 1] == literal,
            };
        }
        previous = current;
    }

    previous[text.len()]
}

fn is_gitignored(repo: &Path, relative_path: &str) -> bool {
    std::process::Command::new("git")
        .args(["check-ignore", "--quiet", "--", relative_path])
        .current_dir(repo)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn temp_repo() -> PathBuf {
        static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "altai-readiness-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn touch(repo: &Path, file: &str) {
        let path = repo.join(file);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, "").unwrap();
    }

    // ---- empty repo ----

    #[test]
    fn empty_repo_has_low_scores() {
        let repo = temp_repo();
        let report = scan(&repo);
        assert!(report.overall_score < 30, "empty repo should score low");
        assert!(!report.recommendations.is_empty());
    }

    // ---- well-configured repo ----

    #[test]
    fn well_configured_repo_scores_high() {
        let repo = temp_repo();
        touch(&repo, "AGENTS.md");
        touch(&repo, "README.md");
        touch(&repo, "docs/ARCHITECTURE.md");
        touch(&repo, "Cargo.toml");
        touch(&repo, "Cargo.lock");
        touch(&repo, "tests/test_basic.rs");
        touch(&repo, ".github/workflows/ci.yml");
        touch(&repo, ".env.example");
        touch(&repo, "Makefile");
        touch(&repo, ".gitignore");
        touch(&repo, "SECURITY.md");
        touch(&repo, ".git/HEAD");

        let report = scan(&repo);
        assert!(
            report.overall_score >= 70,
            "well-configured repo should score 70+, got {}",
            report.overall_score
        );

        let instructions = report.score_for(ReadinessCategory::Instructions).unwrap();
        assert_eq!(instructions, 75, "AGENTS.md(50) + README(25) = 75");

        let test_build = report.score_for(ReadinessCategory::TestBuild).unwrap();
        assert_eq!(test_build, 100);

        let deps = report.score_for(ReadinessCategory::Dependencies).unwrap();
        assert_eq!(deps, 50); // Cargo.lock but no DEPENDENCIES.md
    }

    // ---- instructions ----

    #[test]
    fn only_readme_scores_25() {
        let repo = temp_repo();
        touch(&repo, "README.md");
        let report = scan(&repo);
        assert_eq!(
            report.score_for(ReadinessCategory::Instructions).unwrap(),
            25
        );
    }

    #[test]
    fn agents_md_and_readme_scores_75() {
        let repo = temp_repo();
        touch(&repo, "AGENTS.md");
        touch(&repo, "README.md");
        let report = scan(&repo);
        assert_eq!(
            report.score_for(ReadinessCategory::Instructions).unwrap(),
            75
        );
    }

    // ---- stale instructions ----

    #[test]
    fn competing_instruction_files_detected() {
        let repo = temp_repo();
        touch(&repo, "AGENTS.md");
        touch(&repo, "CLAUDE.md");
        touch(&repo, ".cursorrules");
        let report = scan(&repo);
        let stale = report
            .score_for(ReadinessCategory::StaleInstructions)
            .unwrap();
        assert!(
            stale >= 30,
            "3 competing files should trigger conflict score"
        );
    }

    #[test]
    fn no_conflicts_scores_zero() {
        let repo = temp_repo();
        touch(&repo, "AGENTS.md");
        let report = scan(&repo);
        assert_eq!(
            report
                .score_for(ReadinessCategory::StaleInstructions)
                .unwrap(),
            0
        );
    }

    // ---- security ----

    #[test]
    fn secret_file_in_root_flagged() {
        let repo = temp_repo();
        touch(&repo, ".env");
        let report = scan(&repo);
        let sec = report
            .categories
            .iter()
            .find(|c| c.category == ReadinessCategory::Security)
            .unwrap();
        assert!(sec.notes.iter().any(|n| n.contains("secret")));
    }

    #[test]
    fn gitignore_present_boosts_security() {
        let repo = temp_repo();
        touch(&repo, ".gitignore");
        let report = scan(&repo);
        assert!(report.score_for(ReadinessCategory::Security).unwrap() >= 30);
    }

    #[test]
    fn ignored_secret_file_is_not_flagged() {
        let repo = temp_repo();
        fs::write(repo.join(".gitignore"), ".env\n").unwrap();
        touch(&repo, ".env");
        std::process::Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(&repo)
            .status()
            .unwrap();

        let report = scan(&repo);
        let sec = report
            .categories
            .iter()
            .find(|c| c.category == ReadinessCategory::Security)
            .unwrap();
        assert!(!sec.notes.iter().any(|n| n.contains("secret")));
        assert_eq!(sec.score, 60);
    }

    #[test]
    fn source_directory_is_not_test_evidence() {
        let repo = temp_repo();
        touch(&repo, "Cargo.toml");
        touch(&repo, "src-tauri/src/lib.rs");

        let report = scan(&repo);
        assert_eq!(report.score_for(ReadinessCategory::TestBuild).unwrap(), 40);
    }

    #[test]
    fn glob_matches_the_entire_name() {
        assert!(glob_match("AUTH_PLAN.md", "*PLAN*.md"));
        assert!(glob_match("PLAN.md", "*PLAN*.md"));
        assert!(!glob_match("AUTH_PLAN.txt", "*PLAN*.md"));
        assert!(!glob_match("AUTH_PLAN.md.backup", "*PLAN*.md"));
        assert!(glob_match("aβ.md", "??.md"));
    }

    // ---- worktree ----

    #[test]
    fn git_repo_worktree_compatible() {
        let repo = temp_repo();
        touch(&repo, ".git/HEAD");
        let report = scan(&repo);
        assert!(report.score_for(ReadinessCategory::Worktree).unwrap() >= 50);
    }

    #[test]
    fn non_git_repo_low_worktree_score() {
        let repo = temp_repo();
        let report = scan(&repo);
        assert!(report.score_for(ReadinessCategory::Worktree).unwrap() < 50);
    }

    // ---- browser ----

    #[test]
    fn playwright_detected() {
        let repo = temp_repo();
        touch(&repo, "playwright.config.ts");
        let report = scan(&repo);
        assert!(report.score_for(ReadinessCategory::Browser).unwrap() >= 60);
    }

    #[test]
    fn no_browser_setup_scores_zero() {
        let repo = temp_repo();
        let report = scan(&repo);
        assert_eq!(report.score_for(ReadinessCategory::Browser).unwrap(), 0);
    }

    // ---- every score links to evidence ----

    #[test]
    fn every_category_has_evidence() {
        let repo = temp_repo();
        touch(&repo, "AGENTS.md");
        let report = scan(&repo);
        for cat in &report.categories {
            assert!(
                !cat.evidence.is_empty(),
                "category {:?} has no evidence",
                cat.category
            );
        }
    }

    // ---- scan is read-only ----

    #[test]
    fn scan_does_not_modify_repo() {
        let repo = temp_repo();
        touch(&repo, "AGENTS.md");
        let before: Vec<_> = std::fs::read_dir(&repo).unwrap().collect();
        scan(&repo);
        let after: Vec<_> = std::fs::read_dir(&repo).unwrap().collect();
        assert_eq!(
            before.len(),
            after.len(),
            "scan should not add or remove files"
        );
    }
}
