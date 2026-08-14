//! Evidence-gated Work completion. [`CompletionGate::complete`] consumes a
//! terminal attempt and moves a Work item to `Done` only when its requirements
//! are met: the named attempt succeeded and belongs to the work item, and the
//! work item has recorded evidence. This is the "consume the terminal Attempt to
//! move Work disposition explicitly" step that `finalize_attempt` (package 035)
//! defers — finalization produces the verification signal; the gate moves Work.
//! When a requirement is unmet the gate returns `Blocked` with the reasons and
//! leaves Work untouched; already-`Done` is idempotent. Nothing here governs
//! delivery actions.

use std::sync::Arc;

use altai_control_protocol::{AttemptId, AttemptState, ExecutionPhase, WorkItemId, WorkStatus};

use crate::{
    AttemptError, AttemptRepository, EvidenceError, EvidenceRepository, WorkItemRepository,
    WorkItemRepositoryError,
};

pub struct CompletionGate {
    work_items: Arc<dyn WorkItemRepository>,
    attempts: Arc<dyn AttemptRepository>,
    evidence: Arc<dyn EvidenceRepository>,
}

impl CompletionGate {
    pub fn new(
        work_items: Arc<dyn WorkItemRepository>,
        attempts: Arc<dyn AttemptRepository>,
        evidence: Arc<dyn EvidenceRepository>,
    ) -> Self {
        Self {
            work_items,
            attempts,
            evidence,
        }
    }

    /// Complete `work_item_id` against its succeeded `attempt_id`. Returns
    /// [`CompletionOutcome::Completed`] (transitioning Work to `Done`/`Terminal`)
    /// when a succeeded, matching attempt with recorded evidence exists;
    /// [`CompletionOutcome::Blocked`] with the unmet reasons otherwise, leaving
    /// Work unchanged. Already-`Done` is idempotent.
    pub fn complete(
        &self,
        work_item_id: &WorkItemId,
        attempt_id: &AttemptId,
        updated_at: String,
    ) -> Result<CompletionOutcome, CompletionError> {
        let work = self
            .work_items
            .get(work_item_id)
            .map_err(CompletionError::WorkItem)?;
        if work.status == WorkStatus::Done {
            return Ok(CompletionOutcome::Completed);
        }
        let attempt = self
            .attempts
            .get(attempt_id)
            .map_err(CompletionError::Attempt)?;
        let mut blockers = Vec::new();
        let succeeded = attempt
            .as_ref()
            .map(|a| a.state == AttemptState::Succeeded && a.work_item_id == *work_item_id)
            .unwrap_or(false);
        if !succeeded {
            blockers.push(CompletionBlocker::AttemptNotSucceeded {
                attempt_id: attempt_id.clone(),
            });
        }
        let has_evidence = !self
            .evidence
            .list_for_work(work_item_id)
            .map_err(CompletionError::Evidence)?
            .is_empty();
        if !has_evidence {
            blockers.push(CompletionBlocker::MissingEvidence {
                work_item_id: work_item_id.clone(),
            });
        }
        if !blockers.is_empty() {
            return Ok(CompletionOutcome::Blocked { blockers });
        }
        let mut next = work.clone();
        next.status = WorkStatus::Done;
        next.execution_phase = ExecutionPhase::Terminal;
        next.revision = work.revision.next();
        next.updated_at = updated_at;
        self.work_items
            .replace_if_revision(next, work.revision)
            .map_err(CompletionError::WorkItem)?;
        Ok(CompletionOutcome::Completed)
    }
}

