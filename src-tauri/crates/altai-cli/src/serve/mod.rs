use std::io;
use std::sync::{Arc, Mutex};

use altai_agent_service::{
    coordinator_guard, AgentService, RunCoordinator, SharedRunCoordinator,
};
use altai_core::EventJournal;
use altai_core::WorkspacePaths;
use altai_protocol::{
    validate_message, FrameDecoder, FrameLimits, ProtocolMessage, PROTOCOL_VERSION,
};
use serde_json::{json, Value};
use tokio::io::AsyncReadExt;

use crate::stdio_host::StdioHost;
use crate::stdio_sink::{write_framed, SharedStdout, StdioEventSink};

type Writer = SharedStdout;

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
    let service = Arc::new(AgentService::with_coordinator(host, run_coordinator.clone()));

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
                                "agents/list",
                                "sessions/list",
                                "sessions/get",
                                "sessions/create",
                                "run/start",
                                "run/cancel",
                                "run/steer",
                                "run/replay",
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
                    respond(&writer, id, Some(json!({"chat_id": chat_id})), None).await?;
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
                    match journal.list_chat_summaries(limit as usize) {
                        Ok(sessions) => {
                            let sessions = sessions
                                .into_iter()
                                .map(|session| {
                                    json!({
                                        "chat_id": session.chat_id,
                                        "latest_run_id": session.latest_run_id,
                                        "last_seq": session.last_seq,
                                        "terminal_seq": session.terminal_seq,
                                        "updated_at_ms": session.updated_at_ms,
                                    })
                                })
                                .collect::<Vec<_>>();
                            respond(&writer, id, Some(json!({"sessions": sessions})), None)
                                .await?;
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
                    match journal.latest_run_summary_for_chat(chat_id) {
                        Ok(Some(summary)) => {
                            respond(
                                &writer,
                                id,
                                Some(json!({
                                    "chat_id": summary.chat_id,
                                    "latest_run_id": summary.run_id,
                                    "last_seq": summary.last_seq,
                                    "terminal_seq": summary.terminal_seq,
                                })),
                                None,
                            )
                            .await?;
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
                "config/get" if initialized => match load_run_configuration(&workspace) {
                    Ok(configuration) => {
                        respond(
                            &writer,
                            id,
                            Some(json!({
                                "agent": "altai",
                                "model": configuration.model.map(|value| value.value).unwrap_or_else(|| "auto".to_string()),
                                "permission": "plan",
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
                                    })),
                                    None,
                                )
                                .await?;
                            }
                            Err(error) => {
                                eprintln!("altai-cli serve: could not reload configuration: {error}");
                                respond(&writer, id, None, Some(error_value(-32603, "configuration_unavailable"))).await?;
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
                            respond(
                                &writer,
                                id,
                                None,
                                Some(error_value(-32002, &error)),
                            )
                            .await?;
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
                            respond(
                                &writer,
                                id,
                                None,
                                Some(error_value(-32002, &error)),
                            )
                            .await?;
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
        return respond(
            writer,
            id,
            None,
            Some(error_value(-32602, "invalid_model")),
        )
        .await;
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
    let api_key = std::env::var("ALTAI_API_KEY")
        .or_else(|_| std::env::var("OPENAI_API_KEY"))
        .unwrap_or_default();
    // Scripted CI/test runs intentionally omit provider credentials.
    let scripted = cfg!(debug_assertions)
        && std::env::var_os("ALTAI_CLI_TEST_SCRIPTED_RESPONSE").is_some();
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
            Vec::new(),
            Vec::new(),
            chat_id,
            queue,
        )
        .await
    {
        Ok(ack) => {
            respond(
                writer,
                id,
                Some(json!({
                    "accepted": true,
                    "run_id": ack.run_id,
                    "queued": ack.queued,
                })),
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
    if params.len() != 1 || !params.contains_key("model") {
        return Err("unsupported_config_patch".to_string());
    }
    let model = params
        .get("model")
        .and_then(Value::as_str)
        .ok_or_else(|| "invalid_model".to_string())?
        .trim();
    if model.len() > 512 {
        return Err("invalid_model".to_string());
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
    if model.is_empty() || model == "auto" {
        agent.remove("model");
    } else {
        agent.insert("model".to_string(), toml::Value::String(model.to_string()));
    }
    let parent = path.parent().ok_or_else(|| "configuration_unavailable".to_string())?;
    std::fs::create_dir_all(parent).map_err(|_| "configuration_unavailable".to_string())?;
    let temporary = path.with_extension("toml.tmp");
    let serialized = toml::to_string(&document)
        .map_err(|_| "configuration_unavailable".to_string())?;
    std::fs::write(&temporary, serialized)
        .map_err(|_| "configuration_unavailable".to_string())?;
    std::fs::rename(&temporary, &path).map_err(|_| "configuration_unavailable".to_string())?;
    Ok(())
}
