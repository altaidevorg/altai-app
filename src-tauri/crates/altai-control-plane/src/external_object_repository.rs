//! CP-08 durable ExternalObject storage (package 070, PR 1). One row per
//! provider object, keyed by `(integration, external_id)` so a repeated sync
//! of the same provider payload is a no-op: equal `content_hash` never
//! writes. A changed payload is applied or refused by the object's recorded
//! authority — `External` applies the provider's version, `Local` refuses
//! with a conflict the caller must resolve explicitly. Write order is never
//! the resolution rule.

use altai_control_protocol::{
    ExternalAuthority, ExternalObject, ExternalObjectId, OrganizationId, WorkItemId,
};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use std::{path::Path, sync::Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExternalObjectError {
    NotFound { external_object_id: String },
    /// The object exists but is not in a state that resolution applies to.
    InvalidResolution {
        external_object_id: String,
        reason: String,
    },
    Internal { reason: String },
}

impl std::fmt::Display for ExternalObjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "external object error: {self:?}")
    }
}
impl std::error::Error for ExternalObjectError {}

/// What an upsert did. `Unchanged` is the idempotent no-op; `Conflict` is a
/// refused overwrite of locally-authoritative content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExternalSyncOutcome {
    Inserted,
    Unchanged,
    Updated,
    Conflict {
        external_object_id: ExternalObjectId,
        stored_content_hash: String,
        incoming_content_hash: String,
    },
}

/// An explicit decision on a refused overwrite. Neither direction rewrites
/// content here: `TakeExternal` flips authority so the next sync applies the
/// provider's version through the normal update path, and `KeepLocal`
/// dismisses exactly the refused external content so an identical payload
/// stops re-conflicting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictResolution {
    TakeExternal,
    KeepLocal,
}

pub trait ExternalObjectRepository: Send + Sync {
    /// Idempotent per `(integration, external_id)` + `content_hash`. A new
    /// key inserts; an equal hash is `Unchanged`; a changed hash applies the
    /// incoming object when authority is `External` and refuses with
    /// [`ExternalSyncOutcome::Conflict`] when it is `Local`.
    fn upsert(&self, object: ExternalObject) -> Result<ExternalSyncOutcome, ExternalObjectError>;
    fn get(&self, id: &ExternalObjectId) -> Result<Option<ExternalObject>, ExternalObjectError>;
    /// The object a provider identity maps to, if it has been synced.
    fn find(
        &self,
        integration: &str,
        external_id: &str,
    ) -> Result<Option<ExternalObject>, ExternalObjectError>;
    /// Every synced object for one integration, oldest sync first.
    fn list_by_integration(
        &self,
        organization_id: &OrganizationId,
        integration: &str,
    ) -> Result<Vec<ExternalObject>, ExternalObjectError>;
    /// Link or unlink the local Work item. Explicit state, not a side effect
    /// of sync: an unlinked object stays synced, it just stops driving Work.
    fn link_work_item(
        &self,
        id: &ExternalObjectId,
        work_item_id: Option<WorkItemId>,
    ) -> Result<(), ExternalObjectError>;
    /// Apply an explicit decision to a refused overwrite. Returns the
    /// object as it stands after the decision.
    fn resolve_conflict(
        &self,
        id: &ExternalObjectId,
        resolution: ConflictResolution,
    ) -> Result<ExternalObject, ExternalObjectError>;
}

pub struct SqliteExternalObjectRepository {
    connection: Mutex<Connection>,
}

