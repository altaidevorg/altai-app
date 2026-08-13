//! Postgres implementation of the CP-04 scope repository.

use crate::{ScopeError, ScopeRepository};
use altai_control_protocol::{
    Goal, GoalId, Organization, OrganizationId, Project, ProjectWorkspace, Revision,
};
use postgres::{Client, NoTls};
use std::sync::Mutex;

/// Deployed durable scope adapter. It stores canonical protocol payloads while
/// keeping organization and parent references as queryable relational columns.
pub struct PostgresScopeRepository {
    client: Mutex<Client>,
}

impl PostgresScopeRepository {
    pub const DEFAULT_LOCAL_ORGANIZATION_ID: &'static str = "org_local";

    pub fn connect(url: &str) -> Result<Self, String> {
        let mut client = Client::connect(url, NoTls).map_err(|error| error.to_string())?;
        client
            .batch_execute(
                "
                CREATE TABLE IF NOT EXISTS control_plane_organizations (
                    id TEXT PRIMARY KEY,
                    payload JSONB NOT NULL
                );
                CREATE TABLE IF NOT EXISTS control_plane_goals (
                    id TEXT PRIMARY KEY,
                    organization_id TEXT NOT NULL REFERENCES control_plane_organizations(id),
                    parent_goal_id TEXT NULL REFERENCES control_plane_goals(id),
                    payload JSONB NOT NULL
                );
                CREATE TABLE IF NOT EXISTS control_plane_projects (
                    id TEXT PRIMARY KEY,
                    organization_id TEXT NOT NULL REFERENCES control_plane_organizations(id),
                    payload JSONB NOT NULL
                );
                CREATE TABLE IF NOT EXISTS control_plane_project_goals (
                    project_id TEXT NOT NULL REFERENCES control_plane_projects(id) ON DELETE CASCADE,
                    goal_id TEXT NOT NULL REFERENCES control_plane_goals(id),
                    PRIMARY KEY (project_id, goal_id)
                );
                CREATE TABLE IF NOT EXISTS control_plane_workspaces (
                    id TEXT PRIMARY KEY,
                    project_id TEXT NOT NULL REFERENCES control_plane_projects(id),
                    payload JSONB NOT NULL
                );
                ",
            )
            .map_err(|error| error.to_string())?;
        Ok(Self {
            client: Mutex::new(client),
        })
    }

    /// Create the migration-owned local organization exactly once. This uses a
    /// stable ID so daemon restarts never create duplicate local tenants.
    pub fn ensure_default_local_organization(&self) -> Result<Organization, ScopeError> {
        let organization = Organization {
            id: OrganizationId::new("local"),
            name: "Local organization".to_string(),
            revision: Revision::INITIAL,
            created_at: "1970-01-01T00:00:00Z".to_string(),
            updated_at: "1970-01-01T00:00:00Z".to_string(),
        };
        let mut client = self.lock()?;
        let payload =
            serde_json::to_value(&organization).map_err(|error| ScopeError::Internal {
                reason: error.to_string(),
            })?;
        client
            .execute(
                "INSERT INTO control_plane_organizations (id, payload) VALUES ($1, $2) ON CONFLICT DO NOTHING",
                &[&organization.id.value, &payload],
            )
            .map_err(Self::database_error)?;
        Ok(organization)
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Client>, ScopeError> {
        self.client.lock().map_err(|_| ScopeError::Internal {
            reason: "postgres scope lock poisoned".to_string(),
        })
    }

    fn database_error(error: postgres::Error) -> ScopeError {
        ScopeError::Internal {
            reason: error.to_string(),
        }
    }
}

impl ScopeRepository for PostgresScopeRepository {
    fn create_organization(&self, organization: Organization) -> Result<(), ScopeError> {
        let mut client = self.lock()?;
        let id = organization.id.value.clone();
        let payload = serde_json::to_value(organization).map_err(|error| ScopeError::Internal {
            reason: error.to_string(),
        })?;
        let inserted = client
            .execute(
                "INSERT INTO control_plane_organizations (id, payload) VALUES ($1, $2) ON CONFLICT DO NOTHING",
                &[&id, &payload],
            )
            .map_err(Self::database_error)?;
        if inserted == 0 {
            return Err(ScopeError::AlreadyExists {
                entity: "organization",
                id,
            });
        }
        Ok(())
    }

