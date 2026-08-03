//! Desktop HostAdapter — Tauri-specific seams for the shared AgentService.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use isanagent::bus::BusMessage;
use isanagent::clarification::ClarificationHub;
use isanagent::scheduler::{CronActor, CronSchedulingMode};
use isanagent::workspace::resolve_workspace_root;
use isanagent::{NodeHandle, Supervisor, SupervisorPolicy};
use tauri::async_runtime;
use tauri::{AppHandle, Manager};
use tokio::sync::mpsc;

use altai_agent_service::{
    build_shared_instance, AgentEventSink, BuildInstanceRequest, BuiltInstance, HostAdapter,
    ServiceChannel, SharedInstanceHooks, SharedRunCoordinator, WorkspaceBundle,
    WorkspaceServices as SharedWorkspaceServices,
};
use isanagent::tools::ToolRegistry;
use std::path::Path;

use super::runtime::{
    now_epoch_ms, recover_background_jobs_after_owner_bind, trusted_tauri_inbound,
    validate_tauri_chat_id, WorkspaceDispatcher,
};
use super::tauri_sink::TauriEventSink;
use crate::modules::mcp;

struct WorkspaceLogger {
    handle: isanagent::logging::LoggerHandle,
    #[allow(dead_code)]
    node: NodeHandle<BusMessage>,
    forwarder: std::sync::Mutex<Option<std::thread::JoinHandle<()>>>,
}

impl Drop for WorkspaceLogger {
    fn drop(&mut self) {
        if let Some(forwarder) = self.forwarder.lock().ok().and_then(|mut guard| guard.take()) {
            let _ = forwarder.join();
        }
    }
}

pub(crate) struct WorkspaceCron {
    pub node: NodeHandle<String>,
    #[allow(dead_code)]
    forwarder: async_runtime::JoinHandle<()>,
}

/// Services that must have exactly one owner for a workspace, independent of
/// how many provider/persona instances happen to serve that workspace.
pub(crate) struct DesktopWorkspaceServices {
    /// Durable paths/journal are opened and restart-classified by the shared,
    /// host-neutral service boundary. Desktop retains only its Tauri-specific
    /// actors and routes in this task.
    _shared: Arc<SharedWorkspaceServices>,
    pub memory_node: NodeHandle<isanagent::memory::MemoryMessage>,
    pub event_journal: Arc<altai_core::journal::EventJournal>,
    pub clarification_hub: Arc<ClarificationHub>,
    logger: WorkspaceLogger,
    pub dispatcher: Arc<WorkspaceDispatcher>,
    pub cron: WorkspaceCron,
}

/// Tauri host adapter. Owns workspace actor bundles; AgentService owns instances.
pub struct DesktopHost {
    app: AppHandle,
    run_coordinator: SharedRunCoordinator,
    workspace_services_by_root: tokio::sync::Mutex<HashMap<String, Arc<DesktopWorkspaceServices>>>,
}

impl DesktopHost {
    pub fn new(app: AppHandle, run_coordinator: SharedRunCoordinator) -> Self {
        Self {
            app,
            run_coordinator,
            workspace_services_by_root: tokio::sync::Mutex::new(HashMap::new()),
        }
    }

    pub async fn workspace_services(
        &self,
        workspace_root: &str,
    ) -> Result<Arc<DesktopWorkspaceServices>, String> {
        self.workspace_bundle_inner(workspace_root).await
    }

    async fn workspace_bundle_inner(
        &self,
        workspace_root: &str,
    ) -> Result<Arc<DesktopWorkspaceServices>, String> {
        let mut guard = self.workspace_services_by_root.lock().await;
        if let Some(existing) = guard.get(workspace_root) {
            return Ok(existing.clone());
        }
        let ws_opt = if workspace_root.is_empty() {
            None
        } else {
            Some(workspace_root)
        };
        let dir = resolve_workspace_root(ws_opt);
        let shared = Arc::new(
            SharedWorkspaceServices::open(&dir)
                .map_err(|error| format!("Failed to initialize workspace services: {error}"))?,
        );
        let db_path = shared.memory_db_path();
        let db_path_str = db_path
            .to_str()
            .ok_or("workspace DB path is not valid UTF-8")?;
        let event_journal = shared.event_journal();
        let memory_actor = isanagent::memory::SqliteMemoryActor::new(db_path_str)
            .map_err(|e| format!("Failed to initialize SqliteMemoryActor: {}", e))?;
        let node = NodeHandle::<isanagent::memory::MemoryMessage>::new(
            memory_actor,
            100,
            1,
            Duration::from_millis(5),
        );
        let (logger_handle, logger_rx) =
            isanagent::logging::create_logger_channel(isanagent::logging::LOGGER_QUEUE_CAPACITY);
        let logger_factory = {
            let workspace_dir = dir.clone();
            move || isanagent::logging::create_logging_actor_or_fallback(workspace_dir.clone())
        };
        let logger_node = NodeHandle::<BusMessage>::new(
            Supervisor::new(SupervisorPolicy::Restart, logger_factory),
            1_000,
            1,
            Duration::from_millis(10),
        );
        let logger_forward = logger_node.clone();
        let runtime_handle = tokio::runtime::Handle::current();
        let forwarder = std::thread::Builder::new()
            .name("altai-isanagent-logger".to_string())
            .spawn(move || {
                while let Ok(message) = logger_rx.recv() {
                    if runtime_handle
                        .block_on(logger_forward.send_packet(message))
                        .is_err()
                    {
                        break;
                    }
                }
            })
            .map_err(|error| format!("Failed to start workspace logger forwarder: {error}"))?;

        let dispatcher = Arc::new(WorkspaceDispatcher::new(self.run_coordinator.clone()));
        let (cron_bus_tx, mut cron_bus_rx) = mpsc::channel::<BusMessage>(100);
        let cron_logic = CronActor::new(
            "AltaiWorkspaceCron",
            db_path_str,
            logger_handle.clone(),
            CronSchedulingMode::Local,
            cron_bus_tx,
        )
        .map_err(|error| format!("Failed to initialize workspace cron actor: {error}"))?;
        let cron_node = NodeHandle::new(cron_logic, 100, 1, Duration::from_millis(50));
        let dispatcher_for_cron = dispatcher.clone();
        let cron_forwarder = async_runtime::spawn(async move {
            while let Some(message) = cron_bus_rx.recv().await {
                let BusMessage::Inbound(inbound) = message else {
                    continue;
                };
                let chat_id = inbound.chat_id.clone();
                if inbound.channel != "tauri"
                    || inbound.thread_id.is_some()
                    || validate_tauri_chat_id(&chat_id).is_err()
                {
                    log::warn!("Dropped cron delivery with an invalid ALTAI destination");
                    continue;
                }
                // A missing owner is expected after app restart. CronActor has
                // already persisted its running job, and `route_send` performs a
                // one-shot recovery when the user next reopens that conversation.
                if let Err(error) = dispatcher_for_cron
                    .dispatch(chat_id, trusted_tauri_inbound(inbound))
                    .await
                {
                    log::info!("Deferred cron delivery until its ALTAI chat is active: {error}");
                }
            }
        });

        let services = Arc::new(DesktopWorkspaceServices {
            _shared: shared,
            memory_node: node,
            event_journal,
            clarification_hub: ClarificationHub::shared(),
            logger: WorkspaceLogger {
                handle: logger_handle,
                node: logger_node,
                forwarder: std::sync::Mutex::new(Some(forwarder)),
            },
            dispatcher,
            cron: WorkspaceCron {
                node: cron_node,
                forwarder: cron_forwarder,
            },
        });
        guard.insert(workspace_root.to_string(), services.clone());
        Ok(services)
    }
}

