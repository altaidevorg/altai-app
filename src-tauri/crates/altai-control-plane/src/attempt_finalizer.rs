//! CP-08 attempt finalization. Translates one observed executor run termination
//! into exactly one durable Attempt terminal transition. The finalized Attempt is
//! the verification signal; finalization never completes or mutates the bound
//! WorkItem. That invariant is enforced by construction: `finalize_attempt`
//! holds only an [`AttemptRepository`] and has no handle to any WorkItem
//! repository, so it cannot reach Work disposition. A later governance/review
//! package consumes the terminal Attempt to move Work disposition explicitly.

use crate::attempt_repository::{AttemptError, AttemptRepository};
use altai_control_protocol::{Attempt, AttemptId, AttemptState};

/// Observed outcome of an executor run, translated into attempt state. This is
/// an execution observation, distinct from stored attempt state: keeping it
/// separate makes the run-to-attempt translation explicit, and every variant
/// maps, so there is no unknown-outcome path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunOutcome {
    Succeeded,
    Failed,
    Cancelled,
    TimedOut,
    BudgetStopped,
    PolicyDenied,
    Lost,
}

impl RunOutcome {
    /// The terminal attempt state this observation finalizes into.
    pub fn to_attempt_state(self) -> AttemptState {
        match self {
            Self::Succeeded => AttemptState::Succeeded,
            Self::Failed => AttemptState::Failed,
            Self::Cancelled => AttemptState::Cancelled,
            Self::TimedOut => AttemptState::TimedOut,
            Self::BudgetStopped => AttemptState::BudgetStopped,
            Self::PolicyDenied => AttemptState::PolicyDenied,
            Self::Lost => AttemptState::Lost,
        }
    }
}

/// One idempotent finalization request. `observed_at_unix_seconds` is the run
/// completion time supplied by the observer; the finalizer never invents "now".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttemptFinalization {
    pub attempt_id: AttemptId,
    pub outcome: RunOutcome,
    pub observed_at_unix_seconds: u64,
}

/// Finalization failure. `AlreadyTerminal` reports a divergent observation
/// after the attempt was already finalized: `state` is the winning first
/// observation, `attempted` is the divergent outcome that was rejected. The
/// first terminal observation wins and no state change occurs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttemptFinalizationError {
    NotFound { attempt_id: String },
    AlreadyTerminal { attempt_id: String, state: AttemptState, attempted: AttemptState },
    InvalidTransition { from: AttemptState, to: AttemptState },
    Internal { reason: String },
}

impl std::fmt::Display for AttemptFinalizationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "attempt finalization error: {self:?}")
    }
}
impl std::error::Error for AttemptFinalizationError {}

impl From<AttemptError> for AttemptFinalizationError {
    fn from(error: AttemptError) -> Self {
        match error {
            AttemptError::NotFound { attempt_id } => Self::NotFound { attempt_id },
            AttemptError::InvalidTransition { from, to } => Self::InvalidTransition { from, to },
            AttemptError::Conflict { attempt_id } => {
                Self::Internal { reason: format!("attempt conflict finalizing {attempt_id}") }
            }
            AttemptError::Internal { reason } => Self::Internal { reason },
        }
    }
}

