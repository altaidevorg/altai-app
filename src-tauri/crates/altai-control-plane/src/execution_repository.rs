//! CP-08 durable execution snapshot loader.
//!
//! A scheduler uses this read boundary after it owns a checkout lease. It
//! never manufactures execution identities: both records must already exist
//! in the same local `work.db` and agree on their work item and owner.

use altai_control_protocol::{Attempt, AttemptId, RunBinding};
use rusqlite::{Connection, OptionalExtension};
use std::{path::Path, sync::Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionSnapshotError {
    AttemptNotFound { attempt_id: String },
    BindingNotFound { attempt_id: String },
    BindingMismatch { attempt_id: String },
    Internal { reason: String },
}

impl std::fmt::Display for ExecutionSnapshotError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "execution snapshot error: {self:?}")
    }
}
impl std::error::Error for ExecutionSnapshotError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionSnapshot {
    pub attempt: Attempt,
    pub run_binding: RunBinding,
}

pub trait ExecutionSnapshotRepository: Send + Sync {
    fn load(&self, attempt_id: &AttemptId) -> Result<ExecutionSnapshot, ExecutionSnapshotError>;
}

pub struct SqliteExecutionSnapshotRepository {
    connection: Mutex<Connection>,
}

impl SqliteExecutionSnapshotRepository {
    pub fn open(path: &Path) -> Result<Self, String> {
        let connection = Connection::open(path).map_err(|error| error.to_string())?;
        connection
            .execute_batch("PRAGMA busy_timeout = 5000;")
            .map_err(|error| error.to_string())?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, ExecutionSnapshotError> {
        self.connection
            .lock()
            .map_err(|_| ExecutionSnapshotError::Internal {
                reason: "execution snapshot lock poisoned".into(),
            })
    }

    fn db(error: rusqlite::Error) -> ExecutionSnapshotError {
        ExecutionSnapshotError::Internal {
            reason: error.to_string(),
        }
    }
}

impl ExecutionSnapshotRepository for SqliteExecutionSnapshotRepository {
    fn load(&self, attempt_id: &AttemptId) -> Result<ExecutionSnapshot, ExecutionSnapshotError> {
        let connection = self.lock()?;
        let attempt_payload: Option<String> = connection
            .query_row(
                "SELECT payload_json FROM control_plane_attempts WHERE attempt_id = ?1",
                [&attempt_id.value],
                |row| row.get(0),
            )
            .optional()
            .map_err(Self::db)?;
        let attempt_payload =
            attempt_payload.ok_or_else(|| ExecutionSnapshotError::AttemptNotFound {
                attempt_id: attempt_id.value.clone(),
            })?;
        let attempt: Attempt = serde_json::from_str(&attempt_payload).map_err(|error| {
            ExecutionSnapshotError::Internal {
                reason: error.to_string(),
            }
        })?;
        let binding_payload: Option<String> = connection
            .query_row(
                "SELECT payload_json FROM control_plane_run_bindings WHERE attempt_id = ?1",
                [&attempt_id.value],
                |row| row.get(0),
            )
            .optional()
            .map_err(Self::db)?;
        let binding_payload =
            binding_payload.ok_or_else(|| ExecutionSnapshotError::BindingNotFound {
                attempt_id: attempt_id.value.clone(),
            })?;
        let run_binding: RunBinding = serde_json::from_str(&binding_payload).map_err(|error| {
            ExecutionSnapshotError::Internal {
                reason: error.to_string(),
            }
        })?;
        if attempt.id != run_binding.attempt_id
            || attempt.work_item_id != run_binding.work_item_id
            || attempt.owner_agent_instance_id != run_binding.owner_agent_instance_id
        {
            return Err(ExecutionSnapshotError::BindingMismatch {
                attempt_id: attempt_id.value.clone(),
            });
        }
        Ok(ExecutionSnapshot {
            attempt,
            run_binding,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AttemptRepository, RunBindingRepository, SqliteAttemptRepository,
        SqliteRunBindingRepository,
    };
    use altai_control_protocol::{AgentInstanceId, AgentProfileRevisionId, RunId, WorkItemId};

    fn attempt() -> Attempt {
        Attempt {
            id: AttemptId::new("one"),
            work_item_id: WorkItemId::new("work"),
            owner_agent_instance_id: AgentInstanceId::new("agent"),
            profile_revision_id: AgentProfileRevisionId::new("profile"),
            state: altai_control_protocol::AttemptState::Created,
            created_at_unix_seconds: 1,
            updated_at_unix_seconds: 2,
        }
    }

    #[test]
    fn loads_only_matching_durable_attempt_and_binding() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("work.db");
        let attempts = SqliteAttemptRepository::open(&database).unwrap();
        attempts.create(attempt()).unwrap();
        attempts
            .transition(
                &AttemptId::new("one"),
                altai_control_protocol::AttemptState::Claimed,
                2,
            )
            .unwrap();
        attempts
            .transition(
                &AttemptId::new("one"),
                altai_control_protocol::AttemptState::Dispatched,
                3,
            )
            .unwrap();
        SqliteRunBindingRepository::open(&database)
            .unwrap()
            .bind(RunBinding {
                attempt_id: AttemptId::new("one"),
                work_item_id: WorkItemId::new("work"),
                owner_agent_instance_id: AgentInstanceId::new("agent"),
                run_id: RunId::new("run"),
                bound_at_unix_seconds: 2,
            })
            .unwrap();
        assert_eq!(
            SqliteExecutionSnapshotRepository::open(&database)
                .unwrap()
                .load(&AttemptId::new("one"))
                .unwrap()
                .run_binding
                .run_id,
            RunId::new("run")
        );
    }

    #[test]
    fn refuses_a_binding_for_another_owner() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("work.db");
        let attempts = SqliteAttemptRepository::open(&database).unwrap();
        attempts.create(attempt()).unwrap();
        attempts
            .transition(
                &AttemptId::new("one"),
                altai_control_protocol::AttemptState::Claimed,
                2,
            )
            .unwrap();
        attempts
            .transition(
                &AttemptId::new("one"),
                altai_control_protocol::AttemptState::Dispatched,
                3,
            )
            .unwrap();
        SqliteRunBindingRepository::open(&database)
            .unwrap()
            .bind(RunBinding {
                attempt_id: AttemptId::new("one"),
                work_item_id: WorkItemId::new("work"),
                owner_agent_instance_id: AgentInstanceId::new("other-agent"),
                run_id: RunId::new("run"),
                bound_at_unix_seconds: 2,
            })
            .unwrap();
        assert!(matches!(
            SqliteExecutionSnapshotRepository::open(&database)
                .unwrap()
                .load(&AttemptId::new("one")),
            Err(ExecutionSnapshotError::BindingMismatch { .. })
        ));
    }
}
