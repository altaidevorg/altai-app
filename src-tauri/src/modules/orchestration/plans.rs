//! Execution plans and decision logs (plan §G3).
//!
//! Supports lightweight checked-in execution plans (markdown checklists)
//! and a durable decision log. Tasks and attempts link to plan items so
//! long tasks can resume from repository artifacts. Decisions remain
//! reviewable after session cleanup.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::ledger::{CreateDecisionRequest, DecisionEntry, LedgerResult, OrchestrationLedger};

// ---------------------------------------------------------------------------
// Plan types
// ---------------------------------------------------------------------------

/// The status of a single plan item.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanStatus {
    Done,
    InProgress,
    Pending,
    Blocked,
    Unknown,
}

impl PlanStatus {
    /// Parse a checkbox marker character into a status.
    ///
    /// `[x]` / `[X]` → Done, `[ ]` → Pending, `[~]` → InProgress,
    /// `[!]` → Blocked, anything else → Unknown.
    pub fn from_marker(marker: char) -> Self {
        match marker {
            'x' | 'X' => Self::Done,
            ' ' => Self::Pending,
            '~' => Self::InProgress,
            '!' => Self::Blocked,
            _ => Self::Unknown,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Done => "done",
            Self::InProgress => "in_progress",
            Self::Pending => "pending",
            Self::Blocked => "blocked",
            Self::Unknown => "unknown",
        }
    }
}

/// A single item parsed from an execution plan document.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanItem {
    pub id: String,
    pub title: String,
    pub status: PlanStatus,
    pub line_number: usize,
    pub raw_text: String,
    pub is_header: bool,
}

/// A parsed execution plan with its source metadata.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionPlan {
    pub source_path: String,
    pub revision: String,
    pub items: Vec<PlanItem>,
    pub parsed_at_ms: u64,
}

impl ExecutionPlan {
    /// Whether every item is in Done status.
    pub fn is_complete(&self) -> bool {
        let checkable: Vec<_> = self.items.iter().filter(|i| !i.is_header).collect();
        !checkable.is_empty() && checkable.iter().all(|i| i.status == PlanStatus::Done)
    }

    /// Count items by status (excludes header/section items).
    pub fn counts(&self) -> PlanCounts {
        let mut done = 0;
        let mut in_progress = 0;
        let mut pending = 0;
        let mut blocked = 0;
        for item in &self.items {
            if item.is_header {
                continue;
            }
            match item.status {
                PlanStatus::Done => done += 1,
                PlanStatus::InProgress => in_progress += 1,
                PlanStatus::Pending => pending += 1,
                PlanStatus::Blocked => blocked += 1,
                PlanStatus::Unknown => {}
            }
        }
        PlanCounts {
            total: self.items.iter().filter(|i| !i.is_header).count(),
            done,
            in_progress,
            pending,
            blocked,
        }
    }

    /// Find an item by ID (case-insensitive).
    pub fn find(&self, id: &str) -> Option<&PlanItem> {
        self.items.iter().find(|i| i.id.eq_ignore_ascii_case(id))
    }
}

/// Aggregate counts for a plan.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlanCounts {
    pub total: usize,
    pub done: usize,
    pub in_progress: usize,
    pub pending: usize,
    pub blocked: usize,
}

impl PlanCounts {
    /// Completion percentage (0–100). Returns 0 if no items.
    pub fn completion_pct(&self) -> u8 {
        if self.total == 0 {
            return 0;
        }
        ((self.done as f64 / self.total as f64) * 100.0) as u8
    }
}

// ---------------------------------------------------------------------------
// Plan freshness
// ---------------------------------------------------------------------------

