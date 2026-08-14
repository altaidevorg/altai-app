//! Hard-stop budget enforcement. [`BudgetEnforcer::enforce`] is the
//! enforcement primitive: for a consumption scope and meter it finds every
//! governing budget, sums the immutable usage ledger within each budget's own
//! scope, and returns [`ControlError::BudgetStopped`] once a running total
//! reaches the limit. Consumption is never stored on a budget — the ledger is
//! the single source of truth. Wiring this onto the live admission path is a
//! follow-up; this delivers and tests the enforcement capability itself.

use std::sync::Arc;

use altai_control_protocol::{Budget, ControlError, UsageScope};

use crate::{
    usage_repository::matches_scope, BudgetError, BudgetRepository, UsageError, UsageRepository,
};

pub struct BudgetEnforcer {
    usage: Arc<dyn UsageRepository>,
    budgets: Arc<dyn BudgetRepository>,
}

impl BudgetEnforcer {
    pub fn new(usage: Arc<dyn UsageRepository>, budgets: Arc<dyn BudgetRepository>) -> Self {
        Self { usage, budgets }
    }

    /// Enforce every budget governing `consumption` for `meter`. Returns
    /// `Ok(())` when all governing budgets have headroom; returns
    /// [`ControlError::BudgetStopped`] for the first budget whose ledger total
    /// (within its own scope, for its meter) has reached the limit.
    pub fn enforce(&self, consumption: &UsageScope, meter: &str) -> Result<(), ControlError> {
        let budgets = self
            .budgets
            .list_in_org(&consumption.organization_id)
            .map_err(budget_internal)?;
        for budget in budgets {
            if budget.meter != meter {
                continue;
            }
            // A budget governs a consumption when the consumption falls within
            // the budget's scope (the budget may be broader: org+project spans
            // every attempt and agent in that project).
            if !matches_scope(consumption, &budget.scope) {
                continue;
            }
            let total = self.total_for(&budget)?;
            if total >= budget.limit {
                return Err(ControlError::BudgetStopped {
                    scope: scope_label(&budget.scope),
                });
            }
        }
        Ok(())
    }

    /// Running ledger total for a budget's (scope, meter).
    fn total_for(&self, budget: &Budget) -> Result<u64, ControlError> {
        let records = self
            .usage
            .list_in_scope(&budget.scope)
            .map_err(usage_internal)?;
        Ok(records
            .into_iter()
            .filter(|record| record.meter == budget.meter)
            .map(|record| record.amount)
            .sum())
    }
}

fn budget_internal(error: BudgetError) -> ControlError {
    ControlError::InternalError {
        reason: format!("budget repository failure: {error}"),
    }
}

fn usage_internal(error: UsageError) -> ControlError {
    ControlError::InternalError {
        reason: format!("usage repository failure: {error}"),
    }
}

