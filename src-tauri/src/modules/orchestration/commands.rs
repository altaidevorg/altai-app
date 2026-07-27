//! Tauri command wiring for orchestration modules.
//!
//! Thin wrappers that expose the orchestration backend to the frontend.
//! All commands are stateless or use the shared OrchestrationState.

use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::State;

use super::checks::{evaluate_gate, evaluate_review, CheckResult, ReviewFinding};
use super::context::{build_context_pack, ContextConfig};
use super::credentials::{CredentialKey, CredentialStore, InMemoryCredentialStore};
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
use super::workflow_v2::BudgetsConfig;

use std::collections::{HashMap, HashSet};

// ---------------------------------------------------------------------------
// Shared state for command-bound modules
// ---------------------------------------------------------------------------

/// Shared state for modules that need persistent state across commands.
#[allow(dead_code)]
#[derive(Default)]
pub struct OrchestrationCommandState {
    pub profiles: Mutex<ProfileRegistry>,
    pub hierarchy: Mutex<TaskHierarchy>,
    pub mailbox: Mutex<Mailbox>,
    pub notifications: Mutex<NotificationDispatcher>,
    pub credentials: Mutex<InMemoryCredentialStore>,
    pub usage: Mutex<UsageTracker>,
    pub task_graph: Mutex<TaskGraph>,
    pub claim_state: Mutex<HashMap<String, String>>,
    pub audit_log: Mutex<super::credentials::CredentialAuditLog>,
}

// ---------------------------------------------------------------------------
// Quality dashboard
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn orchestration_quality_metrics(
    db_path: String,
    workspace_key: String,
    stale_threshold_ms: u64,
) -> Result<super::quality::QualityMetrics, String> {
    let ledger = OrchestrationLedger::open(PathBuf::from(&db_path))
        .map_err(|e| format!("Failed to open ledger: {e}"))?;
    compute_quality_metrics(&ledger, &workspace_key, stale_threshold_ms)
        .map_err(|e| format!("Failed to compute metrics: {e}"))
}

// ---------------------------------------------------------------------------
// Readiness scan
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn orchestration_readiness_scan(repo_path: String) -> super::readiness::ReadinessReport {
    readiness_scan(&PathBuf::from(&repo_path))
}

// ---------------------------------------------------------------------------
// Context pack
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn orchestration_context_pack(
    repo_path: String,
    task_description: String,
    budget_bytes: Option<usize>,
) -> super::context::ContextPack {
    let config = ContextConfig {
        budget_bytes: budget_bytes.unwrap_or(48 * 1024),
        ..ContextConfig::default()
    };
    build_context_pack(&PathBuf::from(&repo_path), &task_description, &config)
}

// ---------------------------------------------------------------------------
// Execution plans
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct PlanParseResult {
    pub plan: super::plans::ExecutionPlan,
}

#[tauri::command]
pub fn orchestration_plan_parse(
    source_path: String,
    revision: String,
    content: String,
) -> PlanParseResult {
    PlanParseResult {
        plan: parse_plan(&source_path, &revision, &content),
    }
}

#[tauri::command]
pub fn orchestration_decision_record(
    db_path: String,
    request: CreateDecisionRequest,
) -> Result<DecisionEntry, String> {
    let ledger = OrchestrationLedger::open(PathBuf::from(&db_path))
        .map_err(|e| format!("Failed to open ledger: {e}"))?;
    let log = DecisionLog::new(&ledger);
    log.record(&request)
        .map_err(|e| format!("Failed to record decision: {e}"))
}

#[tauri::command]
pub fn orchestration_decisions_for_task(
    db_path: String,
    task_id: String,
) -> Result<Vec<DecisionEntry>, String> {
    let ledger = OrchestrationLedger::open(PathBuf::from(&db_path))
        .map_err(|e| format!("Failed to open ledger: {e}"))?;
    DecisionLog::new(&ledger)
        .for_task(&task_id)
        .map_err(|e| format!("Failed to fetch decisions: {e}"))
}

// ---------------------------------------------------------------------------
// Task graph
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct AddDependencyRequest {
    pub task_id: String,
    pub depends_on: String,
}

#[derive(Serialize)]
pub struct AddDependencyResult {
    pub success: bool,
    pub error: Option<String>,
}

#[tauri::command]
pub fn orchestration_graph_add_dependency(
    state: State<'_, OrchestrationCommandState>,
    request: AddDependencyRequest,
) -> AddDependencyResult {
    let mut graph = state.task_graph.lock().unwrap();
    match graph.add_dependency(&request.task_id, &request.depends_on) {
        Ok(()) => AddDependencyResult {
            success: true,
            error: None,
        },
        Err(e) => AddDependencyResult {
            success: false,
            error: Some(e.to_string()),
        },
    }
}

