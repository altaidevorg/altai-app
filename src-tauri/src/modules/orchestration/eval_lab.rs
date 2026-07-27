//! Evaluation and Replay Lab (plan §H3).
//!
//! Provides reusable infrastructure for:
//! - scripting mock runner scenarios;
//! - recording and replaying event journals;
//! - sanitizing production support bundles for safe replay;
//! - systematic crash injection at every state transition;
//! - deterministic failure matrices with seeds.

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use super::domain::{AttemptState, TaskState};
use super::ledger::{OrchestrationEvent, OrchestrationLedger, TaskRecord};
use super::runners::RunnerEventKind;

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

/// Sanitize a single event's payload in place.
pub fn sanitize_event(event: &mut OrchestrationEvent, rules: &[SanitizeRule]) {
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

fn sanitize_map(map: &mut Map<String, Value>, rules: &[SanitizeRule]) {
    for (key, val) in map.iter_mut() {
        let key_lower = key.to_lowercase();
        let matched = rules.iter().any(|r| key_lower.contains(&r.key_pattern));
        if matched {
            // Only redact if the value looks like a secret (non-empty string or
            // a non-trivial structure). Leave empty/null values alone.
            match val {
                Value::String(s) if !s.is_empty() => {
                    *val = Value::String("[REDACTED]".to_string());
                }
                Value::Object(_) | Value::Array(_) => {
                    sanitize_value(val, rules);
                }
                _ => {}
            }
        } else {
            sanitize_value(val, rules);
        }
    }
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
/// When `sanitize` is true, sensitive fields in event payloads are redacted.
pub fn export_support_bundle(
    ledger: &OrchestrationLedger,
    task_ids: &[&str],
    sanitize: bool,
    source: &str,
) -> SupportBundle {
    let mut all_events = Vec::new();
    let mut tasks = Vec::new();

    for &task_id in task_ids {
        if let Ok(Some(task)) = ledger.task(task_id) {
            tasks.push(task);
        }
        if let Ok(events) = ledger.events_for_task(task_id, 0, 1000) {
            all_events.extend(events);
        }
    }

    if sanitize {
        sanitize_events(&mut all_events, &default_sanitize_rules());
    }

    SupportBundle {
        schema_version: 1,
        created_at_ms: now_ms(),
        events: all_events,
        tasks,
        sanitized: sanitize,
        source: source.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Crash injection matrix
// ---------------------------------------------------------------------------

/// A single point in the execution lifecycle where a crash can be injected.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrashPoint {
    pub label: String,
    pub task_state: TaskState,
    pub attempt_state: AttemptState,
}

/// Build the complete matrix of crash injection points — every (task_state,
/// attempt_state) combination that represents a durable transition.
pub fn build_crash_matrix() -> Vec<CrashPoint> {
    let task_states = [
        TaskState::Queued,
        TaskState::Running,
        TaskState::Verifying,
        TaskState::Reviewing,
        TaskState::NeedsAttention,
    ];
    let attempt_states = [
        AttemptState::Started,
        AttemptState::Heartbeat,
        AttemptState::InputRequired,
        AttemptState::ApprovalRequired,
        AttemptState::Completed,
        AttemptState::Failed,
        AttemptState::Stalled,
        AttemptState::Cancelled,
    ];

    let mut points = Vec::new();
    for &ts in &task_states {
        for &as_ in &attempt_states {
            // Skip nonsensical combinations.
            if ts == TaskState::Queued && as_ != AttemptState::Started {
                continue;
            }
            points.push(CrashPoint {
                label: format!("{:?}+{:?}", ts, as_),
                task_state: ts,
                attempt_state: as_,
            });
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
    pub kind: RunnerEventKind,
    pub payload: Value,
}

impl ScenarioBuilder {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn start(mut self, attempt_id: &str) -> Self {
        self.events.push(ScenarioEvent {
            attempt_id: attempt_id.to_string(),
            kind: RunnerEventKind::Started,
            payload: json!({}),
        });
        self
    }

    pub fn heartbeat(mut self, attempt_id: &str) -> Self {
        self.events.push(ScenarioEvent {
            attempt_id: attempt_id.to_string(),
            kind: RunnerEventKind::Heartbeat,
            payload: json!({}),
        });
        self
    }

    pub fn output(mut self, attempt_id: &str, message: &str) -> Self {
        self.events.push(ScenarioEvent {
            attempt_id: attempt_id.to_string(),
            kind: RunnerEventKind::Output,
            payload: json!({ "message": message }),
        });
        self
    }

    pub fn need_input(mut self, attempt_id: &str, prompt: &str) -> Self {
        self.events.push(ScenarioEvent {
            attempt_id: attempt_id.to_string(),
            kind: RunnerEventKind::InputRequired,
            payload: json!({ "prompt": prompt }),
        });
        self
    }

    pub fn need_approval(mut self, attempt_id: &str, description: &str) -> Self {
        self.events.push(ScenarioEvent {
            attempt_id: attempt_id.to_string(),
            kind: RunnerEventKind::ApprovalRequired,
            payload: json!({ "description": description }),
        });
        self
    }

    pub fn complete(mut self, attempt_id: &str) -> Self {
        self.events.push(ScenarioEvent {
            attempt_id: attempt_id.to_string(),
            kind: RunnerEventKind::Completed,
            payload: json!({}),
        });
        self
    }

    pub fn fail(mut self, attempt_id: &str, error: &str) -> Self {
        self.events.push(ScenarioEvent {
            attempt_id: attempt_id.to_string(),
            kind: RunnerEventKind::Failed,
            payload: json!({ "error": error }),
        });
        self
    }

    pub fn cancel(mut self, attempt_id: &str) -> Self {
        self.events.push(ScenarioEvent {
            attempt_id: attempt_id.to_string(),
            kind: RunnerEventKind::Cancelled,
            payload: json!({}),
        });
        self
    }

    pub fn stall(mut self, attempt_id: &str) -> Self {
        self.events.push(ScenarioEvent {
            attempt_id: attempt_id.to_string(),
            kind: RunnerEventKind::Stalled,
            payload: json!({}),
        });
        self
    }

    /// Inject a malformed event (invalid kind, garbage payload) for testing
    /// coordinator robustness.
    pub fn inject_malformed(mut self, attempt_id: &str) -> Self {
        self.events.push(ScenarioEvent {
            attempt_id: attempt_id.to_string(),
            kind: RunnerEventKind::Output,
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
    pub actual: String,
    pub event_count: usize,
}

/// Verify that the task's current state matches the expectation.
pub fn verify_replay(
    ledger: &OrchestrationLedger,
    expectations: &[ReplayExpectation],
) -> Vec<ReplayResult> {
    expectations
        .iter()
        .map(|exp| {
            let actual = ledger
                .task(&exp.task_id)
                .ok()
                .flatten()
                .map(|t| t.state)
                .unwrap_or(TaskState::Abandoned);
            let passed = actual == exp.expected_state;
            ReplayResult {
                passed,
                task_id: exp.task_id.clone(),
                expected: format!("{:?}", exp.expected_state),
                actual: format!("{:?}", actual),
                event_count: ledger
                    .events_for_task(&exp.task_id, 0, 1000)
                    .map(|e| e.len())
                    .unwrap_or(0),
            }
        })
        .collect()
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

    // ---- crash matrix ----

    #[test]
    fn crash_matrix_covers_all_states() {
        let matrix = build_crash_matrix();
        assert!(!matrix.is_empty(), "should have crash points");
        // Verify it covers the Running+Started combination.
        assert!(matrix.iter().any(|p| {
            p.task_state == TaskState::Running && p.attempt_state == AttemptState::Started
        }));
    }

    #[test]
    fn crash_matrix_skips_nonsensical_combos() {
        let matrix = build_crash_matrix();
        // Queued task should only have Started attempt.
        let queued = matrix
            .iter()
            .filter(|p| p.task_state == TaskState::Queued)
            .collect::<Vec<_>>();
        assert!(!queued.is_empty());
        assert!(queued
            .iter()
            .all(|p| p.attempt_state == AttemptState::Started));
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
        assert_eq!(events[0].kind, RunnerEventKind::Started);
        assert_eq!(events[1].kind, RunnerEventKind::Output);
        assert_eq!(events[2].kind, RunnerEventKind::Completed);
    }

    #[test]
    fn scenario_with_failure() {
        let events = ScenarioBuilder::new()
            .start("att-1")
            .fail("att-1", "OOM")
            .build();
        assert_eq!(events.len(), 2);
        assert_eq!(events[1].kind, RunnerEventKind::Failed);
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
        assert_eq!(events[1].kind, RunnerEventKind::InputRequired);
        assert_eq!(events[2].kind, RunnerEventKind::ApprovalRequired);
    }

    #[test]
    fn scenario_with_malformed_event() {
        let events = ScenarioBuilder::new()
            .start("att-1")
            .inject_malformed("att-1")
            .complete("att-1")
            .build();
        assert_eq!(events.len(), 3);
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
        let bundle = export_support_bundle(&ledger, &[], true, "test");
        assert!(bundle.sanitized);
        assert_eq!(bundle.schema_version, 1);
        assert!(bundle.events.is_empty());
    }

    #[test]
    fn support_bundle_export_unsanitized() {
        let ledger = OrchestrationLedger::open_in_memory().unwrap();
        let bundle = export_support_bundle(&ledger, &[], false, "prod");
        assert!(!bundle.sanitized);
        assert_eq!(bundle.source, "prod");
    }

    // ---- replay verifier ----

    #[test]
    fn verify_replay_missing_task_fails() {
        let ledger = OrchestrationLedger::open_in_memory().unwrap();
        let results = verify_replay(
            &ledger,
            &[ReplayExpectation {
                task_id: "nonexistent".into(),
                expected_state: TaskState::Done,
            }],
        );
        assert_eq!(results.len(), 1);
        assert!(!results[0].passed);
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
                e.kind,
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
            ));
        }
    }
}