/// Finalize an attempt from an observed run termination.
///
/// - If the attempt is not yet terminal, it transitions to the outcome's
///   terminal state using the observed completion time.
/// - Replaying the same outcome on an already-terminal attempt is an idempotent
///   no-op: the attempt is returned unchanged.
/// - A divergent outcome after the attempt is terminal fails closed as
///   [`AttemptFinalizationError::AlreadyTerminal`]; the first observation wins.
pub fn finalize_attempt(
    repo: &dyn AttemptRepository,
    finalization: &AttemptFinalization,
) -> Result<Attempt, AttemptFinalizationError> {
    let current = repo
        .get(&finalization.attempt_id)?
        .ok_or_else(|| AttemptFinalizationError::NotFound {
            attempt_id: finalization.attempt_id.value.clone(),
        })?;

    if current.state.is_terminal() {
        if current.state == finalization.outcome.to_attempt_state() {
            // Replay of the same completion: idempotent no-op.
            return Ok(current);
        }
        // First terminal observation wins; a divergent late observation fails closed.
        return Err(AttemptFinalizationError::AlreadyTerminal {
            attempt_id: current.id.value.clone(),
            state: current.state,
            attempted: finalization.outcome.to_attempt_state(),
        });
    }

    let finalized = repo.transition(
        &finalization.attempt_id,
        finalization.outcome.to_attempt_state(),
        finalization.observed_at_unix_seconds,
    )?;
    Ok(finalized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SqliteAttemptRepository;
    use altai_control_protocol::{AgentInstanceId, AgentProfileRevisionId, WorkItemId};
    use std::path::Path;

    fn repo_at(path: &Path) -> SqliteAttemptRepository {
        SqliteAttemptRepository::open(path).unwrap()
    }

    fn make_attempt(id: &str) -> Attempt {
        Attempt {
            id: AttemptId::new(id),
            work_item_id: WorkItemId::new("work_one"),
            owner_agent_instance_id: AgentInstanceId::new("agent_one"),
            profile_revision_id: AgentProfileRevisionId::new("rev_one"),
            state: AttemptState::Created,
            created_at_unix_seconds: 1,
            updated_at_unix_seconds: 1,
        }
    }

    fn seed_running(repo: &SqliteAttemptRepository, id: &str) -> Attempt {
        let attempt = make_attempt(id);
        repo.create(attempt.clone()).unwrap();
        repo.transition(&attempt.id, AttemptState::Claimed, 10).unwrap();
        repo.transition(&attempt.id, AttemptState::Dispatched, 20).unwrap();
        repo.transition(&attempt.id, AttemptState::Running, 30).unwrap()
    }

    fn fin(id: &str, outcome: RunOutcome, observed_at: u64) -> AttemptFinalization {
        AttemptFinalization {
            attempt_id: AttemptId::new(id),
            outcome,
            observed_at_unix_seconds: observed_at,
        }
    }

    #[test]
    fn outcome_maps_to_each_terminal_state() {
        let cases = [
            (RunOutcome::Succeeded, AttemptState::Succeeded),
            (RunOutcome::Failed, AttemptState::Failed),
            (RunOutcome::Cancelled, AttemptState::Cancelled),
            (RunOutcome::TimedOut, AttemptState::TimedOut),
            (RunOutcome::BudgetStopped, AttemptState::BudgetStopped),
            (RunOutcome::PolicyDenied, AttemptState::PolicyDenied),
            (RunOutcome::Lost, AttemptState::Lost),
        ];
        for (outcome, expected) in cases {
            let dir = tempfile::tempdir().unwrap();
            let repo = repo_at(&dir.path().join("work.db"));
            seed_running(&repo, "att");

            let finalized = finalize_attempt(
                &repo,
                &fin("att", outcome, 100),
            )
            .unwrap();

            assert_eq!(finalized.state, expected, "outcome {outcome:?} should map to {expected:?}");
            assert_eq!(finalized.updated_at_unix_seconds, 100);
        }
    }

    #[test]
    fn replay_of_same_completion_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let repo = repo_at(&dir.path().join("work.db"));
        seed_running(&repo, "att");

        let first = finalize_attempt(&repo, &fin("att", RunOutcome::Succeeded, 100)).unwrap();
        let second = finalize_attempt(&repo, &fin("att", RunOutcome::Succeeded, 200)).unwrap();

        assert_eq!(first, second);
        assert_eq!(second.updated_at_unix_seconds, 100, "replay must not overwrite observed time");
    }

    #[test]
    fn divergent_outcome_after_terminal_fails_closed() {
        let dir = tempfile::tempdir().unwrap();
        let repo = repo_at(&dir.path().join("work.db"));
        seed_running(&repo, "att");

        finalize_attempt(&repo, &fin("att", RunOutcome::Succeeded, 100)).unwrap();
        let err = finalize_attempt(&repo, &fin("att", RunOutcome::Failed, 200)).unwrap_err();

        assert_eq!(
            err,
            AttemptFinalizationError::AlreadyTerminal {
                attempt_id: AttemptId::new("att").value,
                state: AttemptState::Succeeded,
                attempted: AttemptState::Failed,
            }
        );
        assert_eq!(
            repo.get(&AttemptId::new("att")).unwrap().unwrap().state,
            AttemptState::Succeeded,
            "first terminal observation must win"
        );
    }

    #[test]
    fn unknown_attempt_is_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let repo = repo_at(&dir.path().join("work.db"));
        let expected_id = AttemptId::new("ghost").value;

        let err = finalize_attempt(&repo, &fin("ghost", RunOutcome::Succeeded, 100)).unwrap_err();

        assert!(matches!(
            err,
            AttemptFinalizationError::NotFound { ref attempt_id } if *attempt_id == expected_id
        ));
    }

    #[test]
    fn non_finalizable_source_state_rejects_terminal_transition() {
        let dir = tempfile::tempdir().unwrap();
        let repo = repo_at(&dir.path().join("work.db"));
        let attempt = make_attempt("att");
        repo.create(attempt).unwrap();
        repo.transition(&AttemptId::new("att"), AttemptState::Claimed, 10).unwrap();

        let err = finalize_attempt(&repo, &fin("att", RunOutcome::Succeeded, 100)).unwrap_err();

        assert!(matches!(
            err,
            AttemptFinalizationError::InvalidTransition {
                from: AttemptState::Claimed,
                to: AttemptState::Succeeded,
            }
        ));
    }

    #[test]
    fn cancel_finalizes_from_dispatched() {
        let dir = tempfile::tempdir().unwrap();
        let repo = repo_at(&dir.path().join("work.db"));
        let attempt = make_attempt("att");
        repo.create(attempt).unwrap();
        repo.transition(&AttemptId::new("att"), AttemptState::Claimed, 10).unwrap();
        repo.transition(&AttemptId::new("att"), AttemptState::Dispatched, 20).unwrap();

        let finalized = finalize_attempt(&repo, &fin("att", RunOutcome::Cancelled, 100)).unwrap();

        assert_eq!(finalized.state, AttemptState::Cancelled);
    }

    #[test]
    fn finalized_attempt_carries_work_item_verification_signal() {
        // The finalized Attempt carries the bound work_item_id so a downstream
        // review/governance package can signal verification. Finalization itself
        // never completes Work: finalize_attempt takes only an AttemptRepository
        // and cannot reach any WorkItem repository.
        let dir = tempfile::tempdir().unwrap();
        let repo = repo_at(&dir.path().join("work.db"));
        seed_running(&repo, "att");

        let finalized = finalize_attempt(&repo, &fin("att", RunOutcome::Succeeded, 100)).unwrap();

        assert_eq!(finalized.work_item_id, WorkItemId::new("work_one"));
        assert!(finalized.state.is_terminal());
    }
}
