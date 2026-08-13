use std::io;
use std::sync::{Arc, Mutex};

use altai_agent_service::{
    coordinator_guard, AgentService, DocumentPart, RunCoordinator, SendAck, SharedRunCoordinator,
    WorkspaceServices,
};
use altai_agent_service::mcp::McpServerConfig;
use base64::Engine;
use altai_core::{
    AttemptReconcileMode, ConfigSource, EventJournal, ResolvedConfig, WorkspacePaths,
};
use altai_protocol::{
    validate_message, FrameDecoder, FrameLimits, ProtocolMessage, PROTOCOL_VERSION,
};
use serde_json::{json, Value};
use tokio::io::AsyncReadExt;

use crate::stdio_host::StdioHost;
use crate::stdio_sink::{write_framed, SharedStdout, StdioEventSink};

mod edit_proposals;
mod provider_credentials;
mod skills;
mod work;

use edit_proposals::{
    handle_apply as handle_proposal_apply, handle_deny as handle_proposal_deny,
    handle_list as handle_proposal_list, handle_upsert as handle_proposal_upsert,
    new_shared_store as new_edit_proposal_store, SharedEditProposalStore,
};

type Writer = SharedStdout;

const MAX_RUN_ATTACHMENTS: usize = 4;
const MAX_RUN_ATTACHMENT_ENCODED_BYTES: usize = 2 * 1024 * 1024;
const MAX_RUN_ATTACHMENTS_ENCODED_BYTES: usize = 3 * 1024 * 1024;

