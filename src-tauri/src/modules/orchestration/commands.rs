//! Tauri command wiring for orchestration modules.
//!
//! Thin wrappers that expose the orchestration backend to the frontend.
//! Filesystem access is restricted to authorized workspaces and mutable
//! command state is isolated by canonical workspace path.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::Timelike;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, State};

use crate::modules::secrets::{self, SecretsState};
use crate::modules::workspace::{resolve_path, WorkspaceEnv, WorkspaceRegistry};

use super::checks::{evaluate_gate, evaluate_review, CheckResult, ReviewFinding};
use super::context::{build_context_pack, ContextConfig};
use super::eval_lab::export_support_bundle;
use super::gardening::{run_gardening, GardeningConfig};
use super::integration::{detect_overlaps, ChildDiff};
use super::ledger::{CreateDecisionRequest, DecisionEntry, OrchestrationLedger, SCHEMA_VERSION};
use super::notifications::{NotificationDispatcher, NotificationTrigger};
use super::plans::{parse_plan, DecisionLog};
use super::profiles::{select_profile, AgentProfileDef, ProfileRegistry, ProfileScope};
use super::quality::compute_quality_metrics;
use super::readiness::scan as readiness_scan;
use super::session_analysis::{propose_playbooks, AnalysisConfig};
use super::task_graph::TaskGraph;
use super::team::{
    detect_conflicts, AgentMessage, FileConflict, FileOwnership, Mailbox, TaskHierarchy,
};
use super::usage_wiring::{UsageEvent, UsageTracker};
use super::workflow::PermissionMode;
use super::workflow_v2::{BudgetsConfig, Reasoning};

const MAX_CONTEXT_BUDGET_BYTES: usize = 1024 * 1024;
const MAX_COLLECTION_ITEMS: usize = 10_000;
const MAX_TASK_ID_CHARS: usize = 512;
const MAX_NOTIFICATION_TEXT_CHARS: usize = 16 * 1024;
const MAX_MAILBOX_PAYLOAD_BYTES: usize = 256 * 1024;
const ORCHESTRATION_CREDENTIAL_SERVICE: &str = "dev.altai.orchestration";
const MAX_COMMAND_WORKSPACES: usize = 256;
const MAX_PENDING_NOTIFICATIONS: usize = 1_000;
const MAX_GRAPH_NODES: usize = 10_000;
const MAX_PROFILES_PER_WORKSPACE: usize = 256;

// ---------------------------------------------------------------------------
// Shared state for command-bound modules
// ---------------------------------------------------------------------------

#[derive(Default)]
struct WorkspaceCommandState {
    profiles: ProfileRegistry,
    hierarchy: TaskHierarchy,
    mailbox: Mailbox,
    notifications: NotificationDispatcher,
    usage: UsageTracker,
    task_graph: TaskGraph,
}

/// Shared command state, partitioned by canonical workspace path.
#[derive(Default)]
pub struct OrchestrationCommandState {
    workspaces: Mutex<HashMap<String, WorkspaceCommandState>>,
}

impl OrchestrationCommandState {
    fn with_workspace<R>(
        &self,
        workspace_key: &str,
        operation: impl FnOnce(&WorkspaceCommandState) -> R,
    ) -> Result<R, String> {
        let workspaces = self
            .workspaces
            .lock()
            .map_err(|_| "Orchestration command state is unavailable.".to_string())?;
        let empty = WorkspaceCommandState::default();
        Ok(operation(workspaces.get(workspace_key).unwrap_or(&empty)))
    }

    fn with_workspace_mut<R>(
        &self,
        workspace_key: String,
        operation: impl FnOnce(&mut WorkspaceCommandState) -> R,
    ) -> Result<R, String> {
        let mut workspaces = self
            .workspaces
            .lock()
            .map_err(|_| "Orchestration command state is unavailable.".to_string())?;
        if !workspaces.contains_key(&workspace_key) && workspaces.len() >= MAX_COMMAND_WORKSPACES {
            return Err("Too many orchestration workspaces are active.".into());
        }
        Ok(operation(workspaces.entry(workspace_key).or_default()))
    }
}

fn authorized_workspace(
    registry: &WorkspaceRegistry,
    workspace_key: &str,
    workspace: &WorkspaceEnv,
) -> Result<PathBuf, String> {
    let trimmed = workspace_key.trim();
    if trimmed.is_empty() {
        return Err("A workspace path is required.".into());
    }
    let resolved = resolve_path(trimmed, workspace);
    let canonical = registry
        .canonicalize_cached(&resolved)
        .map_err(|error| format!("Could not access the workspace: {error}"))?;
    if !canonical.is_dir() || !registry.is_authorized(&canonical) {
        return Err("The path is outside the authorized workspace.".into());
    }
    Ok(canonical)
}

