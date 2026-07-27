//! Context-pack builder (plan §G2).
//!
//! Builds a bounded, deterministic context pack for a task by selecting
//! repository-owned documents relevant to the task description. Every
//! included source is recorded with its git revision. Stale links and
//! oversized instruction surfaces are detected.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::Serialize;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// The role a discovered document plays in the repository.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DocRole {
    AgentMap,
    Architecture,
    Product,
    Reliability,
    Security,
    ExecutionPlan,
    Test,
    Readme,
    Other,
}

/// A single entry in the context manifest — whether or not it was included.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextEntry {
    pub path: String,
    pub revision: String,
    pub bytes: usize,
    pub role: DocRole,
    pub included: bool,
    pub relevance: u16,
    pub reason: String,
}

/// A stale link found inside a document.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StaleLink {
    pub source: String,
    pub link_target: String,
}

/// The assembled context pack.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextPack {
    pub task_summary: String,
    pub manifest: Vec<ContextEntry>,
    pub total_bytes: usize,
    pub budget_bytes: usize,
    pub truncated: bool,
    pub warnings: Vec<String>,
    pub stale_links: Vec<StaleLink>,
}

/// Configuration for the builder.
#[derive(Clone, Debug)]
pub struct ContextConfig {
    pub budget_bytes: usize,
    /// Docs larger than this trigger an "oversized" warning.
    pub oversized_threshold: usize,
    /// Maximum depth to walk when discovering docs.
    pub max_depth: usize,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            budget_bytes: 48 * 1024, // 48 KB default budget
            oversized_threshold: 50 * 1024,
            max_depth: 3,
        }
    }
}

// ---------------------------------------------------------------------------
// Build
// ---------------------------------------------------------------------------

/// Build a context pack for the given task. Deterministic: the same repo +
/// task + config always produce the same manifest ordering and inclusions.
pub fn build_context_pack(
    repo_path: &Path,
    task_description: &str,
    config: &ContextConfig,
) -> ContextPack {
    let candidates = discover_documents(repo_path, config.max_depth);
    let task_keywords = extract_keywords(task_description);

    let mut entries: Vec<ContextEntry> = candidates
        .into_iter()
        .map(|(path, role)| {
            let rel = relative_path(repo_path, &path);
            let bytes = file_size(&path);
            let relevance = score_relevance(&path, &task_keywords);
            let revision = git_blob_hash(repo_path, &rel);
            ContextEntry {
                path: rel,
                revision,
                bytes,
                role,
                included: false, // set below
                relevance,
                reason: String::new(),
            }
        })
        .collect();

    // AGENTS.md always gets max relevance so it's packed first.
    for e in &mut entries {
        if e.role == DocRole::AgentMap {
            e.relevance = u16::MAX;
        }
    }

    // Sort: relevance desc, then role priority, then path for determinism.
    entries.sort_by(|a, b| {
        b.relevance
            .cmp(&a.relevance)
            .then(role_priority(a.role).cmp(&role_priority(b.role)))
            .then(a.path.cmp(&b.path))
    });

    // Greedy pack within budget.
    let mut total = 0usize;
    let mut warnings = Vec::new();
    let mut stale_links = Vec::new();

    for e in &mut entries {
        let abs = repo_path.join(&e.path);
        if total + e.bytes <= config.budget_bytes {
            e.included = true;
            e.reason = if e.role == DocRole::AgentMap {
                "agent map (always included)".into()
            } else if e.relevance > 0 {
                format!("relevant to task (score {})", e.relevance)
            } else {
                "included within budget".into()
            };
            total += e.bytes;

            // Check for stale links and oversized.
            if e.bytes > config.oversized_threshold {
                warnings.push(format!(
                    "Oversized document: {} ({} bytes)",
                    e.path, e.bytes
                ));
            }
            let links = find_stale_links(&abs, &e.path);
            stale_links.extend(links);
        } else {
            e.reason = format!(
                "excluded: would exceed budget ({} + {} > {})",
                total, e.bytes, config.budget_bytes
            );
        }
    }

    let truncated = entries.iter().any(|e| !e.included && e.relevance > 0);

    if entries.iter().filter(|e| e.included).count() == 0 {
        warnings.push("Context pack is empty — no documents were included.".into());
    }

    ContextPack {
        task_summary: task_description.to_string(),
        manifest: entries,
        total_bytes: total,
        budget_bytes: config.budget_bytes,
        truncated,
        warnings,
        stale_links,
    }
}

