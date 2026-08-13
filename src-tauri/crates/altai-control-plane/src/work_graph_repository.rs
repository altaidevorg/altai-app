//! CP-06 Work hierarchy, blocker edge, and comment repository boundary.

use altai_control_protocol::{WorkComment, WorkDependency, WorkItemId};
use std::{
    collections::{HashMap, HashSet},
    sync::Mutex,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkGraphError {
    NotFound { work_item_id: String },
    AlreadyExists { relation: &'static str },
    ParentCycle { work_item_id: String },
    Internal { reason: String },
}

pub trait WorkGraphRepository: Send + Sync {
    fn register_work_item(&self, work_item_id: WorkItemId) -> Result<(), WorkGraphError>;
    fn set_parent(
        &self,
        work_item_id: WorkItemId,
        parent_work_item_id: Option<WorkItemId>,
    ) -> Result<(), WorkGraphError>;
    fn add_dependency(&self, dependency: WorkDependency) -> Result<(), WorkGraphError>;
    fn add_comment(&self, comment: WorkComment) -> Result<(), WorkGraphError>;
    fn dependencies(
        &self,
        work_item_id: &WorkItemId,
    ) -> Result<Vec<WorkDependency>, WorkGraphError>;
}

#[derive(Default)]
pub struct InMemoryWorkGraphRepository {
    state: Mutex<WorkGraphState>,
}
#[derive(Default)]
struct WorkGraphState {
    work_items: HashSet<String>,
    parents: HashMap<String, String>,
    dependencies: Vec<WorkDependency>,
    comments: Vec<WorkComment>,
}

impl InMemoryWorkGraphRepository {
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, WorkGraphState>, WorkGraphError> {
        self.state.lock().map_err(|_| WorkGraphError::Internal {
            reason: "work graph lock poisoned".to_string(),
        })
    }
    fn require_exists(state: &WorkGraphState, id: &WorkItemId) -> Result<(), WorkGraphError> {
        if state.work_items.contains(&id.value) {
            Ok(())
        } else {
            Err(WorkGraphError::NotFound {
                work_item_id: id.value.clone(),
            })
        }
    }
}

impl WorkGraphRepository for InMemoryWorkGraphRepository {
    fn register_work_item(&self, work_item_id: WorkItemId) -> Result<(), WorkGraphError> {
        self.lock()?.work_items.insert(work_item_id.value);
        Ok(())
    }
    fn set_parent(
        &self,
        work_item_id: WorkItemId,
        parent: Option<WorkItemId>,
    ) -> Result<(), WorkGraphError> {
        let mut state = self.lock()?;
        Self::require_exists(&state, &work_item_id)?;
        let Some(parent) = parent else {
            state.parents.remove(&work_item_id.value);
            return Ok(());
        };
        Self::require_exists(&state, &parent)?;
        let mut current = parent.value.clone();
        loop {
            if current == work_item_id.value {
                return Err(WorkGraphError::ParentCycle {
                    work_item_id: work_item_id.value,
                });
            }
            match state.parents.get(&current) {
                Some(next) => current = next.clone(),
                None => break,
            }
        }
        state.parents.insert(work_item_id.value, parent.value);
        Ok(())
    }
    fn add_dependency(&self, dependency: WorkDependency) -> Result<(), WorkGraphError> {
        let mut state = self.lock()?;
        Self::require_exists(&state, &dependency.work_item_id)?;
        Self::require_exists(&state, &dependency.blocker_work_item_id)?;
        if state.dependencies.iter().any(|edge| edge == &dependency) {
            return Err(WorkGraphError::AlreadyExists {
                relation: "dependency",
            });
        }
        state.dependencies.push(dependency);
        Ok(())
    }
    fn add_comment(&self, comment: WorkComment) -> Result<(), WorkGraphError> {
        let mut state = self.lock()?;
        Self::require_exists(&state, &comment.work_item_id)?;
        state.comments.push(comment);
        Ok(())
    }
    fn dependencies(
        &self,
        work_item_id: &WorkItemId,
    ) -> Result<Vec<WorkDependency>, WorkGraphError> {
        let state = self.lock()?;
        Self::require_exists(&state, work_item_id)?;
        Ok(state
            .dependencies
            .iter()
            .filter(|edge| edge.work_item_id == *work_item_id)
            .cloned()
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use altai_control_protocol::WorkItemId;

    #[test]
    fn parent_cycle_is_rejected_while_dependency_remains_separate() {
        let repository = InMemoryWorkGraphRepository::default();
        let a = WorkItemId::new("a");
        let b = WorkItemId::new("b");
        repository.register_work_item(a.clone()).unwrap();
        repository.register_work_item(b.clone()).unwrap();
        repository.set_parent(b.clone(), Some(a.clone())).unwrap();
        assert!(matches!(
            repository.set_parent(a.clone(), Some(b.clone())),
            Err(WorkGraphError::ParentCycle { .. })
        ));
        repository
            .add_dependency(WorkDependency {
                work_item_id: a.clone(),
                blocker_work_item_id: b,
                created_at: "2026-08-13T00:00:00Z".to_string(),
            })
            .unwrap();
        assert_eq!(repository.dependencies(&a).unwrap().len(), 1);
    }
}
