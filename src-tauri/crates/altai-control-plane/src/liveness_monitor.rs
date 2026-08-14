//! Attempt liveness monitoring — the detection half of crash/restart recovery.
//! [`LivenessMonitor::reap_stale`] finds attempts in an owned state
//! (`Claimed`, `Dispatched`, `Running`) whose `updated_at_unix_seconds` has not
//! advanced within the staleness window and transitions them to `Lost`,
//! returning what it reaped so the caller retains the trigger evidence.
//! Ownership and explainability are preserved by construction: the attempt row
//! is never deleted or reassigned — it terminates as `Lost` still naming its
//! original owner, and recovery creates a *new* attempt rather than reopening
//! the lost one. The re-enactment (requeue with bounded retries via the
//! package-023 recovery store) is the follow-up recovery pass's job.

use std::sync::Arc;

use altai_control_protocol::{Attempt, AttemptState};

use crate::{AttemptError, AttemptRepository};

/// States where an owner exists and can go silent. `Created` is excluded —
/// nobody owns it yet, so there is no owner to lose.
const MONITORED_STATES: [AttemptState; 3] = [
    AttemptState::Claimed,
    AttemptState::Dispatched,
    AttemptState::Running,
];

pub struct LivenessMonitor {
    attempts: Arc<dyn AttemptRepository>,
}

impl LivenessMonitor {
    pub fn new(attempts: Arc<dyn AttemptRepository>) -> Self {
        Self { attempts }
    }

    /// Transition every stale owned attempt to `Lost`. An attempt is stale when
    /// `now - updated_at >= stale_after_seconds`. Returns the reaped attempts
    /// (post-transition, still naming their original owner) ordered by id.
    /// Idempotent: a reaped attempt leaves the monitored states.
    pub fn reap_stale(
        &self,
        now_unix_seconds: u64,
        stale_after_seconds: u64,
    ) -> Result<Vec<Attempt>, LivenessError> {
        let mut reaped = Vec::new();
        for state in MONITORED_STATES {
            for attempt in self
                .attempts
                .list_in_state(state)
                .map_err(LivenessError::Attempt)?
            {
                let age = now_unix_seconds.saturating_sub(attempt.updated_at_unix_seconds);
                if age < stale_after_seconds {
                    continue;
                }
                let lost = self
                    .attempts
                    .transition(&attempt.id, AttemptState::Lost, now_unix_seconds)
                    .map_err(LivenessError::Attempt)?;
                reaped.push(lost);
            }
        }
        reaped.sort_by(|a, b| a.id.value.cmp(&b.id.value));
        Ok(reaped)
    }
}

#[derive(Debug)]
pub enum LivenessError {
    Attempt(AttemptError),
}

