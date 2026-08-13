//! CP-08 durable local Attempt lifecycle. Attempts retain execution evidence;
//! they do not mutate WorkItem disposition.

use altai_control_protocol::{Attempt, AttemptId, AttemptState};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use std::{path::Path, sync::Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttemptError {
    NotFound { attempt_id: String },
    InvalidTransition { from: AttemptState, to: AttemptState },
    Conflict { attempt_id: String },
    Internal { reason: String },
}
impl std::fmt::Display for AttemptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "attempt error: {self:?}") }
}
impl std::error::Error for AttemptError {}

pub trait AttemptRepository: Send + Sync {
    fn create(&self, attempt: Attempt) -> Result<Attempt, AttemptError>;
    fn transition(&self, id: &AttemptId, to: AttemptState, now: u64) -> Result<Attempt, AttemptError>;
    fn get(&self, id: &AttemptId) -> Result<Option<Attempt>, AttemptError>;
}

pub struct SqliteAttemptRepository { connection: Mutex<Connection> }
impl SqliteAttemptRepository {
    pub fn open(path: &Path) -> Result<Self, String> {
        let connection = Connection::open(path).map_err(|e| e.to_string())?;
        connection.execute_batch("PRAGMA busy_timeout = 5000; CREATE TABLE IF NOT EXISTS control_plane_attempts (attempt_id TEXT PRIMARY KEY, payload_json TEXT NOT NULL);").map_err(|e| e.to_string())?;
        Ok(Self { connection: Mutex::new(connection) })
    }
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, AttemptError> { self.connection.lock().map_err(|_| AttemptError::Internal { reason: "sqlite attempt lock poisoned".into() }) }
    fn db(error: rusqlite::Error) -> AttemptError { AttemptError::Internal { reason: error.to_string() } }
    fn decode(payload: String) -> Result<Attempt, AttemptError> { serde_json::from_str(&payload).map_err(|e| AttemptError::Internal { reason: e.to_string() }) }
}
impl AttemptRepository for SqliteAttemptRepository {
    fn create(&self, attempt: Attempt) -> Result<Attempt, AttemptError> {
        if attempt.state != AttemptState::Created { return Err(AttemptError::InvalidTransition { from: attempt.state, to: AttemptState::Created }); }
        let payload = serde_json::to_string(&attempt).map_err(|e| AttemptError::Internal { reason: e.to_string() })?;
        let inserted = self.lock()?.execute("INSERT INTO control_plane_attempts (attempt_id, payload_json) VALUES (?1, ?2) ON CONFLICT DO NOTHING", params![attempt.id.value, payload]).map_err(Self::db)?;
        if inserted == 1 { return Ok(attempt); }
        let existing = self.get(&attempt.id)?.ok_or_else(|| AttemptError::Internal { reason: "attempt disappeared after insert conflict".into() })?;
        if existing == attempt { Ok(existing) } else { Err(AttemptError::Conflict { attempt_id: attempt.id.value }) }
    }
    fn transition(&self, id: &AttemptId, to: AttemptState, now: u64) -> Result<Attempt, AttemptError> {
        let mut connection = self.lock()?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(Self::db)?;
        let payload: String = tx.query_row("SELECT payload_json FROM control_plane_attempts WHERE attempt_id = ?1", [&id.value], |row| row.get(0)).optional().map_err(Self::db)?.ok_or_else(|| AttemptError::NotFound { attempt_id: id.value.clone() })?;
        let mut attempt = Self::decode(payload)?;
        if attempt.state == to { return Ok(attempt); }
        if !valid_transition(attempt.state, to) { return Err(AttemptError::InvalidTransition { from: attempt.state, to }); }
        attempt.state = to;
        attempt.updated_at_unix_seconds = now;
        let payload = serde_json::to_string(&attempt).map_err(|e| AttemptError::Internal { reason: e.to_string() })?;
        tx.execute("UPDATE control_plane_attempts SET payload_json = ?2 WHERE attempt_id = ?1", params![id.value, payload]).map_err(Self::db)?;
        tx.commit().map_err(Self::db)?;
        Ok(attempt)
    }
    fn get(&self, id: &AttemptId) -> Result<Option<Attempt>, AttemptError> {
        self.lock()?.query_row("SELECT payload_json FROM control_plane_attempts WHERE attempt_id = ?1", [&id.value], |row| row.get(0)).optional().map_err(Self::db)?.map(Self::decode).transpose()
    }
}

fn valid_transition(from: AttemptState, to: AttemptState) -> bool {
    matches!((from, to), (AttemptState::Created, AttemptState::Claimed) | (AttemptState::Claimed, AttemptState::Dispatched) | (AttemptState::Dispatched, AttemptState::Running) | (AttemptState::Running, AttemptState::Succeeded | AttemptState::Failed | AttemptState::Cancelled | AttemptState::TimedOut | AttemptState::BudgetStopped | AttemptState::PolicyDenied | AttemptState::Lost) | (AttemptState::Created | AttemptState::Claimed | AttemptState::Dispatched, AttemptState::Cancelled | AttemptState::PolicyDenied | AttemptState::Lost))
}

#[cfg(test)]
mod tests {
    use super::*;
    use altai_control_protocol::{AgentInstanceId, AgentProfileRevisionId, WorkItemId};
    fn attempt() -> Attempt { Attempt { id: AttemptId::new("one"), work_item_id: WorkItemId::new("one"), owner_agent_instance_id: AgentInstanceId::new("one"), profile_revision_id: AgentProfileRevisionId::new("one"), state: AttemptState::Created, created_at_unix_seconds: 1, updated_at_unix_seconds: 1 } }
    #[test]
    fn lifecycle_is_durable_idempotent_and_never_reopens() {
        let dir = tempfile::tempdir().unwrap(); let path = dir.path().join("work.db"); let repo = SqliteAttemptRepository::open(&path).unwrap();
        assert_eq!(repo.create(attempt()).unwrap(), attempt());
        assert_eq!(repo.transition(&attempt().id, AttemptState::Claimed, 2).unwrap().state, AttemptState::Claimed);
        assert_eq!(repo.transition(&attempt().id, AttemptState::Dispatched, 3).unwrap().state, AttemptState::Dispatched);
        assert_eq!(repo.transition(&attempt().id, AttemptState::Running, 4).unwrap().state, AttemptState::Running);
        assert_eq!(SqliteAttemptRepository::open(&path).unwrap().transition(&attempt().id, AttemptState::Succeeded, 5).unwrap().state, AttemptState::Succeeded);
        assert!(matches!(repo.transition(&attempt().id, AttemptState::Running, 6), Err(AttemptError::InvalidTransition { .. })));
    }
}