pub async fn run(workspace: WorkspacePaths) -> Result<(), String> {
    // Open and restart-classify the durable journal before stdin admission.
    // Memory/logger/cron actors stay lazy; every runtime bundle for the session
    // workspace reuses this exact classified service.
    let session_shared = Arc::new(
        WorkspaceServices::open(&workspace.isanagent_state)
            .map_err(|error| format!("could not initialize workspace journal: {error}"))?,
    );
    let work_journal = session_shared.event_journal();
    let writer: Writer = Arc::new(Mutex::new(io::stdout()));
    let event_sink = Arc::new(StdioEventSink::new(writer.clone()));
    let run_coordinator: SharedRunCoordinator =
        Arc::new(std::sync::Mutex::new(RunCoordinator::default()));
    let host = Arc::new(StdioHost::new(
        workspace.clone(),
        event_sink.clone() as Arc<dyn altai_agent_service::AgentEventSink>,
        run_coordinator.clone(),
        session_shared,
    ));
    let service = Arc::new(AgentService::with_coordinator(
        host.clone(),
        run_coordinator.clone(),
    ));
    let edit_proposals: SharedEditProposalStore = new_edit_proposal_store();

    let mut stdin = tokio::io::stdin();
    let mut decoder = FrameDecoder::new(FrameLimits::default());
    let mut initialized = false;
    let mut work_recovery_pending = true;
    let mut buffer = [0_u8; 4096];
    loop {
        let count = stdin.read(&mut buffer).await.map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        let frames = match decoder.push(&buffer[..count]) {
            Ok(frames) => frames,
            Err(error) => {
                eprintln!("altai-cli serve: malformed frame: {error}");
                cancel_all_active(&service).await;
                return Ok(());
            }
        };
        for frame in frames {
            let value: Value = match serde_json::from_slice(&frame) {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("altai-cli serve: malformed JSON: {error}");
                    cancel_all_active(&service).await;
                    return Ok(());
                }
            };
            let message = match validate_message(value.clone()) {
                Ok(message) => message,
                Err(error) => {
                    if let Some(id) = value.get("id").cloned() {
                        respond(
                            &writer,
                            id,
                            None,
                            Some(error_value(error.code as i32, error.reason)),
                        )
                        .await?;
                    }
                    continue;
                }
            };
            let ProtocolMessage::Request { id, method, params } = message else {
                continue;
            };
            match method.as_str() {
                "initialize" => {
                    initialized = true;
                    respond(
                        &writer,
                        id,
                        Some(json!({
                            "protocol_min": PROTOCOL_VERSION,
                            "protocol_max": PROTOCOL_VERSION,
                            "capabilities": [
                                "initialize",
                                "workspace/status",
                                "config/get",
                                "config/update",
                                "models/list",
                                "providers/status",
                                "providers/connect",
                                "providers/clear",
                                "mcp/servers/list", "mcp/servers/configure", "mcp/servers/enable", "mcp/servers/restart",
                                "skills/list",
                                "skills/install",
                                "work/list",
                                "work/children/list",
                                "work/get",
                                "work/create",
                                "work/transition",
                                "work/start",
                                "work/start-run",
                                "work/attempts/list",
                                "work/ready-for-review",
                                "work/review",
                                "work/inbox/list",
                                "work/tasks/list",
                                "work/tasks/create",
                                "work/tasks/cancel",
                                "work/tasks/retry",
                                "work/tasks/remove",
                                "work/automations/list",
                                "work/automations/create",
                                "work/automations/update",
                                "work/automations/trigger",
                                "work/automations/pause",
                                "work/automations/delete",
                                "agents/list",
                                "sessions/list",
                                "sessions/get",
                                "sessions/create",
                                "sessions/rename",
                                "sessions/archive",
                                "sessions/delete",
                                "sessions/messages",
                                "sessions/truncate",
                                "inbox/list",
                                "inbox/mark-seen",
                                "inbox/resolve",
                                "run/start",
                                "run/cancel",
                                "run/steer",
                                "run/retry",
                                "run/replay",
                                "clarification/respond",
                                "context/compact",
                                "checkpoints/list",
                                "checkpoints/restore",
                                "review/proposals/list",
                                "review/proposals/upsert",
                                "review/proposals/apply",
                                "review/proposals/deny",
                                "shutdown"
                            ]
                        })),
                        None,
                    )
                    .await?;
                }
                "workspace/status" if initialized => {
                    let journal_path = workspace.agent_event_journal_db();
                    let active_run = coordinator_guard(&run_coordinator)
                        .active_runs()
                        .into_iter()
                        .next()
                        .map(|(chat_id, run_id, _)| json!({"chat_id": chat_id, "run_id": run_id}));
                    respond(
                        &writer,
                        id,
                        Some(json!({
                            "root": workspace.root.display().to_string(),
                            "journal": journal_path.display().to_string(),
                            "active_run": active_run.as_ref().and_then(|v| v.get("run_id").cloned()),
                        })),
                        None,
                    )
                    .await?;
                }
                "sessions/create" if initialized => {
                    let params = params
                        .and_then(|value| value.as_object().cloned())
                        .unwrap_or_default();
                    let chat_id = params.get("chat_id").and_then(Value::as_str).unwrap_or("");
                    if chat_id.trim().is_empty() || chat_id.len() > 256 || chat_id.contains(':') {
                        respond(
                            &writer,
                            id,
                            None,
                            Some(error_value(-32602, "invalid_chat_id")),
                        )
                        .await?;
                        continue;
                    }
                    let title = params
                        .get("title")
                        .and_then(Value::as_str)
                        .unwrap_or("New chat");
                    if title.trim().is_empty() || title.len() > 256 {
                        respond(
                            &writer,
                            id,
                            None,
                            Some(error_value(-32602, "invalid_session_title")),
                        )
                        .await?;
                        continue;
                    }
                    let journal = match EventJournal::open(workspace.agent_event_journal_db()) {
                        Ok(journal) => journal,
                        Err(error) => {
                            eprintln!("altai-cli serve: could not open session journal: {error}");
                            respond(
                                &writer,
                                id,
                                None,
                                Some(error_value(-32603, "journal_unavailable")),
                            )
                            .await?;
                            continue;
                        }
                    };
                    match journal.create_session(chat_id, title.trim()) {
                        Ok(session) => {
                            respond(&writer, id, Some(session_metadata_value(session)), None)
                                .await?
                        }
                        Err(_) => {
                            respond(
                                &writer,
                                id,
                                None,
                                Some(error_value(-32602, "session_already_exists")),
                            )
                            .await?
                        }
                    }
                }
                "sessions/list" if initialized => {
                    let params = params
                        .and_then(|value| value.as_object().cloned())
                        .unwrap_or_default();
                    let limit = params.get("limit").and_then(Value::as_u64).unwrap_or(50);
                    if !(1..=200).contains(&limit) {
                        respond(
                            &writer,
                            id,
                            None,
                            Some(error_value(-32602, "invalid_session_limit")),
                        )
                        .await?;
                        continue;
                    }
                    let journal = match EventJournal::open(workspace.agent_event_journal_db()) {
                        Ok(journal) => journal,
                        Err(error) => {
                            eprintln!("altai-cli serve: could not open session journal: {error}");
                            respond(
                                &writer,
                                id,
                                None,
                                Some(error_value(-32603, "journal_unavailable")),
                            )
                            .await?;
                            continue;
                        }
                    };
                    match journal.list_session_metadata(limit as usize) {
                        Ok(sessions) => {
                            let sessions = sessions
                                .into_iter()
                                .map(|session| {
                                    let chat_id = session.chat_id.clone();
                                    let mut response = session_metadata_value(session);
                                    if let Ok(Some(summary)) =
                                        journal.latest_run_summary_for_chat(&chat_id)
                                    {
                                        if let Some(record) = response.as_object_mut() {
                                            record.insert(
                                                "latest_run_id".to_string(),
                                                json!(summary.run_id),
                                            );
                                            record.insert(
                                                "last_seq".to_string(),
                                                json!(summary.last_seq),
                                            );
                                            record.insert(
                                                "terminal_seq".to_string(),
                                                json!(summary.terminal_seq),
                                            );
                                        }
                                    }
                                    response
                                })
                                .collect::<Vec<_>>();
                            respond(&writer, id, Some(json!({"sessions": sessions})), None).await?;
                        }
                        Err(error) => {
                            eprintln!("altai-cli serve: could not list sessions: {error}");
                            respond(
                                &writer,
                                id,
                                None,
                                Some(error_value(-32603, "journal_unavailable")),
                            )
                            .await?;
                        }
                    }
                }
                "sessions/get" if initialized => {
                    let params = params
                        .and_then(|value| value.as_object().cloned())
                        .unwrap_or_default();
                    let chat_id = params.get("chat_id").and_then(Value::as_str).unwrap_or("");
                    if chat_id.trim().is_empty() {
                        respond(
                            &writer,
                            id,
                            None,
                            Some(error_value(-32602, "chat_id_required")),
                        )
                        .await?;
                        continue;
                    }
                    let journal = match EventJournal::open(workspace.agent_event_journal_db()) {
                        Ok(journal) => journal,
                        Err(error) => {
                            eprintln!("altai-cli serve: could not open session journal: {error}");
                            respond(
                                &writer,
                                id,
                                None,
                                Some(error_value(-32603, "journal_unavailable")),
                            )
                            .await?;
                            continue;
                        }
                    };
                    match journal.session_metadata(chat_id) {
                        Ok(Some(session)) => {
                            let mut response = session_metadata_value(session);
                            if let Ok(Some(summary)) = journal.latest_run_summary_for_chat(chat_id)
                            {
                                if let Some(record) = response.as_object_mut() {
                                    record
                                        .insert("latest_run_id".to_string(), json!(summary.run_id));
                                    record.insert("last_seq".to_string(), json!(summary.last_seq));
                                    record.insert(
                                        "terminal_seq".to_string(),
                                        json!(summary.terminal_seq),
                                    );
                                }
                            }
                            respond(&writer, id, Some(response), None).await?;
                        }
                        Ok(None) => {
                            respond(
                                &writer,
                                id,
                                None,
                                Some(error_value(-32002, "session_not_found")),
                            )
                            .await?;
                        }
                        Err(error) => {
                            eprintln!("altai-cli serve: could not inspect session: {error}");
                            respond(
                                &writer,
                                id,
                                None,
                                Some(error_value(-32603, "journal_unavailable")),
                            )
                            .await?;
                        }
                    }
                }
                "sessions/rename" | "sessions/archive" | "sessions/delete" if initialized => {
                    handle_session_mutation(
                        &host,
                        &workspace,
                        &run_coordinator,
                        &writer,
                        id,
                        method.as_str(),
                        params,
                    )
                    .await?;
                }
                "sessions/messages" if initialized => {
                    handle_session_messages(&host, &writer, id, params).await?;
                }
                "sessions/truncate" if initialized => {
                    handle_session_truncate(&host, &run_coordinator, &writer, id, params).await?;
                }
                "inbox/list" if initialized => {
                    let unseen_only = params
                        .as_ref()
                        .and_then(Value::as_object)
                        .and_then(|value| value.get("unseen_only"))
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    match host.list_notifications(unseen_only).await {
                        Ok(notifications) => {
                            respond(
                                &writer,
                                id,
                                Some(json!({"notifications": notifications})),
                                None,
                            )
                            .await?
                        }
                        Err(error) => {
                            eprintln!("altai-cli serve: could not list inbox: {error}");
                            respond(
                                &writer,
                                id,
                                None,
                                Some(error_value(-32603, "inbox_unavailable")),
                            )
                            .await?;
                        }
                    }
                }
                "inbox/mark-seen" | "inbox/resolve" if initialized => {
                    let notification_id = params
                        .as_ref()
                        .and_then(Value::as_object)
                        .and_then(|value| value.get("notification_id"))
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    let result = if method == "inbox/resolve" {
                        host.resolve_notification(notification_id).await
                    } else {
                        host.mark_notification_seen(notification_id).await
                    };
                    match result {
                        Ok(()) => {
                            respond(&writer, id, Some(json!({"accepted": true})), None).await?
                        }
                        Err(error) => {
                            respond(&writer, id, None, Some(error_value(-32002, &error))).await?
                        }
                    }
                }
                "agents/list" if initialized => {
                    respond(
                        &writer,
                        id,
                        Some(json!({"agents":[{"id":"altai","label":"ALTAI"}]})),
                        None,
                    )
                    .await?;
                }
                "models/list" if initialized => match load_run_configuration(&workspace) {
                    Ok(configuration) => {
                        let mut models = vec![json!({"id":"auto","label":"Auto"})];
                        if let Some(model) = configuration.model {
                            models.push(json!({"id": model.value, "label": model.value}));
                        }
                        if let Some(model) = configuration.fallback_model {
                            models.push(json!({"id": model.value, "label": model.value}));
                        }
                        respond(&writer, id, Some(json!({"models": models})), None).await?;
                    }
                    Err(error) => {
                        eprintln!("altai-cli serve: could not load model configuration: {error}");
                        respond(
                            &writer,
                            id,
                            None,
                            Some(error_value(-32603, "configuration_unavailable")),
                        )
                        .await?;
                    }
                },
                "providers/status" if initialized => match load_run_configuration(&workspace) {
                    Ok(configuration) => {
                        let provider_id = configuration
                            .provider
                            .as_ref()
                            .map(|value| value.value.clone())
                            .unwrap_or_else(|| "openai".to_string());
                        let route = secure_provider_route(
                            &provider_id,
                            configuration.provider.as_ref(),
                            configuration.base_url.as_ref(),
                        )
                        .ok();
                        // The native host exposes only the boolean outcome of
                        // credential resolution; raw keys never cross stdio.
                        let connected = route
                            .as_ref()
                            .and_then(|route| resolve_provider_credential(&provider_id, route).ok())
                            .is_some_and(|credential| !credential.trim().is_empty());
                        respond(
                            &writer,
                            id,
                            Some(json!({
                                "providers": [{
                                    "provider_id": provider_id,
                                    "label": "Configured provider",
                                    "connected": connected,
                                }]
                            })),
                            None,
                        )
                        .await?;
                    }
                    Err(error) => {
                        eprintln!(
                            "altai-cli serve: could not load provider configuration: {error}"
                        );
                        respond(
                            &writer,
                            id,
                            None,
                            Some(error_value(-32603, "configuration_unavailable")),
                        )
                        .await?;
                    }
                },
                "providers/connect" if initialized => {
                    let params = params
                        .and_then(|value| value.as_object().cloned())
                        .unwrap_or_default();
                    let provider_id = params
                        .get("provider_id")
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    let credential = params
                        .get("credential")
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    let base_url = params.get("base_url").and_then(Value::as_str);
                    if provider_credentials::validate_provider_id(provider_id).is_err()
                        || credential.trim().is_empty()
                        || credential.len() > 16 * 1024
                        || base_url.is_some_and(|value| !valid_base_url(value))
                    {
                        respond(
                            &writer,
                            id,
                            None,
                            Some(error_value(-32602, "invalid_provider_connection")),
                        )
                        .await?;
                        continue;
                    }
                    let previous_credential = match provider_credentials::get(provider_id) {
                        Ok(credential) => credential,
                        Err(error) => {
                            respond(&writer, id, None, Some(error_value(-32603, &error))).await?;
                            continue;
                        }
                    };
                    if let Err(error) = provider_credentials::set(provider_id, credential) {
                        respond(&writer, id, None, Some(error_value(-32603, &error))).await?;
                        continue;
                    }
                    let mut patch = serde_json::Map::new();
                    patch.insert("provider".to_string(), Value::String(provider_id.trim().to_string()));
                    if let Some(base_url) = base_url {
                        patch.insert("base_url".to_string(), Value::String(base_url.trim().to_string()));
                    }
                    match update_workspace_config(&workspace, &patch) {
                        Ok(()) => {
                            respond(
                                &writer,
                                id,
                                Some(json!({"provider_id": provider_id.trim(), "connected": true})),
                                None,
                            )
                            .await?;
                        }
                        Err(error) => {
                            // Do not leave a newly entered credential active
                            // when the associated non-secret provider config
                            // could not be persisted.
                            let _ = match previous_credential {
                                Some(previous) => provider_credentials::set(provider_id, &previous),
                                None => provider_credentials::delete(provider_id),
                            };
                            respond(&writer, id, None, Some(error_value(-32603, &error))).await?;
                        }
                    }
                }
                "providers/clear" if initialized => {
                    let provider_id = params
                        .as_ref()
                        .and_then(Value::as_object)
                        .and_then(|params| params.get("provider_id"))
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    if provider_credentials::validate_provider_id(provider_id).is_err() {
                        respond(
                            &writer,
                            id,
                            None,
                            Some(error_value(-32602, "invalid_provider_id")),
                        )
                        .await?;
                        continue;
                    }
                    match provider_credentials::delete(provider_id) {
                        Ok(()) => {
                            respond(
                                &writer,
                                id,
                                Some(json!({"provider_id": provider_id.trim(), "cleared": true})),
                                None,
                            )
                            .await?;
                        }
                        Err(error) => {
                            respond(&writer, id, None, Some(error_value(-32603, &error))).await?;
                        }
                    }
                }
                "mcp/servers/list" if initialized => match host.list_mcp_servers().await {
                    Ok((servers, statuses)) => {
                        let status_by_id: std::collections::HashMap<_, _> = statuses.into_iter().map(|status| (status.server_id.clone(), status)).collect();
                        let servers = servers.into_iter().map(|server| { let status = status_by_id.get(&server.id); json!({"id": server.id, "name": server.name, "enabled": server.enabled, "connected": status.is_some_and(|status| status.state == altai_agent_service::mcp::McpState::Connected), "error": status.and_then(|status| status.last_error.clone())}) }).collect::<Vec<_>>();
                        respond(&writer, id, Some(json!({"servers": servers})), None).await?;
                    }
                    Err(error) => respond(&writer, id, None, Some(error_value(-32603, &error))).await?,
                },
                "mcp/servers/configure" if initialized => {
                    let Some(mut params) = params.and_then(|value| value.as_object().cloned()) else { respond(&writer, id, None, Some(error_value(-32602, "invalid_mcp_server"))).await?; continue; };
                    let Some(server_id) = params.remove("id").and_then(|value| value.as_str().map(str::to_string)) else { respond(&writer, id, None, Some(error_value(-32602, "invalid_mcp_server"))).await?; continue; };
                    let Some(mut config) = params.remove("config").and_then(|value| value.as_object().cloned()) else { respond(&writer, id, None, Some(error_value(-32602, "invalid_mcp_server"))).await?; continue; };
                    config.insert("id".to_string(), Value::String(server_id));
                    match serde_json::from_value::<McpServerConfig>(Value::Object(config)) {
                        Ok(server) => match host.configure_mcp_server(server.clone()).await { Ok(()) => respond(&writer, id, Some(json!({"id": server.id, "name": server.name, "enabled": server.enabled, "connected": false})), None).await?, Err(error) => respond(&writer, id, None, Some(error_value(-32602, &error))).await? },
                        Err(_) => respond(&writer, id, None, Some(error_value(-32602, "invalid_mcp_server"))).await?,
                    }
                },
                "mcp/servers/enable" if initialized => {
                    let server_id = params.as_ref().and_then(Value::as_object).and_then(|object| object.get("id")).and_then(Value::as_str).unwrap_or("");
                    let Some(enabled) = params.as_ref().and_then(Value::as_object).and_then(|object| object.get("enabled")).and_then(Value::as_bool) else { respond(&writer, id, None, Some(error_value(-32602, "invalid_mcp_enable"))).await?; continue; };
                    match host.set_mcp_server_enabled(server_id, enabled).await { Ok(()) => respond(&writer, id, Some(json!({"id": server_id, "enabled": enabled})), None).await?, Err(error) => respond(&writer, id, None, Some(error_value(-32602, &error))).await? }
                },
                "mcp/servers/restart" if initialized => {
                    let server_id = params.as_ref().and_then(Value::as_object).and_then(|object| object.get("id")).and_then(Value::as_str).unwrap_or("");
                    match host.restart_mcp_server(server_id).await { Ok(result) => respond(&writer, id, Some(json!({"id": server_id, "connected": true, "tool_count": result.tools.len()})), None).await?, Err(error) => respond(&writer, id, None, Some(error_value(-32603, &error))).await? }
                },
                "skills/list" if initialized => match skills::list_workspace_skills(&workspace.root) {
                    Ok(result) => respond(&writer, id, Some(result), None).await?,
                    Err(error) => respond(&writer, id, None, Some(error_value(-32603, &error))).await?,
                },
                "skills/install" if initialized => {
                    let object = params.as_ref().and_then(Value::as_object);
                    let source = object
                        .and_then(|obj| obj.get("source").or_else(|| obj.get("repo")))
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .trim();
                    let skill = object
                        .and_then(|obj| obj.get("skill"))
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|s| !s.is_empty());
                    match skills::install_workspace_skills(&workspace.root, source, skill).await {
                        Ok(result) => respond(&writer, id, Some(result), None).await?,
                        Err(error) => {
                            respond(&writer, id, None, Some(error_value(-32603, &error))).await?
                        }
                    }
                }
                method if initialized && work::handles(method) => {
                    let recovery_mode = if work_recovery_pending {
                        AttemptReconcileMode::RestartRecovery
                    } else {
                        AttemptReconcileMode::Live
                    };
                    if let Err(error) = work::reconcile(&workspace, &work_journal, recovery_mode) {
                        respond(&writer, id, None, Some(error_value(error.code, &error.message))).await?;
                        continue;
                    }
                    work_recovery_pending = false;
                    if method == "work/start-run" {
                        handle_work_start_run(&service, &workspace, &work_journal, &writer, id, params).await?;
                        continue;
                    }
                    match work::dispatch(&workspace, method, params) {
                        Ok(result) => respond(&writer, id, Some(result), None).await?,
                        Err(error) => {
                            respond(
                                &writer,
                                id,
                                None,
                                Some(error_value(error.code, &error.message)),
                            )
                            .await?
                        }
                    }
                }
                "work/tasks/list" if initialized => {
                    handle_task_list(&workspace, &writer, id).await?;
                }
                "work/tasks/create" if initialized => {
                    handle_task_create(&service, &workspace, &writer, id, params).await?;
                }
                "work/tasks/cancel" if initialized => {
                    handle_task_cancel(&service, &event_sink, &workspace, &writer, id, params)
                        .await?;
                }
                "work/tasks/retry" if initialized => {
                    handle_task_retry(&service, &host, &workspace, &writer, id, params).await?;
                }
                "work/tasks/remove" if initialized => {
                    handle_task_remove(&workspace, &run_coordinator, &writer, id, params).await?;
                }
                "work/automations/list" if initialized => {
                    handle_automation_list(&host, &writer, id).await?;
                }
                "work/automations/create" if initialized => {
                    handle_automation_create(&host, &writer, id, params).await?;
                }
                "work/automations/update" if initialized => {
                    handle_automation_update(&host, &writer, id, params).await?;
                }
                "work/automations/trigger" if initialized => {
                    handle_automation_trigger(&host, &writer, id, params).await?;
                }
                "work/automations/pause" if initialized => {
                    handle_automation_pause(&host, &writer, id, params).await?;
                }
                "work/automations/delete" if initialized => {
                    handle_automation_delete(&host, &writer, id, params).await?;
                }
                "config/get" if initialized => match load_run_configuration(&workspace) {
                    Ok(configuration) => {
                        respond(
                            &writer,
                            id,
                            Some(json!({
                                "agent": "altai",
                                "model": configuration.model.map(|value| value.value).unwrap_or_else(|| "auto".to_string()),
                                "permission": configuration.permission_mode.map(|value| value.value).unwrap_or_else(|| "plan".to_string()),
                                "provider": configuration.provider.map(|value| value.value).unwrap_or_else(|| "openai".to_string()),
                                "base_url": configuration.base_url.map(|value| value.value),
                                "permissions": [
                                    {"id":"ask","label":"Ask","description":"Approve shell and edits"},
                                    {"id":"auto-edit","label":"Auto-edit","description":"Auto-apply edits; ask for shell"},
                                    {"id":"plan","label":"Plan","description":"Read-only planning mode"},
                                    {"id":"bypass","label":"Bypass","description":"No prompts (explicit opt-in)"}
                                ]
                            })),
                            None,
                        )
                        .await?;
                    }
                    Err(error) => {
                        eprintln!("altai-cli serve: could not load run configuration: {error}");
                        respond(
                            &writer,
                            id,
                            None,
                            Some(error_value(-32603, "configuration_unavailable")),
                        )
                        .await?;
                    }
                },
                "config/update" if initialized => {
                    let params = params
                        .and_then(|value| value.as_object().cloned())
                        .unwrap_or_default();
                    match update_workspace_config(&workspace, &params) {
                        Ok(()) => match load_run_configuration(&workspace) {
                            Ok(configuration) => {
                                respond(
                                    &writer,
                                    id,
                                    Some(json!({
                                    "model": configuration.model.map(|value| value.value).unwrap_or_else(|| "auto".to_string()),
                                    "permission": configuration.permission_mode.map(|value| value.value).unwrap_or_else(|| "plan".to_string()),
                                    "provider": configuration.provider.map(|value| value.value).unwrap_or_else(|| "openai".to_string()),
                                    "base_url": configuration.base_url.map(|value| value.value),
                                    })),
                                    None,
                                )
                                .await?;
                            }
                            Err(error) => {
                                eprintln!(
                                    "altai-cli serve: could not reload configuration: {error}"
                                );
                                respond(
                                    &writer,
                                    id,
                                    None,
                                    Some(error_value(-32603, "configuration_unavailable")),
                                )
                                .await?;
                            }
                        },
                        Err(error) => {
                            respond(&writer, id, None, Some(error_value(-32602, &error))).await?;
                        }
                    }
                }
                "run/start" if initialized => {
                    handle_run_start(&service, &workspace, &writer, id, params).await?;
                }
                "run/retry" if initialized => {
                    handle_run_retry(&service, &host, &workspace, &writer, id, params).await?;
                }
                "run/steer" if initialized => {
                    let params = params
                        .and_then(|value| value.as_object().cloned())
                        .unwrap_or_default();
                    let chat_id = params
                        .get("chat_id")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let run_id = params
                        .get("run_id")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let content = params
                        .get("content")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    match service.route_steer(chat_id, run_id, content).await {
                        Ok(_) => {
                            respond(&writer, id, Some(json!({"accepted": true})), None).await?;
                        }
                        Err(error) => {
                            respond(&writer, id, None, Some(error_value(-32002, &error))).await?;
                        }
                    }
                }
                "run/cancel" if initialized => {
                    let params = params
                        .and_then(|value| value.as_object().cloned())
                        .unwrap_or_default();
                    let run_id = params
                        .get("run_id")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let chat_id = params
                        .get("chat_id")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                        .or_else(|| coordinator_guard(&run_coordinator).chat_for_run(&run_id))
                        .or_else(|| event_sink.chat_for_run(&run_id));
                    let Some(chat_id) = chat_id.filter(|value| !value.trim().is_empty()) else {
                        respond(&writer, id, None, Some(error_value(-32002, "stale_run_id")))
                            .await?;
                        continue;
                    };
                    // Claim the wire terminal before asking the service to
                    // cancel, so a racing completed lifecycle can be rewritten
                    // during the test pause window (see StdioEventSink).
                    let terminal_still_open = event_sink.claim_cancel(&run_id);
                    match service.route_cancel(chat_id, run_id).await {
                        Ok(_) => {
                            respond(&writer, id, Some(json!({"accepted": true})), None).await?;
                        }
                        Err(_) if terminal_still_open => {
                            // Lease already released by a completed lifecycle
                            // that is still paused in the sink; the paused
                            // frame will be rewritten to cancelled.
                            respond(&writer, id, Some(json!({"accepted": true})), None).await?;
                        }
                        Err(_) => {
                            respond(&writer, id, None, Some(error_value(-32002, "stale_run_id")))
                                .await?;
                        }
                    }
                }
                "run/replay" if initialized => {
                    handle_run_replay(&workspace, &writer, id, params).await?;
                }
                "clarification/respond" if initialized => {
                    let params = params
                        .and_then(|value| value.as_object().cloned())
                        .unwrap_or_default();
                    let chat_id = params.get("chat_id").and_then(Value::as_str).unwrap_or("");
                    let action = params
                        .get("action")
                        .and_then(Value::as_str)
                        .unwrap_or("reply");
                    let text = params.get("text").and_then(Value::as_str).unwrap_or("");
                    if chat_id.trim().is_empty()
                        || chat_id.len() > 256
                        || !matches!(action, "reply" | "dismiss")
                        || (action == "reply" && (text.trim().is_empty() || text.len() > 16_384))
                    {
                        respond(
                            &writer,
                            id,
                            None,
                            Some(error_value(-32602, "invalid_clarification_response")),
                        )
                        .await?;
                        continue;
                    }
                    let result = if action == "dismiss" {
                        host.dismiss_clarification(chat_id).await
                    } else {
                        host.deliver_clarification_reply(chat_id, text.to_string())
                            .await
                    };
                    match result {
                        Ok(()) => {
                            respond(&writer, id, Some(json!({"accepted": true})), None).await?
                        }
                        Err(error) => {
                            respond(&writer, id, None, Some(error_value(-32002, &error))).await?
                        }
                    }
                }
                "context/compact" if initialized => {
                    let params = params
                        .and_then(|value| value.as_object().cloned())
                        .unwrap_or_default();
                    let chat_id = params
                        .get("chat_id")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    if chat_id.trim().is_empty() {
                        respond(
                            &writer,
                            id,
                            None,
                            Some(error_value(-32602, "chat_id_required")),
                        )
                        .await?;
                        continue;
                    }
                    let focus = params
                        .get("focus")
                        .and_then(Value::as_str)
                        .map(str::to_string);
                    let workspace_path = workspace.root.to_string_lossy().to_string();
                    match service
                        .route_manual_compaction(&workspace_path, chat_id, focus)
                        .await
                    {
                        Ok(_) => {
                            respond(&writer, id, Some(json!({"accepted": true})), None).await?;
                        }
                        Err(error) => {
                            respond(&writer, id, None, Some(error_value(-32002, &error))).await?;
                        }
                    }
                }
                "checkpoints/list" if initialized => {
                    let checkpoints = isanagent::checkpoint::store()
                        .map(|store| {
                            store
                                .list()
                                .into_iter()
                                .map(|entry| {
                                    json!({
                                        "id": entry.id,
                                        "path": entry.path,
                                        "label": entry.label,
                                        "created_ms": u64::try_from(entry.created_ms).unwrap_or(u64::MAX),
                                        "existed": entry.existed,
                                    })
                                })
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    respond(&writer, id, Some(json!({"checkpoints": checkpoints})), None).await?;
                }
                "checkpoints/restore" if initialized => {
                    let params = params
                        .and_then(|value| value.as_object().cloned())
                        .unwrap_or_default();
                    let checkpoint_id = params.get("id").and_then(Value::as_str).unwrap_or("");
                    if checkpoint_id.trim().is_empty()
                        || checkpoint_id.len() > 256
                        || checkpoint_id.contains('/')
                        || checkpoint_id.contains('\\')
                        || checkpoint_id.contains("..")
                    {
                        respond(
                            &writer,
                            id,
                            None,
                            Some(error_value(-32602, "invalid_checkpoint_id")),
                        )
                        .await?;
                        continue;
                    }
                    let Some(store) = isanagent::checkpoint::store() else {
                        respond(
                            &writer,
                            id,
                            None,
                            Some(error_value(-32004, "checkpoints_unavailable")),
                        )
                        .await?;
                        continue;
                    };
                    match store.restore(checkpoint_id) {
                        Ok(summary) => {
                            respond(
                                &writer,
                                id,
                                Some(json!({"restored": true, "summary": summary})),
                                None,
                            )
                            .await?;
                        }
                        Err(error) => {
                            eprintln!("altai-cli serve: checkpoint restore failed: {error}");
                            respond(
                                &writer,
                                id,
                                None,
                                Some(error_value(-32002, "checkpoint_restore_failed")),
                            )
                            .await?;
                        }
                    }
                }
                "review/proposals/list" if initialized => {
                    let params = params
                        .and_then(|value| value.as_object().cloned())
                        .unwrap_or_default();
                    match handle_proposal_list(&edit_proposals, &params) {
                        Ok(result) => respond(&writer, id, Some(result), None).await?,
                        Err(reason) => {
                            respond(&writer, id, None, Some(error_value(-32004, reason))).await?;
                        }
                    }
                }
                "review/proposals/upsert" if initialized => {
                    let params = params
                        .and_then(|value| value.as_object().cloned())
                        .unwrap_or_default();
                    match handle_proposal_upsert(&edit_proposals, &workspace.root, &params) {
                        Ok(result) => respond(&writer, id, Some(result), None).await?,
                        Err(reason) => {
                            let code = match reason {
                                "invalid_proposal_id"
                                | "invalid_proposal_path"
                                | "invalid_proposal_kind"
                                | "proposal_content_too_large"
                                | "path_outside_workspace" => -32602,
                                "already_applied" => -32002,
                                _ => -32004,
                            };
                            respond(&writer, id, None, Some(error_value(code, reason))).await?;
                        }
                    }
                }
                "review/proposals/apply" if initialized => {
                    let params = params
                        .and_then(|value| value.as_object().cloned())
                        .unwrap_or_default();
                    match handle_proposal_apply(&edit_proposals, &workspace.root, &params) {
                        Ok(result) => respond(&writer, id, Some(result), None).await?,
                        Err(reason) => {
                            let code = match reason.as_str() {
                                "invalid_proposal_id"
                                | "invalid_proposal_path"
                                | "invalid_proposal_kind"
                                | "proposal_content_too_large"
                                | "path_outside_workspace" => -32602,
                                "unknown_proposal" | "already_applied" => -32002,
                                _ if reason.starts_with("proposal_apply_failed") => -32002,
                                _ => -32004,
                            };
                            respond(&writer, id, None, Some(error_value(code, &reason))).await?;
                        }
                    }
                }
                "review/proposals/deny" if initialized => {
                    let params = params
                        .and_then(|value| value.as_object().cloned())
                        .unwrap_or_default();
                    match handle_proposal_deny(&edit_proposals, &params) {
                        Ok(result) => respond(&writer, id, Some(result), None).await?,
                        Err(reason) => {
                            let code = if reason == "invalid_proposal_id" {
                                -32602
                            } else {
                                -32004
                            };
                            respond(&writer, id, None, Some(error_value(code, reason))).await?;
                        }
                    }
                }
                "shutdown" => {
                    cancel_all_active(&service).await;
                    respond(&writer, id, Some(json!({"accepted":true})), None).await?;
                    return Ok(());
                }
                _ => {
                    respond(
                        &writer,
                        id,
                        None,
                        Some(error_value(-32004, "capability_unavailable")),
                    )
                    .await?
                }
            }
        }
    }
    cancel_all_active(&service).await;
    Ok(())
}

