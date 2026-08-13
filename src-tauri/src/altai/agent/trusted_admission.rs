//! CP-08-08 host-only composition for one authorized attempt admission.
//!
//! This deliberately returns an in-process value. It is not a Tauri command
//! and is never serialized, because it contains the host-resolved credential.

use altai_agent_service::AttemptExecutionRequest;
use altai_control_plane::{
    load_bounded_run_context, AgentRepository, BoundedRunContext, ExecutionSnapshotRepository,
    ScopeRepository, WorkItemRepository,
};
use altai_control_protocol::{Attempt, AttemptId, RunBinding, SessionId, WorkspaceId};
use tauri::AppHandle;

use super::{
    attempt_adapter::{adapt_attempt, TrustedAttemptInput},
    execution_profile::resolve_authorized_execution_profile,
    trusted_profile::{resolve_trusted_execution_profile, TrustedExecutionProfile},
};
use crate::modules::secrets::SecretsState;

/// All data needed to admit a canonical attempt into the execution runtime.
/// `profile` must remain on the native side because it carries an API key.
#[allow(dead_code)] // CP-08-09 calls this from the native start command.
pub struct TrustedAttemptAdmission {
    pub execution: AttemptExecutionRequest,
    pub profile: TrustedExecutionProfile,
    pub instructions: String,
}

/// Durable, non-secret inputs selected entirely by native repositories before
/// an admission is composed. The renderer never provides the WorkItem or its
/// context pack.
#[allow(dead_code)] // wired by the scheduler-owned native handoff in the next slice.
pub struct DurableAttemptAdmissionInput {
    pub attempt: Attempt,
    pub run_binding: RunBinding,
    pub context: BoundedRunContext,
}

/// Load the immutable execution snapshot and derive its bounded canonical
/// context within a selected workspace. Every lookup is fail-closed.
#[allow(dead_code)] // exposed as a native-only composition seam before handoff wiring.
pub fn load_durable_attempt_admission_input(
    snapshots: &dyn ExecutionSnapshotRepository,
    scopes: &dyn ScopeRepository,
    work_items: &dyn WorkItemRepository,
    workspace_id: &WorkspaceId,
    attempt_id: &AttemptId,
) -> Result<DurableAttemptAdmissionInput, String> {
    let snapshot = snapshots
        .load(attempt_id)
        .map_err(|error| format!("Attempt execution snapshot failed: {error}"))?;
    let context = load_bounded_run_context(
        scopes,
        work_items,
        workspace_id,
        &snapshot.attempt.work_item_id,
    )
    .map_err(|error| format!("Canonical run context failed: {error}"))?;
    Ok(DurableAttemptAdmissionInput {
        attempt: snapshot.attempt,
        run_binding: snapshot.run_binding,
        context,
    })
}

/// Compose one native-only execution admission from durable records.
///
/// The caller provides no model, provider, endpoint, or API key: those values
/// are derived from the attempt's owner revision and the OS secret store.
#[allow(dead_code)] // staged with its CP-08-09 caller to keep the seam testable.
#[allow(clippy::too_many_arguments)]
pub fn prepare_trusted_attempt_admission(
    app: &AppHandle,
    secrets_state: &SecretsState,
    agents: &dyn AgentRepository,
    attempt: Attempt,
    run_binding: RunBinding,
    session_id: SessionId,
    prompt: String,
    context_pack: String,
    permission_policy: String,
) -> Result<TrustedAttemptAdmission, String> {
    let authorized = resolve_authorized_execution_profile(agents, &attempt)
        .map_err(|error| format!("Attempt profile authorization failed: {error:?}"))?;
    let profile = resolve_trusted_execution_profile(
        app,
        secrets_state,
        authorized.revision.model.as_deref(),
        &permission_policy,
    )?;
    let execution = adapt_attempt(TrustedAttemptInput {
        attempt,
        run_binding,
        session_id,
        prompt,
        context_pack,
        permission_policy: profile.permission_mode.clone(),
    })
    .map_err(|error| error.to_string())?;
    Ok(TrustedAttemptAdmission {
        execution,
        profile,
        instructions: authorized.revision.instructions,
    })
}

