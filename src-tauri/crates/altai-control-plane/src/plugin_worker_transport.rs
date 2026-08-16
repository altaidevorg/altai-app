//! CP-08 worker IPC transport (package 072, PR 3). The supervision core
//! treats "answered a health probe" as an observation; this module is
//! the wire that observation travels on. One JSON line each way over the
//! worker's piped stdio: the host writes a
//! [`WorkerFrame::HealthProbe`], the worker answers
//! [`WorkerFrame::HealthOk`]. PR 4 adds the job pair —
//! [`WorkerFrame::JobRequest`] out, [`WorkerFrame::JobResult`] back —
//! and later 072 PRs (webhooks, scoped secrets) extend the vocabulary
//! in place: the transport moves frames, it does not interpret them.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::process::{ChildStdin, ChildStdout};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::plugin_worker::WorkerError;
use crate::plugin_worker_jobs::{JobRequest, JobResult};

/// One frame of the host↔worker wire protocol, as a single JSON line.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkerFrame {
    /// Host → worker: answer with [`WorkerFrame::HealthOk`].
    HealthProbe,
    /// Worker → host: the worker is serving.
    HealthOk,
    /// Host → worker: run this job (072 PR 4).
    JobRequest(JobRequest),
    /// Worker → host: the job finished (072 PR 4).
    JobResult(JobResult),
}

impl WorkerFrame {
    /// The frame as it travels: one JSON line.
    pub fn to_line(&self) -> String {
        serde_json::to_string(self).expect("worker frames always serialize")
    }

    /// Parse one received line. Non-protocol lines yield `None`: a worker
    /// may write anything to its stdout, and an unanswered probe is the
    /// designed consequence, not a host error.
    pub fn from_line(line: &str) -> Option<Self> {
        serde_json::from_str(line).ok()
    }
}

/// The host side of one worker's stdio channel. A reader thread owns the
/// worker's stdout so a receive can *time out* on every platform (a
/// direct read cannot); it exits when the pipe closes, at which point
/// the inbound channel disconnects.
///
/// Two frame kinds arrive unsolicited — health answers and job results —
/// so every receive routes what it got: a probe keeps waiting past job
/// results (stashing them), and a job wait keeps waiting past health
/// answers (ignoring them — every health answer is evidence of a serving
/// worker, consumed by whoever probes next or not at all).
pub struct StdioWorkerTransport {
    stdin: ChildStdin,
    inbound: Receiver<WorkerFrame>,
    stashed_results: VecDeque<JobResult>,
}

impl StdioWorkerTransport {
    /// Take over a freshly spawned worker's piped stdio.
    pub fn new(stdin: ChildStdin, stdout: ChildStdout) -> Self {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if let Some(frame) = WorkerFrame::from_line(&line) {
                    if tx.send(frame).is_err() {
                        // The host side dropped the transport.
                        break;
                    }
                }
            }
        });
        Self {
            stdin,
            inbound: rx,
            stashed_results: VecDeque::new(),
        }
    }

    /// Write one frame to the worker.
    pub fn send(&mut self, frame: &WorkerFrame) -> Result<(), WorkerError> {
        writeln!(self.stdin, "{}", frame.to_line())
            .and_then(|()| self.stdin.flush())
            .map_err(|error| WorkerError::Process {
                reason: format!("cannot write to worker stdin: {error}"),
            })
    }

    /// Ask the worker how it is and wait for its answer. `Ok(true)`: it
    /// answered. `Ok(false)`: no answer within `window` — the worker is
    /// slow, silent, or already gone; the exit observation on the next
    /// reconcile tells those apart. A late reply to an earlier probe
    /// answers the current one: every reply is evidence of a serving
    /// worker.
    pub fn probe(&mut self, window: Duration) -> Result<bool, WorkerError> {
        if self.send(&WorkerFrame::HealthProbe).is_err() {
            // The pipe is broken: the worker cannot answer. Report no
            // answer rather than a host error — its exit explains.
            return Ok(false);
        }
        let deadline = Instant::now() + window;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Ok(false);
            }
            match self.inbound.recv_timeout(remaining) {
                Ok(WorkerFrame::HealthOk) => return Ok(true),
                Ok(WorkerFrame::JobResult(result)) => self.stashed_results.push_back(result),
                Ok(_) => {}
                Err(_) => return Ok(false),
            }
        }
    }

    /// Wait for one job's result. Results for other jobs that arrive
    /// meanwhile are stashed for their own awaiters. `None`: no result
    /// within `window`.
    pub fn await_result(&mut self, job_id: &str, window: Duration) -> Option<JobResult> {
        if let Some(index) = self
            .stashed_results
            .iter()
            .position(|result| result.job_id == job_id)
        {
            return self.stashed_results.remove(index);
        }
        let deadline = Instant::now() + window;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return None;
            }
            match self.inbound.recv_timeout(remaining) {
                Ok(WorkerFrame::JobResult(result)) if result.job_id == job_id => {
                    return Some(result)
                }
                Ok(WorkerFrame::JobResult(result)) => self.stashed_results.push_back(result),
                Ok(_) => {}
                Err(_) => return None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn frames_round_trip_through_lines() {
        let frames = [
            WorkerFrame::HealthProbe,
            WorkerFrame::HealthOk,
            WorkerFrame::JobRequest(JobRequest {
                job_id: "job_1".into(),
                payload: json!({"input": 7}),
            }),
            WorkerFrame::JobResult(JobResult {
                job_id: "job_1".into(),
                ok: true,
                output: json!({"done": true}),
            }),
        ];
        for frame in frames {
            assert_eq!(WorkerFrame::from_line(&frame.to_line()), Some(frame));
        }
    }

    #[test]
    fn the_wire_shape_is_the_documented_contract() {
        assert_eq!(
            WorkerFrame::HealthProbe.to_line(),
            r#"{"type":"health_probe"}"#
        );
        assert_eq!(WorkerFrame::HealthOk.to_line(), r#"{"type":"health_ok"}"#);
        assert_eq!(
            WorkerFrame::JobRequest(JobRequest {
                job_id: "job_1".into(),
                payload: json!({}),
            })
            .to_line(),
            r#"{"type":"job_request","job_id":"job_1","payload":{}}"#
        );
        assert_eq!(
            WorkerFrame::JobResult(JobResult {
                job_id: "job_1".into(),
                ok: true,
                output: json!({"count": 2}),
            })
            .to_line(),
            r#"{"type":"job_result","job_id":"job_1","ok":true,"output":{"count":2}}"#
        );
        // Harness chatter and other junk is not a frame.
        assert_eq!(WorkerFrame::from_line("running 1 test"), None);
        assert_eq!(WorkerFrame::from_line(""), None);
    }
}
