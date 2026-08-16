//! CP-08 worker IPC transport (package 072, PR 3). The supervision core
//! treats "answered a health probe" as an observation; this module is
//! the wire that observation travels on. One JSON line each way over the
//! worker's piped stdio: the host writes a
//! [`WorkerFrame::HealthProbe`], the worker answers
//! [`WorkerFrame::HealthOk`]. Later 072 PRs (jobs, webhooks) extend the
//! frame vocabulary in place — the transport moves frames, it does not
//! interpret them.

use std::io::{BufRead, BufReader, Write};
use std::process::{ChildStdin, ChildStdout};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::plugin_worker::WorkerError;

/// One frame of the host↔worker wire protocol, as a single JSON line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkerFrame {
    /// Host → worker: answer with [`WorkerFrame::HealthOk`].
    HealthProbe,
    /// Worker → host: the worker is serving.
    HealthOk,
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
/// worker's stdout so a probe can *time out* on every platform (a direct
/// read cannot); it exits when the pipe closes, at which point the
/// replies channel disconnects.
pub struct StdioWorkerTransport {
    stdin: ChildStdin,
    replies: Receiver<WorkerFrame>,
}

impl StdioWorkerTransport {
    /// Take over a freshly spawned worker's piped stdio.
    pub fn new(stdin: ChildStdin, stdout: ChildStdout) -> Self {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if let Some(frame @ WorkerFrame::HealthOk) = WorkerFrame::from_line(&line) {
                    if tx.send(frame).is_err() {
                        // The host side dropped the transport.
                        break;
                    }
                }
            }
        });
        Self { stdin, replies: rx }
    }

    /// Ask the worker how it is and wait for its answer. `Ok(true)`: it
    /// answered. `Ok(false)`: no answer within `window` — the worker is
    /// slow, silent, or already gone; the exit observation on the next
    /// reconcile tells those apart. A late reply to an earlier probe
    /// answers the current one: every reply is evidence of a serving
    /// worker.
    pub fn probe(&mut self, window: Duration) -> Result<bool, WorkerError> {
        if writeln!(self.stdin, "{}", WorkerFrame::HealthProbe.to_line())
            .and_then(|()| self.stdin.flush())
            .is_err()
        {
            // The pipe is broken: the worker cannot answer. Report no
            // answer rather than a host error — its exit explains.
            return Ok(false);
        }
        Ok(matches!(
            self.replies.recv_timeout(window),
            Ok(WorkerFrame::HealthOk)
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_round_trip_through_lines() {
        for frame in [WorkerFrame::HealthProbe, WorkerFrame::HealthOk] {
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
        // Harness chatter and other junk is not a frame.
        assert_eq!(WorkerFrame::from_line("running 1 test"), None);
        assert_eq!(WorkerFrame::from_line(""), None);
    }
}
