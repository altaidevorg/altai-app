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
                    respond(&writer, id, Some(json!({"protocol_min":PROTOCOL_VERSION,"protocol_max":PROTOCOL_VERSION,"capabilities":["initialize","run/start","run/cancel","shutdown"]})), None).await?;
                }
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
                    host.resume = Some(chat_id.clone());
                    #[cfg(debug_assertions)]
                    {
                        if let Ok(response) = std::env::var("ALTAI_CLI_TEST_SCRIPTED_RESPONSE") {
                            host.scripted_responses = Some(vec![response]);
                        }
                    }
                    active.lock().await.replace(ActiveRun {
                        chat_id,
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
    match message {
        BusMessage::RunLifecycle(RunLifecycleEvent::Started { run_id, chat_id }) => {
            let mut guard = active.lock().await;
            if let Some(run) = guard.as_mut() {
                run.chat_id = chat_id;
                run.run_id = Some(run_id);
            }
            drop(guard);
            emit(active, writer, json!({"type":"run_started"})).await;
        }
        BusMessage::RunLifecycle(RunLifecycleEvent::Terminated { .. }) => {
            if let Some(pause) = terminal_pause {
                tokio::time::sleep(pause).await;
            }
            terminal(active, writer, "completed").await
        }
        BusMessage::Outbound(outbound) => {
            emit(
                active,
                writer,
                json!({"type":"agent_message","role":"assistant","content":outbound.content}),
            )
            .await
        }
        BusMessage::Telemetry(TelemetryEvent::AgentThought { thought, .. }) => {
            emit(active, writer, json!({"type":"thinking","content":thought})).await
        }
        BusMessage::Telemetry(TelemetryEvent::ToolCallStarted {
            tool_name,
            tool_call_id,
            ..
        }) => {
            emit(
                active,
                writer,
                json!({"type":"tool_call_start","id":tool_call_id,"name":tool_name}),
            )
            .await
        }
        _ => {}
    }
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
        let Some(run_id) = run.run_id.filter(|id| !id.trim().is_empty()) else {
            // There is no valid run identity to put in a protocol event. The
            // `take` above is still deliberate: terminal resource cleanup
            // must not depend on receiving a lifecycle Started event.
            return;
        };
        run.seq += 1;
        json!({"jsonrpc":"2.0","method":"run/event","params":{"chat_id":run.chat_id,"run_id":run_id,"seq":run.seq,"event":{"type":"run_terminated","outcome":{"kind":outcome}}}})
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
