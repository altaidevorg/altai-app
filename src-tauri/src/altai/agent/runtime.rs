use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::async_runtime;
use tauri::{AppHandle, Manager};
use tokio::sync::mpsc;

use altai_agent_service::{
    admit_run, admit_user_message, coordinator_guard, queue_run, rollback_run_admission,
    permission_mode_to_edit_mode, permission_mode_to_shell_mode, AgentEventEnvelope,
    AgentEventSink, AgentService, DocumentPart, ReplayService, RunCoordinator, SessionIdentity,
    SharedRunCoordinator,
};
#[cfg(test)]
use altai_agent_service::RunTransitionError;
use super::desktop_host::{DesktopHost, DesktopWorkspaceServices};

#[cfg(test)]
use altai_agent_service::RunAdmission;

use isanagent::agent::{AgentLogic, AgentLogicParams};
use isanagent::bus::BusMessage;
use isanagent::channels::Channel;
use isanagent::clarification::ClarificationHub;
use isanagent::provider;
use isanagent::scheduler::{CronCommand, CronStore, ScheduleKind};
use isanagent::session::SessionManager;
use isanagent::skills::SkillRegistry;
use isanagent::tools::builtin::{
    CronTool, EditFileTool, FetchMemoryByDateTool, GitWorktreeTool, GlobFilesTool, ListDirTool,
    ReadFileTool, SearchMemoryTool, SearchTextTool, ShellExecTool, WebFetchTool, WebSearchTool,
    WriteFileTool,
};
use isanagent::tools::ml_domain::{ArxivFetchTool, ArxivSearchTool, HfHubFileFetchTool};
use isanagent::tools::workflow::{AskUserTool, TodoWriteTool, ToolSearchTool};
use isanagent::tools::ToolRegistry;
use isanagent::workspace::{resolve_workspace_root, IsanagentWorkspace};
use isanagent::NodeHandle;

use super::commands::DocumentArg;
use super::event_journal::{AppendStatus, EventJournal, JournalEvent};
use super::tauri_channel::{
    map_lifecycle_to_event, map_telemetry_to_event, telemetry_chat_id, TauriChannel,
};
use crate::modules::mcp;

pub use altai_agent_service::{
    AgentReplayEventEnvelope, AgentRunReplayCursor, CancelAck, CompactionArg, EditDiffPayload,
    Event, ManualCompactionAck, SendAck, SteerAck,
};

#[derive(Debug)]
enum RunEventDeliveryError {
    Serialization,
    Transition(String),
    Persistence(String),
    Renderer(String),
}

impl std::fmt::Display for RunEventDeliveryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Serialization => formatter.write_str("agent_event_serialization_failed"),
            Self::Transition(detail) => {
                write!(formatter, "agent_event_transition_rejected: {detail}")
            }
            Self::Persistence(detail) => {
                write!(formatter, "agent_event_persistence_failed: {detail}")
            }
            Self::Renderer(detail) => {
                write!(formatter, "agent_event_renderer_unavailable: {detail}")
            }
        }
    }
}

fn emit_event(
    sink: &dyn AgentEventSink,
    chat_id: &str,
    event: &Event,
    run: Option<(String, u64)>,
) -> Result<(), RunEventDeliveryError> {
    let payload = redacted_event_payload(event)?;
    emit_payload(sink, chat_id, &payload, run)
}

fn emit_payload(
    sink: &dyn AgentEventSink,
    chat_id: &str,
    payload: &serde_json::Value,
    run: Option<(String, u64)>,
) -> Result<(), RunEventDeliveryError> {
    let envelope = match run {
        Some((run_id, seq)) => AgentEventEnvelope::run(chat_id, run_id, seq, payload.clone()),
        None => AgentEventEnvelope::system(chat_id, payload.clone()),
    };
    sink.try_send(envelope)
        .map_err(|error| RunEventDeliveryError::Renderer(error.to_string()))
}

fn redacted_event_payload(event: &Event) -> Result<serde_json::Value, RunEventDeliveryError> {
    let mut payload =
        serde_json::to_value(event).map_err(|_| RunEventDeliveryError::Serialization)?;
    isanagent::redact::shared().redact_json(&mut payload);
    Ok(payload)
}

enum RunEventTransition<'a> {
    Started(&'a str),
    Next,
    NextForRun(&'a str),
    Terminated(&'a str),
}

#[cfg(test)]
fn persist_run_event(
    coordinator: &SharedRunCoordinator,
    journal: &EventJournal,
    chat_id: &str,
    owner_id: &str,
    event: &Event,
    transition: RunEventTransition<'_>,
) -> Result<(String, u64), RunEventDeliveryError> {
    let payload = redacted_event_payload(event)?;
    persist_run_payload(
        coordinator,
        journal,
        chat_id,
        owner_id,
        &payload,
        transition,
    )
}

fn persist_run_payload(
    coordinator: &SharedRunCoordinator,
    journal: &EventJournal,
    chat_id: &str,
    owner_id: &str,
    payload: &serde_json::Value,
    transition: RunEventTransition<'_>,
) -> Result<(String, u64), RunEventDeliveryError> {
    let kind = payload
        .get("type")
        .and_then(serde_json::Value::as_str)
        .ok_or(RunEventDeliveryError::Serialization)?
        .to_string();

    // Sequence assignment and durable append are one coordinator transition.
    // SQLite I/O is intentionally performed while the coordinator is locked:
    // otherwise two producers could reserve the same next sequence or a later
    // event could overtake a failed append and leave a permanent journal gap.
    let mut coordinator = coordinator_guard(coordinator);
    let before = coordinator.clone();
    let run = match transition {
        RunEventTransition::Started(run_id) => coordinator.started(chat_id, run_id, owner_id),
        RunEventTransition::Next => coordinator.next(chat_id, owner_id),
        RunEventTransition::NextForRun(run_id) => {
            coordinator.next_for_run(chat_id, run_id, owner_id)
        }
        RunEventTransition::Terminated(run_id) => coordinator.terminated(chat_id, run_id, owner_id),
    }
    .map_err(|error| RunEventDeliveryError::Transition(format!("{error:?}")))?;

    let journal_event = JournalEvent::now(1, run.0.clone(), run.1, chat_id, kind, payload.clone());
    let append = if matches!(transition, RunEventTransition::Terminated(_)) {
        journal.append_terminal(&journal_event)
    } else {
        journal.append(&journal_event)
    };
    match append {
        Ok(AppendStatus::Appended | AppendStatus::Duplicate) => Ok(run),
        Err(error) => {
            *coordinator = before;
            Err(RunEventDeliveryError::Persistence(error.to_string()))
        }
    }
}

pub(crate) fn deliver_next_run_event(
    sink: &dyn AgentEventSink,
    journal: &EventJournal,
    coordinator: &SharedRunCoordinator,
    chat_id: &str,
    owner_id: &str,
    event: &Event,
) -> Result<(), String> {
    let result = persist_and_deliver_to_renderer(
        sink,
        journal,
        coordinator,
        chat_id,
        owner_id,
        event,
        RunEventTransition::Next,
    );
    match result {
        Ok(_) => Ok(()),
        // Persistence already advanced the durable sequence. A disconnected
        // renderer recovers it via replay, so surfacing delivery failure to
        // IsanAgent would only invite a duplicate semantic event.
        Err(error @ RunEventDeliveryError::Renderer(_)) => {
            log::warn!("Agent event for chat {chat_id} awaits replay: {error}");
            Ok(())
        }
        Err(error) => Err(error.to_string()),
    }
}

fn persist_and_deliver_to_renderer(
    sink: &dyn AgentEventSink,
    journal: &EventJournal,
    coordinator: &SharedRunCoordinator,
    chat_id: &str,
    owner_id: &str,
    event: &Event,
    transition: RunEventTransition<'_>,
) -> Result<(String, u64), RunEventDeliveryError> {
    persist_and_deliver_run_event(
        coordinator,
        journal,
        chat_id,
        owner_id,
        event,
        transition,
        |run, payload| emit_payload(sink, chat_id, payload, Some(run.clone())),
    )
}

fn persist_and_deliver_run_event<F>(
    coordinator: &SharedRunCoordinator,
    journal: &EventJournal,
    chat_id: &str,
    owner_id: &str,
    event: &Event,
    transition: RunEventTransition<'_>,
    deliver: F,
) -> Result<(String, u64), RunEventDeliveryError>
where
    F: FnOnce(&(String, u64), &serde_json::Value) -> Result<(), RunEventDeliveryError>,
{
    let payload = redacted_event_payload(event)?;
    let run = persist_run_payload(
        coordinator,
        journal,
        chat_id,
        owner_id,
        &payload,
        transition,
    )?;
    deliver(&run, &payload)?;
    Ok(run)
}

fn is_system_event(event: &Event) -> bool {
    matches!(
        event,
        Event::BackgroundJobUpdated { .. }
            | Event::NotificationCreated { .. }
            | Event::NotificationUpdated { .. }
    )
}

/// Wall-clock epoch millis. Used to stamp MCP status transitions so the
/// Settings UI can hint at staleness (`updated 2m ago`). `SystemTime::now`
/// is good enough here — this is advisory metadata, not a monotonic clock.
fn now_epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Parse the crate's `metadata.edit_diff` object into the structured
/// [`EditDiffPayload`] the frontend renders. The crate attaches this to
/// `ask_user` outbounds from the edit gate (`builtin.rs`); the shape is
/// `{ file, diff, truncated }`. We validate defensively (model-produced
/// metadata is untrusted) and drop malformed values rather than failing the
/// whole clarification — the user can still approve/deny from the text prompt.
fn parse_edit_diff(value: &serde_json::Value) -> Option<EditDiffPayload> {
    let obj = value.as_object()?;
    let file = obj.get("file").and_then(|v| v.as_str())?.to_string();
    let diff = obj.get("diff").and_then(|v| v.as_str())?.to_string();
    let truncated = obj
        .get("truncated")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    Some(EditDiffPayload {
        file,
        diff,
        truncated,
    })
}

struct WorkspaceIngress {
    chat_id: String,
    inbound: isanagent::bus::InboundMessage,
    reply: tokio::sync::oneshot::Sender<Result<(), String>>,
}

/// A workspace-owned synthetic-inbound dispatcher. It never accepts a model
/// selected destination: a chat must first be bound by `route_send`, and each
/// route is replaced only after another successful send for that same chat.
pub(crate) struct WorkspaceDispatcher {
    #[allow(dead_code)] // consumed by cron/background adapters added after I4/I5
    tx: mpsc::Sender<WorkspaceIngress>,
    routes: Arc<tokio::sync::Mutex<HashMap<String, WorkspaceRoute>>>,
    #[allow(dead_code)]
    task: async_runtime::JoinHandle<()>,
}

#[derive(Clone)]
struct WorkspaceRoute {
    bus_tx: mpsc::Sender<BusMessage>,
    owner_id: String,
}

impl WorkspaceDispatcher {
    pub(crate) fn new(run_coordinator: SharedRunCoordinator) -> Self {
        let routes = Arc::new(tokio::sync::Mutex::new(
            HashMap::<String, WorkspaceRoute>::new(),
        ));
        let (tx, mut rx) = mpsc::channel::<WorkspaceIngress>(100);
        let routes_for_task = routes.clone();
        let coordinator_for_task = run_coordinator.clone();
        let task = async_runtime::spawn(async move {
            while let Some(ingress) = rx.recv().await {
                let route = routes_for_task.lock().await.get(&ingress.chat_id).cloned();
                let result = match route {
                    Some(route) => {
                        let run_id = inbound_run_id(&ingress.inbound).map(str::to_string);
                        match run_id {
                            Some(run_id) => match if is_queueable_synthetic(&ingress.inbound) {
                                queue_run(
                                    &coordinator_for_task,
                                    &ingress.chat_id,
                                    &run_id,
                                    &route.owner_id,
                                )
                                .map(|_| ())
                            } else if is_clarification_reply(&ingress.inbound) {
                                admit_user_message(
                                    &coordinator_for_task,
                                    &ingress.chat_id,
                                    &run_id,
                                    &route.owner_id,
                                )
                                .map(|_| ())
                            } else {
                                admit_run(
                                    &coordinator_for_task,
                                    &ingress.chat_id,
                                    &run_id,
                                    &route.owner_id,
                                )
                            } {
                                Ok(()) => {
                                    let result = route
                                        .bus_tx
                                        .send(BusMessage::Inbound(ingress.inbound))
                                        .await
                                        .map_err(|_| {
                                            "The owning agent runtime is no longer available"
                                                .to_string()
                                        });
                                    if result.is_err() {
                                        rollback_run_admission(
                                            &coordinator_for_task,
                                            &ingress.chat_id,
                                            &run_id,
                                            &route.owner_id,
                                        );
                                    }
                                    result
                                }
                                Err(error) => Err(error),
                            },
                            None => Err("Trusted inbound is missing its run ID".to_string()),
                        }
                    }
                    None => Err("No owning agent runtime is registered for this chat".to_string()),
                };
                let _ = ingress.reply.send(result);
            }
        });
        Self { tx, routes, task }
    }

    pub(crate) async fn bind(&self, chat_id: &str, bus_tx: mpsc::Sender<BusMessage>, owner_id: &str) {
        self.routes.lock().await.insert(
            chat_id.to_string(),
            WorkspaceRoute {
                bus_tx,
                owner_id: owner_id.to_string(),
            },
        );
    }

    #[allow(dead_code)] // exercised in tests; production callers arrive with cron/resume adapters
    pub(crate) async fn dispatch(
        &self,
        chat_id: String,
        inbound: isanagent::bus::InboundMessage,
    ) -> Result<(), String> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.tx
            .send(WorkspaceIngress {
                chat_id,
                inbound,
                reply: tx,
            })
            .await
            .map_err(|_| "The workspace ingress dispatcher is no longer available".to_string())?;
        rx.await
            .map_err(|_| "The workspace ingress dispatcher stopped before routing".to_string())?
    }
}

/// Thin Desktop facade over the shared AgentService lifecycle.
///
/// Long-lived IsanAgent instance ownership lives in `altai-agent-service`.
/// Desktop retains HostAdapter seams (Tauri channel, MCP, workspace actors).
pub struct AgentRuntime {
    pub service: AgentService<DesktopHost>,
}

pub fn init(app: AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let run_coordinator = Arc::new(StdMutex::new(RunCoordinator::default()));
    let host = Arc::new(DesktopHost::new(app.clone(), run_coordinator.clone()));
    app.manage(AgentRuntime {
        service: AgentService::with_coordinator(host, run_coordinator),
    });

    Ok(())
}

/// Get-or-create workspace-owned services (`""` = the default IsanAgent
/// workspace). Delegates to the Desktop HostAdapter.
async fn ensure_workspace_services(
    runtime: &AgentRuntime,
    workspace_root: &str,
) -> Result<Arc<DesktopWorkspaceServices>, String> {
    runtime
        .service
        .host()
        .workspace_services(workspace_root)
        .await
}

/// Compatibility helper for history/inbox calls while they are migrated to
/// consume the full workspace service record.
async fn ensure_memory(
    runtime: &AgentRuntime,
    workspace_root: &str,
) -> Result<NodeHandle<isanagent::memory::MemoryMessage>, String> {
    Ok(ensure_workspace_services(runtime, workspace_root)
        .await?
        .memory_node
        .clone())
}

/// Read durable events strictly after the renderer's last acknowledged
/// sequence. This path only opens the workspace journal; it never constructs
/// an agent instance, dispatches inbound work, or touches provider/tool code.
pub async fn replay_run_events(
    runtime: &AgentRuntime,
    workspace_path: &str,
    chat_id: &str,
    run_id: &str,
    after_seq: u64,
    limit: usize,
) -> Result<Vec<AgentReplayEventEnvelope>, String> {
    let chat_id = SessionIdentity::parse(validate_tauri_chat_id(chat_id)?)
        .map_err(|error| error.to_string())?;

    let workspace_root = format!("{}/.isanagent", workspace_path.trim_end_matches('/'));
    let services = ensure_workspace_services(runtime, &workspace_root).await?;
    ReplayService::new(&services.event_journal)
        .replay_run_events(&chat_id, run_id, after_seq, limit)
        .map_err(|error| error.to_string())
}

/// Discover the newest durable run for a restored chat without replaying or
/// starting work. On a fresh host process, startup classification first makes
/// every run inherited from the previous process terminal; a run started by
/// the current process may still be live during a renderer-only reconnect.
pub async fn latest_run_replay_cursor(
    runtime: &AgentRuntime,
    workspace_path: &str,
    chat_id: &str,
) -> Result<Option<AgentRunReplayCursor>, String> {
    let chat_id = SessionIdentity::parse(validate_tauri_chat_id(chat_id)?)
        .map_err(|error| error.to_string())?;
    let workspace_root = format!("{}/.isanagent", workspace_path.trim_end_matches('/'));
    let services = ensure_workspace_services(runtime, &workspace_root).await?;
    ReplayService::new(&services.event_journal)
        .latest_run_replay_cursor(&chat_id)
        .map_err(|error| error.to_string())
}

#[cfg(test)]
fn replay_events_from_journal(
    journal: &EventJournal,
    chat_id: &str,
    run_id: &str,
    after_seq: u64,
    limit: usize,
) -> Result<Vec<AgentReplayEventEnvelope>, String> {
    let chat_id = SessionIdentity::parse(chat_id).map_err(|error| error.to_string())?;
    ReplayService::new(journal)
        .replay_run_events(&chat_id, run_id, after_seq, limit)
        .map_err(|error| error.to_string())
}

/// Route a user message to the instance for `config` (built or reused).
#[allow(clippy::too_many_arguments)]
pub async fn route_send(
    runtime: &AgentRuntime,
    provider_name: &str,
    api_key: &str,
    model_name: &str,
    persona_instructions: Option<&str>,
    base_url_override: Option<&str>,
    workspace_path: Option<&str>,
    permission_mode: Option<&str>,
    compaction: Option<&CompactionArg>,
    fallback: Option<isanagent::agent::FallbackProviderSpec>,
    message: String,
    images: Vec<String>,
    documents: Vec<DocumentArg>,
    chat_id: String,
    queue: bool,
) -> Result<SendAck, String> {
    let documents = documents
        .into_iter()
        .map(|document| DocumentPart {
            data: document.data,
            media_type: document.media_type,
            name: document.name,
        })
        .collect();
    runtime
        .service
        .route_send(
            provider_name,
            api_key,
            model_name,
            persona_instructions,
            base_url_override,
            workspace_path,
            permission_mode,
            compaction,
            fallback,
            message,
            images,
            documents,
            chat_id,
            queue,
        )
        .await
}

pub(crate) async fn recover_background_jobs_after_owner_bind(
    memory_node: &NodeHandle<isanagent::memory::MemoryMessage>,
    dispatcher: &WorkspaceDispatcher,
    chat_id: &str,
) -> Result<(), String> {
    let records = request_memory(memory_node, |reply| {
        isanagent::memory::MemoryMessage::ListBackgroundJobs {
            chat_id: Some(chat_id.to_string()),
            channel: Some("tauri".to_string()),
            limit: 500,
            reply,
        }
    })
    .await?;
    for job in records.into_iter().filter(|job| {
        job.state == "running"
            && job.resume_after_restart
            && is_tauri_root_identity(
                &job.channel,
                job.thread_id.as_deref(),
                Some(chat_id),
                &job.chat_id,
            )
    }) {
        let content = serde_json::from_str::<serde_json::Value>(&job.payload_json)
            .ok()
            .and_then(|payload| {
                payload
                    .get("message")
                    .and_then(|message| message.as_str())
                    .map(str::to_string)
            })
            .unwrap_or_else(|| format!("Resume background job {}", job.job_id));
        let mut metadata = HashMap::new();
        metadata.insert(
            isanagent::bus::METADATA_SYNTHETIC_BACKGROUND_RESUME.to_string(),
            serde_json::Value::Bool(true),
        );
        metadata.insert(
            isanagent::bus::METADATA_BACKGROUND_JOB_ID.to_string(),
            serde_json::Value::String(job.job_id),
        );
        dispatcher
            .dispatch(
                chat_id.to_string(),
                trusted_tauri_inbound(isanagent::bus::InboundMessage {
                    channel: "tauri".to_string(),
                    sender_id: "altai_background_recovery".to_string(),
                    chat_id: chat_id.to_string(),
                    thread_id: None,
                    content,
                    attachments: Vec::new(),
                    metadata,
                }),
            )
            .await?;
    }
    Ok(())
}

/// Route a trusted synthetic inbound message to the runtime that most recently
/// served this ALTAI chat in the selected workspace. This is intentionally not
/// exposed as generic renderer IPC: cron and background-resume adapters call
/// it after deriving the destination from persisted host-owned state.
#[allow(dead_code)] // intentionally backend-only until cron/resume adapters are enabled
pub async fn dispatch_synthetic_inbound(
    runtime: &AgentRuntime,
    workspace_path: Option<&str>,
    chat_id: &str,
    inbound: isanagent::bus::InboundMessage,
) -> Result<(), String> {
    let chat_id = validate_tauri_chat_id(chat_id)?;
    if inbound.channel != "tauri" || inbound.chat_id != chat_id || inbound.thread_id.is_some() {
        return Err("Synthetic inbound identity does not match the Tauri root chat".to_string());
    }
    let workspace_root = workspace_path
        .map(|path| format!("{}/.isanagent", path.trim_end_matches('/')))
        .unwrap_or_default();
    ensure_workspace_services(runtime, &workspace_root)
        .await?
        .dispatcher
        .dispatch(chat_id.to_string(), trusted_tauri_inbound(inbound))
        .await
}

/// Trusted Rust-side producers, unlike renderer IPC, own run-id generation.
/// Every synthetic Tauri turn receives a fresh ID before IsanAgent admission.
pub(crate) fn trusted_tauri_inbound(
    mut inbound: isanagent::bus::InboundMessage,
) -> isanagent::bus::InboundMessage {
    debug_assert_eq!(inbound.channel, "tauri");
    inbound.metadata.insert(
        isanagent::bus::METADATA_RUN_ID.to_string(),
        serde_json::Value::String(uuid::Uuid::new_v4().to_string()),
    );
    inbound
}

fn inbound_run_id(inbound: &isanagent::bus::InboundMessage) -> Option<&str> {
    inbound
        .metadata
        .get(isanagent::bus::METADATA_RUN_ID)
        .and_then(serde_json::Value::as_str)
        .filter(|run_id| !run_id.trim().is_empty())
}

fn is_queueable_synthetic(inbound: &isanagent::bus::InboundMessage) -> bool {
    inbound
        .metadata
        .get(isanagent::bus::METADATA_SYNTHETIC_BACKGROUND_RESUME)
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
        && !inbound
            .metadata
            .contains_key(isanagent::bus::METADATA_CLARIFICATION_TICKET_ID)
}

fn is_clarification_reply(inbound: &isanagent::bus::InboundMessage) -> bool {
    inbound
        .metadata
        .contains_key(isanagent::bus::METADATA_CLARIFICATION_TICKET_ID)
}

pub async fn route_manual_compaction(
    runtime: &AgentRuntime,
    workspace_path: &str,
    chat_id: String,
    focus_instructions: Option<String>,
) -> Result<ManualCompactionAck, String> {
    let chat_id = validate_tauri_chat_id(&chat_id)?.to_owned();
    runtime
        .service
        .route_manual_compaction(workspace_path, chat_id, focus_instructions)
        .await
}

pub async fn route_cancel(
    runtime: &AgentRuntime,
    chat_id: String,
    run_id: String,
) -> Result<CancelAck, String> {
    runtime.service.route_cancel(chat_id, run_id).await
}

/// Route new user direction to the runtime instance that owns one exact,
/// currently-running lease. Enqueueing on that instance's FIFO is the backend
/// acceptance boundary exposed to Tauri; IsanAgent applies it at its next safe
/// provider/tool boundary.
pub async fn route_steer(
    runtime: &AgentRuntime,
    chat_id: String,
    run_id: String,
    content: String,
) -> Result<SteerAck, String> {
    runtime
        .service
        .route_steer(chat_id, run_id, content)
        .await
}

/// Warm up (or ensure) the instance for a config. Kept for the `agent_start`
/// command; dispatch now happens through `route_send`.
#[allow(clippy::too_many_arguments)]
pub async fn start_agent(
    runtime: &AgentRuntime,
    provider_name: &str,
    api_key: &str,
    model_name: &str,
    persona_instructions: Option<&str>,
    base_url_override: Option<&str>,
    workspace_path: Option<&str>,
    permission_mode: Option<&str>,
    compaction: Option<&CompactionArg>,
) -> Result<(), String> {
    runtime
        .service
        .start_agent(
            provider_name,
            api_key,
            model_name,
            persona_instructions,
            base_url_override,
            workspace_path,
            permission_mode,
            compaction,
        )
        .await
}

/// One chat session as known to the backend memory DB (the source of truth for
/// what conversations have actually happened in this workspace). Returned to
/// the frontend so it can reconcile its own `altai-ai-sessions.json` list and
/// surface chats that were closed (dropped from the frontend store) but still
/// live in the agent memory.
///
/// Mirrors `RootThreadListItem` from the isanagent crate, flattened to JSON-
/// friendly camelCase via `#[serde(rename_all = "camelCase")]`.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    /// Bare chat id, e.g. `s-mrqa417u-hb75wq` (the `tauri:` channel prefix and
    /// trailing colon are stripped from the stored `messages.thread_id`).
    pub id: String,
    /// Latest activity timestamp, epoch milliseconds (UTC). `0` if unknown.
    pub updated_at: i64,
    /// First user message preview (runtime prefix stripped), used as the title.
    pub title: String,
}

