//! Stdio adapter for the host-neutral agent event sink.

use std::collections::{HashMap, HashSet};
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use altai_agent_service::{AgentEventEnvelope, AgentEventSink, AgentEventSinkError};
use altai_protocol::encode_frame;
use serde_json::json;

/// Shared framed stdout writer used by RPC responses and `run/event` frames.
///
/// Uses a synchronous mutex so event delivery never re-enters the Tokio
/// runtime via `block_on` (which deadlocks against the serve loop).
pub type SharedStdout = Arc<Mutex<io::Stdout>>;

/// Frames accepted envelopes as JSON-RPC `run/event` notifications on stdout.
///
/// Debug builds honor `ALTAI_CLI_TEST_PAUSE_TERMINAL_MS` before emitting a
/// terminal event so cancel can race the completed outcome — matching the
/// previous oneshot serve semantics used by integration tests.
#[derive(Clone)]
pub struct StdioEventSink {
    writer: SharedStdout,
    /// Live run identities observed on the wire (survives coordinator release).
    known_runs: Arc<Mutex<HashMap<String, String>>>,
    /// Run ids for which `run/cancel` won the terminal race.
    cancel_claims: Arc<Mutex<HashSet<String>>>,
    /// Run ids that already emitted a terminal frame on the wire.
    terminal_emitted: Arc<Mutex<HashSet<String>>>,
}

impl StdioEventSink {
    pub fn new(writer: SharedStdout) -> Self {
        Self {
            writer,
            known_runs: Arc::new(Mutex::new(HashMap::new())),
            cancel_claims: Arc::new(Mutex::new(HashSet::new())),
            terminal_emitted: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub fn chat_for_run(&self, run_id: &str) -> Option<String> {
        self.known_runs
            .lock()
            .ok()
            .and_then(|runs| runs.get(run_id).cloned())
    }

    /// Mark `run_id` as cancel-claimed. Returns `true` when the sink should
    /// still treat cancel as racing an in-flight completed terminal.
    pub fn claim_cancel(&self, run_id: &str) -> bool {
        if let Ok(mut claims) = self.cancel_claims.lock() {
            claims.insert(run_id.to_string());
        }
        self.terminal_emitted
            .lock()
            .map(|emitted| !emitted.contains(run_id))
            .unwrap_or(false)
    }

    fn remember_run(&self, chat_id: &str, run_id: &str) {
        if let Ok(mut runs) = self.known_runs.lock() {
            runs.insert(run_id.to_string(), chat_id.to_string());
        }
    }

    fn write_envelope(&self, mut envelope: AgentEventEnvelope) -> Result<(), AgentEventSinkError> {
        let (Some(run_id), Some(_seq)) = (envelope.run_id.clone(), envelope.seq) else {
            return Ok(());
        };
        self.remember_run(&envelope.chat_id, &run_id);

        if envelope.is_terminal() {
            if let Some(pause) = test_terminal_pause() {
                // Yield the Tokio worker while sleeping so cancel RPCs keep
                // progressing on other workers during the test race window.
                tokio::task::block_in_place(|| std::thread::sleep(pause));
            }

            let cancel_won = self
                .cancel_claims
                .lock()
                .map(|claims| claims.contains(&run_id))
                .unwrap_or(false);
            if cancel_won {
                if let Some(obj) = envelope.event.as_object_mut() {
                    obj.insert("type".into(), json!("run_terminated"));
                    obj.insert("outcome".into(), json!({ "kind": "cancelled" }));
                    obj.insert("run_id".into(), json!(run_id.clone()));
                }
            }

            let already = self
                .terminal_emitted
                .lock()
                .map(|mut emitted| !emitted.insert(run_id.clone()))
                .unwrap_or(true);
            if already {
                return Ok(());
            }
        }

        let value = json!({
            "jsonrpc": "2.0",
            "method": "run/event",
            "params": {
                "chat_id": envelope.chat_id,
                "run_id": run_id,
                "seq": envelope.seq,
                "event": envelope.event,
            }
        });
        write_framed(&self.writer, &value)
            .map_err(|error| AgentEventSinkError::Unavailable(error.to_string()))
    }
}

impl AgentEventSink for StdioEventSink {
    fn try_send(&self, envelope: AgentEventEnvelope) -> Result<(), AgentEventSinkError> {
        self.write_envelope(envelope)
    }
}

/// Write one framed JSON-RPC message to the shared stdout lock.
pub fn write_framed(writer: &SharedStdout, value: &serde_json::Value) -> Result<(), String> {
    let body = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    let frame = encode_frame(&body);
    let mut stdout = writer
        .lock()
        .map_err(|_| "stdout lock poisoned".to_string())?;
    stdout.write_all(&frame).map_err(|e| e.to_string())?;
    stdout.flush().map_err(|e| e.to_string())
}

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
