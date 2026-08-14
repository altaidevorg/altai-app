//! Budget policy contracts. A [`Budget`] is a governance policy — a hard limit
//! on a named meter within a [`UsageScope`](crate::UsageScope). It stores no
//! consumption: the usage ledger is the single source of truth, and an enforcer
//! sums the ledger to decide whether a budget's limit is reached. Mirrors the
//! plain-data, no-side-effect shape of the approval and usage contracts.

use crate::{BudgetId, UsageScope};
use serde::{Deserialize, Serialize};

/// A hard-stop policy over one meter within a usage scope. `limit` is the
/// consumption ceiling (same unit as the meter's `amount`); an enforcer stops
/// once the ledger's running total for `(scope, meter)` reaches it. The scope
/// may be broader than any single consumption — a project budget spans every
/// attempt and agent in that project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Budget {
    pub id: BudgetId,
    pub scope: UsageScope,
    pub meter: String,
    pub limit: u64,
    pub created_at_unix_seconds: u64,
}
