//! Bounded recovery for lost attempts — the response half of crash/restart
//! recovery. [`RecoveryService::recover_lost`] consumes a reaped (Lost)
//! attempt: the failure is recorded in the package-023 recovery store, which
//! counts retries **per work item** and dead-letters past `max_retries`, and —
//! while retries remain — a *fresh* attempt is created in `Created` for the
//! same work item, owner, and profile revision so the scheduler can
//! redispatch it. The lost attempt is never reopened or reassigned; it stays
//! `Lost` as evidence. The replacement id is derived from the lost id and the
//! retry count, so replaying recovery for the same lost attempt is idempotent.

use std::sync::Arc;

use altai_control_protocol::{Attempt, AttemptId, RecoveryDisposition, RecoveryRecord};

use crate::{AttemptError, AttemptRepository, RecoveryError, RecoveryRepository};

pub struct RecoveryService {
    attempts: Arc<dyn AttemptRepository>,
    recovery: Arc<dyn RecoveryRepository>,
}

impl RecoveryService {
    pub fn new(
        attempts: Arc<dyn AttemptRepository>,
        recovery: Arc<dyn RecoveryRepository>,
    ) -> Self {
        Self { attempts, recovery }
    }

    /// Recover a lost attempt: record the failure (bounded per work item) and,
    /// while retries remain, create a fresh `Created` attempt carrying the same
    /// work item, owner, and profile revision. Past the bound the record is
    /// dead-lettered and nothing is created. Idempotent when replayed for the
    /// same lost attempt and failure.
    pub fn recover_lost(
        &self,
        lost: &Attempt,
        failure: String,
        now_unix_seconds: u64,
        max_retries: u32,
    ) -> Result<RecoveryOutcome, RecoveryServiceError> {
        let record = self
            .recovery
            .record_failure(
                lost.work_item_id.clone(),
                lost.id.clone(),
                max_retries,
                failure,
                now_unix_seconds,
            )
            .map_err(RecoveryServiceError::Recovery)?;
        if record.disposition == RecoveryDisposition::DeadLettered {
            return Ok(RecoveryOutcome {
                record,
                replacement: None,
            });
        }
        let replacement_id = AttemptId::new(format!(
            "{}-retry-{}",
            lost.id.value, record.retry_count
        ));
        let replacement = self
            .attempts
            .create(Attempt {
                id: replacement_id,
                work_item_id: lost.work_item_id.clone(),
                owner_agent_instance_id: lost.owner_agent_instance_id.clone(),
                profile_revision_id: lost.profile_revision_id.clone(),
                state: altai_control_protocol::AttemptState::Created,
                // Birth time is the stored record time, so a replay builds
                // byte-identical replacement bytes and `create` stays
                // idempotent even when `now` differs between attempts.
                created_at_unix_seconds: record.updated_at_unix_seconds,
                updated_at_unix_seconds: record.updated_at_unix_seconds,
            })
            .map_err(RecoveryServiceError::Attempt)?;
        Ok(RecoveryOutcome {
            record,
            replacement: Some(replacement),
        })
    }
}

/// The result of recovering a lost attempt. `record` is the retained trigger
/// evidence either way; `replacement` is the fresh attempt to redispatch, or
/// `None` when the retry bound was hit and the record was dead-lettered.
#[derive(Debug)]
pub struct RecoveryOutcome {
    pub record: RecoveryRecord,
    pub replacement: Option<Attempt>,
}

#[derive(Debug)]
pub enum RecoveryServiceError {
    Recovery(RecoveryError),
    Attempt(AttemptError),
}

