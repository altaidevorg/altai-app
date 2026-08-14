//! Approval-gated delivery authorization. A delivery action for a Work item is
//! governed: [`DeliveryGate::authorize`] answers whether it may proceed by
//! reading the approval audit — an authorization requires an approval in the
//! same organization, scoped `Delivery` to that work item, decided `Approved`,
//! and granted for exactly the payload revision being delivered. Anything else
//! is `Blocked` with the reasons. The gate never mutates: it is the seam the
//! delivery executor (a later workspace package) is required to ask before
//! releasing output.

use std::sync::Arc;

use altai_control_protocol::{
    ApprovalId, ApprovalOutcome, ApprovalScope, OrganizationId, Revision, WorkItemId,
};

use crate::{ApprovalError, ApprovalRepository};

pub struct DeliveryGate {
    approvals: Arc<dyn ApprovalRepository>,
}

impl DeliveryGate {
    pub fn new(approvals: Arc<dyn ApprovalRepository>) -> Self {
        Self { approvals }
    }

    /// Decide whether delivering `payload_revision` of `work_item_id` for
    /// `organization_id` is authorized. `Authorized` names the governing
    /// approval; `Blocked` lists every reason no matching approval authorized
    /// the delivery.
    pub fn authorize(
        &self,
        organization_id: &OrganizationId,
        work_item_id: &WorkItemId,
        payload_revision: Revision,
    ) -> Result<DeliveryDecision, DeliveryError> {
        let matching = self
            .approvals
            .list_in_org(organization_id)
            .map_err(DeliveryError::Approval)?
            .into_iter()
            .filter(|approval| {
                matches!(
                    &approval.scope,
                    ApprovalScope::Delivery { work_item_id: id } if id == work_item_id
                )
            })
            .collect::<Vec<_>>();
        if matching.is_empty() {
            return Ok(DeliveryDecision::Blocked {
                blockers: vec![DeliveryBlocker::NoApproval {
                    work_item_id: work_item_id.clone(),
                }],
            });
        }
        let mut blockers = Vec::new();
        for approval in &matching {
            match approval.outcome {
                Some(ApprovalOutcome::Approved) => {
                    if approval.payload_revision == payload_revision {
                        return Ok(DeliveryDecision::Authorized {
                            approval_id: approval.id.clone(),
                        });
                    }
                    blockers.push(DeliveryBlocker::PayloadRevisionMismatch {
                        approval_id: approval.id.clone(),
                        approved: approval.payload_revision,
                        delivering: payload_revision,
                    });
                }
                Some(ApprovalOutcome::Denied) => {
                    blockers.push(DeliveryBlocker::ApprovalDenied {
                        approval_id: approval.id.clone(),
                    });
                }
                None => blockers.push(DeliveryBlocker::ApprovalPending {
                    approval_id: approval.id.clone(),
                }),
            }
        }
        Ok(DeliveryDecision::Blocked { blockers })
    }
}

