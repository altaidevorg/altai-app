//! Revision for optimistic concurrency control.
//!
//! Every mutable aggregate carries a `Revision` that increases monotonically
//! on each accepted mutation. Concurrent edits must supply the expected
//! revision; a stale revision is rejected with [`ControlError::StaleRevision`].

use serde::{Deserialize, Serialize};
use std::fmt;

/// Monotonically increasing revision number for an aggregate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Revision(pub u64);

impl Revision {
    /// The initial revision for a newly created aggregate.
    pub const INITIAL: Revision = Revision(0);

    /// Create a revision from a raw u64.
    pub fn new(value: u64) -> Self {
        Revision(value)
    }

    /// Return the next revision. Does not mutate self; callers should assign.
    pub fn next(self) -> Self {
        Revision(self.0 + 1)
    }

    /// Current raw value.
    pub fn value(self) -> u64 {
        self.0
    }
}

impl Default for Revision {
    fn default() -> Self {
        Self::INITIAL
    }
}

impl fmt::Display for Revision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "rev_{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revision_increments() {
        let r = Revision::INITIAL;
        assert_eq!(r.value(), 0);
        assert_eq!(r.next().value(), 1);
    }

    #[test]
    fn revision_orders() {
        assert!(Revision(1) > Revision(0));
        assert!(Revision(5) < Revision(10));
    }

    #[test]
    fn revision_serializes_as_number() {
        let json = serde_json::to_string(&Revision(42)).unwrap();
        assert_eq!(json, "42");
        let parsed: Revision = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, Revision(42));
    }
}
