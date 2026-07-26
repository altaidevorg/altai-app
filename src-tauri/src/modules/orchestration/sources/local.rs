//! The always-available local task source.
//!
//! Reads a task board from `<workspace>/.altai/tasks/*.json` — one file per
//! task. Requires no authentication and is the default source for local-only
//! orchestration (§4.2). Each task file:
//!
//! ```json
//! { "id": "fix-login", "title": "Fix the login redirect", "status": "todo",
//!   "prompt": "Reproduce the redirect loop on /login and fix it." }
//! ```

use std::path::{Path, PathBuf};

use super::{SourceTask, TaskSourceAdapter, TaskSourceCapabilities};

const SOURCE_KIND: &str = "local";
const TASKS_DIR: &str = ".altai/tasks";

/// A task source backed by a directory of JSON task files on the local disk.
pub struct LocalTaskSource {
    root: PathBuf,
}

impl LocalTaskSource {
    /// `root` is the workspace root; tasks live under `<root>/.altai/tasks/`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn tasks_dir(&self) -> PathBuf {
        self.root.join(TASKS_DIR)
    }

    /// Read every `*.json` task file under the board directory. A missing
    /// directory is an empty board (the coordinator simply has no candidates),
    /// not an error.
    fn read_all(&self) -> Result<Vec<SourceTask>, String> {
        let dir = self.tasks_dir();
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let canonical = std::fs::canonicalize(&dir)
            .map_err(|error| format!("Cannot resolve tasks directory: {error}"))?;
        let mut tasks = Vec::new();
        let entries = std::fs::read_dir(&canonical)
            .map_err(|error| format!("Cannot read tasks directory: {error}"))?;
        for entry in entries {
            let entry = entry.map_err(|error| format!("Cannot read directory entry: {error}"))?;
            let path = entry.path();
            if !is_json_file(&path) {
                continue;
            }
            // Refuse to follow symlinks: the board must be regular files the
            // user owns, so a task file can't reach outside the workspace.
            let meta = std::fs::symlink_metadata(&path)
                .map_err(|error| format!("Cannot inspect {}: {error}", path.display()))?;
            if meta.file_type().is_symlink() || !meta.is_file() {
                return Err(format!(
                    "Task file {} must be a regular file, not a symlink.",
                    path.display()
                ));
            }
            let body = std::fs::read_to_string(&path)
                .map_err(|error| format!("Cannot read {}: {error}", path.display()))?;
            let task: SourceTask = serde_json::from_str(&body)
                .map_err(|error| format!("Invalid task file {}: {error}", path.display()))?;
            tasks.push(task);
        }
        // Stable, deterministic order by id so reconcile output is reproducible.
        tasks.sort_by(|a, b| a.id.cmp(&b.id));
        tasks.dedup_by(|a, b| a.id == b.id);
        Ok(tasks)
    }
}

impl TaskSourceAdapter for LocalTaskSource {
    fn source_kind(&self) -> &'static str {
        SOURCE_KIND
    }

    fn list_all(&self) -> Result<Vec<SourceTask>, String> {
        self.read_all()
    }

    fn get_task(&self, native_id: &str) -> Result<Option<SourceTask>, String> {
        Ok(self.read_all()?.into_iter().find(|t| t.id == native_id))
    }

    fn capabilities(&self) -> TaskSourceCapabilities {
        // Local board state is driven by the ledger; the source itself does not
        // post status back to an external system.
        TaskSourceCapabilities::default()
    }
}

fn is_json_file(path: &Path) -> bool {
    path.extension().and_then(|ext| ext.to_str()) == Some("json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::orchestration::domain::TaskState;
    use crate::modules::orchestration::ledger::OrchestrationLedger;
    use crate::modules::orchestration::sources::{reconcile_into, SourceStatus};

    fn write_task(dir: &Path, file: &str, id: &str, status: &str) {
        let body =
            format!("{{ \"id\": \"{id}\", \"title\": \"Task {id}\", \"status\": \"{status}\" }}");
        std::fs::write(dir.join(file), body).unwrap();
    }

    #[test]
    fn missing_board_is_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let source = LocalTaskSource::new(tmp.path());
        assert!(source.list_all().unwrap().is_empty());
        assert!(source.get_task("anything").unwrap().is_none());
    }

    #[test]
    fn reads_active_and_terminal_tasks() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join(TASKS_DIR);
        std::fs::create_dir_all(&dir).unwrap();
        write_task(&dir, "a.json", "t-a", "todo");
        write_task(&dir, "b.json", "t-b", "in_progress");
        write_task(&dir, "c.json", "t-c", "done");

        let source = LocalTaskSource::new(tmp.path());
        let tasks = source.list_all().expect("read");
        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[0].id, "t-a");
        assert_eq!(tasks[1].status, SourceStatus::InProgress);

        let fetched = source.get_task("t-c").unwrap().expect("found");
        assert_eq!(fetched.status, SourceStatus::Done);
    }

    #[test]
    fn ignores_non_json_files() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join(TASKS_DIR);
        std::fs::create_dir_all(&dir).unwrap();
        write_task(&dir, "a.json", "t-a", "todo");
        std::fs::write(dir.join("README.md"), "# tasks").unwrap();

        let source = LocalTaskSource::new(tmp.path());
        let tasks = source.list_all().expect("read");
        assert_eq!(tasks.len(), 1);
    }

    #[test]
    fn rejects_symlinked_task_file() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join(TASKS_DIR);
        std::fs::create_dir_all(&dir).unwrap();
        let target = tmp.path().join("real.json");
        std::fs::write(&target, "{ \"id\": \"x\" }").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, dir.join("link.json")).unwrap();

        let source = LocalTaskSource::new(tmp.path());
        #[cfg(unix)]
        assert!(source.list_all().is_err());
    }

    #[test]
    fn reconciles_local_board_into_ledger() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join(TASKS_DIR);
        std::fs::create_dir_all(&dir).unwrap();
        write_task(&dir, "a.json", "local-1", "todo");
        write_task(&dir, "b.json", "local-2", "done");

        let ledger = OrchestrationLedger::open_in_memory().unwrap();
        let source = LocalTaskSource::new(tmp.path());
        let summary = reconcile_into(&source, &ledger, "ws-1", 5_000).expect("reconcile");

        assert_eq!(summary.candidates, 1);
        assert_eq!(summary.upserted, 1);
        let task = ledger.task("local-1").unwrap().expect("present");
        assert_eq!(task.state, TaskState::Queued);
        assert_eq!(task.source_kind, "local");
        assert_eq!(task.workspace_key, "ws-1");
        // The "done" task was not mirrored.
        assert!(ledger.task("local-2").unwrap().is_none());
    }
}
