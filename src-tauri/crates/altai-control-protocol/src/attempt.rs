use crate::{AgentInstanceId, AttemptId, RunId, WorkItemId};
use serde::{Deserialize, Serialize};

/// Immutable control-plane binding between an authorized Attempt and the
/// executor run that performs it. A run is execution evidence, not Work
/// identity; an Attempt may only bind one run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunBinding {
    pub attempt_id: AttemptId,
    pub work_item_id: WorkItemId,
    pub owner_agent_instance_id: AgentInstanceId,
    pub run_id: RunId,
    pub bound_at_unix_seconds: u64,
}
