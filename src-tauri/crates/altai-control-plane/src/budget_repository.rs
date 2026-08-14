//! CP-08 budget policy storage. A [`Budget`] is an immutable policy row (a
//! hard limit over a usage scope and meter); the store is insert-only and
//! idempotent on identical replay, failing closed with [`BudgetError::Conflict`]
//! on a divergent same-id row. Consumption is never stored here — it is derived
//! from the usage ledger by the budget enforcer, so there is one source of
//! truth and no double-bookkeeping.

use altai_control_protocol::{Budget, BudgetId, OrganizationId};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use std::{path::Path, sync::Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BudgetError {
    Conflict { budget_id: String },
    Internal { reason: String },
}

impl std::fmt::Display for BudgetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "budget error: {self:?}")
    }
}
impl std::error::Error for BudgetError {}

pub trait BudgetRepository: Send + Sync {
    /// Create a budget policy row. Idempotent when the same budget is
    /// re-created; fails closed with [`BudgetError::Conflict`] when a different
    /// budget already owns the id.
    fn create(&self, budget: Budget) -> Result<Budget, BudgetError>;
    fn get(&self, id: &BudgetId) -> Result<Option<Budget>, BudgetError>;
    /// Every budget policy in an organization (the set an enforcer scans).
    fn list_in_org(&self, organization_id: &OrganizationId) -> Result<Vec<Budget>, BudgetError>;
}

pub struct SqliteBudgetRepository {
    connection: Mutex<Connection>,
}

impl SqliteBudgetRepository {
    pub fn open(path: &Path) -> Result<Self, String> {
        let connection = Connection::open(path).map_err(|e| e.to_string())?;
        connection.execute_batch(
            "PRAGMA busy_timeout = 5000; CREATE TABLE IF NOT EXISTS control_plane_budgets (budget_id TEXT PRIMARY KEY, payload_json TEXT NOT NULL);",
        )
        .map_err(|e| e.to_string())?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, BudgetError> {
        self.connection.lock().map_err(|_| BudgetError::Internal {
            reason: "sqlite budget lock poisoned".into(),
        })
    }
    fn db(e: rusqlite::Error) -> BudgetError {
        BudgetError::Internal { reason: e.to_string() }
    }
}

impl BudgetRepository for SqliteBudgetRepository {
    fn create(&self, budget: Budget) -> Result<Budget, BudgetError> {
        let payload = serde_json::to_string(&budget)
            .map_err(|e| BudgetError::Internal { reason: e.to_string() })?;
        let mut connection = self.lock()?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(Self::db)?;
        let inserted = tx
            .execute(
                "INSERT INTO control_plane_budgets (budget_id, payload_json) VALUES (?1, ?2) ON CONFLICT DO NOTHING",
                params![budget.id.value, payload],
            )
            .map_err(Self::db)?;
        if inserted == 1 {
            tx.commit().map_err(Self::db)?;
            return Ok(budget);
        }
        let existing = Self::read_budget(&tx, &budget.id)?.ok_or_else(|| BudgetError::Internal {
            reason: "budget disappeared after insert conflict".into(),
        })?;
        if existing == budget {
            tx.commit().map_err(Self::db)?;
            Ok(existing)
        } else {
            Err(BudgetError::Conflict {
                budget_id: budget.id.value,
            })
        }
    }

    fn get(&self, id: &BudgetId) -> Result<Option<Budget>, BudgetError> {
        let connection = self.lock()?;
        Self::read_budget(&connection, id)
    }

    fn list_in_org(&self, organization_id: &OrganizationId) -> Result<Vec<Budget>, BudgetError> {
        let connection = self.lock()?;
        let mut stmt = connection
            .prepare("SELECT payload_json FROM control_plane_budgets")
            .map_err(Self::db)?;
        let payloads = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(Self::db)?;
        let mut budgets = Vec::new();
        for payload in payloads {
            let budget: Budget =
                serde_json::from_str(&payload.map_err(Self::db)?).map_err(|e| BudgetError::Internal {
                    reason: e.to_string(),
                })?;
            if budget.scope.organization_id == *organization_id {
                budgets.push(budget);
            }
        }
        Ok(budgets)
    }
}

impl SqliteBudgetRepository {
    fn read_budget(connection: &Connection, id: &BudgetId) -> Result<Option<Budget>, BudgetError> {
        let payload: Option<String> = connection
            .query_row(
                "SELECT payload_json FROM control_plane_budgets WHERE budget_id=?1",
                [&id.value],
                |row| row.get(0),
            )
            .optional()
            .map_err(Self::db)?;
        payload
            .map(|p| {
                serde_json::from_str(&p).map_err(|e| BudgetError::Internal { reason: e.to_string() })
            })
            .transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use altai_control_protocol::{AttemptId, UsageScope};

    fn budget(id: &str, org: &str, meter: &str, limit: u64) -> Budget {
        Budget {
            id: BudgetId::new(id),
            scope: UsageScope {
                organization_id: OrganizationId::new(org),
                project_id: None,
                agent_instance_id: None,
                work_item_id: None,
                attempt_id: Some(AttemptId::new("att")),
            },
            meter: meter.into(),
            limit,
            created_at_unix_seconds: 10,
        }
    }

    #[test]
    fn create_is_durable_and_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let database = dir.path().join("work.db");
        let repo = SqliteBudgetRepository::open(&database).unwrap();
        let id = BudgetId::new("b1");
        repo.create(budget("b1", "org", "input_tokens", 1000)).unwrap();
        // Idempotent replay.
        repo.create(budget("b1", "org", "input_tokens", 1000)).unwrap();

        let reopened = SqliteBudgetRepository::open(&database).unwrap();
        let stored = reopened.get(&id).unwrap().unwrap();
        assert_eq!(stored.limit, 1000);
        assert_eq!(stored.scope.organization_id, OrganizationId::new("org"));
    }

    #[test]
    fn create_rejects_a_divergent_same_id_budget() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteBudgetRepository::open(&dir.path().join("work.db")).unwrap();
        repo.create(budget("b1", "org", "input_tokens", 1000)).unwrap();
        let err = repo
            .create(budget("b1", "org", "input_tokens", 5000))
            .unwrap_err();
        assert!(matches!(err, BudgetError::Conflict { .. }));
        // Original policy unchanged.
        assert_eq!(repo.get(&BudgetId::new("b1")).unwrap().unwrap().limit, 1000);
    }

    #[test]
    fn list_in_org_isolates_by_organization() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteBudgetRepository::open(&dir.path().join("work.db")).unwrap();
        repo.create(budget("b1", "org-a", "input_tokens", 1000)).unwrap();
        repo.create(budget("b2", "org-a", "compute_seconds", 60)).unwrap();
        repo.create(budget("b3", "org-b", "input_tokens", 1000)).unwrap();

        let org_a = repo.list_in_org(&OrganizationId::new("org-a")).unwrap();
        let ids: Vec<&str> = org_a.iter().map(|b| b.id.value.as_str()).collect();
        assert_eq!(ids, vec!["bud_b1", "bud_b2"]);
    }
}
