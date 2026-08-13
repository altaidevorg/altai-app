//! CP-07-05 deterministic dispatch eligibility boundary.

use crate::{AgentRepository, AgentRepositoryError, WorkGraphError, WorkGraphRepository};
use altai_control_protocol::{AgentInstanceId, WorkItemId};
use std::{collections::HashSet, sync::Arc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchBlocker {
    AgentUnavailable { agent_instance_id: String },
    DependencyIncomplete { work_item_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchEligibility {
    pub eligible: bool,
    pub blockers: Vec<DispatchBlocker>,
}

/// Pure policy over repository-owned facts. The scheduler must evaluate this
/// before it claims a wake or creates an execution attempt.
pub struct DispatchEligibilityEngine {
    agents: Arc<dyn AgentRepository>,
    graph: Arc<dyn WorkGraphRepository>,
}

impl DispatchEligibilityEngine {
    pub fn new(agents: Arc<dyn AgentRepository>, graph: Arc<dyn WorkGraphRepository>) -> Self {
        Self { agents, graph }
    }

    pub fn evaluate(
        &self,
        work_item_id: &WorkItemId,
        agent_instance_id: &AgentInstanceId,
        completed_work_item_ids: &HashSet<String>,
    ) -> Result<DispatchEligibility, DispatchEligibilityError> {
        let mut blockers = Vec::new();
        match self.agents.ensure_dispatchable(agent_instance_id) {
            Ok(_) => {}
            Err(
                AgentRepositoryError::NotDispatchable { .. }
                | AgentRepositoryError::NotFound { .. },
            ) => blockers.push(DispatchBlocker::AgentUnavailable {
                agent_instance_id: agent_instance_id.value.clone(),
            }),
            Err(error) => return Err(DispatchEligibilityError::Agent(error)),
        }
        for dependency in self
            .graph
            .dependencies(work_item_id)
            .map_err(DispatchEligibilityError::Graph)?
        {
            if !completed_work_item_ids.contains(&dependency.blocker_work_item_id.value) {
                blockers.push(DispatchBlocker::DependencyIncomplete {
                    work_item_id: dependency.blocker_work_item_id.value,
                });
            }
        }
        Ok(DispatchEligibility {
            eligible: blockers.is_empty(),
            blockers,
        })
    }
}

#[derive(Debug)]
pub enum DispatchEligibilityError {
    Agent(AgentRepositoryError),
    Graph(WorkGraphError),
}
impl std::fmt::Display for DispatchEligibilityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "dispatch eligibility repository failure: {self:?}")
    }
}
impl std::error::Error for DispatchEligibilityError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InMemoryAgentRepository, InMemoryWorkGraphRepository};
    use altai_control_protocol::{
        AgentInstance, AgentProfileId, AgentProfileRevision, AgentProfileRevisionId, AgentStatus,
        OrganizationId, Revision, WorkDependency,
    };

    fn agent(status: AgentStatus) -> AgentInstance {
        AgentInstance {
            id: AgentInstanceId::new("agent"),
            organization_id: OrganizationId::new("local"),
            profile_revision_id: AgentProfileRevisionId::new("profile-v1"),
            reports_to_agent_id: None,
            name: "Agent".into(),
            role: "worker".into(),
            capabilities: vec![],
            status,
            pause_reason: None,
            revision: Revision::INITIAL,
            created_at: "now".into(),
            updated_at: "now".into(),
        }
    }
    fn engine(status: AgentStatus) -> (DispatchEligibilityEngine, WorkItemId) {
        let agents = Arc::new(InMemoryAgentRepository::default());
        agents
            .append_profile_revision(AgentProfileRevision {
                id: AgentProfileRevisionId::new("profile-v1"),
                profile_id: AgentProfileId::new("profile"),
                revision: Revision::INITIAL,
                instructions: String::new(),
                model: None,
                capabilities: vec![],
                created_at: "now".into(),
            })
            .unwrap();
        agents.create_instance(agent(status)).unwrap();
        let graph = Arc::new(InMemoryWorkGraphRepository::default());
        let work = WorkItemId::new("work");
        graph.register_work_item(work.clone()).unwrap();
        (DispatchEligibilityEngine::new(agents, graph), work)
    }

    #[test]
    fn active_agent_with_completed_dependencies_is_eligible() {
        let (engine, work) = engine(AgentStatus::Active);
        assert!(
            engine
                .evaluate(&work, &AgentInstanceId::new("agent"), &HashSet::new())
                .unwrap()
                .eligible
        );
    }

    #[test]
    fn paused_agent_and_unfinished_dependency_are_both_reported() {
        let (engine, work) = engine(AgentStatus::Paused);
        let graph = engine.graph.clone();
        let blocker = WorkItemId::new("blocker");
        graph.register_work_item(blocker.clone()).unwrap();
        graph
            .add_dependency(WorkDependency {
                work_item_id: work.clone(),
                blocker_work_item_id: blocker,
                created_at: "now".into(),
            })
            .unwrap();
        let result = engine
            .evaluate(&work, &AgentInstanceId::new("agent"), &HashSet::new())
            .unwrap();
        assert!(!result.eligible);
        assert_eq!(result.blockers.len(), 2);
    }
}