/// Result of comparing the current plan revision to a previously stored one.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Freshness {
    /// Revision matches — plan unchanged since last check.
    Fresh,
    /// Revision differs — plan was updated.
    Stale { current: String, last_seen: String },
    /// No previous revision recorded.
    New,
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Parse a markdown execution plan from raw text.
///
/// Recognises:
/// - `#### ID. Title` headers → plan item with that ID and title.
/// - `- [x]`, `- [ ]`, `- [~]`, `- [!]` checkbox items.
/// - Bare items without headers get sequential IDs (`item-1`, `item-2`, …).
pub fn parse_plan(source_path: &str, revision: &str, content: &str) -> ExecutionPlan {
    let mut items = Vec::new();
    let mut current_header_id: Option<String> = None;
    let mut current_header_title: Option<String> = None;
    let mut auto_id = 0usize;

    for (line_idx, line) in content.lines().enumerate() {
        let line_no = line_idx + 1;
        let trimmed = line.trim();

        // Detect header lines: "#### ID. Title" or "### ID. Title" etc.
        if let Some(rest) = strip_header(trimmed) {
            let (id, title) = split_header(rest);
            if !id.is_empty() {
                current_header_id = Some(id.clone());
                current_header_title = Some(title.clone());
                items.push(PlanItem {
                    id,
                    title,
                    status: PlanStatus::Unknown,
                    line_number: line_no,
                    raw_text: trimmed.to_string(),
                    is_header: true,
                });
            }
            continue;
        }

        // Detect checkbox lines: "- [x] text" etc.
        if let Some((marker, text)) = parse_checkbox(trimmed) {
            let status = PlanStatus::from_marker(marker);
            // If we're under a header, this checkbox may belong to it.
            // Update the header item's status if it has Unknown status
            // and the checkbox text is "close" (first checkbox under header).
            if let Some(ref hdr_id) = current_header_id {
                if let Some(hdr_item) = items.iter_mut().rev().find(|i| i.id == *hdr_id) {
                    if hdr_item.status == PlanStatus::Unknown {
                        hdr_item.status = status;
                    }
                }
            }
            // Also add the checkbox as its own item.
            auto_id += 1;
            let id = format!("item-{auto_id}");
            let title = text.trim().to_string();
            // If under a header, prefix the id.
            if let Some(ref hdr_id) = current_header_id {
                items.push(PlanItem {
                    id: format!("{hdr_id}/{id}"),
                    title: if let Some(ref hdr_title) = current_header_title {
                        format!("{hdr_title}: {title}")
                    } else {
                        title
                    },
                    status,
                    line_number: line_no,
                    raw_text: trimmed.to_string(),
                    is_header: false,
                });
            } else {
                items.push(PlanItem {
                    id,
                    title,
                    status,
                    line_number: line_no,
                    raw_text: trimmed.to_string(),
                    is_header: false,
                });
            }
        }
    }

    ExecutionPlan {
        source_path: source_path.to_string(),
        revision: revision.to_string(),
        items,
        parsed_at_ms: now_ms(),
    }
}

/// Strip leading `#` characters from a header line. Returns the rest, or None.
fn strip_header(line: &str) -> Option<&str> {
    let rest = line.trim_start_matches('#').trim_start();
    if !line.starts_with('#') || rest.is_empty() {
        return None;
    }
    Some(rest)
}

/// Split "ID. Title" or "ID Title" into (id, title).
fn split_header(text: &str) -> (String, String) {
    // Look for a short ID token followed by a separator.
    // IDs: alphanumeric + hyphens, e.g., "G3", "O1", "B2-3".
    let mut id_end = 0;
    for (i, ch) in text.char_indices() {
        if ch.is_alphanumeric() || ch == '-' {
            id_end = i + ch.len_utf8();
        } else {
            break;
        }
    }
    if id_end == 0 {
        return (String::new(), text.to_string());
    }
    let id = text[..id_end].to_string();
    let rest = text[id_end..]
        .trim_start_matches(['.', ':', ' ', '\t'])
        .to_string();
    (id, rest)
}

/// Parse "- [x] text" → ('x', "text"). Returns None if not a checkbox line.
fn parse_checkbox(line: &str) -> Option<(char, &str)> {
    // Must start with "- [" or "* [".
    let after_dash = line
        .strip_prefix("- ")
        .or_else(|| line.strip_prefix("* "))?;
    let inner = after_dash.strip_prefix('[')?;
    let marker = inner.chars().next()?;
    let rest = &inner[marker.len_utf8()..];
    let text = rest.strip_prefix("] ")?;
    Some((marker, text))
}

// ---------------------------------------------------------------------------
// Freshness
// ---------------------------------------------------------------------------

/// Check whether the plan is fresh compared to a previously stored revision.
pub fn check_freshness(current_revision: &str, last_seen: Option<&str>) -> Freshness {
    match last_seen {
        None => Freshness::New,
        Some(prev) if prev == current_revision => Freshness::Fresh,
        Some(prev) => Freshness::Stale {
            current: current_revision.to_string(),
            last_seen: prev.to_string(),
        },
    }
}

