//! Canonical organization, goal, project, and workspace contracts.
//!
//! A workspace ID is durable project context. A local filesystem path is only
//! a host-specific locator and therefore cannot be used as its identity.

use crate::{Actor, GoalId, OrganizationId, ProjectId, Revision, WorkspaceId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Organization {
    pub id: OrganizationId,
    pub name: String,
    pub revision: Revision,
    pub created_at: String,
    pub updated_at: String,
}

/// A goal belongs to exactly one organization. `parent_goal_id` is constrained
/// to that organization by the repository and must never form a cycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Goal {
    pub id: GoalId,
    pub organization_id: OrganizationId,
    pub parent_goal_id: Option<GoalId>,
    pub owner: Option<Actor>,
    pub title: String,
    pub description: String,
    pub revision: Revision,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatus {
    Active,
    Paused,
    Archived,
}

/// A project is organization-scoped and can support multiple goals.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub id: ProjectId,
    pub organization_id: OrganizationId,
    pub goal_ids: Vec<GoalId>,
    pub name: String,
    pub description: String,
    pub status: ProjectStatus,
    pub revision: Revision,
    pub created_at: String,
    pub updated_at: String,
}

/// Durable workspace/repository context for a project.
///
/// `local_path_hint` may change when a checkout moves and is deliberately not
/// part of workspace identity or de-duplication.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectWorkspace {
    pub id: WorkspaceId,
    pub project_id: ProjectId,
    pub name: String,
    pub repository_url: Option<String>,
    pub local_path_hint: Option<String>,
    pub revision: Revision,
    pub created_at: String,
    pub updated_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_path_is_not_its_identity() {
        let workspace = ProjectWorkspace {
            id: WorkspaceId::new("design-system"),
            project_id: ProjectId::new("website"),
            name: "Website checkout".to_string(),
            repository_url: Some("https://github.com/altaidevorg/altai-app".to_string()),
            local_path_hint: Some("/Users/a/src/altai-app".to_string()),
            revision: Revision::INITIAL,
            created_at: "2026-08-13T00:00:00Z".to_string(),
            updated_at: "2026-08-13T00:00:00Z".to_string(),
        };

        assert_eq!(workspace.id.value, "ws_design-system");
        assert_ne!(workspace.id.value, workspace.local_path_hint.unwrap());
    }
}
