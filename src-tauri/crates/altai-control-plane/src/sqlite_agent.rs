//! Local SQLite implementation of the CP-05 agent registry.

use crate::{AgentRepository, AgentRepositoryError};
use altai_control_protocol::{
    AgentInstance, AgentInstanceId, AgentProfileRevision, AgentProfileRevisionId, AgentStatus,
};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use std::{path::Path, sync::Mutex};

pub struct SqliteAgentRepository {
    connection: Mutex<Connection>,
}
impl SqliteAgentRepository {
    pub fn open(path: &Path) -> Result<Self, String> {
        let connection = Connection::open(path).map_err(|e| e.to_string())?;
        connection.execute_batch("PRAGMA foreign_keys = ON; PRAGMA busy_timeout = 5000; CREATE TABLE IF NOT EXISTS control_plane_agent_profile_revisions (id TEXT PRIMARY KEY, profile_id TEXT NOT NULL, revision INTEGER NOT NULL, payload_json TEXT NOT NULL); CREATE TABLE IF NOT EXISTS control_plane_agent_instances (id TEXT PRIMARY KEY, organization_id TEXT NOT NULL, profile_revision_id TEXT NOT NULL REFERENCES control_plane_agent_profile_revisions(id), reports_to_agent_id TEXT REFERENCES control_plane_agent_instances(id), status TEXT NOT NULL, payload_json TEXT NOT NULL);").map_err(|e| e.to_string())?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, AgentRepositoryError> {
        self.connection
            .lock()
            .map_err(|_| AgentRepositoryError::Internal {
                reason: "sqlite agent registry lock poisoned".into(),
            })
    }
    fn db(error: rusqlite::Error) -> AgentRepositoryError {
        AgentRepositoryError::Internal {
            reason: error.to_string(),
        }
    }
    fn exists(
        connection: &Connection,
        table: &str,
        id: &str,
    ) -> Result<bool, AgentRepositoryError> {
        connection
            .query_row(
                &format!("SELECT 1 FROM {table} WHERE id = ?1"),
                [id],
                |_| Ok(()),
            )
            .optional()
            .map(|v| v.is_some())
            .map_err(Self::db)
    }
}
impl AgentRepository for SqliteAgentRepository {
    fn append_profile_revision(
        &self,
        revision: AgentProfileRevision,
    ) -> Result<(), AgentRepositoryError> {
        let id = revision.id.value.clone();
        let payload =
            serde_json::to_string(&revision).map_err(|e| AgentRepositoryError::Internal {
                reason: e.to_string(),
            })?;
        let inserted = self.lock()?.execute("INSERT INTO control_plane_agent_profile_revisions (id, profile_id, revision, payload_json) VALUES (?1, ?2, ?3, ?4) ON CONFLICT(id) DO NOTHING", params![id, revision.profile_id.value, revision.revision.value() as i64, payload]).map_err(Self::db)?;
        if inserted == 0 {
            return Err(AgentRepositoryError::AlreadyExists {
                entity: "agent profile revision",
                id,
            });
        }
        Ok(())
    }
    fn create_instance(&self, instance: AgentInstance) -> Result<(), AgentRepositoryError> {
        let mut connection = self.lock()?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(Self::db)?;
        if !Self::exists(
            &tx,
            "control_plane_agent_profile_revisions",
            &instance.profile_revision_id.value,
        )? {
            return Err(AgentRepositoryError::NotFound {
                entity: "agent profile revision",
                id: instance.profile_revision_id.value,
            });
        }
        let mut manager = instance.reports_to_agent_id.clone();
        while let Some(manager_id) = manager {
            if manager_id == instance.id {
                return Err(AgentRepositoryError::ReportingCycle {
                    agent_id: instance.id.value,
                });
            }
            manager = tx
                .query_row(
                    "SELECT reports_to_agent_id FROM control_plane_agent_instances WHERE id = ?1",
                    [&manager_id.value],
                    |row| row.get::<_, Option<String>>(0),
                )
                .optional()
                .map_err(Self::db)?
                .flatten()
                .map(AgentInstanceId::new);
            if manager.is_none()
                && !Self::exists(&tx, "control_plane_agent_instances", &manager_id.value)?
            {
                return Err(AgentRepositoryError::NotFound {
                    entity: "reporting agent",
                    id: manager_id.value,
                });
            }
        }
        let id = instance.id.value.clone();
        let status = match instance.status {
            AgentStatus::Active => "active",
            AgentStatus::Paused => "paused",
            AgentStatus::Terminated => "terminated",
        };
        let payload =
            serde_json::to_string(&instance).map_err(|e| AgentRepositoryError::Internal {
                reason: e.to_string(),
            })?;
        let inserted = tx.execute("INSERT INTO control_plane_agent_instances (id, organization_id, profile_revision_id, reports_to_agent_id, status, payload_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6) ON CONFLICT(id) DO NOTHING", params![id, instance.organization_id.value, instance.profile_revision_id.value, instance.reports_to_agent_id.as_ref().map(|v| v.value.clone()), status, payload]).map_err(Self::db)?;
        if inserted == 0 {
            return Err(AgentRepositoryError::AlreadyExists {
                entity: "agent instance",
                id,
            });
        }
        tx.commit().map_err(Self::db)?;
        Ok(())
    }
    fn get_profile_revision(
        &self,
        revision_id: &AgentProfileRevisionId,
    ) -> Result<AgentProfileRevision, AgentRepositoryError> {
        let payload: Option<String> = self
            .lock()?
            .query_row(
                "SELECT payload_json FROM control_plane_agent_profile_revisions WHERE id = ?1",
                [&revision_id.value],
                |row| row.get(0),
            )
            .optional()
            .map_err(Self::db)?;
        let payload = payload.ok_or_else(|| AgentRepositoryError::NotFound {
            entity: "agent profile revision",
            id: revision_id.value.clone(),
        })?;
        serde_json::from_str(&payload).map_err(|e| AgentRepositoryError::Internal {
            reason: e.to_string(),
        })
    }
    fn ensure_dispatchable(
        &self,
        agent_id: &AgentInstanceId,
    ) -> Result<AgentInstance, AgentRepositoryError> {
        let row: Option<(String, String)> = self
            .lock()?
            .query_row(
                "SELECT status, payload_json FROM control_plane_agent_instances WHERE id = ?1",
                [&agent_id.value],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(Self::db)?;
        let Some((status, payload)) = row else {
            return Err(AgentRepositoryError::NotFound {
                entity: "agent instance",
                id: agent_id.value.clone(),
            });
        };
        if status != "active" {
            return Err(AgentRepositoryError::NotDispatchable {
                agent_id: agent_id.value.clone(),
            });
        }
        serde_json::from_str(&payload).map_err(|e| AgentRepositoryError::Internal {
            reason: e.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use altai_control_protocol::{
        AgentProfileId, AgentProfileRevisionId, OrganizationId, Revision,
    };

    #[test]
    fn active_agent_is_durable_across_reopen() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("work.db");
        let revision = AgentProfileRevision {
            id: AgentProfileRevisionId::new("base-v1"),
            profile_id: AgentProfileId::new("base"),
            revision: Revision::INITIAL,
            instructions: "help".into(),
            model: None,
            capabilities: vec![],
            created_at: "now".into(),
        };
        SqliteAgentRepository::open(&database)
            .unwrap()
            .append_profile_revision(revision)
            .unwrap();
        let instance = AgentInstance {
            id: AgentInstanceId::new("agent-1"),
            organization_id: OrganizationId::new("local"),
            profile_revision_id: AgentProfileRevisionId::new("base-v1"),
            reports_to_agent_id: None,
            name: "Agent".into(),
            role: "worker".into(),
            capabilities: vec![],
            status: AgentStatus::Active,
            pause_reason: None,
            revision: Revision::INITIAL,
            created_at: "now".into(),
            updated_at: "now".into(),
        };
        SqliteAgentRepository::open(&database)
            .unwrap()
            .create_instance(instance)
            .unwrap();
        assert!(SqliteAgentRepository::open(&database)
            .unwrap()
            .ensure_dispatchable(&AgentInstanceId::new("agent-1"))
            .is_ok());
    }

    #[test]
    fn profile_revision_is_durable_across_reopen() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("work.db");
        let revision = AgentProfileRevision {
            id: AgentProfileRevisionId::new("base-v1"),
            profile_id: AgentProfileId::new("base"),
            revision: Revision::INITIAL,
            instructions: "help".into(),
            model: Some("openai/gpt-5".into()),
            capabilities: vec![],
            created_at: "now".into(),
        };
        SqliteAgentRepository::open(&database)
            .unwrap()
            .append_profile_revision(revision)
            .unwrap();
        assert_eq!(
            SqliteAgentRepository::open(&database)
                .unwrap()
                .get_profile_revision(&AgentProfileRevisionId::new("base-v1"))
                .unwrap()
                .model,
            Some("openai/gpt-5".into())
        );
    }
}
