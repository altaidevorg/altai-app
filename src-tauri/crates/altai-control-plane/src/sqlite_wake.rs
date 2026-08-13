//! Local SQLite implementation of CP-07 wake coalescing and checkout leases.

use crate::{WakeError, WakeRepository};
use altai_control_protocol::{AttemptId, WakeRequest, WakeSource, WorkCheckoutLease, WorkItemId};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use std::{path::Path, sync::Mutex};

pub struct SqliteWakeRepository {
    connection: Mutex<Connection>,
}

impl SqliteWakeRepository {
    pub fn open(path: &Path) -> Result<Self, String> {
        let connection = Connection::open(path).map_err(|error| error.to_string())?;
        connection
            .execute_batch(
                "PRAGMA foreign_keys = ON; PRAGMA busy_timeout = 5000;
                 CREATE TABLE IF NOT EXISTS control_plane_wake_requests (
                   work_item_id TEXT PRIMARY KEY, id TEXT NOT NULL, sources_json TEXT NOT NULL,
                   requested_at TEXT NOT NULL, claimed_at TEXT NULL
                 );
                 CREATE TABLE IF NOT EXISTS control_plane_work_checkout_leases (
                   work_item_id TEXT PRIMARY KEY, owner_agent_instance_id TEXT NOT NULL,
                   attempt_id TEXT NOT NULL, expires_at TEXT NOT NULL,
                   expires_at_unix_seconds INTEGER NOT NULL DEFAULT 0
                 );",
            )
            .map_err(|error| error.to_string())?;
        let has_expiry = connection
            .prepare("PRAGMA table_info(control_plane_work_checkout_leases)")
            .and_then(|mut statement| {
                statement
                    .query_map([], |row| row.get::<_, String>(1))?
                    .collect::<Result<Vec<_>, _>>()
            })
            .map_err(|error| error.to_string())?
            .iter()
            .any(|column| column == "expires_at_unix_seconds");
        if !has_expiry {
            connection.execute("ALTER TABLE control_plane_work_checkout_leases ADD COLUMN expires_at_unix_seconds INTEGER NOT NULL DEFAULT 0", []).map_err(|error| error.to_string())?;
        }
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, WakeError> {
        self.connection.lock().map_err(|_| WakeError::Internal {
            reason: "sqlite wake lock poisoned".into(),
        })
    }
    fn db(error: rusqlite::Error) -> WakeError {
        WakeError::Internal {
            reason: error.to_string(),
        }
    }
}

impl WakeRepository for SqliteWakeRepository {
    fn enqueue(
        &self,
        work_item_id: WorkItemId,
        source: WakeSource,
        requested_at: String,
    ) -> Result<WakeRequest, WakeError> {
        let mut connection = self.lock()?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(Self::db)?;
        let existing: Option<(String, String, String, Option<String>)> = tx.query_row(
            "SELECT id, sources_json, requested_at, claimed_at FROM control_plane_wake_requests WHERE work_item_id = ?1",
            [&work_item_id.value], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        ).optional().map_err(Self::db)?;
        let wake = match existing {
            Some((id, sources_json, original_requested_at, claimed_at)) => {
                let mut sources: Vec<WakeSource> =
                    serde_json::from_str(&sources_json).map_err(|error| WakeError::Internal {
                        reason: error.to_string(),
                    })?;
                if !sources.contains(&source) {
                    sources.push(source);
                }
                tx.execute("UPDATE control_plane_wake_requests SET sources_json = ?2 WHERE work_item_id = ?1", params![work_item_id.value, serde_json::to_string(&sources).map_err(|error| WakeError::Internal { reason: error.to_string() })?]).map_err(Self::db)?;
                WakeRequest {
                    id,
                    work_item_id,
                    sources,
                    requested_at: original_requested_at,
                    claimed_at,
                }
            }
            None => {
                let wake = WakeRequest {
                    id: format!("wake_{}", work_item_id.value),
                    work_item_id,
                    sources: vec![source],
                    requested_at,
                    claimed_at: None,
                };
                tx.execute("INSERT INTO control_plane_wake_requests (work_item_id, id, sources_json, requested_at, claimed_at) VALUES (?1, ?2, ?3, ?4, NULL)", params![wake.work_item_id.value, wake.id, serde_json::to_string(&wake.sources).map_err(|error| WakeError::Internal { reason: error.to_string() })?, wake.requested_at]).map_err(Self::db)?;
                wake
            }
        };
        tx.commit().map_err(Self::db)?;
        Ok(wake)
    }

    fn checkout(&self, lease: WorkCheckoutLease, now_unix_seconds: u64) -> Result<(), WakeError> {
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(Self::db)?;
        transaction.execute("DELETE FROM control_plane_work_checkout_leases WHERE work_item_id = ?1 AND expires_at_unix_seconds <= ?2", params![lease.work_item_id.value, now_unix_seconds as i64]).map_err(Self::db)?;
        let inserted = transaction.execute(
            "INSERT INTO control_plane_work_checkout_leases (work_item_id, owner_agent_instance_id, attempt_id, expires_at, expires_at_unix_seconds) VALUES (?1, ?2, ?3, ?4, ?5) ON CONFLICT(work_item_id) DO NOTHING",
            params![lease.work_item_id.value, lease.owner_agent_instance_id.value, lease.attempt_id.value, lease.expires_at_unix_seconds.to_string(), lease.expires_at_unix_seconds as i64],
        ).map_err(Self::db)?;
        if inserted == 0 {
            return Err(WakeError::ActiveCheckout {
                work_item_id: lease.work_item_id.value,
            });
        }
        transaction.commit().map_err(Self::db)?;
        Ok(())
    }

