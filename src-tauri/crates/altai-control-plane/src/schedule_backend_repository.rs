//! CP-08 schedule backend seam. Records exactly one schedule backend per
//! attempt, immutably. The binding is insert-only: rebinding the same backend
//! is idempotent, and a divergent backend fails closed as `Conflict`. This is
//! the seam the package-041 cron bridge consumes — a managed scheduler cannot
//! register a second backend for an attempt the native scheduler already owns.

use altai_control_protocol::{AttemptId, ScheduleBackend, ScheduleBackendBinding};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use std::{path::Path, sync::Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScheduleBackendError {
    Conflict { attempt_id: String },
    Internal { reason: String },
}

impl std::fmt::Display for ScheduleBackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "schedule backend error: {self:?}")
    }
}
impl std::error::Error for ScheduleBackendError {}

pub trait ScheduleBackendRepository: Send + Sync {
    /// Bind a backend to an attempt. Idempotent when the same backend is
    /// rebound; fails closed with [`ScheduleBackendError::Conflict`] when a
    /// different backend is already bound — the first binding wins.
    fn bind(
        &self,
        binding: ScheduleBackendBinding,
    ) -> Result<ScheduleBackendBinding, ScheduleBackendError>;
    fn get(
        &self,
        attempt_id: &AttemptId,
    ) -> Result<Option<ScheduleBackendBinding>, ScheduleBackendError>;
}

pub struct SqliteScheduleBackendRepository {
    connection: Mutex<Connection>,
}