fn session_metadata_value(session: altai_core::SessionJournalMetadata) -> Value {
    json!({
        "chat_id": session.chat_id,
        "title": session.title,
        "archived": session.archived,
        "updated_at_ms": session.updated_at_ms,
    })
}

fn task_status(summary: Option<&altai_core::RunJournalSummary>) -> &'static str {
    let Some(summary) = summary else {
        return "queued";
    };
    if summary.terminal_seq.is_none() {
        return "running";
    }
    match summary
        .terminal_payload
        .as_ref()
        .and_then(|value| value.pointer("/outcome/kind"))
        .and_then(Value::as_str)
    {
        Some("cancelled") => "cancelled",
        Some("failed") | Some("error") => "failed",
        _ => "succeeded",
    }
}

async fn handle_task_list(
    workspace: &WorkspacePaths,
    writer: &Writer,
    id: Value,
) -> Result<(), String> {
    let journal = match EventJournal::open(workspace.agent_event_journal_db()) {
        Ok(journal) => journal,
        Err(_) => {
            return respond(
                writer,
                id,
                None,
                Some(error_value(-32603, "journal_unavailable")),
            )
            .await
        }
    };
    let tasks = match journal.list_task_runs(200) {
        Ok(tasks) => tasks,
        Err(_) => {
            return respond(
                writer,
                id,
                None,
                Some(error_value(-32603, "journal_unavailable")),
            )
            .await
        }
    }
    .into_iter()
    .map(|task| {
        let summary = journal
            .latest_run_summary_for_chat(&task.chat_id)
            .ok()
            .flatten();
        json!({"id": task.chat_id, "chat_id": task.chat_id, "title": task.title,
               "status": task_status(summary.as_ref()), "created_at_ms": task.created_at_ms})
    })
    .collect::<Vec<_>>();
    respond(writer, id, Some(json!({"task_runs": tasks})), None).await
}

