//! Local SQLite implementation of durable control-plane host registration.
//!
//! Tables live beside ALTAI Work data in the existing workspace `work.db`; no
//! desktop user needs a database server or Docker.

use crate::{RegistrationCommit, RegistrationRepository};
use altai_control_protocol::RegisteredHost;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use std::{path::Path, sync::Mutex};

pub struct SqliteRegistrationRepository {
    connection: Mutex<Connection>,
}

impl SqliteRegistrationRepository {
    pub fn open(path: &Path) -> Result<Self, String> {
        let connection = Connection::open(path).map_err(|error| error.to_string())?;
        connection
            .execute_batch(
                "
                PRAGMA foreign_keys = ON;
                CREATE TABLE IF NOT EXISTS control_plane_registration_grants (
                    token_digest TEXT PRIMARY KEY,
                    expires_at_unix_seconds INTEGER NOT NULL
                );
                CREATE TABLE IF NOT EXISTS control_plane_registered_hosts (
                    agent_instance_id TEXT PRIMARY KEY,
                    payload_json TEXT NOT NULL,
                    registered_at_unix_seconds INTEGER NOT NULL
                );
                ",
            )
            .map_err(|error| error.to_string())?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, String> {
        self.connection
            .lock()
            .map_err(|_| "sqlite registration lock poisoned".to_string())
    }
}

impl RegistrationRepository for SqliteRegistrationRepository {
    fn issue_grant(&self, digest: String, expires: u64) -> Result<(), String> {
        self.lock()?.execute(
            "INSERT INTO control_plane_registration_grants (token_digest, expires_at_unix_seconds) VALUES (?1, ?2)",
            params![digest, expires as i64],
        ).map_err(|error| error.to_string())?;
        Ok(())
    }

    fn consume_grant_and_register(
        &self,
        digest: &str,
        now: u64,
        host: RegisteredHost,
    ) -> Result<RegistrationCommit, String> {
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| error.to_string())?;
        let expiry = transaction.query_row(
            "SELECT expires_at_unix_seconds FROM control_plane_registration_grants WHERE token_digest = ?1",
            params![digest], |row| row.get::<_, i64>(0),
        ).optional().map_err(|error| error.to_string())?;
        let Some(expiry) = expiry else {
            return Ok(RegistrationCommit::InvalidGrant);
        };
        transaction
            .execute(
                "DELETE FROM control_plane_registration_grants WHERE token_digest = ?1",
                params![digest],
            )
            .map_err(|error| error.to_string())?;
        if expiry < now as i64 {
            transaction.commit().map_err(|error| error.to_string())?;
            return Ok(RegistrationCommit::ExpiredGrant);
        }
        let payload = serde_json::to_string(&host).map_err(|error| error.to_string())?;
        transaction.execute(
            "INSERT INTO control_plane_registered_hosts (agent_instance_id, payload_json, registered_at_unix_seconds) VALUES (?1, ?2, ?3) ON CONFLICT(agent_instance_id) DO UPDATE SET payload_json = excluded.payload_json, registered_at_unix_seconds = excluded.registered_at_unix_seconds",
            params![host.agent_instance_id.value, payload, host.registered_at_unix_seconds as i64],
        ).map_err(|error| error.to_string())?;
        transaction.commit().map_err(|error| error.to_string())?;
        Ok(RegistrationCommit::Registered(host))
    }

    fn registered_host_count(&self) -> Result<usize, String> {
        self.lock()?
            .query_row(
                "SELECT COUNT(*) FROM control_plane_registered_hosts",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map(|count| count as usize)
            .map_err(|error| error.to_string())
    }

    fn database_adapter_ready(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use altai_control_protocol::{AgentInstanceId, HostCapabilities, WorkspaceId};

    fn host() -> RegisteredHost {
        RegisteredHost {
            agent_instance_id: AgentInstanceId::new("local"),
            workspaces: vec![WorkspaceId::new("local")],
            capabilities: HostCapabilities {
                values: Default::default(),
            },
            registered_at_unix_seconds: 1,
        }
    }

    #[test]
    fn grant_consumption_and_host_write_share_one_sqlite_transaction() {
        let directory = tempfile::tempdir().unwrap();
        let repository =
            SqliteRegistrationRepository::open(&directory.path().join("work.db")).unwrap();
        repository.issue_grant("digest".to_string(), 10).unwrap();
        assert!(matches!(
            repository
                .consume_grant_and_register("digest", 1, host())
                .unwrap(),
            RegistrationCommit::Registered(_)
        ));
        assert_eq!(repository.registered_host_count().unwrap(), 1);
        assert!(matches!(
            repository
                .consume_grant_and_register("digest", 1, host())
                .unwrap(),
            RegistrationCommit::InvalidGrant
        ));
    }
}
