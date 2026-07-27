//! Integration coordinator (plan §F5).
//!
//! Detects overlapping diffs and dependency ordering, merges completed child
//! work into an integration worktree, runs combined verification, routes
//! conflicts to resolution, and preserves commit/evidence lineage.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::task_graph::TaskGraph;

// ---------------------------------------------------------------------------
// Diff overlap detection
// ---------------------------------------------------------------------------

/// The set of files a child task modified.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChildDiff {
    pub task_id: String,
    pub commit_id: String,
    pub modified_files: Vec<String>,
    pub added_files: Vec<String>,
    pub deleted_files: Vec<String>,
}

/// An overlap between two child diffs.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffOverlap {
    pub task_a: String,
    pub task_b: String,
    pub overlapping_files: Vec<String>,
    pub severity: OverlapSeverity,
}

/// How severe a file overlap is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OverlapSeverity {
    /// Both tasks modified the same file — potential conflict.
    ModifyModify,
    /// One task added a file the other deleted.
    AddDelete,
    /// Both tasks deleted the same file — harmless.
    DeleteDelete,
}

/// Detect file overlaps between child diffs.
pub fn detect_overlaps(diffs: &[ChildDiff]) -> Vec<DiffOverlap> {
    let mut overlaps = Vec::new();
    for i in 0..diffs.len() {
        for j in (i + 1)..diffs.len() {
            let a = &diffs[i];
            let b = &diffs[j];
            let overlap = classify_overlap(a, b);
            if !overlap.overlapping_files.is_empty() {
                overlaps.push(overlap);
            }
        }
    }
    overlaps
}

fn classify_overlap(a: &ChildDiff, b: &ChildDiff) -> DiffOverlap {
    let a_all: HashSet<&str> = a
        .modified_files
        .iter()
        .chain(&a.added_files)
        .chain(&a.deleted_files)
        .map(|s| s.as_str())
        .collect();
    let b_all: HashSet<&str> = b
        .modified_files
        .iter()
        .chain(&b.added_files)
        .chain(&b.deleted_files)
        .map(|s| s.as_str())
        .collect();

    let common: Vec<String> = a_all.intersection(&b_all).map(|s| s.to_string()).collect();

    if common.is_empty() {
        return DiffOverlap {
            task_a: a.task_id.clone(),
            task_b: b.task_id.clone(),
            overlapping_files: vec![],
            severity: OverlapSeverity::ModifyModify,
        };
    }

    // Classify the most severe type of overlap.
    let a_mod: HashSet<&str> = a.modified_files.iter().map(|s| s.as_str()).collect();
    let b_mod: HashSet<&str> = b.modified_files.iter().map(|s| s.as_str()).collect();
    let a_del: HashSet<&str> = a.deleted_files.iter().map(|s| s.as_str()).collect();
    let b_del: HashSet<&str> = b.deleted_files.iter().map(|s| s.as_str()).collect();
    let a_add: HashSet<&str> = a.added_files.iter().map(|s| s.as_str()).collect();
    let b_add: HashSet<&str> = b.added_files.iter().map(|s| s.as_str()).collect();

    let has_add_delete = common.iter().any(|f| {
        (a_add.contains(f.as_str()) && b_del.contains(f.as_str()))
            || (a_del.contains(f.as_str()) && b_add.contains(f.as_str()))
    });
    let has_modify_modify = common
        .iter()
        .any(|f| a_mod.contains(f.as_str()) && b_mod.contains(f.as_str()));
    let all_deleted = common
        .iter()
        .all(|f| a_del.contains(f.as_str()) && b_del.contains(f.as_str()));

    let severity = if has_add_delete {
        OverlapSeverity::AddDelete
    } else if has_modify_modify {
        OverlapSeverity::ModifyModify
    } else if all_deleted {
        OverlapSeverity::DeleteDelete
    } else {
        OverlapSeverity::ModifyModify
    };

    DiffOverlap {
        task_a: a.task_id.clone(),
        task_b: b.task_id.clone(),
        overlapping_files: common,
        severity,
    }
}