fn workspace_state_key(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn authorized_ledger_path(
    registry: &WorkspaceRegistry,
    workspace_root: &Path,
    requested_path: &str,
    workspace: &WorkspaceEnv,
    must_exist: bool,
) -> Result<PathBuf, String> {
    let trimmed = requested_path.trim();
    if trimmed.is_empty() {
        return Err("A ledger path is required.".into());
    }
    let resolved = resolve_path(trimmed, workspace);
    let path = if resolved.exists() {
        registry
            .canonicalize_cached(&resolved)
            .map_err(|error| format!("Could not access the ledger: {error}"))?
    } else {
        if must_exist {
            return Err("The orchestration ledger does not exist.".into());
        }
        let file_name = resolved
            .file_name()
            .filter(|name| !name.is_empty())
            .ok_or_else(|| "The ledger path must name a file.".to_string())?;
        let parent = resolved
            .parent()
            .ok_or_else(|| "The ledger path must have a parent directory.".to_string())?;
        let canonical_parent = registry
            .canonicalize_cached(parent)
            .map_err(|error| format!("Could not access the ledger directory: {error}"))?;
        canonical_parent.join(file_name)
    };
    if !path.starts_with(workspace_root) || !registry.is_authorized(&path) {
        return Err("The ledger is outside the selected workspace.".into());
    }
    if path.exists() && !path.is_file() {
        return Err("The ledger path is not a file.".into());
    }
    Ok(path)
}

fn validate_task_id(value: &str) -> Result<(), String> {
    let length = value.chars().count();
    if value.trim().is_empty() || length > MAX_TASK_ID_CHARS {
        return Err("Task identifiers must contain 1 to 512 characters.".into());
    }
    Ok(())
}

fn validate_collection_len(length: usize, label: &str) -> Result<(), String> {
    if length > MAX_COLLECTION_ITEMS {
        Err(format!(
            "{label} exceeds the {MAX_COLLECTION_ITEMS}-item limit."
        ))
    } else {
        Ok(())
    }
}

fn credential_account(workspace_key: &str, source: &str, name: &str) -> Result<String, String> {
    let source = source.trim();
    let name = name.trim();
    if source.is_empty()
        || name.is_empty()
        || source.chars().count() > 128
        || name.chars().count() > 128
        || source.contains(':')
        || name.contains(':')
        || source.chars().any(char::is_control)
        || name.chars().any(char::is_control)
    {
        return Err("Credential source and name are invalid.".into());
    }
    let workspace_digest = hex::encode(Sha256::digest(workspace_key.as_bytes()));
    Ok(format!("{workspace_digest}:{source}:{name}"))
}

fn validate_webview_profile_scope(scope: ProfileScope) -> Result<(), String> {
    if scope == ProfileScope::Managed {
        Err("Managed profiles cannot be registered from the webview.".into())
    } else {
        Ok(())
    }
}

fn notification_clock() -> (u64, u8) {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let now_hour = chrono::Local::now().hour() as u8;
    (now_ms, now_hour)
}

// ---------------------------------------------------------------------------
// Quality dashboard
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn orchestration_quality_metrics(
    db_path: String,
    workspace_key: String,
    stale_threshold_ms: u64,
    workspace: Option<WorkspaceEnv>,
    registry: State<'_, WorkspaceRegistry>,
) -> Result<super::quality::QualityMetrics, String> {
    let workspace = WorkspaceEnv::from_option(workspace);
    let root = authorized_workspace(&registry, &workspace_key, &workspace)?;
    let db_path = authorized_ledger_path(&registry, &root, &db_path, &workspace, true)?;
    let workspace_key = workspace_state_key(&root);
    tauri::async_runtime::spawn_blocking(move || {
        let ledger = OrchestrationLedger::open(db_path)
            .map_err(|error| format!("Failed to open ledger: {error}"))?;
        compute_quality_metrics(&ledger, &workspace_key, stale_threshold_ms)
            .map_err(|error| format!("Failed to compute metrics: {error}"))
    })
    .await
    .map_err(|error| format!("Quality metrics worker failed: {error}"))?
}

// ---------------------------------------------------------------------------
// Readiness scan
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn orchestration_readiness_scan(
    repo_path: String,
    workspace: Option<WorkspaceEnv>,
    registry: State<'_, WorkspaceRegistry>,
) -> Result<super::readiness::ReadinessReport, String> {
    let workspace = WorkspaceEnv::from_option(workspace);
    let repo_path = authorized_workspace(&registry, &repo_path, &workspace)?;
    tauri::async_runtime::spawn_blocking(move || readiness_scan(&repo_path))
        .await
        .map_err(|error| format!("Readiness worker failed: {error}"))
}

// ---------------------------------------------------------------------------
// Context pack
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn orchestration_context_pack(
    repo_path: String,
    task_description: String,
    budget_bytes: Option<usize>,
    workspace: Option<WorkspaceEnv>,
    registry: State<'_, WorkspaceRegistry>,
) -> Result<super::context::ContextPack, String> {
    if task_description.chars().count() > 64 * 1024 {
        return Err("Task description exceeds the 64 KiB limit.".into());
    }
    let budget_bytes = budget_bytes.unwrap_or(48 * 1024);
    if budget_bytes > MAX_CONTEXT_BUDGET_BYTES {
        return Err("Context budget exceeds the 1 MiB limit.".into());
    }
    let workspace = WorkspaceEnv::from_option(workspace);
    let repo_path = authorized_workspace(&registry, &repo_path, &workspace)?;
    let config = ContextConfig {
        budget_bytes,
        ..ContextConfig::default()
    };
    tauri::async_runtime::spawn_blocking(move || {
        build_context_pack(&repo_path, &task_description, &config)
    })
    .await
    .map_err(|error| format!("Context pack worker failed: {error}"))
}

// ---------------------------------------------------------------------------
// Execution plans
// ---------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanParseResult {
    pub plan: super::plans::ExecutionPlan,
}

#[tauri::command]
pub fn orchestration_plan_parse(
    source_path: String,
    revision: String,
    content: String,
) -> Result<PlanParseResult, String> {
    if source_path.chars().count() > 4_096 || revision.chars().count() > 512 {
        return Err("Plan source or revision metadata is too long.".into());
    }
    if content.len() > 1024 * 1024 {
        return Err("Plan content exceeds the 1 MiB limit.".into());
    }
    Ok(PlanParseResult {
        plan: parse_plan(&source_path, &revision, &content),
    })
}

