//! Stdio HostAdapter — machine-facing seams for the shared AgentService.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use isanagent::bus::BusMessage;
use isanagent::clarification::ClarificationHub;
use isanagent::scheduler::{CronActor, CronSchedulingMode};
use isanagent::workspace::resolve_workspace_root;
use isanagent::{NodeHandle, Supervisor, SupervisorPolicy};
use tokio::sync::mpsc;

use altai_agent_service::{
    build_shared_instance, AgentEventSink, BuildInstanceRequest, BuiltInstance, HostAdapter,
    ServiceChannel, SharedInstanceHooks, SharedRunCoordinator, WorkspaceBundle,
    WorkspaceServices as SharedWorkspaceServices,
};
use altai_core::WorkspacePaths;

struct WorkspaceLogger {
    handle: isanagent::logging::LoggerHandle,
    #[allow(dead_code)]
    node: NodeHandle<BusMessage>,
    /// Detached on drop — joining here deadlocks because the forwarder blocks
    /// on `recv` until `handle` is dropped, and field drop order joins first.
    #[allow(dead_code)]
    forwarder: std::thread::JoinHandle<()>,
}

struct WorkspaceCron {
    node: NodeHandle<String>,
    #[allow(dead_code)]
    forwarder: tokio::task::JoinHandle<()>,
}

struct StdioWorkspaceServices {
    _shared: Arc<SharedWorkspaceServices>,
    memory_node: NodeHandle<isanagent::memory::MemoryMessage>,
    event_journal: Arc<altai_core::journal::EventJournal>,
    clarification_hub: Arc<ClarificationHub>,
    logger: WorkspaceLogger,
    cron: WorkspaceCron,
}

/// Stdio host adapter. Owns workspace actor bundles; AgentService owns instances.
pub struct StdioHost {
    workspace: WorkspacePaths,
    event_sink: Arc<dyn AgentEventSink>,
    workspace_services_by_root: tokio::sync::Mutex<HashMap<String, Arc<StdioWorkspaceServices>>>,
}

impl StdioHost {
    pub fn new(
        workspace: WorkspacePaths,
        event_sink: Arc<dyn AgentEventSink>,
        _run_coordinator: SharedRunCoordinator,
    ) -> Self {
        Self {
            workspace,
            event_sink,
            workspace_services_by_root: tokio::sync::Mutex::new(HashMap::new()),
        }
    }

    /// Deliver a reply only when this stdio chat has a live clarification wait.
    /// The hub removes the pending slot before delivery, making repeated replies
    /// fail rather than resuming the same tool twice.
    pub async fn deliver_clarification_reply(
        &self,
        chat_id: &str,
        text: String,
    ) -> Result<(), String> {
        let workspace_root = self.workspace.root.to_string_lossy().to_string();
        let services = self.workspace_bundle_inner(&workspace_root).await?;
        let session_key = isanagent::bus::clarification_session_key("stdio", chat_id, None);
        if services
            .clarification_hub
            .try_deliver_reply(&session_key, text)
        {
            Ok(())
        } else {
            Err("clarification_not_pending".to_string())
        }
    }

    /// Dismiss a live clarification exactly once.
    pub async fn dismiss_clarification(&self, chat_id: &str) -> Result<(), String> {
        let workspace_root = self.workspace.root.to_string_lossy().to_string();
        let services = self.workspace_bundle_inner(&workspace_root).await?;
        let session_key = isanagent::bus::clarification_session_key("stdio", chat_id, None);
        if services
            .clarification_hub
            .cancel_wait_if_pending(&session_key)
        {
            Ok(())
        } else {
            Err("clarification_not_pending".to_string())
        }
    }

    async fn workspace_bundle_inner(
        &self,
        workspace_root: &str,
    ) -> Result<Arc<StdioWorkspaceServices>, String> {
        let mut guard = self.workspace_services_by_root.lock().await;
        if let Some(existing) = guard.get(workspace_root) {
            return Ok(existing.clone());
        }
        let fallback = self.workspace.root.to_string_lossy();
        let ws_opt = if workspace_root.is_empty() {
            Some(fallback.as_ref())
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
            .map_err(|e| format!("Failed to initialize SqliteMemoryActor: {e}"))?;
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
            .name("altai-stdio-logger".to_string())
            .spawn(move || {
                while let Ok(message) = logger_rx.recv() {
                    // Exit when the runtime can no longer dispatch — otherwise
                    // this thread would spin forever after serve shutdown.
                    let send = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        runtime_handle.block_on(logger_forward.send_packet(message))
                    }));
                    match send {
                        Ok(Ok(())) => {}
                        Ok(Err(_)) | Err(_) => break,
                    }
                }
            })
            .map_err(|error| format!("Failed to start workspace logger forwarder: {error}"))?;

        let (cron_bus_tx, mut cron_bus_rx) = mpsc::channel::<BusMessage>(100);
        let cron_logic = CronActor::new(
            "AltaiStdioCron",
            db_path_str,
            logger_handle.clone(),
            CronSchedulingMode::Local,
            cron_bus_tx,
        )
        .map_err(|error| format!("Failed to initialize workspace cron actor: {error}"))?;
        let cron_node = NodeHandle::new(cron_logic, 100, 1, Duration::from_millis(50));
        let cron_forwarder = tokio::spawn(async move {
            while cron_bus_rx.recv().await.is_some() {
                // Cron delivery into long-lived stdio chats lands in a later slice.
            }
        });

        let services = Arc::new(StdioWorkspaceServices {
            _shared: shared,
            memory_node: node,
            event_journal,
            clarification_hub: ClarificationHub::shared(),
            logger: WorkspaceLogger {
                handle: logger_handle,
                node: logger_node,
                forwarder,
            },
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
impl HostAdapter for StdioHost {
    type Channel = ServiceChannel;

    fn channel_owner_id(channel: &Self::Channel) -> &str {
        channel.owner_id()
    }

    fn event_sink(&self) -> Arc<dyn AgentEventSink> {
        self.event_sink.clone()
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

    async fn build_instance(
        &self,
        request: BuildInstanceRequest<'_>,
    ) -> Result<BuiltInstance<Self::Channel>, String> {
        #[cfg(debug_assertions)]
        let scripted = std::env::var("ALTAI_CLI_TEST_SCRIPTED_RESPONSE")
            .ok()
            .map(|response| vec![response]);
        #[cfg(not(debug_assertions))]
        let scripted = None;
        let checkpoint_root = dirs_checkpoint_root();
        build_shared_instance(
            self,
            request,
            SharedInstanceHooks {
                checkpoint_root,
                scripted_responses: scripted,
                channel_name: "stdio",
            },
        )
        .await
    }

    async fn clear_mcp_workspaces(&self, _workspace_roots: &[String]) {}
}

fn dirs_checkpoint_root() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)?;
    let root = home.join(".altai").join("checkpoints");
    if let Err(error) = std::fs::create_dir_all(&root) {
        log::warn!("checkpoint: failed to create checkpoint directory: {error}");
        return None;
    }
    Some(root)
}

#[allow(dead_code)]
fn _path_marker(_: &Path) {}
