//! Actor identity for mutations.
//!
//! Every mutation command includes an [`Actor`] that identifies who
//! performed the change. An actor is never inferred from a session ID or
//! chat name; it is always explicitly supplied by the authenticated caller.

use crate::id::{AgentInstanceId, OrganizationId};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Who performed a mutation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Actor {
    /// A human user.
    User {
        id: UserId,
        /// Display name at the time of the mutation (denormalized for audit).
        display_name: String,
    },
    /// An agent instance acting on behalf of the control plane.
    Agent {
        id: AgentInstanceId,
        /// The attempt that authorized this action, if applicable.
        attempt_id: Option<String>,
    },
    /// The control-plane system itself (e.g. recovery sweeps, scheduler).
    System {
        /// Free-form component name for diagnostics.
        component: String,
    },
    /// An external integration (e.g. GitHub webhook).
    External {
        integration: String,
        external_actor_id: String,
    },
}

/// The kind of actor, for quick filtering without matching the full enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActorKind {
    User,
    Agent,
    System,
    External,
}

impl Actor {
    pub fn kind(&self) -> ActorKind {
        match self {
            Self::User { .. } => ActorKind::User,
            Self::Agent { .. } => ActorKind::Agent,
            Self::System { .. } => ActorKind::System,
            Self::External { .. } => ActorKind::External,
        }
    }

    /// The organization this actor belongs to, if known.
    pub fn organization(&self) -> Option<&OrganizationId> {
        match self {
            Self::User { id, .. } => Some(&id.organization_id),
            _ => None,
        }
    }
}

impl fmt::Display for Actor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::User { id, .. } => write!(f, "user:{}", id.value),
            Self::Agent { id, .. } => write!(f, "agent:{}", id.value),
            Self::System { component } => write!(f, "system:{}", component),
            Self::External { integration, external_actor_id } => {
                write!(f, "external:{}/{}", integration, external_actor_id)
            }
        }
    }
}

/// A user identity scoped to an organization. Distinct from agent IDs.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId {
    pub organization_id: OrganizationId,
    pub value: String,
}

impl UserId {
    pub fn new(organization_id: OrganizationId, value: impl Into<String>) -> Self {
        Self {
            organization_id,
            value: value.into(),
        }
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "user:{}/{}", self.organization_id.value, self.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actor_user_serializes() {
        let org = OrganizationId::new("test-org");
        let user = UserId::new(org, "alice");
        let actor = Actor::User {
            id: user,
            display_name: "Alice".to_string(),
        };
        let json = serde_json::to_string(&actor).unwrap();
        assert!(json.contains("\"kind\":\"user\""));
        assert!(json.contains("\"alice\""));
    }

    #[test]
    fn actor_agent_serializes() {
        let agent = AgentInstanceId::new("dev-agent-1");
        let actor = Actor::Agent {
            id: agent,
            attempt_id: Some("att_123".to_string()),
        };
        let json = serde_json::to_string(&actor).unwrap();
        assert!(json.contains("\"kind\":\"agent\""));
    }

    #[test]
    fn actor_system_serializes() {
        let actor = Actor::System {
            component: "recovery_sweep".to_string(),
        };
        let json = serde_json::to_string(&actor).unwrap();
        assert!(json.contains("\"kind\":\"system\""));
    }

    #[test]
    fn actor_kind_matches() {
        let org = OrganizationId::new("test-org");
        let actor = Actor::User {
            id: UserId::new(org, "bob"),
            display_name: "Bob".to_string(),
        };
        assert_eq!(actor.kind(), ActorKind::User);
    }
}