// ---------------------------------------------------------------------------
// Document discovery
// ---------------------------------------------------------------------------

/// Walk the repo (shallow) and discover candidate documents.
fn discover_documents(repo: &Path, max_depth: usize) -> Vec<(PathBuf, DocRole)> {
    let mut results = Vec::new();
    let skip_dirs: HashSet<&str> = [
        ".git",
        "node_modules",
        "target",
        "dist",
        "build",
        ".cache",
        "__pycache__",
        ".next",
    ]
    .into_iter()
    .collect();

    walk(repo, repo, max_depth, &skip_dirs, &mut results);
    results
}

fn walk(
    root: &Path,
    dir: &Path,
    depth_left: usize,
    skip: &HashSet<&str>,
    out: &mut Vec<(PathBuf, DocRole)>,
) {
    if depth_left == 0 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if path.is_dir() {
            if skip.contains(name_str.as_ref()) {
                continue;
            }
            walk(root, &path, depth_left - 1, skip, out);
        } else if path.is_file() {
            if let Some(role) = classify_file(&path, root) {
                out.push((path, role));
            }
        }
    }
}

/// Classify a file into a DocRole based on its name/path. Returns None if
/// the file is not a documentation candidate.
fn classify_file(path: &Path, root: &Path) -> Option<DocRole> {
    let rel = path.strip_prefix(root).unwrap_or(path);
    let rel_str = rel.to_string_lossy();
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let lower = file_name.to_lowercase();

    // Must be a markdown or text file.
    let is_doc = lower.ends_with(".md")
        || lower.ends_with(".txt")
        || lower.ends_with(".rst")
        || lower == "readme";
    if !is_doc {
        return None;
    }

    // Exact-name checks first.
    if file_name == "AGENTS.md" || lower == "agents.md" {
        return Some(DocRole::AgentMap);
    }
    if lower == "readme.md" || lower == "readme" {
        return Some(DocRole::Readme);
    }
    if lower.contains("security") {
        return Some(DocRole::Security);
    }
    if lower.contains("reliab") || lower.contains("incident") || lower.contains("runbook") {
        return Some(DocRole::Reliability);
    }
    if lower.contains("plan") || lower.contains("roadmap") || lower.contains("todo") {
        return Some(DocRole::ExecutionPlan);
    }
    if lower.contains("arch") || lower.contains("design") {
        return Some(DocRole::Architecture);
    }
    if lower.contains("product") || lower.contains("prd") || lower.contains("spec") {
        return Some(DocRole::Product);
    }
    if lower.contains("test") {
        return Some(DocRole::Test);
    }

    // Path-based checks.
    if rel_str.starts_with("docs/") || rel_str.starts_with("doc/") {
        if rel_str.contains("arch") {
            return Some(DocRole::Architecture);
        }
        if rel_str.contains("security") {
            return Some(DocRole::Security);
        }
        if rel_str.contains("plan") || rel_str.contains("roadmap") {
            return Some(DocRole::ExecutionPlan);
        }
        return Some(DocRole::Other);
    }

    None
}

// ---------------------------------------------------------------------------
// Relevance scoring
// ---------------------------------------------------------------------------

/// Extract lowercase keywords from a task description.
fn extract_keywords(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|s| s.len() >= 3)
        .map(|s| s.to_lowercase())
        .collect()
}

/// Score how relevant a file path is to the given keywords.
fn score_relevance(path: &Path, keywords: &[String]) -> u16 {
    if keywords.is_empty() {
        return 0;
    }
    let path_str = path.to_string_lossy().to_lowercase();
    let mut score: u16 = 0;
    for kw in keywords {
        if path_str.contains(kw) {
            score += 10;
        }
    }
    score
}

// ---------------------------------------------------------------------------
// Stale link detection
// ---------------------------------------------------------------------------

