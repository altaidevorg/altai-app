use altai_protocol::{encode_frame, FrameDecoder, FrameLimits};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

struct ServeProcess {
    child: Child,
    stdin: Option<ChildStdin>,
    frames: Receiver<Value>,
    stderr: thread::JoinHandle<String>,
}

impl ServeProcess {
    fn spawn(workspace: &std::path::Path, pause_terminal: bool) -> Self {
        let mut command = Command::new(env!("CARGO_BIN_EXE_altai-cli"));
        command
            .args([
                "serve",
                "--stdio",
                "--protocol",
                "1",
                "--workspace",
                workspace.to_str().expect("UTF-8 workspace"),
            ])
            .env(
                "ALTAI_CLI_TEST_SCRIPTED_RESPONSE",
                "scripted assistant reply",
            )
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if pause_terminal {
            command.env("ALTAI_CLI_TEST_PAUSE_TERMINAL_MS", "1000");
        }
        let mut child = command.spawn().expect("spawn compiled altai-cli");
        let stdout = child.stdout.take().expect("child stdout");
        let stderr = child.stderr.take().expect("child stderr");
        let stdin = child.stdin.take().expect("child stdin");
        let (sender, frames) = mpsc::channel();
        thread::spawn(move || read_frames(stdout, sender));
        let stderr = thread::spawn(move || {
            let mut stderr = stderr;
            let mut text = String::new();
            stderr.read_to_string(&mut text).expect("read child stderr");
            text
        });
        Self {
            child,
            stdin: Some(stdin),
            frames,
            stderr,
        }
    }

    fn write(&mut self, bytes: &[u8]) {
        self.stdin
            .as_mut()
            .expect("stdin open")
            .write_all(bytes)
            .expect("write request");
        self.stdin
            .as_mut()
            .expect("stdin open")
            .flush()
            .expect("flush request");
    }

    fn frame(&mut self, value: Value) {
        self.write(&encode_frame(
            &serde_json::to_vec(&value).expect("JSON request"),
        ));
    }

    fn next(&self) -> Value {
        self.frames
            .recv_timeout(Duration::from_secs(10))
            .expect("timed out waiting for protocol frame")
    }

    fn shutdown(mut self) -> String {
        self.stdin.take();
        let status = self.child.wait().expect("wait for child");
        assert!(status.success(), "serve exited unsuccessfully: {status}");
        self.stderr.join().expect("join stderr reader")
    }
}

fn read_frames(mut stdout: impl Read, sender: mpsc::Sender<Value>) {
    let mut decoder = FrameDecoder::new(FrameLimits::default());
    let mut bytes = [0_u8; 17];
    loop {
        let count = stdout.read(&mut bytes).expect("read protocol stdout");
        if count == 0 {
            return;
        }
        for body in decoder.push(&bytes[..count]).expect("valid framed stdout") {
            sender
                .send(serde_json::from_slice(&body).expect("JSON stdout frame"))
                .ok();
        }
    }
}

fn initialize(id: Value) -> Value {
    json!({"jsonrpc":"2.0","id":id,"method":"initialize","params":{"protocol_min":1,"protocol_max":1}})
}

fn start(id: Value) -> Value {
    json!({"jsonrpc":"2.0","id":id,"method":"run/start","params":{"chat_id":"chat-test","prompt":"say hello"}})
}

fn event_type(value: &Value) -> Option<&str> {
    value.pointer("/params/event/type").and_then(Value::as_str)
}

fn replay_event_type(value: &Value) -> Option<&str> {
    value.pointer("/event/type").and_then(Value::as_str)
}