#[tauri::command]
pub async fn orchestration_decision_record(
    db_path: String,
    workspace_key: String,
    request: CreateDecisionRequest,
    workspace: Option<WorkspaceEnv>,
    registry: State<'_, WorkspaceRegistry>,
) -> Result<DecisionEntry, String> {
    let workspace = WorkspaceEnv::from_option(workspace);
    let root = authorized_workspace(&registry, &workspace_key, &workspace)?;
    let db_path = authorized_ledger_path(&registry, &root, &db_path, &workspace, false)?;
    tauri::async_runtime::spawn_blocking(move || {
        let ledger = OrchestrationLedger::open(db_path)
            .map_err(|error| format!("Failed to open ledger: {error}"))?;
        DecisionLog::new(&ledger)
            .record(&request)
            .map_err(|error| format!("Failed to record decision: {error}"))
    })
    .await
    .map_err(|error| format!("Decision worker failed: {error}"))?
}

#[tauri::command]
pub async fn orchestration_decisions_for_task(
    db_path: String,
    workspace_key: String,
    task_id: String,
    workspace: Option<WorkspaceEnv>,
    registry: State<'_, WorkspaceRegistry>,
) -> Result<Vec<DecisionEntry>, String> {
    validate_task_id(&task_id)?;
    let workspace = WorkspaceEnv::from_option(workspace);
    let root = authorized_workspace(&registry, &workspace_key, &workspace)?;
    let db_path = authorized_ledger_path(&registry, &root, &db_path, &workspace, true)?;
    tauri::async_runtime::spawn_blocking(move || {
        let ledger = OrchestrationLedger::open(db_path)
            .map_err(|error| format!("Failed to open ledger: {error}"))?;
        DecisionLog::new(&ledger)
            .for_task(&task_id)
            .map_err(|error| format!("Failed to fetch decisions: {error}"))
    })
    .await
    .map_err(|error| format!("Decision query worker failed: {error}"))?
}

// ---------------------------------------------------------------------------
// Task graph
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddDependencyRequest {
    pub task_id: String,
    pub depends_on: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddDependencyResult {
    pub success: bool,
    pub error: Option<String>,
}

#[tauri::command]
pub fn orchestration_graph_add_dependency(
    state: State<'_, OrchestrationCommandState>,
    registry: State<'_, WorkspaceRegistry>,
    workspace_key: String,
    workspace: Option<WorkspaceEnv>,
    request: AddDependencyRequest,
) -> Result<AddDependencyResult, String> {
    validate_task_id(&request.task_id)?;
    validate_task_id(&request.depends_on)?;
    let workspace = WorkspaceEnv::from_option(workspace);
    let root = authorized_workspace(&registry, &workspace_key, &workspace)?;
    state.with_workspace_mut(workspace_state_key(&root), |workspace_state| {
        let graph = &mut workspace_state.task_graph;
        let new_nodes = usize::from(!graph.nodes.contains(&request.task_id))
            + usize::from(!graph.nodes.contains(&request.depends_on));
        if graph.nodes.len().saturating_add(new_nodes) > MAX_GRAPH_NODES {
            return Err("Task graph exceeds the 10,000-node limit.".to_string());
        }
        Ok(
            match graph.add_dependency(&request.task_id, &request.depends_on) {
                Ok(()) => AddDependencyResult {
                    success: true,
                    error: None,
                },
                Err(error) => AddDependencyResult {
                    success: false,
                    error: Some(error.to_string()),
                },
            },
        )
    })?
}

#[tauri::command]
pub fn orchestration_graph_eligible(
    state: State<'_, OrchestrationCommandState>,
    registry: State<'_, WorkspaceRegistry>,
    workspace_key: String,
    workspace: Option<WorkspaceEnv>,
    completed: Vec<String>,
) -> Result<Vec<String>, String> {
    validate_collection_len(completed.len(), "Completed task list")?;
    for task_id in &completed {
        validate_task_id(task_id)?;
    }
    let workspace = WorkspaceEnv::from_option(workspace);
    let root = authorized_workspace(&registry, &workspace_key, &workspace)?;
    let completed_set: HashSet<String> = completed.into_iter().collect();
    state.with_workspace(&workspace_state_key(&root), |workspace_state| {
        let mut eligible = workspace_state.task_graph.eligible_tasks(&completed_set);
        eligible.sort();
        eligible
    })
}

#[tauri::command]
pub fn orchestration_graph_blocked_reason(
    state: State<'_, OrchestrationCommandState>,
    registry: State<'_, WorkspaceRegistry>,
    workspace_key: String,
    workspace: Option<WorkspaceEnv>,
    task_id: String,
    completed: Vec<String>,
) -> Result<Option<Vec<String>>, String> {
    validate_task_id(&task_id)?;
    validate_collection_len(completed.len(), "Completed task list")?;
    for completed_id in &completed {
        validate_task_id(completed_id)?;
    }
    let workspace = WorkspaceEnv::from_option(workspace);
    let root = authorized_workspace(&registry, &workspace_key, &workspace)?;
    let completed_set: HashSet<String> = completed.into_iter().collect();
    state.with_workspace(&workspace_state_key(&root), |workspace_state| {
        workspace_state
            .task_graph
            .blocked_reason(&task_id, &completed_set)
            .map(|mut blocked| {
                blocked.sort();
                blocked
            })
    })
}

#[tauri::command]
pub fn orchestration_graph_topological_order(
    state: State<'_, OrchestrationCommandState>,
    registry: State<'_, WorkspaceRegistry>,
    workspace_key: String,
    workspace: Option<WorkspaceEnv>,
) -> Result<Vec<String>, String> {
    let workspace = WorkspaceEnv::from_option(workspace);
    let root = authorized_workspace(&registry, &workspace_key, &workspace)?;
    state
        .with_workspace(&workspace_state_key(&root), |workspace_state| {
            workspace_state.task_graph.topological_order()
        })?
        .map_err(|error| error.to_string())
}

// ---------------------------------------------------------------------------
// Profiles
// ---------------------------------------------------------------------------

#[derive(Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentProfileInput {
    pub name: String,
    pub description: String,
    pub prompt: String,
    pub model_id: Option<String>,
    pub reasoning: Option<Reasoning>,
    pub permissions: Option<PermissionMode>,
    pub tools: Option<Vec<String>>,
    pub skills: Vec<String>,
    pub mcp_servers: Vec<String>,
    pub budgets: Option<BudgetInput>,
    pub file_scope: Vec<String>,
    pub auto_selectable: bool,
}

