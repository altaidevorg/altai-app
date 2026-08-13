//! Durable agent profile and instance contracts.

use crate::{AgentInstanceId, AgentProfileId, AgentProfileRevisionId, OrganizationId, Revision};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentProfileRevision {
    pub id: AgentProfileRevisionId,
    pub profile_id: AgentProfileId,
    pub revision: Revision,
    pub instructions: String,
    pub model: Option<String>,
    pub capabilities: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Active,
    Paused,
    Terminated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentInstance {
    pub id: AgentInstanceId,
    pub organization_id: OrganizationId,
    pub profile_revision_id: AgentProfileRevisionId,
    pub reports_to_agent_id: Option<AgentInstanceId>,
    pub name: String,
    pub role: String,
    pub capabilities: Vec<String>,
    pub status: AgentStatus,
    pub pause_reason: Option<String>,
    pub revision: Revision,
    pub created_at: String,
    pub updated_at: String,
}

impl AgentInstance {
    pub fn can_receive_dispatch(&self) -> bool {
        self.status == AgentStatus::Active
    }
}