async fn handle_task_create(
    service: &Arc<AgentService<StdioHost>>,
    workspace: &WorkspacePaths,
    writer: &Writer,
    id: Value,
    params: Option<Value>,
) -> Result<(), String> {
    let Some(params) = params.and_then(|value| value.as_object().cloned()) else {
        return respond(
            writer,
            id,
            None,
            Some(error_value(-32602, "invalid_task_params")),
        )
        .await;
    };
    let chat_id = params.get("chat_id").and_then(Value::as_str).unwrap_or("");
    let title = params
        .get("task_title")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("")
        .to_string();
    let prompt = params.get("prompt").and_then(Value::as_str).unwrap_or("");
    if !valid_session_chat_id(chat_id)
        || title.is_empty()
        || title.len() > 256
        || prompt.trim().is_empty()
        || prompt.len() > 16_384
    {
        return respond(
            writer,
            id,
            None,
            Some(error_value(-32602, "invalid_task_params")),
        )
        .await;
    }
    let mut start_params = params;
    start_params.insert("task_title".to_string(), Value::String(title));
    handle_run_start(
        service,
        workspace,
        writer,
        id,
        Some(Value::Object(start_params)),
    )
    .await
}

async fn handle_task_cancel(
    service: &Arc<AgentService<StdioHost>>,
    event_sink: &Arc<StdioEventSink>,
    workspace: &WorkspacePaths,
    writer: &Writer,
    id: Value,
    params: Option<Value>,
) -> Result<(), String> {
    let task_id = params
        .as_ref()
        .and_then(Value::as_object)
        .and_then(|value| value.get("task_id"))
        .and_then(Value::as_str)
        .unwrap_or("");
    if !valid_session_chat_id(task_id) {
        return respond(
            writer,
            id,
            None,
            Some(error_value(-32602, "invalid_task_id")),
        )
        .await;
    }
    let journal = match EventJournal::open(workspace.agent_event_journal_db()) {
        Ok(journal) => journal,
        Err(_) => {
            return respond(
                writer,
                id,
                None,
                Some(error_value(-32603, "journal_unavailable")),
            )
            .await
        }
    };
    let summary = match journal.latest_run_summary_for_chat(task_id) {
        Ok(Some(summary)) => summary,
        Ok(None) => {
            return respond(
                writer,
                id,
                None,
                Some(error_value(-32002, "task_run_not_found")),
            )
            .await
        }
        Err(_) => {
            return respond(
                writer,
                id,
                None,
                Some(error_value(-32603, "journal_unavailable")),
            )
            .await
        }
    };
    let terminal_still_open = event_sink.claim_cancel(&summary.run_id);
    if summary.terminal_seq.is_some() && !terminal_still_open {
        return respond(
            writer,
            id,
            None,
            Some(error_value(-32002, "task_run_not_active")),
        )
        .await;
    }
    match service
        .route_cancel(task_id.to_string(), summary.run_id)
        .await
    {
        Ok(_) => respond(writer, id, Some(json!({"accepted": true})), None).await,
        Err(_) if terminal_still_open => {
            respond(writer, id, Some(json!({"accepted": true})), None).await
        }
        Err(_) => {
            respond(
                writer,
                id,
                None,
                Some(error_value(-32002, "task_run_not_active")),
            )
            .await
        }
    }
}

async fn handle_task_retry(
    service: &Arc<AgentService<StdioHost>>,
    host: &Arc<StdioHost>,
    workspace: &WorkspacePaths,
    writer: &Writer,
    id: Value,
    params: Option<Value>,
) -> Result<(), String> {
    let task_id = params
        .as_ref()
        .and_then(Value::as_object)
        .and_then(|value| value.get("task_id"))
        .and_then(Value::as_str)
        .unwrap_or("");
    if !valid_session_chat_id(task_id) {
        return respond(
            writer,
            id,
            None,
            Some(error_value(-32602, "invalid_task_id")),
        )
        .await;
    }
    let journal = match EventJournal::open(workspace.agent_event_journal_db()) {
        Ok(journal) => journal,
        Err(_) => {
            return respond(
                writer,
                id,
                None,
                Some(error_value(-32603, "journal_unavailable")),
            )
            .await
        }
    };
    let summary = match journal.latest_run_summary_for_chat(task_id) {
        Ok(Some(summary)) if summary.terminal_seq.is_some() => summary,
        Ok(Some(_)) => {
            return respond(
                writer,
                id,
                None,
                Some(error_value(-32002, "task_run_not_terminal")),
            )
            .await
        }
        Ok(None) => {
            return respond(
                writer,
                id,
                None,
                Some(error_value(-32002, "task_run_not_found")),
            )
            .await
        }
        Err(_) => {
            return respond(
                writer,
                id,
                None,
                Some(error_value(-32603, "journal_unavailable")),
            )
            .await
        }
    };
    handle_run_retry(
        service,
        host,
        workspace,
        writer,
        id,
        Some(json!({"chat_id": task_id, "run_id": summary.run_id})),
    )
    .await
}

async fn handle_task_remove(
    workspace: &WorkspacePaths,
    run_coordinator: &SharedRunCoordinator,
    writer: &Writer,
    id: Value,
    params: Option<Value>,
) -> Result<(), String> {
    let task_id = params
        .as_ref()
        .and_then(Value::as_object)
        .and_then(|value| value.get("task_id"))
        .and_then(Value::as_str)
        .unwrap_or("");
    if task_id.trim().is_empty() || task_id.len() > 256 {
        return respond(
            writer,
            id,
            None,
            Some(error_value(-32602, "invalid_task_id")),
        )
        .await;
    }
    if coordinator_guard(run_coordinator)
        .active_runs()
        .into_iter()
        .any(|(chat_id, _, _)| chat_id == task_id)
    {
        return respond(
            writer,
            id,
            None,
            Some(error_value(-32002, "task_run_active")),
        )
        .await;
    }
    let journal = match EventJournal::open(workspace.agent_event_journal_db()) {
        Ok(journal) => journal,
        Err(_) => {
            return respond(
                writer,
                id,
                None,
                Some(error_value(-32603, "journal_unavailable")),
            )
            .await
        }
    };
    match journal.remove_task_run(task_id) {
        Ok(true) => respond(writer, id, Some(json!({"removed": true})), None).await,
        Ok(false) => {
            respond(
                writer,
                id,
                None,
                Some(error_value(-32002, "task_run_not_found")),
            )
            .await
        }
        Err(_) => {
            respond(
                writer,
                id,
                None,
                Some(error_value(-32603, "journal_unavailable")),
            )
            .await
        }
    }
}

async fn handle_automation_list(
    host: &Arc<StdioHost>,
    writer: &Writer,
    id: Value,
) -> Result<(), String> {
    match host.list_automations().await {
        Ok(automations) => respond(
            writer,
            id,
            Some(json!({
                "automations": automations
                    .iter()
                    .map(automation_value)
                    .collect::<Vec<_>>()
            })),
            None,
        )
        .await,
        Err(error) => respond(writer, id, None, Some(automation_error(&error))).await,
    }
}

async fn handle_automation_create(
    host: &Arc<StdioHost>,
    writer: &Writer,
    id: Value,
    params: Option<Value>,
) -> Result<(), String> {
    let Some(params) = params.and_then(|value| value.as_object().cloned()) else {
        return respond(writer, id, None, Some(error_value(-32602, "invalid_automation_params"))).await;
    };
    let chat_id = params.get("chat_id").and_then(Value::as_str).unwrap_or("");
    let title = params.get("title").and_then(Value::as_str).unwrap_or("");
    let prompt = params.get("prompt").and_then(Value::as_str).unwrap_or("");
    let Some(schedule) = params.get("schedule").and_then(parse_automation_schedule) else {
        return respond(writer, id, None, Some(error_value(-32602, "invalid_automation_schedule"))).await;
    };
    match host.create_automation(chat_id, title, prompt, schedule).await {
        Ok(automation) => respond(writer, id, Some(json!({"automation": automation_value(&automation)})), None).await,
        Err(error) => respond(writer, id, None, Some(automation_error(&error))).await,
    }
}

