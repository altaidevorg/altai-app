//! Durable Work hierarchy, dependency, and comment contracts.

use crate::{Actor, AttemptId, Revision, RunId, WorkItemId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkDependency {
    pub work_item_id: WorkItemId,
    pub blocker_work_item_id: WorkItemId,
    pub created_at: String,
}

/// Durable attributed work communication. Parentage and dependencies are not
/// inferred from comments or from each other.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkComment {
    pub id: String,
    pub work_item_id: WorkItemId,
    pub actor: Actor,
    pub body: String,
    pub created_by_attempt_id: Option<AttemptId>,
    pub created_by_run_id: Option<RunId>,
    pub revision: Revision,
    pub created_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkRelationKind {
    Parent,
    Blocks,
}