/// Safe frontend projection of IsanAgent's persisted notification record.
///
/// The raw action payload and transport selectors stay backend-only. Keeping
/// the actor record behind an ALTAI-owned camelCase contract also prevents
/// future IsanAgent schema additions from becoming accidental IPC API changes.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentNotificationInfo {
    pub id: String,
    pub chat_id: String,
    pub kind: String,
    pub title: String,
    pub body: String,
    pub action_kind: Option<String>,
    pub seen_at_ms: Option<i64>,
    pub resolved_at_ms: Option<i64>,
    pub created_at_ms: i64,
}

impl From<isanagent::memory::NotificationRecord> for AgentNotificationInfo {
    fn from(record: isanagent::memory::NotificationRecord) -> Self {
        Self {
            id: record.notification_id,
            chat_id: record.chat_id,
            kind: record.kind,
            title: record.title,
            body: record.body,
            action_kind: record.action_kind,
            seen_at_ms: record.seen_at_ms,
            resolved_at_ms: record.resolved_at_ms,
            created_at_ms: record.created_at_ms,
        }
    }
}

/// Safe list projection of a durable IsanAgent background job.
///
/// `payload_json` is deliberately excluded: it can contain full prompts or
/// execution payloads and is not required to render status in ALTAI.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentBackgroundJobInfo {
    pub id: String,
    pub kind: String,
    pub chat_id: String,
    pub state: String,
    pub resume_after_restart: bool,
    pub detached: bool,
    pub last_error: Option<String>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

impl From<isanagent::memory::BackgroundJobRecord> for AgentBackgroundJobInfo {
    fn from(record: isanagent::memory::BackgroundJobRecord) -> Self {
        Self {
            id: record.job_id,
            kind: record.kind,
            chat_id: record.chat_id,
            state: record.state,
            resume_after_restart: record.resume_after_restart,
            detached: record.detached,
            last_error: record.last_error,
            created_at_ms: record.created_at_ms,
            updated_at_ms: record.updated_at_ms,
        }
    }
}

/// Safe frontend projection of a persisted background clarification ticket.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentClarificationTicketInfo {
    pub id: String,
    pub job_id: String,
    pub chat_id: String,
    pub prompt: String,
    pub choices: Vec<String>,
    pub response: Option<String>,
    pub status: String,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

/// Renderer-safe view of a workspace automation. The scheduler's webhook
/// token deliberately remains host-only.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentAutomationInfo {
    pub id: String,
    pub schedule: AgentAutomationScheduleInfo,
    pub message: String,
    pub chat_id: String,
    pub last_run_at_ms: Option<i64>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AgentAutomationScheduleInfo {
    At { at_ms: i64 },
    Every { every_ms: i64 },
    Cron { cron_expr: String },
}

impl From<ScheduleKind> for AgentAutomationScheduleInfo {
    fn from(schedule: ScheduleKind) -> Self {
        match schedule {
            ScheduleKind::At { at_ms } => Self::At { at_ms },
            ScheduleKind::Every { every_ms } => Self::Every { every_ms },
            ScheduleKind::Cron { cron_expr } => Self::Cron { cron_expr },
        }
    }
}

impl From<isanagent::memory::ClarificationTicketRecord> for AgentClarificationTicketInfo {
    fn from(record: isanagent::memory::ClarificationTicketRecord) -> Self {
        let choices = record
            .choices_json
            .as_deref()
            .and_then(|json| serde_json::from_str::<Vec<String>>(json).ok())
            .unwrap_or_default();
        Self {
            id: record.ticket_id,
            job_id: record.job_id,
            chat_id: record.chat_id,
            prompt: record.prompt,
            choices,
            response: record.response,
            status: record.status,
            created_at_ms: record.created_at_ms,
            updated_at_ms: record.updated_at_ms,
        }
    }
}

/// List all chat sessions persisted in this workspace's backend memory DB.
///
/// Queries the shared per-workspace memory actor via the isanagent crate's
/// `ListRootThreadsForChannelWithPreviews` message — the same store the agent
/// itself uses for history — so the frontend's chat history list reflects what
/// the backend actually knows, not just what survived in the ephemeral
/// `altai-ai-sessions.json`. This is the reconciliation path that makes closed
/// chats reappear in history (Claude Code / Cursor behavior): the backend DB is
/// the durable source of truth.
pub async fn list_sessions(
    runtime: &AgentRuntime,
    workspace_path: Option<&str>,
) -> Result<Vec<SessionInfo>, String> {
    let workspace_root = workspace_path
        .map(|p| format!("{}/.isanagent", p.trim_end_matches('/')))
        .unwrap_or_default();
    let memory_node = ensure_memory(runtime, &workspace_root).await?;

    // Ask the memory actor for all root threads on this channel (`tauri:*`).
    let (tx, rx) = tokio::sync::oneshot::channel();
    let reply = isanagent::memory::SharedReply::new(tx);
    memory_node
        .send_packet(
            isanagent::memory::MemoryMessage::ListRootThreadsForChannelWithPreviews {
                channel: "tauri".to_string(),
                limit: 200,
                reply,
            },
        )
        .await
        .map_err(|e| format!("Failed to query memory actor: {}", e))?;

    let rows = rx
        .await
        .map_err(|_| "Memory actor closed before replying".to_string())?
        .map_err(|e| format!("Memory actor error: {}", e))?;

    // Read existing Desktop history through the shared legacy alias instead
    // of re-parsing `tauri:<chat_id>:` in every host adapter.
    let sessions = rows
        .into_iter()
        .map(|r| {
            let bare_id = SessionIdentity::from_legacy_tauri_thread_id(&r.thread_id)
                .map(|identity| identity.as_str().to_string())
                .unwrap_or(r.thread_id);
            SessionInfo {
                id: bare_id,
                updated_at: r.last_activity_ms,
                title: r.preview,
            }
        })
        .collect();
    Ok(sessions)
}

/// Load the full message history for one chat session from the backend memory DB.
///
/// Returns the raw stored messages (OpenAI-style role/content/tool_calls) so the
/// frontend can hydrate a reopened chat with its actual conversation — including
/// chats that were closed and only survived in the durable backend store. This is
/// the counterpart to [`list_sessions`]: `list_sessions` recovers the *list*,
/// this recovers the *contents*.
pub async fn get_session_messages(
    runtime: &AgentRuntime,
    workspace_path: Option<&str>,
    chat_id: &str,
) -> Result<Vec<isanagent::utils::ChatMessage>, String> {
    let workspace_root = workspace_path
        .map(|p| format!("{}/.isanagent", p.trim_end_matches('/')))
        .unwrap_or_default();
    let memory_node = ensure_memory(runtime, &workspace_root).await?;

    let thread_id = SessionIdentity::parse(chat_id)
        .map_err(|error| error.to_string())?
        .legacy_tauri_thread_id();

    let (tx, rx) = tokio::sync::oneshot::channel();
    let reply = isanagent::memory::SharedReply::new(tx);
    memory_node
        .send_packet(isanagent::memory::MemoryMessage::GetContext { thread_id, reply })
        .await
        .map_err(|e| format!("Failed to query memory actor: {}", e))?;

    rx.await
        .map_err(|_| "Memory actor closed before replying".to_string())?
}

