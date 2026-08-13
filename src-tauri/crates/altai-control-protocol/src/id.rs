//! Canonical typed IDs for every control-plane concept (parent plan §3.3).
//!
//! Each ID serializes as `{"type":"<kind>","value":"<prefix>..."}`. The
//! `type` field makes the JSON self-describing and prevents accidental
//! substitution of one ID for another. Parsing rejects:
//!
//! - non-object input,
//! - wrong or missing `type` field,
//! - empty value,
//! - value missing the required prefix.

use serde::{Deserialize, Serialize};
use std::fmt;

// ---------------------------------------------------------------------------
// Macro to generate typed ID structs without boilerplate.
// ---------------------------------------------------------------------------

macro_rules! define_typed_id {
    (
        $(#[$meta:meta])*
        $struct_name:ident,
        $type_str:literal,
        $prefix:literal,
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $struct_name {
            #[serde(rename = "type")]
            pub kind: String,
            pub value: String,
        }

        impl $struct_name {
            pub const TYPE: &'static str = $type_str;
            pub const PREFIX: &'static str = $prefix;

            /// Create a new ID from a raw value string. The prefix is
            /// prepended if missing; this is the only constructor that
            /// does not require the caller to supply the prefix.
            pub fn new(value: impl Into<String>) -> Self {
                let mut v = value.into();
                if !v.starts_with($prefix) {
                    v = format!("{}{}", $prefix, v);
                }
                Self {
                    kind: $type_str.to_string(),
                    value: v,
                }
            }

            /// Parse an unknown JSON value into this ID, rejecting malformed
            /// input with a typed [`IdError`].
            pub fn parse(input: &serde_json::Value) -> Result<Self, IdError> {
                let obj = input.as_object().ok_or(IdError::InvalidShape)?;
                let kind = obj
                    .get("type")
                    .and_then(|v| v.as_str())
                    .ok_or(IdError::MissingType)?;
                if kind != $type_str {
                    return Err(IdError::WrongType {
                        expected: $type_str,
                        got: kind.to_string(),
                    });
                }
                let value = obj
                    .get("value")
                    .and_then(|v| v.as_str())
                    .ok_or(IdError::InvalidShape)?;
                if value.is_empty() {
                    return Err(IdError::EmptyValue);
                }
                if !value.starts_with($prefix) {
                    return Err(IdError::MissingPrefix {
                        expected: $prefix,
                        got: value.to_string(),
                    });
                }
                Ok(Self {
                    kind: $type_str.to_string(),
                    value: value.to_string(),
                })
            }

            /// Serialize to the canonical compact JSON form.
            pub fn to_json(&self) -> String {
                serde_json::to_string(self).expect("typed IDs always serialize")
            }

            /// Serialize to a `serde_json::Value`.
            pub fn to_json_value(&self) -> serde_json::Value {
                serde_json::to_value(self).expect("typed IDs always serialize")
            }
        }

        impl fmt::Display for $struct_name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.value)
            }
        }
    };
}

define_typed_id!(
    /// Isolation, policy, agent, and budget boundary.
    OrganizationId,
    "organization_id",
    "org_",
);
define_typed_id!(
    /// Desired outcome and ancestry.
    GoalId,
    "goal_id",
    "goal_",
);
define_typed_id!(
    /// Work, repository/workspace, policy, and delivery context.
    ProjectId,
    "project_id",
    "proj_",
);
define_typed_id!(
    /// Workspace identity within a project.
    WorkspaceId,
    "workspace_id",
    "ws_",
);
define_typed_id!(
    /// Durable worker identity.
    AgentInstanceId,
    "agent_instance_id",
    "ai_",
);
define_typed_id!(
    /// Reusable agent configuration.
    AgentProfileId,
    "agent_profile_id",
    "ap_",
);
define_typed_id!(
    /// Immutable snapshot of an agent profile.
    AgentProfileRevisionId,
    "agent_profile_revision_id",
    "apr_",
);
define_typed_id!(
    /// User- or agent-visible unit of project work.
    WorkItemId,
    "work_item_id",
    "wi_",
);
define_typed_id!(
    /// One coordinator-authorized execution attempt.
    AttemptId,
    "attempt_id",
    "att_",
);
define_typed_id!(
    /// IsanAgent execution corresponding to an attempt.
    RunId,
    "run_id",
    "run_",
);
define_typed_id!(
    /// Chat session (execution context, not task ownership).
    SessionId,
    "session_id",
    "sess_",
);
define_typed_id!(
    /// Versioned recurring or event-triggered work definition.
    RoutineId,
    "routine_id",
    "rt_",
);
define_typed_id!(
    /// Immutable revision of a routine.
    RoutineRevisionId,
    "routine_revision_id",
    "rtr_",
);
define_typed_id!(
    /// One execution of a routine trigger.
    RoutineRunId,
    "routine_run_id",
    "rr_",
);
define_typed_id!(
    /// Governance decision with explicit scope and payload.
    ApprovalId,
    "approval_id",
    "apv_",
);
define_typed_id!(
    /// External tracker object link.
    ExternalObjectId,
    "external_object_id",
    "ext_",
);

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

