//! Host-specific seams injected into the shared agent service.
//!
//! Desktop (Tauri) and future stdio hosts implement this trait. The service
//! crate itself never imports `tauri` or UI crates.

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use isanagent::bus::BusMessage;
use isanagent::channels::Channel;
use isanagent::clarification::ClarificationHub;
use isanagent::logging::LoggerHandle;
use isanagent::tools::ToolRegistry;
use isanagent::NodeHandle;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use altai_core::journal::EventJournal;

use crate::compaction::CompactionArg;
use crate::sink::AgentEventSink;
use crate::SharedRunCoordinator;

/// Workspace-scoped actor handles required to build an instance.
pub struct WorkspaceBundle {
    pub memory_node: NodeHandle<isanagent::memory::MemoryMessage>,
    pub clarification_hub: Arc<ClarificationHub>,
    pub logger_handle: LoggerHandle,
    pub cron_node: NodeHandle<String>,
    pub event_journal: Arc<EventJournal>,
}

/// Inputs required to construct one long-lived IsanAgent instance.
pub struct BuildInstanceRequest<'a> {
    pub workspace: WorkspaceBundle,
    pub run_coordinator: SharedRunCoordinator,
    pub event_sink: Arc<dyn AgentEventSink>,
    pub provider_name: &'a str,
    pub api_key: &'a str,
    pub model_name: &'a str,
    pub persona_instructions: Option<&'a str>,
    pub base_url_override: Option<&'a str>,
    pub workspace_root: Option<&'a str>,
    pub permission_mode: Option<&'a str>,
    pub compaction: Option<&'a CompactionArg>,
    pub fallback: Option<&'a isanagent::agent::FallbackProviderSpec>,
}

/// Concrete instance pieces returned by [`HostAdapter::build_instance`].
pub struct BuiltInstance<C> {
    pub channel: Arc<C>,
    pub bus_tx: mpsc::Sender<BusMessage>,
    pub shutdown: tokio::sync::oneshot::Sender<()>,
    pub bus_router: JoinHandle<()>,
    pub outbound_router: JoinHandle<()>,
}

/// Desktop/stdio adapter contract for Tauri-free lifecycle ownership.
#[async_trait]
pub trait HostAdapter: Send + Sync + 'static {
    type Channel: Channel + Send + Sync + 'static;

    /// Stable owner id used by the run coordinator for this channel.
    fn channel_owner_id(channel: &Self::Channel) -> &str;

    /// Shared event sink used for delivery after journal append.
    fn event_sink(&self) -> Arc<dyn AgentEventSink>;

    /// Open or reuse workspace-owned actors/journal for `workspace_root`.
    async fn workspace_bundle(&self, workspace_root: &str) -> Result<WorkspaceBundle, String>;

    /// Drop host-held workspace bundles that are not `keep_root`.
    async fn retain_workspace_bundles(&self, keep_root: &str);

    /// Bind a chat to an instance after a successful user send (host may track
    /// dispatcher owners and recover background work). Default is a no-op.
    ///
    /// `is_first_bind` is true when this chat had no prior owner in the current
    /// process — Desktop uses that to recover persisted background jobs once.
    async fn on_chat_bound(
        &self,
        _workspace_root: &str,
        _chat_id: &str,
        _owner_id: &str,
        _bus_tx: mpsc::Sender<BusMessage>,
        _is_first_bind: bool,
    ) {
    }

    /// Build one IsanAgent instance (channel + routers + tools).
    ///
    /// MCP connect/status and checkpoint app-data roots stay host-specific and
    /// are performed inside this method.
    async fn build_instance(
        &self,
        request: BuildInstanceRequest<'_>,
    ) -> Result<BuiltInstance<Self::Channel>, String>;

    /// Clear MCP status badges for torn-down workspace roots.
    async fn clear_mcp_workspaces(&self, workspace_roots: &[String]);

    /// Optional hook after tools registry construction (default no-op).
    async fn augment_tools(
        &self,
        _sandbox_dir: &Path,
        _tools: &mut ToolRegistry,
    ) -> Result<(), String> {
        Ok(())
    }
}