/// Find markdown links that point to non-existent files.
fn find_stale_links(file_path: &Path, source_name: &str) -> Vec<StaleLink> {
    let Ok(content) = std::fs::read_to_string(file_path) else {
        return Vec::new();
    };
    let parent = file_path.parent().unwrap_or(Path::new("."));

    let mut stale = Vec::new();
    for line in content.lines() {
        // Match [text](relative/path) — skip URLs.
        let mut rest = line;
        while let Some(start) = rest.find("](") {
            let after = &rest[start + 2..];
            if let Some(end) = after.find(')') {
                let target = &after[..end];
                if !target.starts_with("http")
                    && !target.starts_with("#")
                    && !target.starts_with("mailto:")
                {
                    // Strip optional anchor: `path#section` or `path?query`.
                    let clean = target.split(['#', '?']).next().unwrap_or(target);
                    if !clean.is_empty() && !parent.join(clean).exists() {
                        stale.push(StaleLink {
                            source: source_name.to_string(),
                            link_target: clean.to_string(),
                        });
                    }
                }
                rest = &after[end + 1..];
            } else {
                break;
            }
        }
    }
    stale
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn file_size(path: &Path) -> usize {
    std::fs::metadata(path)
        .map(|m| m.len() as usize)
        .unwrap_or(0)
}

fn relative_path(root: &Path, full: &Path) -> String {
    full.strip_prefix(root)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| full.to_string_lossy().to_string())
}

/// Get the git blob hash for a file. Returns "untracked" if git is unavailable
/// or the file is not tracked.
fn git_blob_hash(repo: &Path, rel_path: &str) -> String {
    let output = std::process::Command::new("git")
        .arg("hash-object")
        .arg(rel_path)
        .current_dir(repo)
        .output();

    match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        _ => "untracked".into(),
    }
}

