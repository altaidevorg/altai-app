//! Durable approval (governance) contracts. An [`Approval`] is a governance
//! decision request with an explicit scope and payload revision; its resolution
//! is an immutable [`ApprovalDecision`] audit record. The aggregate advances its
//! `outcome` when decided, mirroring how a [`Routine`](crate::Routine) points at
//! its current revision. Nothing here enqueues a wake or mutates a work item's
//! execution phase — that is a later package's job; this is the durable,
//! immutable-audit seam.

use crate::{ApprovalId, AttemptId, OrganizationId, Revision, WorkItemId};
use serde::{Deserialize, Serialize};

/// What an approval governs. Plan approval gates an attempt's plan before it
/// runs (the `awaiting_plan_approval` execution phase); delivery approval
/// governs releasing a work item's output (the `DeliveryGate` authorization).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ApprovalScope {
    Plan { attempt_id: AttemptId },
    Delivery { work_item_id: WorkItemId },
}

/// A recorded governance outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalOutcome {
    Approved,
    Denied,
}

/// Immutable resolution of an approval. Append-only and first-writer-wins: an
/// approval is decided exactly once. Re-recording an identical decision is
/// idempotent; a divergent decision is rejected so the audit never contradicts
/// itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalDecision {
    pub approval_id: ApprovalId,
    pub outcome: ApprovalOutcome,
    pub decided_by: String,
    pub decided_at_unix_seconds: u64,
    pub reason: Option<String>,
}

/// Governance decision request with explicit scope and payload revision. Owns
/// identity and the pending/resolved lifecycle; `outcome` is `None` until an
/// immutable decision lands. `payload_revision` is the version of the governed
/// payload this request covers; the aggregate `revision` advances on each
/// accepted mutation for optimistic concurrency control.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Approval {
    pub id: ApprovalId,
    pub organization_id: OrganizationId,
    pub scope: ApprovalScope,
    pub payload_revision: Revision,
    pub outcome: Option<ApprovalOutcome>,
    pub revision: Revision,
    pub created_at_unix_seconds: u64,
    pub resolved_at_unix_seconds: Option<u64>,
}