impl std::fmt::Display for LivenessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Attempt(e) => write!(f, "liveness attempt failure: {e}"),
        }
    }
}
impl std::error::Error for LivenessError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SqliteAttemptRepository;
    use altai_control_protocol::{
        AgentInstanceId, AgentProfileRevisionId, AttemptId, WorkItemId,
    };

    struct Harness {
        _dir: tempfile::TempDir,
        monitor: LivenessMonitor,
        attempts: Arc<SqliteAttemptRepository>,
    }

    fn harness() -> Harness {
        let dir = tempfile::tempdir().unwrap();
        let attempts = Arc::new(SqliteAttemptRepository::open(&dir.path().join("work.db")).unwrap());
        let monitor = LivenessMonitor::new(attempts.clone());
        Harness {
            _dir: dir,
            monitor,
            attempts,
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

    /// Create `id` and walk it into `target` state, stamping `updated_at`.
    fn seed(h: &Harness, id: &str, target: AttemptState, updated_at: u64) -> Attempt {
        let attempt = attempt(id);
        h.attempts.create(attempt.clone()).unwrap();
        let chain: &[AttemptState] = match target {
            AttemptState::Claimed => &[AttemptState::Claimed],
            AttemptState::Dispatched => &[AttemptState::Claimed, AttemptState::Dispatched],
            AttemptState::Running => &[
                AttemptState::Claimed,
                AttemptState::Dispatched,
                AttemptState::Running,
            ],
            other => panic!("unsupported seed target: {other:?}"),
        };
        let mut current = attempt;
        for (step, to) in chain.iter().enumerate() {
            current = h.attempts.transition(&current.id, *to, updated_at - step as u64).unwrap();
        }
        // The final transition stamps updated_at; earlier steps stay earlier.
        if current.updated_at_unix_seconds != updated_at {
            current = h.attempts
                .transition(&current.id, target, updated_at)
                .unwrap();
        }
        current
    }

    const WINDOW: u64 = 60;

    #[test]
    fn reaps_a_running_attempt_past_the_staleness_window() {
        let h = harness();
        seed(&h, "att", AttemptState::Running, 100);

        let reaped = h.monitor.reap_stale(200, WINDOW).unwrap();
        assert_eq!(reaped.len(), 1);
        assert_eq!(reaped[0].id, AttemptId::new("att"));
        assert_eq!(reaped[0].state, AttemptState::Lost);
        // Ownership preserved: the lost attempt still names its original owner.
        assert_eq!(reaped[0].owner_agent_instance_id, AgentInstanceId::new("owner-a"));
        // Durable.
        assert_eq!(
            h.attempts.get(&AttemptId::new("att")).unwrap().unwrap().state,
            AttemptState::Lost
        );
    }

    #[test]
    fn keeps_a_fresh_attempt_running() {
        let h = harness();
        seed(&h, "att", AttemptState::Running, 150);

        let reaped = h.monitor.reap_stale(200, WINDOW).unwrap();
        assert!(reaped.is_empty());
        assert_eq!(
            h.attempts.get(&AttemptId::new("att")).unwrap().unwrap().state,
            AttemptState::Running
        );
    }

    #[test]
    fn boundary_age_equal_to_the_window_is_stale() {
        let h = harness();
        seed(&h, "att", AttemptState::Running, 140);

        // age == 60 == WINDOW reaps (>= semantics).
        let reaped = h.monitor.reap_stale(200, WINDOW).unwrap();
        assert_eq!(reaped.len(), 1);
    }

    #[test]
    fn reaps_claimed_and_dispatched_but_not_created() {
        let h = harness();
        seed(&h, "claimed", AttemptState::Claimed, 100);
        seed(&h, "dispatched", AttemptState::Dispatched, 100);
        // Created sits untouched even though it is older than the window.
        h.attempts.create(attempt("created")).unwrap();

        let reaped = h.monitor.reap_stale(200, WINDOW).unwrap();
        let ids: Vec<&str> = reaped.iter().map(|a| a.id.value.as_str()).collect();
        assert_eq!(ids, vec!["att_claimed", "att_dispatched"]);
        assert_eq!(
            h.attempts.get(&AttemptId::new("created")).unwrap().unwrap().state,
            AttemptState::Created
        );
    }

    #[test]
    fn never_reaps_a_terminal_attempt() {
        let h = harness();
        let mut running = seed(&h, "att", AttemptState::Running, 100);
        running = h.attempts.transition(&running.id, AttemptState::Succeeded, 120).unwrap();

        let reaped = h.monitor.reap_stale(10_000, WINDOW).unwrap();
        assert!(reaped.is_empty());
        assert_eq!(
            h.attempts.get(&running.id).unwrap().unwrap().state,
            AttemptState::Succeeded
        );
    }

    #[test]
    fn reap_stale_is_idempotent() {
        let h = harness();
        seed(&h, "att", AttemptState::Running, 100);

        assert_eq!(h.monitor.reap_stale(200, WINDOW).unwrap().len(), 1);
        // Second pass sees nothing: the attempt left the monitored states.
        assert!(h.monitor.reap_stale(400, WINDOW).unwrap().is_empty());
    }

    #[test]
    fn reaped_attempts_are_ordered_by_id() {
        let h = harness();
        seed(&h, "z", AttemptState::Running, 100);
        seed(&h, "a", AttemptState::Dispatched, 100);
        seed(&h, "m", AttemptState::Claimed, 100);

        let reaped = h.monitor.reap_stale(200, WINDOW).unwrap();
        let ids: Vec<&str> = reaped.iter().map(|a| a.id.value.as_str()).collect();
        assert_eq!(ids, vec!["att_a", "att_m", "att_z"]);
    }
}
