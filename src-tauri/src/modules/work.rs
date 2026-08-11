//! Work OS Milestone 1 — durable Work IPC over the existing host.
//!
//! Wraps `altai_core::WorkStore` at `WorkspacePaths::work_db()`. Authoritative
//! lifecycle lives here; IsanAgent remains execution-only (ADR 0003).

use std::path::Path;

use altai_core::{
    resolve_workspace_from, CreateWorkInput, WorkItemRecord, WorkListFilter, WorkState, WorkStore,
};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::modules::workspace::WorkspaceRegistry;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkItemDto {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub description: String,
    pub acceptance_criteria: String,
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
            state: value.state.as_str().to_string(),
            assignee_ref: value.assignee_ref,
            blocker: value.blocker,
            revision: value.revision,
            created_at_ms: value.created_at_ms,
            updated_at_ms: value.updated_at_ms,
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

#[tauri::command]
pub fn work_create(
    registry: State<'_, WorkspaceRegistry>,
    args: WorkCreateArgs,
) -> Result<WorkItemDto, String> {
    let workspace = authorized_workspace(&args.workspace_path, &registry)?;
    let (project_id, store) = open_store(&workspace)?;
    let created = store
        .create_work(CreateWorkInput {
            project_id,
            title: args.title,
            description: args.description.unwrap_or_default(),
            acceptance_criteria: args.acceptance_criteria.unwrap_or_default(),
            assignee_ref: args.assignee_ref,
        })
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