async fn handle_automation_update(
    host: &Arc<StdioHost>,
    writer: &Writer,
    id: Value,
    params: Option<Value>,
) -> Result<(), String> {
    let Some(params) = params.and_then(|value| value.as_object().cloned()) else {
        return respond(writer, id, None, Some(error_value(-32602, "invalid_automation_params"))).await;
    };
    let automation_id = params.get("automation_id").and_then(Value::as_str).unwrap_or("");
    let title = params.get("title").and_then(Value::as_str);
    let prompt = params.get("prompt").and_then(Value::as_str);
    let enabled = params.get("enabled").and_then(Value::as_bool);
    let schedule = match params.get("schedule") {
        Some(value) => match parse_automation_schedule(value) {
            Some(value) => Some(value),
            None => return respond(writer, id, None, Some(error_value(-32602, "invalid_automation_schedule"))).await,
        },
        None => None,
    };
    if title.is_none() && prompt.is_none() && schedule.is_none() && enabled.is_none() {
        return respond(writer, id, None, Some(error_value(-32602, "automation_patch_empty"))).await;
    }
    match host.update_automation(automation_id, title, prompt, schedule, enabled).await {
        Ok(automation) => respond(writer, id, Some(json!({"automation": automation_value(&automation)})), None).await,
        Err(error) => respond(writer, id, None, Some(automation_error(&error))).await,
    }
}

async fn handle_automation_trigger(
    host: &Arc<StdioHost>,
    writer: &Writer,
    id: Value,
    params: Option<Value>,
) -> Result<(), String> {
    let automation_id = automation_id_from_params(params);
    match host.trigger_automation(&automation_id).await {
        Ok(()) => respond(writer, id, Some(json!({"accepted": true})), None).await,
        Err(error) => respond(writer, id, None, Some(automation_error(&error))).await,
    }
}

async fn handle_automation_pause(
    host: &Arc<StdioHost>,
    writer: &Writer,
    id: Value,
    params: Option<Value>,
) -> Result<(), String> {
    let automation_id = automation_id_from_params(params);
    match host.pause_automation(&automation_id).await {
        Ok(()) => respond(writer, id, Some(json!({"accepted": true})), None).await,
        Err(error) => respond(writer, id, None, Some(automation_error(&error))).await,
    }
}

async fn handle_automation_delete(
    host: &Arc<StdioHost>,
    writer: &Writer,
    id: Value,
    params: Option<Value>,
) -> Result<(), String> {
    let automation_id = automation_id_from_params(params);
    match host.delete_automation(&automation_id).await {
        Ok(()) => respond(writer, id, Some(json!({"removed": true})), None).await,
        Err(error) => respond(writer, id, None, Some(automation_error(&error))).await,
    }
}