/// Compose a native-only admission from stable IDs and repositories. This is
/// the preferred CP-08 path; the legacy lower-level constructor remains for
/// migration-only host callers that already possess trusted durable values.
#[allow(clippy::too_many_arguments)]
#[allow(dead_code)] // scheduler-owned handoff consumes this native-only path next.
pub fn prepare_durable_trusted_attempt_admission(
    app: &AppHandle,
    secrets_state: &SecretsState,
    agents: &dyn AgentRepository,
    snapshots: &dyn ExecutionSnapshotRepository,
    scopes: &dyn ScopeRepository,
    work_items: &dyn WorkItemRepository,
    workspace_id: &WorkspaceId,
    attempt_id: &AttemptId,
    session_id: SessionId,
    prompt: String,
    permission_policy: String,
) -> Result<TrustedAttemptAdmission, String> {
    let input = load_durable_attempt_admission_input(
        snapshots,
        scopes,
        work_items,
        workspace_id,
        attempt_id,
    )?;
    prepare_trusted_attempt_admission(
        app,
        secrets_state,
        agents,
        input.attempt,
        input.run_binding,
        session_id,
        prompt,
        input.context.text,
        permission_policy,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use altai_control_plane::{
        AttemptRepository, RunBindingRepository, ScopeRepository, SqliteAttemptRepository,
        SqliteExecutionSnapshotRepository, SqliteRunBindingRepository, SqliteScopeRepository,
        SqliteWorkItemRepository, WorkItemRepository,
    };
    use altai_control_protocol::{
        AgentInstanceId, AgentProfileRevisionId, AttemptState, ExecutionPhase, Organization,
        OrganizationId, Project, ProjectId, ProjectStatus, ProjectWorkspace, Revision, RunBinding,
        RunId, WorkItem, WorkItemId, WorkItemKind, WorkStatus,
    };

    #[test]
    fn loader_derives_context_from_the_durable_attempt_snapshot() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("work.db");
        let organization_id = OrganizationId::new("org");
        let project_id = ProjectId::new("project");
        let workspace_id = WorkspaceId::new("workspace");
        let work_item_id = WorkItemId::new("work");
        let attempt_id = AttemptId::new("attempt");
        let scopes = SqliteScopeRepository::open(&database).unwrap();
        scopes
            .create_organization(Organization {
                id: organization_id.clone(),
                name: "Org".into(),
                revision: Revision::INITIAL,
                created_at: "now".into(),
                updated_at: "now".into(),
            })
            .unwrap();
        scopes
            .create_project(Project {
                id: project_id.clone(),
                organization_id,
                goal_ids: vec![],
                name: "Project".into(),
                description: String::new(),
                status: ProjectStatus::Active,
                revision: Revision::INITIAL,
                created_at: "now".into(),
                updated_at: "now".into(),
            })
            .unwrap();
        scopes
            .create_workspace(ProjectWorkspace {
                id: workspace_id.clone(),
                project_id: project_id.clone(),
                name: "Local".into(),
                repository_url: None,
                local_path_hint: Some("/not-context".into()),
                revision: Revision::INITIAL,
                created_at: "now".into(),
                updated_at: "now".into(),
            })
            .unwrap();
        SqliteWorkItemRepository::open(&database)
            .unwrap()
            .create(WorkItem {
                id: work_item_id.clone(),
                project_id,
                goal_id: None,
                parent_work_item_id: None,
                kind: WorkItemKind::Task,
                title: "Canonical work".into(),
                description: String::new(),
                status: WorkStatus::Todo,
                execution_phase: ExecutionPhase::Queued,
                revision: Revision::INITIAL,
                created_at: "now".into(),
                updated_at: "now".into(),
            })
            .unwrap();
        let attempt = Attempt {
            id: attempt_id.clone(),
            work_item_id: work_item_id.clone(),
            owner_agent_instance_id: AgentInstanceId::new("agent"),
            profile_revision_id: AgentProfileRevisionId::new("profile"),
            state: AttemptState::Created,
            created_at_unix_seconds: 1,
            updated_at_unix_seconds: 1,
        };
        let attempts = SqliteAttemptRepository::open(&database).unwrap();
        attempts.create(attempt).unwrap();
        attempts
            .transition(&attempt_id, AttemptState::Claimed, 2)
            .unwrap();
        attempts
            .transition(&attempt_id, AttemptState::Dispatched, 3)
            .unwrap();
        SqliteRunBindingRepository::open(&database)
            .unwrap()
            .bind(RunBinding {
                attempt_id: attempt_id.clone(),
                work_item_id,
                owner_agent_instance_id: AgentInstanceId::new("agent"),
                run_id: RunId::new("run"),
                bound_at_unix_seconds: 1,
            })
            .unwrap();

        let input = load_durable_attempt_admission_input(
            &SqliteExecutionSnapshotRepository::open(&database).unwrap(),
            &scopes,
            &SqliteWorkItemRepository::open(&database).unwrap(),
            &workspace_id,
            &attempt_id,
        )
        .unwrap();
        assert!(input.context.text.contains("work_title: Canonical work"));
        assert!(!input.context.text.contains("/not-context"));
    }
}
