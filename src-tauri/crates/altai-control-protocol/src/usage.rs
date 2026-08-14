//! Durable usage/cost ledger contracts. A [`UsageRecord`] is an immutable,
//! append-only meter reading — the atomic unit of the ledger. The ledger sums
//! `amount` per (scope, meter) so a budget can enforce hard stops; nothing here
//! sets a limit or stops anything. This is the durable, immutable-audit seam
//! for package 043, mirroring how the approval contract (package 042) is plain
//! data with no side effects.

use crate::{AgentInstanceId, AttemptId, OrganizationId, ProjectId, UsageRecordId, WorkItemId};
use serde::{Deserialize, Serialize};

/// Attribution dimensions for a metered cost fact. Doubles as the scope a
/// budget will govern: `organization_id` is always required (the
/// isolation/policy/budget boundary); the remaining dims narrow attribution —
/// or a budget's span — to a project, agent, work item and/or attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageScope {
    pub organization_id: OrganizationId,
    pub project_id: Option<ProjectId>,
    pub agent_instance_id: Option<AgentInstanceId>,
    pub work_item_id: Option<WorkItemId>,
    pub attempt_id: Option<AttemptId>,
}

/// One immutable, append-only meter reading. A single named meter (e.g.
/// `"input_tokens"`, `"compute_seconds"`) consumed by one attributed
/// [`UsageScope`] at one instant. Callers assign a stable `id` so an identical
/// re-record is idempotent; a divergent same-id record is rejected so the audit
/// never contradicts itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageRecord {
    pub id: UsageRecordId,
    pub scope: UsageScope,
    pub meter: String,
    pub amount: u64,
    pub recorded_at_unix_seconds: u64,
}
