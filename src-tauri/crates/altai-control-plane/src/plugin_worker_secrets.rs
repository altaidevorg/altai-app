//! CP-08 scoped secrets for plugin workers (package 072, PR 5). A
//! plugin with the `ScopedSecrets` capability can be handed secret
//! values; the host is the broker and the worker's private stdio pipe
//! is the only channel they travel on — a secret scoped to one plugin
//! never reaches another worker or a log.
//!
//! Unlike jobs and webhook deliveries, secrets are **per-process
//! state, not at-most-once ledger entries**: a relaunched worker is a
//! fresh process with fresh memory, so the host hands each secret to
//! every process once and re-hands them after a restart. The host
//! remembers the values it has brokered for exactly that purpose.

use serde::{Deserialize, Serialize};

/// A secret value that refuses to print itself. `Debug` and `Display`
/// are redacting on purpose: worker frames derive `Debug`, and a panic
/// or a log line must never become a secret leak. Reading the value is
/// an explicit act — [`expose`](Self::expose).
#[derive(Clone, PartialEq, Eq)]
pub struct SecretString(String);

impl SecretString {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The value, for the one place that must see it: the wire write.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for SecretString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretString([redacted])")
    }
}

impl std::fmt::Display for SecretString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[redacted]")
    }
}

impl Serialize for SecretString {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.expose())
    }
}

impl<'de> Deserialize<'de> for SecretString {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self(String::deserialize(deserializer)?))
    }
}

/// Host → worker: one scoped secret for this worker alone.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SecretHandoff {
    pub name: String,
    pub value: SecretString,
}

/// Worker → host: the secret was received.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretAck {
    pub name: String,
}

/// The result of [`SupervisedWorker::hand_secret`](crate::SupervisedWorker::hand_secret).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretHandoffOutcome {
    /// Written to the worker's pipe. `confirmed`: the worker acked within
    /// the window; an unconfirmed hand-off is ambiguous (slow, or gone —
    /// the next reconcile re-sends if the worker died) and is reported,
    /// not hidden.
    Delivered { confirmed: bool },
    /// This process already has this secret: not sent again.
    AlreadyProvided,
    /// No live worker: nothing sent, safe to retry later.
    WorkerDown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_and_display_never_leak_the_value() {
        let secret = SecretString::new("super-secret-key");
        assert_eq!(format!("{secret:?}"), "SecretString([redacted])");
        assert_eq!(format!("{secret}"), "[redacted]");
        // And a frame carrying it stays safe to print.
        let handoff = SecretHandoff {
            name: "api_token".into(),
            value: secret,
        };
        let printed = format!("{handoff:?}");
        assert!(!printed.contains("super-secret-key"));
        assert!(printed.contains("[redacted]"));
    }

    #[test]
    fn secrets_round_trip_through_the_wire() {
        let secret = SecretString::new("super-secret-key");
        assert_eq!(secret.expose(), "super-secret-key");
        let round: SecretString =
            serde_json::from_str(&serde_json::to_string(&secret).unwrap()).unwrap();
        assert_eq!(round, secret);
    }
}
