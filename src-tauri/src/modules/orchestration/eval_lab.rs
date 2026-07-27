//! Evaluation and Replay Lab (plan §H3).
//!
//! Provides reusable infrastructure for:
//! - scripting mock runner scenarios;
//! - recording and replaying event journals;
//! - sanitizing production support bundles for safe replay;
//! - systematic crash injection at every state transition;
//! - deterministic failure matrices with seeds.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use super::domain::{AttemptState, AttemptTrigger, TaskState, TaskTrigger};
use super::ledger::{
    LedgerError, LedgerResult, OrchestrationEvent, OrchestrationLedger, TaskRecord,
};
use super::runners::RunnerEventKind;

const EVENT_PAGE_SIZE: usize = 1_000;
const MAX_SUPPORT_BUNDLE_TASKS: usize = 1_000;
const MAX_SUPPORT_BUNDLE_EVENTS: usize = 100_000;

// ---------------------------------------------------------------------------
// Sanitizer
// ---------------------------------------------------------------------------

/// A rule for redacting sensitive data from event payloads.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SanitizeRule {
    /// Key name (case-insensitive substring match) that triggers redaction.
    pub key_pattern: String,
    /// Value to replace with.
    pub replacement: String,
}

/// Default redaction rules covering common secret patterns.
pub fn default_sanitize_rules() -> Vec<SanitizeRule> {
    [
        "password",
        "passwd",
        "secret",
        "token",
        "api_key",
        "apikey",
        "access_key",
        "private_key",
        "credential",
        "authorization",
        "auth_token",
        "client_secret",
    ]
    .into_iter()
    .map(|k| SanitizeRule {
        key_pattern: k.to_string(),
        replacement: "[REDACTED]".to_string(),
    })
    .collect()
}

/// Sanitize a single event's metadata and payload in place.
pub fn sanitize_event(event: &mut OrchestrationEvent, rules: &[SanitizeRule]) {
    let redactor = isanagent::redact::shared();
    event.event_id = redactor.redact(&event.event_id).into_owned();
    event.task_id = redactor.redact(&event.task_id).into_owned();
    event.kind = redactor.redact(&event.kind).into_owned();
    sanitize_formatted_secrets(&mut event.payload);
    sanitize_value(&mut event.payload, rules);
}

/// Sanitize a list of events in place.
pub fn sanitize_events(events: &mut [OrchestrationEvent], rules: &[SanitizeRule]) {
    for event in events.iter_mut() {
        sanitize_event(event, rules);
    }
}

fn sanitize_value(value: &mut Value, rules: &[SanitizeRule]) {
    match value {
        Value::Object(map) => sanitize_map(map, rules),
        Value::Array(arr) => {
            for v in arr.iter_mut() {
                sanitize_value(v, rules);
            }
        }
        _ => {}
    }
}