fn role_priority(role: DocRole) -> u8 {
    match role {
        DocRole::AgentMap => 0,
        DocRole::Architecture => 1,
        DocRole::ExecutionPlan => 2,
        DocRole::Product => 3,
        DocRole::Reliability => 4,
        DocRole::Security => 5,
        DocRole::Test => 6,
        DocRole::Readme => 7,
        DocRole::Other => 8,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_repo() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "altai-ctx-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_file(repo: &Path, rel: &str, content: impl AsRef<str>) {
        let path = repo.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, content.as_ref()).unwrap();
    }

    // ---- determinism ----

    #[test]
    fn same_inputs_same_manifest() {
        let repo = temp_repo();
        write_file(&repo, "AGENTS.md", "# Agents\n");
        write_file(&repo, "docs/ARCHITECTURE.md", "# Arch\n");

        let cfg = ContextConfig::default();
        let task = "Implement the authentication module";
        let pack1 = build_context_pack(&repo, task, &cfg);
        let pack2 = build_context_pack(&repo, task, &cfg);

        assert_eq!(pack1.manifest.len(), pack2.manifest.len());
        for (a, b) in pack1.manifest.iter().zip(pack2.manifest.iter()) {
            assert_eq!(a.path, b.path);
            assert_eq!(a.included, b.included);
            assert_eq!(a.relevance, b.relevance);
        }
    }

    // ---- agents_md always included first ----

    #[test]
    fn agents_md_always_included_and_first() {
        let repo = temp_repo();
        write_file(&repo, "AGENTS.md", "# Agents\n");
        write_file(&repo, "README.md", "# Readme\n");

        let pack = build_context_pack(&repo, "unrelated task", &ContextConfig::default());
        let agents = pack.manifest.iter().find(|e| e.role == DocRole::AgentMap);
        assert!(agents.unwrap().included, "AGENTS.md must be included");

        // AGENTS.md should be first in manifest (highest priority).
        assert_eq!(pack.manifest[0].role, DocRole::AgentMap);
    }

    // ---- budget enforcement ----

    #[test]
    fn budget_enforced_before_dispatch() {
        let repo = temp_repo();
        write_file(&repo, "AGENTS.md", "x".repeat(100));
        write_file(&repo, "docs/large.md", "y".repeat(200));

        let cfg = ContextConfig {
            budget_bytes: 150,
            oversized_threshold: 1000,
            max_depth: 3,
        };
        let pack = build_context_pack(&repo, "large", &cfg);

        assert!(pack.total_bytes <= 150, "total must respect budget");
        assert!(
            pack.truncated,
            "should be truncated — relevant doc excluded"
        );
    }

    // ---- relevance scoring ----

    #[test]
    fn relevant_docs_ranked_higher() {
        let repo = temp_repo();
        write_file(&repo, "AGENTS.md", "# Agents\n");
        write_file(&repo, "docs/architecture.md", "# Arch\n");
        write_file(&repo, "docs/security.md", "# Sec\n");

        let pack = build_context_pack(&repo, "Review the architecture", &ContextConfig::default());
        let arch = pack
            .manifest
            .iter()
            .find(|e| e.path.contains("architecture"))
            .unwrap();
        let sec = pack
            .manifest
            .iter()
            .find(|e| e.path.contains("security"))
            .unwrap();
        assert!(
            arch.relevance > sec.relevance,
            "arch should be more relevant"
        );
    }

    // ---- stale link detection ----

    #[test]
    fn stale_links_detected() {
        let repo = temp_repo();
        write_file(
            &repo,
            "AGENTS.md",
            "# Agents\nSee [design](docs/design.md) and [missing](docs/missing.md)\n",
        );
        write_file(&repo, "docs/design.md", "# Design\n");

        let pack = build_context_pack(&repo, "task", &ContextConfig::default());
        assert!(
            pack.stale_links
                .iter()
                .any(|s| s.link_target.contains("missing")),
            "should detect stale link to missing.md"
        );
        assert!(
            !pack
                .stale_links
                .iter()
                .any(|s| s.link_target.contains("design")),
            "should NOT flag valid link to design.md"
        );
    }

    #[test]
    fn urls_not_flagged_as_stale() {
        let repo = temp_repo();
        write_file(
            &repo,
            "AGENTS.md",
            "# Agents\nSee [docs](https://example.com/docs)\n",
        );
        let pack = build_context_pack(&repo, "task", &ContextConfig::default());
        assert!(pack.stale_links.is_empty(), "URLs should not be flagged");
    }

    // ---- oversized warning ----

    #[test]
    fn oversized_document_warned() {
        let repo = temp_repo();
        write_file(&repo, "AGENTS.md", "x".repeat(60_000));

        let cfg = ContextConfig {
            oversized_threshold: 50_000,
            budget_bytes: 100_000,
            max_depth: 3,
        };
        let pack = build_context_pack(&repo, "task", &cfg);
        assert!(pack.warnings.iter().any(|w| w.contains("Oversized")));
    }

    // ---- revision recording ----

    #[test]
    fn entries_have_revision() {
        let repo = temp_repo();
        write_file(&repo, "AGENTS.md", "# Agents\n");
        let pack = build_context_pack(&repo, "task", &ContextConfig::default());
        for e in &pack.manifest {
            assert!(!e.revision.is_empty(), "revision must not be empty");
        }
    }

    // ---- manifest completeness ----

    #[test]
    fn manifest_records_included_and_excluded() {
        let repo = temp_repo();
        write_file(&repo, "AGENTS.md", "x".repeat(100));
        write_file(&repo, "docs/plan.md", "y".repeat(50));
        write_file(&repo, "docs/extra.md", "z".repeat(50));

        let cfg = ContextConfig {
            budget_bytes: 120, // enough for AGENTS + one doc
            oversized_threshold: 10_000,
            max_depth: 3,
        };
        let pack = build_context_pack(&repo, "plan", &cfg);
        let included: Vec<_> = pack.manifest.iter().filter(|e| e.included).collect();
        let excluded: Vec<_> = pack.manifest.iter().filter(|e| !e.included).collect();
        assert!(!included.is_empty());
        assert!(
            excluded.iter().all(|e| !e.reason.is_empty()),
            "excluded entries need a reason"
        );
    }

    // ---- no hidden external knowledge ----

    #[test]
    fn only_repo_files_in_manifest() {
        let repo = temp_repo();
        write_file(&repo, "AGENTS.md", "# Agents\n");
        let pack = build_context_pack(&repo, "task", &ContextConfig::default());
        for e in &pack.manifest {
            assert!(
                repo.join(&e.path).exists(),
                "manifest entry {} must exist in repo",
                e.path
            );
        }
    }

    // ---- empty repo ----

    #[test]
    fn empty_repo_warns() {
        let repo = temp_repo();
        let pack = build_context_pack(&repo, "task", &ContextConfig::default());
        assert!(pack.manifest.is_empty());
        assert!(pack.warnings.iter().any(|w| w.contains("empty")));
    }

    // ---- skip dirs honored ----

    #[test]
    fn node_modules_skipped() {
        let repo = temp_repo();
        write_file(&repo, "AGENTS.md", "# Agents\n");
        write_file(&repo, "node_modules/lib/README.md", "# Lib\n");

        let pack = build_context_pack(&repo, "task", &ContextConfig::default());
        assert!(
            !pack
                .manifest
                .iter()
                .any(|e| e.path.contains("node_modules")),
            "node_modules should be skipped"
        );
    }
}
