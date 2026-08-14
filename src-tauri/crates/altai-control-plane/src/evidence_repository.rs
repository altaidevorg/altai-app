//! CP-08 durable evidence storage. An [`Evidence`] record is an immutable,
//! append-only artifact reference attributed to an attempt and work item; the
//! store never edits or deletes a recorded fact. Recording is idempotent on an
//! identical replay and fails closed with [`EvidenceError::Conflict`] when a
//! divergent record already owns the id, so the audit never contradicts itself.
//! `list_for_work` reads back every evidence row for a work item — the read a
//! completion gate will use to check "does this work have evidence?". Nothing
//! here gates completion or governs delivery.

use altai_control_protocol::{Evidence, EvidenceId, WorkItemId};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use std::{path::Path, sync::Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceError {
    Conflict { evidence_id: String },
    Internal { reason: String },
}

impl std::fmt::Display for EvidenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "evidence error: {self:?}")
    }
}
impl std::error::Error for EvidenceError {}

pub trait EvidenceRepository: Send + Sync {
    /// Record an immutable evidence artifact reference. Idempotent when the
    /// same record is re-recorded; fails closed with [`EvidenceError::Conflict`]
    /// when a different record already owns the id.
    fn record(&self, evidence: Evidence) -> Result<Evidence, EvidenceError>;
    fn get(&self, id: &EvidenceId) -> Result<Option<Evidence>, EvidenceError>;
    /// Every evidence row attributed to a work item (possibly across several
    /// attempts). Ordered by `created_at_unix_seconds` then `id`.
    fn list_for_work(&self, work_item_id: &WorkItemId) -> Result<Vec<Evidence>, EvidenceError>;
}

pub struct SqliteEvidenceRepository {
    connection: Mutex<Connection>,
}

impl SqliteEvidenceRepository {
    pub fn open(path: &Path) -> Result<Self, String> {
        let connection = Connection::open(path).map_err(|e| e.to_string())?;
        connection.execute_batch(
            "PRAGMA busy_timeout = 5000; CREATE TABLE IF NOT EXISTS control_plane_evidence (evidence_id TEXT PRIMARY KEY, payload_json TEXT NOT NULL);",
        )
        .map_err(|e| e.to_string())?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, EvidenceError> {
        self.connection.lock().map_err(|_| EvidenceError::Internal {
            reason: "sqlite evidence lock poisoned".into(),
        })
    }
    fn db(e: rusqlite::Error) -> EvidenceError {
        EvidenceError::Internal { reason: e.to_string() }
    }
}

impl EvidenceRepository for SqliteEvidenceRepository {
    fn record(&self, evidence: Evidence) -> Result<Evidence, EvidenceError> {
        let payload = serde_json::to_string(&evidence)
            .map_err(|e| EvidenceError::Internal { reason: e.to_string() })?;
        let mut connection = self.lock()?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(Self::db)?;
        let inserted = tx
            .execute(
                "INSERT INTO control_plane_evidence (evidence_id, payload_json) VALUES (?1, ?2) ON CONFLICT DO NOTHING",
                params![evidence.id.value, payload],
            )
            .map_err(Self::db)?;
        if inserted == 1 {
            tx.commit().map_err(Self::db)?;
            return Ok(evidence);
        }
        let existing = Self::read_evidence(&tx, &evidence.id)?.ok_or_else(|| EvidenceError::Internal {
            reason: "evidence disappeared after insert conflict".into(),
        })?;
        if existing == evidence {
            tx.commit().map_err(Self::db)?;
            Ok(existing)
        } else {
            Err(EvidenceError::Conflict {
                evidence_id: evidence.id.value,
            })
        }
    }

    fn get(&self, id: &EvidenceId) -> Result<Option<Evidence>, EvidenceError> {
        let connection = self.lock()?;
        Self::read_evidence(&connection, id)
    }

