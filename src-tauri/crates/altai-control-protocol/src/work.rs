//! Canonical global WorkItem contract.
//!
//! This models global planning authority only. The workspace-local ledger
//! retains exact run/session facts and must synchronize through explicit
//! leases and revisions rather than replacing these fields.

use crate::{GoalId, ProjectId, Revision, WorkItemId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkItemKind {
    Task,
    Ticket,
    Campaign,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkStatus {
    Backlog,
    Todo,
    InProgress,
    InReview,
    Done,
    Blocked,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionPhase {
    None,
    Queued,
    Planning,
    AwaitingPlanApproval,
    Running,
    AwaitingInput,
    AwaitingApproval,
    Verifying,
    Reviewing,
    Retrying,
    Paused,
    Failed,
    NeedsAttention,
    Terminal,
}

/// The control-plane projection of a work object. `parent_work_item_id` is
/// decomposition only; dependencies are modeled separately before dispatch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkItem {
    pub id: WorkItemId,
    pub project_id: ProjectId,
    pub goal_id: Option<GoalId>,
    pub parent_work_item_id: Option<WorkItemId>,
    pub kind: WorkItemKind,
    pub title: String,
    pub description: String,
    pub status: WorkStatus,
    pub execution_phase: ExecutionPhase,
    pub revision: Revision,
    pub created_at: String,
    pub updated_at: String,
}
