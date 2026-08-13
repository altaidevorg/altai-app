use crate::{AttemptId, WorkItemId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryDisposition {
    RetryQueued,
    DeadLettered,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryRecord {
    pub work_item_id: WorkItemId,
    pub attempt_id: AttemptId,
    pub retry_count: u32,
    pub max_retries: u32,
    pub last_failure: String,
    pub disposition: RecoveryDisposition,
    pub updated_at_unix_seconds: u64,
}