fn automation_id_from_params(params: Option<Value>) -> String {
    params
        .as_ref()
        .and_then(Value::as_object)
        .and_then(|value| value.get("automation_id"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn parse_automation_schedule(value: &Value) -> Option<isanagent::scheduler::ScheduleKind> {
    let value = value.as_object()?;
    match value.get("kind")?.as_str()? {
        "once" => chrono::DateTime::parse_from_rfc3339(value.get("at")?.as_str()?)
            .ok()
            .map(|at| isanagent::scheduler::ScheduleKind::At { at_ms: at.timestamp_millis() }),
        "every" => value
            .get("every_ms")?
            .as_i64()
            .filter(|every_ms| *every_ms > 0)
            .map(|every_ms| isanagent::scheduler::ScheduleKind::Every { every_ms }),
        _ => None,
    }
}

fn automation_value(automation: &crate::stdio_host::StdioAutomation) -> Value {
    let schedule = match &automation.schedule {
        isanagent::scheduler::ScheduleKind::At { at_ms } => {
            let at = chrono::DateTime::from_timestamp_millis(*at_ms)
                .map(|value| value.to_rfc3339())
                .unwrap_or_default();
            json!({"kind": "once", "at": at})
        }
        isanagent::scheduler::ScheduleKind::Every { every_ms } => {
            json!({"kind": "every", "every_ms": every_ms})
        }
        isanagent::scheduler::ScheduleKind::Cron { cron_expr } => {
            json!({"kind": "cron", "expression": cron_expr})
        }
    };
    json!({
        "id": automation.id,
        "chat_id": automation.chat_id,
        "title": automation.title,
        "prompt": automation.prompt,
        "schedule": schedule,
        "enabled": automation.enabled,
    })
}

fn automation_error(error: &str) -> Value {
    let code = match error {
        "automation_not_found" => -32002,
        "automation_paused" => -32002,
        _ => -32602,
    };
    error_value(code, error)
}

async fn handle_session_mutation(
    host: &Arc<StdioHost>,
    workspace: &WorkspacePaths,
    run_coordinator: &SharedRunCoordinator,
    writer: &Writer,
    id: Value,
    method: &str,
    params: Option<Value>,
) -> Result<(), String> {
    let params = params
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();
    let chat_id = params.get("chat_id").and_then(Value::as_str).unwrap_or("");
    if chat_id.trim().is_empty() || chat_id.len() > 256 {
        return respond(
            writer,
            id,
            None,
            Some(error_value(-32602, "invalid_chat_id")),
        )
        .await;
    }
    let journal = match EventJournal::open(workspace.agent_event_journal_db()) {
        Ok(journal) => journal,
        Err(error) => {
            eprintln!("altai-cli serve: could not open session journal: {error}");
            return respond(
                writer,
                id,
                None,
                Some(error_value(-32603, "journal_unavailable")),
            )
            .await;
        }
    };
    match method {
        "sessions/rename" => {
            let title = params.get("title").and_then(Value::as_str).unwrap_or("");
            match journal.rename_session(chat_id, title.trim()) {
                Ok(Some(session)) => {
                    respond(writer, id, Some(session_metadata_value(session)), None).await
                }
                Ok(None) => {
                    respond(
                        writer,
                        id,
                        None,
                        Some(error_value(-32002, "session_not_found")),
                    )
                    .await
                }
                Err(_) => {
                    respond(
                        writer,
                        id,
                        None,
                        Some(error_value(-32602, "invalid_session_title")),
                    )
                    .await
                }
            }
        }
        "sessions/archive" => match journal.archive_session(chat_id) {
            Ok(Some(session)) => {
                respond(writer, id, Some(session_metadata_value(session)), None).await
            }
            Ok(None) => {
                respond(
                    writer,
                    id,
                    None,
                    Some(error_value(-32002, "session_not_found")),
                )
                .await
            }
            Err(_) => {
                respond(
                    writer,
                    id,
                    None,
                    Some(error_value(-32602, "invalid_chat_id")),
                )
                .await
            }
        },
        "sessions/delete" => {
            if coordinator_guard(run_coordinator)
                .active_runs()
                .into_iter()
                .any(|(active_chat_id, _, _)| active_chat_id == chat_id)
            {
                return respond(
                    writer,
                    id,
                    None,
                    Some(error_value(-32002, "session_run_active")),
                )
                .await;
            }
            if journal
                .session_metadata(chat_id)
                .map_err(|error| error.to_string())?
                .is_none()
            {
                return respond(
                    writer,
                    id,
                    None,
                    Some(error_value(-32002, "session_not_found")),
                )
                .await;
            }
            host.delete_session_memory(chat_id).await?;
            match journal.delete_session(chat_id) {
                Ok(true) => respond(writer, id, Some(json!({"deleted": true})), None).await,
                Ok(false) => {
                    respond(
                        writer,
                        id,
                        None,
                        Some(error_value(-32002, "session_not_found")),
                    )
                    .await
                }
                Err(error) => {
                    eprintln!("altai-cli serve: could not delete session: {error}");
                    respond(
                        writer,
                        id,
                        None,
                        Some(error_value(-32603, "journal_unavailable")),
                    )
                    .await
                }
            }
        }
        _ => {
            respond(
                writer,
                id,
                None,
                Some(error_value(-32004, "capability_unavailable")),
            )
            .await
        }
    }
}

async fn handle_run_start(
    service: &Arc<AgentService<StdioHost>>,
    workspace: &WorkspacePaths,
    writer: &Writer,
    id: Value,
    params: Option<Value>,
) -> Result<(), String> {
    let Some(params) = params.and_then(|value| value.as_object().cloned()) else {
        return respond(
            writer,
            id,
            None,
            Some(error_value(-32602, "invalid_params")),
        )
        .await;
    };
    let Some(chat_id) = params
        .get("chat_id")
        .and_then(Value::as_str)
        .filter(|v| !v.trim().is_empty())
        .map(str::to_string)
    else {
        return respond(
            writer,
            id,
            None,
            Some(error_value(-32602, "chat_id_required")),
        )
        .await;
    };
    let Some(prompt) = params
        .get("prompt")
        .and_then(Value::as_str)
        .filter(|v| !v.trim().is_empty())
        .map(str::to_string)
    else {
        return respond(
            writer,
            id,
            None,
            Some(error_value(-32602, "prompt_required")),
        )
        .await;
    };
    let agent = params
        .get("agent")
        .and_then(Value::as_str)
        .unwrap_or("altai");
    if agent != "altai" {
        return respond(
            writer,
            id,
            None,
            Some(error_value(-32602, "unsupported_agent")),
        )
        .await;
    }
    let model = params
        .get("model")
        .and_then(Value::as_str)
        .filter(|value| *value != "auto");
    if model.is_some_and(|value| value.trim().is_empty() || value.len() > 512) {
        return respond(writer, id, None, Some(error_value(-32602, "invalid_model"))).await;
    }
    let permission = params
        .get("permission")
        .and_then(Value::as_str)
        .unwrap_or("plan");
    if !matches!(
        permission,
        "ask" | "auto-edit" | "auto_edit" | "plan" | "bypass"
    ) {
        return respond(
            writer,
            id,
            None,
            Some(error_value(-32602, "invalid_permission")),
        )
        .await;
    }

    let queue = params
        .get("queue")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let task_title = params
        .get("task_title")
        .and_then(Value::as_str)
        .map(str::trim);
    if task_title.is_some_and(|title| title.is_empty() || title.len() > 256) {
        return respond(
            writer,
            id,
            None,
            Some(error_value(-32602, "invalid_task_title")),
        )
        .await;
    }
    let (images, documents) = match parse_run_attachments(params.get("attachments")) {
        Ok(attachments) => attachments,
        Err(error) => return respond(writer, id, None, Some(error_value(-32602, error))).await,
    };
    let route = match resolve_run_route(workspace, model, Some(permission), true) {
        Ok(route) => route,
        Err(reason) => {
            return respond(writer, id, None, Some(error_value(-32603, reason))).await;
        }
    };

    // Admission returns once the user message is on the bus; the agent loop
    // continues in the background and emits framed `run/event` notifications.
    match dispatch_configured_run(service, workspace, route, ConfiguredRunRequest {
        chat_id: chat_id.clone(),
        prompt,
        queue,
        images,
        documents,
    })
    .await
    {
        Ok(ack) => {
            if let Some(title) = task_title {
                let journal = match EventJournal::open(workspace.agent_event_journal_db()) {
                    Ok(journal) => journal,
                    Err(error) => {
                        eprintln!("altai-cli serve: could not open task journal: {error}");
                        return respond(
                            writer,
                            id,
                            None,
                            Some(error_value(-32603, "journal_unavailable")),
                        )
                        .await;
                    }
                };
                if let Err(error) = journal.create_task_run(&chat_id, title) {
                    eprintln!("altai-cli serve: could not persist task run: {error}");
                    return respond(
                        writer,
                        id,
                        None,
                        Some(error_value(-32603, "journal_unavailable")),
                    )
                    .await;
                }
            }
            let mut result = json!({
                "accepted": true,
                "run_id": ack.run_id,
                "queued": ack.queued,
            });
            if task_title.is_some() {
                result["task_id"] = Value::String(chat_id);
            }
            respond(
                writer,
                id,
                Some(result),
                None,
            )
            .await
        }
        Err(error) => {
            eprintln!("altai-cli serve: run/start failed: {error}");
            respond(
                writer,
                id,
                None,
                Some(error_value(-32603, "run_start_failed")),
            )
            .await
        }
    }
}

struct ConfiguredRunRequest {
    chat_id: String,
    prompt: String,
    queue: bool,
    images: Vec<String>,
    documents: Vec<DocumentPart>,
}

struct ResolvedRunRoute {
    provider_name: String,
    api_key: String,
    model_name: String,
    permission: String,
    base_url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProviderCredentialScope {
    Official,
    EnvironmentTuple,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SecureProviderRoute {
    base_url: String,
    credential_scope: ProviderCredentialScope,
}

fn resolve_run_route(
    workspace: &WorkspacePaths,
    requested_model: Option<&str>,
    requested_permission: Option<&str>,
    allow_explicit_bypass: bool,
) -> Result<ResolvedRunRoute, &'static str> {
    let configuration = load_run_configuration(workspace)
        .map_err(|_| "configuration_unavailable")?;
    let provider_name = configuration
        .provider
        .as_ref()
        .map(|value| value.value.clone())
        .unwrap_or_else(|| "openai".to_string());
    provider_credentials::validate_provider_id(&provider_name)
        .map_err(|_| "invalid_provider_route")?;
    let model_name = requested_model
        .map(str::to_string)
        .or_else(|| configuration.model.as_ref().map(|value| value.value.clone()))
        .unwrap_or_else(|| "gpt-4o-mini".to_string());
    let permission = requested_permission
        .map(str::to_string)
        .or_else(|| {
            configuration
                .permission_mode
                .as_ref()
                .map(|value| value.value.clone())
        })
        .unwrap_or_else(|| "plan".to_string());
    let permission_allowed = matches!(permission.as_str(), "ask" | "auto-edit" | "auto_edit" | "plan")
        || (allow_explicit_bypass
            && requested_permission == Some("bypass")
            && permission == "bypass");
    if !permission_allowed {
        return Err("invalid_permission_configuration");
    }
    let provider_route = secure_provider_route(
        &provider_name,
        configuration.provider.as_ref(),
        configuration.base_url.as_ref(),
    )?;
    let api_key = match resolve_provider_credential(&provider_name, &provider_route) {
        Ok(value) => value,
        Err(error) if error == "api_key_not_configured" => String::new(),
        Err(_) => return Err("credential_unavailable"),
    };
    let scripted =
        cfg!(debug_assertions) && std::env::var_os("ALTAI_CLI_TEST_SCRIPTED_RESPONSE").is_some();
    if api_key.trim().is_empty() && !scripted {
        return Err("api_key_not_configured");
    }
    Ok(ResolvedRunRoute {
        provider_name,
        api_key,
        model_name,
        permission,
        base_url: provider_route.base_url,
    })
}

async fn dispatch_configured_run(
    service: &Arc<AgentService<StdioHost>>,
    workspace: &WorkspacePaths,
    route: ResolvedRunRoute,
    request: ConfiguredRunRequest,
) -> Result<SendAck, &'static str> {
    service
        .route_send(
            &route.provider_name,
            &route.api_key,
            &route.model_name,
            None,
            Some(&route.base_url),
            workspace.root.to_str(),
            Some(&route.permission),
            None,
            None,
            request.prompt,
            request.images,
            request.documents,
            request.chat_id,
            request.queue,
        )
        .await
        .map_err(|_| "run_start_failed")
}

fn canonical_provider_base_url(provider_id: &str) -> Option<&'static str> {
    match provider_id {
        "openai" => Some("https://api.openai.com/v1/chat/completions"),
        "anthropic" => Some("https://api.anthropic.com/v1/messages"),
        "google" => Some(
            "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions",
        ),
        "xai" => Some("https://api.x.ai/v1/chat/completions"),
        "cerebras" => Some("https://api.cerebras.ai/v1/chat/completions"),
        "groq" => Some("https://api.groq.com/openai/v1/chat/completions"),
        "deepseek" => Some("https://api.deepseek.com/v1/chat/completions"),
        "mistral" => Some("https://api.mistral.ai/v1/chat/completions"),
        "zai" => Some("https://api.z.ai/api/paas/v4/chat/completions"),
        "zai-coding-plan" => Some("https://api.z.ai/api/coding/paas/v4/chat/completions"),
        "openrouter" => Some("https://openrouter.ai/api/v1/chat/completions"),
        _ => None,
    }
}

fn secure_provider_route(
    provider_id: &str,
    configured_provider: Option<&ResolvedConfig<String>>,
    configured: Option<&ResolvedConfig<String>>,
) -> Result<SecureProviderRoute, &'static str> {
    let official = canonical_provider_base_url(provider_id);
    if let Some(configured) = configured {
        let value = configured.value.trim();
        if official == Some(value) {
            return Ok(SecureProviderRoute {
                base_url: value.to_string(),
                credential_scope: ProviderCredentialScope::Official,
            });
        }

        // A noncanonical endpoint is trusted only as one host-owned environment
        // tuple. Independent field resolution must never combine a repository
        // provider with an environment URL and reuse the provider store secret.
        let provider_is_environment = configured_provider.is_some_and(|provider| {
            provider.source == ConfigSource::Environment && provider.value.trim() == provider_id
        });
        let environment_tuple_matches = configured.source == ConfigSource::Environment
            && provider_is_environment
            && std::env::var("ALTAI_PROVIDER").ok().as_deref() == Some(provider_id)
            && std::env::var("ALTAI_BASE_URL")
                .ok()
                .is_some_and(|base_url| base_url.trim() == value);
        if !environment_tuple_matches {
            return Err("untrusted_base_url");
        }
        if value.len() > 2_048
            || !value.starts_with("https://")
            || value.contains('@')
            || value.contains('#')
        {
            return Err("invalid_provider_route");
        }
        return Ok(SecureProviderRoute {
            base_url: value.to_string(),
            credential_scope: ProviderCredentialScope::EnvironmentTuple,
        });
    }
    official
        .map(|base_url| SecureProviderRoute {
            base_url: base_url.to_string(),
            credential_scope: ProviderCredentialScope::Official,
        })
        .ok_or("unsupported_provider_route")
}

fn work_session_title(title: &str) -> &str {
    let mut end = title.len().min(256);
    while !title.is_char_boundary(end) {
        end -= 1;
    }
    &title[..end]
}

fn test_work_start_fault() -> Option<String> {
    cfg!(debug_assertions)
        .then(|| std::env::var("ALTAI_CLI_TEST_WORK_START_FAULT").ok())
        .flatten()
}

async fn handle_work_start_run(
    service: &Arc<AgentService<StdioHost>>,
    workspace: &WorkspacePaths,
    journal: &EventJournal,
    writer: &Writer,
    id: Value,
    params: Option<Value>,
) -> Result<(), String> {
    let request = match work::parse_start_run(params) {
        Ok(request) => request,
        Err(error) => {
            return respond(writer, id, None, Some(error_value(error.code, &error.message))).await
        }
    };
    // Resolve the credential route before mutating Work. Repository-controlled
    // configuration must not create an Attempt or session when it is unsafe.
    let route = match resolve_run_route(workspace, None, None, false) {
        Ok(route) => route,
        Err(reason) => {
            return respond(writer, id, None, Some(error_value(-32603, reason))).await;
        }
    };
    let chat_id = format!("work-{}", uuid::Uuid::new_v4());
    let started = match work::begin_start_run(workspace, &request, &chat_id) {
        Ok(started) => started,
        Err(error) => {
            return respond(writer, id, None, Some(error_value(error.code, &error.message))).await
        }
    };
    let attempt_id = started.attempt.id.clone();
    if let Err(error) = journal.create_session(&chat_id, work_session_title(&started.work.title)) {
        eprintln!("altai-cli serve: could not create Work run session: {error}");
        let _ = work::fail_start_run(workspace, &attempt_id, "Could not create the run session.");
        return respond(writer, id, None, Some(error_value(-32603, "journal_unavailable"))).await;
    }

    let mut prompt = format!("Deliver this Work outcome:\n\n{}", started.work.title);
    if !started.work.description.trim().is_empty() {
        prompt.push_str("\n\nDescription:\n");
        prompt.push_str(&started.work.description);
    }
    if !started.work.acceptance_criteria.trim().is_empty() {
        prompt.push_str("\n\nAcceptance criteria:\n");
        prompt.push_str(&started.work.acceptance_criteria);
    }
    prompt.push_str(
        "\n\nStay within the workspace and report concrete evidence for the acceptance criteria.",
    );

    let dispatch = if test_work_start_fault().as_deref() == Some("admission") {
        Err("run_start_failed")
    } else {
        dispatch_configured_run(service, workspace, route, ConfiguredRunRequest {
            chat_id: chat_id.clone(),
            prompt,
            queue: false,
            images: Vec::new(),
            documents: Vec::new(),
        })
        .await
    };
    let ack = match dispatch {
        Ok(ack) if ack.chat_id == chat_id && !ack.queued => ack,
        Ok(ack) => {
            let _ = service.route_cancel(ack.chat_id, ack.run_id).await;
            let _ = work::fail_start_run(
                workspace,
                &attempt_id,
                "Run admission returned an invalid identity.",
            );
            return respond(writer, id, None, Some(error_value(-32603, "run_start_failed"))).await;
        }
        Err(reason) => {
            let _ = work::fail_start_run(
                workspace,
                &attempt_id,
                "The agent run could not be started.",
            );
            return respond(writer, id, None, Some(error_value(-32603, reason))).await;
        }
    };

    if let Err(error) = work::bind_start_run(workspace, &attempt_id, &chat_id, &ack.run_id) {
        eprintln!("altai-cli serve: could not bind Work Attempt run: {}", error.message);
        let _ = service.route_cancel(chat_id.clone(), ack.run_id).await;
        let _ = work::reconcile(workspace, journal, AttemptReconcileMode::Live);
        let _ = work::fail_start_run(
            workspace,
            &attempt_id,
            "The agent run binding could not be persisted.",
        );
        return respond(writer, id, None, Some(error_value(-32603, "work_run_bind_failed"))).await;
    }
    if let Err(error) = work::reconcile(workspace, journal, AttemptReconcileMode::Live) {
        eprintln!("altai-cli serve: could not reconcile started Work run: {}", error.message);
        return respond(writer, id, None, Some(error_value(error.code, &error.message))).await;
    }
    match work::start_run_result(workspace, &request.work_id, &attempt_id) {
        Ok(result) => respond(writer, id, Some(result), None).await,
        Err(error) => {
            respond(writer, id, None, Some(error_value(error.code, &error.message))).await
        }
    }
}

/// Attachments are already materialized by the trusted extension host. The
/// Webview can never supply a filesystem path to this process. Keeping the
/// payload bounded here protects the stdio frame and provider request even if
/// another protocol client sends a forged request.
fn parse_run_attachments(
    value: Option<&Value>,
) -> Result<(Vec<String>, Vec<DocumentPart>), &'static str> {
    let Some(value) = value else {
        return Ok((Vec::new(), Vec::new()));
    };
    let attachments = value.as_array().ok_or("invalid_attachments")?;
    if attachments.len() > MAX_RUN_ATTACHMENTS {
        return Err("too_many_attachments");
    }
    let mut total_encoded_bytes = 0_usize;
    let mut images = Vec::new();
    let mut documents = Vec::new();
    for attachment in attachments {
        let attachment = attachment.as_object().ok_or("invalid_attachment")?;
        let kind = attachment.get("kind").and_then(Value::as_str).ok_or("invalid_attachment")?;
        let media_type = attachment
            .get("media_type")
            .and_then(Value::as_str)
            .ok_or("invalid_attachment")?;
        let data = attachment.get("data").and_then(Value::as_str).ok_or("invalid_attachment")?;
        if data.len() > MAX_RUN_ATTACHMENT_ENCODED_BYTES {
            return Err("attachment_too_large");
        }
        total_encoded_bytes = total_encoded_bytes.saturating_add(data.len());
        if total_encoded_bytes > MAX_RUN_ATTACHMENTS_ENCODED_BYTES {
            return Err("attachments_too_large");
        }
        let name = attachment.get("name").and_then(Value::as_str);
        if name.is_some_and(|name| name.trim().is_empty() || name.len() > 256) {
            return Err("invalid_attachment_name");
        }
        match kind {
            "image" => {
                if !matches!(media_type, "image/jpeg" | "image/png" | "image/gif" | "image/webp") {
                    return Err("unsupported_image_media_type");
                }
                decode_attachment_base64(data)?;
                images.push(format!("data:{media_type};base64,{data}"));
            }
            "document" => {
                if media_type != "application/pdf" {
                    return Err("unsupported_document_media_type");
                }
                decode_attachment_base64(data)?;
                documents.push(DocumentPart {
                    data: data.to_string(),
                    media_type: media_type.to_string(),
                    name: name.map(str::to_string),
                });
            }
            _ => return Err("unsupported_attachment_kind"),
        }
    }
    Ok((images, documents))
}

