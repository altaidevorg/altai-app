//! CP-08 worker IPC transport (package 072, PR 3). The supervision core
//! treats "answered a health probe" as an observation; this module is
//! the wire that observation travels on. One JSON line each way over the
//! worker's piped stdio: the host writes a
//! [`WorkerFrame::HealthProbe`], the worker answers
//! [`WorkerFrame::HealthOk`]. PR 4 added the job pair and PR 5 the
//! webhook and secret pairs — later Work OS PRs extend the vocabulary
//! in place: the transport moves frames, it does not interpret them.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::process::{ChildStdin, ChildStdout};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::plugin_worker::WorkerError;
use crate::plugin_worker_jobs::{JobRequest, JobResult};
use crate::plugin_worker_secrets::{SecretAck, SecretHandoff};
use crate::plugin_worker_webhooks::{WebhookAck, WebhookDelivery};

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
    /// Host → worker: one inbound webhook delivery (072 PR 5).
    WebhookDelivery(WebhookDelivery),
    /// Worker → host: the delivery was processed (072 PR 5).
    WebhookAck(WebhookAck),
    /// Host → worker: one scoped secret (072 PR 5). The value is a
    /// [`SecretString`](crate::plugin_worker_secrets::SecretString):
    /// printing this frame redacts it.
    SecretHandoff(SecretHandoff),
    /// Worker → host: the secret was received (072 PR 5).
    SecretAck(SecretAck),
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
/// Several frame kinds arrive unsolicited — health answers, job
/// results, webhook acks, secret acks — so every receive routes what it
/// got: whatever a waiter is not waiting for is stashed for the waiter
/// that is. No receive consumes another kind's evidence.
pub struct StdioWorkerTransport {
    stdin: ChildStdin,
    inbound: Receiver<WorkerFrame>,
    stashed_results: VecDeque<JobResult>,
    stashed_webhook_acks: VecDeque<WebhookAck>,
    stashed_secret_acks: VecDeque<SecretAck>,
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
            stashed_webhook_acks: VecDeque::new(),
            stashed_secret_acks: VecDeque::new(),
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

    /// The next frame to arrive within `window`, if any.
    fn next_inbound(&mut self, window: Duration) -> Option<WorkerFrame> {
        self.inbound.recv_timeout(window).ok()
    }

    /// Route one inbound frame this waiter is not waiting for into its
    /// stash. Health answers are consumed: they are evidence for
    /// whoever probes next, not a fact to queue.
    fn stash_unwanted(&mut self, frame: WorkerFrame) {
        match frame {
            WorkerFrame::JobResult(result) => self.stashed_results.push_back(result),
            WorkerFrame::WebhookAck(ack) => self.stashed_webhook_acks.push_back(ack),
            WorkerFrame::SecretAck(ack) => self.stashed_secret_acks.push_back(ack),
            _ => {}
        }
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
            match self.next_inbound(remaining) {
                Some(WorkerFrame::HealthOk) => return Ok(true),
                Some(frame) => self.stash_unwanted(frame),
                None => return Ok(false),
            }
        }
    }

    /// Wait for one job's result. `None`: no result within `window`.
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
            match self.next_inbound(remaining) {
                Some(WorkerFrame::JobResult(result)) if result.job_id == job_id => {
                    return Some(result)
                }
                Some(frame) => self.stash_unwanted(frame),
                None => return None,
            }
        }
    }

    /// Wait for one webhook delivery's ack. `None`: none within `window`.
    pub fn await_webhook_ack(&mut self, delivery_id: &str, window: Duration) -> Option<WebhookAck> {
        if let Some(index) = self
            .stashed_webhook_acks
            .iter()
            .position(|ack| ack.delivery_id == delivery_id)
        {
            return self.stashed_webhook_acks.remove(index);
        }
        let deadline = Instant::now() + window;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return None;
            }
            match self.next_inbound(remaining) {
                Some(WorkerFrame::WebhookAck(ack)) if ack.delivery_id == delivery_id => {
                    return Some(ack)
                }
                Some(frame) => self.stash_unwanted(frame),
                None => return None,
            }
        }
    }

    /// Wait for one secret's ack. `None`: none within `window`.
    pub fn await_secret_ack(&mut self, name: &str, window: Duration) -> Option<SecretAck> {
        if let Some(index) = self
            .stashed_secret_acks
            .iter()
            .position(|ack| ack.name == name)
        {
            return self.stashed_secret_acks.remove(index);
        }
        let deadline = Instant::now() + window;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return None;
            }
            match self.next_inbound(remaining) {
                Some(WorkerFrame::SecretAck(ack)) if ack.name == name => return Some(ack),
                Some(frame) => self.stash_unwanted(frame),
                None => return None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin_worker_secrets::SecretString;
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
            WorkerFrame::WebhookDelivery(WebhookDelivery {
                delivery_id: "wh_1".into(),
                event: "issue.opened".into(),
                payload: json!({"n": 1}),
            }),
            WorkerFrame::WebhookAck(WebhookAck {
                delivery_id: "wh_1".into(),
                ok: true,
            }),
            WorkerFrame::SecretHandoff(SecretHandoff {
                name: "api_token".into(),
                value: SecretString::new("super-secret-key"),
            }),
            WorkerFrame::SecretAck(SecretAck {
                name: "api_token".into(),
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
        assert_eq!(
            WorkerFrame::WebhookDelivery(WebhookDelivery {
                delivery_id: "wh_1".into(),
                event: "issue.opened".into(),
                payload: json!({"n": 1}),
            })
            .to_line(),
            r#"{"type":"webhook_delivery","delivery_id":"wh_1","event":"issue.opened","payload":{"n":1}}"#
        );
        assert_eq!(
            WorkerFrame::WebhookAck(WebhookAck {
                delivery_id: "wh_1".into(),
                ok: true,
            })
            .to_line(),
            r#"{"type":"webhook_ack","delivery_id":"wh_1","ok":true}"#
        );
        // The secret travels as a plain string on the wire (the worker
        // must be able to read it), but never prints.
        assert_eq!(
            WorkerFrame::SecretHandoff(SecretHandoff {
                name: "api_token".into(),
                value: SecretString::new("super-secret-key"),
            })
            .to_line(),
            r#"{"type":"secret_handoff","name":"api_token","value":"super-secret-key"}"#
        );
        assert!(!format!(
            "{:?}",
            WorkerFrame::SecretHandoff(SecretHandoff {
                name: "api_token".into(),
                value: SecretString::new("super-secret-key"),
            })
        )
        .contains("super-secret-key"));
        assert_eq!(
            WorkerFrame::SecretAck(SecretAck {
                name: "api_token".into(),
            })
            .to_line(),
            r#"{"type":"secret_ack","name":"api_token"}"#
        );
        // Harness chatter and other junk is not a frame.
        assert_eq!(WorkerFrame::from_line("running 1 test"), None);
        assert_eq!(WorkerFrame::from_line(""), None);
    }
}
