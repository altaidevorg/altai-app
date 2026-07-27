//! Task source adapters (plan §4.2).
//!
//! A source normalizes work from an external system (local board, GitHub,
//! Linear, …) into [`SourceTask`] records the coordinator can claim. This slice
//! delivers the adapter contract and the always-available [`local::LocalTaskSource`].

pub mod local;
pub mod outbox;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;

use super::domain::TaskState;
use super::ledger::{OrchestrationLedger, TaskRecord};

/// Normalized task status in any source's native vocabulary. The four states
/// cover every local/remote board the first preview targets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceStatus {
    Todo,
    InProgress,
    Done,
    Cancelled,
}

impl SourceStatus {
    /// Active tasks are claimable candidates the coordinator may pick up.
    pub fn is_active(self) -> bool {
        matches!(self, SourceStatus::Todo | SourceStatus::InProgress)
    }

    /// Map to the durable domain state. Active work enters as `Queued` so the
    /// coordinator's claim path (`Queued → Planning → Running`) drives it.
    pub fn to_task_state(self) -> TaskState {
        match self {
            SourceStatus::Todo | SourceStatus::InProgress => TaskState::Queued,
            SourceStatus::Done => TaskState::Done,
            SourceStatus::Cancelled => TaskState::Cancelled,
        }
    }
}

/// A normalized task discovered by a source. Provider-agnostic: every source
/// reduces its native representation to this shape before reconciliation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceTask {
    pub id: String,
    pub title: String,
    pub status: SourceStatus,
    #[serde(default)]
    pub prompt: String,
}

/// What a source can write back to its origin (§4.2). Local sources are
/// read-only against the board (they manage state in the ledger); remote
/// sources may also post comments and close issues.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TaskSourceCapabilities {
    pub can_post_status: bool,
    pub can_post_comment: bool,
}

/// The source adapter contract (§4.2). Minimal for the first slice: discovery
/// and status mapping. `post_status`/`post_comment` land with the remote
/// sources that actually need them.
pub trait TaskSourceAdapter {
    /// Stable identifier stored as `TaskRecord.source_kind` (e.g. `"local"`).
    fn source_kind(&self) -> &'static str;

    /// Discover every task the source currently knows about, including terminal
    /// ones (so reconciliation can detect externally-completed work).
    fn list_all(&self) -> Result<Vec<SourceTask>, String>;

    /// Fetch a single task by its native id, if present.
    fn get_task(&self, native_id: &str) -> Result<Option<SourceTask>, String>;

    fn capabilities(&self) -> TaskSourceCapabilities;
}

/// Summary of one reconciliation pass.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ReconcileSummary {
    /// Active candidates discovered (claimable).
    pub candidates: usize,
    /// Tasks newly written or refreshed into the ledger.
    pub upserted: usize,
}

const MAX_NATIVE_ID_CHARS: usize = 512;
const MAX_TITLE_CHARS: usize = 4_096;
const MAX_PROMPT_CHARS: usize = 32_000;

fn validate_source_task(task: &SourceTask) -> Result<(), String> {
    if task.id.trim().is_empty() || task.id.chars().count() > MAX_NATIVE_ID_CHARS {
        return Err(format!(
            "Source task id must be non-empty and at most {MAX_NATIVE_ID_CHARS} characters."
        ));
    }
    if task.title.trim().is_empty() || task.title.chars().count() > MAX_TITLE_CHARS {
        return Err(format!(
            "Source task `{}` title must be non-empty and at most {MAX_TITLE_CHARS} characters.",
            task.id
        ));
    }
    if task.prompt.chars().count() > MAX_PROMPT_CHARS {
        return Err(format!(
            "Source task `{}` prompt cannot exceed {MAX_PROMPT_CHARS} characters.",
            task.id
        ));
    }
    Ok(())
}