fn decode_attachment_base64(value: &str) -> Result<(), &'static str> {
    base64::engine::general_purpose::STANDARD
        .decode(value)
        .map(|_| ())
        .map_err(|_| "invalid_attachment_data")
}

#[cfg(test)]
mod attachment_tests {
    use super::parse_run_attachments;
    use serde_json::json;

    #[test]
    fn accepts_bounded_image_and_pdf_payloads_without_paths() {
        let (images, documents) = parse_run_attachments(Some(&json!([
            {
                "kind": "image",
                "media_type": "image/png",
                "data": "aGVsbG8=",
                "name": "diagram.png"
            },
            {
                "kind": "document",
                "media_type": "application/pdf",
                "data": "aGVsbG8=",
                "name": "notes.pdf"
            }
        ])))
        .expect("attachment payload");
        assert_eq!(images, ["data:image/png;base64,aGVsbG8="]);
        assert_eq!(documents.len(), 1);
        assert_eq!(documents[0].name.as_deref(), Some("notes.pdf"));
    }

    #[test]
    fn rejects_untrusted_or_unsupported_attachment_shapes() {
        assert!(matches!(
            parse_run_attachments(Some(&json!([
                {
                    "kind": "image",
                    "media_type": "image/svg+xml",
                    "data": "aGVsbG8="
                }
            ]))),
            Err("unsupported_image_media_type")
        ));
        assert!(matches!(
            parse_run_attachments(Some(&json!([
                {
                    "kind": "document",
                    "media_type": "application/pdf",
                    "data": "not-base64"
                }
            ]))),
            Err("invalid_attachment_data")
        ));
    }
}

fn valid_session_chat_id(chat_id: &str) -> bool {
    !chat_id.trim().is_empty() && chat_id.len() <= 256 && !chat_id.contains(':')
}

async fn handle_session_messages(
    host: &Arc<StdioHost>,
    writer: &Writer,
    id: Value,
    params: Option<Value>,
) -> Result<(), String> {
    let params = params
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();
    let chat_id = params.get("chat_id").and_then(Value::as_str).unwrap_or("");
    if !valid_session_chat_id(chat_id) {
        return respond(
            writer,
            id,
            None,
            Some(error_value(-32602, "invalid_chat_id")),
        )
        .await;
    }

    match host.get_session_messages(chat_id).await {
        Ok(messages) => {
            let mut user_turn = 0_usize;
            let messages = messages
                .into_iter()
                .enumerate()
                .map(|(index, message)| {
                    let message_id = if message.role == "user" {
                        user_turn += 1;
                        format!("user:{user_turn}")
                    } else {
                        format!("message:{}", index + 1)
                    };
                    json!({
                        "id": message_id,
                        "role": message.role,
                        "content": session_message_content(&message),
                    })
                })
                .collect::<Vec<_>>();
            respond(writer, id, Some(json!({"messages": messages})), None).await
        }
        Err(error) => {
            eprintln!("altai-cli serve: could not load session messages: {error}");
            respond(
                writer,
                id,
                None,
                Some(error_value(-32603, "session_memory_unavailable")),
            )
            .await
        }
    }
}

/// IsanAgent prepends host-derived runtime context to stored inbound user
/// messages. That prompt is model-facing metadata, not chat transcript text.
fn session_message_content(message: &isanagent::utils::ChatMessage) -> String {
    let content = message
        .content
        .as_ref()
        .map(|content| content.text_content())
        .unwrap_or_default();
    if message.role == "user" {
        return content
            .strip_prefix("[RUNTIME CONTEXT]")
            .and_then(|_| content.split_once("---ISANAGENT_RUNTIME_CONTEXT_END---\n\n"))
            .map(|(_, prompt)| prompt.to_string())
            .unwrap_or(content);
    }
    content
}

async fn handle_session_truncate(
    host: &Arc<StdioHost>,
    run_coordinator: &SharedRunCoordinator,
    writer: &Writer,
    id: Value,
    params: Option<Value>,
) -> Result<(), String> {
    let params = params
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();
    let chat_id = params.get("chat_id").and_then(Value::as_str).unwrap_or("");
    let keep_user_messages = params.get("keep_user_messages").and_then(Value::as_u64);
    let Some(keep_user_messages) = keep_user_messages else {
        return respond(
            writer,
            id,
            None,
            Some(error_value(-32602, "invalid_session_truncate_params")),
        )
        .await;
    };
    let Ok(keep_user_messages) = usize::try_from(keep_user_messages) else {
        return respond(
            writer,
            id,
            None,
            Some(error_value(-32602, "invalid_session_truncate_params")),
        )
        .await;
    };
    if !valid_session_chat_id(chat_id) {
        return respond(
            writer,
            id,
            None,
            Some(error_value(-32602, "invalid_chat_id")),
        )
        .await;
    }
    if coordinator_guard(run_coordinator)
        .active_runs()
        .into_iter()
        .any(|(active_chat_id, _, _)| active_chat_id == chat_id)
    {
        return respond(
            writer,
            id,
            None,
            Some(error_value(-32002, "session_run_active")),
        )
        .await;
    }

    match host
        .truncate_session_after_user_message(chat_id, keep_user_messages)
        .await
    {
        Ok(deleted_messages) => {
            respond(
                writer,
                id,
                Some(json!({"deleted_messages": deleted_messages})),
                None,
            )
            .await
        }
        Err(error) => {
            eprintln!("altai-cli serve: could not truncate session: {error}");
            respond(
                writer,
                id,
                None,
                Some(error_value(-32603, "session_memory_unavailable")),
            )
            .await
        }
    }
}

async fn handle_run_retry(
    service: &Arc<AgentService<StdioHost>>,
    host: &Arc<StdioHost>,
    workspace: &WorkspacePaths,
    writer: &Writer,
    id: Value,
    params: Option<Value>,
) -> Result<(), String> {
    let params = params
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();
    let chat_id = params.get("chat_id").and_then(Value::as_str).unwrap_or("");
    let run_id = params.get("run_id").and_then(Value::as_str).unwrap_or("");
    let replacement = params.get("edit_user_message").and_then(Value::as_str);
    if chat_id.trim().is_empty()
        || run_id.trim().is_empty()
        || chat_id.len() > 256
        || run_id.len() > 256
        || replacement.is_some_and(|value| value.trim().is_empty() || value.len() > 16_384)
    {
        return respond(
            writer,
            id,
            None,
            Some(error_value(-32602, "invalid_retry_params")),
        )
        .await;
    }

    let journal = match EventJournal::open(workspace.agent_event_journal_db()) {
        Ok(journal) => journal,
        Err(error) => {
            eprintln!("altai-cli serve: could not open retry journal: {error}");
            return respond(
                writer,
                id,
                None,
                Some(error_value(-32603, "journal_unavailable")),
            )
            .await;
        }
    };
    let latest = match journal.latest_run_summary_for_chat(chat_id) {
        Ok(Some(summary)) => summary,
        Ok(None) => {
            return respond(writer, id, None, Some(error_value(-32002, "run_not_found"))).await
        }
        Err(error) => {
            eprintln!("altai-cli serve: could not inspect retry run: {error}");
            return respond(
                writer,
                id,
                None,
                Some(error_value(-32603, "journal_unavailable")),
            )
            .await;
        }
    };
    if latest.run_id != run_id {
        return respond(
            writer,
            id,
            None,
            Some(error_value(-32002, "retry_not_latest_run")),
        )
        .await;
    }
    if latest.terminal_seq.is_none() {
        return respond(
            writer,
            id,
            None,
            Some(error_value(-32002, "retry_run_not_terminal")),
        )
        .await;
    }

    let prompt = match host
        .rewind_latest_turn_for_retry(chat_id, replacement)
        .await
    {
        Ok(prompt) => prompt,
        Err(error) => return respond(writer, id, None, Some(error_value(-32002, &error))).await,
    };
    let mut start_params = serde_json::Map::new();
    start_params.insert("chat_id".to_string(), Value::String(chat_id.to_string()));
    start_params.insert("prompt".to_string(), Value::String(prompt));
    handle_run_start(
        service,
        workspace,
        writer,
        id,
        Some(Value::Object(start_params)),
    )
    .await
}

async fn handle_run_replay(
    workspace: &WorkspacePaths,
    writer: &Writer,
    id: Value,
    params: Option<Value>,
) -> Result<(), String> {
    let params = params
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();
    let chat_id = params.get("chat_id").and_then(Value::as_str).unwrap_or("");
    let run_id = params.get("run_id").and_then(Value::as_str).unwrap_or("");
    let after_seq = params.get("after_seq").and_then(Value::as_u64).unwrap_or(0);
    let limit = params.get("limit").and_then(Value::as_u64).unwrap_or(500);
    if chat_id.trim().is_empty() || run_id.trim().is_empty() || !(1..=1_000).contains(&limit) {
        return respond(
            writer,
            id,
            None,
            Some(error_value(-32602, "invalid_replay_params")),
        )
        .await;
    }
    let journal = match EventJournal::open(workspace.agent_event_journal_db()) {
        Ok(journal) => journal,
        Err(error) => {
            eprintln!("altai-cli serve: could not open replay journal: {error}");
            return respond(
                writer,
                id,
                None,
                Some(error_value(-32603, "journal_unavailable")),
            )
            .await;
        }
    };
    let summary = match journal.run_summary(run_id) {
        Ok(Some(summary)) if summary.chat_id == chat_id => summary,
        Ok(_) => {
            return respond(writer, id, None, Some(error_value(-32002, "run_not_found"))).await;
        }
        Err(error) => {
            eprintln!("altai-cli serve: could not inspect replay run: {error}");
            return respond(
                writer,
                id,
                None,
                Some(error_value(-32603, "journal_unavailable")),
            )
            .await;
        }
    };
    let records = match journal.fetch_after(run_id, after_seq, limit as usize) {
        Ok(records) => records,
        Err(error) => {
            eprintln!("altai-cli serve: could not fetch replay events: {error}");
            return respond(
                writer,
                id,
                None,
                Some(error_value(-32603, "journal_unavailable")),
            )
            .await;
        }
    };
    let events = records
        .into_iter()
        .map(|record| {
            json!({
                "chat_id": record.chat_id,
                "run_id": record.run_id,
                "seq": record.seq,
                "event": record.payload,
            })
        })
        .collect::<Vec<_>>();
    respond(
        writer,
        id,
        Some(json!({
            "events": events,
            "last_seq": summary.last_seq,
            "terminal_seq": summary.terminal_seq,
        })),
        None,
    )
    .await
}