    fn create_goal(&self, goal: Goal) -> Result<(), ScopeError> {
        let mut client = self.lock()?;
        let mut tx = client.transaction().map_err(Self::database_error)?;
        let org_exists = tx
            .query_opt(
                "SELECT 1 FROM control_plane_organizations WHERE id = $1",
                &[&goal.organization_id.value],
            )
            .map_err(Self::database_error)?;
        if org_exists.is_none() {
            return Err(ScopeError::NotFound {
                entity: "organization",
                id: goal.organization_id.value,
            });
        }
        if let Some(parent) = &goal.parent_goal_id {
            let parent_org = tx
                .query_opt(
                    "SELECT organization_id FROM control_plane_goals WHERE id = $1",
                    &[&parent.value],
                )
                .map_err(Self::database_error)?
                .ok_or_else(|| ScopeError::NotFound {
                    entity: "parent goal",
                    id: parent.value.clone(),
                })?;
            let parent_org: String = parent_org.get(0);
            if parent_org != goal.organization_id.value {
                return Err(ScopeError::CrossOrganization {
                    entity: "parent goal",
                    id: parent.value.clone(),
                });
            }
        }
        let inserted = tx.execute(
            "INSERT INTO control_plane_goals (id, organization_id, parent_goal_id, payload) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
            &[&goal.id.value, &goal.organization_id.value, &goal.parent_goal_id.as_ref().map(|id| id.value.clone()), &serde_json::to_value(&goal).map_err(|e| ScopeError::Internal { reason: e.to_string() })?],
        ).map_err(Self::database_error)?;
        if inserted == 0 {
            return Err(ScopeError::AlreadyExists {
                entity: "goal",
                id: goal.id.value,
            });
        }
        tx.commit().map_err(Self::database_error)?;
        Ok(())
    }

    fn create_project(&self, project: Project) -> Result<(), ScopeError> {
        let mut client = self.lock()?;
        let mut tx = client.transaction().map_err(Self::database_error)?;
        if tx
            .query_opt(
                "SELECT 1 FROM control_plane_organizations WHERE id = $1",
                &[&project.organization_id.value],
            )
            .map_err(Self::database_error)?
            .is_none()
        {
            return Err(ScopeError::NotFound {
                entity: "organization",
                id: project.organization_id.value,
            });
        }
        for goal_id in &project.goal_ids {
            let row = tx
                .query_opt(
                    "SELECT organization_id FROM control_plane_goals WHERE id = $1",
                    &[&goal_id.value],
                )
                .map_err(Self::database_error)?
                .ok_or_else(|| ScopeError::NotFound {
                    entity: "goal",
                    id: goal_id.value.clone(),
                })?;
            let goal_org: String = row.get(0);
            if goal_org != project.organization_id.value {
                return Err(ScopeError::CrossOrganization {
                    entity: "goal",
                    id: goal_id.value.clone(),
                });
            }
        }
        let inserted = tx.execute(
            "INSERT INTO control_plane_projects (id, organization_id, payload) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
            &[&project.id.value, &project.organization_id.value, &serde_json::to_value(&project).map_err(|e| ScopeError::Internal { reason: e.to_string() })?],
        ).map_err(Self::database_error)?;
        if inserted == 0 {
            return Err(ScopeError::AlreadyExists {
                entity: "project",
                id: project.id.value,
            });
        }
        for goal_id in &project.goal_ids {
            tx.execute(
                "INSERT INTO control_plane_project_goals (project_id, goal_id) VALUES ($1, $2)",
                &[&project.id.value, &goal_id.value],
            )
            .map_err(Self::database_error)?;
        }
        tx.commit().map_err(Self::database_error)?;
        Ok(())
    }

    fn create_workspace(&self, workspace: ProjectWorkspace) -> Result<(), ScopeError> {
        let mut client = self.lock()?;
        if client
            .query_opt(
                "SELECT 1 FROM control_plane_projects WHERE id = $1",
                &[&workspace.project_id.value],
            )
            .map_err(Self::database_error)?
            .is_none()
        {
            return Err(ScopeError::NotFound {
                entity: "project",
                id: workspace.project_id.value,
            });
        }
        let inserted = client.execute(
            "INSERT INTO control_plane_workspaces (id, project_id, payload) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
            &[&workspace.id.value, &workspace.project_id.value, &serde_json::to_value(&workspace).map_err(|e| ScopeError::Internal { reason: e.to_string() })?],
        ).map_err(Self::database_error)?;
        if inserted == 0 {
            return Err(ScopeError::AlreadyExists {
                entity: "workspace",
                id: workspace.id.value,
            });
        }
        Ok(())
    }

    fn goal_ancestry(
        &self,
        organization_id: &OrganizationId,
        goal_id: &GoalId,
    ) -> Result<Vec<Goal>, ScopeError> {
        let mut client = self.lock()?;
        let mut result = Vec::new();
        let mut current = goal_id.value.clone();
        let mut seen = std::collections::HashSet::new();
        loop {
            if !seen.insert(current.clone()) {
                return Err(ScopeError::GoalCycle { goal_id: current });
            }
            let row = client.query_opt("SELECT organization_id, parent_goal_id, payload FROM control_plane_goals WHERE id = $1", &[&current]).map_err(Self::database_error)?
                .ok_or_else(|| ScopeError::NotFound { entity: "goal", id: current.clone() })?;
            let goal_org: String = row.get(0);
            if goal_org != organization_id.value {
                return Err(ScopeError::CrossOrganization {
                    entity: "goal",
                    id: current,
                });
            }
            let goal: Goal =
                serde_json::from_value(row.get(2)).map_err(|e| ScopeError::Internal {
                    reason: e.to_string(),
                })?;
            let parent: Option<String> = row.get(1);
            result.push(goal);
            match parent {
                Some(id) => current = id,
                None => return Ok(result),
            }
        }
    }
}
