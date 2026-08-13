//! The narrow CP-08 boundary by which the control plane asks IsanAgent to
//! execute an already-authorized Attempt. This is deliberately not a project
//! management API and does not expose scheduler policy to an execution host.

use async_trait::async_trait;

/// Opaque control-plane identities at the execution boundary. The service must
/// not depend on the control-plane domain crate; the host adapter translates
/// canonical IDs into this sealed input form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionBinding {
    pub attempt_id: String,
    pub run_id: String,
    pub session_id: String,
}

impl ExecutionBinding {
    pub fn validate(&self) -> Result<(), AttemptExecutorError> {
        if self.attempt_id.trim().is_empty() || self.run_id.trim().is_empty() || self.session_id.trim().is_empty() {
            return Err(AttemptExecutorError::InvalidRequest("execution binding contains an empty identifier"));
        }
        Ok(())
    }
}

/// Trusted, bounded input for one authorized executor start. Scope, profile,
/// permissions and context are resolved before this boundary; model output
/// cannot substitute any of their identifiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttemptExecutionRequest {
    pub binding: ExecutionBinding,
    pub prompt: String,
    pub context_pack: String,
    pub permission_policy: String,
}

impl AttemptExecutionRequest {
    pub fn validate(&self) -> Result<(), AttemptExecutorError> {
        self.binding.validate()?;
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
    async fn start(&self, request: AttemptExecutionRequest) -> Result<ExecutionBinding, AttemptExecutorError>;
    async fn inspect(&self, binding: &ExecutionBinding) -> Result<AttemptExecutionStatus, AttemptExecutorError>;
    async fn steer(&self, binding: &ExecutionBinding, content: String) -> Result<(), AttemptExecutorError>;
    async fn cancel(&self, binding: &ExecutionBinding) -> Result<(), AttemptExecutorError>;
    async fn replay(&self, binding: &ExecutionBinding, after_seq: u64, limit: usize) -> Result<Vec<serde_json::Value>, AttemptExecutorError>;
}

/// Reject an operation whose supplied run does not match the immutable
/// Attempt binding before it reaches the execution runtime.
pub fn require_run(binding: &ExecutionBinding, attempt_id: &str, run_id: &str) -> Result<(), AttemptExecutorError> {
    if binding.attempt_id == attempt_id && binding.run_id == run_id { Ok(()) } else {
        Err(AttemptExecutorError::BindingMismatch { attempt_id: attempt_id.to_string(), run_id: run_id.to_string() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn binding() -> ExecutionBinding { ExecutionBinding { attempt_id: "att_one".into(), run_id: "run_one".into(), session_id: "sess_one".into() } }
    #[test]
    fn executor_requests_are_bounded_and_operations_are_binding_scoped() {
        assert!(AttemptExecutionRequest { binding: binding(), prompt: "go".into(), context_pack: "context".into(), permission_policy: "default".into() }.validate().is_ok());
        assert!(require_run(&binding(), "att_one", "run_other").is_err());
    }
}