/// Compact, human-readable rendering of a scope for the `BudgetStopped` message.
fn scope_label(scope: &UsageScope) -> String {
    let mut parts = vec![format!("org={}", scope.organization_id.value)];
    if let Some(project_id) = &scope.project_id {
        parts.push(format!("project={}", project_id.value));
    }
    if let Some(agent_instance_id) = &scope.agent_instance_id {
        parts.push(format!("agent={}", agent_instance_id.value));
    }
    if let Some(work_item_id) = &scope.work_item_id {
        parts.push(format!("work={}", work_item_id.value));
    }
    if let Some(attempt_id) = &scope.attempt_id {
        parts.push(format!("attempt={}", attempt_id.value));
    }
    parts.join(";")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SqliteBudgetRepository, SqliteUsageRepository};
    use altai_control_protocol::{
        AttemptId, BudgetId, OrganizationId, ProjectId, UsageRecord, UsageRecordId,
    };

    fn harness(dir: &std::path::Path) -> BudgetEnforcer {
        let db = dir.join("work.db");
        let usage: Arc<dyn UsageRepository> = Arc::new(SqliteUsageRepository::open(&db).unwrap());
        let budgets: Arc<dyn BudgetRepository> =
            Arc::new(SqliteBudgetRepository::open(&db).unwrap());
        BudgetEnforcer::new(usage, budgets)
    }

    fn usage(id: &str, scope: UsageScope, meter: &str, amount: u64) -> UsageRecord {
        UsageRecord {
            id: UsageRecordId::new(id),
            scope,
            meter: meter.into(),
            amount,
            recorded_at_unix_seconds: 1,
        }
    }

    fn project_scope(org: &str, project: &str) -> UsageScope {
        UsageScope {
            organization_id: OrganizationId::new(org),
            project_id: Some(ProjectId::new(project)),
            agent_instance_id: None,
            work_item_id: None,
            attempt_id: None,
        }
    }

    fn attempt_scope(org: &str, project: &str, attempt: &str) -> UsageScope {
        let mut scope = project_scope(org, project);
        scope.attempt_id = Some(AttemptId::new(attempt));
        scope
    }

    #[test]
    fn enforce_allows_consumption_under_limit() {
        let dir = tempfile::tempdir().unwrap();
        let enforcer = harness(dir.path());
        // Wire the same repos the enforcer holds by reopening over the same db.
        let usage_repo = SqliteUsageRepository::open(&dir.path().join("work.db")).unwrap();
        let budget_repo = SqliteBudgetRepository::open(&dir.path().join("work.db")).unwrap();

        budget_repo
            .create(Budget {
                id: BudgetId::new("proj-cap"),
                scope: project_scope("org", "proj"),
                meter: "input_tokens".into(),
                limit: 1000,
                created_at_unix_seconds: 0,
            })
            .unwrap();
        usage_repo.record(usage("u1", attempt_scope("org", "proj", "a"), "input_tokens", 600)).unwrap();

        // 600 < 1000: headroom remains.
        assert!(enforcer
            .enforce(&attempt_scope("org", "proj", "a"), "input_tokens")
            .is_ok());
    }

    #[test]
    fn enforce_stops_when_consumption_reaches_limit() {
        let dir = tempfile::tempdir().unwrap();
        let enforcer = harness(dir.path());
        let usage_repo = SqliteUsageRepository::open(&dir.path().join("work.db")).unwrap();
        let budget_repo = SqliteBudgetRepository::open(&dir.path().join("work.db")).unwrap();

        budget_repo
            .create(Budget {
                id: BudgetId::new("proj-cap"),
                scope: project_scope("org", "proj"),
                meter: "input_tokens".into(),
                limit: 1000,
                created_at_unix_seconds: 0,
            })
            .unwrap();
        usage_repo.record(usage("u1", attempt_scope("org", "proj", "a"), "input_tokens", 1000)).unwrap();

        // total == limit: a hard stop fires (fail-safe at the limit).
        let err = enforcer
            .enforce(&attempt_scope("org", "proj", "a"), "input_tokens")
            .unwrap_err();
        assert!(matches!(err, ControlError::BudgetStopped { .. }));
    }

    #[test]
    fn enforce_sums_across_attempts_within_a_budget_scope() {
        let dir = tempfile::tempdir().unwrap();
        let enforcer = harness(dir.path());
        let usage_repo = SqliteUsageRepository::open(&dir.path().join("work.db")).unwrap();
        let budget_repo = SqliteBudgetRepository::open(&dir.path().join("work.db")).unwrap();

        budget_repo
            .create(Budget {
                id: BudgetId::new("proj-cap"),
                scope: project_scope("org", "proj"),
                meter: "input_tokens".into(),
                limit: 1000,
                created_at_unix_seconds: 0,
            })
            .unwrap();
        // Two attempts in the same project jointly exceed the project budget.
        usage_repo.record(usage("u1", attempt_scope("org", "proj", "a"), "input_tokens", 400)).unwrap();
        usage_repo.record(usage("u2", attempt_scope("org", "proj", "b"), "input_tokens", 700)).unwrap();

        // A brand-new attempt in that project is stopped by the project-wide total.
        let err = enforcer
            .enforce(&attempt_scope("org", "proj", "c"), "input_tokens")
            .unwrap_err();
        assert!(matches!(err, ControlError::BudgetStopped { .. }));
    }

    #[test]
    fn enforce_ignores_a_budget_for_a_different_meter() {
        let dir = tempfile::tempdir().unwrap();
        let enforcer = harness(dir.path());
        let usage_repo = SqliteUsageRepository::open(&dir.path().join("work.db")).unwrap();
        let budget_repo = SqliteBudgetRepository::open(&dir.path().join("work.db")).unwrap();

        budget_repo
            .create(Budget {
                id: BudgetId::new("tokens-cap"),
                scope: project_scope("org", "proj"),
                meter: "input_tokens".into(),
                limit: 1000,
                created_at_unix_seconds: 0,
            })
            .unwrap();
        usage_repo.record(usage("u1", attempt_scope("org", "proj", "a"), "compute_seconds", 999_999)).unwrap();

        // compute_seconds is unmetered by any budget: no stop.
        assert!(enforcer
            .enforce(&attempt_scope("org", "proj", "a"), "compute_seconds")
            .is_ok());
    }

    #[test]
    fn enforce_ignores_a_budget_that_does_not_govern_the_scope() {
        let dir = tempfile::tempdir().unwrap();
        let enforcer = harness(dir.path());
        let usage_repo = SqliteUsageRepository::open(&dir.path().join("work.db")).unwrap();
        let budget_repo = SqliteBudgetRepository::open(&dir.path().join("work.db")).unwrap();

        // Budget governs project P only.
        budget_repo
            .create(Budget {
                id: BudgetId::new("p-cap"),
                scope: project_scope("org", "p"),
                meter: "input_tokens".into(),
                limit: 1000,
                created_at_unix_seconds: 0,
            })
            .unwrap();
        // Consumption (and over-limit usage) in project Q is not governed by P's budget.
        usage_repo.record(usage("u1", attempt_scope("org", "q", "a"), "input_tokens", 5000)).unwrap();

        assert!(enforcer
            .enforce(&attempt_scope("org", "q", "a"), "input_tokens")
            .is_ok());
    }

    #[test]
    fn enforce_isolates_by_organization() {
        let dir = tempfile::tempdir().unwrap();
        let enforcer = harness(dir.path());
        let usage_repo = SqliteUsageRepository::open(&dir.path().join("work.db")).unwrap();
        let budget_repo = SqliteBudgetRepository::open(&dir.path().join("work.db")).unwrap();

        budget_repo
            .create(Budget {
                id: BudgetId::new("a-cap"),
                scope: project_scope("org-a", "proj"),
                meter: "input_tokens".into(),
                limit: 1000,
                created_at_unix_seconds: 0,
            })
            .unwrap();
        // Org B has over-limit usage but no budget; org A's budget must not see it.
        usage_repo.record(usage("u1", attempt_scope("org-b", "proj", "a"), "input_tokens", 5000)).unwrap();

        assert!(enforcer
            .enforce(&attempt_scope("org-b", "proj", "a"), "input_tokens")
            .is_ok());
    }
}