impl Default for AgentProfileInput {
    fn default() -> Self {
        let profile = AgentProfileDef::default();
        Self {
            name: profile.name,
            description: profile.description,
            prompt: profile.prompt,
            model_id: profile.model_id,
            reasoning: profile.reasoning,
            permissions: profile.permissions,
            tools: profile.tools,
            skills: profile.skills,
            mcp_servers: profile.mcp_servers,
            budgets: profile.budgets.map(BudgetInput::from),
            file_scope: profile.file_scope,
            auto_selectable: profile.auto_selectable,
        }
    }
}

impl From<AgentProfileInput> for AgentProfileDef {
    fn from(profile: AgentProfileInput) -> Self {
        Self {
            name: profile.name,
            description: profile.description,
            prompt: profile.prompt,
            model_id: profile.model_id,
            reasoning: profile.reasoning,
            permissions: profile.permissions,
            tools: profile.tools,
            skills: profile.skills,
            mcp_servers: profile.mcp_servers,
            budgets: profile.budgets.map(Into::into),
            file_scope: profile.file_scope,
            auto_selectable: profile.auto_selectable,
        }
    }
}

#[tauri::command]
pub fn orchestration_profile_register(
    state: State<'_, OrchestrationCommandState>,
    registry: State<'_, WorkspaceRegistry>,
    workspace_key: String,
    workspace: Option<WorkspaceEnv>,
    profile: AgentProfileInput,
    scope: ProfileScope,
) -> Result<(), String> {
    validate_webview_profile_scope(scope)?;
    validate_task_id(&profile.name)?;
    if serde_json::to_vec(&profile)
        .map_err(|error| format!("Agent profile is invalid: {error}"))?
        .len()
        > 1024 * 1024
    {
        return Err("Agent profile exceeds the 1 MiB limit.".into());
    }
    let workspace = WorkspaceEnv::from_option(workspace);
    let root = authorized_workspace(&registry, &workspace_key, &workspace)?;
    state.with_workspace_mut(workspace_state_key(&root), |workspace_state| {
        if !workspace_state
            .profiles
            .profiles
            .contains_key(&profile.name)
            && workspace_state.profiles.profiles.len() >= MAX_PROFILES_PER_WORKSPACE
        {
            return Err("Too many agent profiles are registered.".to_string());
        }
        workspace_state.profiles.register(profile.into(), scope);
        Ok(())
    })?
}

#[tauri::command]
pub fn orchestration_profile_resolve(
    state: State<'_, OrchestrationCommandState>,
    registry: State<'_, WorkspaceRegistry>,
    workspace_key: String,
    workspace: Option<WorkspaceEnv>,
    name: String,
) -> Result<Option<super::profiles::EffectiveProfile>, String> {
    validate_task_id(&name)?;
    let workspace = WorkspaceEnv::from_option(workspace);
    let root = authorized_workspace(&registry, &workspace_key, &workspace)?;
    state.with_workspace(&workspace_state_key(&root), |workspace_state| {
        workspace_state.profiles.resolve(&name)
    })
}

#[tauri::command]
pub fn orchestration_profile_select(
    state: State<'_, OrchestrationCommandState>,
    registry: State<'_, WorkspaceRegistry>,
    workspace_key: String,
    workspace: Option<WorkspaceEnv>,
    manual_choice: Option<String>,
    task_description: String,
    default_profile: String,
) -> Result<super::profiles::ProfileSelection, String> {
    validate_task_id(&default_profile)?;
    if let Some(choice) = manual_choice.as_deref() {
        validate_task_id(choice)?;
    }
    if task_description.chars().count() > 64 * 1024 {
        return Err("Task description exceeds the 64 KiB limit.".into());
    }
    let workspace = WorkspaceEnv::from_option(workspace);
    let root = authorized_workspace(&registry, &workspace_key, &workspace)?;
    state.with_workspace(&workspace_state_key(&root), |workspace_state| {
        select_profile(
            &workspace_state.profiles,
            manual_choice.as_deref(),
            &task_description,
            &default_profile,
        )
    })
}

#[tauri::command]
pub fn orchestration_profile_names(
    state: State<'_, OrchestrationCommandState>,
    registry: State<'_, WorkspaceRegistry>,
    workspace_key: String,
    workspace: Option<WorkspaceEnv>,
) -> Result<Vec<String>, String> {
    let workspace = WorkspaceEnv::from_option(workspace);
    let root = authorized_workspace(&registry, &workspace_key, &workspace)?;
    state.with_workspace(&workspace_state_key(&root), |workspace_state| {
        workspace_state.profiles.names()
    })
}

// ---------------------------------------------------------------------------
// Team hierarchy + mailbox
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddChildRequest {
    pub parent_id: String,
    pub child_id: String,
}

