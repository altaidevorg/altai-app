//! Host-side CP-08 adapter. It is the only place where canonical control-plane
//! identities are converted to the execution service's opaque boundary types.
//! The adapter cannot mutate WorkItem state or scheduler policy.

use altai_agent_service::{AttemptExecutionRequest, AttemptExecutorError, ExecutionBinding};
use altai_control_protocol::{Attempt, AttemptState, RunBinding, SessionId};

/// CP-08-05 consumes this adapter from the authenticated host command.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedAttemptInput {
    pub attempt: Attempt,
    pub run_binding: RunBinding,
    pub session_id: SessionId,
    pub prompt: String,
    pub context_pack: String,
    pub permission_policy: String,
}

#[allow(dead_code)]
pub fn adapt_attempt(
    input: TrustedAttemptInput,
) -> Result<AttemptExecutionRequest, AttemptExecutorError> {
    if !matches!(
        input.attempt.state,
        AttemptState::Dispatched | AttemptState::Running
    ) {
        return Err(AttemptExecutorError::InvalidRequest(
            "attempt must be dispatched or running before execution",
        ));
    }
    if input.attempt.id != input.run_binding.attempt_id
        || input.attempt.work_item_id != input.run_binding.work_item_id
        || input.attempt.owner_agent_instance_id != input.run_binding.owner_agent_instance_id
    {
        return Err(AttemptExecutorError::BindingMismatch {
            attempt_id: input.attempt.id.value,
            run_id: input.run_binding.run_id.value,
        });
    }
    let request = AttemptExecutionRequest {
        binding: ExecutionBinding {
            attempt_id: input.run_binding.attempt_id.value,
            run_id: input.run_binding.run_id.value,
            session_id: input.session_id.value,
        },
        prompt: input.prompt,
        context_pack: input.context_pack,
        permission_policy: input.permission_policy,
    };
    request.validate()?;
    Ok(request)
}

#[cfg(test)]
mod tests {
    use super::*;
    use altai_control_protocol::{
        AgentInstanceId, AgentProfileRevisionId, AttemptId, RunId, WorkItemId,
    };

    fn input() -> TrustedAttemptInput {
        TrustedAttemptInput {
            attempt: Attempt {
                id: AttemptId::new("one"),
                work_item_id: WorkItemId::new("one"),
                owner_agent_instance_id: AgentInstanceId::new("one"),
                profile_revision_id: AgentProfileRevisionId::new("one"),
                state: AttemptState::Dispatched,
                created_at_unix_seconds: 1,
                updated_at_unix_seconds: 2,
            },
            run_binding: RunBinding {
                attempt_id: AttemptId::new("one"),
                work_item_id: WorkItemId::new("one"),
                owner_agent_instance_id: AgentInstanceId::new("one"),
                run_id: RunId::new("one"),
                bound_at_unix_seconds: 2,
            },
            session_id: SessionId::new("one"),
            prompt: "complete the authorized work".into(),
            context_pack: "bounded context".into(),
            permission_policy: "default".into(),
        }
    }

    #[test]
    fn adapts_only_a_matching_dispatched_attempt() {
        let request = adapt_attempt(input()).unwrap();
        assert_eq!(request.binding.attempt_id, "att_one");
        assert_eq!(request.binding.run_id, "run_one");

        let mut mismatched = input();
        mismatched.run_binding.run_id = RunId::new("other");
        mismatched.run_binding.work_item_id = WorkItemId::new("other");
        assert!(matches!(
            adapt_attempt(mismatched),
            Err(AttemptExecutorError::BindingMismatch { .. })
        ));
    }

    #[test]
    fn refuses_unready_attempts() {
        let mut unready = input();
        unready.attempt.state = AttemptState::Claimed;
        assert!(matches!(
            adapt_attempt(unready),
            Err(AttemptExecutorError::InvalidRequest(_))
        ));
    }
}
