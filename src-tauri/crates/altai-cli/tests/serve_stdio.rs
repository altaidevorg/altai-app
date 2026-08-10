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
            .env("ALTAI_CLI_CREDENTIALS_DIR", workspace.join("credentials"))
            .env_remove("ALTAI_API_KEY")
            .env_remove("OPENAI_API_KEY")
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

fn create_fixture_skill_repo(root: &std::path::Path) -> std::path::PathBuf {
    let repo = root.join("fixture-skills");
    let skill = repo.join("skills").join("fixture-review");
    std::fs::create_dir_all(&skill).expect("fixture skill directory");
    std::fs::write(
        skill.join("SKILL.md"),
        "---\nname: fixture-review\ndescription: Fixture skill installed through stdio.\n---\n\nUse this fixture only in tests.\n",
    )
    .expect("fixture skill file");

    for args in [
        vec!["init"],
        vec!["add", "."],
        vec![
            "-c",
            "user.name=ALTAI Test",
            "-c",
            "user.email=altai-test@example.invalid",
            "commit",
            "-m",
            "fixture skill",
        ],
    ] {
        let status = Command::new("git")
            .args(args)
            .current_dir(&repo)
            .status()
            .expect("run fixture git command");
        assert!(status.success(), "fixture git command failed");
    }
    repo
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
    assert_eq!(
        updated["result"]["model"], "openai/gpt-test",
        "response: {updated}"
    );
    process.frame(json!({"jsonrpc":"2.0","id":3,"method":"config/get"}));
    assert_eq!(process.next()["result"]["model"], "openai/gpt-test");
    process.frame(json!({"jsonrpc":"2.0","id":4,"method":"config/update","params":{"permission":"auto-edit"}}));
    assert_eq!(process.next()["result"]["permission"], "auto-edit");
    process.frame(json!({"jsonrpc":"2.0","id":5,"method":"config/get"}));
    assert_eq!(process.next()["result"]["permission"], "auto-edit");
    process.frame(json!({"jsonrpc":"2.0","id":6,"method":"providers/status"}));
    let provider_status = process.next();
    assert!(provider_status["result"]["providers"][0]["connected"].is_boolean());
    assert!(provider_status["result"]["providers"][0]
        .get("api_key")
        .is_none());
    assert!(workspace.path().join(".altai/config.toml").exists());

    process.frame(json!({"jsonrpc":"2.0","id":7,"method":"shutdown"}));
    assert_eq!(process.next()["id"], 7);
    let _stderr = process.shutdown();
}

#[test]
fn provider_credentials_are_host_owned_and_never_returned_over_stdio() {
    let workspace = tempfile::tempdir().expect("workspace");
    let mut process = ServeProcess::spawn(workspace.path(), false);
    process.frame(initialize(json!(1)));
    let initialized = process.next();
    for capability in ["providers/status", "providers/connect", "providers/clear"] {
        assert!(initialized["result"]["capabilities"]
            .as_array()
            .expect("capabilities")
            .contains(&json!(capability)));
    }

    process.frame(json!({
        "jsonrpc":"2.0", "id":2, "method":"providers/connect",
        "params":{"provider_id":"anthropic","credential":"sk-ant-test-secret","base_url":"https://api.anthropic.test"}
    }));
    let connected = process.next();
    assert_eq!(connected["result"]["provider_id"], "anthropic", "response: {connected}");
    assert_eq!(connected["result"]["connected"], true);
    assert!(connected.to_string().contains("sk-ant-test-secret") == false);

    process.frame(json!({"jsonrpc":"2.0","id":3,"method":"providers/status"}));
    let status = process.next();
    assert_eq!(status["result"]["providers"][0]["provider_id"], "anthropic");
    assert_eq!(status["result"]["providers"][0]["connected"], true);
    assert!(status.get("credential").is_none());

    process.frame(json!({"jsonrpc":"2.0","id":4,"method":"providers/clear","params":{"provider_id":"anthropic"}}));
    assert_eq!(process.next()["result"]["cleared"], true);
    process.frame(json!({"jsonrpc":"2.0","id":5,"method":"providers/status"}));
    assert_eq!(process.next()["result"]["providers"][0]["connected"], false);

    process.frame(json!({"jsonrpc":"2.0","id":6,"method":"shutdown"}));
    assert_eq!(process.next()["id"], 6);
    let _stderr = process.shutdown();
}