/// Rewind a chat's backend history to the N-th user message.
///
/// Sends `TruncateAfterUserMessage` to the per-workspace memory actor: keep
/// everything up to and including the `keep_user_messages`-th user-role row
/// (1-based, insert order), delete the rest. Returns the number of deleted
/// rows. `keep_user_messages == 0` wipes the whole thread.
///
/// This is the primitive powering frontend conversation edit / retry /
/// checkpoint-rollback — the backend owns the durable history, so the rewind
/// has to happen here. Tool-result cache rows for dropped tool_call_ids and
/// the thread's reflection/summary are cleared in the same transaction.
pub async fn truncate_after_user_message(
    runtime: &AgentRuntime,
    workspace_path: Option<&str>,
    chat_id: &str,
    keep_user_messages: usize,
) -> Result<usize, String> {
    let workspace_root = workspace_path
        .map(|p| format!("{}/.isanagent", p.trim_end_matches('/')))
        .unwrap_or_default();
    let memory_node = ensure_memory(runtime, &workspace_root).await?;

    let thread_id = SessionIdentity::parse(chat_id)
        .map_err(|error| error.to_string())?
        .legacy_tauri_thread_id();

    let (tx, rx) = tokio::sync::oneshot::channel();
    let reply = isanagent::memory::SharedReply::new(tx);
    memory_node
        .send_packet(isanagent::memory::MemoryMessage::TruncateAfterUserMessage {
            thread_id,
            keep_user_messages,
            reply,
        })
        .await
        .map_err(|e| format!("Failed to rewind memory actor: {}", e))?;

    rx.await
        .map_err(|_| "Memory actor closed before replying".to_string())?
}

async fn memory_for_workspace_path(
    runtime: &AgentRuntime,
    workspace_path: Option<&str>,
) -> Result<NodeHandle<isanagent::memory::MemoryMessage>, String> {
    let workspace_root = workspace_path
        .map(|path| format!("{}/.isanagent", path.trim_end_matches('/')))
        .unwrap_or_default();
    ensure_memory(runtime, &workspace_root).await
}

async fn request_memory<T>(
    memory_node: &NodeHandle<isanagent::memory::MemoryMessage>,
    build: impl FnOnce(
        isanagent::memory::SharedReply<Result<T, String>>,
    ) -> isanagent::memory::MemoryMessage,
) -> Result<T, String>
where
    T: Send + 'static,
{
    let (tx, rx) = tokio::sync::oneshot::channel();
    tokio::time::timeout(Duration::from_secs(5), async {
        memory_node
            .send_packet(build(isanagent::memory::SharedReply::new(tx)))
            .await
            .map_err(|error| format!("Failed to query memory actor: {error}"))?;
        rx.await
            .map_err(|_| "Memory actor closed before replying".to_string())?
            .map_err(|error| format!("Memory actor error: {error}"))
    })
    .await
    .map_err(|_| "Memory actor request timed out".to_string())?
}

pub fn validate_tauri_chat_id(chat_id: &str) -> Result<&str, String> {
    let chat_id = chat_id.trim();
    if chat_id.is_empty() {
        return Err("chatId is required".to_string());
    }
    if chat_id.len() > 256 {
        return Err("chatId is too long".to_string());
    }
    if chat_id.contains(':') {
        return Err("chatId contains an invalid delimiter".to_string());
    }
    Ok(chat_id)
}

fn automation_workspace_root(workspace_path: Option<&str>) -> String {
    workspace_path
        .map(|path| format!("{}/.isanagent", path.trim_end_matches('/')))
        .unwrap_or_default()
}

fn automation_store(workspace_root: &str) -> Result<CronStore, String> {
    let workspace_dir = if workspace_root.is_empty() {
        resolve_workspace_root(None)
    } else {
        resolve_workspace_root(Some(workspace_root))
    };
    let db_path = workspace_dir
        .join(".system_generated")
        .join("agent_memory.db");
    CronStore::new(
        db_path
            .to_str()
            .ok_or("workspace automation DB path is not valid UTF-8")?,
    )
}

/// List only ALTAI-owned root-chat automations in one authorized workspace.
/// The scheduler's cross-channel records and webhook secret never leave the
/// host process.
pub async fn list_automations(
    runtime: &AgentRuntime,
    workspace_path: Option<&str>,
) -> Result<Vec<AgentAutomationInfo>, String> {
    let workspace_root = automation_workspace_root(workspace_path);
    let _services = ensure_workspace_services(runtime, &workspace_root).await?;
    let mut jobs: Vec<_> = automation_store(&workspace_root)?
        .load_jobs()?
        .into_iter()
        .filter(|job| job.channel == "tauri" && validate_tauri_chat_id(&job.chat_id).is_ok())
        .map(|job| AgentAutomationInfo {
            id: job.id,
            schedule: job.schedule.into(),
            message: job.message,
            chat_id: job.chat_id,
            last_run_at_ms: job.last_run_at_ms,
        })
        .collect();
    jobs.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(jobs)
}

/// Add a direct-host automation. The destination is fixed to the current
/// ALTAI Tauri root chat; callers cannot select another transport/channel.
pub async fn create_automation(
    runtime: &AgentRuntime,
    workspace_path: Option<&str>,
    chat_id: &str,
    schedule: ScheduleKind,
    message: &str,
) -> Result<AgentAutomationInfo, String> {
    let chat_id = validate_tauri_chat_id(chat_id)?.to_string();
    let message = message.trim();
    if message.is_empty() {
        return Err("Automation message is required".to_string());
    }
    if message.len() > 10_000 {
        return Err("Automation message is too long".to_string());
    }
    let now_ms: i64 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "System clock is before the Unix epoch".to_string())?
        .as_millis()
        .try_into()
        .map_err(|_| "System clock is out of range".to_string())?;
    match &schedule {
        ScheduleKind::At { at_ms } if *at_ms <= now_ms => {
            return Err("One-time automation must be scheduled in the future".to_string())
        }
        ScheduleKind::Every { every_ms } if *every_ms < 60_000 => {
            return Err("Repeating automation interval must be at least one minute".to_string())
        }
        ScheduleKind::Every { every_ms } if *every_ms > 366 * 24 * 60 * 60 * 1_000 => {
            return Err("Repeating automation interval is too long".to_string())
        }
        ScheduleKind::Cron { .. } => {
            return Err(
                "Direct automations support one-time or repeating schedules only".to_string(),
            )
        }
        _ => {}
    }
    let workspace_root = automation_workspace_root(workspace_path);
    let services = ensure_workspace_services(runtime, &workspace_root).await?;
    let id = format!("altai:{}", uuid::Uuid::new_v4());
    let command = CronCommand::Add {
        id: id.clone(),
        schedule: schedule.clone(),
        message: message.to_string(),
        chat_id: chat_id.clone(),
        channel: "tauri".to_string(),
    };
    services
        .cron
        .node
        .send_packet(
            serde_json::to_string(&command)
                .map_err(|error| format!("Failed to serialize automation: {error}"))?,
        )
        .await
        .map_err(|error| format!("Failed to add automation: {error}"))?;
    Ok(AgentAutomationInfo {
        id,
        schedule: schedule.into(),
        message: message.to_string(),
        chat_id,
        last_run_at_ms: None,
    })
}

/// Remove an automation only after checking its persisted owner. A renderer
/// cannot use a schedule id from a different ALTAI conversation to remove it.
pub async fn remove_automation(
    runtime: &AgentRuntime,
    workspace_path: Option<&str>,
    chat_id: &str,
    automation_id: &str,
) -> Result<(), String> {
    let chat_id = validate_tauri_chat_id(chat_id)?;
    let automation_id = automation_id.trim();
    if automation_id.is_empty() || automation_id.len() > 512 {
        return Err("automationId is invalid".to_string());
    }
    let workspace_root = automation_workspace_root(workspace_path);
    let services = ensure_workspace_services(runtime, &workspace_root).await?;
    let job = automation_store(&workspace_root)?
        .find_job(automation_id)?
        .ok_or_else(|| "Automation was not found".to_string())?;
    if job.channel != "tauri" || job.chat_id != chat_id {
        return Err("Automation does not belong to this Tauri chat".to_string());
    }
    let command = CronCommand::Remove {
        id: automation_id.to_string(),
    };
    services
        .cron
        .node
        .send_packet(
            serde_json::to_string(&command)
                .map_err(|error| format!("Failed to serialize automation removal: {error}"))?,
        )
        .await
        .map_err(|error| format!("Failed to remove automation: {error}"))
}

fn is_tauri_root_identity(
    channel: &str,
    thread_id: Option<&str>,
    expected_chat_id: Option<&str>,
    actual_chat_id: &str,
) -> bool {
    channel == "tauri"
        && thread_id.is_none_or(str::is_empty)
        && expected_chat_id.is_none_or(|expected| expected == actual_chat_id)
}

pub async fn list_notifications(
    runtime: &AgentRuntime,
    workspace_path: Option<&str>,
    chat_id: Option<&str>,
    unseen_only: bool,
    limit: usize,
) -> Result<Vec<AgentNotificationInfo>, String> {
    let memory_node = memory_for_workspace_path(runtime, workspace_path).await?;
    list_notifications_with_memory(&memory_node, chat_id, unseen_only, limit).await
}

async fn list_notifications_with_memory(
    memory_node: &NodeHandle<isanagent::memory::MemoryMessage>,
    chat_id: Option<&str>,
    unseen_only: bool,
    limit: usize,
) -> Result<Vec<AgentNotificationInfo>, String> {
    let limit = limit.clamp(1, 500);
    let records = request_memory(memory_node, |reply| {
        isanagent::memory::MemoryMessage::ListNotifications {
            chat_id: chat_id.map(str::to_string),
            // The upstream query applies this trusted host boundary before its
            // SQL limit, so records from another channel cannot starve ALTAI's
            // workspace inbox.
            channel: Some("tauri".to_string()),
            limit,
            unseen_only,
            reply,
        }
    })
    .await?;
    Ok(records
        .into_iter()
        .filter(|record| {
            is_tauri_root_identity(
                &record.channel,
                record.thread_id.as_deref(),
                chat_id,
                &record.chat_id,
            )
        })
        .take(limit)
        .map(AgentNotificationInfo::from)
        .collect())
}

pub async fn mark_notification_seen(
    runtime: &AgentRuntime,
    workspace_path: Option<&str>,
    chat_id: &str,
    notification_id: &str,
) -> Result<(), String> {
    let memory_node = memory_for_workspace_path(runtime, workspace_path).await?;
    mark_notification_seen_with_memory(&memory_node, chat_id, notification_id).await
}

async fn mark_notification_seen_with_memory(
    memory_node: &NodeHandle<isanagent::memory::MemoryMessage>,
    chat_id: &str,
    notification_id: &str,
) -> Result<(), String> {
    let records = request_memory(memory_node, |reply| {
        isanagent::memory::MemoryMessage::ListNotifications {
            chat_id: Some(chat_id.to_string()),
            channel: Some("tauri".to_string()),
            limit: 500,
            unseen_only: false,
            reply,
        }
    })
    .await?;
    if !records.iter().any(|record| {
        record.notification_id == notification_id
            && is_tauri_root_identity(
                &record.channel,
                record.thread_id.as_deref(),
                Some(chat_id),
                &record.chat_id,
            )
    }) {
        return Err("Notification does not belong to this Tauri chat".to_string());
    }
    request_memory(memory_node, |reply| {
        isanagent::memory::MemoryMessage::MarkNotificationSeen {
            notification_id: notification_id.to_string(),
            reply,
        }
    })
    .await
}

pub async fn resolve_notification(
    runtime: &AgentRuntime,
    workspace_path: Option<&str>,
    chat_id: &str,
    notification_id: &str,
) -> Result<(), String> {
    let memory_node = memory_for_workspace_path(runtime, workspace_path).await?;
    resolve_notification_with_memory(&memory_node, chat_id, notification_id).await
}

async fn resolve_notification_with_memory(
    memory_node: &NodeHandle<isanagent::memory::MemoryMessage>,
    chat_id: &str,
    notification_id: &str,
) -> Result<(), String> {
    let records = request_memory(memory_node, |reply| {
        isanagent::memory::MemoryMessage::ListNotifications {
            chat_id: Some(chat_id.to_string()),
            channel: Some("tauri".to_string()),
            limit: 500,
            unseen_only: false,
            reply,
        }
    })
    .await?;
    let Some(record) = records.iter().find(|record| {
        record.notification_id == notification_id
            && is_tauri_root_identity(
                &record.channel,
                record.thread_id.as_deref(),
                Some(chat_id),
                &record.chat_id,
            )
    }) else {
        return Err("Notification does not belong to this Tauri chat".to_string());
    };
    if record.kind == "clarification_ticket" {
        return Err(
            "Clarification notifications must be dismissed through their ticket".to_string(),
        );
    }
    request_memory(memory_node, |reply| {
        isanagent::memory::MemoryMessage::ResolveNotification {
            notification_id: notification_id.to_string(),
            reply,
        }
    })
    .await
}

pub async fn list_background_jobs(
    runtime: &AgentRuntime,
    workspace_path: Option<&str>,
    chat_id: Option<&str>,
    limit: usize,
) -> Result<Vec<AgentBackgroundJobInfo>, String> {
    let memory_node = memory_for_workspace_path(runtime, workspace_path).await?;
    list_background_jobs_with_memory(&memory_node, chat_id, limit).await
}

async fn list_background_jobs_with_memory(
    memory_node: &NodeHandle<isanagent::memory::MemoryMessage>,
    chat_id: Option<&str>,
    limit: usize,
) -> Result<Vec<AgentBackgroundJobInfo>, String> {
    let limit = limit.clamp(1, 500);
    let records = request_memory(memory_node, |reply| {
        isanagent::memory::MemoryMessage::ListBackgroundJobs {
            chat_id: chat_id.map(str::to_string),
            channel: Some("tauri".to_string()),
            limit,
            reply,
        }
    })
    .await?;
    Ok(records
        .into_iter()
        .filter(|record| {
            is_tauri_root_identity(
                &record.channel,
                record.thread_id.as_deref(),
                chat_id,
                &record.chat_id,
            )
        })
        .take(limit)
        .map(AgentBackgroundJobInfo::from)
        .collect())
}

pub async fn dismiss_background_job(
    runtime: &AgentRuntime,
    workspace_path: Option<&str>,
    chat_id: &str,
    job_id: &str,
) -> Result<(), String> {
    let memory_node = memory_for_workspace_path(runtime, workspace_path).await?;
    dismiss_background_job_with_memory(&memory_node, chat_id, job_id).await
}

async fn dismiss_background_job_with_memory(
    memory_node: &NodeHandle<isanagent::memory::MemoryMessage>,
    chat_id: &str,
    job_id: &str,
) -> Result<(), String> {
    let records = request_memory(memory_node, |reply| {
        isanagent::memory::MemoryMessage::ListBackgroundJobs {
            chat_id: Some(chat_id.to_string()),
            channel: Some("tauri".to_string()),
            limit: 500,
            reply,
        }
    })
    .await?;
    if !records.iter().any(|record| {
        record.job_id == job_id
            && is_tauri_root_identity(
                &record.channel,
                record.thread_id.as_deref(),
                Some(chat_id),
                &record.chat_id,
            )
    }) {
        return Err("Background job does not belong to this Tauri chat".to_string());
    }
    request_memory(memory_node, |reply| {
        isanagent::memory::MemoryMessage::DismissBackgroundJob {
            job_id: Some(job_id.to_string()),
            ticket_id: None,
            reply,
        }
    })
    .await
}

pub async fn list_clarification_tickets(
    runtime: &AgentRuntime,
    workspace_path: Option<&str>,
    chat_id: Option<&str>,
    status: Option<&str>,
    limit: usize,
) -> Result<Vec<AgentClarificationTicketInfo>, String> {
    let memory_node = memory_for_workspace_path(runtime, workspace_path).await?;
    list_clarification_tickets_with_memory(&memory_node, chat_id, status, limit).await
}

