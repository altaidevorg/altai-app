use altai_core::EventJournal;
use altai_core::WorkspacePaths;
use altai_protocol::{
    encode_frame, validate_message, FrameDecoder, FrameLimits, ProtocolMessage, PROTOCOL_VERSION,
};
use isanagent::bus::{BusMessage, RunLifecycleEvent, TelemetryEvent};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;

type Writer = Arc<Mutex<tokio::io::Stdout>>;

#[derive(Default)]
struct ActiveRun {
    chat_id: String,
    run_id: Option<String>,
    seq: u64,
    terminal: bool,
    abort: Option<tokio::task::AbortHandle>,
    journal: Option<crate::journal_sink::JournalSink>,
}

pub async fn run(workspace: WorkspacePaths) -> Result<(), String> {
    let writer = Arc::new(Mutex::new(tokio::io::stdout()));
    let active = Arc::new(Mutex::new(None::<ActiveRun>));
    let mut stdin = tokio::io::stdin();
    let mut decoder = FrameDecoder::new(FrameLimits::default());
    let mut initialized = false;
    let terminal_pause = test_terminal_pause();
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
                cancel_active(&active, &writer).await;
                return Ok(());
            }
        };
        for frame in frames {
            let value: Value = match serde_json::from_slice(&frame) {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("altai-cli serve: malformed JSON: {error}");
                    cancel_active(&active, &writer).await;
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
                    respond(&writer, id, Some(json!({"protocol_min":PROTOCOL_VERSION,"protocol_max":PROTOCOL_VERSION,"capabilities":["initialize","workspace/status","config/get","models/list","agents/list","sessions/list","sessions/get","sessions/create","run/start","run/cancel","run/replay","checkpoints/list","checkpoints/restore","shutdown"]})), None).await?;
                }
                "workspace/status" if initialized => {
                    let journal_path = workspace.agent_event_journal_db();
                    respond(
                        &writer,
                        id,
                        Some(json!({
                            "root": workspace.root.display().to_string(),
                            "journal": journal_path.display().to_string(),
                            "active_run": active.lock().await.as_ref().and_then(|run| run.run_id.clone()),
                        })),
                        None,
                    ).await?;
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
                            .await?
                        }
                        Ok(None) => {
                            respond(
                                &writer,
                                id,
                                None,
                                Some(error_value(-32002, "session_not_found")),
                            )
                            .await?
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
                        Some(json!({
                            "agents": [{
                                "id": "altai",
                                "label": "ALTAI",
                                "description": "Workspace coding agent"
                            }]
                        })),
                        None,
                    )
                    .await?;
                }
                "models/list" if initialized => match load_run_configuration(&workspace) {
                    Ok(configuration) => {
                        let mut models = vec![json!({
                        "id": "auto",
                        "label": "Auto model",
                        "description": "Use the workspace provider configuration"
                        })];
                        if let Some(model) = configuration.model.as_ref() {
                            if model.value != "auto" {
                                models.push(json!({
                                    "id": model.value.clone(),
                                    "label": model.value.clone(),
                                    "description": format!("Resolved from {}", model.source.label())
                                }));
                            }
                        }
                        if let Some(model) = configuration.fallback_model.as_ref() {
                            if !models.iter().any(|item| {
                                item.get("id").and_then(Value::as_str) == Some(model.value.as_str())
                            }) {
                                models.push(json!({
                                    "id": model.value.clone(),
                                    "label": model.value.clone(),
                                    "description": format!("Fallback from {}", model.source.label())
                                }));
                            }
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
                                    "permissions": [{
                                        "id": "plan",
                                        "label": "Plan",
                                        "description": "Read-only mode supported by the current stdio host"
                                    }]
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
                "run/start" if initialized => {
                    let Some(params) = params.and_then(|value| value.as_object().cloned()) else {
                        respond(
                            &writer,
                            id,
                            None,
                            Some(error_value(-32602, "invalid_params")),
                        )
                        .await?;
                        continue;
                    };
                    let Some(chat_id) = params
                        .get("chat_id")
                        .and_then(Value::as_str)
                        .filter(|v| !v.trim().is_empty())
                        .map(str::to_string)
                    else {
                        respond(
                            &writer,
                            id,
                            None,
                            Some(error_value(-32602, "chat_id_required")),
                        )
                        .await?;
                        continue;
                    };
                    let Some(prompt) = params
                        .get("prompt")
                        .and_then(Value::as_str)
                        .filter(|v| !v.trim().is_empty())
                        .map(str::to_string)
                    else {
                        respond(
                            &writer,
                            id,
                            None,
                            Some(error_value(-32602, "prompt_required")),
                        )
                        .await?;
                        continue;
                    };
                    let agent = params
                        .get("agent")
                        .and_then(Value::as_str)
                        .unwrap_or("altai");
                    if agent != "altai" {
                        respond(
                            &writer,
                            id,
                            None,
                            Some(error_value(-32602, "unsupported_agent")),
                        )
                        .await?;
                        continue;
                    }
                    let model = params
                        .get("model")
                        .and_then(Value::as_str)
                        .filter(|value| *value != "auto");
                    if model.is_some_and(|value| value.trim().is_empty() || value.len() > 512) {
                        respond(
                            &writer,
                            id,
                            None,
                            Some(error_value(-32602, "invalid_model")),
                        )
                        .await?;
                        continue;
                    }
                    let permission = params
                        .get("permission")
                        .and_then(Value::as_str)
                        .unwrap_or("plan");
                    if permission != "plan" {
                        respond(
                            &writer,
                            id,
                            None,
                            Some(error_value(-32004, "permission_mode_unavailable")),
                        )
                        .await?;
                        continue;
                    }
                    if active.lock().await.is_some() {
                        respond(
                            &writer,
                            id,
                            None,
                            Some(error_value(-32004, "run_already_active")),
                        )
                        .await?;
                        continue;
                    }
                    let (observe_tx, mut observe_rx) = tokio::sync::mpsc::unbounded_channel();
                    let mut host = super::host_adapter::oneshot_host_config(
                        &workspace,
                        prompt,
                        Some(observe_tx),
                    );
                    host.model = model.map(str::to_string);
                    host.permission = Some(isanagent::host::HostPermissionMode::Plan);
                    host.resume = Some(chat_id.clone());
                    #[cfg(debug_assertions)]
                    {
                        if let Ok(response) = std::env::var("ALTAI_CLI_TEST_SCRIPTED_RESPONSE") {
                            host.scripted_responses = Some(vec![response]);
                        }
                    }
                    active.lock().await.replace(ActiveRun {
                        chat_id,
                        journal: crate::journal_sink::JournalSink::open(&workspace),
                        ..Default::default()
                    });
                    let observer_active = active.clone();
                    let observer_writer = writer.clone();
                    tokio::spawn(async move {
                        while let Some(message) = observe_rx.recv().await {
                            observe(message, &observer_active, &observer_writer, terminal_pause)
                                .await;
                        }
                    });
                    let task_active = active.clone();
                    let task_writer = writer.clone();
                    let task = tokio::spawn(async move {
                        let result = isanagent::host::run_oneshot(host).await;
                        // The bus normally owns the terminal event. Give its
                        // observer a chance to drain first, then provide an
                        // idempotent fallback so a host bug cannot strand an
                        // active run forever.
                        tokio::time::sleep(Duration::from_millis(50)).await;
                        if let Some(run) = task_active.lock().await.as_mut() {
                            if let (Some(journal), Ok(outcome)) =
                                (run.journal.as_mut(), result.as_ref())
                            {
                                journal.finalize(outcome);
                            }
                        }
                        terminal(
                            &task_active,
                            &task_writer,
                            if result.is_ok() {
                                "completed"
                            } else {
                                "failed"
                            },
                        )
                        .await;
                    });
                    if let Some(run) = active.lock().await.as_mut() {
                        run.abort = Some(task.abort_handle());
                    }
                    respond(&writer, id, Some(json!({"accepted":true})), None).await?;
                }
                "run/replay" if initialized => {
                    let params = params
                        .and_then(|value| value.as_object().cloned())
                        .unwrap_or_default();
                    let chat_id = params.get("chat_id").and_then(Value::as_str).unwrap_or("");
                    let run_id = params.get("run_id").and_then(Value::as_str).unwrap_or("");
                    let after_seq = params.get("after_seq").and_then(Value::as_u64).unwrap_or(0);
                    let limit = params.get("limit").and_then(Value::as_u64).unwrap_or(500);
                    if chat_id.trim().is_empty()
                        || run_id.trim().is_empty()
                        || !(1..=1_000).contains(&limit)
                    {
                        respond(
                            &writer,
                            id,
                            None,
                            Some(error_value(-32602, "invalid_replay_params")),
                        )
                        .await?;
                        continue;
                    }
                    let journal = match EventJournal::open(workspace.agent_event_journal_db()) {
                        Ok(journal) => journal,
                        Err(error) => {
                            eprintln!("altai-cli serve: could not open replay journal: {error}");
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
                    let summary = match journal.run_summary(run_id) {
                        Ok(Some(summary)) if summary.chat_id == chat_id => summary,
                        Ok(_) => {
                            respond(
                                &writer,
                                id,
                                None,
                                Some(error_value(-32002, "run_not_found")),
                            )
                            .await?;
                            continue;
                        }
                        Err(error) => {
                            eprintln!("altai-cli serve: could not inspect replay run: {error}");
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
                    let records = match journal.fetch_after(run_id, after_seq, limit as usize) {
                        Ok(records) => records,
                        Err(error) => {
                            eprintln!("altai-cli serve: could not fetch replay events: {error}");
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
                        &writer,
                        id,
                        Some(json!({
                            "events": events,
                            "last_seq": summary.last_seq,
                            "terminal_seq": summary.terminal_seq,
                        })),
                        None,
                    )
                    .await?;
                }
                "run/cancel" if initialized => {
                    let params = params
                        .and_then(|value| value.as_object().cloned())
                        .unwrap_or_default();
                    let run_id = params.get("run_id").and_then(Value::as_str).unwrap_or("");
                    let mut guard = active.lock().await;
                    let valid = guard
                        .as_ref()
                        .is_some_and(|run| run.run_id.as_deref() == Some(run_id) && !run.terminal);
                    if !valid {
                        respond(&writer, id, None, Some(error_value(-32002, "stale_run_id")))
                            .await?;
                        continue;
                    }
                    if let Some(run) = guard.as_mut() {
                        if let Some(abort) = run.abort.take() {
                            abort.abort();
                        }
                    }
                    drop(guard);
                    terminal(&active, &writer, "cancelled").await;
                    respond(&writer, id, Some(json!({"accepted":true})), None).await?;
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
                    cancel_active(&active, &writer).await;
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
    cancel_active(&active, &writer).await;
    Ok(())
}

async fn observe(
    message: BusMessage,
    active: &Arc<Mutex<Option<ActiveRun>>>,
    writer: &Writer,
    terminal_pause: Option<Duration>,
) {
    let (journal_event, has_journal) = {
        let mut guard = active.lock().await;
        match guard.as_mut().and_then(|run| run.journal.as_mut()) {
            Some(journal) => {
                journal.observe_bus_message(&message);
                (journal.take_last_event(), true)
            }
            None => (None, false),
        }
    };
    if let Some(event) = journal_event {
        emit_journal_event(active, writer, event).await;
    }

    // Journaling is best-effort. Retain the small legacy live-event surface
    // when SQLite could not be opened so a run is still usable, even though
    // replay is unavailable for that run.
    if !has_journal {
        match &message {
            BusMessage::RunLifecycle(RunLifecycleEvent::Started { run_id, chat_id }) => {
                if let Some(run) = active.lock().await.as_mut() {
                    run.chat_id = chat_id.clone();
                    run.run_id = Some(run_id.clone());
                }
                emit(
                    active,
                    writer,
                    json!({ "type": "run_started", "run_id": run_id }),
                )
                .await;
            }
            BusMessage::Outbound(outbound) => {
                let in_scope = active
                    .lock()
                    .await
                    .as_ref()
                    .is_some_and(|run| run.chat_id == outbound.chat_id);
                if in_scope {
                    emit(
                        active,
                        writer,
                        json!({
                            "type": "agent_message",
                            "content": outbound.content,
                            "role": "assistant",
                        }),
                    )
                    .await;
                }
            }
            BusMessage::Telemetry(TelemetryEvent::AgentThought {
                chat_id, thought, ..
            }) => {
                let in_scope = active
                    .lock()
                    .await
                    .as_ref()
                    .is_some_and(|run| run.chat_id == *chat_id);
                if in_scope {
                    emit(
                        active,
                        writer,
                        json!({ "type": "thinking", "content": thought }),
                    )
                    .await;
                }
            }
            BusMessage::Telemetry(TelemetryEvent::ToolCallStarted {
                chat_id,
                tool_name,
                args,
                tool_call_id,
                ..
            }) => {
                let in_scope = active
                    .lock()
                    .await
                    .as_ref()
                    .is_some_and(|run| run.chat_id == *chat_id);
                if in_scope {
                    let input =
                        serde_json::from_str(args).unwrap_or_else(|_| Value::String(args.clone()));
                    emit(
                        active,
                        writer,
                        json!({
                            "type": "tool_call_start",
                            "id": tool_call_id.clone().unwrap_or_else(|| tool_name.clone()),
                            "name": tool_name,
                            "input": input,
                        }),
                    )
                    .await;
                }
            }
            _ => {}
        }
    }

    if matches!(
        message,
        BusMessage::RunLifecycle(RunLifecycleEvent::Terminated { .. })
    ) {
        if let Some(pause) = terminal_pause {
            tokio::time::sleep(pause).await;
        }
        terminal(active, writer, "completed").await;
    }
}

async fn emit_journal_event(
    active: &Arc<Mutex<Option<ActiveRun>>>,
    writer: &Writer,
    event: altai_core::JournalEvent,
) {
    {
        let mut guard = active.lock().await;
        let Some(run) = guard.as_mut() else { return };
        if event.kind == "run_started" {
            run.chat_id = event.chat_id.clone();
            run.run_id = Some(event.run_id.clone());
        } else if run.chat_id != event.chat_id
            || run.run_id.as_deref() != Some(event.run_id.as_str())
        {
            return;
        }
        run.seq = event.seq;
    }
    let value = json!({
        "jsonrpc": "2.0",
        "method": "run/event",
        "params": {
            "chat_id": event.chat_id,
            "run_id": event.run_id,
            "seq": event.seq,
            "event": event.payload,
        }
    });
    let _ = write(writer, value).await;
}

/// Atomically claims terminal ownership and releases the run resources before
/// writing. Observers arriving after this point find no active run, so no
/// event can follow the terminal frame (or produce a second terminal frame).
async fn terminal(active: &Arc<Mutex<Option<ActiveRun>>>, writer: &Writer, outcome: &str) {
    let value = {
        let mut guard = active.lock().await;
        let Some(mut run) = guard.take() else { return };
        if run.terminal {
            return;
        }
        run.terminal = true;
        let journal_event = if let Some(journal) = run.journal.as_mut() {
            journal.finalize_outcome(outcome, None);
            journal.take_last_event()
        } else {
            None
        };
        if let Some(event) = journal_event {
            json!({
                "jsonrpc": "2.0",
                "method": "run/event",
                "params": {
                    "chat_id": event.chat_id,
                    "run_id": event.run_id,
                    "seq": event.seq,
                    "event": event.payload,
                }
            })
        } else {
            let Some(run_id) = run.run_id.filter(|id| !id.trim().is_empty()) else {
                // There is no valid run identity to put in a protocol event. The
                // `take` above is still deliberate: terminal resource cleanup
                // must not depend on receiving a lifecycle Started event.
                return;
            };
            run.seq += 1;
            json!({"jsonrpc":"2.0","method":"run/event","params":{"chat_id":run.chat_id,"run_id":run_id,"seq":run.seq,"event":{"type":"run_terminated","outcome":{"kind":outcome}}}})
        }
    };
    let _ = write(writer, value).await;
}

async fn cancel_active(active: &Arc<Mutex<Option<ActiveRun>>>, writer: &Writer) {
    if let Some(run) = active.lock().await.as_mut() {
        if let Some(abort) = run.abort.take() {
            abort.abort();
        }
    }
    terminal(active, writer, "cancelled").await;
}
async fn emit(active: &Arc<Mutex<Option<ActiveRun>>>, writer: &Writer, event: Value) {
    let mut guard = active.lock().await;
    let Some(run) = guard.as_mut() else { return };
    if run.terminal {
        return;
    }
    let Some(run_id) = run.run_id.clone() else {
        return;
    };
    run.seq += 1;
    let value = json!({"jsonrpc":"2.0","method":"run/event","params":{"chat_id":run.chat_id,"run_id":run_id,"seq":run.seq,"event":event}});
    drop(guard);
    let _ = write(writer, value).await;
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
    write(writer, value).await
}
async fn write(writer: &Writer, value: Value) -> Result<(), String> {
    let body = serde_json::to_vec(&value).map_err(|e| e.to_string())?;
    let frame = encode_frame(&body);
    let mut stdout = writer.lock().await;
    stdout.write_all(&frame).await.map_err(|e| e.to_string())?;
    stdout.flush().await.map_err(|e| e.to_string())
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

/// This hook is intentionally only present in non-release builds. It gives
/// the compiled integration binary a deterministic cancellation window after
/// the real scripted host has emitted its lifecycle events.
#[cfg(debug_assertions)]
fn test_terminal_pause() -> Option<Duration> {
    std::env::var("ALTAI_CLI_TEST_PAUSE_TERMINAL_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_millis)
}

#[cfg(not(debug_assertions))]
fn test_terminal_pause() -> Option<Duration> {
    None
}
