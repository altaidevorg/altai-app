//! Repository boundary for the CP-04 global scope model.
//!
//! The in-memory implementation is deliberately useful in tests only. A later
//! adapter persists this exact port in local SQLite; callers do not get to bypass
//! its organization and ancestry rules.

use altai_control_protocol::{
    Goal, GoalId, Organization, OrganizationId, Project, ProjectId, ProjectWorkspace, Revision,
    WorkspaceId,
};
use std::{collections::HashMap, sync::Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeError {
    AlreadyExists { entity: &'static str, id: String },
    NotFound { entity: &'static str, id: String },
    CrossOrganization { entity: &'static str, id: String },
    GoalCycle { goal_id: String },
    StaleRevision {
        entity: &'static str,
        id: String,
        expected: Revision,
        actual: Revision,
    },
    Internal { reason: String },
}

impl std::fmt::Display for ScopeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyExists { entity, id } => write!(f, "{entity} already exists: {id}"),
            Self::NotFound { entity, id } => write!(f, "{entity} not found: {id}"),
            Self::CrossOrganization { entity, id } => {
                write!(f, "{entity} is outside the organization boundary: {id}")
            }
            Self::GoalCycle { goal_id } => write!(f, "goal ancestry contains a cycle: {goal_id}"),
            Self::StaleRevision {
                entity,
                id,
                expected,
                actual,
            } => write!(
                f,
                "{entity} {id} moved on: expected revision {expected:?}, found {actual:?}"
            ),
            Self::Internal { reason } => write!(f, "scope repository internal error: {reason}"),
        }
    }
}

impl std::error::Error for ScopeError {}

/// Durable CP-04 persistence port. All reads carrying an organization ID must
/// fail closed rather than returning a record from another organization.
pub trait ScopeRepository: Send + Sync {
    fn create_organization(&self, organization: Organization) -> Result<(), ScopeError>;
    fn create_goal(&self, goal: Goal) -> Result<(), ScopeError>;
    fn create_project(&self, project: Project) -> Result<(), ScopeError>;
    fn create_workspace(&self, workspace: ProjectWorkspace) -> Result<(), ScopeError>;
    fn get_project(&self, project_id: &ProjectId) -> Result<Project, ScopeError>;
    fn get_workspace(&self, workspace_id: &WorkspaceId) -> Result<ProjectWorkspace, ScopeError>;
    /// Re-attach a moved checkout: update **only** `local_path_hint` (plus
    /// `revision.next()` and `updated_at`) under optimistic concurrency. The
    /// workspace keeps its identity, project, name, and repository binding.
    fn attach_workspace_checkout(
        &self,
        workspace_id: &WorkspaceId,
        local_path_hint: Option<String>,
        expected_revision: Revision,
        updated_at: String,
    ) -> Result<ProjectWorkspace, ScopeError>;
    /// Every registered workspace whose hint exactly matches `local_path_hint`
    /// (ordered by id). Zero = not registered; many = caller disambiguates.
    fn find_workspaces_by_path_hint(
        &self,
        local_path_hint: &str,
    ) -> Result<Vec<ProjectWorkspace>, ScopeError>;
    fn goal_ancestry(
        &self,
        organization_id: &OrganizationId,
        goal_id: &GoalId,
    ) -> Result<Vec<Goal>, ScopeError>;
}

#[derive(Default)]
pub struct InMemoryScopeRepository {
    state: Mutex<ScopeState>,
}

#[derive(Default)]
struct ScopeState {
    organizations: HashMap<String, Organization>,
    goals: HashMap<String, Goal>,
    projects: HashMap<String, Project>,
    workspaces: HashMap<String, ProjectWorkspace>,
}

impl InMemoryScopeRepository {
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, ScopeState>, ScopeError> {
        self.state.lock().map_err(|_| ScopeError::Internal {
            reason: "scope lock poisoned".to_string(),
        })
    }
}

impl ScopeRepository for InMemoryScopeRepository {
    fn create_organization(&self, organization: Organization) -> Result<(), ScopeError> {
        let mut state = self.lock()?;
        let id = organization.id.value.clone();
        if state
            .organizations
            .insert(id.clone(), organization)
            .is_some()
        {
            return Err(ScopeError::AlreadyExists {
                entity: "organization",
                id,
            });
        }
        Ok(())
    }

    fn create_goal(&self, goal: Goal) -> Result<(), ScopeError> {
        let mut state = self.lock()?;
        let id = goal.id.value.clone();
        if state.goals.contains_key(&id) {
            return Err(ScopeError::AlreadyExists { entity: "goal", id });
        }
        if !state
            .organizations
            .contains_key(&goal.organization_id.value)
        {
            return Err(ScopeError::NotFound {
                entity: "organization",
                id: goal.organization_id.value,
            });
        }
        if let Some(parent_id) = &goal.parent_goal_id {
            let parent = state
                .goals
                .get(&parent_id.value)
                .ok_or_else(|| ScopeError::NotFound {
                    entity: "parent goal",
                    id: parent_id.value.clone(),
                })?;
            if parent.organization_id != goal.organization_id {
                return Err(ScopeError::CrossOrganization {
                    entity: "parent goal",
                    id: parent_id.value.clone(),
                });
            }
        }
        state.goals.insert(id, goal);
        Ok(())
    }

    fn create_project(&self, project: Project) -> Result<(), ScopeError> {
        let mut state = self.lock()?;
        let id = project.id.value.clone();
        if state.projects.contains_key(&id) {
            return Err(ScopeError::AlreadyExists {
                entity: "project",
                id,
            });
        }
        if !state
            .organizations
            .contains_key(&project.organization_id.value)
        {
            return Err(ScopeError::NotFound {
                entity: "organization",
                id: project.organization_id.value,
            });
        }
        for goal_id in &project.goal_ids {
            let goal = state
                .goals
                .get(&goal_id.value)
                .ok_or_else(|| ScopeError::NotFound {
                    entity: "goal",
                    id: goal_id.value.clone(),
                })?;
            if goal.organization_id != project.organization_id {
                return Err(ScopeError::CrossOrganization {
                    entity: "goal",
                    id: goal_id.value.clone(),
                });
            }
        }
        state.projects.insert(id, project);
        Ok(())
    }

    fn create_workspace(&self, workspace: ProjectWorkspace) -> Result<(), ScopeError> {
        let mut state = self.lock()?;
        let id = workspace.id.value.clone();
        if state.workspaces.contains_key(&id) {
            return Err(ScopeError::AlreadyExists {
                entity: "workspace",
                id,
            });
        }
        if !state.projects.contains_key(&workspace.project_id.value) {
            return Err(ScopeError::NotFound {
                entity: "project",
                id: workspace.project_id.value,
            });
        }
        state.workspaces.insert(id, workspace);
        Ok(())
    }

    fn get_project(&self, project_id: &ProjectId) -> Result<Project, ScopeError> {
        self.lock()?
            .projects
            .get(&project_id.value)
            .cloned()
            .ok_or_else(|| ScopeError::NotFound {
                entity: "project",
                id: project_id.value.clone(),
            })
    }

    fn get_workspace(&self, workspace_id: &WorkspaceId) -> Result<ProjectWorkspace, ScopeError> {
        self.lock()?
            .workspaces
            .get(&workspace_id.value)
            .cloned()
            .ok_or_else(|| ScopeError::NotFound {
                entity: "workspace",
                id: workspace_id.value.clone(),
            })
    }

    fn attach_workspace_checkout(
        &self,
        workspace_id: &WorkspaceId,
        local_path_hint: Option<String>,
        expected_revision: Revision,
        updated_at: String,
    ) -> Result<ProjectWorkspace, ScopeError> {
        let mut state = self.lock()?;
        let workspace = state
            .workspaces
            .get_mut(&workspace_id.value)
            .ok_or_else(|| ScopeError::NotFound {
                entity: "workspace",
                id: workspace_id.value.clone(),
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
        Ok(workspace.clone())
    }

    fn find_workspaces_by_path_hint(
        &self,
        local_path_hint: &str,
    ) -> Result<Vec<ProjectWorkspace>, ScopeError> {
        let state = self.lock()?;
        let mut matches: Vec<ProjectWorkspace> = state
            .workspaces
            .values()
            .filter(|workspace| {
                workspace
                    .local_path_hint
                    .as_deref()
                    .is_some_and(|hint| hint == local_path_hint)
            })
            .cloned()
            .collect();
        matches.sort_by(|a, b| a.id.value.cmp(&b.id.value));
        Ok(matches)
    }

    fn goal_ancestry(
        &self,
        organization_id: &OrganizationId,
        goal_id: &GoalId,
    ) -> Result<Vec<Goal>, ScopeError> {
        let state = self.lock()?;
        let mut ancestry = Vec::new();
        let mut current = goal_id.clone();
        let mut seen = std::collections::HashSet::new();
        loop {
            if !seen.insert(current.value.clone()) {
                return Err(ScopeError::GoalCycle {
                    goal_id: current.value,
                });
            }
            let goal = state
                .goals
                .get(&current.value)
                .ok_or_else(|| ScopeError::NotFound {
                    entity: "goal",
                    id: current.value.clone(),
                })?;
            if &goal.organization_id != organization_id {
                return Err(ScopeError::CrossOrganization {
                    entity: "goal",
                    id: goal.id.value.clone(),
                });
            }
            ancestry.push(goal.clone());
            match &goal.parent_goal_id {
                Some(parent) => current = parent.clone(),
                None => return Ok(ancestry),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use altai_control_protocol::{
        GoalId, OrganizationId, ProjectId, ProjectStatus, Revision, WorkspaceId,
    };

    fn organization(id: &str) -> Organization {
        Organization {
            id: OrganizationId::new(id),
            name: id.to_string(),
            revision: Revision::INITIAL,
            created_at: "2026-08-13T00:00:00Z".to_string(),
            updated_at: "2026-08-13T00:00:00Z".to_string(),
        }
    }

    fn goal(id: &str, organization_id: OrganizationId, parent_goal_id: Option<GoalId>) -> Goal {
        Goal {
            id: GoalId::new(id),
            organization_id,
            parent_goal_id,
            owner: None,
            title: id.to_string(),
            description: String::new(),
            revision: Revision::INITIAL,
            created_at: "2026-08-13T00:00:00Z".to_string(),
            updated_at: "2026-08-13T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn ancestry_is_ordered_from_goal_to_root() {
        let repository = InMemoryScopeRepository::default();
        let org = organization("a");
        repository.create_organization(org.clone()).unwrap();
        repository
            .create_goal(goal("root", org.id.clone(), None))
            .unwrap();
        repository
            .create_goal(goal("child", org.id.clone(), Some(GoalId::new("root"))))
            .unwrap();

        let ancestry = repository
            .goal_ancestry(&org.id, &GoalId::new("child"))
            .unwrap();
        assert_eq!(
            ancestry
                .iter()
                .map(|goal| goal.id.value.as_str())
                .collect::<Vec<_>>(),
            ["goal_child", "goal_root"]
        );
    }

    #[test]
    fn cross_organization_parent_and_project_goal_are_rejected() {
        let repository = InMemoryScopeRepository::default();
        let org_a = organization("a");
        let org_b = organization("b");
        repository.create_organization(org_a.clone()).unwrap();
        repository.create_organization(org_b.clone()).unwrap();
        repository
            .create_goal(goal("a-root", org_a.id.clone(), None))
            .unwrap();
        assert!(matches!(
            repository.create_goal(goal(
                "b-child",
                org_b.id.clone(),
                Some(GoalId::new("a-root"))
            )),
            Err(ScopeError::CrossOrganization { .. })
        ));

        let project = Project {
            id: ProjectId::new("b-project"),
            organization_id: org_b.id,
            goal_ids: vec![GoalId::new("a-root")],
            name: "B".to_string(),
            description: String::new(),
            status: ProjectStatus::Active,
            revision: Revision::INITIAL,
            created_at: "2026-08-13T00:00:00Z".to_string(),
            updated_at: "2026-08-13T00:00:00Z".to_string(),
        };
        assert!(matches!(
            repository.create_project(project),
            Err(ScopeError::CrossOrganization { .. })
        ));
    }

    #[test]
    fn workspace_requires_its_project_but_not_a_filesystem_path() {
        let repository = InMemoryScopeRepository::default();
        let workspace = ProjectWorkspace {
            id: WorkspaceId::new("portable"),
            project_id: ProjectId::new("missing"),
            name: "Portable".to_string(),
            repository_url: None,
            local_path_hint: None,
            revision: Revision::INITIAL,
            created_at: "2026-08-13T00:00:00Z".to_string(),
            updated_at: "2026-08-13T00:00:00Z".to_string(),
        };
        assert!(matches!(
            repository.create_workspace(workspace),
            Err(ScopeError::NotFound {
                entity: "project",
                ..
            })
        ));
    }

    fn seeded_workspace(id: &str, hint: Option<&str>) -> ProjectWorkspace {
        ProjectWorkspace {
            id: WorkspaceId::new(id),
            project_id: ProjectId::new("project"),
            name: "Checkout".to_string(),
            repository_url: Some("https://github.com/altaidevorg/altai-app".to_string()),
            local_path_hint: hint.map(str::to_string),
            revision: Revision::INITIAL,
            created_at: "2026-08-13T00:00:00Z".to_string(),
            updated_at: "2026-08-13T00:00:00Z".to_string(),
        }
    }

    fn seeded_repository() -> InMemoryScopeRepository {
        let repository = InMemoryScopeRepository::default();
        repository.create_organization(organization("a")).unwrap();
        let project = Project {
            id: ProjectId::new("project"),
            organization_id: OrganizationId::new("org_a"),
            goal_ids: vec![],
            name: "Project".to_string(),
            description: String::new(),
            status: ProjectStatus::Active,
            revision: Revision::INITIAL,
            created_at: "2026-08-13T00:00:00Z".to_string(),
            updated_at: "2026-08-13T00:00:00Z".to_string(),
        };
        repository.create_project(project).unwrap();
        repository
    }

    #[test]
    fn attach_moves_the_checkout_without_changing_identity() {
        let repository = seeded_repository();
        repository
            .create_workspace(seeded_workspace("ws", Some("/old/path")))
            .unwrap();

        let attached = repository
            .attach_workspace_checkout(
                &WorkspaceId::new("ws"),
                Some("/new/path".to_string()),
                Revision::INITIAL,
                "2026-08-14T00:00:00Z".to_string(),
            )
            .unwrap();
        // Identity, project, name, and repository binding are untouched.
        assert_eq!(attached.id, WorkspaceId::new("ws"));
        assert_eq!(attached.project_id, ProjectId::new("project"));
        assert_eq!(attached.name, "Checkout");
        assert_eq!(
            attached.repository_url.as_deref(),
            Some("https://github.com/altaidevorg/altai-app")
        );
        // Only the hint, revision, and updated_at moved.
        assert_eq!(attached.local_path_hint.as_deref(), Some("/new/path"));
        assert_eq!(attached.revision, Revision::new(1));
        assert_eq!(attached.updated_at, "2026-08-14T00:00:00Z");
        // Durable through get.
        assert_eq!(
            repository.get_workspace(&WorkspaceId::new("ws")).unwrap(),
            attached
        );
    }

    #[test]
    fn attach_rejects_a_stale_revision() {
        let repository = seeded_repository();
        repository
            .create_workspace(seeded_workspace("ws", Some("/old/path")))
            .unwrap();

        assert!(matches!(
            repository.attach_workspace_checkout(
                &WorkspaceId::new("ws"),
                Some("/new/path".to_string()),
                Revision::new(7),
                "2026-08-14T00:00:00Z".to_string(),
            ),
            Err(ScopeError::StaleRevision { .. })
        ));
        // The workspace is unchanged.
        let stored = repository.get_workspace(&WorkspaceId::new("ws")).unwrap();
        assert_eq!(stored.local_path_hint.as_deref(), Some("/old/path"));
        assert_eq!(stored.revision, Revision::INITIAL);
    }

    #[test]
    fn attach_rejects_an_unknown_workspace() {
        let repository = seeded_repository();
        assert!(matches!(
            repository.attach_workspace_checkout(
                &WorkspaceId::new("ghost"),
                Some("/any".to_string()),
                Revision::INITIAL,
                "2026-08-14T00:00:00Z".to_string(),
            ),
            Err(ScopeError::NotFound {
                entity: "workspace",
                ..
            })
        ));
    }

    #[test]
    fn find_by_path_hint_returns_exact_matches_in_id_order() {
        let repository = seeded_repository();
        repository
            .create_workspace(seeded_workspace("z", Some("/shared/checkout")))
            .unwrap();
        repository
            .create_workspace(seeded_workspace("a", Some("/shared/checkout")))
            .unwrap();
        repository
            .create_workspace(seeded_workspace("other", Some("/elsewhere")))
            .unwrap();

        let found = repository
            .find_workspaces_by_path_hint("/shared/checkout")
            .unwrap();
        let ids: Vec<&str> = found.iter().map(|ws| ws.id.value.as_str()).collect();
        assert_eq!(ids, vec!["ws_a", "ws_z"]);
        // No registered hint matches.
        assert!(repository
            .find_workspaces_by_path_hint("/not/registered")
            .unwrap()
            .is_empty());
    }

    #[test]
    fn resolution_follows_a_moved_checkout() {
        let repository = seeded_repository();
        repository
            .create_workspace(seeded_workspace("ws", Some("/old/path")))
            .unwrap();
        let attached = repository
            .attach_workspace_checkout(
                &WorkspaceId::new("ws"),
                Some("/new/path".to_string()),
                Revision::INITIAL,
                "2026-08-14T00:00:00Z".to_string(),
            )
            .unwrap();

        // The old location no longer resolves; the new one does.
        assert!(repository.find_workspaces_by_path_hint("/old/path").unwrap().is_empty());
        let found = repository.find_workspaces_by_path_hint("/new/path").unwrap();
        assert_eq!(found, vec![attached]);
    }
}
