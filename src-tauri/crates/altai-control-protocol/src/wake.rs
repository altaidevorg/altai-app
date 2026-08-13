//! Durable wake queue and exclusive checkout lease contracts.

use crate::{AgentInstanceId, AttemptId, WorkItemId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WakeSource {
    Assignment,
    Comment,
    Mention,
    Routine,
    ApprovalResult,
    Retry,
    Recovery,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WakeRequest {
    pub id: String,
    pub work_item_id: WorkItemId,
    pub sources: Vec<WakeSource>,
    pub requested_at: String,
    pub claimed_at: Option<String>,
}

/// Exclusive work ownership. This is not a RunBinding: a lease can expire or
/// be compare-and-cleared independently of the executor's run lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkCheckoutLease {
    pub work_item_id: WorkItemId,
    pub owner_agent_instance_id: AgentInstanceId,
    pub attempt_id: AttemptId,
    pub expires_at: String,
}
