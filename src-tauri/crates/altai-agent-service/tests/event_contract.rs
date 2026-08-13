use std::sync::Mutex;

use altai_agent_service::{
    AgentEventEnvelope, AgentEventSink, AgentEventSinkError, Event, SequencedEventDispatcher,
    WorkspaceServices,
};
use altai_core::journal::{EventJournal, JournalEvent};

struct RecordingSink {
    failures_remaining: Mutex<usize>,
    events: Mutex<Vec<AgentEventEnvelope>>,
}

impl RecordingSink {
    fn new(failures_remaining: usize) -> Self {
        Self {
            failures_remaining: Mutex::new(failures_remaining),
            events: Mutex::new(Vec::new()),
        }
    }
}

impl AgentEventSink for RecordingSink {
    fn try_send(&self, envelope: AgentEventEnvelope) -> Result<(), AgentEventSinkError> {
        let mut failures = self.failures_remaining.lock().unwrap();
        if *failures > 0 {
            *failures -= 1;
            return Err(AgentEventSinkError::Full);
        }
        self.events.lock().unwrap().push(envelope);
        Ok(())
    }
}

#[test]
fn desktop_envelope_json_matches_the_existing_contract() {
    let event = serde_json::to_value(Event::AgentMessage {
        content: "hello".to_string(),
        role: "assistant".to_string(),
    })
    .unwrap();
    let envelope = AgentEventEnvelope::run("chat-1", "run-1", 2, event);
    assert_eq!(
        serde_json::to_value(&envelope).unwrap(),
        serde_json::json!({
            "version": 1,
            "scope": "run",
            "runId": "run-1",
            "seq": 2,
            "chatId": "chat-1",
            "event": { "type": "agent_message", "content": "hello", "role": "assistant" },
        }),
    );
}

#[test]
fn failed_terminal_delivery_does_not_commit_terminal_and_late_terminal_is_rejected() {
    let sink = RecordingSink::new(1);
    let dispatcher = SequencedEventDispatcher::new(sink);
    let terminal = AgentEventEnvelope::run(
        "chat-1",
        "run-1",
        2,
        serde_json::to_value(Event::RunTerminated {
            run_id: "run-1".to_string(),
            outcome: serde_json::json!({"kind": "cancelled"}),
        })
        .unwrap(),
    );

    assert_eq!(
        dispatcher.try_send(terminal.clone()),
        Err(AgentEventSinkError::Full)
    );
    dispatcher.try_send(terminal.clone()).unwrap();
    assert!(matches!(
        dispatcher.try_send(terminal),
        Err(AgentEventSinkError::Rejected(_))
    ));
}

#[test]
fn concurrent_terminal_delivery_accepts_exactly_one_envelope() {
    let dispatcher = std::sync::Arc::new(SequencedEventDispatcher::new(RecordingSink::new(0)));
    let terminal = AgentEventEnvelope::run(
        "chat-1",
        "run-1",
        2,
        serde_json::to_value(Event::RunTerminated {
            run_id: "run-1".to_string(),
            outcome: serde_json::json!({"kind": "completed"}),
        })
        .unwrap(),
    );
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let dispatcher = dispatcher.clone();
            let terminal = terminal.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                dispatcher.try_send(terminal)
            })
        })
        .collect();
    let accepted = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .filter(Result::is_ok)
        .count();
    assert_eq!(accepted, 1);
}

#[test]
fn workspace_open_classifies_an_incomplete_run_once() {
    let directory = tempfile::tempdir().unwrap();
    let generated = directory.path().join(".system_generated");
    std::fs::create_dir_all(&generated).unwrap();
    let journal_path = generated.join("agent_event_journal.db");
    let journal = EventJournal::open(&journal_path).unwrap();
    journal
        .append(&JournalEvent::now(
            1,
            "run-1",
            1,
            "chat-1",
            "run_started",
            serde_json::json!({"type": "run_started", "run_id": "run-1"}),
        ))
        .unwrap();
    drop(journal);

    let services = WorkspaceServices::open(directory.path()).unwrap();
    let summary = services
        .event_journal()
        .run_summary("run-1")
        .unwrap()
        .unwrap();
    assert_eq!(summary.terminal_seq, Some(2));
    drop(services);

    let services = WorkspaceServices::open(directory.path()).unwrap();
    let summary = services
        .event_journal()
        .run_summary("run-1")
        .unwrap()
        .unwrap();
    assert_eq!(summary.last_seq, 2);
    assert_eq!(summary.terminal_seq, Some(2));
}