// ---------------------------------------------------------------------------
// Merge ordering
// ---------------------------------------------------------------------------

/// Compute the order in which child tasks should be merged based on the
/// dependency graph. Tasks with no dependencies are merged first.
pub fn merge_order(
    graph: &TaskGraph,
    completed: &HashSet<String>,
) -> Result<Vec<String>, MergeOrderError> {
    let eligible: Vec<String> = completed
        .iter()
        .filter(|t| graph.nodes.contains(*t))
        .cloned()
        .collect();

    if eligible.is_empty() {
        return Ok(Vec::new());
    }

    // Use topological order restricted to completed tasks.
    let topo = graph
        .topological_order()
        .map_err(|e| MergeOrderError::Cycle { cycle: e.cycle })?;

    Ok(topo.into_iter().filter(|t| completed.contains(t)).collect())
}

/// Error computing merge order.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum MergeOrderError {
    Cycle { cycle: Vec<String> },
}

// ---------------------------------------------------------------------------
// Integration result
// ---------------------------------------------------------------------------

/// The result of integrating multiple child tasks.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationResult {
    pub integrated_tasks: Vec<String>,
    pub skipped_tasks: Vec<SkippedTask>,
    pub conflicts: Vec<DiffOverlap>,
    pub final_revision: Option<String>,
    pub verification_status: VerificationStatus,
    pub lineage: Vec<CommitLineage>,
}

/// A task that was skipped during integration.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkippedTask {
    pub task_id: String,
    pub reason: SkipReason,
}

/// Why a task was skipped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SkipReason {
    NotCompleted,
    Blocked,
    ConflictEscalated,
}

/// The status of combined verification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    Passed,
    Failed,
    NotRun,
    ConflictEscalated,
}

/// Lineage tracking — connects integrated commits to their source tasks.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitLineage {
    pub task_id: String,
    pub child_commit: String,
    pub integrated_commit: Option<String>,
    pub evidence_artifact_ids: Vec<String>,
}

/// Plan an integration: which tasks to merge, in what order, and what to skip.
pub fn plan_integration(
    graph: &TaskGraph,
    diffs: &[ChildDiff],
    completed: &HashSet<String>,
) -> IntegrationPlan {
    // Compute merge order.
    let order = merge_order(graph, completed).unwrap_or_default();

    // Detect overlaps among completed tasks.
    let completed_diffs: Vec<&ChildDiff> = diffs
        .iter()
        .filter(|d| completed.contains(&d.task_id))
        .collect();
    let completed_diffs_owned: Vec<ChildDiff> =
        completed_diffs.iter().map(|d| (*d).clone()).collect();
    let overlaps = detect_overlaps(&completed_diffs_owned);

    // Determine which tasks have conflicts.
    let conflicting: HashSet<String> = overlaps
        .iter()
        .filter(|o| o.severity != OverlapSeverity::DeleteDelete)
        .flat_map(|o| [o.task_a.clone(), o.task_b.clone()])
        .collect();

    let mut integrated = Vec::new();
    let mut skipped = Vec::new();

    for task_id in &order {
        if conflicting.contains(task_id) {
            skipped.push(SkippedTask {
                task_id: task_id.clone(),
                reason: SkipReason::ConflictEscalated,
            });
        } else {
            integrated.push(task_id.clone());
        }
    }

    // Tasks in completed but not in order (unknown to graph).
    for task_id in completed {
        if !order.contains(task_id) && !conflicting.contains(task_id) {
            integrated.push(task_id.clone());
        }
    }

    let verification = if !overlaps.is_empty() && conflicting.iter().any(|t| integrated.contains(t))
    {
        VerificationStatus::ConflictEscalated
    } else {
        VerificationStatus::NotRun
    };

    IntegrationPlan {
        merge_order: integrated.clone(),
        skipped: skipped.clone(),
        overlaps: overlaps.clone(),
        verification,
    }
}

