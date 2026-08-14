//! CP-08 durable usage/cost ledger storage. A [`UsageRecord`] is an immutable,
//! append-only meter reading; the ledger never edits or deletes a recorded
//! fact. Recording is idempotent on an identical replay and fails closed with
//! [`UsageError::Conflict`] when a divergent record already owns the id, so the
//! audit never contradicts itself. `list_in_scope` reads records back filtered
//! by attribution dimension — the read source a budget layer will sum to enforce
//! hard stops. Nothing here sets a limit or stops anything.

use altai_control_protocol::{UsageRecord, UsageRecordId, UsageScope};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use std::{path::Path, sync::Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UsageError {
    Conflict { usage_record_id: String },
    Internal { reason: String },
}

impl std::fmt::Display for UsageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "usage error: {self:?}")
    }
}
impl std::error::Error for UsageError {}

pub trait UsageRepository: Send + Sync {
    /// Record an immutable meter reading. Idempotent when the same record is
    /// re-recorded; fails closed with [`UsageError::Conflict`] when a different
    /// record already owns the id.
    fn record(&self, record: UsageRecord) -> Result<UsageRecord, UsageError>;
    fn get(&self, id: &UsageRecordId) -> Result<Option<UsageRecord>, UsageError>;
    /// Records matching every set dimension of `scope`. `organization_id` is
    /// always an equality filter (no cross-org read); a `None` dimension is a
    /// wildcard. Ordered by `recorded_at_unix_seconds` then `id`.
    fn list_in_scope(&self, scope: &UsageScope) -> Result<Vec<UsageRecord>, UsageError>;
}

pub struct SqliteUsageRepository {
    connection: Mutex<Connection>,
}

impl SqliteUsageRepository {
    pub fn open(path: &Path) -> Result<Self, String> {
        let connection = Connection::open(path).map_err(|e| e.to_string())?;
        connection.execute_batch(
            "PRAGMA busy_timeout = 5000; CREATE TABLE IF NOT EXISTS control_plane_usage_records (usage_record_id TEXT PRIMARY KEY, payload_json TEXT NOT NULL);",
        )
        .map_err(|e| e.to_string())?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, UsageError> {
        self.connection.lock().map_err(|_| UsageError::Internal {
            reason: "sqlite usage lock poisoned".into(),
        })
    }
    fn db(e: rusqlite::Error) -> UsageError {
        UsageError::Internal { reason: e.to_string() }
    }
}

impl UsageRepository for SqliteUsageRepository {
    fn record(&self, record: UsageRecord) -> Result<UsageRecord, UsageError> {
        let payload = serde_json::to_string(&record)
            .map_err(|e| UsageError::Internal { reason: e.to_string() })?;
        let mut connection = self.lock()?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(Self::db)?;
        let inserted = tx
            .execute(
                "INSERT INTO control_plane_usage_records (usage_record_id, payload_json) VALUES (?1, ?2) ON CONFLICT DO NOTHING",
                params![record.id.value, payload],
            )
            .map_err(Self::db)?;
        if inserted == 1 {
            tx.commit().map_err(Self::db)?;
            return Ok(record);
        }
        // A row already owns this id: idempotent only if byte-identical.
        let existing = Self::read_record(&tx, &record.id)?.ok_or_else(|| UsageError::Internal {
            reason: "usage record disappeared after insert conflict".into(),
        })?;
        if existing == record {
            tx.commit().map_err(Self::db)?;
            Ok(existing)
        } else {
            Err(UsageError::Conflict {
                usage_record_id: record.id.value,
            })
        }
    }

    fn get(&self, id: &UsageRecordId) -> Result<Option<UsageRecord>, UsageError> {
        let connection = self.lock()?;
        Self::read_record(&connection, id)
    }

    fn list_in_scope(&self, scope: &UsageScope) -> Result<Vec<UsageRecord>, UsageError> {
        let connection = self.lock()?;
        let mut stmt = connection
            .prepare("SELECT payload_json FROM control_plane_usage_records")
            .map_err(Self::db)?;
        let payloads = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(Self::db)?;
        let mut records = Vec::new();
        for payload in payloads {
            let record: UsageRecord =
                serde_json::from_str(&payload.map_err(Self::db)?).map_err(|e| UsageError::Internal {
                    reason: e.to_string(),
                })?;
            if matches_scope(&record.scope, scope) {
                records.push(record);
            }
        }
        records.sort_by(|a, b| {
            a.recorded_at_unix_seconds
                .cmp(&b.recorded_at_unix_seconds)
                .then_with(|| a.id.value.cmp(&b.id.value))
        });
        Ok(records)
    }
}

impl SqliteUsageRepository {
    fn read_record(
        connection: &Connection,
        id: &UsageRecordId,
    ) -> Result<Option<UsageRecord>, UsageError> {
        let payload: Option<String> = connection
            .query_row(
                "SELECT payload_json FROM control_plane_usage_records WHERE usage_record_id=?1",
                [&id.value],
                |row| row.get(0),
            )
            .optional()
            .map_err(Self::db)?;
        payload
            .map(|p| {
                serde_json::from_str(&p).map_err(|e| UsageError::Internal { reason: e.to_string() })
            })
            .transpose()
    }
}

/// A record's scope matches a filter when the org matches and every `Some`
/// filter dimension equals the record's dimension; a `None` filter dimension is
/// a wildcard. Shared with the budget enforcer to decide whether a (broader)
/// budget scope governs a (narrower) consumption scope.
pub(crate) fn matches_scope(record: &UsageScope, filter: &UsageScope) -> bool {
    record.organization_id == filter.organization_id
        && filter
            .project_id
            .as_ref()
            .is_none_or(|p| record.project_id.as_ref() == Some(p))
        && filter
            .agent_instance_id
            .as_ref()
            .is_none_or(|a| record.agent_instance_id.as_ref() == Some(a))
        && filter
            .work_item_id
            .as_ref()
            .is_none_or(|w| record.work_item_id.as_ref() == Some(w))
        && filter
            .attempt_id
            .as_ref()
            .is_none_or(|a| record.attempt_id.as_ref() == Some(a))
}

#[cfg(test)]
mod tests {
    use super::*;
    use altai_control_protocol::{
        AgentInstanceId, AttemptId, OrganizationId, ProjectId, WorkItemId,
    };

