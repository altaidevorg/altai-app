//! Resolves the durable agent identity attached to an execution attempt.
//!
//! This is deliberately host-only: a webview may request work, but may not
//! choose the profile revision or model used to execute an admitted attempt.

use altai_control_plane::{AgentRepository, AgentRepositoryError};
use altai_control_protocol::{AgentProfileRevision, Attempt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizedExecutionProfile {
    pub agent_name: String,
    pub revision: AgentProfileRevision,
}

pub fn resolve_authorized_execution_profile(
    repository: &dyn AgentRepository,
    attempt: &Attempt,
) -> Result<AuthorizedExecutionProfile, AgentRepositoryError> {
    let agent = repository.ensure_dispatchable(&attempt.owner_agent_instance_id)?;
    if agent.profile_revision_id != attempt.profile_revision_id {
        return Err(AgentRepositoryError::Internal {
            reason: format!(
                "attempt {} does not match active agent {} profile revision",
                attempt.id.value, agent.id.value
            ),
        });
    }
    let revision = repository.get_profile_revision(&attempt.profile_revision_id)?;
    Ok(AuthorizedExecutionProfile {
        agent_name: agent.name,
        revision,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use altai_control_plane::InMemoryAgentRepository;
    use altai_control_protocol::{
        AgentInstance, AgentInstanceId, AgentProfileId, AgentProfileRevisionId, AgentStatus,
        AttemptId, AttemptState, OrganizationId, Revision, WorkItemId,
    };

    fn attempt(profile_revision_id: &str) -> Attempt {
        Attempt {
            id: AttemptId::new("one"),
            work_item_id: WorkItemId::new("one"),
            owner_agent_instance_id: AgentInstanceId::new("agent"),
            profile_revision_id: AgentProfileRevisionId::new(profile_revision_id),
            state: AttemptState::Dispatched,
            created_at_unix_seconds: 1,
            updated_at_unix_seconds: 2,
        }
    }

    fn repository() -> InMemoryAgentRepository {
        let repository = InMemoryAgentRepository::default();
        repository
            .append_profile_revision(AgentProfileRevision {
                id: AgentProfileRevisionId::new("profile-v1"),
                profile_id: AgentProfileId::new("profile"),
                revision: Revision::INITIAL,
                instructions: "complete the work".into(),
                model: Some("openai/gpt-5".into()),
                capabilities: vec!["code".into()],
                created_at: "now".into(),
            })
            .unwrap();
        repository
            .create_instance(AgentInstance {
                id: AgentInstanceId::new("agent"),
                organization_id: OrganizationId::new("local"),
                profile_revision_id: AgentProfileRevisionId::new("profile-v1"),
                reports_to_agent_id: None,
                name: "Build agent".into(),
                role: "worker".into(),
                capabilities: vec![],
                status: AgentStatus::Active,
                pause_reason: None,
                revision: Revision::INITIAL,
                created_at: "now".into(),
                updated_at: "now".into(),
            })
            .unwrap();
        repository
    }

    #[test]
    fn reads_the_immutable_profile_owned_by_the_attempt_agent() {
        let profile =
            resolve_authorized_execution_profile(&repository(), &attempt("profile-v1")).unwrap();
        assert_eq!(profile.agent_name, "Build agent");
        assert_eq!(profile.revision.model.as_deref(), Some("openai/gpt-5"));
    }

    #[test]
    fn rejects_an_attempt_with_a_different_profile_revision() {
        assert!(matches!(
            resolve_authorized_execution_profile(&repository(), &attempt("other-v1")),
            Err(AgentRepositoryError::Internal { .. })
        ));
    }
}
