//! CP-08 bounded execution context contract.
//!
//! Execution receives a concise, reproducible snapshot of canonical records,
//! not an unbounded conversation or a renderer-assembled context ferry.

use crate::{AttemptRepository, RunBindingRepository, ScopeRepository, WorkItemRepository};
use altai_control_protocol::{AttemptId, Goal, Project, ProjectWorkspace, WorkItem, WorkItemId, WorkspaceId};

pub const MAX_RUN_CONTEXT_BYTES: usize = 24 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunContextInput {
    pub workspace: ProjectWorkspace,
    pub project: Project,
    /// Ordered nearest-goal to root. The builder preserves this order so the
    /// active goal remains prominent without any hidden ranking policy.
    pub goal_ancestry: Vec<Goal>,
    pub work_item: WorkItem,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedRunContext {
    pub text: String,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunContextError {
    ScopeMismatch(&'static str),
    IntegrityMismatch(&'static str),
    OversizedIdentity,
    Repository(String),
}

impl std::fmt::Display for RunContextError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "run context error: {self:?}")
    }
}
impl std::error::Error for RunContextError {}

/// Build a deterministic, bounded context pack for one canonical WorkItem.
pub fn build_bounded_run_context(
    input: RunContextInput,
) -> Result<BoundedRunContext, RunContextError> {
    if input.workspace.project_id != input.project.id {
        return Err(RunContextError::ScopeMismatch("workspace project"));
    }
    if input.work_item.project_id != input.project.id {
        return Err(RunContextError::ScopeMismatch("work item project"));
    }
    if input
        .goal_ancestry
        .iter()
        .any(|goal| goal.organization_id != input.project.organization_id)
    {
        return Err(RunContextError::ScopeMismatch("goal organization"));
    }
    if let Some(goal_id) = &input.work_item.goal_id {
        if input.goal_ancestry.first().map(|goal| &goal.id) != Some(goal_id) {
            return Err(RunContextError::ScopeMismatch("work item goal"));
        }
    } else if !input.goal_ancestry.is_empty() {
        return Err(RunContextError::ScopeMismatch("unexpected goal ancestry"));
    }

    let mut text = String::new();
    push(&mut text, "workspace_id", &input.workspace.id.value)?;
    push(&mut text, "workspace_name", &input.workspace.name)?;
    push(&mut text, "project_id", &input.project.id.value)?;
    push(&mut text, "project_name", &input.project.name)?;
    for goal in &input.goal_ancestry {
        push(&mut text, "goal_id", &goal.id.value)?;
        push(&mut text, "goal_title", &goal.title)?;
        push(&mut text, "goal_description", &goal.description)?;
    }
    push(&mut text, "work_item_id", &input.work_item.id.value)?;
    push(&mut text, "work_title", &input.work_item.title)?;
    push(&mut text, "work_description", &input.work_item.description)?;
    push(
        &mut text,
        "work_status",
        &format!("{:?}", input.work_item.status),
    )?;

    let truncated = text.len() > MAX_RUN_CONTEXT_BYTES;
    if truncated {
        text.truncate(MAX_RUN_CONTEXT_BYTES);
        while !text.is_char_boundary(text.len()) {
            text.pop();
        }
    }
    Ok(BoundedRunContext { text, truncated })
}

/// Read scope records from the configured repository, then build a context for
/// an already-resolved canonical WorkItem. It never derives WorkItem fields
/// from renderer strings or filesystem paths.
pub fn assemble_bounded_run_context(
    scopes: &dyn ScopeRepository,
    workspace_id: &WorkspaceId,
    work_item: WorkItem,
) -> Result<BoundedRunContext, RunContextError> {
    let workspace = scopes
        .get_workspace(workspace_id)
        .map_err(|error| RunContextError::Repository(error.to_string()))?;
    let project = scopes
        .get_project(&workspace.project_id)
        .map_err(|error| RunContextError::Repository(error.to_string()))?;
    let goal_ancestry = match &work_item.goal_id {
        Some(goal_id) => scopes
            .goal_ancestry(&project.organization_id, goal_id)
            .map_err(|error| RunContextError::Repository(error.to_string()))?,
        None => Vec::new(),
    };
    build_bounded_run_context(RunContextInput {
        workspace,
        project,
        goal_ancestry,
        work_item,
    })
}

/// Resolve the WorkItem through its workspace's canonical project before
/// assembling context. A caller cannot substitute a renderer-provided item or
/// reuse an item that belongs to another project.
pub fn load_bounded_run_context(
    scopes: &dyn ScopeRepository,
    work_items: &dyn WorkItemRepository,
    workspace_id: &WorkspaceId,
    work_item_id: &WorkItemId,
) -> Result<BoundedRunContext, RunContextError> {
    let workspace = scopes
        .get_workspace(workspace_id)
        .map_err(|error| RunContextError::Repository(error.to_string()))?;
    let work_item = work_items
        .get_in_project(&workspace.project_id, work_item_id)
        .map_err(|error| RunContextError::Repository(error.to_string()))?;
    assemble_bounded_run_context(scopes, workspace_id, work_item)
}

/// Resolve context from the immutable Attempt and its run binding. This keeps
/// the WorkItem used for execution tied to the Attempt that the scheduler
/// authorized, rather than accepting a second WorkItem identifier at dispatch.
pub fn load_attempt_bound_run_context(
    attempts: &dyn AttemptRepository,
    bindings: &dyn RunBindingRepository,
    scopes: &dyn ScopeRepository,
    work_items: &dyn WorkItemRepository,
    workspace_id: &WorkspaceId,
    attempt_id: &AttemptId,
) -> Result<BoundedRunContext, RunContextError> {
    let attempt = attempts
        .get(attempt_id)
        .map_err(|error| RunContextError::Repository(error.to_string()))?
        .ok_or_else(|| RunContextError::Repository(format!("attempt not found: {}", attempt_id.value)))?;
    let binding = bindings
        .get(attempt_id)
        .map_err(|error| RunContextError::Repository(error.to_string()))?
        .ok_or_else(|| RunContextError::Repository(format!("run binding not found: {}", attempt_id.value)))?;
    if binding.work_item_id != attempt.work_item_id {
        return Err(RunContextError::IntegrityMismatch("attempt run binding work item"));
    }
    if binding.owner_agent_instance_id != attempt.owner_agent_instance_id {
        return Err(RunContextError::IntegrityMismatch("attempt run binding agent"));
    }
    load_bounded_run_context(scopes, work_items, workspace_id, &attempt.work_item_id)
}

fn push(output: &mut String, key: &str, value: &str) -> Result<(), RunContextError> {
    if key.len().saturating_add(value.len()).saturating_add(2) > MAX_RUN_CONTEXT_BYTES {
        return Err(RunContextError::OversizedIdentity);
    }
    output.push_str(key);
    output.push_str(": ");
    output.push_str(value);
    output.push('\n');
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use altai_control_protocol::{
        GoalId, OrganizationId, ProjectId, ProjectStatus, Revision, WorkItemId, WorkItemKind,
        WorkStatus, WorkspaceId,
    };

    fn input() -> RunContextInput {
        let organization_id = OrganizationId::new("org");
        let project = Project {
            id: ProjectId::new("project"),
            organization_id: organization_id.clone(),
            goal_ids: vec![GoalId::new("goal")],
            name: "App".into(),
            description: String::new(),
            status: ProjectStatus::Active,
            revision: Revision::INITIAL,
            created_at: "now".into(),
            updated_at: "now".into(),
        };
        RunContextInput {
            workspace: ProjectWorkspace {
                id: WorkspaceId::new("workspace"),
                project_id: project.id.clone(),
                name: "Local app".into(),
                repository_url: None,
                local_path_hint: Some("/not-an-identity".into()),
                revision: Revision::INITIAL,
                created_at: "now".into(),
                updated_at: "now".into(),
            },
            project: project.clone(),
            goal_ancestry: vec![Goal {
                id: GoalId::new("goal"),
                organization_id,
                parent_goal_id: None,
                owner: None,
                title: "Ship safely".into(),
                description: "Keep changes reviewable".into(),
                revision: Revision::INITIAL,
                created_at: "now".into(),
                updated_at: "now".into(),
            }],
            work_item: WorkItem {
                id: WorkItemId::new("work"),
                project_id: project.id,
                goal_id: Some(GoalId::new("goal")),
                parent_work_item_id: None,
                kind: WorkItemKind::Task,
                title: "Wire executor".into(),
                description: "Use durable IDs".into(),
                status: WorkStatus::Todo,
                execution_phase: altai_control_protocol::ExecutionPhase::Queued,
                revision: Revision::INITIAL,
                created_at: "now".into(),
                updated_at: "now".into(),
            },
        }
    }

    #[test]
    fn context_is_canonical_bounded_and_excludes_local_path_hints() {
        let context = build_bounded_run_context(input()).unwrap();
        assert!(context.text.contains("work_title: Wire executor"));
        assert!(context.text.contains("goal_title: Ship safely"));
        assert!(!context.text.contains("/not-an-identity"));
        assert!(!context.truncated);
    }

    #[test]
    fn context_fails_closed_for_cross_project_work() {
        let mut invalid = input();
        invalid.work_item.project_id = ProjectId::new("other");
        assert_eq!(
            build_bounded_run_context(invalid),
            Err(RunContextError::ScopeMismatch("work item project"))
        );
    }

    #[test]
    fn assembly_reads_scope_from_the_repository() {
        use crate::{InMemoryScopeRepository, ScopeRepository};
        let source = input();
        let scopes = InMemoryScopeRepository::default();
        scopes
            .create_organization(altai_control_protocol::Organization {
                id: source.project.organization_id.clone(),
                name: "Org".into(),
                revision: Revision::INITIAL,
                created_at: "now".into(),
                updated_at: "now".into(),
            })
            .unwrap();
        for goal in &source.goal_ancestry {
            scopes.create_goal(goal.clone()).unwrap();
        }
        scopes.create_project(source.project.clone()).unwrap();
        scopes.create_workspace(source.workspace.clone()).unwrap();
        let context =
            assemble_bounded_run_context(&scopes, &source.workspace.id, source.work_item).unwrap();
        assert!(context.text.contains("project_name: App"));
    }

    #[test]
    fn loading_context_resolves_the_work_item_in_the_workspace_project() {
        use crate::{
            ScopeRepository, SqliteScopeRepository, SqliteWorkItemRepository, WorkItemRepository,
        };

        let source = input();
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("work.db");
        let scopes = SqliteScopeRepository::open(&database).unwrap();
        scopes
            .create_organization(altai_control_protocol::Organization {
                id: source.project.organization_id.clone(),
                name: "Org".into(),
                revision: Revision::INITIAL,
                created_at: "now".into(),
                updated_at: "now".into(),
            })
            .unwrap();
        for goal in &source.goal_ancestry {
            scopes.create_goal(goal.clone()).unwrap();
        }
        scopes.create_project(source.project.clone()).unwrap();
        scopes.create_workspace(source.workspace.clone()).unwrap();
        let work_items = SqliteWorkItemRepository::open(&database).unwrap();
        work_items.create(source.work_item.clone()).unwrap();

        let context = load_bounded_run_context(
            &scopes,
            &work_items,
            &source.workspace.id,
            &source.work_item.id,
        )
        .unwrap();

        assert!(context.text.contains("work_title: Wire executor"));
    }

    #[test]
    fn attempt_bound_context_rejects_a_mismatched_run_binding() {
        use crate::{
            AttemptRepository, InMemoryScopeRepository, RunBindingRepository,
            SqliteAttemptRepository, SqliteRunBindingRepository, SqliteWorkItemRepository,
        };
        use altai_control_protocol::{
            AgentInstanceId, AgentProfileRevisionId, Attempt, AttemptId, AttemptState, RunBinding,
            RunId,
        };

        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("work.db");
        let attempts = SqliteAttemptRepository::open(&database).unwrap();
        let bindings = SqliteRunBindingRepository::open(&database).unwrap();
        attempts
            .create(Attempt {
                id: AttemptId::new("attempt"),
                work_item_id: WorkItemId::new("work-a"),
                owner_agent_instance_id: AgentInstanceId::new("agent"),
                profile_revision_id: AgentProfileRevisionId::new("profile"),
                state: AttemptState::Created,
                created_at_unix_seconds: 1,
                updated_at_unix_seconds: 1,
            })
            .unwrap();
        bindings
            .bind(RunBinding {
                attempt_id: AttemptId::new("attempt"),
                work_item_id: WorkItemId::new("work-b"),
                owner_agent_instance_id: AgentInstanceId::new("agent"),
                run_id: RunId::new("run"),
                bound_at_unix_seconds: 1,
            })
            .unwrap();

        let result = load_attempt_bound_run_context(
            &attempts,
            &bindings,
            &InMemoryScopeRepository::default(),
            &SqliteWorkItemRepository::open(&database).unwrap(),
            &WorkspaceId::new("workspace"),
            &AttemptId::new("attempt"),
        );
        assert_eq!(
            result,
            Err(RunContextError::IntegrityMismatch(
                "attempt run binding work item"
            ))
        );
    }

    #[test]
    fn attempt_bound_context_uses_the_attempt_work_item() {
        use crate::{
            AttemptRepository, RunBindingRepository, SqliteAttemptRepository,
            SqliteRunBindingRepository, SqliteScopeRepository, SqliteWorkItemRepository,
        };
        use altai_control_protocol::{
            AgentInstanceId, AgentProfileRevisionId, Attempt, AttemptId, AttemptState, RunBinding,
            RunId,
        };

        let source = input();
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("work.db");
        let scopes = SqliteScopeRepository::open(&database).unwrap();
        scopes
            .create_organization(altai_control_protocol::Organization {
                id: source.project.organization_id.clone(),
                name: "Org".into(),
                revision: Revision::INITIAL,
                created_at: "now".into(),
                updated_at: "now".into(),
            })
            .unwrap();
        for goal in &source.goal_ancestry {
            scopes.create_goal(goal.clone()).unwrap();
        }
        scopes.create_project(source.project.clone()).unwrap();
        scopes.create_workspace(source.workspace.clone()).unwrap();
        let work_items = SqliteWorkItemRepository::open(&database).unwrap();
        work_items.create(source.work_item.clone()).unwrap();
        let attempts = SqliteAttemptRepository::open(&database).unwrap();
        let bindings = SqliteRunBindingRepository::open(&database).unwrap();
        attempts
            .create(Attempt {
                id: AttemptId::new("attempt"),
                work_item_id: source.work_item.id.clone(),
                owner_agent_instance_id: AgentInstanceId::new("agent"),
                profile_revision_id: AgentProfileRevisionId::new("profile"),
                state: AttemptState::Created,
                created_at_unix_seconds: 1,
                updated_at_unix_seconds: 1,
            })
            .unwrap();
        bindings
            .bind(RunBinding {
                attempt_id: AttemptId::new("attempt"),
                work_item_id: source.work_item.id,
                owner_agent_instance_id: AgentInstanceId::new("agent"),
                run_id: RunId::new("run"),
                bound_at_unix_seconds: 1,
            })
            .unwrap();

        let context = load_attempt_bound_run_context(
            &attempts,
            &bindings,
            &scopes,
            &work_items,
            &source.workspace.id,
            &AttemptId::new("attempt"),
        )
        .unwrap();
        assert!(context.text.contains("work_title: Wire executor"));
    }
}