#[async_trait]
impl HostAdapter for DesktopHost {
    type Channel = ServiceChannel;

    fn channel_owner_id(channel: &Self::Channel) -> &str {
        channel.owner_id()
    }

    fn event_sink(&self) -> Arc<dyn AgentEventSink> {
        Arc::new(TauriEventSink::new(self.app.clone()))
    }

    async fn workspace_bundle(&self, workspace_root: &str) -> Result<WorkspaceBundle, String> {
        let services = self.workspace_bundle_inner(workspace_root).await?;
        Ok(WorkspaceBundle {
            memory_node: services.memory_node.clone(),
            clarification_hub: services.clarification_hub.clone(),
            logger_handle: services.logger.handle.clone(),
            cron_node: services.cron.node.clone(),
            event_journal: services.event_journal.clone(),
        })
    }

    async fn retain_workspace_bundles(&self, keep_root: &str) {
        self.workspace_services_by_root
            .lock()
            .await
            .retain(|k, _| k == keep_root);
    }

    async fn on_chat_bound(
        &self,
        workspace_root: &str,
        chat_id: &str,
        owner_id: &str,
        bus_tx: mpsc::Sender<BusMessage>,
        is_first_bind: bool,
    ) {
        let Ok(services) = self.workspace_bundle_inner(workspace_root).await else {
            return;
        };
        services
            .dispatcher
            .bind(chat_id, bus_tx, owner_id)
            .await;
        if is_first_bind {
            if let Err(error) = recover_background_jobs_after_owner_bind(
                &services.memory_node,
                &services.dispatcher,
                chat_id,
            )
            .await
            {
                // The foreground message is already accepted. Recovery is a
                // best-effort side effect and must not turn that accepted send
                // into an IPC rejection that invites a duplicate retry.
                log::warn!(
                    "Could not recover persisted background work for chat {chat_id}: {error}"
                );
            }
        }
    }

    async fn build_instance(
        &self,
        request: BuildInstanceRequest<'_>,
    ) -> Result<BuiltInstance<Self::Channel>, String> {
        let checkpoint_root = self
            .app
            .path()
            .app_data_dir()
            .ok()
            .map(|dir| dir.join("checkpoints"));
        build_shared_instance(
            self,
            request,
            SharedInstanceHooks {
                checkpoint_root,
                scripted_responses: None,
                channel_name: "tauri",
            },
        )
        .await
    }

    async fn augment_tools(
        &self,
        sandbox_dir: &Path,
        tools: &mut ToolRegistry,
    ) -> Result<(), String> {
        let mcp_statuses = self.app.state::<mcp::McpStatusRegistry>();
        if let Ok(servers) = mcp::load_servers(sandbox_dir) {
            let enabled: Vec<mcp::McpServerConfig> =
                servers.into_iter().filter(|s| s.enabled).collect();
            if !enabled.is_empty() {
                let mut connect_set = tokio::task::JoinSet::new();
                for server in enabled {
                    let sandbox = sandbox_dir.to_path_buf();
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
                                    sandbox_dir,
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
                                    sandbox_dir,
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
            log::warn!("MCP configuration skipped");
        }
        Ok(())
    }

    async fn clear_mcp_workspaces(&self, workspace_roots: &[String]) {
        if let Some(mcp_statuses) = self.app.try_state::<mcp::McpStatusRegistry>() {
            for root in workspace_roots {
                if !root.is_empty() {
                    mcp_statuses
                        .clear_workspace(std::path::Path::new(root))
                        .await;
                }
            }
        }
    }
}