async fn list_clarification_tickets_with_memory(
    memory_node: &NodeHandle<isanagent::memory::MemoryMessage>,
    chat_id: Option<&str>,
    status: Option<&str>,
    limit: usize,
) -> Result<Vec<AgentClarificationTicketInfo>, String> {
    let limit = limit.clamp(1, 500);
    let records = request_memory(memory_node, |reply| {
        isanagent::memory::MemoryMessage::ListClarificationTickets {
            job_id: None,
            chat_id: chat_id.map(str::to_string),
            channel: Some("tauri".to_string()),
            status: status.map(str::to_string),
            limit,
            reply,
        }
    })
    .await?;
    Ok(records
        .into_iter()
        .filter(|record| {
            is_tauri_root_identity(
                &record.channel,
                record.thread_id.as_deref(),
                chat_id,
                &record.chat_id,
            )
        })
        .take(limit)
        .map(AgentClarificationTicketInfo::from)
        .collect())
}

pub async fn dismiss_clarification_ticket(
    runtime: &AgentRuntime,
    workspace_path: Option<&str>,
    chat_id: &str,
    ticket_id: &str,
) -> Result<(), String> {
    let memory_node = memory_for_workspace_path(runtime, workspace_path).await?;
    dismiss_clarification_ticket_with_memory(&memory_node, chat_id, ticket_id).await
}

async fn dismiss_clarification_ticket_with_memory(
    memory_node: &NodeHandle<isanagent::memory::MemoryMessage>,
    chat_id: &str,
    ticket_id: &str,
) -> Result<(), String> {
    let record = request_memory(memory_node, |reply| {
        isanagent::memory::MemoryMessage::GetClarificationTicket {
            ticket_id: ticket_id.to_string(),
            reply,
        }
    })
    .await?
    .ok_or_else(|| "Clarification ticket was not found".to_string())?;
    if !is_tauri_root_identity(
        &record.channel,
        record.thread_id.as_deref(),
        Some(chat_id),
        &record.chat_id,
    ) {
        return Err("Clarification ticket does not belong to this Tauri chat".to_string());
    }
    if record.status != "waiting" {
        return Err("Clarification ticket is no longer waiting".to_string());
    }
    request_memory(memory_node, |reply| {
        isanagent::memory::MemoryMessage::DismissBackgroundJob {
            job_id: None,
            ticket_id: Some(ticket_id.to_string()),
            reply,
        }
    })
    .await
}

/// Build the only synthetic inbound shape ALTAI accepts for a persisted
/// clarification reply. The ticket lookup is an authorization check, not the
/// state transition: IsanAgent #66 atomically claims the waiting ticket when
/// the owning runtime processes this message.
async fn clarification_ticket_reply_inbound_with_memory(
    memory_node: &NodeHandle<isanagent::memory::MemoryMessage>,
    chat_id: &str,
    ticket_id: &str,
    response: &str,
) -> Result<isanagent::bus::InboundMessage, String> {
    let response = response.trim();
    if response.is_empty() {
        return Err("response is required".to_string());
    }
    if response.len() > 10_000 {
        return Err("response is too long".to_string());
    }

    let ticket = request_memory(memory_node, |reply| {
        isanagent::memory::MemoryMessage::GetClarificationTicket {
            ticket_id: ticket_id.to_string(),
            reply,
        }
    })
    .await?
    .ok_or_else(|| "Clarification ticket was not found".to_string())?;
    if !is_tauri_root_identity(
        &ticket.channel,
        ticket.thread_id.as_deref(),
        Some(chat_id),
        &ticket.chat_id,
    ) {
        return Err("Clarification ticket does not belong to this Tauri chat".to_string());
    }
    if ticket.status != "waiting" {
        return Err("Clarification ticket is no longer waiting".to_string());
    }

    let mut metadata = HashMap::new();
    metadata.insert(
        isanagent::bus::METADATA_CLARIFICATION_TICKET_ID.to_string(),
        serde_json::Value::String(ticket.ticket_id),
    );
    metadata.insert(
        isanagent::bus::METADATA_BACKGROUND_JOB_ID.to_string(),
        serde_json::Value::String(ticket.job_id),
    );
    metadata.insert(
        isanagent::bus::METADATA_SYNTHETIC_BACKGROUND_RESUME.to_string(),
        serde_json::Value::Bool(true),
    );
    Ok(isanagent::bus::InboundMessage {
        channel: "tauri".to_string(),
        sender_id: "altai_clarification_reply".to_string(),
        chat_id: chat_id.to_string(),
        thread_id: None,
        content: response.to_string(),
        attachments: Vec::new(),
        metadata,
    })
}

/// Submit a human response to a persisted background clarification. The
/// workspace dispatcher delivers it only to the runtime that most recently
/// served this chat; IsanAgent then atomically claims/resumes the ticket.
pub async fn reply_to_clarification_ticket(
    runtime: &AgentRuntime,
    workspace_path: Option<&str>,
    chat_id: &str,
    ticket_id: &str,
    response: &str,
) -> Result<(), String> {
    let chat_id = validate_tauri_chat_id(chat_id)?;
    let ticket_id = ticket_id.trim();
    if ticket_id.is_empty() {
        return Err("ticketId is required".to_string());
    }
    let memory_node = memory_for_workspace_path(runtime, workspace_path).await?;
    let inbound =
        clarification_ticket_reply_inbound_with_memory(&memory_node, chat_id, ticket_id, response)
            .await?;
    dispatch_synthetic_inbound(runtime, workspace_path, chat_id, inbound).await
}

/// Register the interaction and memory tools that already exist in IsanAgent
/// but were previously absent from ALTAI's embedded runtime.
///
/// Keep this helper dependency-only: lifecycle-owning capabilities such as the
/// cron actor and reflection engine are assembled at workspace scope instead
/// of being duplicated for every model/persona instance.
fn register_existing_claw_tools(
    tools: &mut ToolRegistry,
    memory_node: NodeHandle<isanagent::memory::MemoryMessage>,
    clarification_hub: Arc<ClarificationHub>,
    outbound_tx: mpsc::Sender<BusMessage>,
) {
    tools.register(Box::new(AskUserTool {
        clarification_hub,
        outbound_tx,
        memory_node: Some(memory_node.clone()),
    }));
    tools.register(Box::new(SearchMemoryTool {
        memory_node: memory_node.clone(),
    }));
    tools.register(Box::new(FetchMemoryByDateTool { memory_node }));
}

/// Build ONE IsanAgent instance: a fresh `TauriChannel` + agent node + bus
/// routers, sharing the passed-in (per-workspace) memory actor. Returns the
/// instance's channel. No teardown/fingerprint logic — the registry
/// (`ensure_instance`) owns instance lifecycle now.
///
/// `persona_instructions`, when `Some`, is appended to the system prompt under
/// a `## Persona` block. `base_url_override`, when `Some`, is the *full*
/// chat-completions endpoint POSTed as-is.
///
/// `permission_mode` ("ask" | "auto-edit" | "bypass"), when it maps to a shell
/// mode, overrides the interactive shell-policy gate so the UI permission
/// toggle actually governs code-exec / destructive-shell for this instance.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn build_instance(
    app: AppHandle,
    event_sink: Arc<dyn AgentEventSink>,
    memory_node: NodeHandle<isanagent::memory::MemoryMessage>,
    clarification_hub: Arc<ClarificationHub>,
    logger_handle: isanagent::logging::LoggerHandle,
    cron_node: NodeHandle<String>,
    event_journal: Arc<EventJournal>,
    run_coordinator: SharedRunCoordinator,
    provider_name: &str,
    api_key: &str,
    model_name: &str,
    persona_instructions: Option<&str>,
    base_url_override: Option<&str>,
    workspace_root: Option<&str>,
    permission_mode: Option<&str>,
    compaction: Option<&CompactionArg>,
    fallback: Option<&isanagent::agent::FallbackProviderSpec>,
) -> Result<
    (
        Arc<TauriChannel>,
        mpsc::Sender<BusMessage>,
        tokio::sync::oneshot::Sender<()>,
        tokio::task::JoinHandle<()>,
        tokio::task::JoinHandle<()>,
    ),
    String,