/// The plan for an integration — computed before the actual merge.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationPlan {
    pub merge_order: Vec<String>,
    pub skipped: Vec<SkippedTask>,
    pub overlaps: Vec<DiffOverlap>,
    pub verification: VerificationStatus,
}

// ---------------------------------------------------------------------------
// Conflict routing
// ---------------------------------------------------------------------------

/// What action to take when a conflict is detected.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictAction {
    /// Auto-resolvable (e.g., both deleted the same file).
    AutoResolved,
    /// Needs a dedicated resolution attempt.
    NeedsResolution,
    /// Escalate to NeedsAttention — no blind retries.
    Escalate,
}

/// Route a conflict to the appropriate action.
pub fn route_conflict(overlap: &DiffOverlap) -> ConflictAction {
    match overlap.severity {
        OverlapSeverity::DeleteDelete => ConflictAction::AutoResolved,
        OverlapSeverity::AddDelete => ConflictAction::Escalate,
        OverlapSeverity::ModifyModify => {
            // If only one file overlaps, try resolution. Multiple → escalate.
            if overlap.overlapping_files.len() <= 1 {
                ConflictAction::NeedsResolution
            } else {
                ConflictAction::Escalate
            }
        }
    }
}

/// Compute the conflict actions for all detected overlaps.
pub fn route_all_conflicts(overlaps: &[DiffOverlap]) -> Vec<(DiffOverlap, ConflictAction)> {
    overlaps
        .iter()
        .map(|o| (o.clone(), route_conflict(o)))
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_diff(
        task: &str,
        commit: &str,
        modified: &[&str],
        added: &[&str],
        deleted: &[&str],
    ) -> ChildDiff {
        ChildDiff {
            task_id: task.into(),
            commit_id: commit.into(),
            modified_files: modified.iter().map(|s| s.to_string()).collect(),
            added_files: added.iter().map(|s| s.to_string()).collect(),
            deleted_files: deleted.iter().map(|s| s.to_string()).collect(),
        }
    }

    // ---- overlap detection ----

    #[test]
    fn no_overlap_disjoint_files() {
        let diffs = vec![
            make_diff("t1", "c1", &["src/a.rs"], &[], &[]),
            make_diff("t2", "c2", &["src/b.rs"], &[], &[]),
        ];
        let overlaps = detect_overlaps(&diffs);
        assert!(overlaps.is_empty());
    }

    #[test]
    fn modify_modify_detected() {
        let diffs = vec![
            make_diff("t1", "c1", &["src/main.rs"], &[], &[]),
            make_diff("t2", "c2", &["src/main.rs"], &[], &[]),
        ];
        let overlaps = detect_overlaps(&diffs);
        assert_eq!(overlaps.len(), 1);
        assert_eq!(overlaps[0].severity, OverlapSeverity::ModifyModify);
        assert!(overlaps[0]
            .overlapping_files
            .contains(&"src/main.rs".to_string()));
    }

    #[test]
    fn add_delete_detected() {
        let diffs = vec![
            make_diff("t1", "c1", &[], &["new_file.rs"], &[]),
            make_diff("t2", "c2", &[], &[], &["new_file.rs"]),
        ];
        let overlaps = detect_overlaps(&diffs);
        assert_eq!(overlaps.len(), 1);
        assert_eq!(overlaps[0].severity, OverlapSeverity::AddDelete);
    }

    #[test]
    fn delete_delete_harmless() {
        let diffs = vec![
            make_diff("t1", "c1", &[], &[], &["old_file.rs"]),
            make_diff("t2", "c2", &[], &[], &["old_file.rs"]),
        ];
        let overlaps = detect_overlaps(&diffs);
        assert_eq!(overlaps.len(), 1);
        assert_eq!(overlaps[0].severity, OverlapSeverity::DeleteDelete);
    }

    #[test]
    fn added_files_no_conflict() {
        let diffs = vec![
            make_diff("t1", "c1", &[], &["src/new_a.rs"], &[]),
            make_diff("t2", "c2", &[], &["src/new_b.rs"], &[]),
        ];
        let overlaps = detect_overlaps(&diffs);
        assert!(overlaps.is_empty());
    }

    // ---- merge ordering ----

    #[test]
    fn merge_order_respects_dependencies() {
        let mut graph = TaskGraph::new();
        graph.add_dependency("B", "A").unwrap();
        graph.add_dependency("C", "B").unwrap();

        let completed: HashSet<String> = ["A".into(), "B".into(), "C".into()].into();
        let order = merge_order(&graph, &completed).unwrap();

        let a_pos = order.iter().position(|s| s == "A").unwrap();
        let b_pos = order.iter().position(|s| s == "B").unwrap();
        let c_pos = order.iter().position(|s| s == "C").unwrap();
        assert!(a_pos < b_pos);
        assert!(b_pos < c_pos);
    }

    #[test]
    fn merge_order_excludes_incomplete() {
        let mut graph = TaskGraph::new();
        graph.add_dependency("B", "A").unwrap();

        let completed: HashSet<String> = ["A".into()].into(); // B not done
        let order = merge_order(&graph, &completed).unwrap();
        assert!(order.contains(&"A".to_string()));
        assert!(!order.contains(&"B".to_string()));
    }

    #[test]
    fn merge_order_empty_when_none_completed() {
        let mut graph = TaskGraph::new();
        graph.add_node("A");

        let order = merge_order(&graph, &HashSet::new()).unwrap();
        assert!(order.is_empty());
    }

    // ---- integration planning ----

    #[test]
    fn plan_integration_no_conflicts() {
        let mut graph = TaskGraph::new();
        graph.add_node("A");
        graph.add_node("B");

        let diffs = vec![
            make_diff("A", "c1", &["src/a.rs"], &[], &[]),
            make_diff("B", "c2", &["src/b.rs"], &[], &[]),
        ];
        let completed: HashSet<String> = ["A".into(), "B".into()].into();

        let plan = plan_integration(&graph, &diffs, &completed);
        assert_eq!(plan.merge_order.len(), 2);
        assert!(plan.skipped.is_empty());
        assert!(plan.overlaps.is_empty());
    }

    #[test]
    fn plan_integration_skips_conflicting() {
        let mut graph = TaskGraph::new();
        graph.add_node("A");
        graph.add_node("B");

        let diffs = vec![
            make_diff("A", "c1", &["src/shared.rs"], &[], &[]),
            make_diff("B", "c2", &["src/shared.rs"], &[], &[]),
        ];
        let completed: HashSet<String> = ["A".into(), "B".into()].into();

        let plan = plan_integration(&graph, &diffs, &completed);
        // Both tasks conflict on src/shared.rs — both should be skipped.
        assert!(!plan.skipped.is_empty());
        assert!(plan
            .skipped
            .iter()
            .all(|s| s.reason == SkipReason::ConflictEscalated));
    }

    #[test]
    fn plan_integration_includes_non_conflicting() {
        let mut graph = TaskGraph::new();
        graph.add_node("A");
        graph.add_node("B");
        graph.add_node("C");

        let diffs = vec![
            make_diff("A", "c1", &["src/shared.rs"], &[], &[]),
            make_diff("B", "c2", &["src/shared.rs"], &[], &[]),
            make_diff("C", "c3", &["src/other.rs"], &[], &[]),
        ];
        let completed: HashSet<String> = ["A".into(), "B".into(), "C".into()].into();

        let plan = plan_integration(&graph, &diffs, &completed);
        // C has no conflicts — should be integrated.
        assert!(plan.merge_order.contains(&"C".to_string()));
    }

    // ---- conflict routing ----

    #[test]
    fn delete_delete_auto_resolved() {
        let overlap = DiffOverlap {
            task_a: "t1".into(),
            task_b: "t2".into(),
            overlapping_files: vec!["old.rs".into()],
            severity: OverlapSeverity::DeleteDelete,
        };
        assert_eq!(route_conflict(&overlap), ConflictAction::AutoResolved);
    }

    #[test]
    fn add_delete_escalated() {
        let overlap = DiffOverlap {
            task_a: "t1".into(),
            task_b: "t2".into(),
            overlapping_files: vec!["new.rs".into()],
            severity: OverlapSeverity::AddDelete,
        };
        assert_eq!(route_conflict(&overlap), ConflictAction::Escalate);
    }

    #[test]
    fn single_file_modify_modify_needs_resolution() {
        let overlap = DiffOverlap {
            task_a: "t1".into(),
            task_b: "t2".into(),
            overlapping_files: vec!["main.rs".into()],
            severity: OverlapSeverity::ModifyModify,
        };
        assert_eq!(route_conflict(&overlap), ConflictAction::NeedsResolution);
    }

    #[test]
    fn multi_file_modify_modify_escalated() {
        let overlap = DiffOverlap {
            task_a: "t1".into(),
            task_b: "t2".into(),
            overlapping_files: vec!["a.rs".into(), "b.rs".into(), "c.rs".into()],
            severity: OverlapSeverity::ModifyModify,
        };
        assert_eq!(route_conflict(&overlap), ConflictAction::Escalate);
    }

    // ---- lineage ----

    #[test]
    fn lineage_preserves_commit_and_evidence() {
        let lineage = CommitLineage {
            task_id: "t1".into(),
            child_commit: "abc123".into(),
            integrated_commit: Some("def456".into()),
            evidence_artifact_ids: vec!["art-1".into(), "art-2".into()],
        };
        assert_eq!(lineage.child_commit, "abc123");
        assert_eq!(lineage.integrated_commit.as_deref(), Some("def456"));
        assert_eq!(lineage.evidence_artifact_ids.len(), 2);
    }

    // ---- F5 acceptance ----

    #[test]
    fn successful_children_cannot_silently_overwrite() {
        let diffs = vec![
            make_diff("A", "c1", &["src/shared.rs"], &[], &[]),
            make_diff("B", "c2", &["src/shared.rs"], &[], &[]),
        ];
        let overlaps = detect_overlaps(&diffs);
        assert!(!overlaps.is_empty(), "overlapping diffs must be detected");
    }

    #[test]
    fn unresolved_conflicts_produce_needs_attention() {
        let overlap = DiffOverlap {
            task_a: "t1".into(),
            task_b: "t2".into(),
            overlapping_files: vec!["a.rs".into(), "b.rs".into()],
            severity: OverlapSeverity::ModifyModify,
        };
        let action = route_conflict(&overlap);
        assert_eq!(
            action,
            ConflictAction::Escalate,
            "multi-file conflicts should escalate to NeedsAttention, not retry"
        );
    }

    #[test]
    fn integration_plan_respects_dependencies() {
        let mut graph = TaskGraph::new();
        graph.add_dependency("B", "A").unwrap();

        let diffs = vec![
            make_diff("A", "c1", &["src/a.rs"], &[], &[]),
            make_diff("B", "c2", &["src/b.rs"], &[], &[]),
        ];
        let completed: HashSet<String> = ["A".into(), "B".into()].into();

        let plan = plan_integration(&graph, &diffs, &completed);
        let a_pos = plan.merge_order.iter().position(|s| s == "A").unwrap();
        let b_pos = plan.merge_order.iter().position(|s| s == "B").unwrap();
        assert!(a_pos < b_pos, "A must be merged before B");
    }
}
