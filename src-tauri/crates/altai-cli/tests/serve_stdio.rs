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

#[test]
fn compiled_stdio_handles_split_and_multiple_frames_with_ordered_terminal_stream() {
    let workspace = tempfile::tempdir().expect("workspace");
    let mut process = ServeProcess::spawn(workspace.path(), false);

    let init = encode_frame(&serde_json::to_vec(&initialize(json!(1))).unwrap());
    process.write(&init[..9]); // split within the Content-Length header
    process.write(&init[9..]); // completes header and body across writes
    assert_eq!(process.next()["id"], 1);

    let unsupported = encode_frame(
        &serde_json::to_vec(&json!({"jsonrpc":"2.0","id":"u","method":"models/list"})).unwrap(),
    );
    let run = encode_frame(&serde_json::to_vec(&start(json!(2))).unwrap());
    let mut combined = unsupported;
    combined.extend(run); // two full frames in one stdin write
    process.write(&combined);

    let mut response_ids = Vec::new();
    let mut events = Vec::new();
    while events.last().and_then(event_type) != Some("run_terminated") {
        let frame = process.next();
        if let Some(id) = frame.get("id") {
            response_ids.push(id.clone());
        }
        if event_type(&frame).is_some() {
            events.push(frame);
        }
    }
    assert!(response_ids.contains(&json!("u")));
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