    fn scope(org: &str) -> UsageScope {
        UsageScope {
            organization_id: OrganizationId::new(org),
            project_id: None,
            agent_instance_id: None,
            work_item_id: None,
            attempt_id: None,
        }
    }

    fn record(id: &str, scope: UsageScope, meter: &str, amount: u64, at: u64) -> UsageRecord {
        UsageRecord {
            id: UsageRecordId::new(id),
            scope,
            meter: meter.into(),
            amount,
            recorded_at_unix_seconds: at,
        }
    }

    #[test]
    fn record_is_durable_across_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let database = dir.path().join("work.db");
        let repo = SqliteUsageRepository::open(&database).unwrap();
        let id = UsageRecordId::new("u1");
        repo.record(record("u1", scope("org"), "input_tokens", 1200, 10)).unwrap();

        let reopened = SqliteUsageRepository::open(&database).unwrap();
        let stored = reopened.get(&id).unwrap().unwrap();
        assert_eq!(stored.amount, 1200);
        assert_eq!(stored.meter, "input_tokens");
        assert_eq!(stored.scope.organization_id, OrganizationId::new("org"));
    }

    #[test]
    fn record_is_idempotent_on_identical_replay() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteUsageRepository::open(&dir.path().join("work.db")).unwrap();
        repo.record(record("u1", scope("org"), "input_tokens", 1200, 10)).unwrap();
        // Replaying the same record succeeds without error.
        repo.record(record("u1", scope("org"), "input_tokens", 1200, 10)).unwrap();
    }

    #[test]
    fn record_rejects_a_divergent_same_id_record() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteUsageRepository::open(&dir.path().join("work.db")).unwrap();
        repo.record(record("u1", scope("org"), "input_tokens", 1200, 10)).unwrap();
        // A different amount under the same id fails closed.
        let err = repo
            .record(record("u1", scope("org"), "input_tokens", 9999, 10))
            .unwrap_err();
        assert!(matches!(err, UsageError::Conflict { .. }));
        // The original fact is unchanged.
        let stored = repo.get(&UsageRecordId::new("u1")).unwrap().unwrap();
        assert_eq!(stored.amount, 1200);
    }

    #[test]
    fn get_returns_none_for_unknown() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteUsageRepository::open(&dir.path().join("work.db")).unwrap();
        assert!(repo.get(&UsageRecordId::new("ghost")).unwrap().is_none());
    }

    #[test]
    fn list_in_scope_matches_set_dims_and_isolates_org() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteUsageRepository::open(&dir.path().join("work.db")).unwrap();
        let mut proj_scope = scope("org");
        proj_scope.project_id = Some(ProjectId::new("proj"));

        // Two attempts and a second work item within project `proj` of org `org`.
        let mut attempt_a = proj_scope.clone();
        attempt_a.attempt_id = Some(AttemptId::new("att-a"));
        let mut attempt_b = proj_scope.clone();
        attempt_b.attempt_id = Some(AttemptId::new("att-b"));
        let mut work_c = proj_scope.clone();
        work_c.work_item_id = Some(WorkItemId::new("wi-c"));
        let mut agent_d = proj_scope.clone();
        agent_d.agent_instance_id = Some(AgentInstanceId::new("ai-d"));
        // A different org's record must never appear.
        let other_org = scope("other");

        repo.record(record("a", attempt_a.clone(), "input_tokens", 10, 1)).unwrap();
        repo.record(record("b", attempt_b.clone(), "input_tokens", 20, 2)).unwrap();
        repo.record(record("c", work_c.clone(), "input_tokens", 30, 3)).unwrap();
        repo.record(record("d", agent_d.clone(), "input_tokens", 40, 4)).unwrap();
        repo.record(record("e", other_org, "input_tokens", 50, 5)).unwrap();

        // Filter by org+project: every org/proj record, regardless of attempt/agent/work.
        let by_project = repo.list_in_scope(&proj_scope).unwrap();
        let ids: Vec<&str> = by_project.iter().map(|r| r.id.value.as_str()).collect();
        assert_eq!(ids, vec!["usage_a", "usage_b", "usage_c", "usage_d"]);

        // Narrow to attempt-a only.
        let mut only_a = proj_scope.clone();
        only_a.attempt_id = Some(AttemptId::new("att-a"));
        let by_attempt = repo.list_in_scope(&only_a).unwrap();
        let ids: Vec<&str> = by_attempt.iter().map(|r| r.id.value.as_str()).collect();
        assert_eq!(ids, vec!["usage_a"]);

        // Different org sees nothing from `org`.
        let other = repo.list_in_scope(&scope("other")).unwrap();
        let ids: Vec<&str> = other.iter().map(|r| r.id.value.as_str()).collect();
        assert_eq!(ids, vec!["usage_e"]);
    }
}
