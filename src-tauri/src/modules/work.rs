//! Work OS Milestone 1 — durable Work IPC over the existing host.
//!
//! Wraps `altai_core::WorkStore` at `WorkspacePaths::work_db()`. Authoritative
//! lifecycle lives here; IsanAgent remains execution-only (ADR 0003).

use std::path::Path;

use altai_core::{
    resolve_workspace_from, AttemptPhase, AttemptReconcileMode, AttemptRecord, CreateWorkInput,
    WorkAttemptStart, WorkInboxRecord, WorkItemKind, WorkItemRecord, WorkListFilter, WorkState,
    WorkStore,
};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::altai::agent::runtime::{self, AgentRuntime};
use crate::modules::workspace::WorkspaceRegistry;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkItemDto {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub description: String,
    pub acceptance_criteria: String,
    pub kind: String,
    pub parent_work_id: Option<String>,
    pub state: String,
    pub assignee_ref: Option<String>,
    pub blocker: Option<String>,
    pub revision: i64,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

impl From<WorkItemRecord> for WorkItemDto {
    fn from(value: WorkItemRecord) -> Self {
        Self {
            id: value.id,
            project_id: value.project_id,
            title: value.title,
            description: value.description,
            acceptance_criteria: value.acceptance_criteria,
            kind: value.kind.as_str().to_string(),
            parent_work_id: value.parent_work_id,
            state: value.state.as_str().to_string(),
            assignee_ref: value.assignee_ref,
            blocker: value.blocker,
            revision: value.revision,
            created_at_ms: value.created_at_ms,
            updated_at_ms: value.updated_at_ms,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkAttemptDto {
    pub id: String,
    pub work_id: String,
    pub number: i64,
    pub role: String,
    pub phase: String,
    pub chat_id: Option<String>,
    pub session_id: Option<String>,
    pub run_id: Option<String>,
    pub input_json: Option<String>,
    pub result_json: Option<String>,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

impl From<AttemptRecord> for WorkAttemptDto {
    fn from(value: AttemptRecord) -> Self {
        Self {
            id: value.id,
            work_id: value.work_id,
            number: value.number,
            role: value.role,
            phase: value.phase.as_str().to_string(),
            chat_id: value.chat_id,
            session_id: value.session_id,
            run_id: value.run_id,
            input_json: value.input_json,
            result_json: value.result_json,
            created_at_ms: value.created_at_ms,
            updated_at_ms: value.updated_at_ms,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkStartResultDto {
    pub work: WorkItemDto,
    pub attempt: WorkAttemptDto,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkReconcileResultDto {
    pub changed_work_ids: Vec<String>,
}

impl From<WorkAttemptStart> for WorkStartResultDto {
    fn from(value: WorkAttemptStart) -> Self {
        Self {
            work: value.work.into(),
            attempt: value.attempt.into(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkInboxItemDto {
    pub id: String,
    pub work_id: String,
    pub kind: String,
    pub title: String,
    pub why: String,
    pub created_at_ms: u64,
    pub attempt_id: Option<String>,
    pub chat_id: Option<String>,
    pub run_id: Option<String>,
}

impl From<WorkInboxRecord> for WorkInboxItemDto {
    fn from(value: WorkInboxRecord) -> Self {
        Self {
            id: value.id,
            work_id: value.work_id,
            kind: value.kind.as_str().to_string(),
            title: value.title,
            why: value.why,
            created_at_ms: value.created_at_ms,
            attempt_id: value.attempt_id,
            chat_id: value.chat_id,
            run_id: value.run_id,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkCreateArgs {
    pub workspace_path: String,
    pub title: String,
    pub description: Option<String>,
    pub acceptance_criteria: Option<String>,
    pub assignee_ref: Option<String>,
    pub kind: Option<String>,
    pub parent_work_id: Option<String>,
}

fn authorized_workspace(
    workspace_path: &str,
    registry: &WorkspaceRegistry,
) -> Result<String, String> {
    let raw = workspace_path.trim();
    if raw.is_empty() {
        return Err("workspacePath is required".into());
    }
    let canonical = registry
        .canonicalize_cached(raw)
        .map_err(|error| format!("Workspace is not accessible: {error}"))?;
    if !canonical.is_dir() || !registry.is_authorized(&canonical) {
        return Err("Workspace is not authorized.".into());
    }
    Ok(canonical.to_string_lossy().replace('\\', "/"))
}

fn open_store(workspace_path: &str) -> Result<(String, WorkStore), String> {
    let paths = resolve_workspace_from(Some(Path::new(workspace_path)), Path::new(workspace_path))
        .map_err(|error| error.to_string())?;
    let project_id = paths
        .root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("workspace")
        .to_string();
    let store = WorkStore::open(&paths.work_db()).map_err(|error| error.to_string())?;
    store
        .ensure_project(
            &project_id,
            &project_id,
            &paths.root.to_string_lossy(),
        )
        .map_err(|error| error.to_string())?;
    Ok((project_id, store))
}

fn parse_filter(raw: Option<&str>) -> Result<WorkListFilter, String> {
    match raw.unwrap_or("my_active") {
        "my_active" | "my-active" | "active" => Ok(WorkListFilter::MyActive),
        "review" => Ok(WorkListFilter::Review),
        "backlog" => Ok(WorkListFilter::Backlog),
        "done" => Ok(WorkListFilter::Done),
        other => Err(format!("unknown work filter: {other}")),
    }
}

fn parse_state(raw: &str) -> Result<WorkState, String> {
    WorkState::parse(raw).ok_or_else(|| format!("unknown work state: {raw}"))
}

fn parse_terminal_phase(raw: &str) -> Result<AttemptPhase, String> {
    match raw.trim() {
        "succeeded" => Ok(AttemptPhase::Succeeded),
        "failed" => Ok(AttemptPhase::Failed),
        "cancelled" => Ok(AttemptPhase::Cancelled),
        other => Err(format!("unknown terminal attempt phase: {other}")),
    }
}

#[tauri::command]
pub fn work_create(
    registry: State<'_, WorkspaceRegistry>,
    args: WorkCreateArgs,
) -> Result<WorkItemDto, String> {
    let workspace = authorized_workspace(&args.workspace_path, &registry)?;
    let (project_id, store) = open_store(&workspace)?;
    let kind = args
        .kind
        .as_deref()
        .map(WorkItemKind::parse)
        .unwrap_or(Some(WorkItemKind::Task))
        .ok_or_else(|| "kind must be task, ticket, or campaign".to_string())?;
    let created = store
        .create_work_item(
            CreateWorkInput {
                project_id,
                title: args.title,
                description: args.description.unwrap_or_default(),
                acceptance_criteria: args.acceptance_criteria.unwrap_or_default(),
                assignee_ref: args.assignee_ref,
            },
            kind,
            args.parent_work_id,
        )
        .map_err(|error| error.to_string())?;
    Ok(created.into())
}

#[tauri::command]
pub fn work_list(
    registry: State<'_, WorkspaceRegistry>,
    workspace_path: String,
    filter: Option<String>,
) -> Result<Vec<WorkItemDto>, String> {
    let workspace = authorized_workspace(&workspace_path, &registry)?;
    let (project_id, store) = open_store(&workspace)?;
    let listed = store
        .list_work(&project_id, parse_filter(filter.as_deref())?)
        .map_err(|error| error.to_string())?;
    Ok(listed.into_iter().map(WorkItemDto::from).collect())
}

#[tauri::command]
pub fn work_children(
    registry: State<'_, WorkspaceRegistry>,
    workspace_path: String,
    parent_work_id: String,
) -> Result<Vec<WorkItemDto>, String> {
    let workspace = authorized_workspace(&workspace_path, &registry)?;
    let (_project_id, store) = open_store(&workspace)?;
    let children = store
        .list_child_work(&parent_work_id)
        .map_err(|error| error.to_string())?;
    Ok(children.into_iter().map(WorkItemDto::from).collect())
}

#[tauri::command]
pub fn work_inbox_list(
    registry: State<'_, WorkspaceRegistry>,
    workspace_path: String,
) -> Result<Vec<WorkInboxItemDto>, String> {
    let workspace = authorized_workspace(&workspace_path, &registry)?;
    let (project_id, store) = open_store(&workspace)?;
    let listed = store
        .list_work_inbox(&project_id)
        .map_err(|error| error.to_string())?;
    Ok(listed.into_iter().map(WorkInboxItemDto::from).collect())
}

#[tauri::command]
pub fn work_get(
    registry: State<'_, WorkspaceRegistry>,
    workspace_path: String,
    work_id: String,
) -> Result<Option<WorkItemDto>, String> {
    let workspace = authorized_workspace(&workspace_path, &registry)?;
    let (_project_id, store) = open_store(&workspace)?;
    Ok(store
        .get_work(work_id.trim())
        .map_err(|error| error.to_string())?
        .map(WorkItemDto::from))
}

#[tauri::command]
pub fn work_transition(
    registry: State<'_, WorkspaceRegistry>,
    workspace_path: String,
    work_id: String,
    expected_revision: i64,
    next_state: String,
) -> Result<WorkItemDto, String> {
    let workspace = authorized_workspace(&workspace_path, &registry)?;
    let (_project_id, store) = open_store(&workspace)?;
    let updated = store
        .transition(
            work_id.trim(),
            expected_revision,
            parse_state(next_state.trim())?,
        )
        .map_err(|error| error.to_string())?;
    Ok(updated.into())
}

#[tauri::command]
pub fn work_start(
    registry: State<'_, WorkspaceRegistry>,
    workspace_path: String,
    work_id: String,
    expected_revision: i64,
) -> Result<WorkItemDto, String> {
    let workspace = authorized_workspace(&workspace_path, &registry)?;
    let (_project_id, store) = open_store(&workspace)?;
    let updated = store
        .start_attempt(work_id.trim(), expected_revision)
        .map_err(|error| error.to_string())?;
    Ok(updated.into())
}

async fn reconcile_store<'a>(
    agent_runtime: &'a AgentRuntime,
    workspace: &str,
    store: &WorkStore,
) -> Result<(Vec<String>, runtime::WorkRecoveryLease<'a>), String> {
    let lease = runtime::work_recovery_lease(agent_runtime, workspace).await?;
    let mode = if lease.first_recovery_pass {
        AttemptReconcileMode::RestartRecovery
    } else {
        AttemptReconcileMode::Live
    };
    let changed_work_ids = store
        .reconcile_attempts_from_journal(&lease.event_journal, mode)
        .map_err(|error| error.to_string())?;
    Ok((changed_work_ids, lease))
}

#[tauri::command]
pub async fn work_start_attempt(
    agent_runtime: State<'_, AgentRuntime>,
    registry: State<'_, WorkspaceRegistry>,
    workspace_path: String,
    work_id: String,
    expected_revision: i64,
    chat_id: String,
    session_id: Option<String>,
) -> Result<WorkStartResultDto, String> {
    let workspace = authorized_workspace(&workspace_path, &registry)?;
    let (_project_id, store) = open_store(&workspace)?;
    // A direct Start may be the first Work command in this host process. Run
    // the one restart-recovery pass before creating current-process state so
    // that the new cold dispatch can never be mistaken for an inherited orphan.
    let (_changed_work_ids, recovery_lease) =
        reconcile_store(&agent_runtime, &workspace, &store).await?;
    let chat_id = chat_id.trim();
    if chat_id.is_empty() {
        return Err("chatId is required before starting a Desktop Attempt".into());
    }
    let started = store
        .start_attempt_with_dispatch(
            work_id.trim(),
            expected_revision,
            Some(chat_id),
            session_id.as_deref(),
        )
        .map_err(|error| error.to_string())?;
    let result = started.into();
    recovery_lease.commit();
    Ok(result)
}

#[tauri::command]
pub async fn work_attempt_reconcile(
    agent_runtime: State<'_, AgentRuntime>,
    registry: State<'_, WorkspaceRegistry>,
    workspace_path: String,
) -> Result<WorkReconcileResultDto, String> {
    let workspace = authorized_workspace(&workspace_path, &registry)?;
    let (_project_id, store) = open_store(&workspace)?;
    let (changed_work_ids, recovery_lease) =
        reconcile_store(&agent_runtime, &workspace, &store).await?;
    recovery_lease.commit();
    Ok(WorkReconcileResultDto { changed_work_ids })
}

#[tauri::command]
pub fn work_attempts(
    registry: State<'_, WorkspaceRegistry>,
    workspace_path: String,
    work_id: String,
) -> Result<Vec<WorkAttemptDto>, String> {
    let workspace = authorized_workspace(&workspace_path, &registry)?;
    let (_project_id, store) = open_store(&workspace)?;
    let attempts = store
        .list_attempts(work_id.trim())
        .map_err(|error| error.to_string())?;
    Ok(attempts.into_iter().map(WorkAttemptDto::from).collect())
}

#[tauri::command]
pub fn work_attempt_bind(
    registry: State<'_, WorkspaceRegistry>,
    workspace_path: String,
    attempt_id: String,
    chat_id: String,
    session_id: Option<String>,
    run_id: String,
) -> Result<WorkAttemptDto, String> {
    let workspace = authorized_workspace(&workspace_path, &registry)?;
    let (_project_id, store) = open_store(&workspace)?;
    let attempt = store
        .bind_attempt_run(
            attempt_id.trim(),
            chat_id.trim(),
            session_id.as_deref(),
            run_id.trim(),
        )
        .map_err(|error| error.to_string())?;
    Ok(attempt.into())
}

#[tauri::command]
pub fn work_attempt_finish(
    registry: State<'_, WorkspaceRegistry>,
    workspace_path: String,
    attempt_id: Option<String>,
    run_id: Option<String>,
    phase: String,
    result_json: Option<String>,
) -> Result<Option<WorkItemDto>, String> {
    let workspace = authorized_workspace(&workspace_path, &registry)?;
    let (_project_id, store) = open_store(&workspace)?;
    let phase = parse_terminal_phase(&phase)?;
    let result_json = result_json.as_deref().unwrap_or("{}");
    let updated = match (
        attempt_id
            .as_deref()
            .map(str::trim)
            .filter(|id| !id.is_empty()),
        run_id.as_deref().map(str::trim).filter(|id| !id.is_empty()),
    ) {
        (Some(id), None) => Some(
            store
                .finish_attempt_by_id(id, phase, result_json)
                .map_err(|error| error.to_string())?,
        ),
        (None, Some(id)) => store
            .finish_attempt_by_run(id, phase, result_json)
            .map_err(|error| error.to_string())?,
        _ => return Err("exactly one attemptId or runId is required".into()),
    };
    Ok(updated.map(WorkItemDto::from))
}

#[tauri::command]
pub fn work_ready_for_review(
    registry: State<'_, WorkspaceRegistry>,
    workspace_path: String,
    work_id: String,
    expected_revision: i64,
) -> Result<WorkItemDto, String> {
    let workspace = authorized_workspace(&workspace_path, &registry)?;
    let (_project_id, store) = open_store(&workspace)?;
    let updated = store
        .mark_attempt_ready_for_review(work_id.trim(), expected_revision)
        .map_err(|error| error.to_string())?;
    Ok(updated.into())
}

#[tauri::command]
pub fn work_review(
    registry: State<'_, WorkspaceRegistry>,
    workspace_path: String,
    work_id: String,
    expected_revision: i64,
    accept: bool,
    guidance: Option<String>,
) -> Result<WorkItemDto, String> {
    let workspace = authorized_workspace(&workspace_path, &registry)?;
    let (_project_id, store) = open_store(&workspace)?;
    let updated = store
        .human_review(
            work_id.trim(),
            expected_revision,
            accept,
            guidance.as_deref().unwrap_or(""),
        )
        .map_err(|error| error.to_string())?;
    Ok(updated.into())
}

#[cfg(test)]
mod tests {
    use super::WorkInboxItemDto;
    use altai_core::{WorkInboxKind, WorkInboxRecord};

    #[test]
    fn work_inbox_dto_is_camel_case_and_keeps_nullable_source_refs() {
        let dto = WorkInboxItemDto::from(WorkInboxRecord {
            id: "review_required:work_1".into(),
            work_id: "work_1".into(),
            kind: WorkInboxKind::ReviewRequired,
            title: "Review Work".into(),
            why: "Attempt finished".into(),
            created_at_ms: 42,
            attempt_id: Some("attempt_1".into()),
            chat_id: None,
            run_id: None,
        });
        let value = serde_json::to_value(dto).expect("serialize Work Inbox DTO");
        assert_eq!(value["workId"], "work_1");
        assert_eq!(value["createdAtMs"], 42);
        assert_eq!(value["attemptId"], "attempt_1");
        assert!(value["chatId"].is_null());
        assert!(value["runId"].is_null());
        assert!(value.get("work_id").is_none());
        assert!(value.get("created_at_ms").is_none());
    }
}
