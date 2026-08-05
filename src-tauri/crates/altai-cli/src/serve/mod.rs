use std::io;
use std::sync::{Arc, Mutex};

use altai_agent_service::{coordinator_guard, AgentService, DocumentPart, RunCoordinator, SharedRunCoordinator};
use base64::Engine;
use altai_core::EventJournal;
use altai_core::WorkspacePaths;
use altai_protocol::{
    validate_message, FrameDecoder, FrameLimits, ProtocolMessage, PROTOCOL_VERSION,
};
use serde_json::{json, Value};
use tokio::io::AsyncReadExt;

use crate::stdio_host::StdioHost;
use crate::stdio_sink::{write_framed, SharedStdout, StdioEventSink};

mod provider_credentials;

type Writer = SharedStdout;

const MAX_RUN_ATTACHMENTS: usize = 4;
const MAX_RUN_ATTACHMENT_ENCODED_BYTES: usize = 2 * 1024 * 1024;
const MAX_RUN_ATTACHMENTS_ENCODED_BYTES: usize = 3 * 1024 * 1024;

pub async fn run(workspace: WorkspacePaths) -> Result<(), String> {
    let writer: Writer = Arc::new(Mutex::new(io::stdout()));
    let event_sink = Arc::new(StdioEventSink::new(writer.clone()));
    let run_coordinator: SharedRunCoordinator =
        Arc::new(std::sync::Mutex::new(RunCoordinator::default()));
    let host = Arc::new(StdioHost::new(
        workspace.clone(),
        event_sink.clone() as Arc<dyn altai_agent_service::AgentEventSink>,
        run_coordinator.clone(),
    ));
    let service = Arc::new(AgentService::with_coordinator(
        host.clone(),
        run_coordinator.clone(),
    ));

    let mut stdin = tokio::io::stdin();
    let mut decoder = FrameDecoder::new(FrameLimits::default());
    let mut initialized = false;
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
                            .map(|value| value.value)
                            .unwrap_or_else(|| "openai".to_string());
                        // The native host exposes only the boolean outcome of
                        // credential resolution; raw keys never cross stdio.
                        let connected = resolve_provider_credential(&provider_id)
                            .map(|credential| !credential.trim().is_empty())
                            .unwrap_or(false);
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
        Ok(Some(summary)) if summary.terminal_seq.is_none() => summary,
        Ok(Some(_)) => {
            return respond(
                writer,
                id,
                None,
                Some(error_value(-32002, "task_run_not_active")),
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
    let terminal_still_open = event_sink.claim_cancel(&summary.run_id);
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

    let configuration = load_run_configuration(workspace).unwrap_or_default();
    let provider_name = configuration
        .provider
        .map(|value| value.value)
        .unwrap_or_else(|| "openai".to_string());
    let model_name = model
        .map(str::to_string)
        .or_else(|| configuration.model.map(|value| value.value))
        .unwrap_or_else(|| "gpt-4o-mini".to_string());
    let api_key = resolve_provider_credential(&provider_name).unwrap_or_default();
    // Scripted CI/test runs intentionally omit provider credentials.
    let scripted =
        cfg!(debug_assertions) && std::env::var_os("ALTAI_CLI_TEST_SCRIPTED_RESPONSE").is_some();
    if api_key.trim().is_empty() && !scripted {
        return respond(
            writer,
            id,
            None,
            Some(error_value(-32603, "api_key_not_configured")),
        )
        .await;
    }
    let workspace_path = workspace.root.to_str().map(str::to_string);
    let base_url = configuration.base_url.map(|value| value.value);
    let permission = permission.to_string();
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

    // Admission returns once the user message is on the bus; the agent loop
    // continues in the background and emits framed `run/event` notifications.
    match service
        .route_send(
            &provider_name,
            &api_key,
            &model_name,
            None,
            base_url.as_deref(),
            workspace_path.as_deref(),
            Some(&permission),
            None,
            None,
            prompt,
            images,
            documents,
            chat_id.clone(),
            queue,
        )
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

fn update_workspace_config(
    workspace: &WorkspacePaths,
    params: &serde_json::Map<String, Value>,
) -> Result<(), String> {
    if params.is_empty()
        || !params.keys().all(|key| {
            matches!(key.as_str(), "model" | "permission" | "provider" | "base_url")
        })
    {
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

fn resolve_provider_credential(provider_id: &str) -> Result<String, String> {
    std::env::var("ALTAI_API_KEY")
        .or_else(|_| std::env::var("OPENAI_API_KEY"))
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(Ok)
        .unwrap_or_else(|| provider_credentials::get(provider_id)?.ok_or_else(|| "api_key_not_configured".to_string()))
}