impl SqliteScheduleBackendRepository {
    pub fn open(path: &Path) -> Result<Self, String> {
        let connection = Connection::open(path).map_err(|e| e.to_string())?;
        connection.execute_batch("PRAGMA busy_timeout = 5000; CREATE TABLE IF NOT EXISTS control_plane_schedule_backend_bindings (attempt_id TEXT PRIMARY KEY, backend TEXT NOT NULL, bound_at_unix_seconds INTEGER NOT NULL, payload_json TEXT NOT NULL);").map_err(|e| e.to_string())?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, ScheduleBackendError> {
        self.connection.lock().map_err(|_| ScheduleBackendError::Internal {
            reason: "sqlite schedule backend lock poisoned".into(),
        })
    }
    fn db(e: rusqlite::Error) -> ScheduleBackendError {
        ScheduleBackendError::Internal { reason: e.to_string() }
    }
}

impl ScheduleBackendRepository for SqliteScheduleBackendRepository {
    fn bind(
        &self,
        binding: ScheduleBackendBinding,
    ) -> Result<ScheduleBackendBinding, ScheduleBackendError> {
        let payload = serde_json::to_string(&binding).map_err(|e| ScheduleBackendError::Internal {
            reason: e.to_string(),
        })?;
        let mut connection = self.lock()?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(Self::db)?;
        let existing: Option<String> = tx
            .query_row(
                "SELECT payload_json FROM control_plane_schedule_backend_bindings WHERE attempt_id=?1",
                [&binding.attempt_id.value],
                |row| row.get(0),
            )
            .optional()
            .map_err(Self::db)?;
        if let Some(payload) = existing {
            let stored: ScheduleBackendBinding =
                serde_json::from_str(&payload).map_err(|e| ScheduleBackendError::Internal {
                    reason: e.to_string(),
                })?;
            if stored == binding {
                return Ok(stored);
            }
            return Err(ScheduleBackendError::Conflict {
                attempt_id: binding.attempt_id.value,
            });
        }
        let inserted = tx
            .execute(
                "INSERT INTO control_plane_schedule_backend_bindings (attempt_id, backend, bound_at_unix_seconds, payload_json) VALUES (?1, ?2, ?3, ?4) ON CONFLICT DO NOTHING",
                params![
                    binding.attempt_id.value,
                    backend_tag(binding.backend),
                    binding.bound_at_unix_seconds as i64,
                    payload
                ],
            )
            .map_err(Self::db)?;
        if inserted == 0 {
            return Err(ScheduleBackendError::Conflict {
                attempt_id: binding.attempt_id.value,
            });
        }
        tx.commit().map_err(Self::db)?;
        Ok(binding)
    }
    fn get(
        &self,
        attempt_id: &AttemptId,
    ) -> Result<Option<ScheduleBackendBinding>, ScheduleBackendError> {
        let payload: Option<String> = self
            .lock()?
            .query_row(
                "SELECT payload_json FROM control_plane_schedule_backend_bindings WHERE attempt_id=?1",
                [&attempt_id.value],
                |row| row.get(0),
            )
            .optional()
            .map_err(Self::db)?;
        payload
            .map(|p| {
                serde_json::from_str(&p).map_err(|e| ScheduleBackendError::Internal {
                    reason: e.to_string(),
                })
            })
            .transpose()
    }
}

fn backend_tag(backend: ScheduleBackend) -> &'static str {
    match backend {
        ScheduleBackend::NativeLocal => "native_local",
        ScheduleBackend::Managed => "managed",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn binding(backend: ScheduleBackend) -> ScheduleBackendBinding {
        ScheduleBackendBinding {
            attempt_id: AttemptId::new("attempt"),
            backend,
            bound_at_unix_seconds: 1,
        }
    }

    #[test]
    fn bindings_are_durable_idempotent_and_immutable() {
        let dir = tempfile::tempdir().unwrap();
        let database = dir.path().join("work.db");
        let repo = SqliteScheduleBackendRepository::open(&database).unwrap();
        assert_eq!(
            repo.bind(binding(ScheduleBackend::NativeLocal)).unwrap(),
            binding(ScheduleBackend::NativeLocal)
        );
        // Idempotent rebind of the same backend.
        assert_eq!(
            repo.bind(binding(ScheduleBackend::NativeLocal)).unwrap(),
            binding(ScheduleBackend::NativeLocal)
        );
        // Durable across reopen: the first binding still wins.
        let reopened = SqliteScheduleBackendRepository::open(&database).unwrap();
        assert_eq!(
            reopened
                .bind(binding(ScheduleBackend::NativeLocal))
                .unwrap(),
            binding(ScheduleBackend::NativeLocal)
        );
        // A divergent backend fails closed; the native binding is unchanged.
        assert!(matches!(
            repo.bind(binding(ScheduleBackend::Managed)),
            Err(ScheduleBackendError::Conflict { .. })
        ));
        assert_eq!(
            repo.get(&AttemptId::new("attempt")).unwrap().unwrap().backend,
            ScheduleBackend::NativeLocal
        );
    }

    #[test]
    fn get_returns_the_bound_backend_and_none_when_unbound() {
        let dir = tempfile::tempdir().unwrap();
        let database = dir.path().join("work.db");
        let repo = SqliteScheduleBackendRepository::open(&database).unwrap();
        assert!(repo.get(&AttemptId::new("attempt")).unwrap().is_none());
        repo.bind(binding(ScheduleBackend::NativeLocal)).unwrap();
        assert_eq!(
            repo.get(&AttemptId::new("attempt")).unwrap(),
            Some(binding(ScheduleBackend::NativeLocal))
        );
    }

    #[test]
    fn exactly_one_backend_per_attempt_across_distinct_attempts() {
        let dir = tempfile::tempdir().unwrap();
        let database = dir.path().join("work.db");
        let repo = SqliteScheduleBackendRepository::open(&database).unwrap();
        // Two attempts each bind their own backend independently.
        repo.bind(ScheduleBackendBinding {
            attempt_id: AttemptId::new("a"),
            backend: ScheduleBackend::NativeLocal,
            bound_at_unix_seconds: 1,
        })
        .unwrap();
        repo.bind(ScheduleBackendBinding {
            attempt_id: AttemptId::new("b"),
            backend: ScheduleBackend::Managed,
            bound_at_unix_seconds: 2,
        })
        .unwrap();
        assert_eq!(
            repo.get(&AttemptId::new("a")).unwrap().unwrap().backend,
            ScheduleBackend::NativeLocal
        );
        assert_eq!(
            repo.get(&AttemptId::new("b")).unwrap().unwrap().backend,
            ScheduleBackend::Managed
        );
    }
}