#[test]
fn mcp_server_configuration_uses_the_native_lifecycle_protocol() {
    let workspace = tempfile::tempdir().expect("workspace");
    let mut process = ServeProcess::spawn(workspace.path(), false);
    process.frame(initialize(json!(1)));
    let initialized = process.next();
    for capability in ["mcp/servers/list", "mcp/servers/configure", "mcp/servers/enable", "mcp/servers/restart"] {
        assert!(initialized["result"]["capabilities"].as_array().expect("capabilities").contains(&json!(capability)));
    }
    process.frame(json!({"jsonrpc":"2.0", "id":2, "method":"mcp/servers/configure", "params":{"id":"files","config":{"name":"Files","command":"echo","args":[],"env":{},"enabled":true}}}));
    assert_eq!(process.next()["result"]["id"], "files");
    process.frame(json!({"jsonrpc":"2.0","id":3,"method":"mcp/servers/list"}));
    assert_eq!(process.next()["result"]["servers"][0]["enabled"], true);
    process.frame(json!({"jsonrpc":"2.0","id":4,"method":"mcp/servers/enable","params":{"id":"files","enabled":false}}));
    assert_eq!(process.next()["result"]["enabled"], false);
    process.frame(json!({"jsonrpc":"2.0","id":5,"method":"mcp/servers/list"}));
    assert_eq!(process.next()["result"]["servers"][0]["enabled"], false);
    process.frame(json!({"jsonrpc":"2.0","id":6,"method":"shutdown"}));
    assert_eq!(process.next()["id"], 6);
    let _stderr = process.shutdown();
}

#[test]
fn skills_install_and_list_use_the_stdio_host_protocol() {
    let root = tempfile::tempdir().expect("test root");
    let workspace = root.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("workspace directory");
    let source = create_fixture_skill_repo(root.path());
    let source = format!("file://{}", source.display());
    let mut process = ServeProcess::spawn(&workspace, false);

    process.frame(initialize(json!(1)));
    let initialized = process.next();
    for capability in ["skills/list", "skills/install"] {
        assert!(initialized["result"]["capabilities"]
            .as_array()
            .expect("capabilities")
            .contains(&json!(capability)));
    }

    process.frame(json!({"jsonrpc":"2.0","id":2,"method":"skills/list"}));
    assert_eq!(process.next()["result"]["skills"], json!([]));

    process.frame(json!({
        "jsonrpc":"2.0", "id":3, "method":"skills/install",
        "params":{"source": source, "skill":"fixture-review"}
    }));
    let installed = process.next();
    assert_eq!(installed["result"]["installed"], json!(["fixture-review"]));
    assert_eq!(installed["result"]["skills"][0]["name"], "fixture-review");
    assert_eq!(
        installed["result"]["skills"][0]["description"],
        "Fixture skill installed through stdio."
    );

    process.frame(json!({"jsonrpc":"2.0","id":4,"method":"skills/list"}));
    let listed = process.next();
    assert_eq!(listed["result"]["skills"][0]["name"], "fixture-review");

    process.frame(json!({"jsonrpc":"2.0","id":5,"method":"shutdown"}));
    assert_eq!(process.next()["id"], 5);
    let _stderr = process.shutdown();
}

