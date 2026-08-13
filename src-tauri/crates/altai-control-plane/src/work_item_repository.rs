//! CP-06 durable canonical WorkItem repository.

use altai_control_protocol::{ProjectId, WorkItem, WorkItemId};
use rusqlite::{params, Connection, OptionalExtension};
use std::{path::Path, sync::Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkItemRepositoryError {
    AlreadyExists { work_item_id: String },
    NotFound { work_item_id: String },
    ProjectMismatch { work_item_id: String },
    Internal { reason: String },
}
impl std::fmt::Display for WorkItemRepositoryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "work item repository error: {self:?}")
    }
}
impl std::error::Error for WorkItemRepositoryError {}

pub trait WorkItemRepository: Send + Sync {
    fn create(&self, item: WorkItem) -> Result<(), WorkItemRepositoryError>;
    fn get(&self, id: &WorkItemId) -> Result<WorkItem, WorkItemRepositoryError>;
    fn get_in_project(
        &self,
        project_id: &ProjectId,
        id: &WorkItemId,
    ) -> Result<WorkItem, WorkItemRepositoryError>;
}

pub struct SqliteWorkItemRepository {
    connection: Mutex<Connection>,
}
impl SqliteWorkItemRepository {
    pub fn open(path: &Path) -> Result<Self, String> {
        let connection = Connection::open(path).map_err(|error| error.to_string())?;
        connection.execute_batch("PRAGMA foreign_keys = ON; PRAGMA busy_timeout = 5000; CREATE TABLE IF NOT EXISTS control_plane_work_items (id TEXT PRIMARY KEY, project_id TEXT NOT NULL REFERENCES control_plane_projects(id), payload_json TEXT NOT NULL);").map_err(|error| error.to_string())?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, WorkItemRepositoryError> {
        self.connection
            .lock()
            .map_err(|_| WorkItemRepositoryError::Internal {
                reason: "work item lock poisoned".into(),
            })
    }
    fn db(error: rusqlite::Error) -> WorkItemRepositoryError {
        WorkItemRepositoryError::Internal {
            reason: error.to_string(),
        }
    }
    fn decode(payload: String) -> Result<WorkItem, WorkItemRepositoryError> {
        serde_json::from_str(&payload).map_err(|error| WorkItemRepositoryError::Internal {
            reason: error.to_string(),
        })
    }
}
impl WorkItemRepository for SqliteWorkItemRepository {
    fn create(&self, item: WorkItem) -> Result<(), WorkItemRepositoryError> {
        let payload =
            serde_json::to_string(&item).map_err(|error| WorkItemRepositoryError::Internal {
                reason: error.to_string(),
            })?;
        let inserted = self.lock()?.execute("INSERT INTO control_plane_work_items (id, project_id, payload_json) VALUES (?1, ?2, ?3) ON CONFLICT(id) DO NOTHING", params![item.id.value, item.project_id.value, payload]).map_err(Self::db)?;
        if inserted == 1 {
            Ok(())
        } else {
            Err(WorkItemRepositoryError::AlreadyExists {
                work_item_id: item.id.value,
            })
        }
    }
    fn get(&self, id: &WorkItemId) -> Result<WorkItem, WorkItemRepositoryError> {
        let payload: Option<String> = self
            .lock()?
            .query_row(
                "SELECT payload_json FROM control_plane_work_items WHERE id = ?1",
                [&id.value],
                |row| row.get(0),
            )
            .optional()
            .map_err(Self::db)?;
        payload
            .map(Self::decode)
            .transpose()?
            .ok_or_else(|| WorkItemRepositoryError::NotFound {
                work_item_id: id.value.clone(),
            })
    }
    fn get_in_project(
        &self,
        project_id: &ProjectId,
        id: &WorkItemId,
    ) -> Result<WorkItem, WorkItemRepositoryError> {
        let item = self.get(id)?;
        if item.project_id == *project_id {
            Ok(item)
        } else {
            Err(WorkItemRepositoryError::ProjectMismatch {
                work_item_id: id.value.clone(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ScopeRepository, SqliteScopeRepository};
    use altai_control_protocol::{
        ExecutionPhase, Organization, OrganizationId, Project, ProjectStatus, Revision,
        WorkItemKind, WorkStatus,
    };

    fn item(project_id: ProjectId) -> WorkItem {
        WorkItem {
            id: WorkItemId::new("work"),
            project_id,
            goal_id: None,
            parent_work_item_id: None,
            kind: WorkItemKind::Task,
            title: "Ship".into(),
            description: String::new(),
            status: WorkStatus::Todo,
            execution_phase: ExecutionPhase::Queued,
            revision: Revision::INITIAL,
            created_at: "now".into(),
            updated_at: "now".into(),
        }
    }
    #[test]
    fn durable_work_items_are_project_scoped() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("work.db");
        let scopes = SqliteScopeRepository::open(&database).unwrap();
        let org = Organization {
            id: OrganizationId::new("org"),
            name: "Org".into(),
            revision: Revision::INITIAL,
            created_at: "now".into(),
            updated_at: "now".into(),
        };
        scopes.create_organization(org.clone()).unwrap();
        let project = Project {
            id: ProjectId::new("project"),
            organization_id: org.id,
            goal_ids: vec![],
            name: "Project".into(),
            description: String::new(),
            status: ProjectStatus::Active,
            revision: Revision::INITIAL,
            created_at: "now".into(),
            updated_at: "now".into(),
        };
        scopes.create_project(project.clone()).unwrap();
        SqliteWorkItemRepository::open(&database)
            .unwrap()
            .create(item(project.id.clone()))
            .unwrap();
        let repository = SqliteWorkItemRepository::open(&database).unwrap();
        assert_eq!(
            repository
                .get_in_project(&project.id, &WorkItemId::new("work"))
                .unwrap()
                .title,
            "Ship"
        );
        assert!(matches!(
            repository.get_in_project(&ProjectId::new("other"), &WorkItemId::new("work")),
            Err(WorkItemRepositoryError::ProjectMismatch { .. })
        ));
    }
}
