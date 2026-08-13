//! CP-07 coalesced wake queue and exclusive checkout repository boundary.

use altai_control_protocol::{AttemptId, WakeRequest, WakeSource, WorkCheckoutLease, WorkItemId};
use std::{collections::HashMap, sync::Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WakeError {
    ActiveCheckout { work_item_id: String },
    NotFound { work_item_id: String },
    Internal { reason: String },
}
pub trait WakeRepository: Send + Sync {
    fn enqueue(
        &self,
        work_item_id: WorkItemId,
        source: WakeSource,
        requested_at: String,
    ) -> Result<WakeRequest, WakeError>;
    fn checkout(&self, lease: WorkCheckoutLease) -> Result<(), WakeError>;
    fn release_checkout(
        &self,
        work_item_id: &WorkItemId,
        attempt_id: &AttemptId,
    ) -> Result<(), WakeError>;
}
#[derive(Default)]
pub struct InMemoryWakeRepository {
    state: Mutex<WakeState>,
}
#[derive(Default)]
struct WakeState {
    wakes: HashMap<String, WakeRequest>,
    leases: HashMap<String, WorkCheckoutLease>,
}
impl InMemoryWakeRepository {
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, WakeState>, WakeError> {
        self.state.lock().map_err(|_| WakeError::Internal {
            reason: "wake lock poisoned".to_string(),
        })
    }
}
impl WakeRepository for InMemoryWakeRepository {
    fn enqueue(
        &self,
        work_item_id: WorkItemId,
        source: WakeSource,
        requested_at: String,
    ) -> Result<WakeRequest, WakeError> {
        let mut state = self.lock()?;
        let key = work_item_id.value.clone();
        let wake = state
            .wakes
            .entry(key.clone())
            .or_insert_with(|| WakeRequest {
                id: format!("wake_{key}"),
                work_item_id: work_item_id.clone(),
                sources: vec![],
                requested_at: requested_at.clone(),
                claimed_at: None,
            });
        if !wake.sources.contains(&source) {
            wake.sources.push(source)
        };
        Ok(wake.clone())
    }
    fn checkout(&self, lease: WorkCheckoutLease) -> Result<(), WakeError> {
        let mut state = self.lock()?;
        let key = lease.work_item_id.value.clone();
        if state.leases.contains_key(&key) {
            return Err(WakeError::ActiveCheckout { work_item_id: key });
        };
        state.leases.insert(key, lease);
        Ok(())
    }
    fn release_checkout(
        &self,
        work_item_id: &WorkItemId,
        attempt_id: &AttemptId,
    ) -> Result<(), WakeError> {
        let mut state = self.lock()?;
        let lease = state
            .leases
            .get(&work_item_id.value)
            .ok_or_else(|| WakeError::NotFound {
                work_item_id: work_item_id.value.clone(),
            })?;
        if &lease.attempt_id != attempt_id {
            return Err(WakeError::ActiveCheckout {
                work_item_id: work_item_id.value.clone(),
            });
        };
        state.leases.remove(&work_item_id.value);
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use altai_control_protocol::AgentInstanceId;
    #[test]
    fn coalesces_sources_and_allows_only_one_checkout() {
        let repo = InMemoryWakeRepository::default();
        let work = WorkItemId::new("a");
        let first = repo
            .enqueue(work.clone(), WakeSource::Assignment, "t".to_string())
            .unwrap();
        let second = repo
            .enqueue(work.clone(), WakeSource::Comment, "later".to_string())
            .unwrap();
        assert_eq!(first.id, second.id);
        assert_eq!(second.sources.len(), 2);
        let lease = WorkCheckoutLease {
            work_item_id: work.clone(),
            owner_agent_instance_id: AgentInstanceId::new("a"),
            attempt_id: AttemptId::new("a"),
            expires_at: "later".to_string(),
        };
        repo.checkout(lease.clone()).unwrap();
        assert!(matches!(
            repo.checkout(lease),
            Err(WakeError::ActiveCheckout { .. })
        ));
    }
}
