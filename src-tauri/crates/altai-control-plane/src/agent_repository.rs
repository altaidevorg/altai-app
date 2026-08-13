//! CP-05 durable agent registry repository boundary.

use altai_control_protocol::{
    AgentInstance, AgentInstanceId, AgentProfileRevision, AgentProfileRevisionId, AgentStatus,
};
use std::{collections::HashMap, sync::Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentRepositoryError {
    AlreadyExists { entity: &'static str, id: String },
    NotFound { entity: &'static str, id: String },
    ReportingCycle { agent_id: String },
    NotDispatchable { agent_id: String },
    Internal { reason: String },
}

pub trait AgentRepository: Send + Sync {
    fn append_profile_revision(
        &self,
        revision: AgentProfileRevision,
    ) -> Result<(), AgentRepositoryError>;
    fn create_instance(&self, instance: AgentInstance) -> Result<(), AgentRepositoryError>;
    /// Reads an immutable revision by its canonical identifier. Execution
    /// adapters use this rather than accepting a model/profile from a client.
    fn get_profile_revision(
        &self,
        revision_id: &AgentProfileRevisionId,
    ) -> Result<AgentProfileRevision, AgentRepositoryError>;
    fn ensure_dispatchable(
        &self,
        agent_id: &AgentInstanceId,
    ) -> Result<AgentInstance, AgentRepositoryError>;
}

#[derive(Default)]
pub struct InMemoryAgentRepository {
    state: Mutex<AgentState>,
}
#[derive(Default)]
struct AgentState {
    revisions: HashMap<String, AgentProfileRevision>,
    instances: HashMap<String, AgentInstance>,
}

impl InMemoryAgentRepository {
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, AgentState>, AgentRepositoryError> {
        self.state
            .lock()
            .map_err(|_| AgentRepositoryError::Internal {
                reason: "agent registry lock poisoned".to_string(),
            })
    }
}

impl AgentRepository for InMemoryAgentRepository {
    fn append_profile_revision(
        &self,
        revision: AgentProfileRevision,
    ) -> Result<(), AgentRepositoryError> {
        let mut state = self.lock()?;
        let id = revision.id.value.clone();
        if state.revisions.insert(id.clone(), revision).is_some() {
            return Err(AgentRepositoryError::AlreadyExists {
                entity: "agent profile revision",
                id,
            });
        }
        Ok(())
    }

    fn create_instance(&self, instance: AgentInstance) -> Result<(), AgentRepositoryError> {
        let mut state = self.lock()?;
        let id = instance.id.value.clone();
        if state.instances.contains_key(&id) {
            return Err(AgentRepositoryError::AlreadyExists {
                entity: "agent instance",
                id,
            });
        }
        if !state
            .revisions
            .contains_key(&instance.profile_revision_id.value)
        {
            return Err(AgentRepositoryError::NotFound {
                entity: "agent profile revision",
                id: instance.profile_revision_id.value,
            });
        }
        let mut current = instance.reports_to_agent_id.clone();
        while let Some(manager_id) = current {
            if manager_id == instance.id {
                return Err(AgentRepositoryError::ReportingCycle {
                    agent_id: instance.id.value,
                });
            }
            let manager = state.instances.get(&manager_id.value).ok_or_else(|| {
                AgentRepositoryError::NotFound {
                    entity: "reporting agent",
                    id: manager_id.value.clone(),
                }
            })?;
            current = manager.reports_to_agent_id.clone();
        }
        state.instances.insert(id, instance);
        Ok(())
    }

    fn get_profile_revision(
        &self,
        revision_id: &AgentProfileRevisionId,
    ) -> Result<AgentProfileRevision, AgentRepositoryError> {
        self.lock()?
            .revisions
            .get(&revision_id.value)
            .cloned()
            .ok_or_else(|| AgentRepositoryError::NotFound {
                entity: "agent profile revision",
                id: revision_id.value.clone(),
            })
    }

    fn ensure_dispatchable(
        &self,
        agent_id: &AgentInstanceId,
    ) -> Result<AgentInstance, AgentRepositoryError> {
        let state = self.lock()?;
        let agent = state
            .instances
            .get(&agent_id.value)
            .cloned()
            .ok_or_else(|| AgentRepositoryError::NotFound {
                entity: "agent instance",
                id: agent_id.value.clone(),
            })?;
        if agent.status != AgentStatus::Active {
            return Err(AgentRepositoryError::NotDispatchable {
                agent_id: agent.id.value,
            });
        }
        Ok(agent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use altai_control_protocol::{
        AgentProfileId, AgentProfileRevisionId, OrganizationId, Revision,
    };

    fn profile_revision() -> AgentProfileRevision {
        AgentProfileRevision {
            id: AgentProfileRevisionId::new("base-v1"),
            profile_id: AgentProfileId::new("base"),
            revision: Revision::INITIAL,
            instructions: "be helpful".to_string(),
            model: None,
            capabilities: vec!["code".to_string()],
            created_at: "2026-08-13T00:00:00Z".to_string(),
        }
    }
    fn agent(id: &str, manager: Option<&str>, status: AgentStatus) -> AgentInstance {
        AgentInstance {
            id: AgentInstanceId::new(id),
            organization_id: OrganizationId::new("local"),
            profile_revision_id: AgentProfileRevisionId::new("base-v1"),
            reports_to_agent_id: manager.map(AgentInstanceId::new),
            name: id.to_string(),
            role: "worker".to_string(),
            capabilities: vec![],
            status,
            pause_reason: None,
            revision: Revision::INITIAL,
            created_at: "2026-08-13T00:00:00Z".to_string(),
            updated_at: "2026-08-13T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn active_agents_are_dispatchable_and_paused_agents_are_not() {
        let repository = InMemoryAgentRepository::default();
        repository
            .append_profile_revision(profile_revision())
            .unwrap();
        repository
            .create_instance(agent("active", None, AgentStatus::Active))
            .unwrap();
        repository
            .create_instance(agent("paused", None, AgentStatus::Paused))
            .unwrap();
        assert!(repository
            .ensure_dispatchable(&AgentInstanceId::new("active"))
            .is_ok());
        assert!(matches!(
            repository.ensure_dispatchable(&AgentInstanceId::new("paused")),
            Err(AgentRepositoryError::NotDispatchable { .. })
        ));
    }

    #[test]
    fn instances_share_immutable_profile_revision_without_shared_identity() {
        let repository = InMemoryAgentRepository::default();
        repository
            .append_profile_revision(profile_revision())
            .unwrap();
        repository
            .create_instance(agent("one", None, AgentStatus::Active))
            .unwrap();
        repository
            .create_instance(agent("two", None, AgentStatus::Active))
            .unwrap();
        assert_ne!(
            repository
                .ensure_dispatchable(&AgentInstanceId::new("one"))
                .unwrap()
                .id,
            repository
                .ensure_dispatchable(&AgentInstanceId::new("two"))
                .unwrap()
                .id
        );
    }

    #[test]
    fn retrieves_profile_revisions_by_canonical_id() {
        let repository = InMemoryAgentRepository::default();
        repository
            .append_profile_revision(profile_revision())
            .unwrap();
        assert_eq!(
            repository
                .get_profile_revision(&AgentProfileRevisionId::new("base-v1"))
                .unwrap()
                .instructions,
            "be helpful"
        );
    }
}
