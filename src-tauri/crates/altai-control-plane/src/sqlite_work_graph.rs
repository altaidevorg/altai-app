//! Local SQLite implementation of the CP-06 Work graph repository.

use crate::{WorkGraphError, WorkGraphRepository};
use altai_control_protocol::{WorkComment, WorkDependency, WorkItemId};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use std::{path::Path, sync::Mutex};

pub struct SqliteWorkGraphRepository {
    connection: Mutex<Connection>,
}
impl SqliteWorkGraphRepository {
    pub fn open(path: &Path) -> Result<Self, String> {
        let connection = Connection::open(path).map_err(|e| e.to_string())?;
        connection.execute_batch("PRAGMA foreign_keys = ON; PRAGMA busy_timeout = 5000; CREATE TABLE IF NOT EXISTS control_plane_work_graph_items (id TEXT PRIMARY KEY); CREATE TABLE IF NOT EXISTS control_plane_work_parents (work_item_id TEXT PRIMARY KEY REFERENCES control_plane_work_graph_items(id), parent_work_item_id TEXT NOT NULL REFERENCES control_plane_work_graph_items(id)); CREATE TABLE IF NOT EXISTS control_plane_work_dependencies (work_item_id TEXT NOT NULL REFERENCES control_plane_work_graph_items(id), blocker_work_item_id TEXT NOT NULL REFERENCES control_plane_work_graph_items(id), created_at TEXT NOT NULL, PRIMARY KEY(work_item_id, blocker_work_item_id)); CREATE TABLE IF NOT EXISTS control_plane_work_comments (id TEXT PRIMARY KEY, work_item_id TEXT NOT NULL REFERENCES control_plane_work_graph_items(id), payload_json TEXT NOT NULL);").map_err(|e| e.to_string())?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, WorkGraphError> {
        self.connection
            .lock()
            .map_err(|_| WorkGraphError::Internal {
                reason: "sqlite work graph lock poisoned".into(),
            })
    }
    fn db(error: rusqlite::Error) -> WorkGraphError {
        WorkGraphError::Internal {
            reason: error.to_string(),
        }
    }
    fn exists(tx: &Transaction<'_>, id: &WorkItemId) -> Result<(), WorkGraphError> {
        if tx
            .query_row(
                "SELECT 1 FROM control_plane_work_graph_items WHERE id = ?1",
                [&id.value],
                |_| Ok(()),
            )
            .optional()
            .map_err(Self::db)?
            .is_some()
        {
            Ok(())
        } else {
            Err(WorkGraphError::NotFound {
                work_item_id: id.value.clone(),
            })
        }
    }
}
impl WorkGraphRepository for SqliteWorkGraphRepository {
    fn register_work_item(&self, id: WorkItemId) -> Result<(), WorkGraphError> {
        self.lock()?.execute("INSERT INTO control_plane_work_graph_items (id) VALUES (?1) ON CONFLICT(id) DO NOTHING", [&id.value]).map_err(Self::db)?;
        Ok(())
    }
    fn set_parent(&self, id: WorkItemId, parent: Option<WorkItemId>) -> Result<(), WorkGraphError> {
        let mut connection = self.lock()?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(Self::db)?;
        Self::exists(&tx, &id)?;
        let Some(parent) = parent else {
            tx.execute(
                "DELETE FROM control_plane_work_parents WHERE work_item_id = ?1",
                [&id.value],
            )
            .map_err(Self::db)?;
            tx.commit().map_err(Self::db)?;
            return Ok(());
        };
        Self::exists(&tx, &parent)?;
        let mut current = parent.value.clone();
        loop {
            if current == id.value {
                return Err(WorkGraphError::ParentCycle {
                    work_item_id: id.value,
                });
            }
            let next: Option<String> = tx.query_row("SELECT parent_work_item_id FROM control_plane_work_parents WHERE work_item_id = ?1", [&current], |row| row.get(0)).optional().map_err(Self::db)?;
            match next {
                Some(next) => current = next,
                None => break,
            }
        }
        tx.execute("INSERT INTO control_plane_work_parents (work_item_id, parent_work_item_id) VALUES (?1, ?2) ON CONFLICT(work_item_id) DO UPDATE SET parent_work_item_id = excluded.parent_work_item_id", params![id.value, parent.value]).map_err(Self::db)?;
        tx.commit().map_err(Self::db)?;
        Ok(())
    }
    fn add_dependency(&self, dependency: WorkDependency) -> Result<(), WorkGraphError> {
        let mut connection = self.lock()?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(Self::db)?;
        Self::exists(&tx, &dependency.work_item_id)?;
        Self::exists(&tx, &dependency.blocker_work_item_id)?;
        let inserted = tx.execute("INSERT INTO control_plane_work_dependencies (work_item_id, blocker_work_item_id, created_at) VALUES (?1, ?2, ?3) ON CONFLICT DO NOTHING", params![dependency.work_item_id.value, dependency.blocker_work_item_id.value, dependency.created_at]).map_err(Self::db)?;
        if inserted == 0 {
            return Err(WorkGraphError::AlreadyExists {
                relation: "dependency",
            });
        }
        tx.commit().map_err(Self::db)?;
        Ok(())
    }
    fn add_comment(&self, comment: WorkComment) -> Result<(), WorkGraphError> {
        let mut connection = self.lock()?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(Self::db)?;
        Self::exists(&tx, &comment.work_item_id)?;
        let payload = serde_json::to_string(&comment).map_err(|e| WorkGraphError::Internal {
            reason: e.to_string(),
        })?;
        let inserted = tx.execute("INSERT INTO control_plane_work_comments (id, work_item_id, payload_json) VALUES (?1, ?2, ?3) ON CONFLICT(id) DO NOTHING", params![comment.id, comment.work_item_id.value, payload]).map_err(Self::db)?;
        if inserted == 0 {
            return Err(WorkGraphError::AlreadyExists {
                relation: "comment",
            });
        }
        tx.commit().map_err(Self::db)?;
        Ok(())
    }
    fn dependencies(&self, id: &WorkItemId) -> Result<Vec<WorkDependency>, WorkGraphError> {
        let connection = self.lock()?;
        if !connection
            .query_row(
                "SELECT 1 FROM control_plane_work_graph_items WHERE id = ?1",
                [&id.value],
                |_| Ok(()),
            )
            .optional()
            .map_err(Self::db)?
            .is_some()
        {
            return Err(WorkGraphError::NotFound {
                work_item_id: id.value.clone(),
            });
        }
        let mut statement = connection.prepare("SELECT blocker_work_item_id, created_at FROM control_plane_work_dependencies WHERE work_item_id = ?1").map_err(Self::db)?;
        let rows = statement
            .query_map([&id.value], |row| {
                Ok(WorkDependency {
                    work_item_id: id.clone(),
                    blocker_work_item_id: WorkItemId::new(row.get::<_, String>(0)?),
                    created_at: row.get(1)?,
                })
            })
            .map_err(Self::db)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Self::db)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dependencies_are_durable_across_reopen() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("work.db");
        let repository = SqliteWorkGraphRepository::open(&database).unwrap();
        let first = WorkItemId::new("first");
        let second = WorkItemId::new("second");
        repository.register_work_item(first.clone()).unwrap();
        repository.register_work_item(second.clone()).unwrap();
        repository
            .add_dependency(WorkDependency {
                work_item_id: first.clone(),
                blocker_work_item_id: second,
                created_at: "now".into(),
            })
            .unwrap();
        assert_eq!(
            SqliteWorkGraphRepository::open(&database)
                .unwrap()
                .dependencies(&first)
                .unwrap()
                .len(),
            1
        );
    }
}
