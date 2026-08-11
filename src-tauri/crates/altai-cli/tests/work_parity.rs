use altai_protocol::{encode_frame, FrameDecoder, FrameLimits};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

struct NativeHost {
    child: Child,
    stdin: Option<ChildStdin>,
    frames: Receiver<Value>,
    stderr: thread::JoinHandle<String>,
}

impl NativeHost {
    fn spawn(workspace: &std::path::Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_altai-cli"))
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
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn native host");
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

    fn frame(&mut self, value: Value) {
        let bytes = encode_frame(&serde_json::to_vec(&value).expect("JSON request"));
        let stdin = self.stdin.as_mut().expect("stdin open");
        stdin.write_all(&bytes).expect("write request");
        stdin.flush().expect("flush request");
    }

    fn next(&self) -> Value {
        self.frames
            .recv_timeout(Duration::from_secs(10))
            .expect("timed out waiting for protocol frame")
    }

    fn shutdown(mut self, id: i64) {
        self.frame(json!({"jsonrpc":"2.0","id":id,"method":"shutdown"}));
        assert_eq!(self.next()["id"], id);
        self.stdin.take();
        let status = self.child.wait().expect("wait for child");
        let stderr = self.stderr.join().expect("join stderr reader");
        assert!(status.success(), "serve failed: {status}; stderr: {stderr}");
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

fn initialize(host: &mut NativeHost, id: i64) {
    host.frame(json!({
        "jsonrpc":"2.0",
        "id":id,
        "method":"initialize",
        "params":{"protocol_min":1,"protocol_max":1}
    }));
    assert_eq!(host.next()["id"], id);
}

/// Host-storage parity gate for the canonical core, direct CLI, and stdio RPC
/// paths. This harness does not exercise Tauri commands, renderer UI, the
/// VS Code webview, or Open Run inspection; those require their own adapters
/// and integration coverage.
#[test]
fn core_cli_and_stdio_share_one_restart_safe_work_lifecycle() {
    let workspace = tempfile::tempdir().expect("workspace");
    let paths = altai_core::resolve_workspace(Some(workspace.path())).expect("workspace paths");
    let project_id = paths
        .root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("workspace")
        .to_string();
    let work_id = {
        let store = altai_core::WorkStore::open(&paths.work_db()).expect("open canonical work.db");
        store
            .ensure_project(&project_id, &project_id, &paths.root.to_string_lossy())
            .expect("project");
        store
            .create_work(altai_core::CreateWorkInput {
                project_id,
                title: "Host-storage durable Work".into(),
                description: "Created through the canonical core storage route".into(),
                acceptance_criteria: "Same ID survives every host and human retry".into(),
                assignee_ref: Some("agent:altai".into()),
            })
            .expect("create core Work")
            .id
    };

    // Task Runs are a legacy migration source in a sibling database. Canonical
    // Work operations must leave them inspectable until an explicit import.
    let journal = altai_core::EventJournal::open(paths.agent_event_journal_db())
        .expect("open legacy task-run journal");
    journal
        .create_task_run("legacy-task-chat", "Legacy task run sentinel")
        .expect("legacy task run");
    drop(journal);

    let mut host = NativeHost::spawn(workspace.path());
    initialize(&mut host, 1);
    host.frame(json!({
        "jsonrpc":"2.0","id":2,"method":"work/get","params":{"workId":work_id}
    }));
    let fetched = host.next();
    assert_eq!(fetched["result"]["id"], work_id);
    assert_eq!(fetched["result"]["state"], "backlog");
    host.frame(json!({
        "jsonrpc":"2.0","id":3,"method":"work/transition",
        "params":{"workId":work_id,"expectedRevision":fetched["result"]["revision"],"nextState":"ready"}
    }));
    let ready = host.next();
    host.frame(json!({
        "jsonrpc":"2.0","id":4,"method":"work/start",
        "params":{"workId":work_id,"expectedRevision":ready["result"]["revision"]}
    }));
    let started = host.next();
    host.frame(json!({
        "jsonrpc":"2.0","id":5,"method":"work/ready-for-review",
        "params":{"workId":work_id,"expectedRevision":started["result"]["revision"]}
    }));
    let in_review = host.next()["result"].clone();
    assert_eq!(in_review["state"], "in_review");
    assert_ne!(in_review["state"], "done");
    host.shutdown(6);

    let mut restarted = NativeHost::spawn(workspace.path());
    initialize(&mut restarted, 7);
    restarted.frame(json!({
        "jsonrpc":"2.0","id":8,"method":"work/get","params":{"workId":work_id}
    }));
    assert_eq!(restarted.next()["result"], in_review);
    let return_request = json!({
        "jsonrpc":"2.0","id":9,"method":"work/review",
        "params":{
            "workId":work_id,
            "expectedRevision":in_review["revision"],
            "accept":false,
            "guidance":"Add the parity acceptance gate"
        }
    });
    restarted.frame(return_request.clone());
    let returned = restarted.next()["result"].clone();
    assert_eq!(returned["state"], "ready");
    let mut retry_return = return_request;
    retry_return["id"] = json!(10);
    restarted.frame(retry_return);
    assert_eq!(restarted.next()["result"], returned);
    restarted.shutdown(11);

    // Direct CLI opens work.db in a fresh process, independently from stdio.
    let cli_show = Command::new(env!("CARGO_BIN_EXE_altai-cli"))
        .args([
            "work",
            "show",
            &work_id,
            "--workspace",
            workspace.path().to_str().expect("UTF-8 workspace"),
            "--json",
        ])
        .output()
        .expect("run direct CLI show");
    assert!(
        cli_show.status.success(),
        "CLI stderr: {}",
        String::from_utf8_lossy(&cli_show.stderr)
    );
    let cli_work: Value = serde_json::from_slice(&cli_show.stdout).expect("CLI Work JSON");
    assert_eq!(cli_work["id"], work_id);
    assert_eq!(cli_work["state"], "ready");

    let mut final_host = NativeHost::spawn(workspace.path());
    initialize(&mut final_host, 12);
    final_host.frame(json!({
        "jsonrpc":"2.0","id":13,"method":"work/start",
        "params":{"workId":work_id,"expectedRevision":returned["revision"]}
    }));
    let retry_attempt = final_host.next();
    final_host.frame(json!({
        "jsonrpc":"2.0","id":14,"method":"work/ready-for-review",
        "params":{"workId":work_id,"expectedRevision":retry_attempt["result"]["revision"]}
    }));
    let second_review = final_host.next();
    let accept_request = json!({
        "jsonrpc":"2.0","id":15,"method":"work/review",
        "params":{
            "workId":work_id,
            "expectedRevision":second_review["result"]["revision"],
            "accept":true,
            "guidance":"Parity evidence accepted"
        }
    });
    final_host.frame(accept_request.clone());
    let accepted = final_host.next()["result"].clone();
    assert_eq!(accepted["state"], "done");
    let mut retry_accept = accept_request;
    retry_accept["id"] = json!(16);
    final_host.frame(retry_accept);
    assert_eq!(final_host.next()["result"], accepted);
    final_host.shutdown(17);

    let reopened = altai_core::WorkStore::open(&paths.work_db()).expect("reopen canonical store");
    let final_work = reopened
        .get_work(&work_id)
        .expect("get final Work")
        .expect("same Work ID");
    assert_eq!(final_work.state, altai_core::WorkState::Done);
    let legacy_task_runs = altai_core::EventJournal::open(paths.agent_event_journal_db())
        .expect("reopen task-run journal")
        .list_task_runs(10)
        .expect("list legacy task runs");
    assert!(legacy_task_runs
        .iter()
        .any(|task| task.chat_id == "legacy-task-chat"));
}
