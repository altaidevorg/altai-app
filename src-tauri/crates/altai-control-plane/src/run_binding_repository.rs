//! CP-08 immutable Attempt ↔ executor Run binding in workspace work.db.

use altai_control_protocol::{AttemptId, RunBinding};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use std::{path::Path, sync::Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunBindingError {
    Conflict { attempt_id: String },
    Internal { reason: String },
}
impl std::fmt::Display for RunBindingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "run binding error: {self:?}")
    }
}
impl std::error::Error for RunBindingError {}
pub trait RunBindingRepository: Send + Sync {
    fn bind(&self, binding: RunBinding) -> Result<RunBinding, RunBindingError>;
    fn get(&self, attempt_id: &AttemptId) -> Result<Option<RunBinding>, RunBindingError>;
}
pub struct SqliteRunBindingRepository {
    connection: Mutex<Connection>,
}
impl SqliteRunBindingRepository {
    pub fn open(path: &Path) -> Result<Self, String> {
        let connection = Connection::open(path).map_err(|e| e.to_string())?;
        connection.execute_batch("PRAGMA busy_timeout = 5000; CREATE TABLE IF NOT EXISTS control_plane_run_bindings (attempt_id TEXT PRIMARY KEY, run_id TEXT NOT NULL UNIQUE, payload_json TEXT NOT NULL);").map_err(|e|e.to_string())?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, RunBindingError> {
        self.connection
            .lock()
            .map_err(|_| RunBindingError::Internal {
                reason: "sqlite run binding lock poisoned".into(),
            })
    }
    fn db(e: rusqlite::Error) -> RunBindingError {
        RunBindingError::Internal {
            reason: e.to_string(),
        }
    }
}
impl RunBindingRepository for SqliteRunBindingRepository {
    fn bind(&self, binding: RunBinding) -> Result<RunBinding, RunBindingError> {
        let payload = serde_json::to_string(&binding).map_err(|e| RunBindingError::Internal {
            reason: e.to_string(),
        })?;
        let mut connection = self.lock()?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(Self::db)?;
        let existing: Option<String> = tx
            .query_row(
                "SELECT payload_json FROM control_plane_run_bindings WHERE attempt_id=?1",
                [&binding.attempt_id.value],
                |r| r.get(0),
            )
            .optional()
            .map_err(Self::db)?;
        if let Some(payload) = existing {
            let stored: RunBinding =
                serde_json::from_str(&payload).map_err(|e| RunBindingError::Internal {
                    reason: e.to_string(),
                })?;
            if stored == binding {
                return Ok(stored);
            }
            return Err(RunBindingError::Conflict {
                attempt_id: binding.attempt_id.value,
            });
        };
        let inserted=tx.execute("INSERT INTO control_plane_run_bindings (attempt_id,run_id,payload_json) VALUES (?1,?2,?3) ON CONFLICT DO NOTHING",params![binding.attempt_id.value,binding.run_id.value,payload]).map_err(Self::db)?;
        if inserted == 0 {
            return Err(RunBindingError::Conflict {
                attempt_id: binding.attempt_id.value,
            });
        };
        tx.commit().map_err(Self::db)?;
        Ok(binding)
    }
    fn get(&self, attempt_id: &AttemptId) -> Result<Option<RunBinding>, RunBindingError> {
        let payload: Option<String> = self
            .lock()?
            .query_row(
                "SELECT payload_json FROM control_plane_run_bindings WHERE attempt_id=?1",
                [&attempt_id.value],
                |r| r.get(0),
            )
            .optional()
            .map_err(Self::db)?;
        payload
            .map(|p| {
                serde_json::from_str(&p).map_err(|e| RunBindingError::Internal {
                    reason: e.to_string(),
                })
            })
            .transpose()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use altai_control_protocol::{AgentInstanceId, RunId, WorkItemId};
    fn binding(run: &str) -> RunBinding {
        RunBinding {
            attempt_id: AttemptId::new("attempt"),
            work_item_id: WorkItemId::new("work"),
            owner_agent_instance_id: AgentInstanceId::new("agent"),
            run_id: RunId::new(run),
            bound_at_unix_seconds: 1,
        }
    }
    #[test]
    fn bindings_are_durable_idempotent_and_immutable() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("work.db");
        let repo = SqliteRunBindingRepository::open(&db).unwrap();
        assert_eq!(repo.bind(binding("run-1")).unwrap(), binding("run-1"));
        assert_eq!(repo.bind(binding("run-1")).unwrap(), binding("run-1"));
        assert!(matches!(
            SqliteRunBindingRepository::open(&db)
                .unwrap()
                .bind(binding("run-2")),
            Err(RunBindingError::Conflict { .. })
        ));
    }
}
