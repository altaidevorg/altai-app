//! Durable routine contracts. A Routine is a versioned recurring or
//! event-triggered work definition (the `routines.*` abstraction the package-041
//! cron bridge materializes). Its intent lives in immutable [`RoutineRevision`]
//! records; the [`Routine`] aggregate points at its current revision. Nothing
//! here registers a scheduler — that is the bridge's job, and the package-036
//! schedule-backend seam already enforces exactly one backend per attempt.

use crate::{OrganizationId, Revision, RoutineId, RoutineRevisionId, WorkItemId};
use serde::{Deserialize, Serialize};

/// Lifecycle of a routine aggregate. Mirrors the agent lifecycle: only an
/// `Active` routine is materialized by a scheduler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutineStatus {
    Active,
    Paused,
    Retired,
}

/// When a routine fires. `Recurring` carries a cron-compatible expression that
/// the package-041 cron bridge interprets; `Event` fires on a named source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutineTrigger {
    Recurring { cron_expression: String },
    Event { source: String },
}

/// Immutable, append-only routine intent. `revision` is the monotonic intent
/// version within the routine (distinct from the aggregate [`Routine::revision`],
/// which advances on each accepted mutation). `target_work_item_id` is the work
/// item this routine drives when its trigger fires.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoutineRevision {
    pub id: RoutineRevisionId,
    pub routine_id: RoutineId,
    pub revision: Revision,
    pub trigger: RoutineTrigger,
    pub target_work_item_id: WorkItemId,
    pub created_at_unix_seconds: u64,
}

/// Versioned recurring or event-triggered work definition. Owns identity and
/// lifecycle; points at its current immutable revision. The aggregate `revision`
/// advances on each accepted mutation for optimistic concurrency control.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Routine {
    pub id: RoutineId,
    pub organization_id: OrganizationId,
    pub current_revision_id: Option<RoutineRevisionId>,
    pub status: RoutineStatus,
    pub revision: Revision,
    pub created_at_unix_seconds: u64,
    pub updated_at_unix_seconds: u64,
}
