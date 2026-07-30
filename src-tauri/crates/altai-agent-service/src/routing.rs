//! Host-neutral foreground run admission, ordering, and stale-command checks.
//!
//! A host owns the actual IsanAgent channel, but it must use this one state
//! machine before admitting input or delivering an event.  Keeping the lease
//! here makes the invariants identical for Desktop and the persistent stdio
//! host without leaking a host prefix (such as `tauri`) into run identities.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex, MutexGuard};

pub type SharedRunCoordinator = Arc<Mutex<RunCoordinator>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunPhase {
    Admitted,
    Running,
    WaitingUser,
    CancellingBeforeStart,
    CancellingRunning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunAdmission {
    New,
    ExistingReply,
    Queued,
    Confirmed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunTransitionError {
    ActiveLease,
    MissingLease,
    RunMismatch,
    OwnerMismatch,
    OwnerDraining,
    InvalidPhase,
}

#[derive(Clone, Default)]
pub struct RunCoordinator {
    active: HashMap<String, ActiveRun>,
    pending: HashMap<String, VecDeque<PendingRun>>,
    draining_owners: HashSet<String>,
}

#[derive(Clone)]
struct ActiveRun {
    run_id: String,
    owner_id: String,
    next_seq: u64,
    phase: RunPhase,
}

#[derive(Clone)]
struct PendingRun {
    run_id: String,
    owner_id: String,
}

impl RunCoordinator {
    pub fn admit(
        &mut self,
        chat_id: &str,
        run_id: &str,
        owner_id: &str,
    ) -> Result<(), RunTransitionError> {
        if self.draining_owners.contains(owner_id) {
            return Err(RunTransitionError::OwnerDraining);
        }
        if self.active.contains_key(chat_id) {
            return Err(RunTransitionError::ActiveLease);
        }
        self.active.insert(
            chat_id.to_string(),
            ActiveRun {
                run_id: run_id.to_string(),
                owner_id: owner_id.to_string(),
                next_seq: 1,
                phase: RunPhase::Admitted,
            },
        );
        Ok(())
    }

    pub fn admit_user_message(
        &mut self,
        chat_id: &str,
        run_id: &str,
        owner_id: &str,
    ) -> Result<RunAdmission, RunTransitionError> {
        if let Some(active) = self.active.get(chat_id) {
            return if active.owner_id == owner_id && active.phase == RunPhase::WaitingUser {
                Ok(RunAdmission::ExistingReply)
            } else {
                Err(RunTransitionError::ActiveLease)
            };
        }
        self.admit(chat_id, run_id, owner_id)?;
        Ok(RunAdmission::New)
    }

    pub fn admit_or_queue(
        &mut self,
        chat_id: &str,
        run_id: &str,
        owner_id: &str,
    ) -> Result<RunAdmission, RunTransitionError> {
        let Some(active) = self.active.get(chat_id) else {
            self.admit(chat_id, run_id, owner_id)?;
            return Ok(RunAdmission::New);
        };
        if active.owner_id != owner_id {
            return Err(RunTransitionError::OwnerMismatch);
        }
        if active.phase == RunPhase::WaitingUser {
            return Err(RunTransitionError::InvalidPhase);
        }
        if active.run_id == run_id
            && matches!(
                active.phase,
                RunPhase::Admitted | RunPhase::CancellingBeforeStart
            )
        {
            return Ok(RunAdmission::Confirmed);
        }
        let pending = self.pending.entry(chat_id.to_string()).or_default();
        if pending
            .iter()
            .any(|run| run.run_id == run_id && run.owner_id == owner_id)
        {
            return Ok(RunAdmission::Confirmed);
        }
        pending.push_back(PendingRun {
            run_id: run_id.to_string(),
            owner_id: owner_id.to_string(),
        });
        Ok(RunAdmission::Queued)
    }

    pub fn started(
        &mut self,
        chat_id: &str,
        run_id: &str,
        owner_id: &str,
    ) -> Result<(String, u64), RunTransitionError> {
        let active = self
            .active
            .get_mut(chat_id)
            .ok_or(RunTransitionError::MissingLease)?;
        if active.run_id != run_id {
            return Err(RunTransitionError::RunMismatch);
        }
        if active.owner_id != owner_id {
            return Err(RunTransitionError::OwnerMismatch);
        }
        active.phase = match active.phase {
            RunPhase::Admitted => RunPhase::Running,
            RunPhase::CancellingBeforeStart => RunPhase::CancellingRunning,
            _ => return Err(RunTransitionError::InvalidPhase),
        };
        active.next_seq = 2;
        Ok((run_id.to_string(), 1))
    }

    pub fn next(
        &mut self,
        chat_id: &str,
        owner_id: &str,
    ) -> Result<(String, u64), RunTransitionError> {
        let active = self
            .active
            .get_mut(chat_id)
            .ok_or(RunTransitionError::MissingLease)?;
        if active.owner_id != owner_id {
            return Err(RunTransitionError::OwnerMismatch);
        }
        if !matches!(
            active.phase,
            RunPhase::Running | RunPhase::WaitingUser | RunPhase::CancellingRunning
        ) {
            return Err(RunTransitionError::InvalidPhase);
        }
        if active.phase == RunPhase::WaitingUser {
            active.phase = RunPhase::Running;
        }
        let seq = active.next_seq;
        active.next_seq = active.next_seq.saturating_add(1);
        Ok((active.run_id.clone(), seq))
    }

    pub fn next_for_run(
        &mut self,
        chat_id: &str,
        run_id: &str,
        owner_id: &str,
    ) -> Result<(String, u64), RunTransitionError> {
        let active = self
            .active
            .get(chat_id)
            .ok_or(RunTransitionError::MissingLease)?;
        if active.run_id != run_id {
            return Err(RunTransitionError::RunMismatch);
        }
        self.next(chat_id, owner_id)
    }

    pub fn cancel_requested(
        &mut self,
        chat_id: &str,
        expected_run_id: Option<&str>,
    ) -> Result<String, RunTransitionError> {
        let active = self
            .active
            .get_mut(chat_id)
            .ok_or(RunTransitionError::MissingLease)?;
        if expected_run_id.is_some_and(|run_id| run_id != active.run_id) {
            return Err(RunTransitionError::RunMismatch);
        }
        self.pending.remove(chat_id);
        active.phase = match active.phase {
            RunPhase::Admitted => RunPhase::CancellingBeforeStart,
            RunPhase::Running | RunPhase::WaitingUser => RunPhase::CancellingRunning,
            _ => return Err(RunTransitionError::InvalidPhase),
        };
        Ok(active.run_id.clone())
    }

    pub fn active_run(&self, chat_id: &str) -> Option<(&str, &str)> {
        self.active
            .get(chat_id)
            .map(|run| (run.run_id.as_str(), run.owner_id.as_str()))
    }

    pub fn accepts_steer(
        &self,
        chat_id: &str,
        run_id: &str,
        owner_id: &str,
    ) -> Result<(), RunTransitionError> {
        let active = self
            .active
            .get(chat_id)
            .ok_or(RunTransitionError::MissingLease)?;
        if active.run_id != run_id {
            return Err(RunTransitionError::RunMismatch);
        }
        if active.owner_id != owner_id {
            return Err(RunTransitionError::OwnerMismatch);
        }
        if active.phase != RunPhase::Running {
            return Err(RunTransitionError::InvalidPhase);
        }
        Ok(())
    }

    pub fn begin_draining(
        &mut self,
        owner_ids: &HashSet<String>,
    ) -> Result<(), RunTransitionError> {
        if self
            .active
            .values()
            .any(|run| owner_ids.contains(&run.owner_id))
        {
            return Err(RunTransitionError::ActiveLease);
        }
        self.draining_owners.extend(owner_ids.iter().cloned());
        Ok(())
    }

    pub fn end_draining(&mut self, owner_ids: &HashSet<String>) {
        self.draining_owners
            .retain(|owner_id| !owner_ids.contains(owner_id));
    }

    pub fn terminated(
        &mut self,
        chat_id: &str,
        run_id: &str,
        owner_id: &str,
    ) -> Result<(String, u64), RunTransitionError> {
        let active = self
            .active
            .get(chat_id)
            .ok_or(RunTransitionError::MissingLease)?;
        if active.run_id != run_id {
            return Err(RunTransitionError::RunMismatch);
        }
        if active.owner_id != owner_id {
            return Err(RunTransitionError::OwnerMismatch);
        }
        if !matches!(
            active.phase,
            RunPhase::Running | RunPhase::WaitingUser | RunPhase::CancellingRunning
        ) {
            return Err(RunTransitionError::InvalidPhase);
        }
        let seq = active.next_seq;
        self.active.remove(chat_id);
        self.promote_next(chat_id);
        Ok((run_id.to_string(), seq))
    }

    pub fn mark_waiting_user(
        &mut self,
        chat_id: &str,
        owner_id: &str,
    ) -> Result<(), RunTransitionError> {
        let active = self
            .active
            .get_mut(chat_id)
            .ok_or(RunTransitionError::MissingLease)?;
        if active.owner_id != owner_id {
            return Err(RunTransitionError::OwnerMismatch);
        }
        if active.phase != RunPhase::Running {
            return Err(RunTransitionError::InvalidPhase);
        }
        active.phase = RunPhase::WaitingUser;
        Ok(())
    }

    pub fn rollback_admission(&mut self, chat_id: &str, run_id: &str, owner_id: &str) {
        let should_remove = self.active.get(chat_id).is_some_and(|active| {
            active.run_id == run_id
                && active.owner_id == owner_id
                && matches!(
                    active.phase,
                    RunPhase::Admitted | RunPhase::CancellingBeforeStart
                )
        });
        if should_remove {
            self.active.remove(chat_id);
            self.promote_next(chat_id);
            return;
        }
        if let Some(pending) = self.pending.get_mut(chat_id) {
            pending.retain(|run| run.run_id != run_id || run.owner_id != owner_id);
            if pending.is_empty() {
                self.pending.remove(chat_id);
            }
        }
    }

    fn promote_next(&mut self, chat_id: &str) {
        let next = self.pending.get_mut(chat_id).and_then(VecDeque::pop_front);
        if self.pending.get(chat_id).is_some_and(VecDeque::is_empty) {
            self.pending.remove(chat_id);
        }
        if let Some(next) = next {
            self.active.insert(
                chat_id.to_string(),
                ActiveRun {
                    run_id: next.run_id,
                    owner_id: next.owner_id,
                    next_seq: 1,
                    phase: RunPhase::Admitted,
                },
            );
        }
    }
}

pub fn coordinator_guard(coordinator: &SharedRunCoordinator) -> MutexGuard<'_, RunCoordinator> {
    coordinator
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub fn admit_run(
    coordinator: &SharedRunCoordinator,
    chat_id: &str,
    run_id: &str,
    owner_id: &str,
) -> Result<(), String> {
    coordinator_guard(coordinator)
        .admit(chat_id, run_id, owner_id)
        .map_err(|error| format!("Cannot start a second run for chat {chat_id}: {error:?}"))
}

pub fn admit_user_message(
    coordinator: &SharedRunCoordinator,
    chat_id: &str,
    run_id: &str,
    owner_id: &str,
) -> Result<String, String> {
    let mut coordinator = coordinator_guard(coordinator);
    match coordinator
        .admit_user_message(chat_id, run_id, owner_id)
        .map_err(|error| format!("Cannot accept user input for chat {chat_id}: {error:?}"))?
    {
        RunAdmission::New => Ok(run_id.to_string()),
        RunAdmission::ExistingReply => coordinator
            .active_run(chat_id)
            .map(|(active_run_id, _)| active_run_id.to_string())
            .ok_or_else(|| format!("The active run for chat {chat_id} disappeared")),
        admission => Err(format!(
            "Unexpected direct-message admission for chat {chat_id}: {admission:?}"
        )),
    }
}

pub fn admit_queued_user_message(
    coordinator: &SharedRunCoordinator,
    chat_id: &str,
    run_id: &str,
    owner_id: &str,
) -> Result<(String, bool), String> {
    match coordinator_guard(coordinator)
        .admit_or_queue(chat_id, run_id, owner_id)
        .map_err(|error| format!("Cannot queue run for chat {chat_id}: {error:?}"))?
    {
        RunAdmission::New | RunAdmission::Confirmed => Ok((run_id.to_string(), false)),
        RunAdmission::Queued => Ok((run_id.to_string(), true)),
        RunAdmission::ExistingReply => Err(format!(
            "Unexpected queued-message admission for chat {chat_id}: ExistingReply"
        )),
    }
}

pub fn queue_run(
    coordinator: &SharedRunCoordinator,
    chat_id: &str,
    run_id: &str,
    owner_id: &str,
) -> Result<RunAdmission, String> {
    coordinator_guard(coordinator)
        .admit_or_queue(chat_id, run_id, owner_id)
        .map_err(|error| format!("Cannot queue run for chat {chat_id}: {error:?}"))
}

pub fn rollback_run_admission(
    coordinator: &SharedRunCoordinator,
    chat_id: &str,
    run_id: &str,
    owner_id: &str,
) {
    coordinator_guard(coordinator).rollback_admission(chat_id, run_id, owner_id);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_cancel_and_steer_cannot_target_a_newer_run() {
        let mut coordinator = RunCoordinator::default();
        coordinator.admit("chat", "run-1", "owner").unwrap();
        coordinator.started("chat", "run-1", "owner").unwrap();
        assert_eq!(
            coordinator.cancel_requested("chat", Some("stale")),
            Err(RunTransitionError::RunMismatch)
        );
        assert_eq!(
            coordinator.accepts_steer("chat", "stale", "owner"),
            Err(RunTransitionError::RunMismatch)
        );
        assert_eq!(
            coordinator.next("chat", "owner"),
            Ok(("run-1".to_string(), 2))
        );
    }

    #[test]
    fn terminal_event_promotes_a_queued_run_with_a_fresh_sequence() {
        let mut coordinator = RunCoordinator::default();
        coordinator.admit("chat", "run-1", "owner").unwrap();
        coordinator.started("chat", "run-1", "owner").unwrap();
        assert_eq!(
            coordinator.admit_or_queue("chat", "run-2", "owner"),
            Ok(RunAdmission::Queued)
        );
        assert_eq!(
            coordinator.terminated("chat", "run-1", "owner"),
            Ok(("run-1".to_string(), 2))
        );
        assert_eq!(
            coordinator.started("chat", "run-2", "owner"),
            Ok(("run-2".to_string(), 1))
        );
    }
}
