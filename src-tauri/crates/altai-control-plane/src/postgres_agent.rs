//! Postgres implementation of the CP-05 agent registry.

use crate::{AgentRepository, AgentRepositoryError};
use altai_control_protocol::{AgentInstance, AgentInstanceId, AgentProfileRevision, AgentStatus};
use postgres::{Client, NoTls};
use std::sync::Mutex;

pub struct PostgresAgentRepository {
    client: Mutex<Client>,
}

impl PostgresAgentRepository {
    pub fn connect(url: &str) -> Result<Self, String> {
        let mut client = Client::connect(url, NoTls).map_err(|e| e.to_string())?;
        client.batch_execute("\
            CREATE TABLE IF NOT EXISTS control_plane_agent_profile_revisions (\
              id TEXT PRIMARY KEY, profile_id TEXT NOT NULL, revision BIGINT NOT NULL, payload JSONB NOT NULL\
            );\
            CREATE TABLE IF NOT EXISTS control_plane_agent_instances (\
              id TEXT PRIMARY KEY, organization_id TEXT NOT NULL, profile_revision_id TEXT NOT NULL REFERENCES control_plane_agent_profile_revisions(id),\
              reports_to_agent_id TEXT NULL REFERENCES control_plane_agent_instances(id), status TEXT NOT NULL, payload JSONB NOT NULL\
            );")
            .map_err(|e| e.to_string())?;
        Ok(Self {
            client: Mutex::new(client),
        })
    }
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Client>, AgentRepositoryError> {
        self.client
            .lock()
            .map_err(|_| AgentRepositoryError::Internal {
                reason: "postgres agent registry lock poisoned".to_string(),
            })
    }
    fn database_error(error: postgres::Error) -> AgentRepositoryError {
        AgentRepositoryError::Internal {
            reason: error.to_string(),
        }
    }
}

impl AgentRepository for PostgresAgentRepository {
    fn append_profile_revision(
        &self,
        revision: AgentProfileRevision,
    ) -> Result<(), AgentRepositoryError> {
        let mut client = self.lock()?;
        let id = revision.id.value.clone();
        let payload =
            serde_json::to_value(&revision).map_err(|e| AgentRepositoryError::Internal {
                reason: e.to_string(),
            })?;
        let inserted = client.execute("INSERT INTO control_plane_agent_profile_revisions (id, profile_id, revision, payload) VALUES ($1,$2,$3,$4) ON CONFLICT DO NOTHING", &[&id, &revision.profile_id.value, &(revision.revision.value() as i64), &payload]).map_err(Self::database_error)?;
        if inserted == 0 {
            return Err(AgentRepositoryError::AlreadyExists {
                entity: "agent profile revision",
                id,
            });
        }
        Ok(())
    }
    fn create_instance(&self, instance: AgentInstance) -> Result<(), AgentRepositoryError> {
        let mut client = self.lock()?;
        let mut tx = client.transaction().map_err(Self::database_error)?;
        if tx
            .query_opt(
                "SELECT 1 FROM control_plane_agent_profile_revisions WHERE id=$1",
                &[&instance.profile_revision_id.value],
            )
            .map_err(Self::database_error)?
            .is_none()
        {
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
            let row = tx
                .query_opt(
                    "SELECT reports_to_agent_id FROM control_plane_agent_instances WHERE id=$1",
                    &[&manager_id.value],
                )
                .map_err(Self::database_error)?
                .ok_or_else(|| AgentRepositoryError::NotFound {
                    entity: "reporting agent",
                    id: manager_id.value.clone(),
                })?;
            manager = row.get::<_, Option<String>>(0).map(AgentInstanceId::new);
        }
        let id = instance.id.value.clone();
        let status = match instance.status {
            AgentStatus::Active => "active",
            AgentStatus::Paused => "paused",
            AgentStatus::Terminated => "terminated",
        };
        let payload =
            serde_json::to_value(&instance).map_err(|e| AgentRepositoryError::Internal {
                reason: e.to_string(),
            })?;
        let inserted = tx.execute("INSERT INTO control_plane_agent_instances (id,organization_id,profile_revision_id,reports_to_agent_id,status,payload) VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT DO NOTHING", &[&id,&instance.organization_id.value,&instance.profile_revision_id.value,&instance.reports_to_agent_id.as_ref().map(|x|x.value.clone()),&status,&payload]).map_err(Self::database_error)?;
        if inserted == 0 {
            return Err(AgentRepositoryError::AlreadyExists {
                entity: "agent instance",
                id,
            });
        }
        tx.commit().map_err(Self::database_error)?;
        Ok(())
    }
    fn ensure_dispatchable(
        &self,
        agent_id: &AgentInstanceId,
    ) -> Result<AgentInstance, AgentRepositoryError> {
        let mut client = self.lock()?;
        let row = client
            .query_opt(
                "SELECT status,payload FROM control_plane_agent_instances WHERE id=$1",
                &[&agent_id.value],
            )
            .map_err(Self::database_error)?
            .ok_or_else(|| AgentRepositoryError::NotFound {
                entity: "agent instance",
                id: agent_id.value.clone(),
            })?;
        let status: String = row.get(0);
        if status != "active" {
            return Err(AgentRepositoryError::NotDispatchable {
                agent_id: agent_id.value.clone(),
            });
        }
        serde_json::from_value(row.get(1)).map_err(|e| AgentRepositoryError::Internal {
            reason: e.to_string(),
        })
    }
}