#[test]
fn automation_rpc_persists_a_prompt_and_full_lifecycle() {
    let workspace = tempfile::tempdir().expect("workspace");
    let mut process = ServeProcess::spawn(workspace.path(), false);
    process.frame(initialize(json!(1)));
    let initialized = process.next();
    for capability in [
        "work/automations/list",
        "work/automations/create",
        "work/automations/update",
        "work/automations/trigger",
        "work/automations/pause",
        "work/automations/delete",
    ] {
        assert!(initialized["result"]["capabilities"]
            .as_array()
            .expect("capabilities")
            .contains(&json!(capability)));
    }

    process.frame(json!({
        "jsonrpc":"2.0", "id":2, "method":"work/automations/create",
        "params":{
            "chat_id":"automation-chat", "title":"Nightly checks",
            "prompt":"Run the project checks and report failures.",
            "schedule":{"kind":"every","every_ms":60000}
        }
    }));
    let created = process.next();
    let automation_id = created["result"]["automation"]["id"]
        .as_str()
        .expect("automation id")
        .to_string();
    assert_eq!(
        created["result"]["automation"]["prompt"],
        "Run the project checks and report failures."
    );

    // Commands are actor-backed. Retrying the read gives the actor a bounded
    // opportunity to persist the accepted create command without timing tests
    // against the scheduler tick interval.
    let mut listed = None;
    for request_id in 3..13 {
        process.frame(json!({"jsonrpc":"2.0","id":request_id,"method":"work/automations/list"}));
        let response = process.next();
        if response["result"]["automations"]
            .as_array()
            .is_some_and(|items| items.iter().any(|item| item["id"] == automation_id))
        {
            listed = Some(response);
            break;
        }
        thread::sleep(Duration::from_millis(25));
    }
    let listed = listed.expect("automation persisted");
    assert_eq!(listed["result"]["automations"][0]["title"], "Nightly checks");

    process.frame(json!({
        "jsonrpc":"2.0", "id":13, "method":"work/automations/update",
        "params":{"automation_id":automation_id,"title":"Updated checks","enabled":true}
    }));
    assert_eq!(process.next()["result"]["automation"]["title"], "Updated checks");
    process.frame(json!({
        "jsonrpc":"2.0", "id":14, "method":"work/automations/trigger",
        "params":{"automation_id":automation_id}
    }));
    assert_eq!(process.next()["result"]["accepted"], true);
    process.frame(json!({
        "jsonrpc":"2.0", "id":15, "method":"work/automations/pause",
        "params":{"automation_id":automation_id}
    }));
    assert_eq!(process.next()["result"]["accepted"], true);
    process.frame(json!({
        "jsonrpc":"2.0", "id":16, "method":"work/automations/delete",
        "params":{"automation_id":automation_id}
    }));
    assert_eq!(process.next()["result"]["removed"], true);
    process.frame(json!({"jsonrpc":"2.0","id":17,"method":"shutdown"}));
    assert_eq!(process.next()["id"], 17);
    let _stderr = process.shutdown();
}

#[test]
fn session_metadata_is_durable_and_can_be_archived_or_deleted() {
    let workspace = tempfile::tempdir().expect("workspace");
    let mut process = ServeProcess::spawn(workspace.path(), false);
    process.frame(initialize(json!(1)));
    let initialized = process.next();
    assert!(initialized["result"]["capabilities"]
        .as_array()
        .expect("capabilities")
        .contains(&json!("sessions/rename")));

    process.frame(json!({"jsonrpc":"2.0","id":2,"method":"sessions/create","params":{"chat_id":"session-meta","title":"Original"}}));
    let created = process.next();
    assert_eq!(
        created["result"]["title"], "Original",
        "response: {created}"
    );
    process.frame(json!({"jsonrpc":"2.0","id":3,"method":"sessions/rename","params":{"chat_id":"session-meta","title":"Renamed"}}));
    assert_eq!(process.next()["result"]["title"], "Renamed");
    process.frame(json!({"jsonrpc":"2.0","id":4,"method":"sessions/archive","params":{"chat_id":"session-meta"}}));
    assert_eq!(process.next()["result"]["archived"], true);
    process.frame(json!({"jsonrpc":"2.0","id":5,"method":"sessions/list","params":{"limit":10}}));
    assert_eq!(process.next()["result"]["sessions"][0]["title"], "Renamed");
    process.frame(json!({"jsonrpc":"2.0","id":6,"method":"sessions/delete","params":{"chat_id":"session-meta"}}));
    assert_eq!(process.next()["result"]["deleted"], true);
    process.frame(
        json!({"jsonrpc":"2.0","id":7,"method":"sessions/get","params":{"chat_id":"session-meta"}}),
    );
    assert_eq!(process.next()["error"]["message"], "session_not_found");
    process.frame(json!({"jsonrpc":"2.0","id":8,"method":"shutdown"}));
    assert_eq!(process.next()["id"], 8);
    let _stderr = process.shutdown();
}

