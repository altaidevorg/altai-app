//! Local SQLite implementation of the CP-04 scope repository.
//!
//! Scope data shares the workspace `work.db`; it is not a second desktop
//! database and does not require a database server.

use crate::{ScopeError, ScopeRepository};
use altai_control_protocol::{
    Goal, GoalId, Organization, OrganizationId, Project, ProjectId, ProjectWorkspace, Revision,
    WorkspaceId,
};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use std::{collections::HashSet, path::Path, sync::Mutex};

pub struct SqliteScopeRepository {
    connection: Mutex<Connection>,
}

impl SqliteScopeRepository {
    pub fn open(path: &Path) -> Result<Self, String> {
        let connection = Connection::open(path).map_err(|e| e.to_string())?;
        connection.execute_batch("PRAGMA foreign_keys = ON; PRAGMA busy_timeout = 5000;
            CREATE TABLE IF NOT EXISTS control_plane_organizations (id TEXT PRIMARY KEY, payload_json TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS control_plane_goals (id TEXT PRIMARY KEY, organization_id TEXT NOT NULL REFERENCES control_plane_organizations(id), parent_goal_id TEXT REFERENCES control_plane_goals(id), payload_json TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS control_plane_projects (id TEXT PRIMARY KEY, organization_id TEXT NOT NULL REFERENCES control_plane_organizations(id), payload_json TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS control_plane_project_goals (project_id TEXT NOT NULL REFERENCES control_plane_projects(id) ON DELETE CASCADE, goal_id TEXT NOT NULL REFERENCES control_plane_goals(id), PRIMARY KEY(project_id, goal_id));
            CREATE TABLE IF NOT EXISTS control_plane_workspaces (id TEXT PRIMARY KEY, project_id TEXT NOT NULL REFERENCES control_plane_projects(id), payload_json TEXT NOT NULL);")
            .map_err(|e| e.to_string())?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }
    pub fn ensure_default_local_organization(&self) -> Result<Organization, ScopeError> {
        let organization = Organization {
            id: OrganizationId::new("local"),
            name: "Local organization".into(),
            revision: Revision::INITIAL,
            created_at: "1970-01-01T00:00:00Z".into(),
            updated_at: "1970-01-01T00:00:00Z".into(),
        };
        let payload = Self::json(&organization)?;
        self.lock()?.execute("INSERT INTO control_plane_organizations (id, payload_json) VALUES (?1, ?2) ON CONFLICT(id) DO NOTHING", params![organization.id.value, payload]).map_err(Self::db)?;
        Ok(organization)
    }
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, ScopeError> {
        self.connection.lock().map_err(|_| ScopeError::Internal {
            reason: "sqlite scope lock poisoned".into(),
        })
    }
    fn db(error: rusqlite::Error) -> ScopeError {
        ScopeError::Internal {
            reason: error.to_string(),
        }
    }
    fn json<T: serde::Serialize>(value: &T) -> Result<String, ScopeError> {
        serde_json::to_string(value).map_err(|e| ScopeError::Internal {
            reason: e.to_string(),
        })
    }
    fn org_exists(connection: &Connection, id: &str) -> Result<bool, ScopeError> {
        connection
            .query_row(
                "SELECT 1 FROM control_plane_organizations WHERE id = ?1",
                [id],
                |_| Ok(()),
            )
            .optional()
            .map(|v| v.is_some())
            .map_err(Self::db)
    }
}

impl ScopeRepository for SqliteScopeRepository {
    fn create_organization(&self, organization: Organization) -> Result<(), ScopeError> {
        let id = organization.id.value.clone();
        let inserted = self.lock()?.execute("INSERT INTO control_plane_organizations (id, payload_json) VALUES (?1, ?2) ON CONFLICT(id) DO NOTHING", params![id, Self::json(&organization)?]).map_err(Self::db)?;
        if inserted == 0 {
            return Err(ScopeError::AlreadyExists {
                entity: "organization",
                id,
            });
        }
        Ok(())
    }
    fn create_goal(&self, goal: Goal) -> Result<(), ScopeError> {
        let mut connection = self.lock()?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(Self::db)?;
        if !Self::org_exists(&tx, &goal.organization_id.value)? {
            return Err(ScopeError::NotFound {
                entity: "organization",
                id: goal.organization_id.value,
            });
        }
        if let Some(parent) = &goal.parent_goal_id {
            let parent_org: Option<String> = tx
                .query_row(
                    "SELECT organization_id FROM control_plane_goals WHERE id = ?1",
                    [&parent.value],
                    |row| row.get(0),
                )
                .optional()
                .map_err(Self::db)?;
            match parent_org {
                None => {
                    return Err(ScopeError::NotFound {
                        entity: "parent goal",
                        id: parent.value.clone(),
                    })
                }
                Some(org) if org != goal.organization_id.value => {
                    return Err(ScopeError::CrossOrganization {
                        entity: "parent goal",
                        id: parent.value.clone(),
                    })
                }
                _ => {}
            }
        }
        let id = goal.id.value.clone();
        let parent = goal.parent_goal_id.as_ref().map(|v| v.value.clone());
        let payload = Self::json(&goal)?;
        let inserted = tx.execute("INSERT INTO control_plane_goals (id, organization_id, parent_goal_id, payload_json) VALUES (?1, ?2, ?3, ?4) ON CONFLICT(id) DO NOTHING", params![id, goal.organization_id.value, parent, payload]).map_err(Self::db)?;
        if inserted == 0 {
            return Err(ScopeError::AlreadyExists { entity: "goal", id });
        }
        tx.commit().map_err(Self::db)?;
        Ok(())
    }
    fn create_project(&self, project: Project) -> Result<(), ScopeError> {
        let mut connection = self.lock()?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(Self::db)?;
        if !Self::org_exists(&tx, &project.organization_id.value)? {
            return Err(ScopeError::NotFound {
                entity: "organization",
                id: project.organization_id.value,
            });
        }
        for goal in &project.goal_ids {
            let org: Option<String> = tx
                .query_row(
                    "SELECT organization_id FROM control_plane_goals WHERE id = ?1",
                    [&goal.value],
                    |row| row.get(0),
                )
                .optional()
                .map_err(Self::db)?;
            match org {
                None => {
                    return Err(ScopeError::NotFound {
                        entity: "goal",
                        id: goal.value.clone(),
                    })
                }
                Some(found) if found != project.organization_id.value => {
                    return Err(ScopeError::CrossOrganization {
                        entity: "goal",
                        id: goal.value.clone(),
                    })
                }
                _ => {}
            }
        }
        let id = project.id.value.clone();
        let payload = Self::json(&project)?;
        let inserted = tx.execute("INSERT INTO control_plane_projects (id, organization_id, payload_json) VALUES (?1, ?2, ?3) ON CONFLICT(id) DO NOTHING", params![id, project.organization_id.value, payload]).map_err(Self::db)?;
        if inserted == 0 {
            return Err(ScopeError::AlreadyExists {
                entity: "project",
                id,
            });
        }
        for goal in &project.goal_ids {
            tx.execute(
                "INSERT INTO control_plane_project_goals (project_id, goal_id) VALUES (?1, ?2)",
                params![project.id.value, goal.value],
            )
            .map_err(Self::db)?;
        }
        tx.commit().map_err(Self::db)?;
        Ok(())
    }
    fn create_workspace(&self, workspace: ProjectWorkspace) -> Result<(), ScopeError> {
        let connection = self.lock()?;
        if connection
            .query_row(
                "SELECT 1 FROM control_plane_projects WHERE id = ?1",
                [&workspace.project_id.value],
                |_| Ok(()),
            )
            .optional()
            .map_err(Self::db)?
            .is_none()
        {
            return Err(ScopeError::NotFound {
                entity: "project",
                id: workspace.project_id.value,
            });
        }
        let id = workspace.id.value.clone();
        let inserted = connection.execute("INSERT INTO control_plane_workspaces (id, project_id, payload_json) VALUES (?1, ?2, ?3) ON CONFLICT(id) DO NOTHING", params![id, workspace.project_id.value, Self::json(&workspace)?]).map_err(Self::db)?;
        if inserted == 0 {
            return Err(ScopeError::AlreadyExists {
                entity: "workspace",
                id,
            });
        }
        Ok(())
    }
    fn get_project(&self, project_id: &ProjectId) -> Result<Project, ScopeError> {
        let payload: Option<String> = self
            .lock()?
            .query_row(
                "SELECT payload_json FROM control_plane_projects WHERE id = ?1",
                [&project_id.value],
                |row| row.get(0),
            )
            .optional()
            .map_err(Self::db)?;
        let payload = payload.ok_or_else(|| ScopeError::NotFound {
            entity: "project",
            id: project_id.value.clone(),
        })?;
        serde_json::from_str(&payload).map_err(|error| ScopeError::Internal {
            reason: error.to_string(),
        })
    }
    fn get_workspace(&self, workspace_id: &WorkspaceId) -> Result<ProjectWorkspace, ScopeError> {
        let payload: Option<String> = self
            .lock()?
            .query_row(
                "SELECT payload_json FROM control_plane_workspaces WHERE id = ?1",
                [&workspace_id.value],
                |row| row.get(0),
            )
            .optional()
            .map_err(Self::db)?;
        let payload = payload.ok_or_else(|| ScopeError::NotFound {
            entity: "workspace",
            id: workspace_id.value.clone(),
        })?;
        serde_json::from_str(&payload).map_err(|error| ScopeError::Internal {
            reason: error.to_string(),
        })
    }

    fn attach_workspace_checkout(
        &self,
        workspace_id: &WorkspaceId,
        local_path_hint: Option<String>,
        expected_revision: Revision,
        updated_at: String,
    ) -> Result<ProjectWorkspace, ScopeError> {
        let mut connection = self.lock()?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(Self::db)?;
        let payload: Option<String> = tx
            .query_row(
                "SELECT payload_json FROM control_plane_workspaces WHERE id = ?1",
                [&workspace_id.value],
                |row| row.get(0),
            )
            .optional()
            .map_err(Self::db)?;
        let payload = payload.ok_or_else(|| ScopeError::NotFound {
            entity: "workspace",
            id: workspace_id.value.clone(),
        })?;
        let mut workspace: ProjectWorkspace =
            serde_json::from_str(&payload).map_err(|error| ScopeError::Internal {
                reason: error.to_string(),
            })?;
        if workspace.revision != expected_revision {
            return Err(ScopeError::StaleRevision {
                entity: "workspace",
                id: workspace_id.value.clone(),
                expected: expected_revision,
                actual: workspace.revision,
            });
        }
        workspace.local_path_hint = local_path_hint;
        workspace.revision = workspace.revision.next();
        workspace.updated_at = updated_at;
        let payload = Self::json(&workspace)?;
        tx.execute(
            "UPDATE control_plane_workspaces SET payload_json = ?2 WHERE id = ?1",
            params![workspace_id.value, payload],
        )
        .map_err(Self::db)?;
        tx.commit().map_err(Self::db)?;
        Ok(workspace)
    }

    fn find_workspaces_by_path_hint(
        &self,
        local_path_hint: &str,
    ) -> Result<Vec<ProjectWorkspace>, ScopeError> {
        let connection = self.lock()?;
        let mut stmt = connection
            .prepare("SELECT payload_json FROM control_plane_workspaces")
            .map_err(Self::db)?;
        let payloads = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(Self::db)?;
        let mut matches = Vec::new();
        for payload in payloads {
            let workspace: ProjectWorkspace =
                serde_json::from_str(&payload.map_err(Self::db)?).map_err(|error| {
                    ScopeError::Internal {
                        reason: error.to_string(),
                    }
                })?;
            if workspace
                .local_path_hint
                .as_deref()
                .is_some_and(|hint| hint == local_path_hint)
            {
                matches.push(workspace);
            }
        }
        matches.sort_by(|a, b| a.id.value.cmp(&b.id.value));
        Ok(matches)
    }
    fn goal_ancestry(
        &self,
        organization_id: &OrganizationId,
        goal_id: &GoalId,
    ) -> Result<Vec<Goal>, ScopeError> {
        let connection = self.lock()?;
        let mut result = Vec::new();
        let mut current = goal_id.value.clone();
        let mut seen = HashSet::new();
        loop {
            if !seen.insert(current.clone()) {
                return Err(ScopeError::GoalCycle { goal_id: current });
            }
            let row: Option<(String, Option<String>, String)> = connection.query_row("SELECT organization_id, parent_goal_id, payload_json FROM control_plane_goals WHERE id = ?1", [&current], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).optional().map_err(Self::db)?;
            let Some((org, parent, payload)) = row else {
                return Err(ScopeError::NotFound {
                    entity: "goal",
                    id: current,
                });
            };
            if org != organization_id.value {
                return Err(ScopeError::CrossOrganization {
                    entity: "goal",
                    id: current,
                });
            }
            result.push(
                serde_json::from_str(&payload).map_err(|e| ScopeError::Internal {
                    reason: e.to_string(),
                })?,
            );
            match parent {
                Some(parent) => current = parent,
                None => return Ok(result),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_local_organization_survives_reopen() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("work.db");
        SqliteScopeRepository::open(&database)
            .unwrap()
            .ensure_default_local_organization()
            .unwrap();
        let repository = SqliteScopeRepository::open(&database).unwrap();
        assert!(repository
            .create_organization(Organization {
                id: OrganizationId::new("local"),
                name: "duplicate".into(),
                revision: Revision::INITIAL,
                created_at: "now".into(),
                updated_at: "now".into(),
            })
            .is_err());
    }

    fn seeded_repository(database: &std::path::Path) -> SqliteScopeRepository {
        let repository = SqliteScopeRepository::open(database).unwrap();
        repository
            .create_organization(Organization {
                id: OrganizationId::new("org"),
                name: "Org".into(),
                revision: Revision::INITIAL,
                created_at: "now".into(),
                updated_at: "now".into(),
            })
            .unwrap();
        repository
            .create_project(Project {
                id: ProjectId::new("project"),
                organization_id: OrganizationId::new("org"),
                goal_ids: vec![],
                name: "Project".into(),
                description: String::new(),
                status: altai_control_protocol::ProjectStatus::Active,
                revision: Revision::INITIAL,
                created_at: "now".into(),
                updated_at: "now".into(),
            })
            .unwrap();
        repository
            .create_workspace(ProjectWorkspace {
                id: WorkspaceId::new("ws"),
                project_id: ProjectId::new("project"),
                name: "Checkout".into(),
                repository_url: Some("https://github.com/altaidevorg/altai-app".into()),
                local_path_hint: Some("/old/path".into()),
                revision: Revision::INITIAL,
                created_at: "now".into(),
                updated_at: "now".into(),
            })
            .unwrap();
        repository
    }

    #[test]
    fn moved_checkout_keeps_identity_across_reopen() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("work.db");
        let repository = seeded_repository(&database);

        let attached = repository
            .attach_workspace_checkout(
                &WorkspaceId::new("ws"),
                Some("/new/path".into()),
                Revision::INITIAL,
                "later".into(),
            )
            .unwrap();
        assert_eq!(attached.revision, Revision::new(1));

        let reopened = SqliteScopeRepository::open(&database).unwrap();
        let stored = reopened.get_workspace(&WorkspaceId::new("ws")).unwrap();
        // Same identity and project; only the hint moved.
        assert_eq!(stored.id, WorkspaceId::new("ws"));
        assert_eq!(stored.project_id, ProjectId::new("project"));
        assert_eq!(stored.local_path_hint.as_deref(), Some("/new/path"));
        assert_eq!(stored.revision, Revision::new(1));
        // The new location resolves; the old one no longer does.
        assert!(reopened
            .find_workspaces_by_path_hint("/old/path")
            .unwrap()
            .is_empty());
        assert_eq!(
            reopened
                .find_workspaces_by_path_hint("/new/path")
                .unwrap()
                .len(),
            1
        );
        // A stale re-attach against the pre-move revision fails.
        assert!(matches!(
            reopened.attach_workspace_checkout(
                &WorkspaceId::new("ws"),
                Some("/again".into()),
                Revision::INITIAL,
                "later".into(),
            ),
            Err(ScopeError::StaleRevision { .. })
        ));
    }
}
