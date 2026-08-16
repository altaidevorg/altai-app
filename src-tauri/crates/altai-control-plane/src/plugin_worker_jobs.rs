//! CP-08 plugin job dispatch (package 072, PR 4). A plugin with the
//! `Jobs` capability runs background jobs in its worker; this module is
//! the host's half of that contract: the job frame types that travel on
//! the [`worker transport`](crate::plugin_worker_transport), and the
//! dispatch ledger that makes delivery idempotent.
//!
//! The host's guarantee is **at-most-once dispatch**: a `job_id` is sent
//! to the worker once, ever — across crashes, restarts, and re-dispatch
//! calls. The frame carries the same id to the worker, so a well-behaved
//! plugin can dedup on its side too; honoring that is the plugin's
//! contract obligation, not the host's to enforce. A job whose result
//! never arrives stays `Dispatched` in the ledger: visible, not silently
//! retried.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Host → worker: run this job. `job_id` is the idempotency key.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobRequest {
    pub job_id: String,
    pub payload: Value,
}

/// Worker → host: the job finished (or failed).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobResult {
    pub job_id: String,
    pub ok: bool,
    pub output: Value,
}

/// What the host knows about one dispatched job.
#[derive(Debug, Clone, PartialEq)]
pub enum JobState {
    /// Sent; no result has arrived. Includes jobs whose worker died
    /// mid-flight — the honest state is "never completed", not "failed".
    Dispatched,
    /// A result arrived and was recorded.
    Completed { ok: bool, output: Value },
}

/// The result of a [`SupervisedWorker::dispatch_job`](crate::SupervisedWorker::dispatch_job)
/// call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobDispatch {
    /// Recorded and sent to a live worker.
    Sent,
    /// The job id is already known: nothing was sent, nothing will be.
    AlreadyKnown,
    /// No live worker to carry the job: nothing was recorded, so a later
    /// dispatch (once the worker is back) is safe.
    WorkerDown,
}

/// Host-side record of every job id this worker has been told about.
/// In-memory for now: it survives worker restarts (the host does not
/// restart with them) and is the seam a durable store attaches to when
/// jobs join the work graph.
#[derive(Debug, Default)]
pub struct JobDispatchLedger {
    entries: std::collections::HashMap<String, JobState>,
}

impl JobDispatchLedger {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record dispatch intent for `job_id`. Returns `false` when the job
    /// is already known — the caller must not send again. Recording
    /// *before* sending is the at-most-once guarantee: a crash between
    /// the two cannot lead to a second send on the next attempt.
    pub fn record_dispatch(&mut self, job_id: &str) -> bool {
        self.entries
            .insert(job_id.to_string(), JobState::Dispatched)
            .is_none()
    }

    /// Record a result. Results for unknown job ids are recorded too:
    /// evidence from the worker is never discarded.
    pub fn record_result(&mut self, result: JobResult) {
        let JobResult { job_id, ok, output } = result;
        self.entries.insert(
            job_id,
            JobState::Completed { ok, output },
        );
    }

    pub fn state(&self, job_id: &str) -> Option<&JobState> {
        self.entries.get(job_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn result(job_id: &str, ok: bool) -> JobResult {
        JobResult {
            job_id: job_id.into(),
            ok,
            output: json!({"echo": job_id}),
        }
    }

    #[test]
    fn a_job_id_is_dispatched_at_most_once() {
        let mut ledger = JobDispatchLedger::new();
        assert!(ledger.record_dispatch("job_1"));
        assert!(!ledger.record_dispatch("job_1"));
        assert_eq!(ledger.state("job_1"), Some(&JobState::Dispatched));
    }

    #[test]
    fn a_result_completes_its_job() {
        let mut ledger = JobDispatchLedger::new();
        ledger.record_dispatch("job_1");
        ledger.record_result(result("job_1", true));
        assert_eq!(
            ledger.state("job_1"),
            Some(&JobState::Completed {
                ok: true,
                output: json!({"echo": "job_1"}),
            })
        );
    }

    #[test]
    fn a_result_for_an_unknown_job_is_still_evidence() {
        let mut ledger = JobDispatchLedger::new();
        ledger.record_result(result("from_before", false));
        assert!(matches!(
            ledger.state("from_before"),
            Some(JobState::Completed { ok: false, .. })
        ));
    }
}