#[tauri::command]
pub fn orchestration_graph_eligible(
    state: State<'_, OrchestrationCommandState>,
    completed: Vec<String>,
) -> Vec<String> {
    let graph = state.task_graph.lock().unwrap();
    let completed_set: HashSet<String> = completed.into_iter().collect();
    graph.eligible_tasks(&completed_set)
}

#[tauri::command]
pub fn orchestration_graph_blocked_reason(
    state: State<'_, OrchestrationCommandState>,
    task_id: String,
    completed: Vec<String>,
) -> Option<Vec<String>> {
    let graph = state.task_graph.lock().unwrap();
    let completed_set: HashSet<String> = completed.into_iter().collect();
    graph.blocked_reason(&task_id, &completed_set)
}

#[tauri::command]
pub fn orchestration_graph_topological_order(
    state: State<'_, OrchestrationCommandState>,
) -> Result<Vec<String>, String> {
    let graph = state.task_graph.lock().unwrap();
    graph.topological_order().map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Profiles
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn orchestration_profile_register(
    state: State<'_, OrchestrationCommandState>,
    profile: AgentProfileDef,
    scope: ProfileScope,
) {
    let mut registry = state.profiles.lock().unwrap();
    registry.register(profile, scope);
}

#[tauri::command]
pub fn orchestration_profile_resolve(
    state: State<'_, OrchestrationCommandState>,
    name: String,
) -> Option<super::profiles::EffectiveProfile> {
    let registry = state.profiles.lock().unwrap();
    registry.resolve(&name)
}

#[tauri::command]
pub fn orchestration_profile_select(
    state: State<'_, OrchestrationCommandState>,
    manual_choice: Option<String>,
    task_description: String,
    default_profile: String,
) -> super::profiles::ProfileSelection {
    let registry = state.profiles.lock().unwrap();
    select_profile(
        &registry,
        manual_choice.as_deref(),
        &task_description,
        &default_profile,
    )
}

#[tauri::command]
pub fn orchestration_profile_names(state: State<'_, OrchestrationCommandState>) -> Vec<String> {
    state.profiles.lock().unwrap().names()
}

// ---------------------------------------------------------------------------
// Team hierarchy + mailbox
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct AddChildRequest {
    pub parent_id: String,
    pub child_id: String,
}

#[tauri::command]
pub fn orchestration_hierarchy_add_child(
    state: State<'_, OrchestrationCommandState>,
    request: AddChildRequest,
) -> Result<(), String> {
    let mut hierarchy = state.hierarchy.lock().unwrap();
    hierarchy
        .add_child(&request.parent_id, &request.child_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn orchestration_hierarchy_children(
    state: State<'_, OrchestrationCommandState>,
    task_id: String,
) -> Vec<String> {
    state.hierarchy.lock().unwrap().children_of(&task_id)
}

#[tauri::command]
pub fn orchestration_hierarchy_descendants(
    state: State<'_, OrchestrationCommandState>,
    task_id: String,
) -> Vec<String> {
    state
        .hierarchy
        .lock()
        .unwrap()
        .descendants_of(&task_id)
        .into_iter()
        .collect()
}

#[tauri::command]
pub fn orchestration_mailbox_post(
    state: State<'_, OrchestrationCommandState>,
    message: AgentMessage,
) -> Result<(), String> {
    let mut mailbox = state.mailbox.lock().unwrap();
    mailbox
        .post(message)
        .map_err(|e| format!("Mailbox full (capacity: {})", e.capacity))
}

#[tauri::command]
pub fn orchestration_mailbox_deliver(
    state: State<'_, OrchestrationCommandState>,
) -> Option<AgentMessage> {
    state.mailbox.lock().unwrap().deliver()
}

// ---------------------------------------------------------------------------
// File conflict detection
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn orchestration_detect_file_conflicts(ownerships: Vec<FileOwnership>) -> Vec<FileConflict> {
    detect_conflicts(&ownerships)
}

// ---------------------------------------------------------------------------
// Notifications
// ---------------------------------------------------------------------------

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn orchestration_notify(
    state: State<'_, OrchestrationCommandState>,
    trigger: NotificationTrigger,
    task_id: String,
    attempt_id: Option<String>,
    title: String,
    body: String,
    now_ms: u64,
    now_hour: u8,
) -> String {
    let mut dispatcher = state.notifications.lock().unwrap();
    let result = dispatcher.dispatch(
        trigger,
        &task_id,
        attempt_id.as_deref(),
        &title,
        &body,
        now_ms,
        now_hour,
    );
    format!("{result:?}")
}

#[tauri::command]
pub fn orchestration_notifications_drain(
    state: State<'_, OrchestrationCommandState>,
) -> Vec<super::notifications::Notification> {
    state.notifications.lock().unwrap().drain()
}

// ---------------------------------------------------------------------------
// Credentials
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn orchestration_credential_store(
    state: State<'_, OrchestrationCommandState>,
    source: String,
    name: String,
    value: String,
) -> Result<(), String> {
    let store = state.credentials.lock().unwrap();
    store
        .store(&CredentialKey::new(source, name), &value)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn orchestration_credential_retrieve(
    state: State<'_, OrchestrationCommandState>,
    source: String,
    name: String,
) -> Result<Option<String>, String> {
    let store = state.credentials.lock().unwrap();
    store
        .retrieve(&CredentialKey::new(source, name))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn orchestration_credential_revoke(
    state: State<'_, OrchestrationCommandState>,
    source: String,
    name: String,
) -> Result<bool, String> {
    let store = state.credentials.lock().unwrap();
    store
        .revoke(&CredentialKey::new(source, name))
        .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Checks + review
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn orchestration_check_gate(results: Vec<CheckResult>) -> super::checks::GateResult {
    evaluate_gate(&results)
}

#[tauri::command]
pub fn orchestration_review_evaluate(
    findings: Vec<ReviewFinding>,
    allow_style_blocking: bool,
) -> super::checks::ReviewResult {
    evaluate_review(&findings, allow_style_blocking)
}

// ---------------------------------------------------------------------------
// Usage tracking
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn orchestration_usage_process(
    state: State<'_, OrchestrationCommandState>,
    event: UsageEvent,
    config: BudgetsConfig,
) -> super::usage_wiring::UsageResult {
    let mut tracker = state.usage.lock().unwrap();
    tracker.process(&event, &config)
}

#[tauri::command]
pub fn orchestration_usage_should_stop(
    state: State<'_, OrchestrationCommandState>,
    task_id: String,
) -> bool {
    let tracker = state.usage.lock().unwrap();
    super::usage_wiring::should_stop_task(&tracker, &task_id)
}

// ---------------------------------------------------------------------------
// Worker pool (stateless info)
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn orchestration_detect_overlaps(
    diffs: Vec<ChildDiff>,
) -> Vec<super::integration::DiffOverlap> {
    detect_overlaps(&diffs)
}

// ---------------------------------------------------------------------------
// Gardening
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn orchestration_gardening_scan(
    repo_path: String,
    stale_doc_days: Option<u32>,
    now_ms: u64,
) -> super::gardening::GardeningReport {
    let config = GardeningConfig {
        stale_doc_days: stale_doc_days.unwrap_or(90),
        ..GardeningConfig::default()
    };
    run_gardening(&PathBuf::from(&repo_path), &config, now_ms)
}

// ---------------------------------------------------------------------------
// Session analysis
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn orchestration_session_analyze(
    db_path: String,
    workspace_key: String,
) -> Result<Vec<super::session_analysis::AttemptAnalysis>, String> {
    let ledger = OrchestrationLedger::open(PathBuf::from(&db_path))
        .map_err(|e| format!("Failed to open ledger: {e}"))?;
    let tasks = ledger
        .tasks_for_workspace(&workspace_key)
        .map_err(|e| format!("Failed to fetch tasks: {e}"))?;
    let config = AnalysisConfig::default();
    let mut analyses = Vec::with_capacity(tasks.len());
    for task in &tasks {
        let analysis = super::session_analysis::analyze_task(&ledger, task, &config)
            .map_err(|e| format!("Analysis failed: {e}"))?;
        analyses.push(analysis);
    }
    Ok(analyses)
}

#[tauri::command]
pub fn orchestration_playbook_propose(
    analyses: Vec<super::session_analysis::AttemptAnalysis>,
) -> Vec<super::session_analysis::PlaybookProposal> {
    propose_playbooks(&analyses)
}

// ---------------------------------------------------------------------------
// Eval lab / support bundle
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn orchestration_support_bundle(
    db_path: String,
    task_ids: Vec<String>,
    sanitize: bool,
    source: String,
) -> Result<super::eval_lab::SupportBundle, String> {
    let ledger = OrchestrationLedger::open(PathBuf::from(&db_path))
        .map_err(|e| format!("Failed to open ledger: {e}"))?;
    let task_refs: Vec<&str> = task_ids.iter().map(|s| s.as_str()).collect();
    export_support_bundle(&ledger, &task_refs, sanitize, &source)
        .map_err(|e| format!("Failed to export bundle: {e}"))
}

// ---------------------------------------------------------------------------
// Schema version
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn orchestration_schema_version() -> i64 {
    SCHEMA_VERSION
}
