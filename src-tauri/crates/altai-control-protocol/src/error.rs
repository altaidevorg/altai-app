//! Typed errors for control-plane operations.
//!
//! All errors are typed enums, never string matching. The [`ControlErrorCode`]
//! provides a stable numeric code for protocol-level error reporting.

use crate::id::IdError;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Stable numeric error codes for the control-plane protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlErrorCode {
    /// The provided ID was malformed.
    InvalidId = 1,
    /// The revision supplied by the caller is stale.
    StaleRevision = 2,
    /// The referenced entity was not found.
    NotFound = 3,
    /// The actor is not authorized for this operation.
    Unauthorized = 4,
    /// A policy rule prevented the operation.
    PolicyDenied = 5,
    /// A budget hard stop prevented the operation.
    BudgetStopped = 6,
    /// A dependency blocker prevents checkout.
    Blocked = 7,
    /// The payload exceeded the maximum allowed size.
    PayloadTooLarge = 8,
    /// A concurrent modification conflict occurred.
    Conflict = 9,
    /// The operation was attempted on a terminal entity.
    AlreadyTerminal = 10,
    /// An internal invariant was violated.
    InternalError = 99,
}

/// Typed control-plane error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlError {
    /// Malformed ID input.
    Id(IdError),
    /// Stale revision for optimistic concurrency.
    StaleRevision {
        expected: u64,
        got: u64,
    },
    /// Entity not found.
    NotFound {
        entity: String,
        id: String,
    },
    /// Actor lacks authorization.
    Unauthorized {
        actor: String,
        action: String,
    },
    /// Policy denied the operation.
    PolicyDenied {
        reason: String,
    },
    /// Budget hard stop.
    BudgetStopped {
        scope: String,
    },
    /// Dependency blocker.
    Blocked {
        blocker_id: String,
    },
    /// Payload too large.
    PayloadTooLarge {
        max_bytes: usize,
        actual_bytes: usize,
    },
    /// Concurrent conflict.
    Conflict {
        reason: String,
    },
    /// Operation on a terminal entity.
    AlreadyTerminal {
        entity: String,
        id: String,
    },
    /// Internal invariant violation.
    InternalError {
        reason: String,
    },
}

impl ControlError {
    pub fn code(&self) -> ControlErrorCode {
        match self {
            Self::Id(_) => ControlErrorCode::InvalidId,
            Self::StaleRevision { .. } => ControlErrorCode::StaleRevision,
            Self::NotFound { .. } => ControlErrorCode::NotFound,
            Self::Unauthorized { .. } => ControlErrorCode::Unauthorized,
            Self::PolicyDenied { .. } => ControlErrorCode::PolicyDenied,
            Self::BudgetStopped { .. } => ControlErrorCode::BudgetStopped,
            Self::Blocked { .. } => ControlErrorCode::Blocked,
            Self::PayloadTooLarge { .. } => ControlErrorCode::PayloadTooLarge,
            Self::Conflict { .. } => ControlErrorCode::Conflict,
            Self::AlreadyTerminal { .. } => ControlErrorCode::AlreadyTerminal,
            Self::InternalError { .. } => ControlErrorCode::InternalError,
        }
    }
}

impl fmt::Display for ControlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Id(e) => write!(f, "invalid id: {e}"),
            Self::StaleRevision { expected, got } => {
                write!(f, "stale revision: expected {expected}, got {got}")
            }
            Self::NotFound { entity, id } => {
                write!(f, "{entity} not found: {id}")
            }
            Self::Unauthorized { actor, action } => {
                write!(f, "unauthorized: {actor} cannot {action}")
            }
            Self::PolicyDenied { reason } => write!(f, "policy denied: {reason}"),
            Self::BudgetStopped { scope } => write!(f, "budget stopped: {scope}"),
            Self::Blocked { blocker_id } => write!(f, "blocked by: {blocker_id}"),
            Self::PayloadTooLarge { max_bytes, actual_bytes } => {
                write!(f, "payload too large: {actual_bytes} > {max_bytes}")
            }
            Self::Conflict { reason } => write!(f, "conflict: {reason}"),
            Self::AlreadyTerminal { entity, id } => {
                write!(f, "{entity} already terminal: {id}")
            }
            Self::InternalError { reason } => write!(f, "internal error: {reason}"),
        }
    }
}

impl std::error::Error for ControlError {}

impl From<IdError> for ControlError {
    fn from(e: IdError) -> Self {
        Self::Id(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_code_mapping() {
        assert_eq!(ControlError::NotFound {
            entity: "work_item".to_string(),
            id: "wi_123".to_string(),
        }
        .code(), ControlErrorCode::NotFound);

        assert_eq!(
            ControlError::StaleRevision { expected: 5, got: 3 }.code(),
            ControlErrorCode::StaleRevision
        );
    }

    #[test]
    fn id_error_converts() {
        let id_err = IdError::EmptyValue;
        let control_err: ControlError = id_err.into();
        assert_eq!(control_err.code(), ControlErrorCode::InvalidId);
    }

    #[test]
    fn display_formats() {
        let err = ControlError::BudgetStopped {
            scope: "org_test".to_string(),
        };
        assert!(format!("{err}").contains("budget stopped"));
    }
}
