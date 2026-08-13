//! The narrow CP-08 boundary by which the control plane asks IsanAgent to
//! execute an already-authorized Attempt. This is deliberately not a project
//! management API and does not expose scheduler policy to an execution host.

use altai_control_protocol::{AttemptId, RunBinding, RunId, SessionId};
use async_trait::async_trait;

/// Trusted, bounded input for one authorized executor start. Scope, profile,
/// permissions and context are resolved before this boundary; model output
/// cannot substitute any of their identifiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttemptExecutionRequest {
    pub attempt_id: AttemptId,
    pub session_id: SessionId,
    pub prompt: String,
    pub context_pack: String,
    pub permission_policy: String,
}

impl AttemptExecutionRequest {
    pub fn validate(&self) -> Result<(), AttemptExecutorError> {
        if self.prompt.trim().is_empty() {
            return Err(AttemptExecutorError::InvalidRequest("prompt is empty"));
        }
        if self.context_pack.len() > 128 * 1024 {
            return Err(AttemptExecutorError::InvalidRequest("context pack exceeds 128 KiB"));
        }
        if self.permission_policy.trim().is_empty() {
            return Err(AttemptExecutorError::InvalidRequest("permission policy is empty"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttemptExecutionStatus {
    Dispatched,
    Running,
    WaitingForInput,
    Terminal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttemptExecutorError {
    InvalidRequest(&'static str),
    BindingMismatch { attempt_id: String, run_id: String },
    Unavailable(String),
}
impl std::fmt::Display for AttemptExecutorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "attempt executor error: {self:?}")
    }
}
impl std::error::Error for AttemptExecutorError {}

/// IsanAgent execution port. Every control operation names the immutable
/// binding, preventing a stale command from being redirected to another run.
#[async_trait]
pub trait AttemptExecutor: Send + Sync {
    async fn start(&self, request: AttemptExecutionRequest) -> Result<RunBinding, AttemptExecutorError>;
    async fn inspect(&self, binding: &RunBinding) -> Result<AttemptExecutionStatus, AttemptExecutorError>;
    async fn steer(&self, binding: &RunBinding, content: String) -> Result<(), AttemptExecutorError>;
    async fn cancel(&self, binding: &RunBinding) -> Result<(), AttemptExecutorError>;
    async fn replay(&self, binding: &RunBinding, after_seq: u64, limit: usize) -> Result<Vec<serde_json::Value>, AttemptExecutorError>;
}

/// Reject an operation whose supplied run does not match the immutable
/// Attempt binding before it reaches the execution runtime.
pub fn require_run(binding: &RunBinding, attempt_id: &AttemptId, run_id: &RunId) -> Result<(), AttemptExecutorError> {
    if &binding.attempt_id == attempt_id && &binding.run_id == run_id { Ok(()) } else {
        Err(AttemptExecutorError::BindingMismatch { attempt_id: attempt_id.value.clone(), run_id: run_id.value.clone() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use altai_control_protocol::{AgentInstanceId, WorkItemId};
    fn binding() -> RunBinding { RunBinding { attempt_id: AttemptId::new("one"), work_item_id: WorkItemId::new("one"), owner_agent_instance_id: AgentInstanceId::new("one"), run_id: RunId::new("one"), bound_at_unix_seconds: 1 } }
    #[test]
    fn executor_requests_are_bounded_and_operations_are_binding_scoped() {
        assert!(AttemptExecutionRequest { attempt_id: AttemptId::new("one"), session_id: SessionId::new("one"), prompt: "go".into(), context_pack: "context".into(), permission_policy: "default".into() }.validate().is_ok());
        assert!(require_run(&binding(), &AttemptId::new("one"), &RunId::new("other")).is_err());
    }
}
