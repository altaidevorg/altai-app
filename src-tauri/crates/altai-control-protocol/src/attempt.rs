use crate::{AgentInstanceId, AgentProfileRevisionId, AttemptId, RunId, WorkItemId};
use serde::{Deserialize, Serialize};

/// Durable lifecycle of a coordinator-authorized execution attempt. Work
/// disposition is deliberately not inferred from any one of these transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptState {
    Created,
    Claimed,
    Dispatched,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    TimedOut,
    BudgetStopped,
    PolicyDenied,
    Lost,
}

impl AttemptState {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Succeeded
                | Self::Failed
                | Self::Cancelled
                | Self::TimedOut
                | Self::BudgetStopped
                | Self::PolicyDenied
                | Self::Lost
        )
    }
}

/// Immutable Attempt identity and execution authority snapshot plus its current
/// lifecycle state. The profile revision is never taken from model input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attempt {
    pub id: AttemptId,
    pub work_item_id: WorkItemId,
    pub owner_agent_instance_id: AgentInstanceId,
    pub profile_revision_id: AgentProfileRevisionId,
    pub state: AttemptState,
    pub created_at_unix_seconds: u64,
    pub updated_at_unix_seconds: u64,
}

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

/// Which scheduler backend owns an attempt. Exactly one backend is bound per
/// attempt and it is immutable once set, so a later managed scheduler (the
/// package-041 cron bridge) cannot register a second backend for an attempt the
/// native scheduler already owns. `NativeLocal` is the single-writer CP-07
/// scheduler; `Managed` is realized by the package-041 cron bridge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleBackend {
    NativeLocal,
    Managed,
}

/// Immutable, exactly-one-per-attempt record of which schedule backend owns an
/// attempt. Mirrors [`RunBinding`]'s immutability contract: rebinding the same
/// backend is idempotent; a divergent backend fails closed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduleBackendBinding {
    pub attempt_id: AttemptId,
    pub backend: ScheduleBackend,
    pub bound_at_unix_seconds: u64,
}