#[tauri::command]
pub fn orchestration_hierarchy_add_child(
    state: State<'_, OrchestrationCommandState>,
    registry: State<'_, WorkspaceRegistry>,
    workspace_key: String,
    workspace: Option<WorkspaceEnv>,
    request: AddChildRequest,
) -> Result<(), String> {
    validate_task_id(&request.parent_id)?;
    validate_task_id(&request.child_id)?;
    let workspace = WorkspaceEnv::from_option(workspace);
    let root = authorized_workspace(&registry, &workspace_key, &workspace)?;
    state
        .with_workspace_mut(workspace_state_key(&root), |workspace_state| {
            workspace_state
                .hierarchy
                .add_child(&request.parent_id, &request.child_id)
        })?
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn orchestration_hierarchy_children(
    state: State<'_, OrchestrationCommandState>,
    registry: State<'_, WorkspaceRegistry>,
    workspace_key: String,
    workspace: Option<WorkspaceEnv>,
    task_id: String,
) -> Result<Vec<String>, String> {
    validate_task_id(&task_id)?;
    let workspace = WorkspaceEnv::from_option(workspace);
    let root = authorized_workspace(&registry, &workspace_key, &workspace)?;
    state.with_workspace(&workspace_state_key(&root), |workspace_state| {
        workspace_state.hierarchy.children_of(&task_id)
    })
}

#[tauri::command]
pub fn orchestration_hierarchy_descendants(
    state: State<'_, OrchestrationCommandState>,
    registry: State<'_, WorkspaceRegistry>,
    workspace_key: String,
    workspace: Option<WorkspaceEnv>,
    task_id: String,
) -> Result<Vec<String>, String> {
    validate_task_id(&task_id)?;
    let workspace = WorkspaceEnv::from_option(workspace);
    let root = authorized_workspace(&registry, &workspace_key, &workspace)?;
    state.with_workspace(&workspace_state_key(&root), |workspace_state| {
        let mut descendants: Vec<String> = workspace_state
            .hierarchy
            .descendants_of(&task_id)
            .into_iter()
            .collect();
        descendants.sort();
        descendants
    })
}

#[tauri::command]
pub fn orchestration_mailbox_post(
    state: State<'_, OrchestrationCommandState>,
    registry: State<'_, WorkspaceRegistry>,
    workspace_key: String,
    workspace: Option<WorkspaceEnv>,
    message: AgentMessage,
) -> Result<(), String> {
    validate_task_id(&message.id)?;
    validate_task_id(&message.from_task)?;
    validate_task_id(&message.to_task)?;
    let payload_size = serde_json::to_vec(&message.payload)
        .map_err(|error| format!("Mailbox payload is invalid: {error}"))?
        .len();
    if payload_size > MAX_MAILBOX_PAYLOAD_BYTES {
        return Err("Mailbox payload exceeds the 256 KiB limit.".into());
    }
    let workspace = WorkspaceEnv::from_option(workspace);
    let root = authorized_workspace(&registry, &workspace_key, &workspace)?;
    state
        .with_workspace_mut(workspace_state_key(&root), |workspace_state| {
            workspace_state.mailbox.post(message)
        })?
        .map_err(|error| format!("Mailbox full (capacity: {})", error.capacity))
}

#[tauri::command]
pub fn orchestration_mailbox_deliver(
    state: State<'_, OrchestrationCommandState>,
    registry: State<'_, WorkspaceRegistry>,
    workspace_key: String,
    workspace: Option<WorkspaceEnv>,
    recipient_task_id: String,
) -> Result<Option<AgentMessage>, String> {
    validate_task_id(&recipient_task_id)?;
    let workspace = WorkspaceEnv::from_option(workspace);
    let root = authorized_workspace(&registry, &workspace_key, &workspace)?;
    state.with_workspace_mut(workspace_state_key(&root), |workspace_state| {
        workspace_state.mailbox.deliver_for(&recipient_task_id)
    })
}

// ---------------------------------------------------------------------------
// File conflict detection
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn orchestration_detect_file_conflicts(
    ownerships: Vec<FileOwnership>,
) -> Result<Vec<FileConflict>, String> {
    if ownerships.len() > 128 {
        return Err("File ownership input exceeds the 128-item limit.".into());
    }
    let total_globs = ownerships
        .iter()
        .map(|ownership| ownership.file_globs.len())
        .fold(0usize, usize::saturating_add);
    if ownerships.iter().any(|ownership| {
        ownership.task_id.trim().is_empty()
            || ownership.task_id.chars().count() > MAX_TASK_ID_CHARS
            || ownership.file_globs.len() > 64
            || ownership.file_globs.iter().any(|glob| glob.len() > 512)
    }) {
        return Err("File ownership input contains invalid task IDs or globs.".into());
    }
    if total_globs > 512 {
        return Err("File ownership input exceeds the 512-glob limit.".into());
    }
    tauri::async_runtime::spawn_blocking(move || detect_conflicts(&ownerships))
        .await
        .map_err(|error| format!("File conflict worker failed: {error}"))
}

// ---------------------------------------------------------------------------
// Notifications
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationRequest {
    pub workspace_key: String,
    pub workspace: Option<WorkspaceEnv>,
    pub trigger: NotificationTrigger,
    pub task_id: String,
    pub attempt_id: Option<String>,
    pub title: String,
    pub body: String,
}

#[tauri::command]
pub fn orchestration_notify(
    state: State<'_, OrchestrationCommandState>,
    registry: State<'_, WorkspaceRegistry>,
    request: NotificationRequest,
) -> Result<super::notifications::DispatchResult, String> {
    validate_task_id(&request.task_id)?;
    if let Some(attempt_id) = request.attempt_id.as_deref() {
        validate_task_id(attempt_id)?;
    }
    if request.title.chars().count() > MAX_NOTIFICATION_TEXT_CHARS
        || request.body.chars().count() > MAX_NOTIFICATION_TEXT_CHARS
    {
        return Err("Notification text exceeds the 16 KiB limit.".into());
    }
    let workspace = WorkspaceEnv::from_option(request.workspace);
    let root = authorized_workspace(&registry, &request.workspace_key, &workspace)?;
    let (now_ms, now_hour) = notification_clock();
    state.with_workspace_mut(workspace_state_key(&root), |workspace_state| {
        if workspace_state.notifications.pending() >= MAX_PENDING_NOTIFICATIONS {
            return Err("The notification queue is full.".to_string());
        }
        Ok(workspace_state.notifications.dispatch(
            request.trigger,
            &request.task_id,
            request.attempt_id.as_deref(),
            &request.title,
            &request.body,
            now_ms,
            now_hour,
        ))
    })?
}

#[tauri::command]
pub fn orchestration_notifications_drain(
    state: State<'_, OrchestrationCommandState>,
    registry: State<'_, WorkspaceRegistry>,
    workspace_key: String,
    workspace: Option<WorkspaceEnv>,
) -> Result<Vec<super::notifications::Notification>, String> {
    let workspace = WorkspaceEnv::from_option(workspace);
    let root = authorized_workspace(&registry, &workspace_key, &workspace)?;
    state.with_workspace_mut(workspace_state_key(&root), |workspace_state| {
        workspace_state.notifications.drain()
    })
}

// ---------------------------------------------------------------------------
// Credentials
// ---------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialStatus {
    pub configured: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialStoreRequest {
    pub workspace_key: String,
    pub workspace: Option<WorkspaceEnv>,
    pub source: String,
    pub name: String,
    pub value: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialLocator {
    pub workspace_key: String,
    pub workspace: Option<WorkspaceEnv>,
    pub source: String,
    pub name: String,
}

#[tauri::command]
pub fn orchestration_credential_store(
    app: AppHandle,
    secrets_state: State<'_, SecretsState>,
    registry: State<'_, WorkspaceRegistry>,
    request: CredentialStoreRequest,
) -> Result<(), String> {
    if request.value.is_empty() || request.value.len() > 256 * 1024 {
        return Err("Credential value must contain 1 byte to 256 KiB.".into());
    }
    let workspace = WorkspaceEnv::from_option(request.workspace);
    let root = authorized_workspace(&registry, &request.workspace_key, &workspace)?;
    let account = credential_account(&workspace_state_key(&root), &request.source, &request.name)?;
    secrets::set_secret(
        &app,
        secrets_state.inner(),
        ORCHESTRATION_CREDENTIAL_SERVICE,
        &account,
        &request.value,
    )
}

#[tauri::command]
pub fn orchestration_credential_status(
    app: AppHandle,
    secrets_state: State<'_, SecretsState>,
    registry: State<'_, WorkspaceRegistry>,
    request: CredentialLocator,
) -> Result<CredentialStatus, String> {
    let workspace = WorkspaceEnv::from_option(request.workspace);
    let root = authorized_workspace(&registry, &request.workspace_key, &workspace)?;
    let account = credential_account(&workspace_state_key(&root), &request.source, &request.name)?;
    let configured = secrets::get_secret(
        &app,
        secrets_state.inner(),
        ORCHESTRATION_CREDENTIAL_SERVICE,
        &account,
    )?
    .is_some();
    Ok(CredentialStatus { configured })
}

#[tauri::command]
pub fn orchestration_credential_revoke(
    app: AppHandle,
    secrets_state: State<'_, SecretsState>,
    registry: State<'_, WorkspaceRegistry>,
    request: CredentialLocator,
) -> Result<(), String> {
    let workspace = WorkspaceEnv::from_option(request.workspace);
    let root = authorized_workspace(&registry, &request.workspace_key, &workspace)?;
    let account = credential_account(&workspace_state_key(&root), &request.source, &request.name)?;
    secrets::delete_secret(
        &app,
        secrets_state.inner(),
        ORCHESTRATION_CREDENTIAL_SERVICE,
        &account,
    )
}

// ---------------------------------------------------------------------------
// Checks + review
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn orchestration_check_gate(
    results: Vec<CheckResult>,
) -> Result<super::checks::GateResult, String> {
    validate_collection_len(results.len(), "Check result list")?;
    if serde_json::to_vec(&results)
        .map_err(|error| format!("Check results are invalid: {error}"))?
        .len()
        > 10 * 1024 * 1024
    {
        return Err("Check results exceed the 10 MiB limit.".into());
    }
    Ok(evaluate_gate(&results))
}

#[tauri::command]
pub fn orchestration_review_evaluate(
    findings: Vec<ReviewFinding>,
    allow_style_blocking: bool,
) -> Result<super::checks::ReviewResult, String> {
    validate_collection_len(findings.len(), "Review finding list")?;
    if serde_json::to_vec(&findings)
        .map_err(|error| format!("Review findings are invalid: {error}"))?
        .len()
        > 10 * 1024 * 1024
    {
        return Err("Review findings exceed the 10 MiB limit.".into());
    }
    Ok(evaluate_review(&findings, allow_style_blocking))
}

// ---------------------------------------------------------------------------
// Usage tracking
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct BudgetInput {
    pub max_task_minutes: Option<u64>,
    pub max_attempt_tokens: Option<u64>,
    pub max_task_cost_usd: Option<f64>,
    pub warn_at_percent: u8,
}

impl Default for BudgetInput {
    fn default() -> Self {
        Self::from(BudgetsConfig::default())
    }
}

impl From<BudgetsConfig> for BudgetInput {
    fn from(config: BudgetsConfig) -> Self {
        Self {
            max_task_minutes: config.max_task_minutes,
            max_attempt_tokens: config.max_attempt_tokens,
            max_task_cost_usd: config.max_task_cost_usd,
            warn_at_percent: config.warn_at_percent,
        }
    }
}

impl From<BudgetInput> for BudgetsConfig {
    fn from(config: BudgetInput) -> Self {
        Self {
            max_task_minutes: config.max_task_minutes,
            max_attempt_tokens: config.max_attempt_tokens,
            max_task_cost_usd: config.max_task_cost_usd,
            warn_at_percent: config.warn_at_percent,
        }
    }
}

#[tauri::command]
pub fn orchestration_usage_process(
    state: State<'_, OrchestrationCommandState>,
    registry: State<'_, WorkspaceRegistry>,
    workspace_key: String,
    workspace: Option<WorkspaceEnv>,
    event: UsageEvent,
    config: BudgetInput,
) -> Result<super::usage_wiring::UsageResult, String> {
    validate_task_id(&event.task_id)?;
    validate_task_id(&event.attempt_id)?;
    if config.warn_at_percent > 100
        || config
            .max_task_cost_usd
            .is_some_and(|cost| !cost.is_finite() || cost < 0.0)
    {
        return Err("Budget configuration is invalid.".into());
    }
    let config: BudgetsConfig = config.into();
    let workspace = WorkspaceEnv::from_option(workspace);
    let root = authorized_workspace(&registry, &workspace_key, &workspace)?;
    state.with_workspace_mut(workspace_state_key(&root), |workspace_state| {
        workspace_state.usage.process(&event, &config)
    })
}

#[tauri::command]
pub fn orchestration_usage_should_stop(
    state: State<'_, OrchestrationCommandState>,
    registry: State<'_, WorkspaceRegistry>,
    workspace_key: String,
    workspace: Option<WorkspaceEnv>,
    task_id: String,
) -> Result<bool, String> {
    validate_task_id(&task_id)?;
    let workspace = WorkspaceEnv::from_option(workspace);
    let root = authorized_workspace(&registry, &workspace_key, &workspace)?;
    state.with_workspace(&workspace_state_key(&root), |workspace_state| {
        super::usage_wiring::should_stop_task(&workspace_state.usage, &task_id)
    })
}

// ---------------------------------------------------------------------------
// Worker pool (stateless info)
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn orchestration_detect_overlaps(
    diffs: Vec<ChildDiff>,
) -> Result<Vec<super::integration::DiffOverlap>, String> {
    if diffs.len() > 256 {
        return Err("Diff input exceeds the 256-item limit.".into());
    }
    let total_files = diffs
        .iter()
        .map(|diff| {
            diff.modified_files
                .len()
                .saturating_add(diff.added_files.len())
                .saturating_add(diff.deleted_files.len())
        })
        .fold(0usize, usize::saturating_add);
    if total_files > 10_000 {
        return Err("Diff input exceeds the 10,000-file limit.".into());
    }
    if serde_json::to_vec(&diffs)
        .map_err(|error| format!("Diff input is invalid: {error}"))?
        .len()
        > 10 * 1024 * 1024
    {
        return Err("Diff input exceeds the 10 MiB limit.".into());
    }
    tauri::async_runtime::spawn_blocking(move || detect_overlaps(&diffs))
        .await
        .map_err(|error| format!("Diff overlap worker failed: {error}"))
}

// ---------------------------------------------------------------------------
// Gardening
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn orchestration_gardening_scan(
    repo_path: String,
    stale_doc_days: Option<u32>,
    now_ms: u64,
    workspace: Option<WorkspaceEnv>,
    registry: State<'_, WorkspaceRegistry>,
) -> Result<super::gardening::GardeningReport, String> {
    let stale_doc_days = stale_doc_days.unwrap_or(90);
    if !(1..=36_500).contains(&stale_doc_days) {
        return Err("Stale document age must be between 1 and 36,500 days.".into());
    }
    let workspace = WorkspaceEnv::from_option(workspace);
    let repo_path = authorized_workspace(&registry, &repo_path, &workspace)?;
    let config = GardeningConfig {
        stale_doc_days,
        ..GardeningConfig::default()
    };
    tauri::async_runtime::spawn_blocking(move || run_gardening(&repo_path, &config, now_ms))
        .await
        .map_err(|error| format!("Gardening worker failed: {error}"))
}

// ---------------------------------------------------------------------------
// Session analysis
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn orchestration_session_analyze(
    db_path: String,
    workspace_key: String,
    workspace: Option<WorkspaceEnv>,
    registry: State<'_, WorkspaceRegistry>,
) -> Result<Vec<super::session_analysis::AttemptAnalysis>, String> {
    let workspace = WorkspaceEnv::from_option(workspace);
    let root = authorized_workspace(&registry, &workspace_key, &workspace)?;
    let db_path = authorized_ledger_path(&registry, &root, &db_path, &workspace, true)?;
    let workspace_key = workspace_state_key(&root);
    tauri::async_runtime::spawn_blocking(move || {
        let ledger = OrchestrationLedger::open(db_path)
            .map_err(|error| format!("Failed to open ledger: {error}"))?;
        let tasks = ledger
            .tasks_for_workspace(&workspace_key)
            .map_err(|error| format!("Failed to fetch tasks: {error}"))?;
        let config = AnalysisConfig::default();
        let mut analyses = Vec::with_capacity(tasks.len());
        for task in &tasks {
            let analysis = super::session_analysis::analyze_task(&ledger, task, &config)
                .map_err(|error| format!("Analysis failed: {error}"))?;
            analyses.push(analysis);
        }
        Ok(analyses)
    })
    .await
    .map_err(|error| format!("Session analysis worker failed: {error}"))?
}

#[tauri::command]
pub fn orchestration_playbook_propose(
    analyses: Vec<super::session_analysis::AttemptAnalysis>,
) -> Result<Vec<super::session_analysis::PlaybookProposal>, String> {
    validate_collection_len(analyses.len(), "Session analysis list")?;
    if serde_json::to_vec(&analyses)
        .map_err(|error| format!("Session analyses are invalid: {error}"))?
        .len()
        > 10 * 1024 * 1024
    {
        return Err("Session analyses exceed the 10 MiB limit.".into());
    }
    Ok(propose_playbooks(&analyses))
}

// ---------------------------------------------------------------------------
// Eval lab / support bundle
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn orchestration_support_bundle(
    db_path: String,
    workspace_key: String,
    task_ids: Vec<String>,
    source: String,
    workspace: Option<WorkspaceEnv>,
    registry: State<'_, WorkspaceRegistry>,
) -> Result<super::eval_lab::SupportBundle, String> {
    validate_collection_len(task_ids.len(), "Support bundle task list")?;
    for task_id in &task_ids {
        validate_task_id(task_id)?;
    }
    if source.chars().count() > 512 {
        return Err("Support bundle source exceeds the 512-character limit.".into());
    }
    let workspace = WorkspaceEnv::from_option(workspace);
    let root = authorized_workspace(&registry, &workspace_key, &workspace)?;
    let db_path = authorized_ledger_path(&registry, &root, &db_path, &workspace, true)?;
    tauri::async_runtime::spawn_blocking(move || {
        let ledger = OrchestrationLedger::open(db_path)
            .map_err(|error| format!("Failed to open ledger: {error}"))?;
        let task_refs: Vec<&str> = task_ids.iter().map(String::as_str).collect();
        // IPC support bundles always redact secrets. Unsanitized exports are
        // reserved for trusted Rust-side diagnostic flows.
        export_support_bundle(&ledger, &task_refs, true, &source)
            .map_err(|error| format!("Failed to export bundle: {error}"))
    })
    .await
    .map_err(|error| format!("Support bundle worker failed: {error}"))?
}

// ---------------------------------------------------------------------------
// Schema version
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn orchestration_schema_version() -> i64 {
    SCHEMA_VERSION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_state_is_isolated_by_workspace() {
        let state = OrchestrationCommandState::default();
        state
            .with_workspace_mut("workspace-a".into(), |workspace| {
                workspace.task_graph.add_dependency("task", "dependency")
            })
            .unwrap()
            .unwrap();

        let workspace_a = state
            .with_workspace("workspace-a", |workspace| workspace.task_graph.nodes.len())
            .unwrap();
        let workspace_b = state
            .with_workspace("workspace-b", |workspace| workspace.task_graph.nodes.len())
            .unwrap();

        assert_eq!(workspace_a, 2);
        assert_eq!(workspace_b, 0);
    }

    #[test]
    fn poisoned_command_state_fails_closed() {
        let state = OrchestrationCommandState::default();
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = state.workspaces.lock().unwrap();
            panic!("poison command state");
        }));

        assert!(state.with_workspace("workspace", |_| ()).is_err());
        assert!(state
            .with_workspace_mut("workspace".into(), |_| ())
            .is_err());
    }

    #[test]
    fn ledger_path_cannot_escape_selected_workspace() {
        let workspace = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let registry = WorkspaceRegistry::default();
        let root = registry.authorize(workspace.path()).unwrap();
        let outside_ledger = outside.path().join("orchestration.sqlite");
        std::fs::write(&outside_ledger, []).unwrap();

        assert!(authorized_ledger_path(
            &registry,
            &root,
            outside_ledger.to_str().unwrap(),
            &WorkspaceEnv::Local,
            true,
        )
        .is_err());
        assert!(authorized_ledger_path(
            &registry,
            &root,
            workspace
                .path()
                .join("orchestration.sqlite")
                .to_str()
                .unwrap(),
            &WorkspaceEnv::Local,
            false,
        )
        .is_ok());
    }

    #[test]
    fn credential_accounts_are_workspace_scoped_and_opaque() {
        let first = credential_account("/private/workspace-a", "github", "token").unwrap();
        let second = credential_account("/private/workspace-b", "github", "token").unwrap();

        assert_ne!(first, second);
        assert!(!first.contains("/private/workspace-a"));
        assert!(credential_account("workspace", "bad:source", "token").is_err());
    }

    #[test]
    fn webview_cannot_register_managed_profiles() {
        assert!(validate_webview_profile_scope(ProfileScope::Managed).is_err());
        assert!(validate_webview_profile_scope(ProfileScope::User).is_ok());
        assert!(validate_webview_profile_scope(ProfileScope::Project).is_ok());
    }

    #[test]
    fn collection_and_identifier_limits_fail_closed() {
        assert!(validate_collection_len(MAX_COLLECTION_ITEMS + 1, "items").is_err());
        assert!(validate_task_id("").is_err());
        assert!(validate_task_id(&"x".repeat(MAX_TASK_ID_CHARS + 1)).is_err());
    }

    #[test]
    fn frontend_dtos_use_camel_case_and_preserve_defaults() {
        let profile: AgentProfileInput = serde_json::from_value(serde_json::json!({
            "name": "reviewer",
            "modelId": "provider/model",
            "autoSelectable": true
        }))
        .unwrap();
        assert_eq!(profile.model_id.as_deref(), Some("provider/model"));
        assert!(profile.auto_selectable);

        let budget: BudgetInput = serde_json::from_value(serde_json::json!({})).unwrap();
        assert_eq!(budget.warn_at_percent, 80);
        assert_eq!(budget.max_attempt_tokens, Some(200_000));

        let dependency: AddDependencyRequest = serde_json::from_value(serde_json::json!({
            "taskId": "task",
            "dependsOn": "dependency"
        }))
        .unwrap();
        assert_eq!(dependency.depends_on, "dependency");
    }
}
