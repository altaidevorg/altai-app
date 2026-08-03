//! Long-lived agent lifecycle owned by the shared service.

use std::collections::HashSet;
use std::sync::Arc;

use isanagent::bus::BusMessage;
use tokio::sync::mpsc;

use crate::acks::{CancelAck, DocumentPart, ManualCompactionAck, SendAck, SteerAck};
use crate::compaction::CompactionArg;
use crate::host::{BuildInstanceRequest, HostAdapter, HostControlPlane};
use crate::instance::{stop_instance, Instance, RuntimeFingerprint};
use crate::routing::{coordinator_guard, RunCoordinator, SharedRunCoordinator};
use crate::AgentInstanceRegistry;

/// Sole owner of the long-lived IsanAgent instance registry and run coordinator.
pub struct AgentService<H: HostAdapter> {
    host: Arc<H>,
    instance_registry: AgentInstanceRegistry<RuntimeFingerprint, Instance<H::Channel>>,
    run_coordinator: SharedRunCoordinator,
}

impl<H: HostAdapter> AgentService<H> {
    pub fn new(host: Arc<H>) -> Self {
        Self::with_coordinator(
            host,
            Arc::new(std::sync::Mutex::new(RunCoordinator::default())),
        )
    }

    pub fn with_coordinator(host: Arc<H>, run_coordinator: SharedRunCoordinator) -> Self {
        Self {
            host,
            instance_registry: AgentInstanceRegistry::new(),
            run_coordinator,
        }
    }

    pub fn host(&self) -> &Arc<H> {
        &self.host
    }

    pub fn run_coordinator(&self) -> &SharedRunCoordinator {
        &self.run_coordinator
    }

    pub fn instance_registry(
        &self,
    ) -> &AgentInstanceRegistry<RuntimeFingerprint, Instance<H::Channel>> {
        &self.instance_registry
    }

    /// Warm up (or ensure) the instance for a config.
    #[allow(clippy::too_many_arguments)]
    pub async fn start_agent(
        &self,
        provider_name: &str,
        api_key: &str,
        model_name: &str,
        persona_instructions: Option<&str>,
        base_url_override: Option<&str>,
        workspace_path: Option<&str>,
        permission_mode: Option<&str>,
        compaction: Option<&CompactionArg>,
    ) -> Result<(), String> {
        self.ensure_instance(
            provider_name,
            api_key,
            model_name,
            persona_instructions,
            base_url_override,
            workspace_path,
            permission_mode,
            compaction,
            None,
        )
        .await
        .map(|_| ())
    }