    fn claim_wake(
        &self,
        work_item_id: &WorkItemId,
        claimed_at: String,
    ) -> Result<WakeRequest, WakeError> {
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(Self::db)?;
        let updated = transaction.execute("UPDATE control_plane_wake_requests SET claimed_at = ?2 WHERE work_item_id = ?1 AND claimed_at IS NULL", params![work_item_id.value, claimed_at]).map_err(Self::db)?;
        if updated == 0 {
            let exists = transaction
                .query_row(
                    "SELECT 1 FROM control_plane_wake_requests WHERE work_item_id = ?1",
                    [&work_item_id.value],
                    |_| Ok(()),
                )
                .optional()
                .map_err(Self::db)?
                .is_some();
            return Err(if exists {
                WakeError::AlreadyClaimed {
                    work_item_id: work_item_id.value.clone(),
                }
            } else {
                WakeError::NotFound {
                    work_item_id: work_item_id.value.clone(),
                }
            });
        }
        let wake = transaction.query_row("SELECT id, sources_json, requested_at, claimed_at FROM control_plane_wake_requests WHERE work_item_id = ?1", [&work_item_id.value], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, Option<String>>(3)?))).map_err(Self::db)?;
        transaction.commit().map_err(Self::db)?;
        Ok(WakeRequest {
            id: wake.0,
            work_item_id: work_item_id.clone(),
            sources: serde_json::from_str(&wake.1).map_err(|error| WakeError::Internal {
                reason: error.to_string(),
            })?,
            requested_at: wake.2,
            claimed_at: wake.3,
        })
    }

    fn release_checkout(
        &self,
        work_item_id: &WorkItemId,
        attempt_id: &AttemptId,
    ) -> Result<(), WakeError> {
        let connection = self.lock()?;
        let deleted = connection.execute(
            "DELETE FROM control_plane_work_checkout_leases WHERE work_item_id = ?1 AND attempt_id = ?2",
            params![work_item_id.value, attempt_id.value],
        ).map_err(Self::db)?;
        if deleted == 1 {
            return Ok(());
        }
        let exists = connection
            .query_row(
                "SELECT 1 FROM control_plane_work_checkout_leases WHERE work_item_id = ?1",
                [&work_item_id.value],
                |_| Ok(()),
            )
            .optional()
            .map_err(Self::db)?
            .is_some();
        Err(if exists {
            WakeError::ActiveCheckout {
                work_item_id: work_item_id.value.clone(),
            }
        } else {
            WakeError::NotFound {
                work_item_id: work_item_id.value.clone(),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use altai_control_protocol::AgentInstanceId;

    #[test]
    fn wake_and_exclusive_lease_survive_reopen() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("work.db");
        let work = WorkItemId::new("work-1");
        let first = SqliteWakeRepository::open(&database).unwrap();
        first
            .enqueue(work.clone(), WakeSource::Assignment, "first".into())
            .unwrap();
        first
            .checkout(
                WorkCheckoutLease {
                    work_item_id: work.clone(),
                    owner_agent_instance_id: AgentInstanceId::new("agent-1"),
                    attempt_id: AttemptId::new("attempt-1"),
                    expires_at_unix_seconds: 10,
                },
                0,
            )
            .unwrap();
        let reopened = SqliteWakeRepository::open(&database).unwrap();
        let wake = reopened
            .enqueue(work.clone(), WakeSource::Comment, "later".into())
            .unwrap();
        assert_eq!(
            wake.sources,
            vec![WakeSource::Assignment, WakeSource::Comment]
        );
        assert!(matches!(
            reopened.checkout(
                WorkCheckoutLease {
                    work_item_id: work.clone(),
                    owner_agent_instance_id: AgentInstanceId::new("agent-2"),
                    attempt_id: AttemptId::new("attempt-2"),
                    expires_at_unix_seconds: 10
                },
                0
            ),
            Err(WakeError::ActiveCheckout { .. })
        ));
        reopened
            .release_checkout(&work, &AttemptId::new("attempt-1"))
            .unwrap();
    }

    #[test]
    fn claim_is_compare_and_set_and_expired_lease_is_replaced() {
        let directory = tempfile::tempdir().unwrap();
        let repository = SqliteWakeRepository::open(&directory.path().join("work.db")).unwrap();
        let work = WorkItemId::new("work-1");
        repository
            .enqueue(work.clone(), WakeSource::Manual, "now".into())
            .unwrap();
        assert_eq!(
            repository
                .claim_wake(&work, "claimed".into())
                .unwrap()
                .claimed_at
                .as_deref(),
            Some("claimed")
        );
        assert!(matches!(
            repository.claim_wake(&work, "again".into()),
            Err(WakeError::AlreadyClaimed { .. })
        ));
        repository
            .checkout(
                WorkCheckoutLease {
                    work_item_id: work.clone(),
                    owner_agent_instance_id: AgentInstanceId::new("agent-1"),
                    attempt_id: AttemptId::new("expired"),
                    expires_at_unix_seconds: 10,
                },
                0,
            )
            .unwrap();
        repository
            .checkout(
                WorkCheckoutLease {
                    work_item_id: work.clone(),
                    owner_agent_instance_id: AgentInstanceId::new("agent-2"),
                    attempt_id: AttemptId::new("current"),
                    expires_at_unix_seconds: 11,
                },
                10,
            )
            .unwrap();
        assert!(matches!(
            repository.release_checkout(&work, &AttemptId::new("expired")),
            Err(WakeError::ActiveCheckout { .. })
        ));
        repository
            .release_checkout(&work, &AttemptId::new("current"))
            .unwrap();
    }
}