/// Typed error for ID parsing failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdError {
    /// Input is not a JSON object.
    InvalidShape,
    /// `type` field is missing.
    MissingType,
    /// `type` field does not match the expected kind.
    WrongType { expected: &'static str, got: String },
    /// `value` is empty.
    EmptyValue,
    /// `value` does not start with the required prefix.
    MissingPrefix { expected: &'static str, got: String },
}

impl fmt::Display for IdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidShape => write!(f, "id must be a JSON object"),
            Self::MissingType => write!(f, "id missing 'type' field"),
            Self::WrongType { expected, got } => {
                write!(f, "id type mismatch: expected {expected}, got {got}")
            }
            Self::EmptyValue => write!(f, "id value must not be empty"),
            Self::MissingPrefix { expected, got } => {
                write!(f, "id value must start with {expected}, got {got}")
            }
        }
    }
}

impl std::error::Error for IdError {}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn organization_id_round_trip() {
        let id = OrganizationId::new("01923abc-def0-7abc-8def-0123456789ab");
        let json = id.to_json();
        assert_eq!(
            json,
            r#"{"type":"organization_id","value":"org_01923abc-def0-7abc-8def-0123456789ab"}"#
        );
        let parsed = OrganizationId::parse(&serde_json::from_str(&json).unwrap()).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn work_item_id_round_trip() {
        let id = WorkItemId::new("01923abc-def0-7abc-8def-0123456789ab");
        let json = id.to_json();
        assert_eq!(
            json,
            r#"{"type":"work_item_id","value":"wi_01923abc-def0-7abc-8def-0123456789ab"}"#
        );
        let parsed = WorkItemId::parse(&serde_json::from_str(&json).unwrap()).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn all_id_types_have_distinct_prefixes() {
        // Ensure no two ID types share the same type string.
        let types = [
            OrganizationId::TYPE, GoalId::TYPE, ProjectId::TYPE, WorkspaceId::TYPE,
            AgentInstanceId::TYPE, AgentProfileId::TYPE, AgentProfileRevisionId::TYPE,
            WorkItemId::TYPE, AttemptId::TYPE, RunId::TYPE, SessionId::TYPE,
            RoutineId::TYPE, RoutineRevisionId::TYPE, RoutineRunId::TYPE,
            ApprovalId::TYPE, ExternalObjectId::TYPE,
        ];
        let mut seen = std::collections::HashSet::new();
        for t in types {
            assert!(seen.insert(t), "duplicate ID type string: {t}");
        }
    }

    #[test]
    fn parse_rejects_wrong_type() {
        let json = serde_json::json!({"type": "organization_id", "value": "org_test"});
        let err = WorkItemId::parse(&json).unwrap_err();
        assert_eq!(
            err,
            IdError::WrongType {
                expected: "work_item_id",
                got: "organization_id".to_string(),
            }
        );
    }

    #[test]
    fn parse_rejects_empty_value() {
        let json = serde_json::json!({"type": "work_item_id", "value": ""});
        let err = WorkItemId::parse(&json).unwrap_err();
        assert_eq!(err, IdError::EmptyValue);
    }

    #[test]
    fn parse_rejects_missing_prefix() {
        let json = serde_json::json!({"type": "work_item_id", "value": "no_prefix"});
        let err = WorkItemId::parse(&json).unwrap_err();
        assert!(matches!(err, IdError::MissingPrefix { .. }));
    }

    #[test]
    fn parse_rejects_non_object() {
        let json = serde_json::json!("not an object");
        let err = WorkItemId::parse(&json).unwrap_err();
        assert_eq!(err, IdError::InvalidShape);
    }

    #[test]
    fn parse_rejects_missing_type() {
        let json = serde_json::json!({"value": "wi_test"});
        let err = WorkItemId::parse(&json).unwrap_err();
        assert_eq!(err, IdError::MissingType);
    }

    #[test]
    fn new_prepends_prefix_if_missing() {
        let id = WorkItemId::new("abc123");
        assert_eq!(id.value, "wi_abc123");
    }

    #[test]
    fn new_preserves_prefix_if_present() {
        let id = WorkItemId::new("wi_abc123");
        assert_eq!(id.value, "wi_abc123");
    }
}
