use std::collections::HashSet;
use std::fmt;
use std::sync::Mutex;

use crate::AgentEventEnvelope;

/// A non-blocking, bounded host delivery boundary.
///
/// Implementations must return `Full` instead of unboundedly buffering and
/// must not report `Ok(())` until the host accepted the envelope. The service
/// persists durable run events before calling a sink, so an unavailable UI is
/// replayable rather than a reason to duplicate the agent action.
pub trait AgentEventSink: Send + Sync {
    fn try_send(&self, envelope: AgentEventEnvelope) -> Result<(), AgentEventSinkError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentEventSinkError {
    Full,
    Unavailable(String),
    Rejected(String),
}

impl fmt::Display for AgentEventSinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full => f.write_str("agent event sink is full"),
            Self::Unavailable(detail) => write!(f, "agent event sink is unavailable: {detail}"),
            Self::Rejected(detail) => write!(f, "agent event sink rejected event: {detail}"),
        }
    }
}

impl std::error::Error for AgentEventSinkError {}

/// Protects a sink from duplicate and late run-terminal envelopes.
///
/// State advances only after the wrapped sink accepts an event, so a failed
/// host delivery never turns a not-performed terminal action into success.
pub struct SequencedEventDispatcher<S> {
    sink: S,
    terminal_runs: Mutex<HashSet<String>>,
}

impl<S> SequencedEventDispatcher<S>
where
    S: AgentEventSink,
{
    pub fn new(sink: S) -> Self {
        Self {
            sink,
            terminal_runs: Mutex::new(HashSet::new()),
        }
    }

    pub fn try_send(&self, envelope: AgentEventEnvelope) -> Result<(), AgentEventSinkError> {
        let terminal_run = envelope
            .is_terminal()
            .then(|| envelope.run_id.clone())
            .flatten();

        if let Some(run_id) = terminal_run {
            // Keep the terminal lease while the non-blocking sink accepts the
            // envelope. Releasing it before `try_send` would let two threads
            // both observe an unfinished run and emit duplicate terminals.
            let mut terminal_runs = self.terminal_runs.lock().map_err(|_| {
                AgentEventSinkError::Unavailable("terminal state lock poisoned".to_string())
            })?;
            if terminal_runs.contains(&run_id) {
                return Err(AgentEventSinkError::Rejected(format!(
                    "run {run_id} already has a terminal event"
                )));
            }
            self.sink.try_send(envelope)?;
            terminal_runs.insert(run_id);
            return Ok(());
        }

        self.sink.try_send(envelope)
    }
}