#[test]
fn config_update_persists_a_non_secret_model_setting() {
    let workspace = tempfile::tempdir().expect("workspace");
    let mut process = ServeProcess::spawn(workspace.path(), false);
    process.frame(initialize(json!(1)));
    let initialized = process.next();
    assert!(initialized["result"]["capabilities"]
        .as_array()
        .expect("capabilities")
        .contains(&json!("config/update")));

    process.frame(json!({"jsonrpc":"2.0","id":2,"method":"config/update","params":{"model":"openai/gpt-test"}}));
    let updated = process.next();
    assert_eq!(updated["result"]["model"], "openai/gpt-test", "response: {updated}");
    process.frame(json!({"jsonrpc":"2.0","id":3,"method":"config/get"}));
    assert_eq!(process.next()["result"]["model"], "openai/gpt-test");
    assert!(workspace.path().join(".altai/config.toml").exists());

    process.frame(json!({"jsonrpc":"2.0","id":4,"method":"shutdown"}));
    assert_eq!(process.next()["id"], 4);
    let _stderr = process.shutdown();
}

#[test]
fn clarification_response_rejects_a_non_pending_ticket() {
    let workspace = tempfile::tempdir().expect("workspace");
    let mut process = ServeProcess::spawn(workspace.path(), false);
    process.frame(initialize(json!(1)));
    assert_eq!(process.next()["id"], 1);
    process.frame(json!({"jsonrpc":"2.0","id":2,"method":"clarification/respond","params":{"chat_id":"chat-test","text":"yes"}}));
    assert_eq!(process.next()["error"]["message"], "clarification_not_pending");
    process.frame(json!({"jsonrpc":"2.0","id":3,"method":"clarification/respond","params":{"chat_id":"chat-test","action":"dismiss"}}));
    assert_eq!(process.next()["error"]["message"], "clarification_not_pending");
    process.frame(json!({"jsonrpc":"2.0","id":4,"method":"shutdown"}));
    assert_eq!(process.next()["id"], 4);
    let _stderr = process.shutdown();
}

#[test]
fn compiled_stdio_handles_split_and_multiple_frames_with_ordered_terminal_stream() {
    let workspace = tempfile::tempdir().expect("workspace");
    let mut process = ServeProcess::spawn(workspace.path(), false);

    let init = encode_frame(&serde_json::to_vec(&initialize(json!(1))).unwrap());
    process.write(&init[..9]); // split within the Content-Length header
    process.write(&init[9..]); // completes header and body across writes
    assert_eq!(process.next()["id"], 1);

    let models_request = encode_frame(
        &serde_json::to_vec(&json!({"jsonrpc":"2.0","id":"u","method":"models/list"})).unwrap(),
    );
    let run = encode_frame(&serde_json::to_vec(&start(json!(2))).unwrap());
    let mut combined = models_request;
    combined.extend(run); // two full frames in one stdin write
    process.write(&combined);

    let mut response_ids = Vec::new();
    let mut models_response = None;
    let mut events = Vec::new();
    while events.last().and_then(event_type) != Some("run_terminated") {
        let frame = process.next();
        if let Some(id) = frame.get("id") {
            response_ids.push(id.clone());
            if id.as_str() == Some("u") {
                models_response = frame.pointer("/result/models").cloned();
            }
        }
        if event_type(&frame).is_some() {
            events.push(frame);
        }
    }
    assert!(response_ids.contains(&json!("u")));
    assert!(models_response
        .and_then(|value| value.as_array().cloned())
        .is_some_and(|models| !models.is_empty()));
    assert!(response_ids.contains(&json!(2)));
    let kinds: Vec<_> = events.iter().filter_map(event_type).collect();
    assert_eq!(kinds.first(), Some(&"run_started"));
    assert!(kinds
        .iter()
        .any(|kind| *kind == "agent_message" || *kind == "tool_call_start"));
    assert_eq!(
        kinds
            .iter()
            .filter(|kind| **kind == "run_terminated")
            .count(),
        1
    );
    assert_eq!(kinds.last(), Some(&"run_terminated"));

    process.frame(json!({"jsonrpc":"2.0","id":3,"method":"shutdown"}));
    assert_eq!(process.next()["id"], 3);
    let _stderr = process.shutdown(); // host warnings are permitted on stderr only.
}