impl std::fmt::Display for RecoveryServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Recovery(e) => write!(f, "recovery store failure: {e}"),
            Self::Attempt(e) => write!(f, "recovery attempt failure: {e}"),
        }
    }
}
impl std::error::Error for RecoveryServiceError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        LivenessMonitor, SqliteAttemptRepository, SqliteRecoveryRepository,
    };
    use altai_control_protocol::{
        AgentInstanceId, AgentProfileRevisionId, AttemptState, WorkItemId,
    };

    struct Harness {
        _dir: tempfile::TempDir,
        service: RecoveryService,
        monitor: LivenessMonitor,
        attempts: Arc<SqliteAttemptRepository>,
        recovery: Arc<SqliteRecoveryRepository>,
    }

    fn harness() -> Harness {
        let dir = tempfile::tempdir().unwrap();
        let attempts = Arc::new(SqliteAttemptRepository::open(&dir.path().join("work.db")).unwrap());
        let recovery = Arc::new(SqliteRecoveryRepository::open(&dir.path().join("work.db")).unwrap());
        let service = RecoveryService::new(attempts.clone(), recovery.clone());
        let monitor = LivenessMonitor::new(attempts.clone());
        Harness {
            _dir: dir,
            service,
            monitor,
            attempts,
            recovery,
        }
    }

    fn attempt(id: &str) -> Attempt {
        Attempt {
            id: AttemptId::new(id),
            work_item_id: WorkItemId::new("work"),
            owner_agent_instance_id: AgentInstanceId::new("owner-a"),
            profile_revision_id: AgentProfileRevisionId::new("apr"),
            state: AttemptState::Created,
            created_at_unix_seconds: 1,
            updated_at_unix_seconds: 1,
        }
    }

    /// Create `id` and walk it to Running, stamping `updated_at` on the last step.
    fn running(h: &Harness, id: &str, updated_at: u64) -> Attempt {
        let attempt = attempt(id);
        h.attempts.create(attempt.clone()).unwrap();
        h.attempts
            .transition(&attempt.id, AttemptState::Claimed, updated_at)
            .unwrap();
        h.attempts
            .transition(&attempt.id, AttemptState::Dispatched, updated_at)
            .unwrap();
        h.attempts
            .transition(&attempt.id, AttemptState::Running, updated_at)
            .unwrap();
        h.attempts.get(&attempt.id).unwrap().unwrap()
    }

    #[test]
    fn recover_lost_creates_a_fresh_attempt_with_the_same_assignment() {
        let h = harness();
        let lost = running(&h, "lost", 100);
        h.attempts
            .transition(&lost.id, AttemptState::Lost, 200)
            .unwrap();

        let outcome = h
            .service
            .recover_lost(&lost, "host went silent".into(), 300, 3)
            .unwrap();
        let replacement = outcome.replacement.expect("expected a replacement");
        assert_eq!(replacement.state, AttemptState::Created);
        assert_eq!(replacement.work_item_id, lost.work_item_id);
        assert_eq!(replacement.owner_agent_instance_id, lost.owner_agent_instance_id);
        assert_eq!(replacement.profile_revision_id, lost.profile_revision_id);
        // Derived id carries the lost id and the retry count.
        assert_eq!(replacement.id.value, format!("{}-retry-1", lost.id.value));
        assert_eq!(outcome.record.retry_count, 1);
        assert_eq!(outcome.record.last_failure, "host went silent");
        // Durable.
        assert_eq!(
            h.attempts.get(&replacement.id).unwrap().unwrap().state,
            AttemptState::Created
        );
        // The lost attempt stays lost, untouched, still naming its owner.
        let stored = h.attempts.get(&lost.id).unwrap().unwrap();
        assert_eq!(stored.state, AttemptState::Lost);
        assert_eq!(stored.owner_agent_instance_id, lost.owner_agent_instance_id);
    }

    #[test]
    fn recover_lost_is_idempotent_on_replay() {
        let h = harness();
        let lost = running(&h, "lost", 100);
        h.attempts
            .transition(&lost.id, AttemptState::Lost, 200)
            .unwrap();

        let first = h
            .service
            .recover_lost(&lost, "silence".into(), 300, 3)
            .unwrap();
        let second = h
            .service
            .recover_lost(&lost, "silence".into(), 400, 3)
            .unwrap();
        let a = first.replacement.expect("expected a replacement");
        let b = second.replacement.expect("expected a replacement");
        assert_eq!(first.record.retry_count, second.record.retry_count);
        // Same derived id, no second row.
        assert_eq!(a.id, b.id);
        let created = h.attempts.list_in_state(AttemptState::Created).unwrap();
        assert_eq!(created.len(), 1);
    }

    #[test]
    fn recover_lost_dead_letters_past_the_bound_and_creates_nothing() {
        let h = harness();
        let first = running(&h, "first", 100);
        h.attempts
            .transition(&first.id, AttemptState::Lost, 200)
            .unwrap();
        let replacement_id = h
            .service
            .recover_lost(&first, "silence".into(), 300, 1)
            .unwrap()
            .replacement
            .expect("expected a replacement")
            .id;
        // The replacement itself is later lost with a new failure.
        h.attempts
            .transition(&replacement_id, AttemptState::Lost, 500)
            .unwrap();
        let replacement = h.attempts.get(&replacement_id).unwrap().unwrap();

        let outcome = h
            .service
            .recover_lost(&replacement, "silence again".into(), 600, 1)
            .unwrap();
        assert!(outcome.replacement.is_none());
        assert_eq!(outcome.record.retry_count, 2);
        assert_eq!(outcome.record.disposition, RecoveryDisposition::DeadLettered);
        // Nothing new was created: only the two lost attempts exist.
        assert!(h.attempts.list_in_state(AttemptState::Created).unwrap().is_empty());
        assert_eq!(h.attempts.list_in_state(AttemptState::Lost).unwrap().len(), 2);
        // The dead letter is visible to operators.
        assert_eq!(h.recovery.dead_letters().unwrap().len(), 1);
    }

    #[test]
    fn recovery_chains_from_the_liveness_monitor() {
        let h = harness();
        running(&h, "stale", 100);

        let reaped = h.monitor.reap_stale(200, 60).unwrap();
        assert_eq!(reaped.len(), 1);
        let outcome = h
            .service
            .recover_lost(&reaped[0], "liveness window expired".into(), 300, 3)
            .unwrap();
        let replacement = outcome.replacement.expect("expected a replacement");
        assert_eq!(replacement.state, AttemptState::Created);
        // The monitor did not touch the replacement (fresh, not stale).
        assert!(h.monitor.reap_stale(400, 60).unwrap().is_empty());
    }
}