    fn list_for_work(&self, work_item_id: &WorkItemId) -> Result<Vec<Evidence>, EvidenceError> {
        let connection = self.lock()?;
        let mut stmt = connection
            .prepare("SELECT payload_json FROM control_plane_evidence")
            .map_err(Self::db)?;
        let payloads = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(Self::db)?;
        let mut records = Vec::new();
        for payload in payloads {
            let evidence: Evidence =
                serde_json::from_str(&payload.map_err(Self::db)?).map_err(|e| EvidenceError::Internal {
                    reason: e.to_string(),
                })?;
            if evidence.work_item_id == *work_item_id {
                records.push(evidence);
            }
        }
        records.sort_by(|a, b| {
            a.created_at_unix_seconds
                .cmp(&b.created_at_unix_seconds)
                .then_with(|| a.id.value.cmp(&b.id.value))
        });
        Ok(records)
    }
}

impl SqliteEvidenceRepository {
    fn read_evidence(
        connection: &Connection,
        id: &EvidenceId,
    ) -> Result<Option<Evidence>, EvidenceError> {
        let payload: Option<String> = connection
            .query_row(
                "SELECT payload_json FROM control_plane_evidence WHERE evidence_id=?1",
                [&id.value],
                |row| row.get(0),
            )
            .optional()
            .map_err(Self::db)?;
        payload
            .map(|p| {
                serde_json::from_str(&p)
                    .map_err(|e| EvidenceError::Internal { reason: e.to_string() })
            })
            .transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use altai_control_protocol::{AttemptId, OrganizationId};

    fn evidence(id: &str, work: &str, attempt: &str, kind: &str, reference: &str) -> Evidence {
        Evidence {
            id: EvidenceId::new(id),
            organization_id: OrganizationId::new("org"),
            work_item_id: WorkItemId::new(work),
            attempt_id: AttemptId::new(attempt),
            kind: kind.into(),
            reference: reference.into(),
            created_at_unix_seconds: 10,
        }
    }

    #[test]
    fn record_is_durable_and_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let database = dir.path().join("work.db");
        let repo = SqliteEvidenceRepository::open(&database).unwrap();
        let id = EvidenceId::new("e1");
        repo.record(evidence("e1", "w1", "a1", "artifact_ref", "out/diff.patch")).unwrap();
        // Idempotent replay.
        repo.record(evidence("e1", "w1", "a1", "artifact_ref", "out/diff.patch")).unwrap();

        let reopened = SqliteEvidenceRepository::open(&database).unwrap();
        let stored = reopened.get(&id).unwrap().unwrap();
        assert_eq!(stored.kind, "artifact_ref");
        assert_eq!(stored.reference, "out/diff.patch");
        assert_eq!(stored.work_item_id, WorkItemId::new("w1"));
    }

    #[test]
    fn record_rejects_a_divergent_same_id_record() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteEvidenceRepository::open(&dir.path().join("work.db")).unwrap();
        repo.record(evidence("e1", "w1", "a1", "artifact_ref", "out/a.patch")).unwrap();
        let err = repo
            .record(evidence("e1", "w1", "a1", "artifact_ref", "out/b.patch"))
            .unwrap_err();
        assert!(matches!(err, EvidenceError::Conflict { .. }));
        // Original reference is unchanged.
        assert_eq!(
            repo.get(&EvidenceId::new("e1")).unwrap().unwrap().reference,
            "out/a.patch"
        );
    }

    #[test]
    fn get_returns_none_for_unknown() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteEvidenceRepository::open(&dir.path().join("work.db")).unwrap();
        assert!(repo.get(&EvidenceId::new("ghost")).unwrap().is_none());
    }

    #[test]
    fn list_for_work_returns_only_that_works_evidence() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteEvidenceRepository::open(&dir.path().join("work.db")).unwrap();
        // Work w1 gathers evidence from two attempts; work w2 has its own.
        repo.record(evidence("e1", "w1", "a1", "artifact_ref", "out/1.patch")).unwrap();
        repo.record(evidence("e2", "w1", "a2", "summary", "ran 3 tests")).unwrap();
        repo.record(evidence("e3", "w2", "a3", "artifact_ref", "out/2.patch")).unwrap();

        let w1 = repo.list_for_work(&WorkItemId::new("w1")).unwrap();
        let ids: Vec<&str> = w1.iter().map(|e| e.id.value.as_str()).collect();
        assert_eq!(ids, vec!["ev_e1", "ev_e2"]);
    }
}
