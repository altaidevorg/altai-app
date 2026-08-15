//! Explicit, one-way bridge from the existing local WorkStore to canonical
//! WorkItem projections.
//!
//! The legacy UI ledger remains its own lifecycle writer during migration.
//! This bridge creates a canonical projection once and fails closed if the
//! source revision subsequently changes; a later reconciliation package must
//! make that ownership transfer explicit.

use crate::{WorkItemRepository, WorkItemRepositoryError};
use altai_control_protocol::{
    ExecutionPhase, ProjectId, Revision, WorkItem, WorkItemId, WorkItemKind, WorkStatus,
};
use altai_core::{WorkItemKind as LegacyWorkItemKind, WorkItemRecord, WorkState};
use rusqlite::{params, Connection, OptionalExtension};
use std::{path::Path, sync::Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LegacyWorkBridgeError {
    SourceRevisionChanged { legacy_work_id: String },
    CanonicalConflict { legacy_work_id: String },
    Repository(String),
    Internal(String),
}
impl std::fmt::Display for LegacyWorkBridgeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "legacy work bridge error: {self:?}")
    }
}
impl std::error::Error for LegacyWorkBridgeError {}

/// Owns only the migration mapping table, not either Work lifecycle.
pub struct LegacyWorkBridge {
    connection: Mutex<Connection>,
}