#[test]
fn invalid_version_is_typed_and_eof_shuts_down_cleanly() {
    let workspace = tempfile::tempdir().expect("workspace");
    let mut process = ServeProcess::spawn(workspace.path(), false);
    process.frame(json!({"jsonrpc":"2.0","id":"bad-version","method":"initialize","params":{"protocol_min":2,"protocol_max":2}}));
    let response = process.next();
    assert_eq!(response["id"], "bad-version");
    assert_eq!(response["error"]["code"], -32001);
    process.stdin.take(); // EOF is a clean protocol shutdown.
    let _stderr = process.shutdown();
}

#[test]
fn malformed_json_is_reported_to_stderr_and_connection_closes() {
    let workspace = tempfile::tempdir().expect("workspace");
    let mut process = ServeProcess::spawn(workspace.path(), false);
    process.write(&encode_frame(
        br#"{"jsonrpc":"2.0","id":7,"method":"initialize""#,
    ));
    process.stdin.take();
    let stderr = process.shutdown();
    assert!(stderr.contains("malformed JSON"), "stderr was: {stderr}");
}

#[test]
fn cancel_emits_one_terminal_event_and_no_later_run_events() {
    let workspace = tempfile::tempdir().expect("workspace");
    let mut process = ServeProcess::spawn(workspace.path(), true);
    process.frame(initialize(json!(1)));
    assert_eq!(process.next()["id"], 1);
    process.frame(start(json!(2)));

    let run_id = loop {
        let frame = process.next();
        if frame["id"] == 2 {
            continue;
        }
        if event_type(&frame) == Some("run_started") {
            break frame
                .pointer("/params/run_id")
                .and_then(Value::as_str)
                .map(str::to_string)
                .expect("started run id");
        }
    };
    process.frame(json!({"jsonrpc":"2.0","id":3,"method":"run/cancel","params":{"run_id":run_id}}));

    let mut terminal_count = 0;
    let mut saw_cancel_ack = false;
    while !saw_cancel_ack || terminal_count == 0 {
        let frame = process.next();
        if frame["id"] == 3 {
            saw_cancel_ack = frame.get("result").is_some();
        }
        if event_type(&frame) == Some("run_terminated") {
            terminal_count += 1;
            assert_eq!(frame["params"]["event"]["outcome"]["kind"], "cancelled");
        }
    }
    process.frame(json!({"jsonrpc":"2.0","id":4,"method":"shutdown"}));
    assert_eq!(process.next()["id"], 4);
    assert_eq!(terminal_count, 1);
    let _stderr = process.shutdown();
}