    /// Ensure an instance exists for this config and return its channel.
    #[allow(clippy::too_many_arguments)]
    pub async fn ensure_instance(
        &self,
        provider_name: &str,
        api_key: &str,
        model_name: &str,
        persona_instructions: Option<&str>,
        base_url_override: Option<&str>,
        workspace_path: Option<&str>,
        permission_mode: Option<&str>,
        compaction: Option<&CompactionArg>,
        fallback: Option<&isanagent::agent::FallbackProviderSpec>,
    ) -> Result<Arc<H::Channel>, String> {
        let fp = RuntimeFingerprint::make(
            provider_name,
            api_key,
            model_name,
            persona_instructions,
            base_url_override,
            workspace_path,
            permission_mode,
            compaction,
            fallback,
        );
        let workspace_root = fp.workspace_root.clone();

        if let Some(channel) = self
            .instance_registry
            .with_instance(&fp, |instance| instance.channel.clone())
            .map_err(|error| error.to_string())?
        {
            return Ok(channel);
        }

        let stale_owner_ids: HashSet<String> = self
            .instance_registry
            .collect_matching(
                |key, _| key.workspace_root != workspace_root,
                |_, instance| H::channel_owner_id(&instance.channel).to_string(),
            )
            .map_err(|error| error.to_string())?
            .into_iter()
            .collect();
        coordinator_guard(&self.run_coordinator)
            .begin_draining(&stale_owner_ids)
            .map_err(|_| "Stop active agent runs before switching workspaces".to_string())?;

        let stale = self
            .instance_registry
            .take_matching(|key, _| key.workspace_root != workspace_root)
            .map_err(|error| error.to_string())?;
        let stale_workspace_roots: Vec<String> = stale
            .iter()
            .map(|(fp, _)| fp.workspace_root.clone())
            .collect();
        for (_, inst) in stale {
            stop_instance(inst).await;
        }
        coordinator_guard(&self.run_coordinator).end_draining(&stale_owner_ids);

        self.host.retain_workspace_bundles(&workspace_root).await;
        self.instance_registry
            .retain_chat_owners_for_workspace(&workspace_root)
            .map_err(|error| error.to_string())?;
        self.host
            .clear_mcp_workspaces(&stale_workspace_roots)
            .await;

        let workspace = self.host.workspace_bundle(&workspace_root).await?;
        let workspace_root_opt = if workspace_root.is_empty() {
            None
        } else {
            Some(workspace_root.as_str())
        };
        let built = self
            .host
            .build_instance(BuildInstanceRequest {
                workspace,
                run_coordinator: self.run_coordinator.clone(),
                event_sink: self.host.event_sink(),
                provider_name,
                api_key,
                model_name,
                persona_instructions,
                base_url_override,
                workspace_root: workspace_root_opt,
                permission_mode,
                compaction,
                fallback,
            })
            .await?;

        let channel = built.channel.clone();
        let candidate = Instance {
            channel: channel.clone(),
            bus_tx: built.bus_tx,
            shutdown: built.shutdown,
            bus_router: built.bus_router,
            outbound_router: built.outbound_router,
        };
        match self
            .instance_registry
            .insert_if_absent(fp.clone(), candidate)
            .map_err(|error| error.to_string())?
        {
            Ok(()) => Ok(channel),
            Err(loser) => {
                let winner = self
                    .instance_registry
                    .with_instance(&fp, |instance| instance.channel.clone())
                    .map_err(|error| error.to_string())?
                    .ok_or_else(|| "The concurrent agent runtime is unavailable".to_string())?;
                stop_instance(loser).await;
                Ok(winner)
            }
        }
    }

    /// Route a user message through the service-owned instance registry.
    #[allow(clippy::too_many_arguments)]
    pub async fn route_send(
        &self,
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
        documents: Vec<DocumentPart>,
        chat_id: String,
        queue: bool,
    ) -> Result<SendAck, String>
    where
        H::Channel: HostControlPlane,
    {
        let fingerprint = RuntimeFingerprint::make(
            provider_name,
            api_key,
            model_name,
            persona_instructions,
            base_url_override,
            workspace_path,
            permission_mode,
            compaction,
            fallback.as_ref(),
        );
        let channel = self
            .ensure_instance(
                provider_name,
                api_key,
                model_name,
                persona_instructions,
                base_url_override,
                workspace_path,
                permission_mode,
                compaction,
                fallback.as_ref(),
            )
            .await?;

        let acknowledgement = channel
            .inject_user_message(message, images, documents, chat_id.clone(), queue)
            .await?;
        if !chat_id.trim().is_empty() {
            let owner_id = H::channel_owner_id(&channel).to_string();
            let bus_tx = self
                .instance_registry
                .with_instance(&fingerprint, |instance| instance.bus_tx.clone())
                .map_err(|error| error.to_string())?
                .ok_or_else(|| "The owning agent runtime is no longer available".to_string())?;
            let previous_owner = self
                .instance_registry
                .bind_chat(
                    fingerprint.workspace_root.clone(),
                    chat_id.clone(),
                    fingerprint.clone(),
                )
                .map_err(|error| error.to_string())?;
            self.host
                .on_chat_bound(
                    &fingerprint.workspace_root,
                    &chat_id,
                    &owner_id,
                    bus_tx,
                    previous_owner.is_none(),
                )
                .await;
        }
        Ok(acknowledgement)
    }