impl LegacyWorkBridge {
    pub fn open(path: &Path) -> Result<Self, String> {
        let connection = Connection::open(path).map_err(|error| error.to_string())?;
        connection
            .execute_batch(
                "PRAGMA busy_timeout = 5000;
                 CREATE TABLE IF NOT EXISTS control_plane_legacy_work_mappings (
                    legacy_work_id TEXT PRIMARY KEY,
                    canonical_work_item_id TEXT NOT NULL UNIQUE,
                    source_revision INTEGER NOT NULL
                 );",
            )
            .map_err(|error| error.to_string())?;
        match connection.execute_batch(
            "ALTER TABLE control_plane_legacy_work_mappings
             ADD COLUMN canonical_revision INTEGER NOT NULL DEFAULT 0;",
        ) {
            Ok(()) => {}
            Err(error) if error.to_string().contains("duplicate column name") => {}
            Err(error) => return Err(error.to_string()),
        }
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    /// Project one legacy work record into a selected canonical project.
    /// Repeating the exact source revision is idempotent; changed source state
    /// requires an explicit reconciliation instead of a hidden update.
    pub fn project(
        &self,
        repository: &dyn WorkItemRepository,
        canonical_project_id: &ProjectId,
        legacy: &WorkItemRecord,
    ) -> Result<WorkItem, LegacyWorkBridgeError> {
        let canonical = project_record(canonical_project_id, legacy);
        let source_revision = legacy.revision;
        let connection = self.connection.lock().map_err(|_| {
            LegacyWorkBridgeError::Internal("legacy bridge lock poisoned".into())
        })?;
        let mapping: Option<(String, i64, i64)> = connection
            .query_row(
                "SELECT canonical_work_item_id, source_revision, canonical_revision
                 FROM control_plane_legacy_work_mappings WHERE legacy_work_id = ?1",
                [&legacy.id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(|error| LegacyWorkBridgeError::Internal(error.to_string()))?;
        if let Some((canonical_id, mapped_revision, _)) = mapping {
            if canonical_id != canonical.id.value {
                return Err(LegacyWorkBridgeError::CanonicalConflict {
                    legacy_work_id: legacy.id.clone(),
                });
            }
            if mapped_revision != source_revision {
                return Err(LegacyWorkBridgeError::SourceRevisionChanged {
                    legacy_work_id: legacy.id.clone(),
                });
            }
            let stored = repository.get(&canonical.id).map_err(repository_error)?;
            return if stored == canonical {
                Ok(stored)
            } else {
                Err(LegacyWorkBridgeError::CanonicalConflict {
                    legacy_work_id: legacy.id.clone(),
                })
            };
        }

        match repository.create(canonical.clone()) {
            Ok(()) => {}
            Err(WorkItemRepositoryError::AlreadyExists { .. }) => {
                let stored = repository.get(&canonical.id).map_err(repository_error)?;
                if stored != canonical {
                    return Err(LegacyWorkBridgeError::CanonicalConflict {
                        legacy_work_id: legacy.id.clone(),
                    });
                }
            }
            Err(error) => return Err(repository_error(error)),
        }
        connection
            .execute(
                "INSERT INTO control_plane_legacy_work_mappings
                 (legacy_work_id, canonical_work_item_id, source_revision, canonical_revision) VALUES (?1, ?2, ?3, ?4)",
                params![legacy.id, canonical.id.value, source_revision, canonical.revision.0 as i64],
            )
            .map_err(|error| LegacyWorkBridgeError::Internal(error.to_string()))?;
        Ok(canonical)
    }

    /// Reverse lookup for read-side projections: the legacy WorkStore id a
    /// canonical WorkItem was projected from, if the bridge ever projected
    /// it. `None` means the canonical id has no legacy counterpart (yet).
    pub fn legacy_id_for(
        &self,
        canonical_work_item_id: &WorkItemId,
    ) -> Result<Option<String>, LegacyWorkBridgeError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| LegacyWorkBridgeError::Internal("legacy bridge lock poisoned".into()))?;
        connection
            .query_row(
                "SELECT legacy_work_id FROM control_plane_legacy_work_mappings
                 WHERE canonical_work_item_id = ?1",
                [&canonical_work_item_id.value],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| LegacyWorkBridgeError::Internal(error.to_string()))
    }

    /// Advance a projection only if no independent canonical writer changed it
    /// since this bridge last recorded ownership.
    pub fn reconcile(
        &self,
        repository: &dyn WorkItemRepository,
        canonical_project_id: &ProjectId,
        legacy: &WorkItemRecord,
    ) -> Result<WorkItem, LegacyWorkBridgeError> {
        let next = project_record(canonical_project_id, legacy);
        let mapping: Option<(String, i64, i64)> = {
            let connection = self.connection.lock().map_err(|_| LegacyWorkBridgeError::Internal("legacy bridge lock poisoned".into()))?;
            connection.query_row(
                "SELECT canonical_work_item_id, source_revision, canonical_revision FROM control_plane_legacy_work_mappings WHERE legacy_work_id = ?1",
                [&legacy.id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            ).optional().map_err(|error| LegacyWorkBridgeError::Internal(error.to_string()))?
        };
        let Some((canonical_id, source_revision, canonical_revision)) = mapping else {
            return self.project(repository, canonical_project_id, legacy);
        };
        if canonical_id != next.id.value { return Err(LegacyWorkBridgeError::CanonicalConflict { legacy_work_id: legacy.id.clone() }); }
        if source_revision == legacy.revision { return self.project(repository, canonical_project_id, legacy); }
        let expected = Revision::new(canonical_revision as u64);
        let updated = repository.replace_if_revision(next, expected).map_err(repository_error)?;
        let connection = self.connection.lock().map_err(|_| LegacyWorkBridgeError::Internal("legacy bridge lock poisoned".into()))?;
        connection.execute(
            "UPDATE control_plane_legacy_work_mappings SET source_revision = ?2, canonical_revision = ?3 WHERE legacy_work_id = ?1",
            params![legacy.id, legacy.revision, updated.revision.0 as i64],
        ).map_err(|error| LegacyWorkBridgeError::Internal(error.to_string()))?;
        Ok(updated)
    }
}

fn repository_error(error: WorkItemRepositoryError) -> LegacyWorkBridgeError {
    LegacyWorkBridgeError::Repository(error.to_string())
}

fn project_record(project_id: &ProjectId, legacy: &WorkItemRecord) -> WorkItem {
    WorkItem {
        id: WorkItemId::new(legacy.id.clone()),
        project_id: project_id.clone(),
        goal_id: None,
        parent_work_item_id: legacy.parent_work_id.clone().map(WorkItemId::new),
        kind: match legacy.kind {
            LegacyWorkItemKind::Task => WorkItemKind::Task,
            LegacyWorkItemKind::Ticket => WorkItemKind::Ticket,
            LegacyWorkItemKind::Campaign => WorkItemKind::Campaign,
        },
        title: legacy.title.clone(),
        description: legacy.description.clone(),
        status: match legacy.state {
            WorkState::Backlog => WorkStatus::Backlog,
            WorkState::Ready => WorkStatus::Todo,
            WorkState::InProgress => WorkStatus::InProgress,
            WorkState::InReview => WorkStatus::InReview,
            WorkState::Done => WorkStatus::Done,
            WorkState::Cancelled => WorkStatus::Cancelled,
        },
        execution_phase: match legacy.state {
            WorkState::Backlog => ExecutionPhase::None,
            WorkState::Ready => ExecutionPhase::Queued,
            WorkState::InProgress => ExecutionPhase::Running,
            WorkState::InReview => ExecutionPhase::Reviewing,
            WorkState::Done | WorkState::Cancelled => ExecutionPhase::Terminal,
        },
        revision: Revision::new(legacy.revision as u64),
        created_at: format!("legacy-ms:{}", legacy.created_at_ms),
        updated_at: format!("legacy-ms:{}", legacy.updated_at_ms),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ScopeRepository, SqliteScopeRepository, SqliteWorkItemRepository};
    use altai_control_protocol::{Organization, OrganizationId, Project, ProjectStatus};
    use altai_core::{CreateWorkInput, WorkStore};

    #[test]
    fn projects_once_and_rejects_a_changed_legacy_revision() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("work.db");
        let project_id = ProjectId::new("canonical");
        let scopes = SqliteScopeRepository::open(&database).unwrap();
        scopes
            .create_organization(Organization {
                id: OrganizationId::new("org"),
                name: "Org".into(),
                revision: Revision::INITIAL,
                created_at: "now".into(),
                updated_at: "now".into(),
            })
            .unwrap();
        scopes
            .create_project(Project {
                id: project_id.clone(),
                organization_id: OrganizationId::new("org"),
                goal_ids: vec![],
                name: "Canonical".into(),
                description: String::new(),
                status: ProjectStatus::Active,
                revision: Revision::INITIAL,
                created_at: "now".into(),
                updated_at: "now".into(),
            })
            .unwrap();
        let store = WorkStore::open(&database).unwrap();
        store.ensure_project("legacy", "Legacy", "local").unwrap();
        let legacy = store
            .create_work(CreateWorkInput {
                project_id: "legacy".into(),
                title: "Ship bridge".into(),
                description: "Explicit projection".into(),
                acceptance_criteria: String::new(),
                assignee_ref: None,
            })
            .unwrap();
        let bridge = LegacyWorkBridge::open(&database).unwrap();
        let repository = SqliteWorkItemRepository::open(&database).unwrap();
        let projected = bridge.project(&repository, &project_id, &legacy).unwrap();
        assert_eq!(projected.title, "Ship bridge");
        assert_eq!(bridge.project(&repository, &project_id, &legacy).unwrap(), projected);
        let changed = store.start_attempt(&legacy.id, legacy.revision).unwrap();
        assert!(matches!(
            bridge.project(&repository, &project_id, &changed),
            Err(LegacyWorkBridgeError::SourceRevisionChanged { .. })
        ));
        let reconciled = bridge.reconcile(&repository, &project_id, &changed).unwrap();
        assert_eq!(reconciled.status, WorkStatus::InProgress);
        assert_eq!(reconciled.revision, Revision::new(changed.revision as u64));

        let mut independently_changed = repository.get(&reconciled.id).unwrap();
        independently_changed.title = "Canonical edit".into();
        independently_changed.revision = independently_changed.revision.next();
        repository
            .replace_if_revision(independently_changed.clone(), reconciled.revision)
            .unwrap();
        assert!(matches!(
            bridge.project(&repository, &project_id, &changed),
            Err(LegacyWorkBridgeError::CanonicalConflict { .. })
        ));
    }
}
