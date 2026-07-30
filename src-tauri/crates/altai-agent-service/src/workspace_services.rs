use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use altai_core::journal::{EventJournal, JournalEvent};

use crate::Event;

/// Workspace-owned durable resources shared by every host adapter.
///
/// The runtime-specific memory actor and task routers stay in their hosts for
/// TVS-03. This record is deliberately the single opener/classifier for the
/// event journal, so a workspace is restart-classified at most once per
/// `WorkspaceServices::open` call and the journal CAS makes repeats harmless.
pub struct WorkspaceServices {
    root: PathBuf,
    memory_db_path: PathBuf,
    event_journal: Arc<EventJournal>,
}

impl WorkspaceServices {
    pub fn open(root: impl AsRef<Path>) -> Result<Self, WorkspaceServiceError> {
        let root = root.as_ref().to_path_buf();
        let generated = root.join(".system_generated");
        std::fs::create_dir_all(&generated).map_err(WorkspaceServiceError::Io)?;
        let event_journal = Arc::new(
            EventJournal::open(generated.join("agent_event_journal.db"))
                .map_err(WorkspaceServiceError::Journal)?,
        );
        classify_runs_abandoned_by_restart(&event_journal)?;
        Ok(Self {
            memory_db_path: generated.join("agent_memory.db"),
            root,
            event_journal,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn memory_db_path(&self) -> &Path {
        &self.memory_db_path
    }

    pub fn event_journal(&self) -> Arc<EventJournal> {
        self.event_journal.clone()
    }
}

/// Append one restart-recovery terminal event for every incomplete durable run.
/// Repeated calls are safe because the journal's terminal compare-and-set wins
/// exactly once for each run.
pub fn classify_runs_abandoned_by_restart(
    journal: &EventJournal,
) -> Result<(), WorkspaceServiceError> {
    for summary in journal
        .incomplete_run_summaries()
        .map_err(WorkspaceServiceError::Journal)?
    {
        let seq = summary
            .last_seq
            .checked_add(1)
            .ok_or(WorkspaceServiceError::SequenceExhausted)?;
        let event = Event::RunTerminated {
            run_id: summary.run_id.clone(),
            outcome: serde_json::json!({
                "kind": "failed",
                "failure": "The previous app process ended before this run completed.",
                "retryable": false
            }),
        };
        let payload = serde_json::to_value(event).map_err(WorkspaceServiceError::Serialization)?;
        let terminal = JournalEvent::now(
            1,
            summary.run_id.clone(),
            seq,
            summary.chat_id,
            "run_terminated",
            payload,
        );
        if let Err(error) = journal.append_terminal(&terminal) {
            let committed = journal
                .run_summary(&summary.run_id)
                .map_err(WorkspaceServiceError::Journal)?
                .is_some_and(|current| current.terminal_seq.is_some());
            if !committed {
                return Err(WorkspaceServiceError::Journal(error));
            }
        }
    }
    Ok(())
}

#[derive(Debug)]
pub enum WorkspaceServiceError {
    Io(std::io::Error),
    Journal(altai_core::journal::JournalError),
    Serialization(serde_json::Error),
    SequenceExhausted,
}

impl fmt::Display for WorkspaceServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "workspace service I/O error: {error}"),
            Self::Journal(error) => write!(f, "workspace service journal error: {error}"),
            Self::Serialization(error) => write!(f, "workspace service event error: {error}"),
            Self::SequenceExhausted => f.write_str("workspace service run sequence exhausted"),
        }
    }
}

impl std::error::Error for WorkspaceServiceError {}
