//! Postgres implementation of the CP-06 Work graph repository.

use crate::{WorkGraphError, WorkGraphRepository};
use altai_control_protocol::{WorkComment, WorkDependency, WorkItemId};
use postgres::{Client, NoTls};
use std::sync::Mutex;

pub struct PostgresWorkGraphRepository {
    client: Mutex<Client>,
}
impl PostgresWorkGraphRepository {
    pub fn connect(url: &str) -> Result<Self, String> {
        let mut client = Client::connect(url, NoTls).map_err(|e| e.to_string())?;
        client.batch_execute("\
          CREATE TABLE IF NOT EXISTS control_plane_work_graph_items (id TEXT PRIMARY KEY);\
          CREATE TABLE IF NOT EXISTS control_plane_work_parents (work_item_id TEXT PRIMARY KEY REFERENCES control_plane_work_graph_items(id), parent_work_item_id TEXT NOT NULL REFERENCES control_plane_work_graph_items(id));\
          CREATE TABLE IF NOT EXISTS control_plane_work_dependencies (work_item_id TEXT NOT NULL REFERENCES control_plane_work_graph_items(id), blocker_work_item_id TEXT NOT NULL REFERENCES control_plane_work_graph_items(id), created_at TEXT NOT NULL, PRIMARY KEY(work_item_id, blocker_work_item_id));\
          CREATE TABLE IF NOT EXISTS control_plane_work_comments (id TEXT PRIMARY KEY, work_item_id TEXT NOT NULL REFERENCES control_plane_work_graph_items(id), payload JSONB NOT NULL);")
          .map_err(|e| e.to_string())?;
        Ok(Self {
            client: Mutex::new(client),
        })
    }
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Client>, WorkGraphError> {
        self.client.lock().map_err(|_| WorkGraphError::Internal {
            reason: "postgres work graph lock poisoned".to_string(),
        })
    }
    fn database_error(error: postgres::Error) -> WorkGraphError {
        WorkGraphError::Internal {
            reason: error.to_string(),
        }
    }
    fn exists(tx: &mut postgres::Transaction<'_>, id: &WorkItemId) -> Result<(), WorkGraphError> {
        if tx
            .query_opt(
                "SELECT 1 FROM control_plane_work_graph_items WHERE id=$1",
                &[&id.value],
            )
            .map_err(Self::database_error)?
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
impl WorkGraphRepository for PostgresWorkGraphRepository {
    fn register_work_item(&self, id: WorkItemId) -> Result<(), WorkGraphError> {
        self.lock()?.execute("INSERT INTO control_plane_work_graph_items (id) VALUES ($1) ON CONFLICT DO NOTHING", &[&id.value]).map_err(Self::database_error)?;
        Ok(())
    }
    fn set_parent(&self, id: WorkItemId, parent: Option<WorkItemId>) -> Result<(), WorkGraphError> {
        let mut client = self.lock()?;
        let mut tx = client.transaction().map_err(Self::database_error)?;
        Self::exists(&mut tx, &id)?;
        let Some(parent) = parent else {
            tx.execute(
                "DELETE FROM control_plane_work_parents WHERE work_item_id=$1",
                &[&id.value],
            )
            .map_err(Self::database_error)?;
            tx.commit().map_err(Self::database_error)?;
            return Ok(());
        };
        Self::exists(&mut tx, &parent)?;
        let mut current = parent.value.clone();
        loop {
            if current == id.value {
                return Err(WorkGraphError::ParentCycle {
                    work_item_id: id.value,
                });
            }
            match tx.query_opt("SELECT parent_work_item_id FROM control_plane_work_parents WHERE work_item_id=$1", &[&current]).map_err(Self::database_error)? { Some(row) => current = row.get(0), None => break }
        }
        tx.execute("INSERT INTO control_plane_work_parents (work_item_id,parent_work_item_id) VALUES ($1,$2) ON CONFLICT (work_item_id) DO UPDATE SET parent_work_item_id=EXCLUDED.parent_work_item_id", &[&id.value,&parent.value]).map_err(Self::database_error)?;
        tx.commit().map_err(Self::database_error)?;
        Ok(())
    }
    fn add_dependency(&self, dependency: WorkDependency) -> Result<(), WorkGraphError> {
        let mut client = self.lock()?;
        let mut tx = client.transaction().map_err(Self::database_error)?;
        Self::exists(&mut tx, &dependency.work_item_id)?;
        Self::exists(&mut tx, &dependency.blocker_work_item_id)?;
        let inserted=tx.execute("INSERT INTO control_plane_work_dependencies (work_item_id,blocker_work_item_id,created_at) VALUES ($1,$2,$3) ON CONFLICT DO NOTHING", &[&dependency.work_item_id.value,&dependency.blocker_work_item_id.value,&dependency.created_at]).map_err(Self::database_error)?;
        if inserted == 0 {
            return Err(WorkGraphError::AlreadyExists {
                relation: "dependency",
            });
        }
        tx.commit().map_err(Self::database_error)?;
        Ok(())
    }
    fn add_comment(&self, comment: WorkComment) -> Result<(), WorkGraphError> {
        let mut client = self.lock()?;
        let mut tx = client.transaction().map_err(Self::database_error)?;
        Self::exists(&mut tx, &comment.work_item_id)?;
        let payload = serde_json::to_value(&comment).map_err(|e| WorkGraphError::Internal {
            reason: e.to_string(),
        })?;
        let inserted=tx.execute("INSERT INTO control_plane_work_comments (id,work_item_id,payload) VALUES ($1,$2,$3) ON CONFLICT DO NOTHING", &[&comment.id,&comment.work_item_id.value,&payload]).map_err(Self::database_error)?;
        if inserted == 0 {
            return Err(WorkGraphError::AlreadyExists {
                relation: "comment",
            });
        }
        tx.commit().map_err(Self::database_error)?;
        Ok(())
    }
    fn dependencies(&self, id: &WorkItemId) -> Result<Vec<WorkDependency>, WorkGraphError> {
        let mut client = self.lock()?;
        if client
            .query_opt(
                "SELECT 1 FROM control_plane_work_graph_items WHERE id=$1",
                &[&id.value],
            )
            .map_err(Self::database_error)?
            .is_none()
        {
            return Err(WorkGraphError::NotFound {
                work_item_id: id.value.clone(),
            });
        }
        client.query("SELECT blocker_work_item_id,created_at FROM control_plane_work_dependencies WHERE work_item_id=$1", &[&id.value]).map_err(Self::database_error)?.into_iter().map(|row|Ok(WorkDependency{work_item_id:id.clone(),blocker_work_item_id:WorkItemId::new(row.get::<_,String>(0)),created_at:row.get(1)})).collect()
    }
}