#[test]
fn completed_run_is_replayable_from_the_shared_journal() {
    let workspace = tempfile::tempdir().expect("workspace");
    let mut process = ServeProcess::spawn(workspace.path(), false);
    process.frame(initialize(json!(1)));
    let initialized = process.next();
    assert!(initialized["result"]["capabilities"]
        .as_array()
        .expect("capabilities")
        .contains(&json!("run/replay")));
    assert!(initialized["result"]["capabilities"]
        .as_array()
        .expect("capabilities")
        .contains(&json!("sessions/list")));
    process.frame(start(json!(2)));

    let mut run_id = None;
    let mut terminal_seq = 0;
    while terminal_seq == 0 {
        let frame = process.next();
        if event_type(&frame) == Some("run_started") {
            run_id = frame
                .pointer("/params/run_id")
                .and_then(Value::as_str)
                .map(str::to_string);
        }
        if event_type(&frame) == Some("run_terminated") {
            terminal_seq = frame
                .pointer("/params/seq")
                .and_then(Value::as_u64)
                .expect("terminal sequence");
        }
    }
    let run_id = run_id.expect("run id");
    process.frame(json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "run/replay",
        "params": {
            "chat_id": "chat-test",
            "run_id": run_id,
            "after_seq": 0,
            "limit": 500
        }
    }));
    let replay = process.next();
    assert_eq!(replay["id"], 3);
    assert_eq!(replay["result"]["terminal_seq"], terminal_seq);
    let kinds: Vec<_> = replay["result"]["events"]
        .as_array()
        .expect("replay events")
        .iter()
        .filter_map(replay_event_type)
        .collect();
    assert_eq!(kinds.first(), Some(&"run_started"));
    assert_eq!(kinds.last(), Some(&"run_terminated"));

    process.frame(json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "sessions/list",
        "params": { "limit": 10 }
    }));
    let sessions = process.next();
    assert_eq!(sessions["id"], 4);
    assert_eq!(sessions["result"]["sessions"][0]["chat_id"], "chat-test");
    assert_eq!(sessions["result"]["sessions"][0]["latest_run_id"], run_id);
    assert_eq!(
        sessions["result"]["sessions"][0]["terminal_seq"],
        terminal_seq
    );

    process.frame(json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "sessions/get",
        "params": { "chat_id": "chat-test" }
    }));
    let session = process.next();
    assert_eq!(session["id"], 5);
    assert_eq!(session["result"]["latest_run_id"], run_id);
    assert_eq!(session["result"]["terminal_seq"], terminal_seq);

    process.frame(json!({
        "jsonrpc": "2.0",
        "id": 6,
        "method": "sessions/messages",
        "params": { "chat_id": "chat-test" }
    }));
    let messages = process.next();
    assert_eq!(messages["id"], 6);
    assert_eq!(messages["result"]["messages"][0]["id"], "user:1");
    assert_eq!(messages["result"]["messages"][0]["role"], "user");
    assert_eq!(messages["result"]["messages"][0]["content"], "say hello");
    assert!(
        messages["result"]["messages"]
            .as_array()
            .expect("messages")
            .len()
            > 1
    );

    process.frame(json!({
        "jsonrpc": "2.0",
        "id": 7,
        "method": "sessions/truncate",
        "params": { "chat_id": "chat-test", "keep_user_messages": 1 }
    }));
    let truncated = process.next();
    assert_eq!(truncated["id"], 7);
    assert!(
        truncated["result"]["deleted_messages"]
            .as_u64()
            .unwrap_or(0)
            > 0
    );

    process.frame(json!({
        "jsonrpc": "2.0",
        "id": 8,
        "method": "sessions/messages",
        "params": { "chat_id": "chat-test" }
    }));
    let trimmed_messages = process.next();
    assert_eq!(trimmed_messages["id"], 8);
    assert_eq!(
        trimmed_messages["result"]["messages"]
            .as_array()
            .expect("messages")
            .len(),
        1
    );
    assert_eq!(trimmed_messages["result"]["messages"][0]["id"], "user:1");

    process.frame(json!({"jsonrpc":"2.0","id":9,"method":"shutdown"}));
    assert_eq!(process.next()["id"], 9);
    let _stderr = process.shutdown();
}

#[test]
fn retry_rewinds_the_latest_terminal_run_and_starts_a_new_one() {
    let workspace = tempfile::tempdir().expect("workspace");
    let mut process = ServeProcess::spawn(workspace.path(), false);
    process.frame(initialize(json!(1)));
    let initialized = process.next();
    assert!(initialized["result"]["capabilities"]
        .as_array()
        .expect("capabilities")
        .contains(&json!("run/retry")));

    process.frame(start(json!(2)));
    let first_run_id = loop {
        let frame = process.next();
        if event_type(&frame) == Some("run_started") {
            break frame["params"]["run_id"]
                .as_str()
                .expect("first run id")
                .to_string();
        }
    };
    loop {
        if event_type(&process.next()) == Some("run_terminated") {
            break;
        }
    }

    process.frame(json!({
        "jsonrpc":"2.0",
        "id":3,
        "method":"run/retry",
        "params":{"chat_id":"chat-test","run_id":first_run_id}
    }));
    let mut retry_ack = false;
    let second_run_id = loop {
        let frame = process.next();
        if frame["id"] == 3 {
            assert!(frame.get("error").is_none(), "retry response: {frame}");
            retry_ack = frame["result"]["accepted"].as_bool() == Some(true);
        }
        if event_type(&frame) == Some("run_started") {
            break frame["params"]["run_id"]
                .as_str()
                .expect("second run id")
                .to_string();
        }
    };
    assert!(retry_ack);
    assert_ne!(first_run_id, second_run_id);
    loop {
        if event_type(&process.next()) == Some("run_terminated") {
            break;
        }
    }

    process.frame(json!({
        "jsonrpc":"2.0",
        "id":4,
        "method":"run/retry",
        "params":{"chat_id":"chat-test","run_id":first_run_id}
    }));
    assert_eq!(process.next()["error"]["message"], "retry_not_latest_run");

    process.frame(json!({"jsonrpc":"2.0","id":5,"method":"shutdown"}));
    assert_eq!(process.next()["id"], 5);
    let _stderr = process.shutdown();
}

