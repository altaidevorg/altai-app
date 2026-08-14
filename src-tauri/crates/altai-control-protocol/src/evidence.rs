//! Evidence contracts. An [`Evidence`] record is an immutable, append-only
//! reference to an artifact an attempt produced — a file path, diff ref,
//! test-result blob, or summary. It is the foundational unit package 045's
//! completion gate will require: before Work may complete, the system checks
//! that the governing attempt produced the required evidence. Nothing here
//! gates anything; this is plain, attributed data with no side effects,
//! mirroring the usage and approval contracts.

use crate::{AttemptId, EvidenceId, OrganizationId, WorkItemId};
use serde::{Deserialize, Serialize};

/// One immutable evidence artifact reference, attributed to the attempt that
/// produced it and the work item it belongs to. `kind` (e.g. `"artifact_ref"`,
/// `"summary"`, `"test_result"`) lets a gate require a specific kind without
/// enumerating them; `reference` is the artifact locator (path, URI, or digest).
/// Callers assign a stable `id` so an identical re-record is idempotent; a
/// divergent same-id record is rejected so the audit never contradicts itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evidence {
    pub id: EvidenceId,
    pub organization_id: OrganizationId,
    pub work_item_id: WorkItemId,
    pub attempt_id: AttemptId,
    pub kind: String,
    pub reference: String,
    pub created_at_unix_seconds: u64,
}