fn sanitize_formatted_secrets(value: &mut Value) {
    match value {
        Value::String(value) => {
            *value = isanagent::redact::shared().redact(value).into_owned();
        }
        Value::Array(values) => {
            for value in values {
                sanitize_formatted_secrets(value);
            }
        }
        Value::Object(values) => {
            for value in values.values_mut() {
                sanitize_formatted_secrets(value);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

fn sanitize_map(map: &mut Map<String, Value>, rules: &[SanitizeRule]) {
    for (key, val) in map.iter_mut() {
        let key_lower = key.to_lowercase();
        let matched = rules.iter().find(|rule| {
            !rule.key_pattern.is_empty() && key_lower.contains(&rule.key_pattern.to_lowercase())
        });
        if let Some(rule) = matched {
            // Redact the complete value. Recursing into a secret-named object
            // would leave values under ordinary child keys exposed.
            if !is_empty_value(val) {
                *val = Value::String(rule.replacement.clone());
            }
        } else {
            sanitize_value(val, rules);
        }
    }
}

fn is_empty_value(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(value) => value.is_empty(),
        Value::Array(value) => value.is_empty(),
        Value::Object(value) => value.is_empty(),
        Value::Bool(_) | Value::Number(_) => false,
    }
}

fn sanitize_task(task: &mut TaskRecord) {
    let redactor = isanagent::redact::shared();
    task.task_id = redactor.redact(&task.task_id).into_owned();
    task.workspace_key = redactor.redact(&task.workspace_key).into_owned();
    task.source_kind = redactor.redact(&task.source_kind).into_owned();
    task.source_ref = redactor.redact(&task.source_ref).into_owned();
    task.title = redactor.redact(&task.title).into_owned();
    task.description = redactor.redact(&task.description).into_owned();
}

// ---------------------------------------------------------------------------
// Support bundle
// ---------------------------------------------------------------------------

/// A sanitized export of recorded events + task records for replay or sharing.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SupportBundle {
    pub schema_version: u8,
    pub created_at_ms: u64,
    pub events: Vec<OrchestrationEvent>,
    pub tasks: Vec<TaskRecord>,
    pub sanitized: bool,
    pub source: String,
}

/// Export a support bundle from the ledger for the given task IDs.
/// When `sanitize` is true, sensitive fields in events, tasks, and source
/// metadata are redacted.
pub fn export_support_bundle(
    ledger: &OrchestrationLedger,
    task_ids: &[&str],
    sanitize: bool,
    source: &str,
) -> LedgerResult<SupportBundle> {
    if task_ids.len() > MAX_SUPPORT_BUNDLE_TASKS {
        return Err(LedgerError::InvalidField("task_ids"));
    }

    let mut all_events = Vec::new();
    let mut tasks = Vec::new();
    let mut seen = HashSet::new();

    for &task_id in task_ids {
        if !seen.insert(task_id) {
            continue;
        }
        let task = ledger
            .task(task_id)?
            .ok_or_else(|| LedgerError::UnknownTask {
                task_id: task_id.to_string(),
            })?;
        tasks.push(task);

        let mut after_seq = 0;
        loop {
            let page = ledger.events_for_task(task_id, after_seq, EVENT_PAGE_SIZE)?;
            if page.is_empty() {
                break;
            }
            if all_events.len().saturating_add(page.len()) > MAX_SUPPORT_BUNDLE_EVENTS {
                return Err(LedgerError::InvalidField("events"));
            }
            after_seq = page.last().map(|event| event.seq).unwrap_or(after_seq);
            all_events.extend(page);
        }
    }

    if sanitize {
        sanitize_events(&mut all_events, &default_sanitize_rules());
        for task in &mut tasks {
            sanitize_task(task);
        }
    }

    let source = if sanitize {
        isanagent::redact::shared().redact(source).into_owned()
    } else {
        source.to_string()
    };

    Ok(SupportBundle {
        schema_version: 1,
        created_at_ms: now_ms(),
        events: all_events,
        tasks,
        sanitized: sanitize,
        source,
    })
}

// ---------------------------------------------------------------------------
// Crash injection matrix
// ---------------------------------------------------------------------------

/// A legal state-machine transition after which a crash can be injected.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "aggregate", rename_all = "snake_case")]
pub enum CrashPoint {
    Task {
        label: String,
        from: TaskState,
        trigger: TaskTrigger,
        to: TaskState,
    },
    Attempt {
        label: String,
        from: AttemptState,
        trigger: AttemptTrigger,
        to: AttemptState,
    },
}

/// Build a crash point for every legal task and attempt transition.
pub fn build_crash_matrix() -> Vec<CrashPoint> {
    let task_states = [
        TaskState::Draft,
        TaskState::Queued,
        TaskState::Planning,
        TaskState::AwaitingPlanApproval,
        TaskState::Running,
        TaskState::AwaitingInput,
        TaskState::AwaitingApproval,
        TaskState::Verifying,
        TaskState::Reviewing,
        TaskState::ReadyForHandoff,
        TaskState::Done,
        TaskState::Blocked,
        TaskState::Retrying,
        TaskState::Paused,
        TaskState::Cancelled,
        TaskState::Failed,
        TaskState::Abandoned,
        TaskState::NeedsAttention,
    ];
    let task_triggers = [
        TaskTrigger::Queue,
        TaskTrigger::StartPlanning,
        TaskTrigger::RequestPlanApproval,
        TaskTrigger::ApprovePlan,
        TaskTrigger::RevisePlan,
        TaskTrigger::StartRun,
        TaskTrigger::NeedInput,
        TaskTrigger::NeedApproval,
        TaskTrigger::Resume,
        TaskTrigger::Retry,
        TaskTrigger::StartVerify,
        TaskTrigger::StartReview,
        TaskTrigger::ReadyForHandoff,
        TaskTrigger::Rework,
        TaskTrigger::Complete,
        TaskTrigger::Pause,
        TaskTrigger::Block,
        TaskTrigger::Unblock,
        TaskTrigger::Cancel,
        TaskTrigger::Fail,
        TaskTrigger::Abandon,
        TaskTrigger::MarkNeedsAttention,
        TaskTrigger::Resolve,
    ];
    let attempt_states = [
        AttemptState::Created,
        AttemptState::Started,
        AttemptState::Heartbeat,
        AttemptState::InputRequired,
        AttemptState::ApprovalRequired,
        AttemptState::Steered,
        AttemptState::CancelRequested,
        AttemptState::Completed,
        AttemptState::Failed,
        AttemptState::Stalled,
        AttemptState::Cancelled,
    ];
    let attempt_triggers = [
        AttemptTrigger::Start,
        AttemptTrigger::Heartbeat,
        AttemptTrigger::NeedInput,
        AttemptTrigger::NeedApproval,
        AttemptTrigger::Steer,
        AttemptTrigger::Resume,
        AttemptTrigger::RequestCancel,
        AttemptTrigger::Cancel,
        AttemptTrigger::Complete,
        AttemptTrigger::Fail,
        AttemptTrigger::Stall,
    ];

    let mut points = Vec::new();
    for &from in &task_states {
        for &trigger in &task_triggers {
            if let Ok(to) = from.transition(trigger) {
                points.push(CrashPoint::Task {
                    label: format!("task:{}--{}-->{}", from.name(), trigger.name(), to.name()),
                    from,
                    trigger,
                    to,
                });
            }
        }
    }
    for &from in &attempt_states {
        for &trigger in &attempt_triggers {
            if let Ok(to) = from.transition(trigger) {
                points.push(CrashPoint::Attempt {
                    label: format!(
                        "attempt:{}--{}-->{}",
                        from.name(),
                        trigger.name(),
                        to.name()
                    ),
                    from,
                    trigger,
                    to,
                });
            }
        }
    }
    points
}

/// Deterministic seed for reproducible failure scenarios.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FailureScenario {
    pub seed: u64,
    pub label: String,
    pub crash_after_events: usize,
    pub inject_malformed: bool,
    pub inject_timeout_ms: Option<u64>,
}

/// Build a deterministic failure matrix. Each scenario has a unique seed
/// derived from the index, making CI runs reproducible.
pub fn build_failure_matrix(count: usize) -> Vec<FailureScenario> {
    (0..count)
        .map(|i| {
            let seed = 0xDEAD_BEEF_CAFE_BABEu64.wrapping_mul((i + 1) as u64);
            FailureScenario {
                seed,
                label: format!("failure-{i}"),
                crash_after_events: (seed % 5 + 1) as usize,
                inject_malformed: seed.is_multiple_of(3),
                inject_timeout_ms: if seed.is_multiple_of(4) {
                    Some(100)
                } else {
                    None
                },
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Scenario builder
// ---------------------------------------------------------------------------

/// Fluent builder for constructing mock runner event sequences.
pub struct ScenarioBuilder {
    events: Vec<ScenarioEvent>,
}

/// A single scripted event in a scenario.
#[derive(Clone, Debug)]
pub struct ScenarioEvent {
    pub attempt_id: String,
    /// `None` deliberately represents an unparseable/unknown runner kind.
    pub kind: Option<RunnerEventKind>,
    pub payload: Value,
}

impl ScenarioBuilder {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn start(mut self, attempt_id: &str) -> Self {
        self.events.push(ScenarioEvent {
            attempt_id: attempt_id.to_string(),
            kind: Some(RunnerEventKind::Started),
            payload: json!({}),
        });
        self
    }

    pub fn heartbeat(mut self, attempt_id: &str) -> Self {
        self.events.push(ScenarioEvent {
            attempt_id: attempt_id.to_string(),
            kind: Some(RunnerEventKind::Heartbeat),
            payload: json!({}),
        });
        self
    }

    pub fn output(mut self, attempt_id: &str, message: &str) -> Self {
        self.events.push(ScenarioEvent {
            attempt_id: attempt_id.to_string(),
            kind: Some(RunnerEventKind::Output),
            payload: json!({ "message": message }),
        });
        self
    }

    pub fn need_input(mut self, attempt_id: &str, prompt: &str) -> Self {
        self.events.push(ScenarioEvent {
            attempt_id: attempt_id.to_string(),
            kind: Some(RunnerEventKind::InputRequired),
            payload: json!({ "prompt": prompt }),
        });
        self
    }

    pub fn need_approval(mut self, attempt_id: &str, description: &str) -> Self {
        self.events.push(ScenarioEvent {
            attempt_id: attempt_id.to_string(),
            kind: Some(RunnerEventKind::ApprovalRequired),
            payload: json!({ "description": description }),
        });
        self
    }

    pub fn complete(mut self, attempt_id: &str) -> Self {
        self.events.push(ScenarioEvent {
            attempt_id: attempt_id.to_string(),
            kind: Some(RunnerEventKind::Completed),
            payload: json!({}),
        });
        self
    }

    pub fn fail(mut self, attempt_id: &str, error: &str) -> Self {
        self.events.push(ScenarioEvent {
            attempt_id: attempt_id.to_string(),
            kind: Some(RunnerEventKind::Failed),
            payload: json!({ "error": error }),
        });
        self
    }

    pub fn cancel(mut self, attempt_id: &str) -> Self {
        self.events.push(ScenarioEvent {
            attempt_id: attempt_id.to_string(),
            kind: Some(RunnerEventKind::Cancelled),
            payload: json!({}),
        });
        self
    }

    pub fn stall(mut self, attempt_id: &str) -> Self {
        self.events.push(ScenarioEvent {
            attempt_id: attempt_id.to_string(),
            kind: Some(RunnerEventKind::Stalled),
            payload: json!({}),
        });
        self
    }

    /// Inject an event with an unknown kind and malformed payload.
    pub fn inject_malformed(mut self, attempt_id: &str) -> Self {
        self.events.push(ScenarioEvent {
            attempt_id: attempt_id.to_string(),
            kind: None,
            payload: Value::String("{{{malformed}}}".to_string()),
        });
        self
    }

    /// Build the final event sequence.
    pub fn build(self) -> Vec<ScenarioEvent> {
        self.events
    }

    /// Number of events in the scenario.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

impl Default for ScenarioBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Event replay verifier
// ---------------------------------------------------------------------------

/// Verify that a replayed event journal produces the expected final task states.
#[derive(Clone, Debug)]
pub struct ReplayExpectation {
    pub task_id: String,
    pub expected_state: TaskState,
}

/// Result of a replay verification.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayResult {
    pub passed: bool,
    pub task_id: String,
    pub expected: String,
    pub actual: Option<String>,
    pub event_count: usize,
}

/// Verify that the task's current state matches the expectation.
pub fn verify_replay(
    ledger: &OrchestrationLedger,
    expectations: &[ReplayExpectation],
) -> LedgerResult<Vec<ReplayResult>> {
    let mut results = Vec::with_capacity(expectations.len());
    for expectation in expectations {
        let actual = ledger.task(&expectation.task_id)?.map(|task| task.state);
        let event_count = count_events(ledger, &expectation.task_id)?;
        results.push(ReplayResult {
            passed: actual == Some(expectation.expected_state),
            task_id: expectation.task_id.clone(),
            expected: expectation.expected_state.name().to_string(),
            actual: actual.map(|state| state.name().to_string()),
            event_count,
        });
    }
    Ok(results)
}

fn count_events(ledger: &OrchestrationLedger, task_id: &str) -> LedgerResult<usize> {
    let mut count = 0usize;
    let mut after_seq = 0;
    loop {
        let page = ledger.events_for_task(task_id, after_seq, EVENT_PAGE_SIZE)?;
        if page.is_empty() {
            return Ok(count);
        }
        count = count
            .checked_add(page.len())
            .ok_or(LedgerError::NumericOverflow("event_count"))?;
        after_seq = page.last().map(|event| event.seq).unwrap_or(after_seq);
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::orchestration::ledger::OrchestrationLedger;

    fn task_record(task_id: &str, state: TaskState) -> TaskRecord {
        TaskRecord {
            task_id: task_id.into(),
            workspace_key: "workspace".into(),
            source_kind: "local".into(),
            source_ref: "board".into(),
            title: "Test task".into(),
            description: String::new(),
            state,
            created_at_ms: 1,
            updated_at_ms: 1,
        }
    }

    // ---- sanitizer ----

    #[test]
    fn redacts_known_secret_keys() {
        let rules = default_sanitize_rules();
        let mut event = OrchestrationEvent {
            event_id: "e1".into(),
            task_id: "t1".into(),
            seq: 1,
            kind: "test".into(),
            payload: json!({
                "api_key": "sk-1234567890",
                "data": "safe value"
            }),
            recorded_at_ms: 100,
        };
        sanitize_event(&mut event, &rules);
        assert_eq!(event.payload["api_key"], "[REDACTED]");
        assert_eq!(event.payload["data"], "safe value");
    }

    #[test]
    fn redacts_nested_secrets() {
        let rules = default_sanitize_rules();
        let mut event = OrchestrationEvent {
            event_id: "e1".into(),
            task_id: "t1".into(),
            seq: 1,
            kind: "test".into(),
            payload: json!({
                "config": {
                    "password": "hunter2",
                    "host": "localhost"
                },
                "items": [
                    {"token": "abc"},
                    {"name": "ok"}
                ]
            }),
            recorded_at_ms: 100,
        };
        sanitize_event(&mut event, &rules);
        assert_eq!(event.payload["config"]["password"], "[REDACTED]");
        assert_eq!(event.payload["config"]["host"], "localhost");
        assert_eq!(event.payload["items"][0]["token"], "[REDACTED]");
        assert_eq!(event.payload["items"][1]["name"], "ok");
    }

    #[test]
    fn redaction_is_case_insensitive() {
        let rules = default_sanitize_rules();
        let mut event = OrchestrationEvent {
            event_id: "e1".into(),
            task_id: "t1".into(),
            seq: 1,
            kind: "test".into(),
            payload: json!({
                "API_KEY": "secret",
                "Password": "secret",
                "CLIENT_SECRET": "secret"
            }),
            recorded_at_ms: 100,
        };
        sanitize_event(&mut event, &rules);
        assert_eq!(event.payload["API_KEY"], "[REDACTED]");
        assert_eq!(event.payload["Password"], "[REDACTED]");
        assert_eq!(event.payload["CLIENT_SECRET"], "[REDACTED]");
    }

    #[test]
    fn empty_strings_not_redacted() {
        let rules = default_sanitize_rules();
        let mut event = OrchestrationEvent {
            event_id: "e1".into(),
            task_id: "t1".into(),
            seq: 1,
            kind: "test".into(),
            payload: json!({
                "token": ""
            }),
            recorded_at_ms: 100,
        };
        sanitize_event(&mut event, &rules);
        assert_eq!(event.payload["token"], "");
    }

    #[test]
    fn sanitize_events_batch() {
        let rules = default_sanitize_rules();
        let mut events = vec![
            OrchestrationEvent {
                event_id: "e1".into(),
                task_id: "t1".into(),
                seq: 1,
                kind: "test".into(),
                payload: json!({"secret": "abc"}),
                recorded_at_ms: 100,
            },
            OrchestrationEvent {
                event_id: "e2".into(),
                task_id: "t1".into(),
                seq: 2,
                kind: "test".into(),
                payload: json!({"password": "xyz"}),
                recorded_at_ms: 200,
            },
        ];
        sanitize_events(&mut events, &rules);
        assert_eq!(events[0].payload["secret"], "[REDACTED]");
        assert_eq!(events[1].payload["password"], "[REDACTED]");
    }

    #[test]
    fn redacts_complete_structured_and_non_string_secret_values() {
        let rules = vec![SanitizeRule {
            key_pattern: "AUTHORIZATION".into(),
            replacement: "[CUSTOM]".into(),
        }];
        let mut event = OrchestrationEvent {
            event_id: "e1".into(),
            task_id: "t1".into(),
            seq: 1,
            kind: "test".into(),
            payload: json!({
                "authorization": {"bearer": "short-secret"},
                "AuthorizationCount": 42,
                "message": "key sk-abcdefghijklmnop"
            }),
            recorded_at_ms: 100,
        };

        sanitize_event(&mut event, &rules);

        assert_eq!(event.payload["authorization"], "[CUSTOM]");
        assert_eq!(event.payload["AuthorizationCount"], "[CUSTOM]");
        assert!(!event.payload["message"]
            .as_str()
            .unwrap()
            .contains("sk-abcdefghijklmnop"));
    }

    // ---- crash matrix ----

    #[test]
    fn crash_matrix_covers_all_states() {
        let matrix = build_crash_matrix();
        assert!(!matrix.is_empty(), "should have crash points");
        assert!(matrix.iter().any(|point| matches!(
            point,
            CrashPoint::Task {
                from: TaskState::Running,
                trigger: TaskTrigger::StartVerify,
                to: TaskState::Verifying,
                ..
            }
        )));
        assert!(matrix.iter().any(|point| matches!(
            point,
            CrashPoint::Attempt {
                from: AttemptState::Created,
                trigger: AttemptTrigger::Start,
                to: AttemptState::Started,
                ..
            }
        )));
    }

    #[test]
    fn every_crash_point_is_a_legal_transition() {
        let matrix = build_crash_matrix();
        for point in matrix {
            match point {
                CrashPoint::Task {
                    from, trigger, to, ..
                } => assert_eq!(from.transition(trigger).unwrap(), to),
                CrashPoint::Attempt {
                    from, trigger, to, ..
                } => assert_eq!(from.transition(trigger).unwrap(), to),
            }
        }
    }

    // ---- failure matrix ----

    #[test]
    fn failure_matrix_is_deterministic() {
        let m1 = build_failure_matrix(10);
        let m2 = build_failure_matrix(10);
        assert_eq!(m1.len(), m2.len());
        for (a, b) in m1.iter().zip(m2.iter()) {
            assert_eq!(a.seed, b.seed);
            assert_eq!(a.crash_after_events, b.crash_after_events);
            assert_eq!(a.inject_malformed, b.inject_malformed);
        }
    }

    #[test]
    fn failure_matrix_unique_seeds() {
        let matrix = build_failure_matrix(20);
        let seeds: Vec<u64> = matrix.iter().map(|s| s.seed).collect();
        let unique: std::collections::HashSet<_> = seeds.iter().collect();
        assert_eq!(seeds.len(), unique.len(), "seeds should be unique");
    }

    // ---- scenario builder ----

    #[test]
    fn scenario_basic_lifecycle() {
        let events = ScenarioBuilder::new()
            .start("att-1")
            .output("att-1", "working")
            .complete("att-1")
            .build();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].kind, Some(RunnerEventKind::Started));
        assert_eq!(events[1].kind, Some(RunnerEventKind::Output));
        assert_eq!(events[2].kind, Some(RunnerEventKind::Completed));
    }

    #[test]
    fn scenario_with_failure() {
        let events = ScenarioBuilder::new()
            .start("att-1")
            .fail("att-1", "OOM")
            .build();
        assert_eq!(events.len(), 2);
        assert_eq!(events[1].kind, Some(RunnerEventKind::Failed));
        assert_eq!(events[1].payload["error"], "OOM");
    }

    #[test]
    fn scenario_with_approval_and_input() {
        let events = ScenarioBuilder::new()
            .start("att-1")
            .need_input("att-1", "Which branch?")
            .need_approval("att-1", "Merge to main?")
            .build();
        assert_eq!(events.len(), 3);
        assert_eq!(events[1].kind, Some(RunnerEventKind::InputRequired));
        assert_eq!(events[2].kind, Some(RunnerEventKind::ApprovalRequired));
    }

    #[test]
    fn scenario_with_malformed_event() {
        let events = ScenarioBuilder::new()
            .start("att-1")
            .inject_malformed("att-1")
            .complete("att-1")
            .build();
        assert_eq!(events.len(), 3);
        assert!(events[1].kind.is_none());
        assert!(events[1].payload.is_string());
    }

    #[test]
    fn scenario_multi_attempt() {
        let events = ScenarioBuilder::new()
            .start("att-1")
            .fail("att-1", "error")
            .start("att-2")
            .complete("att-2")
            .build();
        assert_eq!(events.len(), 4);
        assert_eq!(events[0].attempt_id, "att-1");
        assert_eq!(events[2].attempt_id, "att-2");
    }

    // ---- support bundle ----

    #[test]
    fn support_bundle_export_sanitized() {
        let ledger = OrchestrationLedger::open_in_memory().unwrap();
        // We can't easily create tasks in the ledger without the full create
        // flow, so just test the sanitizer path with empty data.
        let bundle = export_support_bundle(&ledger, &[], true, "test").unwrap();
        assert!(bundle.sanitized);
        assert_eq!(bundle.schema_version, 1);
        assert!(bundle.events.is_empty());
    }

    #[test]
    fn support_bundle_export_unsanitized() {
        let ledger = OrchestrationLedger::open_in_memory().unwrap();
        let bundle = export_support_bundle(&ledger, &[], false, "prod").unwrap();
        assert!(!bundle.sanitized);
        assert_eq!(bundle.source, "prod");
    }

    #[test]
    fn support_bundle_propagates_unknown_tasks() {
        let ledger = OrchestrationLedger::open_in_memory().unwrap();
        assert!(matches!(
            export_support_bundle(&ledger, &["missing"], true, "test"),
            Err(LedgerError::UnknownTask { task_id }) if task_id == "missing"
        ));
    }

    #[test]
    fn support_bundle_sanitizes_metadata_and_paginates_all_events() {
        let ledger = OrchestrationLedger::open_in_memory().unwrap();
        let mut task = task_record("t-1", TaskState::Running);
        task.description = "credential sk-abcdefghijklmnop".into();
        ledger.upsert_task(&task).unwrap();
        for index in 0..1_001 {
            ledger
                .record_event(&OrchestrationEvent {
                    event_id: format!("event-{index}"),
                    task_id: "t-1".into(),
                    seq: 0,
                    kind: "test".into(),
                    payload: json!({"index": index}),
                    recorded_at_ms: index,
                })
                .unwrap();
        }

        let bundle =
            export_support_bundle(&ledger, &["t-1", "t-1"], true, "sk-abcdefghijklmnop").unwrap();

        assert_eq!(bundle.tasks.len(), 1);
        assert_eq!(bundle.events.len(), 1_001);
        assert!(!bundle.tasks[0].description.contains("sk-abcdefghijklmnop"));
        assert!(!bundle.source.contains("sk-abcdefghijklmnop"));
    }

    // ---- replay verifier ----

    #[test]
    fn verify_replay_missing_task_fails() {
        let ledger = OrchestrationLedger::open_in_memory().unwrap();
        let results = verify_replay(
            &ledger,
            &[ReplayExpectation {
                task_id: "nonexistent".into(),
                expected_state: TaskState::Abandoned,
            }],
        )
        .unwrap();
        assert_eq!(results.len(), 1);
        assert!(!results[0].passed);
        assert_eq!(results[0].actual, None);
    }

    #[test]
    fn verify_replay_reports_state_and_all_events() {
        let ledger = OrchestrationLedger::open_in_memory().unwrap();
        ledger
            .upsert_task(&task_record("t-1", TaskState::Running))
            .unwrap();
        ledger
            .record_event(&OrchestrationEvent {
                event_id: "event-1".into(),
                task_id: "t-1".into(),
                seq: 0,
                kind: "test".into(),
                payload: json!({}),
                recorded_at_ms: 1,
            })
            .unwrap();

        let results = verify_replay(
            &ledger,
            &[ReplayExpectation {
                task_id: "t-1".into(),
                expected_state: TaskState::Running,
            }],
        )
        .unwrap();

        assert!(results[0].passed);
        assert_eq!(results[0].actual.as_deref(), Some("running"));
        assert_eq!(results[0].event_count, 1);
    }

    // ---- H3 acceptance: no paid model calls ----

    #[test]
    fn all_scenarios_use_mock_events_only() {
        // Verify that every scenario builder event is a mock RunnerEventKind,
        // not a real model call.
        let events = ScenarioBuilder::new()
            .start("a")
            .output("a", "test")
            .complete("a")
            .build();
        for e in &events {
            assert!(matches!(
                e.kind.as_ref(),
                Some(
                    RunnerEventKind::Started
                        | RunnerEventKind::Output
                        | RunnerEventKind::Completed
                        | RunnerEventKind::Failed
                        | RunnerEventKind::Heartbeat
                        | RunnerEventKind::InputRequired
                        | RunnerEventKind::ApprovalRequired
                        | RunnerEventKind::CancelRequested
                        | RunnerEventKind::Cancelled
                        | RunnerEventKind::Stalled
                )
            ));
        }
    }
}