impl SqliteExternalObjectRepository {
    pub fn open(path: &Path) -> Result<Self, String> {
        let connection = Connection::open(path).map_err(|e| e.to_string())?;
        connection
            .execute_batch(
                "PRAGMA busy_timeout = 5000;
                 CREATE TABLE IF NOT EXISTS control_plane_external_objects (
                   external_object_id TEXT PRIMARY KEY,
                   integration TEXT NOT NULL,
                   external_id TEXT NOT NULL,
                   last_synced_at INTEGER NOT NULL,
                   payload_json TEXT NOT NULL,
                   UNIQUE(integration, external_id)
                 );
                 CREATE INDEX IF NOT EXISTS control_plane_external_objects_org_synced
                   ON control_plane_external_objects (integration, last_synced_at);",
            )
            .map_err(|e| e.to_string())?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    fn lock(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, Connection>, ExternalObjectError> {
        self.connection.lock().map_err(|_| ExternalObjectError::Internal {
            reason: "sqlite external object lock poisoned".into(),
        })
    }

    fn db(e: rusqlite::Error) -> ExternalObjectError {
        ExternalObjectError::Internal { reason: e.to_string() }
    }

    fn decode(payload: String) -> Result<ExternalObject, ExternalObjectError> {
        serde_json::from_str(&payload).map_err(|e| ExternalObjectError::Internal {
            reason: format!("external object payload decode failed: {e}"),
        })
    }
}

impl ExternalObjectRepository for SqliteExternalObjectRepository {
    fn upsert(
        &self,
        object: ExternalObject,
    ) -> Result<ExternalSyncOutcome, ExternalObjectError> {
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(Self::db)?;
        let stored: Option<String> = transaction
            .query_row(
                "SELECT payload_json FROM control_plane_external_objects
                 WHERE integration = ?1 AND external_id = ?2",
                params![object.integration, object.external_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(Self::db)?;
        let payload =
            serde_json::to_string(&object).map_err(|e| ExternalObjectError::Internal {
                reason: e.to_string(),
            })?;

        let outcome = match stored {
            None => {
                transaction
                    .execute(
                        "INSERT INTO control_plane_external_objects
                         (external_object_id, integration, external_id, last_synced_at, payload_json)
                         VALUES (?1, ?2, ?3, ?4, ?5)",
                        params![
                            object.id.value,
                            object.integration,
                            object.external_id,
                            object.last_synced_at_unix_seconds as i64,
                            payload
                        ],
                    )
                    .map_err(Self::db)?;
                ExternalSyncOutcome::Inserted
            }
            Some(stored) => {
                let mut existing = Self::decode(stored)?;
                if existing.content_hash == object.content_hash {
                    ExternalSyncOutcome::Unchanged
                } else if existing.authority == ExternalAuthority::External {
                    // The stored id stays: a provider object keeps one local
                    // identity across syncs even if the caller minted a new
                    // ExternalObjectId for the same (integration, external_id).
                    transaction
                        .execute(
                            "UPDATE control_plane_external_objects
                             SET payload_json = ?2, last_synced_at = ?3
                             WHERE external_object_id = ?1",
                            params![
                                existing.id.value,
                                payload,
                                object.last_synced_at_unix_seconds as i64
                            ],
                        )
                        .map_err(Self::db)?;
                    ExternalSyncOutcome::Updated
                } else if existing.declined_content_hash.as_deref()
                    == Some(object.content_hash.as_str())
                {
                    // A `KeepLocal` resolution already dismissed exactly this
                    // external content; re-reporting it would contradict the
                    // decision. Nothing is written.
                    ExternalSyncOutcome::Unchanged
                } else {
                    // Record what was refused so a resolver can present the
                    // divergence without re-deriving it; local content and
                    // the sync ordering stay untouched.
                    existing.refused_content_hash = Some(object.content_hash.clone());
                    let refused = serde_json::to_string(&existing).map_err(|e| {
                        ExternalObjectError::Internal { reason: e.to_string() }
                    })?;
                    transaction
                        .execute(
                            "UPDATE control_plane_external_objects SET payload_json = ?2
                             WHERE external_object_id = ?1",
                            params![existing.id.value, refused],
                        )
                        .map_err(Self::db)?;
                    ExternalSyncOutcome::Conflict {
                        external_object_id: existing.id,
                        stored_content_hash: existing.content_hash,
                        incoming_content_hash: object.content_hash,
                    }
                }
            }
        };
        transaction.commit().map_err(Self::db)?;
        Ok(outcome)
    }

    fn get(&self, id: &ExternalObjectId) -> Result<Option<ExternalObject>, ExternalObjectError> {
        let connection = self.lock()?;
        let payload: Option<String> = connection
            .query_row(
                "SELECT payload_json FROM control_plane_external_objects
                 WHERE external_object_id = ?1",
                params![id.value],
                |row| row.get(0),
            )
            .optional()
            .map_err(Self::db)?;
        payload.map(Self::decode).transpose()
    }

    fn find(
        &self,
        integration: &str,
        external_id: &str,
    ) -> Result<Option<ExternalObject>, ExternalObjectError> {
        let connection = self.lock()?;
        let payload: Option<String> = connection
            .query_row(
                "SELECT payload_json FROM control_plane_external_objects
                 WHERE integration = ?1 AND external_id = ?2",
                params![integration, external_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(Self::db)?;
        payload.map(Self::decode).transpose()
    }

    fn list_by_integration(
        &self,
        organization_id: &OrganizationId,
        integration: &str,
    ) -> Result<Vec<ExternalObject>, ExternalObjectError> {
        let connection = self.lock()?;
        let mut statement = connection
            .prepare(
                "SELECT payload_json FROM control_plane_external_objects
                 WHERE integration = ?1 ORDER BY last_synced_at, external_object_id",
            )
            .map_err(Self::db)?;
        // Payloads are filtered after decode: organization is a field of the
        // object, not a column, so the index keeps the scan narrow and the
        // check stays exact.
        let mut rows = statement.query(params![integration]).map_err(Self::db)?;
        let mut objects = Vec::new();
        while let Some(row) = rows.next().map_err(Self::db)? {
            let object = Self::decode(row.get(0).map_err(Self::db)?)?;
            if object.organization_id == *organization_id {
                objects.push(object);
            }
        }
        Ok(objects)
    }

    fn link_work_item(
        &self,
        id: &ExternalObjectId,
        work_item_id: Option<WorkItemId>,
    ) -> Result<(), ExternalObjectError> {
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(Self::db)?;
        let payload: Option<String> = transaction
            .query_row(
                "SELECT payload_json FROM control_plane_external_objects
                 WHERE external_object_id = ?1",
                params![id.value],
                |row| row.get(0),
            )
            .optional()
            .map_err(Self::db)?;
        let Some(payload) = payload else {
            return Err(ExternalObjectError::NotFound {
                external_object_id: id.value.clone(),
            });
        };
        let mut object = Self::decode(payload)?;
        object.linked_work_item_id = work_item_id;
        let updated =
            serde_json::to_string(&object).map_err(|e| ExternalObjectError::Internal {
                reason: e.to_string(),
            })?;
        transaction
            .execute(
                "UPDATE control_plane_external_objects SET payload_json = ?2
                 WHERE external_object_id = ?1",
                params![id.value, updated],
            )
            .map_err(Self::db)?;
        transaction.commit().map_err(Self::db)?;
        Ok(())
    }

    fn resolve_conflict(
        &self,
        id: &ExternalObjectId,
        resolution: ConflictResolution,
    ) -> Result<ExternalObject, ExternalObjectError> {
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(Self::db)?;
        let payload: Option<String> = transaction
            .query_row(
                "SELECT payload_json FROM control_plane_external_objects
                 WHERE external_object_id = ?1",
                params![id.value],
                |row| row.get(0),
            )
            .optional()
            .map_err(Self::db)?;
        let Some(payload) = payload else {
            return Err(ExternalObjectError::NotFound {
                external_object_id: id.value.clone(),
            });
        };
        let mut object = Self::decode(payload)?;
        let invalid = |reason: &str| ExternalObjectError::InvalidResolution {
            external_object_id: id.value.clone(),
            reason: reason.to_string(),
        };
        match resolution {
            ConflictResolution::TakeExternal => {
                if object.authority != ExternalAuthority::Local {
                    return Err(invalid(
                        "authority is already external; there is no local version to override",
                    ));
                }
                object.authority = ExternalAuthority::External;
                object.refused_content_hash = None;
                object.declined_content_hash = None;
            }
            ConflictResolution::KeepLocal => {
                if object.authority != ExternalAuthority::Local {
                    return Err(invalid(
                        "authority is external; the provider version already applies",
                    ));
                }
                let Some(refused) = object.refused_content_hash.clone() else {
                    return Err(invalid("no refused external content to keep against"));
                };
                object.declined_content_hash = Some(refused);
                object.refused_content_hash = None;
            }
        }
        let updated =
            serde_json::to_string(&object).map_err(|e| ExternalObjectError::Internal {
                reason: e.to_string(),
            })?;
        transaction
            .execute(
                "UPDATE control_plane_external_objects SET payload_json = ?2
                 WHERE external_object_id = ?1",
                params![id.value, updated],
            )
            .map_err(Self::db)?;
        transaction.commit().map_err(Self::db)?;
        Ok(object)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn object(id: &str, external_id: &str, content_hash: &str) -> ExternalObject {
        ExternalObject {
            id: ExternalObjectId::new(id),
            organization_id: OrganizationId::new("org_1"),
            integration: "github".into(),
            external_id: external_id.into(),
            object_kind: "issue".into(),
            url: Some("https://example.invalid/issue".into()),
            title: "Sync me".into(),
            content_hash: content_hash.into(),
            authority: ExternalAuthority::External,
            refused_content_hash: None,
            declined_content_hash: None,
            linked_work_item_id: Some(WorkItemId::new("work_1")),
            external_updated_at_unix_seconds: Some(1_000),
            last_synced_at_unix_seconds: 2_000,
            created_at_unix_seconds: 1_000,
            updated_at_unix_seconds: 2_000,
        }
    }

    /// The TempDir must outlive the repository: returning it keeps the
    /// database writable for the whole test.
    fn repository() -> (SqliteExternalObjectRepository, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let database = dir.path().join("work.db");
        let repository = SqliteExternalObjectRepository::open(&database).unwrap();
        (repository, dir)
    }

    #[test]
    fn inserting_then_replaying_the_same_payload_is_unchanged() {
        let (repository, _dir) = repository();
        let first = object("ext_1", "node_1", "hash_a");
        assert_eq!(
            repository.upsert(first.clone()).unwrap(),
            ExternalSyncOutcome::Inserted
        );
        assert_eq!(
            repository.upsert(first).unwrap(),
            ExternalSyncOutcome::Unchanged
        );
    }

    #[test]
    fn a_changed_payload_applies_when_the_provider_is_authoritative() {
        let (repository, _dir) = repository();
        repository
            .upsert(object("ext_1", "node_1", "hash_a"))
            .unwrap();
        let mut changed = object("ext_1", "node_1", "hash_b");
        changed.title = "Renamed upstream".into();
        assert_eq!(
            repository.upsert(changed.clone()).unwrap(),
            ExternalSyncOutcome::Updated
        );
        assert_eq!(repository.get(&changed.id).unwrap().unwrap().title, "Renamed upstream");
    }

    #[test]
    fn a_changed_payload_is_refused_when_the_local_side_is_authoritative() {
        let (repository, _dir) = repository();
        let mut local = object("ext_1", "node_1", "hash_a");
        local.authority = ExternalAuthority::Local;
        repository.upsert(local).unwrap();
        let outcome = repository
            .upsert(object("ext_1", "node_1", "hash_b"))
            .unwrap();
        assert_eq!(
            outcome,
            ExternalSyncOutcome::Conflict {
                external_object_id: ExternalObjectId::new("ext_1"),
                stored_content_hash: "hash_a".into(),
                incoming_content_hash: "hash_b".into(),
            }
        );
        // The refusal left the stored content untouched.
        assert_eq!(
            repository
                .find("github", "node_1")
                .unwrap()
                .unwrap()
                .content_hash,
            "hash_a"
        );
    }

    #[test]
    fn the_same_external_id_under_another_integration_is_a_distinct_object() {
        let (repository, _dir) = repository();
        repository
            .upsert(object("ext_1", "node_1", "hash_a"))
            .unwrap();
        let mut other = object("ext_2", "node_1", "hash_a");
        other.integration = "gmail".into();
        assert_eq!(
            repository.upsert(other).unwrap(),
            ExternalSyncOutcome::Inserted
        );
        assert_eq!(repository.list_by_integration(&OrganizationId::new("org_1"), "github").unwrap().len(), 1);
        assert_eq!(repository.list_by_integration(&OrganizationId::new("org_1"), "gmail").unwrap().len(), 1);
    }

    #[test]
    fn list_by_integration_is_scoped_to_the_organization() {
        let (repository, _dir) = repository();
        repository
            .upsert(object("ext_1", "node_1", "hash_a"))
            .unwrap();
        let mut foreign = object("ext_2", "node_2", "hash_a");
        foreign.organization_id = OrganizationId::new("org_2");
        repository.upsert(foreign).unwrap();
        let listed =
            repository.list_by_integration(&OrganizationId::new("org_1"), "github").unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, ExternalObjectId::new("ext_1"));
    }

    #[test]
    fn linking_and_unlinking_is_explicit_state() {
        let (repository, _dir) = repository();
        let first = object("ext_1", "node_1", "hash_a");
        repository.upsert(first.clone()).unwrap();
        assert_eq!(
            repository.get(&first.id).unwrap().unwrap().linked_work_item_id,
            Some(WorkItemId::new("work_1"))
        );
        repository
            .link_work_item(&first.id, None)
            .unwrap();
        assert_eq!(
            repository.get(&first.id).unwrap().unwrap().linked_work_item_id,
            None
        );
        let missing = ExternalObjectId::new("ext_missing");
        assert!(matches!(
            repository.link_work_item(&missing, None),
            Err(ExternalObjectError::NotFound { .. })
        ));
    }

    fn local_object(id: &str, external_id: &str, content_hash: &str) -> ExternalObject {
        let mut object = object(id, external_id, content_hash);
        object.authority = ExternalAuthority::Local;
        object
    }

    #[test]
    fn a_refusal_records_the_refused_hash_without_touching_local_content() {
        let (repository, _dir) = repository();
        repository
            .upsert(local_object("ext_1", "node_1", "hash_a"))
            .unwrap();
        assert_eq!(
            repository.upsert(object("ext_1", "node_1", "hash_b")).unwrap(),
            ExternalSyncOutcome::Conflict {
                external_object_id: ExternalObjectId::new("ext_1"),
                stored_content_hash: "hash_a".into(),
                incoming_content_hash: "hash_b".into(),
            }
        );
        let stored = repository
            .find("github", "node_1")
            .unwrap()
            .unwrap();
        assert_eq!(stored.content_hash, "hash_a");
        assert_eq!(stored.refused_content_hash.as_deref(), Some("hash_b"));
        assert_eq!(stored.declined_content_hash, None);
    }

    #[test]
    fn keep_local_dismisses_exactly_the_refused_content() {
        let (repository, _dir) = repository();
        let local = local_object("ext_1", "node_1", "hash_a");
        repository.upsert(local.clone()).unwrap();
        repository
            .upsert(object("ext_1", "node_1", "hash_b"))
            .unwrap();

        let resolved = repository
            .resolve_conflict(&local.id, ConflictResolution::KeepLocal)
            .unwrap();
        assert_eq!(resolved.authority, ExternalAuthority::Local);
        assert_eq!(resolved.declined_content_hash.as_deref(), Some("hash_b"));
        assert_eq!(resolved.refused_content_hash, None);

        // The dismissed external content stops re-conflicting…
        assert_eq!(
            repository.upsert(object("ext_1", "node_1", "hash_b")).unwrap(),
            ExternalSyncOutcome::Unchanged
        );
        // …but a new external version still surfaces.
        assert!(matches!(
            repository.upsert(object("ext_1", "node_1", "hash_c")).unwrap(),
            ExternalSyncOutcome::Conflict { .. }
        ));
    }

    #[test]
    fn take_external_flips_authority_so_the_next_sync_applies() {
        let (repository, _dir) = repository();
        let local = local_object("ext_1", "node_1", "hash_a");
        repository.upsert(local.clone()).unwrap();
        repository
            .upsert(object("ext_1", "node_1", "hash_b"))
            .unwrap();

        let resolved = repository
            .resolve_conflict(&local.id, ConflictResolution::TakeExternal)
            .unwrap();
        assert_eq!(resolved.authority, ExternalAuthority::External);
        assert_eq!(resolved.refused_content_hash, None);
        assert_eq!(resolved.declined_content_hash, None);

        assert_eq!(
            repository.upsert(object("ext_1", "node_1", "hash_b")).unwrap(),
            ExternalSyncOutcome::Updated
        );
        assert_eq!(
            repository
                .find("github", "node_1")
                .unwrap()
                .unwrap()
                .content_hash,
            "hash_b"
        );
    }

    #[test]
    fn resolution_rules_reject_states_that_have_no_conflict() {
        let (repository, _dir) = repository();
        let local = local_object("ext_1", "node_1", "hash_a");
        repository.upsert(local.clone()).unwrap();

        // Nothing was refused yet, so there is nothing to keep.
        assert!(matches!(
            repository.resolve_conflict(&local.id, ConflictResolution::KeepLocal),
            Err(ExternalObjectError::InvalidResolution { .. })
        ));
        // External authority has no local version to override.
        repository
            .upsert(object("ext_2", "node_2", "hash_a"))
            .unwrap();
        assert!(matches!(
            repository.resolve_conflict(
                &ExternalObjectId::new("ext_2"),
                ConflictResolution::TakeExternal
            ),
            Err(ExternalObjectError::InvalidResolution { .. })
        ));
        let missing = ExternalObjectId::new("ext_missing");
        assert!(matches!(
            repository.resolve_conflict(&missing, ConflictResolution::TakeExternal),
            Err(ExternalObjectError::NotFound { .. })
        ));
    }
}