#[derive(Debug)]
pub enum DeliveryDecision {
    /// A governing approval authorizes the delivery; names the approval.
    Authorized { approval_id: ApprovalId },
    /// No matching approval authorizes the delivery; lists every reason.
    Blocked { blockers: Vec<DeliveryBlocker> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeliveryBlocker {
    /// No delivery-scoped approval exists for the work item.
    NoApproval { work_item_id: WorkItemId },
    /// A governing approval exists but has not been decided.
    ApprovalPending { approval_id: ApprovalId },
    /// A governing approval was decided `Denied`.
    ApprovalDenied { approval_id: ApprovalId },
    /// The approval was granted for a different payload revision than the one
    /// being delivered.
    PayloadRevisionMismatch {
        approval_id: ApprovalId,
        approved: Revision,
        delivering: Revision,
    },
}

#[derive(Debug)]
pub enum DeliveryError {
    Approval(ApprovalError),
}

impl std::fmt::Display for DeliveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Approval(e) => write!(f, "delivery approval failure: {e}"),
        }
    }
}
impl std::error::Error for DeliveryError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SqliteApprovalRepository;
    use altai_control_protocol::{
        Approval, ApprovalDecision, ApprovalOutcome, ApprovalScope, AttemptId,
    };

    struct Harness {
        _dir: tempfile::TempDir,
        gate: DeliveryGate,
        approvals: Arc<SqliteApprovalRepository>,
        org: OrganizationId,
        work_id: WorkItemId,
    }

    fn harness() -> Harness {
        let dir = tempfile::tempdir().unwrap();
        let approvals = Arc::new(
            SqliteApprovalRepository::open(&dir.path().join("work.db")).unwrap(),
        );
        let gate = DeliveryGate::new(approvals.clone());
        Harness {
            _dir: dir,
            gate,
            approvals,
            org: OrganizationId::new("org"),
            work_id: WorkItemId::new("work"),
        }
    }

    fn approval(h: &Harness, id: &str, revision: Revision) -> Approval {
        Approval {
            id: ApprovalId::new(id),
            organization_id: h.org.clone(),
            scope: ApprovalScope::Delivery {
                work_item_id: h.work_id.clone(),
            },
            payload_revision: revision,
            outcome: None,
            revision: Revision::INITIAL,
            created_at_unix_seconds: 10,
            resolved_at_unix_seconds: None,
        }
    }

    fn decide(h: &Harness, id: &str, outcome: ApprovalOutcome) {
        h.approvals
            .record_decision(ApprovalDecision {
                approval_id: ApprovalId::new(id),
                outcome,
                decided_by: "principal".into(),
                decided_at_unix_seconds: 20,
                reason: None,
            })
            .unwrap();
    }

    #[test]
    fn authorize_allows_an_approved_matching_revision_delivery() {
        let h = harness();
        h.approvals.create(approval(&h, "apv", Revision::new(1))).unwrap();
        decide(&h, "apv", ApprovalOutcome::Approved);

        let decision = h
            .gate
            .authorize(&h.org, &h.work_id, Revision::new(1))
            .unwrap();
        match decision {
            DeliveryDecision::Authorized { approval_id } => {
                assert_eq!(approval_id, ApprovalId::new("apv"));
            }
            other => panic!("expected Authorized, got {other:?}"),
        }
    }

    #[test]
    fn authorize_blocks_when_no_delivery_approval_exists() {
        let h = harness();
        // A Plan-scoped approval does not govern delivery.
        let mut plan = approval(&h, "plan", Revision::new(1));
        plan.scope = ApprovalScope::Plan {
            attempt_id: AttemptId::new("att"),
        };
        h.approvals.create(plan).unwrap();

        let decision = h
            .gate
            .authorize(&h.org, &h.work_id, Revision::new(1))
            .unwrap();
        match decision {
            DeliveryDecision::Blocked { blockers } => {
                assert_eq!(
                    blockers,
                    vec![DeliveryBlocker::NoApproval {
                        work_item_id: h.work_id.clone()
                    }]
                );
            }
            other => panic!("expected Blocked, got {other:?}"),
        }
    }

    #[test]
    fn authorize_blocks_while_the_approval_is_pending() {
        let h = harness();
        h.approvals.create(approval(&h, "apv", Revision::new(1))).unwrap();

        let decision = h
            .gate
            .authorize(&h.org, &h.work_id, Revision::new(1))
            .unwrap();
        match decision {
            DeliveryDecision::Blocked { blockers } => {
                assert_eq!(
                    blockers,
                    vec![DeliveryBlocker::ApprovalPending {
                        approval_id: ApprovalId::new("apv")
                    }]
                );
            }
            other => panic!("expected Blocked, got {other:?}"),
        }
    }

    #[test]
    fn authorize_blocks_a_denied_approval() {
        let h = harness();
        h.approvals.create(approval(&h, "apv", Revision::new(1))).unwrap();
        decide(&h, "apv", ApprovalOutcome::Denied);

        let decision = h
            .gate
            .authorize(&h.org, &h.work_id, Revision::new(1))
            .unwrap();
        match decision {
            DeliveryDecision::Blocked { blockers } => {
                assert_eq!(
                    blockers,
                    vec![DeliveryBlocker::ApprovalDenied {
                        approval_id: ApprovalId::new("apv")
                    }]
                );
            }
            other => panic!("expected Blocked, got {other:?}"),
        }
    }

    #[test]
    fn authorize_blocks_a_revision_the_approval_was_not_granted_for() {
        let h = harness();
        h.approvals.create(approval(&h, "apv", Revision::new(1))).unwrap();
        decide(&h, "apv", ApprovalOutcome::Approved);

        // Delivering revision 2 under an approval granted for revision 1.
        let decision = h
            .gate
            .authorize(&h.org, &h.work_id, Revision::new(2))
            .unwrap();
        match decision {
            DeliveryDecision::Blocked { blockers } => {
                assert_eq!(
                    blockers,
                    vec![DeliveryBlocker::PayloadRevisionMismatch {
                        approval_id: ApprovalId::new("apv"),
                        approved: Revision::new(1),
                        delivering: Revision::new(2),
                    }]
                );
            }
            other => panic!("expected Blocked, got {other:?}"),
        }
    }

    #[test]
    fn authorize_is_isolated_by_organization() {
        let h = harness();
        h.approvals.create(approval(&h, "apv", Revision::new(1))).unwrap();
        decide(&h, "apv", ApprovalOutcome::Approved);

        // The same work id in another org has no governing approval.
        let decision = h
            .gate
            .authorize(&OrganizationId::new("other"), &h.work_id, Revision::new(1))
            .unwrap();
        assert!(matches!(
            decision,
            DeliveryDecision::Blocked {
                blockers
            } if blockers.iter().any(|b| matches!(
                b,
                DeliveryBlocker::NoApproval { .. }
            ))
        ));
    }

    #[test]
    fn authorize_scopes_to_the_named_work_item() {
        let h = harness();
        h.approvals.create(approval(&h, "apv", Revision::new(1))).unwrap();
        decide(&h, "apv", ApprovalOutcome::Approved);

        // A different work item is not governed by this approval.
        let decision = h
            .gate
            .authorize(&h.org, &WorkItemId::new("other-work"), Revision::new(1))
            .unwrap();
        assert!(matches!(
            decision,
            DeliveryDecision::Blocked {
                blockers
            } if blockers.iter().any(|b| matches!(
                b,
                DeliveryBlocker::NoApproval { .. }
            ))
        ));
    }

    #[test]
    fn authorize_reports_every_blocking_approval() {
        let h = harness();
        // One pending, one denied, one approved-for-the-wrong-revision.
        h.approvals.create(approval(&h, "pending", Revision::new(1))).unwrap();
        h.approvals.create(approval(&h, "denied", Revision::new(1))).unwrap();
        h.approvals.create(approval(&h, "wrong-rev", Revision::new(3))).unwrap();
        decide(&h, "denied", ApprovalOutcome::Denied);
        decide(&h, "wrong-rev", ApprovalOutcome::Approved);

        let decision = h
            .gate
            .authorize(&h.org, &h.work_id, Revision::new(2))
            .unwrap();
        match decision {
            DeliveryDecision::Blocked { blockers } => {
                assert_eq!(blockers.len(), 3);
                assert!(blockers.contains(&DeliveryBlocker::ApprovalPending {
                    approval_id: ApprovalId::new("pending")
                }));
                assert!(blockers.contains(&DeliveryBlocker::ApprovalDenied {
                    approval_id: ApprovalId::new("denied")
                }));
                assert!(blockers.contains(&DeliveryBlocker::PayloadRevisionMismatch {
                    approval_id: ApprovalId::new("wrong-rev"),
                    approved: Revision::new(3),
                    delivering: Revision::new(2),
                }));
            }
            other => panic!("expected Blocked, got {other:?}"),
        }
    }

    #[test]
    fn authorize_accepts_when_any_matching_approval_authorizes() {
        let h = harness();
        // A denied approval and an approved, revision-matching one.
        h.approvals.create(approval(&h, "denied", Revision::new(1))).unwrap();
        h.approvals.create(approval(&h, "ok", Revision::new(2))).unwrap();
        decide(&h, "denied", ApprovalOutcome::Denied);
        decide(&h, "ok", ApprovalOutcome::Approved);

        let decision = h
            .gate
            .authorize(&h.org, &h.work_id, Revision::new(2))
            .unwrap();
        match decision {
            DeliveryDecision::Authorized { approval_id } => {
                assert_eq!(approval_id, ApprovalId::new("ok"));
            }
            other => panic!("expected Authorized, got {other:?}"),
        }
    }

    #[test]
    fn delivery_scope_round_trips_through_storage() {
        let h = harness();
        h.approvals.create(approval(&h, "apv", Revision::new(1))).unwrap();

        let stored = h.approvals.get(&ApprovalId::new("apv")).unwrap().unwrap();
        assert_eq!(
            stored.scope,
            ApprovalScope::Delivery {
                work_item_id: h.work_id.clone()
            }
        );
    }
}