#[test]
fn second_chat_can_start_while_first_is_active() {
    let workspace = tempfile::tempdir().expect("workspace");
    let mut process = ServeProcess::spawn(workspace.path(), true);
    process.frame(initialize(json!(1)));
    assert_eq!(process.next()["id"], 1);

    process.frame(json!({
        "jsonrpc":"2.0",
        "id":2,
        "method":"run/start",
        "params":{"chat_id":"chat-a","prompt":"first"}
    }));
    let mut first_started = false;
    while !first_started {
        let frame = process.next();
        if frame["id"] == 2 {
            assert!(frame.get("result").is_some(), "first start should be accepted: {frame}");
            continue;
        }
        if event_type(&frame) == Some("run_started") {
            assert_eq!(frame["params"]["chat_id"], "chat-a");
            first_started = true;
        }
    }

    process.frame(json!({
        "jsonrpc":"2.0",
        "id":3,
        "method":"run/start",
        "params":{"chat_id":"chat-b","prompt":"second"}
    }));
    let mut second_started = false;
    while !second_started {
        let frame = process.next();
        if frame["id"] == 3 {
            assert!(
                frame.get("result").is_some(),
                "second chat must not hit process-wide single-active gate: {frame}"
            );
            continue;
        }
        if event_type(&frame) == Some("run_started") && frame["params"]["chat_id"] == "chat-b" {
            second_started = true;
        }
    }

    process.frame(json!({"jsonrpc":"2.0","id":4,"method":"shutdown"}));
    // Shutdown cancels active leases first, so terminal run/event frames may
    // arrive before the shutdown acknowledgement.
    let shutdown = loop {
        let frame = process.next();
        if frame["id"] == 4 {
            break frame;
        }
    };
    assert!(shutdown.get("result").is_some(), "shutdown ack: {shutdown}");
    let _stderr = process.shutdown();
}

#[test]
fn run_start_accepts_ask_permission_mode() {
    let workspace = tempfile::tempdir().expect("workspace");
    let mut process = ServeProcess::spawn(workspace.path(), false);
    process.frame(initialize(json!(1)));
    assert_eq!(process.next()["id"], 1);

    process.frame(json!({
        "jsonrpc":"2.0",
        "id":2,
        "method":"run/start",
        "params":{
            "chat_id":"chat-ask",
            "prompt":"say hello",
            "permission":"ask"
        }
    }));

    let mut saw_ack = false;
    let mut saw_terminal = false;
    while !saw_ack || !saw_terminal {
        let frame = process.next();
        if frame["id"] == 2 {
            assert!(
                frame.get("result").is_some(),
                "ask permission must be accepted: {frame}"
            );
            assert_ne!(
                frame.pointer("/error/message").and_then(Value::as_str),
                Some("permission_mode_unavailable")
            );
            saw_ack = true;
        }
        if event_type(&frame) == Some("run_terminated") {
            saw_terminal = true;
        }
    }

    process.frame(json!({"jsonrpc":"2.0","id":3,"method":"shutdown"}));
    assert_eq!(process.next()["id"], 3);
    let _stderr = process.shutdown();
}