> {
    // Each instance gets its own channel with a unique bootstrap chat_id;
    // actual routing is by per-message chat_id.
    let owner_id = uuid::Uuid::new_v4().to_string();
    let channel = Arc::new(TauriChannel::new(
        event_sink.clone(),
        uuid::Uuid::new_v4().to_string(),
        owner_id.clone(),
        run_coordinator.clone(),
        event_journal.clone(),
    ));

    // Resolve workspace — `<selected-folder>/.isanagent`, or `~/.isanagent`.
    let workspace_dir = resolve_workspace_root(workspace_root);
    if !workspace_dir.exists() {
        // Auto-create minimal workspace
        let _ = std::fs::create_dir_all(workspace_dir.join(".system_generated"));
    }

    let workspace = IsanagentWorkspace::new(workspace_root, None)
        .map_err(|e| format!("Failed to load IsanAgent workspace: {}", e))?;

    // Memory (SQLite) is the shared per-workspace actor passed in by
    // `ensure_instance` — one actor per project, reused across this
    // workspace's model-instances so history transfers and DB access is
    // serialized through a single actor (no contention).
    let session_manager = SessionManager::new(memory_node.clone());
    let skills = SkillRegistry::new(workspace.skills_path());
    // Outbound channel for agent → UI (typed as BusMessage per IsanAgent API)
    let (global_outbound_tx, mut global_outbound_rx) = mpsc::channel::<BusMessage>(100);
    // Inbound bus
    let (bus_tx, mut bus_rx) = mpsc::channel::<BusMessage>(100);

    // Tools
    let mut tools = ToolRegistry::new();
    let restrict = workspace.config.restrict_to_workspace.unwrap_or(true);
    // Sandbox root is the selected project folder (the parent of `.isanagent`),
    // matching the industry-standard pattern used by Claude Code, Codex CLI, and
    // Cline: the agent operates on the project root, NOT a nested
    // `.isanagent/workspace` subfolder. `workspace.sandbox_dir` resolves to that
    // nested folder (isanagent crate default), so we override it here. We fall
    // back to the crate default only when the parent can't be resolved (e.g. the
    // `~/.isanagent` default with no project selected).
    let sandbox_dir = workspace_dir
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| workspace.sandbox_dir.clone());

    tools.register(Box::new(ReadFileTool {
        workspace_dir: sandbox_dir.clone(),
        restrict_to_workspace: restrict,
    }));
    tools.register(Box::new(WriteFileTool {
        workspace_dir: sandbox_dir.clone(),
        restrict_to_workspace: restrict,
    }));
    tools.register(Box::new(EditFileTool {
        workspace_dir: sandbox_dir.clone(),
        restrict_to_workspace: restrict,
    }));
    tools.register(Box::new(ListDirTool {
        workspace_dir: sandbox_dir.clone(),
        restrict_to_workspace: restrict,
    }));
    tools.register(Box::new(GlobFilesTool {
        workspace_dir: sandbox_dir.clone(),
        restrict_to_workspace: restrict,
    }));
    tools.register(Box::new(SearchTextTool {
        workspace_dir: sandbox_dir.clone(),
        restrict_to_workspace: restrict,
        ripgrep_timeout_secs: workspace
            .config
            .effective_search_text_ripgrep_timeout_secs(),
    }));
    tools.register(Box::new(ShellExecTool {
        workspace_dir: sandbox_dir.clone(),
        restrict_to_workspace: restrict,
    }));
    if workspace.config.git_worktree_tool_enabled() {
        tools.register(Box::new(GitWorktreeTool {
            workspace_dir: sandbox_dir.clone(),
            restrict_to_workspace: restrict,
            allow_path_outside_sandbox: workspace.config.git_worktree_allow_path_outside_sandbox(),
        }));
    }

    // Pre-edit checkpoints for one-step undo of agent edits (isanagent
    // #53/#56). WriteFileTool/EditFileTool snapshot a file's prior content
    // before mutating it; the `checkpoint` tool (and the `checkpoint_*` Tauri
    // commands) roll them back. isanagent's store is process-global and
    // set-once, while this runtime is rebuilt per workspace — so we root it at
    // an APP-level directory (not the workspace) and restore by absolute path
    // (`base = None`), which stays correct across workspace switches. Restores
    // are safe here because every checkpoint is created by our own sandboxed
    // edit tools. Trade-off: `base = None` forgoes isanagent's symlink/TOCTOU
    // restore guard (which only applies when a sandbox `base` is set) — an
    // acceptable choice since restores act only on agent-authored snapshots of
    // already sandbox-confined paths, and a single set-once `base` could not
    // stay correct once the workspace changes. Enabled by default; opt out with
    // `checkpoint_enabled = false` in `<workspace>/.isanagent/config.toml`.
    if workspace.config.checkpoint_enabled.unwrap_or(true) {
        // `init` sets a process-global set-once `OnceLock`. build_instance runs
        // again on every workspace/model switch, so only initialize when the
        // store isn't already up — a second `init` would allocate a throwaway
        // store the OnceLock silently drops. The app-level root means the first
        // init stays correct for the whole session regardless of workspace.
        if isanagent::checkpoint::store().is_none() {
            match app.path().app_data_dir() {
                Ok(data_dir) => isanagent::checkpoint::init(data_dir.join("checkpoints"), None),
                Err(e) => {
                    log::warn!("checkpoint: app data dir unavailable ({e}); edit undo disabled")
                }
            }
        }
        // Register the tool on every runtime build (each gets a fresh
        // ToolRegistry), but only while the store is actually active.
        if isanagent::checkpoint::store().is_some() {
            tools.register(Box::new(isanagent::checkpoint::CheckpointTool));
        }
    }

    // ML domain tools
    let max_web_chars = workspace.config.effective_max_web_tool_output_chars();
    let jina = workspace.config.jina_web_backend();
    tools.register(Box::new(WebSearchTool {
        jina: jina.clone(),
        max_output_chars: max_web_chars,
    }));
    tools.register(Box::new(WebFetchTool {
        jina,
        max_output_chars: max_web_chars,
        workspace_dir: workspace.dir.clone(),
    }));
    tools.register(Box::new(ArxivSearchTool {
        max_output_chars: max_web_chars,
    }));
    tools.register(Box::new(ArxivFetchTool {
        workspace_dir: workspace.dir.clone(),
    }));
    tools.register(Box::new(HfHubFileFetchTool {
        max_output_chars: max_web_chars,
    }));
    register_existing_claw_tools(
        &mut tools,
        memory_node.clone(),
        clarification_hub.clone(),
        global_outbound_tx.clone(),
    );
    let cron_db_path = workspace_dir
        .join(".system_generated")
        .join("agent_memory.db")
        .to_string_lossy()
        .to_string();
    // CronTool binds its destination to IsanAgent's trusted ToolExecCtx
    // (#67), while the actor itself is shared at workspace scope above.
    tools.register(Box::new(CronTool {
        cron_node,
        multi_tenant_edge_cron_enabled: false,
        mte_cron_scheduler: None,
        db_path: cron_db_path,
    }));
    tools.register(Box::new(TodoWriteTool {
        memory_node: memory_node.clone(),
    }));

    // User-configured MCP servers. Each successful server advertises its
    // tools through `tools/list`; those tools are registered like ALTAI's
    // built-ins and therefore participate in the model's native tool-calling
    // loop. One unavailable optional server must never prevent the agent from
    // starting, so failures are logged AND surfaced to the Settings UI via
    // the process-global MCP status registry.
    //
    // Servers connect CONCURRENTLY so a single slow server (e.g. an `npx`
    // spawn that takes 10s to initialize) doesn't gate every other server
    // behind it — total MCP startup latency is max(connect) rather than sum.
    // Each task owns its full Starting → Connected/Error status transition;
    // the registry mutex is fine for concurrent callers (brief critical
    // sections, no awaits held under the lock).
    let mcp_statuses = app.state::<mcp::McpStatusRegistry>();
    if let Ok(servers) = mcp::load_servers(&sandbox_dir) {
        let enabled: Vec<mcp::McpServerConfig> =
            servers.into_iter().filter(|s| s.enabled).collect();
        if !enabled.is_empty() {
            let mut connect_set = tokio::task::JoinSet::new();
            for server in enabled {
                let sandbox = sandbox_dir.clone();
                let statuses = mcp_statuses.inner().clone();
                connect_set.spawn(async move {
                    let now_ms_start = now_epoch_ms();
                    statuses
                        .set(
                            &sandbox,
                            mcp::McpServerStatus {
                                server_id: server.id.clone(),
                                state: mcp::McpState::Starting,
                                tool_count: None,
                                last_error: None,
                                updated_at_ms: now_ms_start,
                            },
                            now_ms_start,
                        )
                        .await;
                    let outcome = mcp::connect_server(&server, &sandbox).await;
                    (server, outcome)
                });
            }
            // Await every connect; registration happens on the main task so
            // `ToolRegistry` insertion order is deterministic (the registry is
            // name-keyed and safe under concurrent insert, but keeping it on
            // one task keeps the log lines ordered).
            while let Some(joined) = connect_set.join_next().await {
                let Ok((server, outcome)) = joined else {
                    continue;
                };
                match outcome {
                    Ok(mcp_tools) => {
                        let count = mcp_tools.len();
                        log::info!("MCP '{}' connected with {} tools", server.name, count);
                        let now_ms = now_epoch_ms();
                        mcp_statuses
                            .set(
                                &sandbox_dir,
                                mcp::McpServerStatus {
                                    server_id: server.id.clone(),
                                    state: mcp::McpState::Connected,
                                    tool_count: Some(count),
                                    last_error: None,
                                    updated_at_ms: now_ms,
                                },
                                now_ms,
                            )
                            .await;
                        for tool in mcp_tools {
                            tools.register(Box::new(tool));
                        }
                    }
                    Err(error) => {
                        let msg = error.to_string();
                        log::warn!("MCP '{}' unavailable: {msg}", server.name);
                        let now_ms = now_epoch_ms();
                        mcp_statuses
                            .set(
                                &sandbox_dir,
                                mcp::McpServerStatus {
                                    server_id: server.id.clone(),
                                    state: mcp::McpState::Error,
                                    tool_count: None,
                                    last_error: Some(msg),
                                    updated_at_ms: now_ms,
                                },
                                now_ms,
                            )
                            .await;
                    }
                }
            }
        }
    } else {
        // load_servers itself failed — log and continue. Doesn't block the
        // agent (built-in tools already registered above).
        log::warn!("MCP configuration skipped");
    }

    // Compaction overhaul (upstream isanagent — altaidevorg/isanagent#39). The agent can
    // now schedule a between-turns context compaction via `compact_context`
    // and re-fetch a tool result that fell out of the live context via
    // `recall_tool_result`. Both surface in the chat as their own tool
    // entries (TOOL_META in tool.tsx) so the user can see when compaction
    // ran and what got recalled.
    tools.register(Box::new(isanagent::tools::compact::CompactContextTool {
        outbound_tx: global_outbound_tx.clone(),
    }));
    tools.register(Box::new(isanagent::tools::recall::RecallToolResultTool {
        memory_node: memory_node.clone(),
        outbound_tx: global_outbound_tx.clone(),
    }));

    // Execution harness (if enabled)
    if workspace.config.execution_harness_enabled() {
        let harness = isanagent::execution::build_execution_harness(
            workspace.dir.clone(),
            sandbox_dir.clone(),
            restrict,
            &workspace.config,
        )
        .map_err(|e| format!("execution harness: {e}"))?;

        let execution_jobs = Arc::new(isanagent::execution::ExecutionJobManager::new(
            harness.clone(),
            global_outbound_tx.clone(),
            Some(bus_tx.clone()),
            workspace.config.execution_wake_on_job_terminal(),
        ));
        let inflight_sync = Arc::new(isanagent::execution::InflightSyncRegistry::new());

        tools.register(Box::new(
            isanagent::tools::execution::ExecutionSessionCreateTool {
                harness: harness.clone(),
            },
        ));
        tools.register(Box::new(isanagent::tools::execution::ExecutionRunTool {
            harness: harness.clone(),
            outbound_tx: global_outbound_tx.clone(),
            jobs: Some(execution_jobs.clone()),
            inflight: Some(inflight_sync.clone()),
        }));
        tools.register(Box::new(
            isanagent::tools::execution::ExecutionRunBackgroundTool {
                harness: harness.clone(),
                jobs: execution_jobs.clone(),
            },
        ));
        tools.register(Box::new(
            isanagent::tools::execution::ExecutionJobStatusTool {
                jobs: execution_jobs.clone(),
            },
        ));
        tools.register(Box::new(
            isanagent::tools::execution::ExecutionJobResultTool {
                jobs: execution_jobs.clone(),
                max_tool_output_chars: workspace
                    .config
                    .resolved_max_tool_output_chars()
                    .unwrap_or(3000),
            },
        ));
        tools.register(Box::new(
            isanagent::tools::execution::ExecutionJobListTool {
                jobs: execution_jobs.clone(),
            },
        ));
        tools.register(Box::new(
            isanagent::tools::execution::ExecutionJobCancelTool {
                jobs: execution_jobs.clone(),
            },
        ));
        tools.register(Box::new(
            isanagent::tools::execution::ExecutionArtifactListTool {
                harness: harness.clone(),
            },
        ));
        tools.register(Box::new(
            isanagent::tools::execution::ExecutionEnvInfoTool {
                harness: harness.clone(),
            },
        ));
        tools.register(Box::new(
            isanagent::tools::execution::ExecutionSessionCloseTool {
                harness: harness.clone(),
            },
        ));
        tools.register(Box::new(isanagent::tools::execution::ExecutionCancelTool {
            harness: harness.clone(),
        }));

        // Read background-job stdout/stderr line-by-line — lets the agent
        // inspect a long-running job's logs without fetching the full result.
        tools.register(Box::new(
            isanagent::tools::execution::ExecutionReadLogTool {
                jobs: execution_jobs.clone(),
                harness: harness.clone(),
            },
        ));

        // (The colab_mcp extra-tool-call proxy was removed upstream in
        // isanagent #47 "Colab CLI" — ColabMcpToolCallTool /
        // compile_colab_mcp_tool_allowlist and the related config accessors no
        // longer exist, so there's nothing to register here.)
    }

    if workspace.config.kernel_porting_harness_enabled() {
        isanagent::tools::kernel_porting::register_kernel_porting_tools(
            &mut tools,
            sandbox_dir.clone(),
            Arc::new(workspace.config.clone()),
        );
    }

    // Register discovery last so its shared catalog contains every concrete
    // tool available to this instance. This mirrors IsanAgent's reference
    // binary and lets the model find opt-in MCP, execution, worktree, and
    // Claw-parity tools without duplicating a static catalogue in ALTAI.
    let tool_catalog = tools.catalog_handle();
    tools.register(Box::new(ToolSearchTool {
        catalog: tool_catalog,
    }));

    // Provider — `base_url_override` (from the JS side, derived from the
    // active model) wins. Otherwise fall back to workspace config, then
    // to Gemini's `v1beta` as a last resort.
    //
    // Note: `cfg.resolved_base_url()` has shifted between `Option<String>`
    // and `Result<String, String>` across isanagent revisions. `.unwrap_or`
    // is defined on both, so this branch survives that drift without
    // pinning the crate.
    let resolved_base_url = if let Some(override_url) = base_url_override {
        override_url.to_string()
    } else {
        // Gemini's OpenAI-compatible chat-completions endpoint. The runtime
        // POSTs to `base_url` as-is (no path appended), so this must be the
        // *full* endpoint — `…/v1beta` alone would 404.
        let default =
            "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions".to_string();
        if let Some(ref cfg) = workspace.config.provider {
            cfg.resolved_base_url().unwrap_or(default)
        } else {
            default
        }
    };
    let llm_provider =
        provider::create_provider(provider_name, &resolved_base_url, api_key, model_name);
    let provider_credentials = isanagent::provider::ProviderCredentials {
        provider_name: provider_name.to_string(),
        base_url: resolved_base_url.clone(),
        api_key: api_key.to_string(),
        model_name: model_name.to_string(),
    };
    let fallback_providers = fallback
        .cloned()
        .map(|fallback| {
            isanagent::agent::build_fallback_specs(
                provider_name,
                &resolved_base_url,
                model_name,
                vec![fallback],
            )
        })
        .unwrap_or_default();
    // System prompt
    let mut system_prompt = workspace.compile_system_prompt();
    if workspace.config.ml_engineer_harness_enabled() {
        system_prompt.push_str("\n\n");
        system_prompt.push_str(isanagent::ml_engineer::HARNESS_OVERLAY);
    }
    if let Some(persona) = persona_instructions {
        system_prompt.push_str("\n\n## Persona\n\n");
        system_prompt.push_str(persona);
    }

    // Subagent prompt/summary fields are derived from the system prompt as it
    // stands *before* the named-agent catalog is appended below — subagents do
    // not spawn nested subagents, so they don't need the catalog. This mirrors
    // the ordering in isanagent's reference binary (src/main.rs).
    let subagent_system_prompt = if workspace.config.ml_engineer_subagent_research_overlay() {
        format!(
            "{}\n{}",
            system_prompt,
            isanagent::ml_engineer::SUBAGENT_RESEARCH_APPEND
        )
    } else {
        system_prompt.clone()
    };
    let harness_runtime_summary = workspace.config.runtime_harness_summary_lines().join("\n");
    let forbid_final_without_tools = workspace.config.ml_engineer_forbid_final_without_tools();

    // Named-agent registry (researcher / coder / evaluator, plus any defined in
    // `.isanagent/config.toml` under `[harness.agents.*]`). Fall back to the
    // crate's built-in defaults when none are configured. The catalog is
    // injected into the main agent's system prompt so the LLM knows which
    // specialized agents it can dispatch via `subagent_spawn`.
    let agent_defs = {
        let defs = workspace.config.agent_definitions();
        if defs.is_empty() {
            isanagent::agent::registry::default_agent_definitions()
        } else {
            defs
        }
    };
    let agent_registry = Arc::new(isanagent::agent::AgentRegistry::from_definitions(
        &agent_defs,
        &sandbox_dir,
    ));
    let agent_prompt_section = agent_registry.compile_agent_prompt_section();
    if !agent_prompt_section.is_empty() {
        system_prompt.push_str(&agent_prompt_section);
    }

    // Build the subagent harness params only when enabled in config
    // (`[harness.subagents] enabled = true`). When disabled, no subagent tools
    // are registered and no spawn can happen — but note this is *not* a total
    // no-op vs. the pre-subagent runtime: `harness_runtime_summary` (a per-step
    // harness snapshot) and the named-agent catalog are now always built into
    // the prompt regardless of this flag, matching isanagent's reference binary.
    // Subagent lifecycle telemetry (SubagentSpawned / SubagentFinished) is
    // emitted on `outbound_tx` and surfaced to the UI by the outbound router
    // below; wake-on-completion follow-ups ride `bus_tx`.
    let subagent = if workspace.config.subagent_harness_enabled() {
        Some(isanagent::agent::SubagentHarnessParams {
            cancel_children_on_parent_cancel: workspace
                .config
                .subagent_cancel_children_on_parent_cancel(),
            allowed_tools: workspace.config.subagent_allowed_tools_set().map(Arc::new),
            max_tasks: workspace.config.subagent_max_tasks(),
            max_wait_secs: workspace.config.subagent_max_wait_secs(),
            agent_registry: Some(agent_registry),
            wake_on_completion: workspace.config.subagent_wake_on_completion(),
            task_history_retention: workspace.config.subagent_task_history_retention(),
            bus_tx: Some(bus_tx.clone()),
            workspace_dir: sandbox_dir.clone(),
        })
    } else {
        None
    };

    // Match isanagent onboarding default (999). Peer agents (Claude Code, Cursor,
    // Codex) do not stop healthy work at a low turn ceiling; 50 felt like a crash.
    let max_iterations = workspace.config.resolved_max_iterations().unwrap_or(999);
    let max_tool_output_chars = workspace
        .config
        .resolved_max_tool_output_chars()
        .unwrap_or(3000);
    // Resolve compaction knobs. The user-facing prefs (auto/thresholdTokens/
    // tailTurns) flow in from JS; when absent we keep the isanagent crate's
    // built-in defaults so direct CLI/canonical callers aren't affected.
    let (max_recent_summaries, short_term_threshold_turns, short_term_threshold_tokens) =
        match compaction {
            Some(c) => c.to_logic_params(),
            None => (5, 20, 100_000),
        };
    // Start from the on-disk shell policy, then let the active UI permission
    // mode override BOTH the interactive shell gate and the file-edit gate.
    //
    // The two surfaces are mapped independently (see `permission_mode_to_*_mode`)
    // because their risk profiles differ: "auto-edit" auto-applies file changes
    // but still prompts for shell commands, while "plan" blocks edits entirely
    // but lets read-only shell runs through with approval. Without overriding
    // `interactive_edit_mode` here, edits would always fall back to the on-disk
    // default (`Ask`) and the toolbar toggle would silently do nothing for the
    // edit surface — which was the core ITEM 1 wiring gap.
    let mut shell_policy = workspace.config.resolved_shell_policy();
    if let Some(mode) = permission_mode_to_shell_mode(permission_mode) {
        shell_policy.interactive_mode = mode;
    }
    if let Some(mode) = permission_mode_to_edit_mode(permission_mode) {
        shell_policy.interactive_edit_mode = mode;
        // `unattended_*_mode` only matters for autonomous/background sessions,
        // but keep it in lockstep with the interactive setting so a background
        // turn doesn't silently use the on-disk default (which is `Deny`) while
        // the user picked `auto-edit` in the toolbar.
        shell_policy.unattended_edit_mode = mode;
    }
    let default_harness = isanagent::config::HarnessConfig::default();
    let harness_ref = workspace
        .config
        .harness
        .as_ref()
        .unwrap_or(&default_harness);
    let hook_tool_ctx = isanagent::hooks::ToolCallHookContext::from_harness_config(
        &workspace.dir,
        &sandbox_dir,
        harness_ref,
    );

    let agent_logic = AgentLogic::new_with_fallback_providers(
        AgentLogicParams {
            name: "altai-agent".to_string(),
            provider: llm_provider,
            provider_credentials,
            session_manager,
            tools,
            skills,
            system_prompt,
            max_iterations,
            max_tool_output_chars,
            max_recent_summaries,
            short_term_threshold_turns,
            short_term_threshold_tokens,
            outbound_tx: global_outbound_tx.clone(),
            logger_tx: logger_handle,
            clarification_hub,
            subagent,
            doom_loop_enabled: workspace.config.doom_loop_enabled(),
            harness_runtime_summary,
            subagent_system_prompt,
            forbid_final_without_tools,
            shell_policy,
            hook_tool_ctx,
        },
        fallback_providers,
    );

    let agent_node = NodeHandle::<BusMessage>::new(agent_logic, 100, 3, Duration::from_millis(50));

    // Start the TauriChannel
    channel
        .start(bus_tx.clone())
        .await
        .map_err(|e| format!("TauriChannel start failed: {}", e))?;

    // Bus router: forward inbound → agent, outbound → channel. (Telemetry is
    // emitted by the outbound router below, not here — see note in the loop.)
    let channel_for_outbound = channel.clone();
    // Shutdown trigger: `agent_node` (moved into this task) holds `bus_tx`
    // clones, so `channel.stop()` can't drop the last sender. On teardown we
    // fire `shutdown_tx`; the task breaks, drops `agent_node`, and the cycle
    // unwinds (its `global_outbound_tx` clones drop, ending the outbound task).
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let coordinator_for_bus = run_coordinator.clone();
    let owner_for_bus = owner_id.clone();
    let bus_router = tokio::spawn(async move {
        loop {
            let msg = tokio::select! {
                m = bus_rx.recv() => m,
                _ = &mut shutdown_rx => break,
            };
            let Some(msg) = msg else { break };
            match msg {
                BusMessage::Inbound(mut inbound) => {
                    let generated_run_id =
                        if inbound.channel == "tauri" && inbound_run_id(&inbound).is_none() {
                            inbound = trusted_tauri_inbound(inbound);
                            inbound_run_id(&inbound).map(str::to_string)
                        } else {
                            None
                        };
                    let run_id_for_rollback = inbound_run_id(&inbound).map(str::to_string);
                    if let Some(run_id) = generated_run_id.as_deref() {
                        if let Err(error) = queue_run(
                            &coordinator_for_bus,
                            &inbound.chat_id,
                            run_id,
                            &owner_for_bus,
                        ) {
                            log::warn!(
                                "Dropped internal synthetic inbound for chat {}: {}",
                                inbound.chat_id,
                                error
                            );
                            continue;
                        }
                    }
                    let chat_id = inbound.chat_id.clone();
                    let result = agent_node.send_packet(BusMessage::Inbound(inbound)).await;
                    if result.is_err() {
                        if let Some(run_id) = run_id_for_rollback.as_deref() {
                            rollback_run_admission(
                                &coordinator_for_bus,
                                &chat_id,
                                run_id,
                                &owner_for_bus,
                            );
                        }
                    }
                }
                BusMessage::Outbound(outbound) => {
                    let _ = channel_for_outbound.send(outbound).await;
                }
                // NOTE: telemetry is intentionally NOT handled here. Agent and
                // tool telemetry flows through `global_outbound_tx` (the
                // outbound router below) and is emitted there exactly once.
                // `bus_tx` only ever carries Inbound (user + synthetic
                // execution-job follow-ups) and Cancel, so handling Telemetry
                // here would be dead code today and a double-emit footgun if
                // anything later routed telemetry to this channel.
                BusMessage::Cancel(chat_id) => {
                    let _ =
                        coordinator_guard(&coordinator_for_bus).cancel_requested(&chat_id, None);
                    let _ = agent_node.send_packet(BusMessage::Cancel(chat_id)).await;
                }
                BusMessage::CancelRun { chat_id, run_id }
                    if coordinator_guard(&coordinator_for_bus)
                        .cancel_requested(&chat_id, Some(&run_id))
                        .is_ok() =>
                {
                    let _ = agent_node
                        .send_packet(BusMessage::CancelRun { chat_id, run_id })
                        .await;
                }
                BusMessage::Steer {
                    chat_id,
                    run_id,
                    content,
                } if coordinator_guard(&coordinator_for_bus)
                    .accepts_steer(&chat_id, &run_id, &owner_for_bus)
                    .is_ok() =>
                {
                    let _ = agent_node
                        .send_packet(BusMessage::Steer {
                            chat_id,
                            run_id,
                            content,
                        })
                        .await;
                }
                _ => {}
            }
        }
    });

    // Outbound router: forward everything the agent emits on its outbound
    // channel — final assistant messages AND telemetry (tool calls, thoughts,
    // progress). Previously this task only handled `Outbound`, so every
    // `BusMessage::Telemetry(...)` the AgentLogic emitted was silently
    // dropped — the UI saw no tool calls or thinking between "Sending to
    // ALTAI…" and the final answer.
    let sink_for_outbound = event_sink.clone();
    let coordinator_for_outbound = run_coordinator.clone();
    let journal_for_outbound = event_journal.clone();
    let owner_for_outbound = owner_id.clone();
    let outbound_router = tokio::spawn(async move {
        while let Some(out_msg) = global_outbound_rx.recv().await {
            match out_msg {
                BusMessage::Outbound(outbound) => {
                    // Clarifications (`ask_user`) ride on outbound metadata —
                    // surface them as a distinct event so the UI can render the
                    // preset choices as buttons. A normal reply resolves them.
                    //
                    // The crate's edit gate additionally attaches a structured
                    // `edit_diff` to the same outbound when the clarification is
                    // really a file-mutation approval. We extract it here so the
                    // frontend can render a diff-review card instead of the plain
                    // "approve / deny" chips — the reply path is identical.
                    let chat_id = outbound.chat_id.clone();
                    let is_clarification = outbound
                        .metadata
                        .contains_key(isanagent::clarification::METADATA_CLARIFICATION);
                    let edit_diff = outbound.metadata.get("edit_diff").and_then(parse_edit_diff);
                    let event = if is_clarification {
                        let choices = outbound
                            .metadata
                            .get(isanagent::clarification::METADATA_CLARIFICATION_CHOICES)
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|x| x.as_str().map(str::to_string))
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default();
                        Event::Clarification {
                            content: outbound.content,
                            choices,
                            edit_diff,
                        }
                    } else {
                        Event::AgentMessage {
                            content: outbound.content,
                            role: "assistant".to_string(),
                        }
                    };
                    let transition = persist_and_deliver_run_event(
                        &coordinator_for_outbound,
                        &journal_for_outbound,
                        &chat_id,
                        &owner_for_outbound,
                        &event,
                        RunEventTransition::Next,
                        |run, payload| {
                            // Waiting-user is runtime state, not renderer
                            // state. Commit it after durability even when the
                            // live window is gone and must replay the prompt.
                            if is_clarification {
                                if let Err(error) = coordinator_guard(&coordinator_for_outbound)
                                    .mark_waiting_user(&chat_id, &owner_for_outbound)
                                {
                                    log::warn!(
                                        "Could not mark clarification wait for chat {chat_id}: {error:?}"
                                    );
                                }
                            }
                            emit_payload(sink_for_outbound.as_ref(), &chat_id, payload, Some(run.clone()))
                        },
                    );
                    match transition {
                        Ok(_) => {}
                        Err(error @ RunEventDeliveryError::Renderer(_)) => {
                            log::warn!("Agent event for chat {chat_id} awaits replay: {error}")
                        }
                        Err(error) => {
                            log::warn!("Dropped outbound event for chat {chat_id}: {error}")
                        }
                    }
                }
                BusMessage::Telemetry(ref telemetry) => {
                    if let Some(event) = map_telemetry_to_event(telemetry) {
                        let chat_id = telemetry_chat_id(telemetry).unwrap_or("");
                        if is_system_event(&event) {
                            if let Err(error) =
                                emit_event(sink_for_outbound.as_ref(), chat_id, &event, None)
                            {
                                log::warn!("Could not deliver system event: {error}");
                            }
                        } else {
                            if let Err(error) = deliver_next_run_event(
                                sink_for_outbound.as_ref(),
                                &journal_for_outbound,
                                &coordinator_for_outbound,
                                chat_id,
                                &owner_for_outbound,
                                &event,
                            ) {
                                log::warn!("Dropped telemetry event for chat {chat_id}: {error}");
                            }
                        }
                    }
                }
                BusMessage::RunLifecycle(lifecycle) => {
                    use isanagent::bus::RunLifecycleEvent;

                    let event = map_lifecycle_to_event(&lifecycle);
                    match lifecycle {
                        RunLifecycleEvent::Started { run_id, chat_id } => {
                            let transition = persist_and_deliver_to_renderer(
                                sink_for_outbound.as_ref(),
                                &journal_for_outbound,
                                &coordinator_for_outbound,
                                &chat_id,
                                &owner_for_outbound,
                                &event,
                                RunEventTransition::Started(&run_id),
                            );
                            match transition {
                                Ok(_) => {}
                                Err(error) => log::warn!(
                                    "Could not persist or deliver run_started for chat {chat_id}: {error}"
                                ),
                            }
                        }
                        RunLifecycleEvent::Warning {
                            run_id, chat_id, ..
                        } => {
                            let transition = persist_and_deliver_to_renderer(
                                sink_for_outbound.as_ref(),
                                &journal_for_outbound,
                                &coordinator_for_outbound,
                                &chat_id,
                                &owner_for_outbound,
                                &event,
                                RunEventTransition::NextForRun(&run_id),
                            );
                            match transition {
                                Ok(_) => {}
                                Err(error) => log::warn!(
                                    "Could not persist or deliver run_warning for chat {chat_id}: {error}"
                                ),
                            }
                        }
                        RunLifecycleEvent::WarningCleared {
                            run_id, chat_id, ..
                        } => {
                            let transition = persist_and_deliver_to_renderer(
                                sink_for_outbound.as_ref(),
                                &journal_for_outbound,
                                &coordinator_for_outbound,
                                &chat_id,
                                &owner_for_outbound,
                                &event,
                                RunEventTransition::NextForRun(&run_id),
                            );
                            match transition {
                                Ok(_) => {}
                                Err(error) => log::warn!(
                                    "Could not persist or deliver run_warning_cleared for chat {chat_id}: {error}"
                                ),
                            }
                        }
                        RunLifecycleEvent::Terminated {
                            run_id, chat_id, ..
                        } => {
                            let transition = persist_and_deliver_to_renderer(
                                sink_for_outbound.as_ref(),
                                &journal_for_outbound,
                                &coordinator_for_outbound,
                                &chat_id,
                                &owner_for_outbound,
                                &event,
                                RunEventTransition::Terminated(&run_id),
                            );
                            match transition {
                                Ok(_) => {}
                                Err(error) => log::warn!(
                                    "Could not persist or deliver run_terminated for chat {chat_id}: {error}"
                                ),
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    });

    // Emit ready event under the runtime's bootstrap chat_id. It does not match
    // any ALTAI chat tab, so the frontend filters it out — it exists only as a
    // lifecycle signal, not a message to render in a user's chat.
    if let Err(error) = emit_event(
        event_sink.as_ref(),
        channel.chat_id(),
        &Event::AgentMessage {
            content: "IsanAgent runtime initialized.".to_string(),
            role: "system".to_string(),
        },
        None,
    ) {
        log::warn!("Could not deliver runtime bootstrap event: {error}");
    }

    Ok((channel, bus_tx, shutdown_tx, bus_router, outbound_router))
}

#[cfg(test)]
mod run_event_tests {
    use super::*;

    fn journal() -> (tempfile::TempDir, EventJournal) {
        let directory = tempfile::tempdir().expect("journal directory");
        let journal = EventJournal::open(directory.path().join("events.db")).expect("open journal");
        (directory, journal)
    }

    fn admitted_coordinator() -> SharedRunCoordinator {
        let coordinator = Arc::new(StdMutex::new(RunCoordinator::default()));
        coordinator_guard(&coordinator)
            .admit("chat-a", "run-1", "owner-1")
            .expect("admit run");
        coordinator
    }

    #[test]
    fn journal_append_precedes_delivery() {
        let (_directory, journal) = journal();
        let coordinator = admitted_coordinator();
        let event = Event::RunStarted {
            run_id: "run-1".to_string(),
        };

        persist_and_deliver_run_event(
            &coordinator,
            &journal,
            "chat-a",
            "owner-1",
            &event,
            RunEventTransition::Started("run-1"),
            |run, _| {
                let persisted = journal.fetch_after("run-1", 0, 10).expect("replay");
                assert_eq!(
                    (persisted[0].run_id.as_str(), persisted[0].seq),
                    (run.0.as_str(), run.1)
                );
                Ok(())
            },
        )
        .expect("persist and deliver");
    }

    #[test]
    fn delivery_failure_leaves_event_replayable() {
        let (_directory, journal) = journal();
        let coordinator = admitted_coordinator();
        let started = Event::RunStarted {
            run_id: "run-1".to_string(),
        };
        persist_run_event(
            &coordinator,
            &journal,
            "chat-a",
            "owner-1",
            &started,
            RunEventTransition::Started("run-1"),
        )
        .expect("start run");
        let message = Event::AgentMessage {
            content: "durable".to_string(),
            role: "assistant".to_string(),
        };

        let error = persist_and_deliver_run_event(
            &coordinator,
            &journal,
            "chat-a",
            "owner-1",
            &message,
            RunEventTransition::Next,
            |_, _| Err(RunEventDeliveryError::Renderer("unavailable".to_string())),
        )
        .expect_err("delivery must fail");

        assert!(matches!(error, RunEventDeliveryError::Renderer(_)));
        let replay = journal.fetch_after("run-1", 1, 10).expect("replay");
        assert_eq!(replay.len(), 1);
        assert_eq!(replay[0].seq, 2);
        assert_eq!(replay[0].payload["content"], "durable");
    }

    #[test]
    fn journal_failure_blocks_delivery_and_rolls_back_sequence() {
        let (_directory, journal) = journal();
        journal
            .append(&JournalEvent::now(
                1,
                "run-1",
                1,
                "other-chat",
                "run_started",
                serde_json::json!({"type":"run_started","run_id":"run-1"}),
            ))
            .expect("seed conflicting ownership");
        let coordinator = admitted_coordinator();
        let event = Event::RunStarted {
            run_id: "run-1".to_string(),
        };
        let delivered = std::cell::Cell::new(false);

        assert!(persist_and_deliver_run_event(
            &coordinator,
            &journal,
            "chat-a",
            "owner-1",
            &event,
            RunEventTransition::Started("run-1"),
            |_, _| {
                delivered.set(true);
                Ok(())
            },
        )
        .is_err());
        assert!(!delivered.get());
        assert_eq!(
            coordinator_guard(&coordinator).started("chat-a", "run-1", "owner-1"),
            Ok(("run-1".to_string(), 1))
        );
    }

    #[test]
    fn terminal_persistence_failure_keeps_the_cancelling_lease() {
        let (_directory, journal) = journal();
        let coordinator = admitted_coordinator();
        {
            let mut coordinator = coordinator_guard(&coordinator);
            coordinator
                .started("chat-a", "run-1", "owner-1")
                .expect("start run");
            coordinator
                .cancel_requested("chat-a", Some("run-1"))
                .expect("cancel run");
        }
        let delivered = std::cell::Cell::new(false);
        let terminal = Event::RunTerminated {
            run_id: "run-1".to_string(),
            outcome: serde_json::json!({ "kind": "cancelled" }),
        };

        // Sequence 1 was intentionally not persisted. The terminal append
        // must fail and restore the coordinator snapshot before the lock is
        // released, so a replacement cannot enter during cancellation unwind.
        assert!(persist_and_deliver_run_event(
            &coordinator,
            &journal,
            "chat-a",
            "owner-1",
            &terminal,
            RunEventTransition::Terminated("run-1"),
            |_, _| {
                delivered.set(true);
                Ok(())
            },
        )
        .is_err());
        assert!(!delivered.get());
        let mut coordinator = coordinator_guard(&coordinator);
        assert_eq!(coordinator.active_run("chat-a"), Some(("run-1", "owner-1")));
        assert_eq!(
            coordinator.admit("chat-a", "run-2", "owner-2"),
            Err(RunTransitionError::ActiveLease)
        );
    }

    #[test]
    fn terminal_transition_commits_event_and_summary_together() {
        let (_directory, journal) = journal();
        let coordinator = admitted_coordinator();
        persist_run_event(
            &coordinator,
            &journal,
            "chat-a",
            "owner-1",
            &Event::RunStarted {
                run_id: "run-1".to_string(),
            },
            RunEventTransition::Started("run-1"),
        )
        .expect("start run");
        persist_run_event(
            &coordinator,
            &journal,
            "chat-a",
            "owner-1",
            &Event::RunTerminated {
                run_id: "run-1".to_string(),
                outcome: serde_json::json!({"status":"completed"}),
            },
            RunEventTransition::Terminated("run-1"),
        )
        .expect("terminate run");

        let summary = journal
            .run_summary("run-1")
            .expect("summary")
            .expect("run summary");
        assert_eq!(summary.last_seq, 2);
        assert_eq!(summary.terminal_seq, Some(2));
        assert_eq!(summary.terminal_kind.as_deref(), Some("run_terminated"));
        assert_eq!(
            coordinator_guard(&coordinator).next("chat-a", "owner-1"),
            Err(RunTransitionError::MissingLease)
        );
    }

    #[test]
    fn replay_is_exclusive_ordered_and_chat_scoped() {
        let (_directory, journal) = journal();
        let coordinator = admitted_coordinator();
        for (event, transition) in [
            (
                Event::RunStarted {
                    run_id: "run-1".to_string(),
                },
                RunEventTransition::Started("run-1"),
            ),
            (
                Event::Thinking {
                    content: "step".to_string(),
                },
                RunEventTransition::Next,
            ),
            (
                Event::RunTerminated {
                    run_id: "run-1".to_string(),
                    outcome: serde_json::json!({"status":"completed"}),
                },
                RunEventTransition::Terminated("run-1"),
            ),
        ] {
            persist_run_event(
                &coordinator,
                &journal,
                "chat-a",
                "owner-1",
                &event,
                transition,
            )
            .expect("persist event");
        }

        let replay = replay_events_from_journal(&journal, "chat-a", "run-1", 1, 10)
            .expect("replay after acknowledged sequence");
        assert_eq!(
            replay.iter().map(|event| event.seq).collect::<Vec<_>>(),
            [2, 3]
        );
        assert!(replay_events_from_journal(&journal, "chat-b", "run-1", 0, 10).is_err());
        assert!(replay_events_from_journal(&journal, "chat-a", "unknown", 0, 10).is_err());
    }

    #[test]
    fn concurrent_replay_reads_are_identical_and_read_only() {
        let (_directory, journal) = journal();
        let journal = Arc::new(journal);
        journal
            .append(&JournalEvent::now(
                1,
                "run-1",
                1,
                "chat-a",
                "run_started",
                serde_json::json!({"type":"run_started","run_id":"run-1"}),
            ))
            .expect("seed event");

        let reads = std::thread::scope(|scope| {
            let left_journal = journal.clone();
            let right_journal = journal.clone();
            let left = scope.spawn(move || {
                replay_events_from_journal(&left_journal, "chat-a", "run-1", 0, 10)
                    .map(|events| serde_json::to_value(events).expect("serialize replay"))
            });
            let right = scope.spawn(move || {
                replay_events_from_journal(&right_journal, "chat-a", "run-1", 0, 10)
                    .map(|events| serde_json::to_value(events).expect("serialize replay"))
            });
            (
                left.join().expect("left replay").expect("left result"),
                right.join().expect("right replay").expect("right result"),
            )
        });

        assert_eq!(reads.0, reads.1);
        assert_eq!(
            journal
                .run_summary("run-1")
                .expect("summary")
                .expect("run")
                .last_seq,
            1
        );
    }

    #[test]
    fn restart_classifies_incomplete_runs_once_without_resuming_work() {
        let (_directory, journal) = journal();
        for event in [
            JournalEvent::now(
                1,
                "run-before-tool-end",
                1,
                "chat-a",
                "run_started",
                serde_json::json!({"type":"run_started","run_id":"run-before-tool-end"}),
            ),
            JournalEvent::now(
                1,
                "run-before-tool-end",
                2,
                "chat-a",
                "tool_call_start",
                serde_json::json!({"type":"tool_call_start","id":"tool-1","name":"edit_file","input":{}}),
            ),
            JournalEvent::now(
                1,
                "run-after-tool-end",
                1,
                "chat-b",
                "run_started",
                serde_json::json!({"type":"run_started","run_id":"run-after-tool-end"}),
            ),
            JournalEvent::now(
                1,
                "run-after-tool-end",
                2,
                "chat-b",
                "tool_call_end",
                serde_json::json!({"type":"tool_call_end","id":"tool-2","name":"read_file","output":"ok"}),
            ),
        ] {
            journal.append(&event).expect("seed incomplete run");
        }

        altai_agent_service::classify_runs_abandoned_by_restart(&journal)
            .expect("classify abandoned runs");
        altai_agent_service::classify_runs_abandoned_by_restart(&journal)
            .expect("repeat is a no-op");

        for (run_id, terminal_seq) in [("run-before-tool-end", 3), ("run-after-tool-end", 3)] {
            let summary = journal.run_summary(run_id).expect("summary").expect("run");
            assert_eq!(summary.last_seq, terminal_seq);
            assert_eq!(summary.terminal_seq, Some(terminal_seq));
            assert_eq!(
                summary.terminal_payload.as_ref().unwrap()["outcome"]["retryable"],
                false
            );
            assert_eq!(
                journal.fetch_after(run_id, 0, 10).expect("replay").len(),
                terminal_seq as usize
            );
        }
        assert!(journal.incomplete_run_summaries().unwrap().is_empty());
    }

    #[test]
    fn latest_chat_cursor_uses_durable_event_order_and_preserves_terminal() {
        let (_directory, journal) = journal();
        let mut old = JournalEvent::now(
            1,
            "run-old",
            1,
            "chat-a",
            "run_started",
            serde_json::json!({"type":"run_started","run_id":"run-old"}),
        );
        old.recorded_at_ms = 1;
        journal.append(&old).unwrap();
        let mut new = JournalEvent::now(
            1,
            "run-new",
            1,
            "chat-a",
            "run_terminated",
            serde_json::json!({
                "type":"run_terminated",
                "run_id":"run-new",
                "outcome":{"kind":"completed"}
            }),
        );
        new.recorded_at_ms = 2;
        journal.append_terminal(&new).unwrap();

        let latest = journal
            .latest_run_summary_for_chat("chat-a")
            .unwrap()
            .unwrap();
        assert_eq!(latest.run_id, "run-new");
        assert_eq!(latest.terminal_seq, Some(1));
        assert!(journal
            .latest_run_summary_for_chat("chat-b")
            .unwrap()
            .is_none());
    }

    #[test]
    fn sequence_is_monotonic_and_terminal_closes_only_the_matching_run() {
        let mut coordinator = RunCoordinator::default();
        coordinator
            .admit("chat-a", "run-1", "owner-1")
            .expect("admit first run");
        assert_eq!(
            coordinator.started("chat-a", "run-1", "owner-1"),
            Ok(("run-1".to_string(), 1))
        );
        assert_eq!(
            coordinator.next("chat-a", "owner-1"),
            Ok(("run-1".to_string(), 2))
        );
        assert_eq!(
            coordinator.terminated("chat-a", "other-run", "owner-1"),
            Err(RunTransitionError::RunMismatch)
        );
        assert_eq!(
            coordinator.terminated("chat-a", "run-1", "owner-1"),
            Ok(("run-1".to_string(), 3))
        );
        assert_eq!(
            coordinator.next("chat-a", "owner-1"),
            Err(RunTransitionError::MissingLease)
        );
    }

    #[test]
    fn stale_run_warning_cannot_consume_the_active_sequence() {
        let mut coordinator = RunCoordinator::default();
        coordinator
            .admit("chat-a", "run-1", "owner-1")
            .expect("admit run");
        coordinator
            .started("chat-a", "run-1", "owner-1")
            .expect("start run");

        assert_eq!(
            coordinator.next_for_run("chat-a", "stale-run", "owner-1"),
            Err(RunTransitionError::RunMismatch)
        );
        assert_eq!(
            coordinator.next_for_run("chat-a", "run-1", "owner-1"),
            Ok(("run-1".to_string(), 2))
        );
    }

    #[test]
    fn cancellation_keeps_lease_until_matching_terminal() {
        let mut coordinator = RunCoordinator::default();
        coordinator
            .admit("chat-a", "run-1", "owner-1")
            .expect("admit first run");
        coordinator
            .started("chat-a", "run-1", "owner-1")
            .expect("start first run");

        assert_eq!(
            coordinator.cancel_requested("chat-a", Some("run-1")),
            Ok("run-1".to_string())
        );
        assert_eq!(
            coordinator.admit("chat-a", "run-2", "owner-2"),
            Err(RunTransitionError::ActiveLease)
        );
        assert_eq!(
            coordinator.terminated("chat-a", "run-2", "owner-2"),
            Err(RunTransitionError::RunMismatch)
        );
        assert_eq!(
            coordinator.terminated("chat-a", "run-1", "owner-1"),
            Ok(("run-1".to_string(), 2))
        );
    }

    #[test]
    fn steering_accepts_only_the_exact_running_lease() {
        let mut coordinator = RunCoordinator::default();
        coordinator
            .admit("chat-a", "run-1", "owner-1")
            .expect("admit run");
        assert_eq!(
            coordinator.accepts_steer("chat-a", "run-1", "owner-1"),
            Err(RunTransitionError::InvalidPhase)
        );
        coordinator
            .started("chat-a", "run-1", "owner-1")
            .expect("start run");
        assert_eq!(
            coordinator.accepts_steer("chat-a", "stale", "owner-1"),
            Err(RunTransitionError::RunMismatch)
        );
        assert_eq!(
            coordinator.accepts_steer("chat-a", "run-1", "owner-2"),
            Err(RunTransitionError::OwnerMismatch)
        );
        assert_eq!(
            coordinator.accepts_steer("chat-a", "run-1", "owner-1"),
            Ok(())
        );
        coordinator
            .cancel_requested("chat-a", Some("run-1"))
            .expect("cancel run");
        assert_eq!(
            coordinator.accepts_steer("chat-a", "run-1", "owner-1"),
            Err(RunTransitionError::InvalidPhase)
        );
    }

    #[test]
    fn clarification_reply_preserves_the_active_run_identity() {
        let coordinator = Arc::new(StdMutex::new(RunCoordinator::default()));
        {
            let mut guard = coordinator_guard(&coordinator);
            guard
                .admit("chat-a", "run-1", "owner-1")
                .expect("admit run");
            guard
                .started("chat-a", "run-1", "owner-1")
                .expect("start run");
            guard
                .mark_waiting_user("chat-a", "owner-1")
                .expect("wait for user");
        }
        assert_eq!(
            admit_user_message(&coordinator, "chat-a", "reply-id", "owner-1"),
            Ok("run-1".to_string())
        );
        assert_eq!(
            coordinator_guard(&coordinator).next("chat-a", "owner-1"),
            Ok(("run-1".to_string(), 2))
        );
    }

    #[test]
    fn queued_run_is_promoted_only_after_terminal() {
        let mut coordinator = RunCoordinator::default();
        coordinator
            .admit("chat-a", "run-1", "owner-1")
            .expect("admit current run");
        coordinator
            .started("chat-a", "run-1", "owner-1")
            .expect("start current run");
        assert_eq!(
            coordinator.admit_or_queue("chat-a", "run-2", "owner-1"),
            Ok(RunAdmission::Queued)
        );
        assert_eq!(
            coordinator.started("chat-a", "run-2", "owner-1"),
            Err(RunTransitionError::RunMismatch)
        );
        coordinator
            .terminated("chat-a", "run-1", "owner-1")
            .expect("terminate current run");
        assert_eq!(
            coordinator.started("chat-a", "run-2", "owner-1"),
            Ok(("run-2".to_string(), 1))
        );
    }

    #[test]
    fn concurrent_admission_grants_exactly_one_chat_lease() {
        let coordinator = Arc::new(StdMutex::new(RunCoordinator::default()));
        let barrier = Arc::new(std::sync::Barrier::new(3));
        let mut handles = Vec::new();
        for index in 1..=2 {
            let coordinator = coordinator.clone();
            let barrier = barrier.clone();
            handles.push(std::thread::spawn(move || {
                barrier.wait();
                coordinator_guard(&coordinator).admit(
                    "chat-a",
                    &format!("run-{index}"),
                    &format!("owner-{index}"),
                )
            }));
        }
        barrier.wait();
        let outcomes: Vec<_> = handles
            .into_iter()
            .map(|handle| handle.join().expect("admission thread"))
            .collect();
        assert_eq!(outcomes.iter().filter(|outcome| outcome.is_ok()).count(), 1);
        assert_eq!(
            outcomes
                .iter()
                .filter(|outcome| **outcome == Err(RunTransitionError::ActiveLease))
                .count(),
            1
        );
    }

    #[test]
    fn envelope_is_versioned_and_carries_run_identity() {
        let event = Event::RunStarted {
            run_id: "run-1".to_string(),
        };
        let payload = redacted_event_payload(&event).expect("event payload");
        let envelope = AgentEventEnvelope::run("chat-a", "run-1", 1, payload);
        let value = serde_json::to_value(envelope).expect("serialize envelope");
        assert_eq!(value["version"], 1);
        assert_eq!(value["scope"], "run");
        assert_eq!(value["runId"], "run-1");
        assert_eq!(value["seq"], 1);
        assert_eq!(value["chatId"], "chat-a");
        assert_eq!(value["event"]["type"], "run_started");
    }

    #[test]
    fn event_boundary_redacts_secrets_identically_for_journal_and_renderer() {
        let (_directory, journal) = journal();
        let coordinator = admitted_coordinator();
        persist_run_event(
            &coordinator,
            &journal,
            "chat-a",
            "owner-1",
            &Event::RunStarted {
                run_id: "run-1".to_string(),
            },
            RunEventTransition::Started("run-1"),
        )
        .expect("start run");
        let recognizable_secret = "sk-abcdef0123456789ABCDEF";
        let bare_secret = "plain-private-value";
        let event = Event::ToolCallStart {
            id: "call-1".to_string(),
            name: "example".to_string(),
            input: serde_json::json!({
                "api_key": bare_secret,
                "command": format!("echo {recognizable_secret}"),
            }),
        };
        let mut delivered = None;

        persist_and_deliver_run_event(
            &coordinator,
            &journal,
            "chat-a",
            "owner-1",
            &event,
            RunEventTransition::Next,
            |_, payload| {
                delivered = Some(payload.clone());
                Ok(())
            },
        )
        .expect("persist and deliver redacted event");

        let delivered = delivered.expect("renderer payload");
        let journaled = journal
            .fetch_after("run-1", 1, 10)
            .expect("journal events")
            .pop()
            .expect("journal event")
            .payload;
        assert_eq!(delivered, journaled);
        let encoded = delivered.to_string();
        assert!(!encoded.contains(recognizable_secret), "{encoded}");
        assert!(!encoded.contains(bare_secret), "{encoded}");
        assert!(encoded.contains("REDACTED"), "{encoded}");
    }
}


#[cfg(test)]
mod provider_fingerprint_tests {
    use super::*;

    fn fallback(
        provider: &str,
        model: &str,
        api_key: &str,
    ) -> isanagent::agent::FallbackProviderSpec {
        isanagent::agent::FallbackProviderSpec {
            provider_name: provider.to_string(),
            base_url: format!("https://{provider}.test/v1"),
            api_key: api_key.to_string(),
            model_name: model.to_string(),
        }
    }

    fn fingerprint(
        primary_key: &str,
        fallback: Option<&isanagent::agent::FallbackProviderSpec>,
    ) -> altai_agent_service::RuntimeFingerprint {
        altai_agent_service::RuntimeFingerprint::make(
            "primary",
            primary_key,
            "primary-model",
            None,
            Some("https://primary.test/v1"),
            Some("/tmp/altai-provider-fingerprint-test"),
            Some("ask"),
            None,
            fallback,
        )
    }

    #[test]
    fn fingerprint_debug_uses_secret_identity_instead_of_raw_keys() {
        let fallback = fallback("backup", "backup-model", "fallback-secret-value");
        let runtime_fingerprint = fingerprint("primary-secret-value", Some(&fallback));
        let debug = format!("{runtime_fingerprint:?}");

        assert!(!debug.contains("primary-secret-value"), "{debug}");
        assert!(!debug.contains("fallback-secret-value"), "{debug}");
        assert!(debug.contains("sha256:"), "{debug}");
        assert_ne!(
            fingerprint("primary-secret-value", Some(&fallback)),
            fingerprint("different-primary-secret", Some(&fallback))
        );
    }

    #[test]
    fn runtime_fingerprint_distinguishes_fallback_configurations() {
        let fallback_a = fallback("backup-a", "model-a", "fallback-key-a");
        let fallback_b = fallback("backup-b", "model-b", "fallback-key-b");
        let (config_a, config_b) = std::thread::scope(|scope| {
            let config_a = scope.spawn(|| fingerprint("shared-primary-key", Some(&fallback_a)));
            let config_b = scope.spawn(|| fingerprint("shared-primary-key", Some(&fallback_b)));
            (
                config_a.join().expect("chat-a fingerprint"),
                config_b.join().expect("chat-b fingerprint"),
            )
        });

        let mut owners = HashMap::new();
        owners.insert(config_a.clone(), "chat-a");
        owners.insert(config_b.clone(), "chat-b");

        assert_ne!(config_a, config_b);
        assert_eq!(owners.len(), 2);
        assert_eq!(owners.get(&config_a), Some(&"chat-a"));
        assert_eq!(owners.get(&config_b), Some(&"chat-b"));
    }
}

#[cfg(test)]
mod claw_parity_tests {
    use super::*;

    fn record<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> T {
        serde_json::from_value(value).expect("valid IsanAgent record")
    }

    fn notification(
        id: &str,
        chat_id: &str,
        channel: &str,
        thread_id: Option<&str>,
        kind: &str,
    ) -> isanagent::memory::NotificationRecord {
        record(serde_json::json!({
            "notification_id": id,
            "chat_id": chat_id,
            "channel": channel,
            "thread_id": thread_id,
            "kind": kind,
            "title": format!("title-{id}"),
            "body": format!("body-{id}"),
            "action_kind": null,
            "action_payload": null,
            "seen_at_ms": null,
            "resolved_at_ms": null,
            "created_at_ms": 1,
        }))
    }

    fn background_job(
        id: &str,
        chat_id: &str,
        channel: &str,
        thread_id: Option<&str>,
        payload_json: &str,
    ) -> isanagent::memory::BackgroundJobRecord {
        record(serde_json::json!({
            "job_id": id,
            "kind": "cron",
            "chat_id": chat_id,
            "channel": channel,
            "thread_id": thread_id,
            "state": "waiting",
            "payload_json": payload_json,
            "resume_after_restart": true,
            "detached": true,
            "last_error": null,
            "created_at_ms": 1,
            "updated_at_ms": 1,
        }))
    }

    fn ticket(
        id: &str,
        job_id: &str,
        chat_id: &str,
        channel: &str,
        thread_id: Option<&str>,
        status: &str,
        choices_json: Option<&str>,
    ) -> isanagent::memory::ClarificationTicketRecord {
        record(serde_json::json!({
            "ticket_id": id,
            "job_id": job_id,
            "chat_id": chat_id,
            "channel": channel,
            "thread_id": thread_id,
            "tool_call_id": format!("tool-{id}"),
            "prompt": format!("prompt-{id}"),
            "choices_json": choices_json,
            "response": null,
            "status": status,
            "created_at_ms": 1,
            "updated_at_ms": 1,
        }))
    }

    fn memory_node(db_path: &std::path::Path) -> NodeHandle<isanagent::memory::MemoryMessage> {
        let memory_actor = isanagent::memory::SqliteMemoryActor::new(
            db_path.to_str().expect("utf-8 database path"),
        )
        .expect("memory actor");
        NodeHandle::<isanagent::memory::MemoryMessage>::new(
            memory_actor,
            100,
            1,
            Duration::from_millis(5),
        )
    }

    async fn seed_notification(
        memory_node: &NodeHandle<isanagent::memory::MemoryMessage>,
        record: isanagent::memory::NotificationRecord,
    ) {
        request_memory(memory_node, |reply| {
            isanagent::memory::MemoryMessage::InsertNotification { record, reply }
        })
        .await
        .expect("insert notification");
    }

    async fn seed_background_job(
        memory_node: &NodeHandle<isanagent::memory::MemoryMessage>,
        record: isanagent::memory::BackgroundJobRecord,
    ) {
        request_memory(memory_node, |reply| {
            isanagent::memory::MemoryMessage::UpsertBackgroundJob { record, reply }
        })
        .await
        .expect("insert background job");
    }

    async fn seed_ticket(
        memory_node: &NodeHandle<isanagent::memory::MemoryMessage>,
        record: isanagent::memory::ClarificationTicketRecord,
    ) {
        request_memory(memory_node, |reply| {
            isanagent::memory::MemoryMessage::UpsertClarificationTicket { record, reply }
        })
        .await
        .expect("insert clarification ticket");
    }

    #[tokio::test]
    async fn registers_existing_interaction_and_memory_tools() {
        let dir = tempfile::tempdir().expect("temp dir");
        let db_path = dir.path().join("memory.db");
        let memory_node = memory_node(&db_path);
        let (outbound_tx, _outbound_rx) = mpsc::channel(8);
        let mut tools = ToolRegistry::new();

        register_existing_claw_tools(
            &mut tools,
            memory_node,
            ClarificationHub::shared(),
            outbound_tx,
        );

        let names = tools.get_tool_names();
        for expected in ["ask_user", "search_memory", "fetch_memory_by_date"] {
            assert!(
                names.iter().any(|name| name == expected),
                "missing IsanAgent parity tool {expected}; registered: {names:?}"
            );
        }
        assert!(
            !names.iter().any(|name| name == "message"),
            "raw message must remain disabled until completion and destination contracts are safe"
        );
    }

    #[test]
    fn validates_opaque_tauri_chat_ids_and_root_identity() {
        assert_eq!(validate_tauri_chat_id("  s-chat-1  ").unwrap(), "s-chat-1");
        assert!(validate_tauri_chat_id("").is_err());
        assert!(validate_tauri_chat_id("tauri:chat:").is_err());
        assert!(validate_tauri_chat_id(&"x".repeat(257)).is_err());

        assert!(is_tauri_root_identity(
            "tauri",
            None,
            Some("chat-a"),
            "chat-a"
        ));
        assert!(is_tauri_root_identity(
            "tauri",
            Some(""),
            Some("chat-a"),
            "chat-a"
        ));
        assert!(!is_tauri_root_identity(
            "slack",
            None,
            Some("chat-a"),
            "chat-a"
        ));
        assert!(!is_tauri_root_identity(
            "tauri",
            Some("thread"),
            Some("chat-a"),
            "chat-a"
        ));
        assert!(!is_tauri_root_identity(
            "tauri",
            None,
            Some("chat-b"),
            "chat-a"
        ));
    }

    #[tokio::test]
    async fn persisted_facade_enforces_identity_and_mutation_guards() {
        let dir = tempfile::tempdir().expect("temp dir");
        let memory_node = memory_node(&dir.path().join("memory.db"));

        for record in [
            notification("notification-a", "chat-a", "tauri", None, "cron_triggered"),
            notification("notification-b", "chat-b", "tauri", None, "cron_triggered"),
            notification(
                "notification-slack",
                "chat-a",
                "slack",
                None,
                "cron_triggered",
            ),
            notification(
                "notification-thread",
                "chat-a",
                "tauri",
                Some("subthread"),
                "cron_triggered",
            ),
            notification(
                "notification-ticket",
                "chat-a",
                "tauri",
                None,
                "clarification_ticket",
            ),
        ] {
            seed_notification(&memory_node, record).await;
        }

        for record in [
            background_job(
                "job-a",
                "chat-a",
                "tauri",
                None,
                r#"{"secret":"keep-me-private"}"#,
            ),
            background_job("job-b", "chat-b", "tauri", None, "{}"),
            background_job("job-slack", "chat-a", "slack", None, "{}"),
            background_job("job-thread", "chat-a", "tauri", Some("subthread"), "{}"),
        ] {
            seed_background_job(&memory_node, record).await;
        }

        for record in [
            ticket(
                "ticket-a",
                "job-a",
                "chat-a",
                "tauri",
                None,
                "waiting",
                Some(r#"["one","two"]"#),
            ),
            ticket(
                "ticket-b", "job-b", "chat-b", "tauri", None, "waiting", None,
            ),
            ticket(
                "ticket-slack",
                "job-slack",
                "chat-a",
                "slack",
                None,
                "waiting",
                None,
            ),
            ticket(
                "ticket-thread",
                "job-thread",
                "chat-a",
                "tauri",
                Some("subthread"),
                "waiting",
                None,
            ),
            ticket(
                "ticket-answered",
                "job-a",
                "chat-a",
                "tauri",
                None,
                "answered",
                None,
            ),
        ] {
            seed_ticket(&memory_node, record).await;
        }

        let notifications =
            list_notifications_with_memory(&memory_node, Some("chat-a"), false, 500)
                .await
                .expect("list notifications");
        let mut notification_ids: Vec<_> =
            notifications.into_iter().map(|record| record.id).collect();
        notification_ids.sort();
        assert_eq!(
            notification_ids,
            [
                "notification-a".to_string(),
                "notification-ticket".to_string()
            ]
        );

        let jobs = list_background_jobs_with_memory(&memory_node, Some("chat-a"), 500)
            .await
            .expect("list background jobs");
        assert_eq!(
            jobs.iter()
                .map(|record| record.id.as_str())
                .collect::<Vec<_>>(),
            ["job-a"]
        );

        let tickets =
            list_clarification_tickets_with_memory(&memory_node, Some("chat-a"), None, 500)
                .await
                .expect("list clarification tickets");
        let mut ticket_ids: Vec<_> = tickets.into_iter().map(|record| record.id).collect();
        ticket_ids.sort();
        assert_eq!(
            ticket_ids,
            ["ticket-a".to_string(), "ticket-answered".to_string()]
        );

        assert!(
            mark_notification_seen_with_memory(&memory_node, "chat-a", "missing-notification",)
                .await
                .expect_err("unknown notification must be denied")
                .contains("does not belong")
        );
        assert!(
            mark_notification_seen_with_memory(&memory_node, "chat-a", "notification-b")
                .await
                .expect_err("wrong-chat notification must be denied")
                .contains("does not belong")
        );
        assert!(
            resolve_notification_with_memory(&memory_node, "chat-a", "notification-ticket")
                .await
                .expect_err("clarification notification must use ticket dismissal")
                .contains("through their ticket")
        );

        assert!(
            dismiss_background_job_with_memory(&memory_node, "chat-a", "missing-job")
                .await
                .expect_err("unknown job must be denied")
                .contains("does not belong")
        );
        assert!(
            dismiss_background_job_with_memory(&memory_node, "chat-a", "job-b")
                .await
                .expect_err("wrong-chat job must be denied")
                .contains("does not belong")
        );

        assert!(
            dismiss_clarification_ticket_with_memory(&memory_node, "chat-a", "missing-ticket",)
                .await
                .expect_err("unknown ticket must be denied")
                .contains("not found")
        );
        assert!(
            dismiss_clarification_ticket_with_memory(&memory_node, "chat-a", "ticket-b")
                .await
                .expect_err("wrong-chat ticket must be denied")
                .contains("does not belong")
        );
        assert!(
            dismiss_clarification_ticket_with_memory(&memory_node, "chat-a", "ticket-slack")
                .await
                .expect_err("wrong-channel ticket must be denied")
                .contains("does not belong")
        );
        assert!(
            dismiss_clarification_ticket_with_memory(&memory_node, "chat-a", "ticket-thread")
                .await
                .expect_err("subthread ticket must be denied")
                .contains("does not belong")
        );
        assert!(dismiss_clarification_ticket_with_memory(
            &memory_node,
            "chat-a",
            "ticket-answered",
        )
        .await
        .expect_err("answered ticket must be denied")
        .contains("no longer waiting"));

        let inbound = clarification_ticket_reply_inbound_with_memory(
            &memory_node,
            "chat-a",
            "ticket-a",
            "  one  ",
        )
        .await
        .expect("trusted ticket reply inbound");
        assert_eq!(inbound.channel, "tauri");
        assert_eq!(inbound.chat_id, "chat-a");
        assert_eq!(inbound.thread_id, None);
        assert_eq!(inbound.content, "one");
        assert_eq!(
            inbound
                .metadata
                .get(isanagent::bus::METADATA_CLARIFICATION_TICKET_ID)
                .and_then(|value| value.as_str()),
            Some("ticket-a")
        );
        assert_eq!(
            inbound
                .metadata
                .get(isanagent::bus::METADATA_BACKGROUND_JOB_ID)
                .and_then(|value| value.as_str()),
            Some("job-a")
        );
        assert!(clarification_ticket_reply_inbound_with_memory(
            &memory_node,
            "chat-a",
            "ticket-b",
            "one",
        )
        .await
        .expect_err("cross-chat ticket reply must be denied")
        .contains("does not belong"));
        assert!(clarification_ticket_reply_inbound_with_memory(
            &memory_node,
            "chat-a",
            "ticket-answered",
            "one",
        )
        .await
        .expect_err("answered ticket reply must be denied")
        .contains("no longer waiting"));
    }

    #[tokio::test]
    async fn channel_scoped_inbox_query_applies_before_the_limit() {
        let dir = tempfile::tempdir().expect("temp dir");
        let memory_node = memory_node(&dir.path().join("memory.db"));

        let mut tauri_record = notification(
            "tauri-notification",
            "chat-a",
            "tauri",
            None,
            "cron_triggered",
        );
        tauri_record.created_at_ms = 1;
        seed_notification(&memory_node, tauri_record).await;

        // These are all newer than the Tauri record. The former API adapter
        // fetched the newest 500 global records, filtered afterward, and
        // would return an empty ALTAI inbox here. The upstream channel filter
        // must execute in SQLite before `limit` is applied.
        for index in 0..501 {
            let mut record = notification(
                &format!("slack-notification-{index}"),
                "chat-a",
                "slack",
                None,
                "cron_triggered",
            );
            record.created_at_ms = i64::from(index) + 2;
            seed_notification(&memory_node, record).await;
        }

        let records = list_notifications_with_memory(&memory_node, None, false, 1)
            .await
            .expect("list notifications");
        assert_eq!(
            records
                .iter()
                .map(|record| record.id.as_str())
                .collect::<Vec<_>>(),
            ["tauri-notification"]
        );
    }

    #[test]
    fn clarification_dto_treats_malformed_choices_as_empty() {
        let dto = AgentClarificationTicketInfo::from(ticket(
            "ticket-malformed",
            "job-a",
            "chat-a",
            "tauri",
            None,
            "waiting",
            Some("{not-json"),
        ));
        assert!(dto.choices.is_empty());
    }

    #[test]
    fn background_job_dto_never_serializes_payload_json() {
        let dto = AgentBackgroundJobInfo::from(background_job(
            "job-secret",
            "chat-a",
            "tauri",
            None,
            r#"{"prompt":"TOP SECRET"}"#,
        ));
        let json = serde_json::to_value(dto).expect("serialize background job DTO");
        let encoded = serde_json::to_string(&json).expect("encode background job DTO");

        assert!(json.get("payloadJson").is_none());
        assert!(json.get("payload_json").is_none());
        assert!(!encoded.contains("TOP SECRET"));
    }

    #[tokio::test]
    async fn workspace_dispatcher_routes_only_an_explicitly_bound_chat() {
        let coordinator = Arc::new(StdMutex::new(RunCoordinator::default()));
        let dispatcher = WorkspaceDispatcher::new(coordinator);
        let (bus_tx, mut bus_rx) = mpsc::channel(1);
        dispatcher.bind("chat-a", bus_tx, "owner-a").await;

        let inbound = trusted_tauri_inbound(isanagent::bus::InboundMessage {
            channel: "tauri".to_string(),
            sender_id: "system".to_string(),
            chat_id: "chat-a".to_string(),
            thread_id: None,
            content: "Synthetic work".to_string(),
            attachments: Vec::new(),
            metadata: HashMap::new(),
        });
        dispatcher
            .dispatch("chat-a".to_string(), inbound)
            .await
            .expect("bound chat routes");

        let routed = bus_rx.recv().await.expect("inbound routed to owner");
        assert!(matches!(routed, BusMessage::Inbound(message) if message.chat_id == "chat-a"));

        let unbound = isanagent::bus::InboundMessage {
            channel: "tauri".to_string(),
            sender_id: "system".to_string(),
            chat_id: "chat-b".to_string(),
            thread_id: None,
            content: "Synthetic work".to_string(),
            attachments: Vec::new(),
            metadata: HashMap::new(),
        };
        assert!(dispatcher
            .dispatch("chat-b".to_string(), unbound)
            .await
            .expect_err("unbound chat must fail closed")
            .contains("No owning"));
    }

    #[tokio::test]
    async fn owner_bind_recovers_only_persisted_tauri_background_work() {
        let dir = tempfile::tempdir().expect("temp dir");
        let memory_node = memory_node(&dir.path().join("memory.db"));

        let mut recoverable = background_job(
            "cron:daily",
            "chat-a",
            "tauri",
            None,
            r#"{"message":"Run the daily briefing"}"#,
        );
        recoverable.state = "running".to_string();
        recoverable.resume_after_restart = true;
        let mut no_resume = background_job("job-no-resume", "chat-a", "tauri", None, "{}");
        no_resume.state = "running".to_string();
        no_resume.resume_after_restart = false;
        let mut foreign = background_job("job-foreign", "chat-b", "tauri", None, "{}");
        foreign.state = "running".to_string();
        foreign.resume_after_restart = true;
        for job in [recoverable, no_resume, foreign] {
            seed_background_job(&memory_node, job).await;
        }

        let coordinator = Arc::new(StdMutex::new(RunCoordinator::default()));
        let dispatcher = WorkspaceDispatcher::new(coordinator);
        let (bus_tx, mut bus_rx) = mpsc::channel(2);
        dispatcher.bind("chat-a", bus_tx, "owner-a").await;
        recover_background_jobs_after_owner_bind(&memory_node, &dispatcher, "chat-a")
            .await
            .expect("recover background work");

        let routed = bus_rx.recv().await.expect("one recovery inbound");
        let BusMessage::Inbound(inbound) = routed else {
            panic!("expected recovery inbound");
        };
        assert_eq!(inbound.chat_id, "chat-a");
        assert_eq!(inbound.content, "Run the daily briefing");
        assert_eq!(
            inbound
                .metadata
                .get(isanagent::bus::METADATA_BACKGROUND_JOB_ID)
                .and_then(|value| value.as_str()),
            Some("cron:daily")
        );
        assert!(
            tokio::time::timeout(Duration::from_millis(20), bus_rx.recv())
                .await
                .is_err()
        );
    }
}
