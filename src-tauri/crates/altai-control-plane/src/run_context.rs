//! CP-08 bounded execution context contract.
//!
//! Execution receives a concise, reproducible snapshot of canonical records,
//! not an unbounded conversation or a renderer-assembled context ferry.

use altai_control_protocol::{Goal, Project, ProjectWorkspace, WorkItem};

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
    OversizedIdentity,
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
}