#[test]
fn clarification_response_rejects_a_non_pending_ticket() {
    let workspace = tempfile::tempdir().expect("workspace");
    let mut process = ServeProcess::spawn(workspace.path(), false);
    process.frame(initialize(json!(1)));
    assert_eq!(process.next()["id"], 1);
    process.frame(json!({"jsonrpc":"2.0","id":2,"method":"clarification/respond","params":{"chat_id":"chat-test","text":"yes"}}));
    assert_eq!(
        process.next()["error"]["message"],
        "clarification_not_pending"
    );
    process.frame(json!({"jsonrpc":"2.0","id":3,"method":"clarification/respond","params":{"chat_id":"chat-test","action":"dismiss"}}));
    assert_eq!(
        process.next()["error"]["message"],
        "clarification_not_pending"
    );
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
fn task_run_rpc_persists_status_and_supports_lifecycle_actions() {
    let workspace = tempfile::tempdir().expect("workspace");
    let mut process = ServeProcess::spawn(workspace.path(), true);
    process.frame(initialize(json!(1)));
    let initialized = process.next();
    let capabilities = initialized["result"]["capabilities"]
        .as_array()
        .expect("capabilities");
    for capability in [
        "work/tasks/list",
        "work/tasks/create",
        "work/tasks/cancel",
        "work/tasks/retry",
        "work/tasks/remove",
    ] {
        assert!(
            capabilities.contains(&json!(capability)),
            "missing {capability}"
        );
    }

    process.frame(json!({
        "jsonrpc":"2.0",
        "id":2,
        "method":"work/tasks/create",
        "params":{"chat_id":"task-rpc","task_title":"Review pull request","prompt":"say hello"}
    }));
    let mut create_ack = false;
    let first_run_id = loop {
        let frame = process.next();
        if frame["id"] == 2 {
            assert!(frame.get("error").is_none(), "create response: {frame}");
            create_ack = frame["result"]["accepted"].as_bool() == Some(true);
            assert_eq!(frame["result"]["task_id"], "task-rpc");
        }
        if event_type(&frame) == Some("run_started") {
            break frame["params"]["run_id"]
                .as_str()
                .expect("first task run id")
                .to_string();
        }
    };
    assert!(create_ack);

    process.frame(json!({"jsonrpc":"2.0","id":3,"method":"work/tasks/list"}));
    let listed = loop {
        let frame = process.next();
        if frame["id"] == 3 {
            break frame;
        }
    };
    assert_eq!(listed["result"]["task_runs"][0]["id"], "task-rpc");
    assert_eq!(
        listed["result"]["task_runs"][0]["title"],
        "Review pull request"
    );
    assert!(
        matches!(
            listed["result"]["task_runs"][0]["status"].as_str(),
            Some("running" | "succeeded")
        ),
        "task status: {listed}"
    );

    process.frame(json!({
        "jsonrpc":"2.0", "id":4, "method":"work/tasks/cancel", "params":{"task_id":"task-rpc"}
    }));
    let mut cancel_ack = false;
    loop {
        let frame = process.next();
        if frame["id"] == 4 {
            cancel_ack = frame["result"]["accepted"].as_bool() == Some(true);
        }
        if event_type(&frame) == Some("run_terminated") {
            assert_eq!(frame["params"]["run_id"], first_run_id);
            assert_eq!(frame["params"]["event"]["outcome"]["kind"], "cancelled");
            break;
        }
    }
    while !cancel_ack {
        let frame = process.next();
        if frame["id"] == 4 {
            cancel_ack = frame["result"]["accepted"].as_bool() == Some(true);
        }
    }
    assert!(cancel_ack);

    process.frame(json!({
        "jsonrpc":"2.0", "id":5, "method":"work/tasks/retry", "params":{"task_id":"task-rpc"}
    }));
    let mut retry_ack = false;
    let second_run_id = loop {
        let frame = process.next();
        if frame["id"] == 5 {
            assert!(frame.get("error").is_none(), "retry response: {frame}");
            retry_ack = frame["result"]["accepted"].as_bool() == Some(true);
        }
        if event_type(&frame) == Some("run_started") {
            break frame["params"]["run_id"]
                .as_str()
                .expect("second task run id")
                .to_string();
        }
    };
    assert!(retry_ack);
    assert_ne!(first_run_id, second_run_id);

    loop {
        let frame = process.next();
        if event_type(&frame) == Some("run_terminated") {
            assert_eq!(frame["params"]["run_id"], second_run_id);
            break;
        }
    }

    process.frame(json!({
        "jsonrpc":"2.0", "id":6, "method":"work/tasks/remove", "params":{"task_id":"task-rpc"}
    }));
    loop {
        let frame = process.next();
        if frame["id"] == 6 {
            assert_eq!(frame["result"]["removed"], true);
            break;
        }
    }
    process.frame(json!({"jsonrpc":"2.0","id":7,"method":"work/tasks/list"}));
    loop {
        let frame = process.next();
        if frame["id"] == 7 {
            assert!(frame["result"]["task_runs"]
                .as_array()
                .expect("task runs")
                .is_empty());
            break;
        }
    }
    process.frame(json!({"jsonrpc":"2.0","id":8,"method":"shutdown"}));
    assert_eq!(process.next()["id"], 8);
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
            assert!(
                frame.get("result").is_some(),
                "first start should be accepted: {frame}"
            );
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

#[test]
fn edit_proposal_apply_writes_file_and_rejects_escape() {
    let workspace = tempfile::tempdir().expect("workspace");
    let mut process = ServeProcess::spawn(workspace.path(), false);
    process.frame(initialize(json!(1)));
    let init = process.next();
    assert_eq!(init["id"], 1);
    let caps = init["result"]["capabilities"]
        .as_array()
        .expect("capabilities array");
    assert!(
        caps.iter().any(|c| c.as_str() == Some("review/proposals/apply")),
        "initialize must advertise review/proposals/apply: {init}"
    );

    process.frame(json!({
        "jsonrpc":"2.0",
        "id":2,
        "method":"review/proposals/apply",
        "params":{
            "id":"prop-1",
            "path":"src/hello.txt",
            "kind":"create_file",
            "proposed_content":"hello from review"
        }
    }));
    let apply = process.next();
    assert_eq!(apply["id"], 2);
    assert!(
        apply.get("result").is_some(),
        "apply should succeed: {apply}"
    );
    let written = std::fs::read_to_string(workspace.path().join("src/hello.txt"))
        .expect("applied file");
    assert_eq!(written, "hello from review");

    process.frame(json!({
        "jsonrpc":"2.0",
        "id":3,
        "method":"review/proposals/apply",
        "params":{
            "id":"prop-1",
            "path":"src/hello.txt",
            "kind":"create_file",
            "proposed_content":"again"
        }
    }));
    let reapply = process.next();
    assert_eq!(reapply["id"], 3);
    assert_eq!(
        reapply.pointer("/error/message").and_then(Value::as_str),
        Some("already_applied")
    );

    process.frame(json!({
        "jsonrpc":"2.0",
        "id":4,
        "method":"review/proposals/apply",
        "params":{
            "id":"escape-1",
            "path":"../outside.txt",
            "proposed_content":"nope"
        }
    }));
    let escape = process.next();
    assert_eq!(escape["id"], 4);
    assert_eq!(
        escape.pointer("/error/message").and_then(Value::as_str),
        Some("path_outside_workspace")
    );

    process.frame(json!({
        "jsonrpc":"2.0",
        "id":5,
        "method":"review/proposals/deny",
        "params":{"id":"missing-ok"}
    }));
    let deny = process.next();
    assert_eq!(deny["id"], 5);
    assert_eq!(deny["result"]["denied"], true);

    process.frame(json!({"jsonrpc":"2.0","id":6,"method":"shutdown"}));
    assert_eq!(process.next()["id"], 6);
    let _stderr = process.shutdown();
}