async fn cancel_all_active(service: &AgentService<StdioHost>) {
    let actives: Vec<(String, String)> = {
        let coordinator = coordinator_guard(service.run_coordinator());
        coordinator
            .active_runs()
            .into_iter()
            .map(|(chat_id, run_id, _owner)| (chat_id, run_id))
            .collect()
    };
    for (chat_id, run_id) in actives {
        let _ = service.route_cancel(chat_id, run_id).await;
    }
}

async fn respond(
    writer: &Writer,
    id: Value,
    result: Option<Value>,
    error: Option<Value>,
) -> Result<(), String> {
    let mut value = json!({"jsonrpc":"2.0","id":id});
    if let Some(result) = result {
        value["result"] = result;
    }
    if let Some(error) = error {
        value["error"] = error;
    }
    write_framed(writer, &value)
}

fn error_value(code: i32, message: &str) -> Value {
    json!({"code":code,"message":message})
}

fn load_run_configuration(
    workspace: &WorkspacePaths,
) -> Result<altai_core::ResolvedAgentConfig, altai_core::AgentConfigError> {
    altai_core::load_agent_config(
        &workspace.root.join(".altai/config.toml"),
        &workspace.isanagent_state.join("config.toml"),
    )
}

/// Normalize host/UI config patch keys into CLI wire names.
/// Accepts camelCase HostPorts patches and ignores unknown keys so a single
/// extra field (or desktop alias) does not reject the whole update.
fn normalize_config_patch(
    params: &serde_json::Map<String, Value>,
) -> serde_json::Map<String, Value> {
    let mut out = serde_json::Map::new();
    for (key, value) in params {
        let canonical = match key.as_str() {
            "model" | "defaultModelId" | "default_model_id" | "default_model" => "model",
            "permission" | "permissionMode" | "permission_mode" => "permission",
            "provider" | "providerId" | "provider_id" => "provider",
            "base_url" | "baseUrl" => "base_url",
            _ => continue,
        };
        out.insert(canonical.to_string(), value.clone());
    }
    out
}

fn update_workspace_config(
    workspace: &WorkspacePaths,
    params: &serde_json::Map<String, Value>,
) -> Result<(), String> {
    let params = normalize_config_patch(params);
    if params.is_empty() {
        return Err("unsupported_config_patch".to_string());
    }
    let model = params.get("model").and_then(Value::as_str).map(str::trim);
    let permission = params
        .get("permission")
        .and_then(Value::as_str)
        .map(str::trim);
    let provider = params.get("provider").and_then(Value::as_str).map(str::trim);
    let base_url = params.get("base_url").and_then(Value::as_str).map(str::trim);
    if model.is_some_and(|value| value.len() > 512) {
        return Err("invalid_model".to_string());
    }
    if permission.is_some_and(|value| !matches!(value, "ask" | "auto-edit" | "plan")) {
        return Err("invalid_permission".to_string());
    }
    if provider.is_some_and(|value| provider_credentials::validate_provider_id(value).is_err()) {
        return Err("invalid_provider_id".to_string());
    }
    if base_url.is_some_and(|value| !valid_base_url(value)) {
        return Err("invalid_base_url".to_string());
    }

    let path = workspace.root.join(".altai/config.toml");
    let source = if path.exists() {
        std::fs::read_to_string(&path).map_err(|_| "configuration_unavailable".to_string())?
    } else {
        String::new()
    };
    let mut document = if source.trim().is_empty() {
        toml::Value::Table(toml::map::Map::new())
    } else {
        source
            .parse::<toml::Value>()
            .map_err(|_| "configuration_invalid".to_string())?
    };
    let root = document
        .as_table_mut()
        .ok_or_else(|| "configuration_invalid".to_string())?;
    let agent = root
        .entry("agent")
        .or_insert_with(|| toml::Value::Table(toml::map::Map::new()))
        .as_table_mut()
        .ok_or_else(|| "configuration_invalid".to_string())?;
    if let Some(model) = model {
        if model.is_empty() || model == "auto" {
            agent.remove("model");
        } else {
            agent.insert("model".to_string(), toml::Value::String(model.to_string()));
        }
    }
    if let Some(permission) = permission {
        agent.insert(
            "permission_mode".to_string(),
            toml::Value::String(permission.to_string()),
        );
    }
    if let Some(provider) = provider {
        agent.insert("provider".to_string(), toml::Value::String(provider.to_string()));
    }
    if let Some(base_url) = base_url {
        agent.insert("base_url".to_string(), toml::Value::String(base_url.to_string()));
    }
    let parent = path
        .parent()
        .ok_or_else(|| "configuration_unavailable".to_string())?;
    std::fs::create_dir_all(parent).map_err(|_| "configuration_unavailable".to_string())?;
    let temporary = path.with_extension("toml.tmp");
    let serialized =
        toml::to_string(&document).map_err(|_| "configuration_unavailable".to_string())?;
    std::fs::write(&temporary, serialized).map_err(|_| "configuration_unavailable".to_string())?;
    std::fs::rename(&temporary, &path).map_err(|_| "configuration_unavailable".to_string())?;
    Ok(())
}

fn valid_base_url(value: &str) -> bool {
    let value = value.trim();
    value.len() <= 2_048 && (value.starts_with("https://") || value.starts_with("http://"))
}

fn resolve_provider_credential(
    provider_id: &str,
    route: &SecureProviderRoute,
) -> Result<String, String> {
    if route.credential_scope == ProviderCredentialScope::EnvironmentTuple {
        let tuple_matches = std::env::var("ALTAI_PROVIDER").ok().as_deref() == Some(provider_id)
            && std::env::var("ALTAI_BASE_URL")
                .ok()
                .is_some_and(|base_url| base_url.trim() == route.base_url);
        if tuple_matches {
            return std::env::var("ALTAI_API_KEY")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| "api_key_not_configured".to_string());
        }
        return Err("api_key_not_configured".to_string());
    }

    let official = canonical_provider_base_url(provider_id);
    let provider_env = provider_api_key_env(provider_id);
    if Some(route.base_url.as_str()) == official {
        if let Some(value) = provider_env
            .and_then(|name| std::env::var(name).ok())
            .filter(|value| !value.trim().is_empty())
        {
            return Ok(value);
        }
    }
    let altai_tuple_matches = std::env::var("ALTAI_PROVIDER").ok().as_deref() == Some(provider_id)
        && match std::env::var("ALTAI_BASE_URL").ok() {
            Some(configured) => configured.trim() == route.base_url,
            None => Some(route.base_url.as_str()) == official,
        };
    if altai_tuple_matches {
        if let Some(value) = std::env::var("ALTAI_API_KEY")
            .ok()
            .filter(|value| !value.trim().is_empty())
        {
            return Ok(value);
        }
    }
    provider_credentials::get(provider_id)?
        .ok_or_else(|| "api_key_not_configured".to_string())
}

fn provider_api_key_env(provider_id: &str) -> Option<&'static str> {
    match provider_id {
        "openai" => Some("OPENAI_API_KEY"),
        "anthropic" => Some("ANTHROPIC_API_KEY"),
        "google" => Some("GOOGLE_API_KEY"),
        "xai" => Some("XAI_API_KEY"),
        "cerebras" => Some("CEREBRAS_API_KEY"),
        "groq" => Some("GROQ_API_KEY"),
        "deepseek" => Some("DEEPSEEK_API_KEY"),
        "mistral" => Some("MISTRAL_API_KEY"),
        "zai" | "zai-coding-plan" => Some("ZAI_API_KEY"),
        "openrouter" => Some("OPENROUTER_API_KEY"),
        _ => None,
    }
}

#[cfg(test)]
mod config_patch_tests {
    use super::{
        canonical_provider_base_url, normalize_config_patch, provider_api_key_env,
        secure_provider_route, work_session_title, ProviderCredentialScope, SecureProviderRoute,
    };
    use altai_core::{ConfigSource, ResolvedConfig};
    use serde_json::{json, Map, Value};

    #[test]
    fn accepts_hostports_camel_case_and_ignores_noise() {
        let mut params = Map::new();
        params.insert("permissionMode".into(), Value::String("auto-edit".into()));
        params.insert("defaultModelId".into(), Value::String("gpt-test".into()));
        params.insert("unknown_field".into(), Value::Bool(true));
        let normalized = normalize_config_patch(&params);
        assert_eq!(
            normalized.get("permission").and_then(Value::as_str),
            Some("auto-edit")
        );
        assert_eq!(
            normalized.get("model").and_then(Value::as_str),
            Some("gpt-test")
        );
        assert!(!normalized.contains_key("unknown_field"));
    }

    #[test]
    fn empty_after_normalize_still_errors_outside() {
        let params = Map::new();
        assert!(normalize_config_patch(&params).is_empty());
        let mut noise = Map::new();
        noise.insert("foo".into(), json!(1));
        assert!(normalize_config_patch(&noise).is_empty());
    }

    #[test]
    fn work_session_title_is_utf8_safe_and_journal_bounded() {
        let title = format!("{}é", "a".repeat(255));
        assert_eq!(work_session_title(&title), "a".repeat(255));
        assert!(work_session_title(&"x".repeat(400)).len() <= 256);
    }

    #[test]
    fn provider_routes_are_official_and_reject_mixed_config_sources() {
        assert_eq!(
            secure_provider_route("openai", None, None),
            Ok(SecureProviderRoute {
                base_url: "https://api.openai.com/v1/chat/completions".to_string(),
                credential_scope: ProviderCredentialScope::Official,
            })
        );
        let malicious = ResolvedConfig {
            value: "https://api.openai.com.evil.invalid/collect".to_string(),
            source: ConfigSource::ProjectConfig,
        };
        assert_eq!(
            secure_provider_route("openai", None, Some(&malicious)),
            Err("untrusted_base_url")
        );
        let repository_provider = ResolvedConfig {
            value: "openai".to_string(),
            source: ConfigSource::ProjectConfig,
        };
        let environment_endpoint = ResolvedConfig {
            value: "https://trusted-relay.example/v1/chat/completions".to_string(),
            source: ConfigSource::Environment,
        };
        assert_eq!(
            secure_provider_route(
                "openai",
                Some(&repository_provider),
                Some(&environment_endpoint),
            ),
            Err("untrusted_base_url")
        );
    }

    #[test]
    fn environment_credentials_are_provider_scoped() {
        assert_eq!(provider_api_key_env("openai"), Some("OPENAI_API_KEY"));
        assert_eq!(
            provider_api_key_env("anthropic"),
            Some("ANTHROPIC_API_KEY")
        );
        assert_eq!(provider_api_key_env("openai-compatible"), None);
        assert_eq!(canonical_provider_base_url("unknown"), None);
    }
}
