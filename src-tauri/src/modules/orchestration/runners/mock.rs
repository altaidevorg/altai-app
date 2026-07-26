//! Deterministic mock runner for coordinator tests and local development.
//!
//! The mock is fully scripted: tests enqueue the event sequence an attempt
//! should emit and the coordinator drains them through [`RunnerAdapter`]. No
//! network, model provider, or real process is involved.

use super::{
    AttemptIdentity, AttemptSpec, RunnerAdapter, RunnerCapabilities, RunnerError, RunnerEvent,
    RunnerEventKind, RunnerResult,
};
use serde_json::Value;
use std::collections::{HashMap, VecDeque};

/// A scripted, in-process runner. Each attempt has a queue of pending event
/// kinds; [`RunnerAdapter::poll_event`] pops the next one.
#[derive(Default)]
pub struct MockRunner {
    queues: HashMap<String, VecDeque<RunnerEventKind>>,
    seq: HashMap<String, u64>,
    started: Vec<String>,
    steered: Vec<(String, String)>,
    cancelled: Vec<String>,
}

impl MockRunner {
    pub fn new() -> Self {
        Self::default()
    }

    /// Pre-load the event sequence for an attempt. Usually called before
    /// `start_attempt` so the coordinator can drain the full run.
    pub fn enqueue<I>(&mut self, attempt_id: &str, kinds: I)
    where
        I: IntoIterator<Item = RunnerEventKind>,
    {
        self.queues
            .entry(attempt_id.to_string())
            .or_default()
            .extend(kinds);
    }

    pub fn was_started(&self, attempt_id: &str) -> bool {
        self.started.iter().any(|id| id == attempt_id)
    }

    pub fn was_cancelled(&self, attempt_id: &str) -> bool {
        self.cancelled.iter().any(|id| id == attempt_id)
    }

    pub fn steers(&self) -> &[(String, String)] {
        &self.steered
    }

    fn next_seq(&mut self, attempt_id: &str) -> u64 {
        let entry = self.seq.entry(attempt_id.to_string()).or_insert(0);
        *entry += 1;
        *entry
    }
}

impl RunnerAdapter for MockRunner {
    fn capabilities(&self) -> RunnerCapabilities {
        RunnerCapabilities {
            can_steer: true,
            can_cancel: true,
            can_resume: true,
        }
    }

    fn start_attempt(&mut self, spec: &AttemptSpec) -> RunnerResult<AttemptIdentity> {
        self.started.push(spec.attempt_id.clone());
        // Ensure the attempt has a queue even if none was pre-loaded.
        self.queues.entry(spec.attempt_id.clone()).or_default();
        Ok(AttemptIdentity {
            attempt_id: spec.attempt_id.clone(),
            handle: spec.attempt_id.clone(),
        })
    }

    fn poll_event(&mut self, identity: &AttemptIdentity) -> RunnerResult<Option<RunnerEvent>> {
        let Some(queue) = self.queues.get_mut(&identity.attempt_id) else {
            return Err(RunnerError::UnknownAttempt {
                attempt_id: identity.attempt_id.clone(),
            });
        };
        Ok(queue.pop_front().map(|kind| {
            let seq = self.next_seq(&identity.attempt_id);
            RunnerEvent {
                attempt_id: identity.attempt_id.clone(),
                kind,
                seq,
                payload: Value::Null,
            }
        }))
    }

    fn steer(&mut self, identity: &AttemptIdentity, message: &str) -> RunnerResult<()> {
        if !self.queues.contains_key(&identity.attempt_id) {
            return Err(RunnerError::UnknownAttempt {
                attempt_id: identity.attempt_id.clone(),
            });
        }
        self.steered
            .push((identity.attempt_id.clone(), message.to_string()));
        Ok(())
    }

    fn cancel(&mut self, identity: &AttemptIdentity) -> RunnerResult<()> {
        if !self.queues.contains_key(&identity.attempt_id) {
            return Err(RunnerError::UnknownAttempt {
                attempt_id: identity.attempt_id.clone(),
            });
        }
        self.cancelled.push(identity.attempt_id.clone());
        Ok(())
    }

    fn shutdown(&mut self) {
        self.queues.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drains_scripted_events_in_order() {
        let mut runner = MockRunner::new();
        runner.enqueue(
            "att-1",
            [RunnerEventKind::Started, RunnerEventKind::Completed],
        );
        let id = runner
            .start_attempt(&AttemptSpec {
                task_id: "t1".into(),
                attempt_id: "att-1".into(),
                input: "do it".into(),
            })
            .expect("start");
        let first = runner.poll_event(&id).expect("poll").expect("event");
        assert_eq!(first.kind, RunnerEventKind::Started);
        assert_eq!(first.seq, 1);
        let second = runner.poll_event(&id).expect("poll").expect("event");
        assert_eq!(second.kind, RunnerEventKind::Completed);
        assert_eq!(second.seq, 2);
        assert!(runner.poll_event(&id).expect("poll").is_none());
    }

    #[test]
    fn unknown_attempt_errors() {
        let mut runner = MockRunner::new();
        let id = AttemptIdentity {
            attempt_id: "ghost".into(),
            handle: "ghost".into(),
        };
        assert!(matches!(
            runner.poll_event(&id),
            Err(RunnerError::UnknownAttempt { .. })
        ));
    }

    #[test]
    fn cancel_and_steer_are_recorded() {
        let mut runner = MockRunner::new();
        runner.enqueue("att-1", [RunnerEventKind::Started]);
        let id = runner
            .start_attempt(&AttemptSpec {
                task_id: "t1".into(),
                attempt_id: "att-1".into(),
                input: "".into(),
            })
            .expect("start");
        runner.steer(&id, "focus on tests").expect("steer");
        runner.cancel(&id).expect("cancel");
        assert_eq!(
            runner.steers(),
            &[("att-1".into(), "focus on tests".into())]
        );
        assert!(runner.was_cancelled("att-1"));
    }
}
