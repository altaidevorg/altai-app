use crate::{RegistrationCommit, RegistrationRepository};
use altai_control_protocol::RegisteredHost;
use postgres::{Client, NoTls};
use std::sync::Mutex;

/// Deployed durable registration adapter; PGlite remains a separate adapter.
pub struct PostgresRegistrationRepository {
    client: Mutex<Client>,
}

impl PostgresRegistrationRepository {
    pub fn connect(url: &str) -> Result<Self, String> {
        let mut client = Client::connect(url, NoTls).map_err(|e| e.to_string())?;
        client
            .batch_execute(
                "
                CREATE TABLE IF NOT EXISTS control_plane_registration_grants (
                    token_digest TEXT PRIMARY KEY,
                    expires_at_unix_seconds BIGINT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS control_plane_registered_hosts (
                    agent_instance_id TEXT PRIMARY KEY,
                    payload JSONB NOT NULL,
                    registered_at_unix_seconds BIGINT NOT NULL
                );
                ",
            )
            .map_err(|error| error.to_string())?;
        Ok(Self {
            client: Mutex::new(client),
        })
    }
}
impl RegistrationRepository for PostgresRegistrationRepository {
    fn issue_grant(&self, digest: String, expires: u64) -> Result<(), String> {
        let mut c = self
            .client
            .lock()
            .map_err(|_| "postgres registration lock poisoned".to_string())?;
        c.execute(
            "INSERT INTO control_plane_registration_grants \
             (token_digest, expires_at_unix_seconds) VALUES ($1, $2)",
            &[&digest, &(expires as i64)],
        )
        .map_err(|error| error.to_string())?;
        Ok(())
    }
    fn consume_grant_and_register(
        &self,
        digest: &str,
        now: u64,
        host: RegisteredHost,
    ) -> Result<RegistrationCommit, String> {
        let mut c = self
            .client
            .lock()
            .map_err(|_| "postgres registration lock poisoned".to_string())?;
        let mut tx = c.transaction().map_err(|error| error.to_string())?;
        let Some(row) = tx
            .query_opt(
                "DELETE FROM control_plane_registration_grants \
                 WHERE token_digest = $1 RETURNING expires_at_unix_seconds",
                &[&digest],
            )
            .map_err(|error| error.to_string())?
        else {
            return Ok(RegistrationCommit::InvalidGrant);
        };
        let expires: i64 = row.get(0);
        if expires < now as i64 {
            tx.commit().map_err(|error| error.to_string())?;
            return Ok(RegistrationCommit::ExpiredGrant);
        }
        let payload = serde_json::to_value(&host).map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT INTO control_plane_registered_hosts \
             (agent_instance_id, payload, registered_at_unix_seconds) VALUES ($1, $2, $3) \
             ON CONFLICT (agent_instance_id) DO UPDATE \
             SET payload = EXCLUDED.payload, \
                 registered_at_unix_seconds = EXCLUDED.registered_at_unix_seconds",
            &[
                &host.agent_instance_id.value,
                &payload,
                &(host.registered_at_unix_seconds as i64),
            ],
        )
        .map_err(|error| error.to_string())?;
        tx.commit().map_err(|error| error.to_string())?;
        Ok(RegistrationCommit::Registered(host))
    }
    fn registered_host_count(&self) -> Result<usize, String> {
        let mut c = self
            .client
            .lock()
            .map_err(|_| "postgres registration lock poisoned".to_string())?;
        let n: i64 = c
            .query_one("SELECT COUNT(*) FROM control_plane_registered_hosts", &[])
            .map_err(|e| e.to_string())?
            .get(0);
        Ok(n as usize)
    }
    fn database_adapter_ready(&self) -> bool {
        true
    }
}
