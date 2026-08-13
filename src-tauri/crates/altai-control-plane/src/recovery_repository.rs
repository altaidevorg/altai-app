//! CP-07-06 durable retry/recovery/dead-letter repository boundary.

use altai_control_protocol::{AttemptId, RecoveryDisposition, RecoveryRecord, WorkItemId};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use std::{path::Path, sync::Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryError {
    Internal { reason: String },
}
impl std::fmt::Display for RecoveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "recovery repository error: {self:?}")
    }
}
impl std::error::Error for RecoveryError {}

pub trait RecoveryRepository: Send + Sync {
    fn record_failure(
        &self,
        work_item_id: WorkItemId,
        attempt_id: AttemptId,
        max_retries: u32,
        failure: String,
        now_unix_seconds: u64,
    ) -> Result<RecoveryRecord, RecoveryError>;
    fn dead_letters(&self) -> Result<Vec<RecoveryRecord>, RecoveryError>;
}

pub struct SqliteRecoveryRepository {
    connection: Mutex<Connection>,
}
impl SqliteRecoveryRepository {
    pub fn open(path: &Path) -> Result<Self, String> {
        let connection = Connection::open(path).map_err(|e| e.to_string())?;
        connection.execute_batch("PRAGMA busy_timeout = 5000; CREATE TABLE IF NOT EXISTS control_plane_recovery_records (work_item_id TEXT PRIMARY KEY, attempt_id TEXT NOT NULL, retry_count INTEGER NOT NULL, max_retries INTEGER NOT NULL, last_failure TEXT NOT NULL, disposition TEXT NOT NULL, updated_at_unix_seconds INTEGER NOT NULL);").map_err(|e| e.to_string())?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, RecoveryError> {
        self.connection.lock().map_err(|_| RecoveryError::Internal {
            reason: "sqlite recovery lock poisoned".into(),
        })
    }
    fn db(error: rusqlite::Error) -> RecoveryError {
        RecoveryError::Internal {
            reason: error.to_string(),
        }
    }
    fn record(row: (String, String, i64, i64, String, String, i64)) -> RecoveryRecord {
        RecoveryRecord {
            work_item_id: WorkItemId::new(row.0),
            attempt_id: AttemptId::new(row.1),
            retry_count: row.2 as u32,
            max_retries: row.3 as u32,
            last_failure: row.4,
            disposition: if row.5 == "dead_lettered" {
                RecoveryDisposition::DeadLettered
            } else {
                RecoveryDisposition::RetryQueued
            },
            updated_at_unix_seconds: row.6 as u64,
        }
    }
}
impl RecoveryRepository for SqliteRecoveryRepository {
    fn record_failure(
        &self,
        work_item_id: WorkItemId,
        attempt_id: AttemptId,
        max_retries: u32,
        failure: String,
        now_unix_seconds: u64,
    ) -> Result<RecoveryRecord, RecoveryError> {
        let mut connection = self.lock()?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(Self::db)?;
        let previous: Option<(String, String, i64, String, i64)> = tx
            .query_row(
                "SELECT attempt_id, last_failure, retry_count, disposition, updated_at_unix_seconds FROM control_plane_recovery_records WHERE work_item_id = ?1",
                [&work_item_id.value],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .optional()
            .map_err(Self::db)?;
        if let Some((previous_attempt, previous_failure, retry_count, disposition, updated_at)) =
            &previous
        {
            if previous_attempt == &attempt_id.value && previous_failure == &failure {
                return Ok(RecoveryRecord {
                    work_item_id,
                    attempt_id,
                    retry_count: *retry_count as u32,
                    max_retries,
                    last_failure: failure,
                    disposition: if disposition == "dead_lettered" {
                        RecoveryDisposition::DeadLettered
                    } else {
                        RecoveryDisposition::RetryQueued
                    },
                    updated_at_unix_seconds: *updated_at as u64,
                });
            }
        }
        let retry_count = previous.map(|row| row.2).unwrap_or(0) + 1;
        let disposition = if retry_count > max_retries as i64 {
            "dead_lettered"
        } else {
            "retry_queued"
        };
        tx.execute("INSERT INTO control_plane_recovery_records (work_item_id, attempt_id, retry_count, max_retries, last_failure, disposition, updated_at_unix_seconds) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) ON CONFLICT(work_item_id) DO UPDATE SET attempt_id=excluded.attempt_id, retry_count=excluded.retry_count, max_retries=excluded.max_retries, last_failure=excluded.last_failure, disposition=excluded.disposition, updated_at_unix_seconds=excluded.updated_at_unix_seconds", params![work_item_id.value, attempt_id.value, retry_count, max_retries as i64, failure, disposition, now_unix_seconds as i64]).map_err(Self::db)?;
        tx.commit().map_err(Self::db)?;
        Ok(RecoveryRecord {
            work_item_id,
            attempt_id,
            retry_count: retry_count as u32,
            max_retries,
            last_failure: failure,
            disposition: if disposition == "dead_lettered" {
                RecoveryDisposition::DeadLettered
            } else {
                RecoveryDisposition::RetryQueued
            },
            updated_at_unix_seconds: now_unix_seconds,
        })
    }
    fn dead_letters(&self) -> Result<Vec<RecoveryRecord>, RecoveryError> {
        let connection = self.lock()?;
        let mut statement = connection.prepare("SELECT work_item_id, attempt_id, retry_count, max_retries, last_failure, disposition, updated_at_unix_seconds FROM control_plane_recovery_records WHERE disposition = 'dead_lettered' ORDER BY updated_at_unix_seconds").map_err(Self::db)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            })
            .map_err(Self::db)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map(|rows| rows.into_iter().map(Self::record).collect())
            .map_err(Self::db)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retries_are_durable_idempotent_and_eventually_dead_lettered() {
        let directory = tempfile::tempdir().unwrap();
        let db = directory.path().join("work.db");
        let repo = SqliteRecoveryRepository::open(&db).unwrap();
        let work = WorkItemId::new("work");
        assert_eq!(
            repo.record_failure(work.clone(), AttemptId::new("a1"), 1, "first".into(), 1)
                .unwrap()
                .disposition,
            RecoveryDisposition::RetryQueued
        );
        assert_eq!(
            repo.record_failure(work.clone(), AttemptId::new("a1"), 1, "first".into(), 2)
                .unwrap()
                .retry_count,
            1
        );
        assert_eq!(
            SqliteRecoveryRepository::open(&db)
                .unwrap()
                .record_failure(work, AttemptId::new("a2"), 1, "second".into(), 3)
                .unwrap()
                .disposition,
            RecoveryDisposition::DeadLettered
        );
        assert_eq!(repo.dead_letters().unwrap().len(), 1);
    }
}
