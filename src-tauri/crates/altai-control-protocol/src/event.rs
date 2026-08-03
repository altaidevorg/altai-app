//! Event types for the control plane.
//!
//! [`ActivityEvent`] is the audit-level event: who did what, when, and why.
//! [`ControlEvent`] is the append-only per-aggregate event used for
//! event-sourced projections and recovery.

use crate::actor::Actor;
use crate::id::*;
use crate::revision::Revision;
use serde::{Deserialize, Serialize};

/// The kind of activity, for filtering and projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    /// An entity was created.
    Created,
    /// An entity was updated.
    Updated,
    /// An entity's status changed.
    StatusChanged,
    /// An entity was assigned.
    Assigned,
    /// A wake request was created or claimed.
    WakeRequested,
    /// An attempt was created, dispatched, or finalized.
    AttemptTransitioned,
    /// A routine was created or triggered.
    RoutineTriggered,
    /// An approval was requested or resolved.
    ApprovalTransitioned,
    /// A budget event occurred.
    BudgetEvent,
    /// An external sync event occurred.
    ExternalSync,
    /// A recovery action occurred.
    Recovery,
}

/// Audit-level activity event. Appended to `activity_events` and used for
/// the Activity & Audit screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivityEvent {
    /// Globally unique event ID (ULID).
    pub event_id: String,
    /// What kind of activity.
    pub kind: EventKind,
    /// Who performed the action.
    pub actor: Actor,
    /// When the event occurred (ISO 8601 UTC, supplied by the service clock).
    pub timestamp: String,
    /// The organization this event belongs to.
    pub organization_id: OrganizationId,
    /// Optional project scope.
    pub project_id: Option<ProjectId>,
    /// Optional work item scope.
    pub work_item_id: Option<WorkItemId>,
    /// Optional attempt scope.
    pub attempt_id: Option<AttemptId>,
    /// Human-readable summary for the audit feed.
    pub summary: String,
    /// Correlation ID for tracing across events.
    pub correlation_id: Option<String>,
    /// Causation ID (what caused this event, if any).
    pub causation_id: Option<String>,
}

/// Append-only per-aggregate event. Each aggregate has its own sequence
/// number; replaying events from a checkpoint reconstructs projections.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlEvent {
    /// The aggregate this event belongs to (e.g. "work_item", "attempt").
    pub aggregate: String,
    /// The aggregate's ID as a JSON value (typed ID struct).
    pub aggregate_id: serde_json::Value,
    /// Monotonically increasing sequence within the aggregate.
    pub sequence: u64,
    /// The event kind.
    pub kind: EventKind,
    /// Who caused the event.
    pub actor: Actor,
    /// When the event occurred (ISO 8601 UTC).
    pub timestamp: String,
    /// The revision of the aggregate after this event.
    pub revision: Revision,
    /// The event payload (domain-specific JSON).
    pub payload: serde_json::Value,
    /// Correlation ID for tracing.
    pub correlation_id: Option<String>,
    /// Causation ID.
    pub causation_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activity_event_serializes() {
        let org = OrganizationId::new("test-org");
        let event = ActivityEvent {
            event_id: "evt_01".to_string(),
            kind: EventKind::Created,
            actor: Actor::System {
                component: "migration".to_string(),
            },
            timestamp: "2026-08-03T19:00:00Z".to_string(),
            organization_id: org,
            project_id: None,
            work_item_id: None,
            attempt_id: None,
            summary: "imported legacy data".to_string(),
            correlation_id: None,
            causation_id: None,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"kind\":\"created\""));
        assert!(json.contains("\"evt_01\""));
    }

    #[test]
    fn control_event_serializes() {
        let org = OrganizationId::new("test-org");
        let event = ControlEvent {
            aggregate: "work_item".to_string(),
            aggregate_id: WorkItemId::new("test-wi").to_json_value(),
            sequence: 1,
            kind: EventKind::Created,
            actor: Actor::System {
                component: "migration".to_string(),
            },
            timestamp: "2026-08-03T19:00:00Z".to_string(),
            revision: Revision::new(1),
            payload: serde_json::json!({"title": "Test work item"}),
            correlation_id: None,
            causation_id: None,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"aggregate\":\"work_item\""));
        assert!(json.contains("\"sequence\":1"));
        // Revision serializes as a bare number.
        assert!(json.contains("\"revision\":1"));
    }

    #[test]
    fn event_kind_round_trip() {
        let kinds = [
            EventKind::Created,
            EventKind::Updated,
            EventKind::StatusChanged,
            EventKind::Assigned,
            EventKind::WakeRequested,
            EventKind::AttemptTransitioned,
            EventKind::RoutineTriggered,
            EventKind::ApprovalTransitioned,
            EventKind::BudgetEvent,
            EventKind::ExternalSync,
            EventKind::Recovery,
        ];
        for kind in kinds {
            let json = serde_json::to_string(&kind).unwrap();
            let parsed: EventKind = serde_json::from_str(&json).unwrap();
            assert_eq!(kind, parsed);
        }
    }
}