    pub async fn route_cancel(&self, chat_id: String, run_id: String) -> Result<CancelAck, String>
    where
        H::Channel: HostControlPlane,
    {
        let (active_run_id, owner_id) = coordinator_guard(&self.run_coordinator)
            .active_run(&chat_id)
            .map(|(active_run_id, owner_id)| (active_run_id.to_string(), owner_id.to_string()))
            .ok_or_else(|| "No active agent run exists for this chat".to_string())?;
        if active_run_id != run_id {
            return Err("The requested agent run is no longer active".to_string());
        }
        let channel = self
            .instance_registry
            .find_instance(|instance| {
                (H::channel_owner_id(&instance.channel) == owner_id)
                    .then(|| instance.channel.clone())
            })
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "The owning agent runtime is unavailable".to_string())?;
        channel.cancel_run(chat_id.clone(), run_id.clone()).await?;
        Ok(CancelAck { chat_id, run_id })
    }

    pub async fn route_steer(
        &self,
        chat_id: String,
        run_id: String,
        content: String,
    ) -> Result<SteerAck, String>
    where
        H::Channel: HostControlPlane,
    {
        let content = content.trim().to_string();
        if content.is_empty() {
            return Err("Steering instructions cannot be empty".to_string());
        }
        let owner_id = {
            let coordinator = coordinator_guard(&self.run_coordinator);
            let (_, owner_id) = coordinator
                .active_run(&chat_id)
                .ok_or_else(|| "No active agent run exists for this chat".to_string())?;
            coordinator
                .accepts_steer(&chat_id, &run_id, owner_id)
                .map_err(|error| match error {
                    crate::RunTransitionError::RunMismatch => {
                        "The requested agent run is no longer active".to_string()
                    }
                    crate::RunTransitionError::InvalidPhase => {
                        "The active agent run cannot be steered in its current state".to_string()
                    }
                    _ => "The active agent run is unavailable".to_string(),
                })?;
            owner_id.to_string()
        };
        let channel = self
            .instance_registry
            .find_instance(|instance| {
                (H::channel_owner_id(&instance.channel) == owner_id)
                    .then(|| instance.channel.clone())
            })
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "The owning agent runtime is unavailable".to_string())?;
        channel
            .steer_run(chat_id.clone(), run_id.clone(), content)
            .await?;
        Ok(SteerAck { chat_id, run_id })
    }

    pub async fn route_manual_compaction(
        &self,
        workspace_path: &str,
        chat_id: String,
        focus_instructions: Option<String>,
    ) -> Result<ManualCompactionAck, String> {
        let focus_instructions = focus_instructions
            .map(|focus| focus.trim().to_string())
            .filter(|focus| !focus.is_empty());
        if focus_instructions
            .as_ref()
            .is_some_and(|focus| focus.len() > 4_000)
        {
            return Err("Compaction focus instructions are too long".to_string());
        }
        let workspace_root = format!("{}/.isanagent", workspace_path.trim_end_matches('/'));
        let fingerprint = self
            .instance_registry
            .chat_owner(&workspace_root, &chat_id)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| {
                "This chat has no active runtime in the current app session; send a message first"
                    .to_string()
            })?;
        let bus_tx = self
            .instance_registry
            .with_instance(&fingerprint, |instance| instance.bus_tx.clone())
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "The chat's agent runtime is unavailable".to_string())?;
        let coordinator = coordinator_guard(&self.run_coordinator);
        if coordinator.active_run(&chat_id).is_some() {
            return Err("Wait for the active run to finish before compacting context".to_string());
        }
        enqueue_manual_compaction(&bus_tx, &chat_id, focus_instructions)?;
        drop(coordinator);
        Ok(ManualCompactionAck { chat_id })
    }
}


fn enqueue_manual_compaction(
    bus_tx: &mpsc::Sender<BusMessage>,
    chat_id: &str,
    focus_instructions: Option<String>,
) -> Result<(), String> {
    let session_key = isanagent::bus::clarification_session_key("tauri", chat_id, None);
    bus_tx
        .try_send(BusMessage::TriggerCompaction {
            session_key,
            focus_instructions,
            trigger: Some(isanagent::bus::CompactionTrigger::Manual),
        })
        .map_err(|error| format!("Could not enqueue manual compaction: {error}"))
}