#[derive(Debug)]
pub enum CompletionOutcome {
    /// Work was (or already is) `Done`. Read the item back from the repository.
    Completed,
    /// A requirement was unmet; Work is unchanged. `blockers` lists every reason.
    Blocked { blockers: Vec<CompletionBlocker> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionBlocker {
    /// No succeeded attempt for this work item under the given id.
    AttemptNotSucceeded { attempt_id: AttemptId },
    /// The work item has no recorded evidence.
    MissingEvidence { work_item_id: WorkItemId },
}

#[derive(Debug)]
pub enum CompletionError {
    WorkItem(WorkItemRepositoryError),
    Attempt(AttemptError),
    Evidence(EvidenceError),
}

impl std::fmt::Display for CompletionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WorkItem(e) => write!(f, "completion work-item failure: {e}"),
            Self::Attempt(e) => write!(f, "completion attempt failure: {e}"),
            Self::Evidence(e) => write!(f, "completion evidence failure: {e}"),
        }
    }
}
impl std::error::Error for CompletionError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ScopeRepository, SqliteAttemptRepository, SqliteEvidenceRepository, SqliteScopeRepository,
        SqliteWorkItemRepository,
    };
    use altai_control_protocol::{
        AgentInstanceId, AgentProfileRevisionId, Attempt, AttemptState, Evidence, EvidenceId,
        Organization, OrganizationId, Project, ProjectId, ProjectStatus, Revision, WorkItem,
        WorkItemKind,
    };

    struct Harness {
        _dir: tempfile::TempDir,
        gate: CompletionGate,
        work_items: Arc<SqliteWorkItemRepository>,
        attempts: Arc<SqliteAttemptRepository>,
        evidence: Arc<SqliteEvidenceRepository>,
        work_id: WorkItemId,
    }

    fn harness(work_status: WorkStatus) -> Harness {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("work.db");
        let scopes = SqliteScopeRepository::open(&db).unwrap();
        let org = Organization {
            id: OrganizationId::new("org"),
            name: "Org".into(),
            revision: Revision::INITIAL,
            created_at: "now".into(),
            updated_at: "now".into(),
        };
        scopes.create_organization(org).unwrap();
        let project = Project {
            id: ProjectId::new("project"),
            organization_id: OrganizationId::new("org"),
            goal_ids: vec![],
            name: "Project".into(),
            description: String::new(),
            status: ProjectStatus::Active,
            revision: Revision::INITIAL,
            created_at: "now".into(),
            updated_at: "now".into(),
        };
        scopes.create_project(project).unwrap();

        let work_id = WorkItemId::new("work");
        let work_items = Arc::new(SqliteWorkItemRepository::open(&db).unwrap());
        work_items
            .create(WorkItem {
                id: work_id.clone(),
                project_id: ProjectId::new("project"),
                goal_id: None,
                parent_work_item_id: None,
                kind: WorkItemKind::Task,
                title: "Ship".into(),
                description: String::new(),
                status: work_status,
                execution_phase: ExecutionPhase::Running,
                revision: Revision::INITIAL,
                created_at: "now".into(),
                updated_at: "now".into(),
            })
            .unwrap();
        let attempts = Arc::new(SqliteAttemptRepository::open(&db).unwrap());
        let evidence = Arc::new(SqliteEvidenceRepository::open(&db).unwrap());
        let gate = CompletionGate::new(
            work_items.clone(),
            attempts.clone(),
            evidence.clone(),
        );
        Harness {
            _dir: dir,
            gate,
            work_items,
            attempts,
            evidence,
            work_id,
        }
    }

    fn attempt(attempt_id: &str, work_id: &WorkItemId, state: AttemptState) -> Attempt {
        Attempt {
            id: AttemptId::new(attempt_id),
            work_item_id: work_id.clone(),
            owner_agent_instance_id: AgentInstanceId::new("ai"),
            profile_revision_id: AgentProfileRevisionId::new("apr"),
            state,
            created_at_unix_seconds: 1,
            updated_at_unix_seconds: 2,
        }
    }

    /// Seed an attempt in `target` state by walking the real state machine
    /// (create requires `Created`; transitions are validated).
    fn seed_attempt(h: &Harness, attempt_id: &str, work_id: &WorkItemId, target: AttemptState) {
        let id = AttemptId::new(attempt_id);
        h.attempts
            .create(attempt(attempt_id, work_id, AttemptState::Created))
            .unwrap();
        let chain: &[AttemptState] = match target {
            AttemptState::Running => &[
                AttemptState::Claimed,
                AttemptState::Dispatched,
                AttemptState::Running,
            ],
            AttemptState::Succeeded => &[
                AttemptState::Claimed,
                AttemptState::Dispatched,
                AttemptState::Running,
                AttemptState::Succeeded,
            ],
            other => panic!("unsupported seed target: {other:?}"),
        };
        for to in chain {
            h.attempts.transition(&id, *to, 2).unwrap();
        }
    }

    fn evidence_for(work_id: &WorkItemId) -> Evidence {
        Evidence {
            id: EvidenceId::new("ev1"),
            organization_id: OrganizationId::new("org"),
            work_item_id: work_id.clone(),
            attempt_id: AttemptId::new("att"),
            kind: "artifact_ref".into(),
            reference: "out/diff.patch".into(),
            created_at_unix_seconds: 3,
        }
    }

    #[test]
    fn complete_moves_work_to_done_when_succeeded_attempt_has_evidence() {
        let h = harness(WorkStatus::InProgress);
        seed_attempt(&h, "att", &h.work_id, AttemptState::Succeeded);
        h.evidence.record(evidence_for(&h.work_id)).unwrap();

        let outcome = h.gate.complete(&h.work_id, &AttemptId::new("att"), "now2".into()).unwrap();
        assert!(matches!(outcome, CompletionOutcome::Completed));
        // Durable: the transition persisted with the bumped revision.
        let stored = h.work_items.get(&h.work_id).unwrap();
        assert_eq!(stored.status, WorkStatus::Done);
        assert_eq!(stored.execution_phase, ExecutionPhase::Terminal);
        assert_eq!(stored.revision, Revision::new(1));
        assert_eq!(stored.updated_at, "now2");
    }

    #[test]
    fn complete_blocks_when_attempt_not_succeeded() {
        let h = harness(WorkStatus::InProgress);
        seed_attempt(&h, "att", &h.work_id, AttemptState::Running);
        h.evidence.record(evidence_for(&h.work_id)).unwrap();

        let outcome = h.gate.complete(&h.work_id, &AttemptId::new("att"), "now2".into()).unwrap();
        match outcome {
            CompletionOutcome::Blocked { blockers } => {
                assert_eq!(
                    blockers,
                    vec![CompletionBlocker::AttemptNotSucceeded {
                        attempt_id: AttemptId::new("att")
                    }]
                );
            }
            other => panic!("expected Blocked, got {other:?}"),
        }
        // Work unchanged.
        assert_eq!(
            h.work_items.get(&h.work_id).unwrap().status,
            WorkStatus::InProgress
        );
    }

    #[test]
    fn complete_blocks_when_evidence_missing() {
        let h = harness(WorkStatus::InProgress);
        seed_attempt(&h, "att", &h.work_id, AttemptState::Succeeded);

        let outcome = h.gate.complete(&h.work_id, &AttemptId::new("att"), "now2".into()).unwrap();
        match outcome {
            CompletionOutcome::Blocked { blockers } => {
                assert_eq!(
                    blockers,
                    vec![CompletionBlocker::MissingEvidence {
                        work_item_id: h.work_id.clone()
                    }]
                );
            }
            other => panic!("expected Blocked, got {other:?}"),
        }
        assert_eq!(
            h.work_items.get(&h.work_id).unwrap().status,
            WorkStatus::InProgress
        );
    }

    #[test]
    fn complete_blocks_when_attempt_belongs_to_a_different_work_item() {
        let h = harness(WorkStatus::InProgress);
        // A succeeded attempt for a *different* work item.
        let other = WorkItemId::new("other");
        seed_attempt(&h, "att", &other, AttemptState::Succeeded);
        h.evidence.record(evidence_for(&h.work_id)).unwrap();

        let outcome = h.gate.complete(&h.work_id, &AttemptId::new("att"), "now2".into()).unwrap();
        assert!(matches!(
            outcome,
            CompletionOutcome::Blocked {
                blockers
            } if blockers.iter().any(|b| matches!(
                b,
                CompletionBlocker::AttemptNotSucceeded { .. }
            ))
        ));
    }

    #[test]
    fn complete_is_idempotent_when_already_done() {
        let h = harness(WorkStatus::Done);
        seed_attempt(&h, "att", &h.work_id, AttemptState::Succeeded);
        h.evidence.record(evidence_for(&h.work_id)).unwrap();

        let outcome = h.gate.complete(&h.work_id, &AttemptId::new("att"), "now2".into()).unwrap();
        assert!(matches!(outcome, CompletionOutcome::Completed));
        // No re-transition: revision stays at the seeded INITIAL.
        let stored = h.work_items.get(&h.work_id).unwrap();
        assert_eq!(stored.revision, Revision::INITIAL);
        assert_eq!(stored.updated_at, "now");
    }

    #[test]
    fn complete_collects_every_unmet_blocker() {
        let h = harness(WorkStatus::InProgress);
        // Neither succeeded nor any evidence.
        seed_attempt(&h, "att", &h.work_id, AttemptState::Running);

        let outcome = h.gate.complete(&h.work_id, &AttemptId::new("att"), "now2".into()).unwrap();
        match outcome {
            CompletionOutcome::Blocked { blockers } => {
                assert_eq!(blockers.len(), 2);
                assert!(blockers.contains(&CompletionBlocker::AttemptNotSucceeded {
                    attempt_id: AttemptId::new("att")
                }));
                assert!(blockers.contains(&CompletionBlocker::MissingEvidence {
                    work_item_id: h.work_id.clone()
                }));
            }
            other => panic!("expected Blocked, got {other:?}"),
        }
    }
}
