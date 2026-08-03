//! GLM-CAL-03: pure mapping from the legacy assignment record shape
//! (`TaskRunInfo` in `packages/host-contract/src/types.ts`) to the canonical
//! WorkItem status axes defined by the control-plane plan (Sections 5.1/5.2).
//!
//! The mapping is pure: no I/O, no network, no database access, deterministic.
//! Unknown legacy statuses are rejected with a typed error, never silently
//! mapped. Legacy IDs are preserved verbatim in `legacy_compat_id`; no durable
//! `work_item_id` is minted here — that is the CP-20 migration runner's job.

use std::fmt;

/// Canonical work status values (control-plane plan, Section 5.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkStatus {
    Backlog,
    Todo,
    InProgress,
    InReview,
    Blocked,
    Done,
    Cancelled,
}

impl WorkStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Backlog => "backlog",
            Self::Todo => "todo",
            Self::InProgress => "in_progress",
            Self::InReview => "in_review",
            Self::Blocked => "blocked",
            Self::Done => "done",
            Self::Cancelled => "cancelled",
        }
    }
}

impl fmt::Display for WorkStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Canonical execution phase values (control-plane plan, Section 5.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

impl ExecutionPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Queued => "queued",
            Self::Planning => "planning",
            Self::AwaitingPlanApproval => "awaiting_plan_approval",
            Self::Running => "running",
            Self::AwaitingInput => "awaiting_input",
            Self::AwaitingApproval => "awaiting_approval",
            Self::Verifying => "verifying",
            Self::Reviewing => "reviewing",
            Self::Retrying => "retrying",
            Self::Paused => "paused",
            Self::Failed => "failed",
            Self::NeedsAttention => "needs_attention",
            Self::Terminal => "terminal",
        }
    }
}

impl fmt::Display for ExecutionPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Legacy assignment record as stored in altai-assignments.json (the
/// `TaskRunInfo` concept). `Option` fields model JSON `null` or absent keys;
/// both are rejected as `MissingRequiredField`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyAssignment {
    pub id: Option<String>,
    pub title: Option<String>,
    pub status: Option<String>,
    pub created_at: Option<String>,
}

/// Canonical WorkItem draft produced by the mapping. This is not a durable
/// record: `work_item_id` is always `None` because this pure function never
/// invents IDs — the CP-20 migration runner assigns durable IDs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalWorkItemDraft {
    pub work_item_id: Option<String>,
    pub title: String,
    pub work_status: WorkStatus,
    pub execution_phase: ExecutionPhase,
    pub legacy_compat_id: String,
    pub created_at: String,
}

/// Typed mapping errors. Malformed input is always rejected, never silently
/// mapped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LegacyMappingError {
    MissingRequiredField(&'static str),
    UnknownLegacyStatus(String),
    InvalidLegacyId,
}

impl fmt::Display for LegacyMappingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRequiredField(field) => {
                write!(f, "missing required field {field:?}")
            }
            Self::UnknownLegacyStatus(status) => {
                write!(f, "unknown legacy status {status:?}")
            }
            Self::InvalidLegacyId => write!(f, "legacy id is empty"),
        }
    }
}

impl std::error::Error for LegacyMappingError {}

/// Maps one legacy assignment record to the canonical WorkItem status axes.
///
/// Status mapping (amended 2026-08-03):
/// - `queued`    -> work_status `todo`        + execution_phase `queued`
/// - `running`   -> work_status `in_progress` + execution_phase `running`
/// - `succeeded` -> work_status `done`        + execution_phase `terminal`
/// - `failed`    -> work_status `in_progress` + execution_phase `failed`
/// - `cancelled` -> work_status `cancelled`   + execution_phase `terminal`
///
/// Pure: same input always produces the same output, with no side effects.
pub fn map_legacy_assignment(
    input: &LegacyAssignment,
) -> Result<CanonicalWorkItemDraft, LegacyMappingError> {
    let id = input
        .id
        .as_deref()
        .ok_or(LegacyMappingError::MissingRequiredField("id"))?;
    if id.is_empty() {
        return Err(LegacyMappingError::InvalidLegacyId);
    }
    let title = input
        .title
        .clone()
        .ok_or(LegacyMappingError::MissingRequiredField("title"))?;
    let status = input
        .status
        .as_deref()
        .ok_or(LegacyMappingError::MissingRequiredField("status"))?;
    let created_at = input
        .created_at
        .clone()
        .ok_or(LegacyMappingError::MissingRequiredField("created_at"))?;

    let (work_status, execution_phase) = match status {
        "queued" => (WorkStatus::Todo, ExecutionPhase::Queued),
        "running" => (WorkStatus::InProgress, ExecutionPhase::Running),
        "succeeded" => (WorkStatus::Done, ExecutionPhase::Terminal),
        "failed" => (WorkStatus::InProgress, ExecutionPhase::Failed),
        "cancelled" => (WorkStatus::Cancelled, ExecutionPhase::Terminal),
        other => return Err(LegacyMappingError::UnknownLegacyStatus(other.to_string())),
    };

    Ok(CanonicalWorkItemDraft {
        work_item_id: None,
        title,
        work_status,
        execution_phase,
        legacy_compat_id: id.to_string(),
        created_at,
    })
}
