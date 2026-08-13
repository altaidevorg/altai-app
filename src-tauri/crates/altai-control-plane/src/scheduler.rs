//! CP-07 single-writer scheduler handoff.

use crate::{
    DispatchEligibility, DispatchEligibilityEngine, DispatchEligibilityError, SqliteWakeRepository,
    WakeError,
};
use altai_control_protocol::WorkCheckoutLease;
use std::collections::HashSet;

#[derive(Debug)]
pub enum ScheduleResult {
    Blocked(DispatchEligibility),
    Claimed,
}
#[derive(Debug)]
pub enum SchedulerError {
    Eligibility(DispatchEligibilityError),
    Wake(WakeError),
}
impl std::fmt::Display for SchedulerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "scheduler error: {self:?}")
    }
}
impl std::error::Error for SchedulerError {}

pub struct SingleWriterScheduler {
    eligibility: DispatchEligibilityEngine,
    wakes: SqliteWakeRepository,
}
impl SingleWriterScheduler {
    pub fn new(eligibility: DispatchEligibilityEngine, wakes: SqliteWakeRepository) -> Self {
        Self { eligibility, wakes }
    }
    pub fn claim_if_eligible(
        &self,
        lease: WorkCheckoutLease,
        claimed_at: String,
        now_unix_seconds: u64,
        completed_work_item_ids: &HashSet<String>,
    ) -> Result<ScheduleResult, SchedulerError> {
        let decision = self
            .eligibility
            .evaluate(
                &lease.work_item_id,
                &lease.owner_agent_instance_id,
                completed_work_item_ids,
            )
            .map_err(SchedulerError::Eligibility)?;
        if !decision.eligible {
            return Ok(ScheduleResult::Blocked(decision));
        }
        self.wakes
            .claim_and_checkout(lease, claimed_at, now_unix_seconds)
            .map_err(SchedulerError::Wake)?;
        Ok(ScheduleResult::Claimed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AgentRepository, InMemoryAgentRepository, InMemoryWorkGraphRepository, WakeRepository,
        WorkGraphRepository,
    };
    use altai_control_protocol::{
        AgentInstance, AgentInstanceId, AgentProfileId, AgentProfileRevision,
        AgentProfileRevisionId, AgentStatus, OrganizationId, Revision, WakeSource, WorkItemId,
    };

    fn scheduler(
        directory: &std::path::Path,
        status: AgentStatus,
    ) -> (SingleWriterScheduler, WorkItemId) {
        let agents = std::sync::Arc::new(InMemoryAgentRepository::default());
        agents
            .append_profile_revision(AgentProfileRevision {
                id: AgentProfileRevisionId::new("profile-v1"),
                profile_id: AgentProfileId::new("profile"),
                revision: Revision::INITIAL,
                instructions: String::new(),
                model: None,
                capabilities: vec![],
                created_at: "now".into(),
            })
            .unwrap();
        agents
            .create_instance(AgentInstance {
                id: AgentInstanceId::new("agent"),
                organization_id: OrganizationId::new("local"),
                profile_revision_id: AgentProfileRevisionId::new("profile-v1"),
                reports_to_agent_id: None,
                name: "agent".into(),
                role: "worker".into(),
                capabilities: vec![],
                status,
                pause_reason: None,
                revision: Revision::INITIAL,
                created_at: "now".into(),
                updated_at: "now".into(),
            })
            .unwrap();
        let graph = std::sync::Arc::new(InMemoryWorkGraphRepository::default());
        let work = WorkItemId::new("work");
        graph.register_work_item(work.clone()).unwrap();
        let eligibility = DispatchEligibilityEngine::new(agents, graph);
        (
            SingleWriterScheduler::new(
                eligibility,
                SqliteWakeRepository::open(&directory.join("work.db")).unwrap(),
            ),
            work,
        )
    }

    #[test]
    fn eligible_wake_is_claimed_with_its_lease() {
        let directory = tempfile::tempdir().unwrap();
        let (scheduler, work) = scheduler(directory.path(), AgentStatus::Active);
        scheduler
            .wakes
            .enqueue(work.clone(), WakeSource::Manual, "queued".into())
            .unwrap();
        assert!(matches!(
            scheduler
                .claim_if_eligible(
                    WorkCheckoutLease {
                        work_item_id: work,
                        owner_agent_instance_id: AgentInstanceId::new("agent"),
                        attempt_id: altai_control_protocol::AttemptId::new("attempt"),
                        expires_at_unix_seconds: 20
                    },
                    "claimed".into(),
                    10,
                    &HashSet::new()
                )
                .unwrap(),
            ScheduleResult::Claimed
        ));
    }

    #[test]
    fn blocked_wake_is_not_claimed() {
        let directory = tempfile::tempdir().unwrap();
        let (scheduler, work) = scheduler(directory.path(), AgentStatus::Paused);
        scheduler
            .wakes
            .enqueue(work.clone(), WakeSource::Manual, "queued".into())
            .unwrap();
        assert!(matches!(
            scheduler
                .claim_if_eligible(
                    WorkCheckoutLease {
                        work_item_id: work.clone(),
                        owner_agent_instance_id: AgentInstanceId::new("agent"),
                        attempt_id: altai_control_protocol::AttemptId::new("attempt"),
                        expires_at_unix_seconds: 20
                    },
                    "claimed".into(),
                    10,
                    &HashSet::new()
                )
                .unwrap(),
            ScheduleResult::Blocked(_)
        ));
        assert!(scheduler.wakes.claim_wake(&work, "later".into()).is_ok());
    }
}
