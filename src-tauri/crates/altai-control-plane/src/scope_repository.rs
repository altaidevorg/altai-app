//! Repository boundary for the CP-04 global scope model.
//!
//! The in-memory implementation is deliberately useful in tests only. A later
//! adapter persists this exact port in Postgres; callers do not get to bypass
//! its organization and ancestry rules.

use altai_control_protocol::{
    Goal, GoalId, Organization, OrganizationId, Project, ProjectWorkspace,
};
use std::{collections::HashMap, sync::Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeError {
    AlreadyExists { entity: &'static str, id: String },
    NotFound { entity: &'static str, id: String },
    CrossOrganization { entity: &'static str, id: String },
    GoalCycle { goal_id: String },
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