/// Derive the stable ALTAI task identity from the full source namespace. Native
/// ids are not globally unique: two workspaces (or two providers) may both have
/// task `1`. Length-prefix each component before hashing to avoid ambiguous
/// concatenations.
pub fn task_id_for(workspace_key: &str, source_kind: &str, native_id: &str) -> String {
    let mut hash = Sha256::new();
    for component in [workspace_key, source_kind, native_id] {
        hash.update((component.len() as u64).to_be_bytes());
        hash.update(component.as_bytes());
    }
    format!("source:{}", hex::encode(hash.finalize()))
}

/// Upsert active candidates from `source` into the ledger as claimable tasks.
///
/// Only active (`Todo`/`InProgress`) tasks become `Queued` ledger records — the
/// coordinator claims from there. Terminal source tasks are intentionally not
/// upserted here: their ledger state is driven by the coordinator's own
/// terminal transitions, not by external mirroring (avoids races between an
/// external "done" and an in-flight attempt). Idempotent: re-running only
/// refreshes `updated_at_ms`.
pub fn reconcile_into(
    source: &dyn TaskSourceAdapter,
    ledger: &OrchestrationLedger,
    workspace_key: &str,
    now_ms: u64,
) -> Result<ReconcileSummary, String> {
    let tasks = source.list_all()?;
    let source_kind = source.source_kind();
    if source_kind.trim().is_empty() {
        return Err("Task source kind must be non-empty.".into());
    }
    if workspace_key.trim().is_empty() {
        return Err("Workspace key must be non-empty.".into());
    }

    // Validate the complete snapshot before mutating the ledger. This prevents
    // a malformed or duplicate task late in the source response from leaving a
    // partially reconciled board.
    let mut native_ids = HashSet::with_capacity(tasks.len());
    for task in &tasks {
        validate_source_task(task)?;
        if !native_ids.insert(task.id.as_str()) {
            return Err(format!(
                "Task source `{source_kind}` returned duplicate task id `{}`.",
                task.id
            ));
        }
    }

    let mut summary = ReconcileSummary::default();
    for task in tasks {
        if !task.status.is_active() {
            continue;
        }
        summary.candidates += 1;
        let native_id = task.id;
        let task_id = task_id_for(workspace_key, source_kind, &native_id);
        let record = TaskRecord {
            task_id: task_id.clone(),
            workspace_key: workspace_key.to_string(),
            source_kind: source_kind.to_string(),
            source_ref: native_id.clone(),
            title: task.title,
            description: task.prompt,
            state: task.status.to_task_state(),
            created_at_ms: now_ms,
            updated_at_ms: now_ms,
        };
        ledger
            .upsert_task(&record)
            .map_err(|error| format!("ledger upsert failed for task {native_id}: {error}"))?;
        summary.upserted += 1;
    }
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An in-memory source for testing the contract + reconcile logic without
    /// touching the filesystem.
    struct InMemorySource {
        tasks: Vec<SourceTask>,
    }

    impl TaskSourceAdapter for InMemorySource {
        fn source_kind(&self) -> &'static str {
            "memory"
        }
        fn list_all(&self) -> Result<Vec<SourceTask>, String> {
            Ok(self.tasks.clone())
        }
        fn get_task(&self, native_id: &str) -> Result<Option<SourceTask>, String> {
            Ok(self.tasks.iter().find(|t| t.id == native_id).cloned())
        }
        fn capabilities(&self) -> TaskSourceCapabilities {
            TaskSourceCapabilities::default()
        }
    }

    fn task(id: &str, status: SourceStatus) -> SourceTask {
        SourceTask {
            id: id.into(),
            title: format!("Task {id}"),
            status,
            prompt: String::new(),
        }
    }

    #[test]
    fn source_status_active_and_mapping() {
        assert!(SourceStatus::Todo.is_active());
        assert!(SourceStatus::InProgress.is_active());
        assert!(!SourceStatus::Done.is_active());
        assert!(!SourceStatus::Cancelled.is_active());
        assert_eq!(SourceStatus::Todo.to_task_state(), TaskState::Queued);
        assert_eq!(SourceStatus::Done.to_task_state(), TaskState::Done);
        assert_eq!(
            SourceStatus::Cancelled.to_task_state(),
            TaskState::Cancelled
        );
    }

    #[test]
    fn reconcile_upserts_only_active_candidates() {
        let ledger = OrchestrationLedger::open_in_memory().unwrap();
        let source = InMemorySource {
            tasks: vec![
                task("t-active", SourceStatus::Todo),
                task("t-running", SourceStatus::InProgress),
                task("t-done", SourceStatus::Done),
                task("t-cancelled", SourceStatus::Cancelled),
            ],
        };

        let summary = reconcile_into(&source, &ledger, "ws-1", 1_000).expect("reconcile");

        // Two active candidates; the two terminal tasks are skipped.
        assert_eq!(summary.candidates, 2);
        assert_eq!(summary.upserted, 2);

        let active_id = task_id_for("ws-1", "memory", "t-active");
        let active = ledger.task(&active_id).unwrap().expect("active present");
        assert_eq!(active.state, TaskState::Queued);
        assert_eq!(active.workspace_key, "ws-1");
        assert_eq!(active.source_kind, "memory");
        assert_eq!(active.source_ref, "t-active");
        let running_id = task_id_for("ws-1", "memory", "t-running");
        let running = ledger.task(&running_id).unwrap().expect("running present");
        assert_eq!(running.state, TaskState::Queued);
        // Terminal source tasks were not mirrored into the ledger.
        assert!(ledger
            .task(&task_id_for("ws-1", "memory", "t-done"))
            .unwrap()
            .is_none());
        assert!(ledger
            .task(&task_id_for("ws-1", "memory", "t-cancelled"))
            .unwrap()
            .is_none());
    }

    #[test]
    fn reconcile_is_idempotent() {
        let ledger = OrchestrationLedger::open_in_memory().unwrap();
        let source = InMemorySource {
            tasks: vec![task("t-1", SourceStatus::Todo)],
        };
        reconcile_into(&source, &ledger, "ws", 1_000).expect("first");
        let second = reconcile_into(&source, &ledger, "ws", 2_000).expect("second");
        // Same candidate, re-upserted (refreshed updated_at_ms).
        assert_eq!(
            second,
            ReconcileSummary {
                candidates: 1,
                upserted: 1
            }
        );
        let task_id = task_id_for("ws", "memory", "t-1");
        let record = ledger.task(&task_id).unwrap().expect("present");
        assert_eq!(record.updated_at_ms, 2_000);
        assert_eq!(record.created_at_ms, 1_000);
    }

    #[test]
    fn reconcile_empty_source_is_a_noop() {
        let ledger = OrchestrationLedger::open_in_memory().unwrap();
        let source = InMemorySource { tasks: vec![] };
        let summary = reconcile_into(&source, &ledger, "ws", 0).expect("reconcile");
        assert_eq!(summary, ReconcileSummary::default());
    }

    #[test]
    fn native_ids_are_namespaced_by_workspace_and_source() {
        let first = task_id_for("ws-a", "local", "1");
        let second = task_id_for("ws-b", "local", "1");
        let third = task_id_for("ws-a", "github", "1");
        assert_ne!(first, second);
        assert_ne!(first, third);
        assert_eq!(first, task_id_for("ws-a", "local", "1"));
    }

    #[test]
    fn reconcile_validates_entire_snapshot_before_writing() {
        let ledger = OrchestrationLedger::open_in_memory().unwrap();
        let source = InMemorySource {
            tasks: vec![
                task("valid", SourceStatus::Todo),
                task("valid", SourceStatus::InProgress),
            ],
        };

        let error = reconcile_into(&source, &ledger, "ws", 1_000).unwrap_err();
        assert!(error.contains("duplicate task id `valid`"), "{error}");
        let task_id = task_id_for("ws", "memory", "valid");
        assert!(ledger.task(&task_id).unwrap().is_none());
    }
}