// ---------------------------------------------------------------------------
// Decision log (ledger-backed)
// ---------------------------------------------------------------------------

/// High-level decision-log API backed by the orchestration ledger.
pub struct DecisionLog<'a> {
    ledger: &'a OrchestrationLedger,
}

impl<'a> DecisionLog<'a> {
    pub fn new(ledger: &'a OrchestrationLedger) -> Self {
        Self { ledger }
    }

    /// Record a decision. Returns the stored entry.
    pub fn record(&self, req: &CreateDecisionRequest) -> LedgerResult<DecisionEntry> {
        self.ledger.create_decision(req)
    }

    /// Fetch all decisions for a task.
    pub fn for_task(&self, task_id: &str) -> LedgerResult<Vec<DecisionEntry>> {
        self.ledger.decisions_for_task(task_id)
    }

    /// Fetch all decisions linked to a plan item.
    pub fn for_plan_item(&self, plan_item_id: &str) -> LedgerResult<Vec<DecisionEntry>> {
        self.ledger.decisions_for_plan_item(plan_item_id)
    }

    /// Fetch recent decisions across all tasks.
    pub fn recent(&self, limit: usize) -> LedgerResult<Vec<DecisionEntry>> {
        self.ledger.recent_decisions(limit)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Compute a simple content revision (hash) for plan content when git is not
/// available. Uses a basic FNV-1a hash.
pub fn content_revision(content: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in content.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

/// Load and parse a plan from a file path.
pub fn load_plan_file(path: &Path) -> ExecutionPlan {
    let content = std::fs::read_to_string(path).unwrap_or_default();
    let source = path.to_string_lossy().to_string();
    let revision = content_revision(&content);
    parse_plan(&source, &revision, &content)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::orchestration::ledger::OrchestrationLedger;

    // ---- parsing: headers + checkboxes ----

    #[test]
    fn parse_header_with_id() {
        let plan = parse_plan(
            "PLAN.md",
            "rev1",
            "#### G3. Execution plans\nSome detail.\n",
        );
        assert_eq!(plan.items.len(), 1);
        assert_eq!(plan.items[0].id, "G3");
        assert_eq!(plan.items[0].title, "Execution plans");
    }

    #[test]
    fn parse_header_updates_status_from_checkbox() {
        let plan = parse_plan(
            "PLAN.md",
            "rev1",
            "#### O1. Domain model\n- [x] Define TaskState machine\n",
        );
        let o1 = plan.find("O1").unwrap();
        assert_eq!(o1.status, PlanStatus::Done);
    }

    #[test]
    fn parse_pending_checkbox() {
        let plan = parse_plan("PLAN.md", "rev1", "- [ ] Not done yet\n");
        assert_eq!(plan.items.len(), 1);
        assert_eq!(plan.items[0].status, PlanStatus::Pending);
    }

    #[test]
    fn parse_blocked_and_in_progress() {
        let plan = parse_plan(
            "PLAN.md",
            "rev1",
            "- [!] Blocked by upstream\n- [~] Working on it\n",
        );
        assert_eq!(plan.items[0].status, PlanStatus::Blocked);
        assert_eq!(plan.items[1].status, PlanStatus::InProgress);
    }

    #[test]
    fn parse_nested_items_under_header() {
        let plan = parse_plan(
            "PLAN.md",
            "rev1",
            "#### G1. Readiness scan\n- [x] Score instructions\n- [x] Score architecture\n- [ ] Score browser\n",
        );
        let g1_items: Vec<_> = plan
            .items
            .iter()
            .filter(|i| i.id.starts_with("G1/"))
            .collect();
        assert_eq!(g1_items.len(), 3);
        assert_eq!(g1_items[2].status, PlanStatus::Pending);
    }

    #[test]
    fn parse_bare_items_get_auto_ids() {
        let plan = parse_plan("PLAN.md", "rev1", "- [x] First\n- [ ] Second\n");
        assert_eq!(plan.items[0].id, "item-1");
        assert_eq!(plan.items[1].id, "item-2");
    }

    // ---- completion ----

    #[test]
    fn all_done_is_complete() {
        let plan = parse_plan("PLAN.md", "rev1", "- [x] Done one\n- [x] Done two\n");
        assert!(plan.is_complete());
    }

    #[test]
    fn not_all_done_not_complete() {
        let plan = parse_plan("PLAN.md", "rev1", "- [x] Done\n- [ ] Pending\n");
        assert!(!plan.is_complete());
    }

    #[test]
    fn empty_plan_not_complete() {
        let plan = parse_plan("PLAN.md", "rev1", "# Just a title\nNo items.\n");
        assert!(!plan.is_complete());
        assert_eq!(plan.counts().completion_pct(), 0);
    }

    // ---- counts ----

    #[test]
    fn counts_aggregate_correctly() {
        let plan = parse_plan(
            "PLAN.md",
            "rev1",
            "#### A. Section\n- [x] Done\n- [~] WIP\n- [ ] Todo\n- [!] Stuck\n",
        );
        let counts = plan.counts();
        assert_eq!(counts.total, 4); // 4 checkboxes (header excluded from counts)
        assert_eq!(counts.done, 1);
        assert_eq!(counts.in_progress, 1);
        assert_eq!(counts.pending, 1);
        assert_eq!(counts.blocked, 1);
    }

    #[test]
    fn completion_pct() {
        let plan = parse_plan(
            "PLAN.md",
            "rev1",
            "- [x] One\n- [x] Two\n- [ ] Three\n- [ ] Four\n",
        );
        assert_eq!(plan.counts().completion_pct(), 50);
    }

    // ---- freshness ----

    #[test]
    fn freshness_new() {
        assert_eq!(check_freshness("rev1", None), Freshness::New);
    }

    #[test]
    fn freshness_fresh() {
        assert_eq!(check_freshness("rev1", Some("rev1")), Freshness::Fresh);
    }

    #[test]
    fn freshness_stale() {
        assert_eq!(
            check_freshness("rev2", Some("rev1")),
            Freshness::Stale {
                current: "rev2".into(),
                last_seen: "rev1".into(),
            }
        );
    }

    // ---- content revision determinism ----

    #[test]
    fn content_revision_deterministic() {
        assert_eq!(content_revision("hello"), content_revision("hello"));
        assert_ne!(content_revision("hello"), content_revision("world"));
    }

    // ---- find by id ----

    #[test]
    fn find_case_insensitive() {
        let plan = parse_plan("PLAN.md", "rev1", "#### g3. Title\n");
        assert!(plan.find("G3").is_some());
        assert!(plan.find("g3").is_some());
    }

    // ---- decision log (ledger-backed) ----

    #[test]
    fn decision_log_record_and_fetch() {
        let ledger = OrchestrationLedger::open_in_memory().unwrap();
        let log = DecisionLog::new(&ledger);

        let entry = log
            .record(&CreateDecisionRequest {
                task_id: Some("t-1".into()),
                attempt_id: Some("t-1-att-1".into()),
                plan_item_id: Some("G3".into()),
                decision: "Use FNV-1a for content hashing".into(),
                rationale: "Deterministic and dependency-free".into(),
                alternatives: vec!["SHA-256".into()],
            })
            .unwrap();

        assert!(!entry.id.is_empty());

        let for_task = log.for_task("t-1").unwrap();
        assert_eq!(for_task.len(), 1);
        assert_eq!(for_task[0].decision, "Use FNV-1a for content hashing");

        let for_plan = log.for_plan_item("G3").unwrap();
        assert_eq!(for_plan.len(), 1);

        let recent = log.recent(10).unwrap();
        assert_eq!(recent.len(), 1);
    }

    #[test]
    fn decision_log_multiple_tasks() {
        let ledger = OrchestrationLedger::open_in_memory().unwrap();
        let log = DecisionLog::new(&ledger);

        for i in 0..5 {
            log.record(&CreateDecisionRequest {
                task_id: Some(format!("t-{i}")),
                attempt_id: None,
                plan_item_id: None,
                decision: format!("Decision {i}"),
                rationale: "Because".into(),
                alternatives: vec![],
            })
            .unwrap();
        }

        assert_eq!(log.for_task("t-0").unwrap().len(), 1);
        assert_eq!(log.for_task("t-3").unwrap().len(), 1);
        assert_eq!(log.recent(3).unwrap().len(), 3);
    }

    // ---- load from file ----

    #[test]
    fn load_plan_file_roundtrip() {
        let dir = std::env::temp_dir().join(format!(
            "altai-plan-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("PLAN.md");
        std::fs::write(&path, "#### X1. Test\n- [x] Done\n").unwrap();

        let plan = load_plan_file(&path);
        assert_eq!(plan.find("X1").unwrap().status, PlanStatus::Done);
    }
}
