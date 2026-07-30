//! Stable collaboration-domain contracts shared by the future control plane,
//! desktop clients, and runner adapters.
//!
//! This crate intentionally has no database or transport dependency. It keeps
//! the rules that must remain identical whether a work item is created from
//! Studio, the IDE, a VS Code extension, or the collaboration service.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

pub const TERRA_DEFAULT_MODEL: &str = "terra";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorKind {
    Human,
    Agent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorRef {
    pub id: String,
    pub kind: ActorKind,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkItemState {
    Backlog,
    Ready,
    InProgress,
    WaitingForReview,
    Done,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionMode {
    ReadOnly,
    WorkspaceWrite,
    BypassApprovals,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentExecutionSpec {
    /// Stable agent profile identifier, for example `coder` or `reviewer`.
    pub agent_id: String,
    /// Model requested by the human owner. Terra is the product default.
    pub model_id: String,
    pub permission_mode: PermissionMode,
    pub skills: Vec<String>,
    /// The runner should lease this worktree before it starts executing.
    pub workspace_ref: Option<String>,
}

impl AgentExecutionSpec {
    pub fn terra(agent_id: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            model_id: TERRA_DEFAULT_MODEL.to_string(),
            permission_mode: PermissionMode::WorkspaceWrite,
            skills: Vec::new(),
            workspace_ref: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DelegationState {
    Queued,
    Running,
    WaitingForApproval,
    Succeeded,
    Failed,
    Cancelled,
}

impl DelegationState {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentDelegation {
    pub id: String,
    pub agent: ActorRef,
    pub requested_by: ActorRef,
    pub state: DelegationState,
    pub execution: AgentExecutionSpec,
    /// Set only after a runner accepts the delegation. This is not a task id.
    pub run_id: Option<String>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkItem {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub description: String,
    pub state: WorkItemState,
    /// A human remains accountable even while one or more agents execute.
    pub accountable_owner: ActorRef,
    pub collaborators: Vec<ActorRef>,
    pub delegations: Vec<AgentDelegation>,
    pub version: u64,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WorkEventKind {
    WorkItemCreated,
    StateChanged {
        from: WorkItemState,
        to: WorkItemState,
    },
    DelegationCreated {
        delegation_id: String,
    },
    DelegationStateChanged {
        delegation_id: String,
        from: DelegationState,
        to: DelegationState,
    },
    CommentAdded {
        body: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkEvent {
    pub id: String,
    pub work_item_id: String,
    pub actor: ActorRef,
    pub kind: WorkEventKind,
    /// Free-form operational data, such as a runner id or pull-request URL.
    pub metadata: Value,
    pub occurred_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    Empty(&'static str),
    AccountableOwnerMustBeHuman,
    DelegatedActorMustBeAgent,
    DelegationRequesterMustBeHuman,
    DuplicateActor(String),
    DuplicateDelegation(String),
    UnknownDelegation(String),
    InvalidStateTransition {
        from: WorkItemState,
        to: WorkItemState,
    },
    InvalidDelegationTransition {
        from: DelegationState,
        to: DelegationState,
    },
    TerminalDelegationHasNoRun,
}

impl std::fmt::Display for DomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty(field) => write!(f, "{field} cannot be empty"),
            Self::AccountableOwnerMustBeHuman => write!(f, "the accountable owner must be a human"),
            Self::DelegatedActorMustBeAgent => write!(f, "a delegation target must be an agent"),
            Self::DelegationRequesterMustBeHuman => {
                write!(f, "a delegation must be requested by a human")
            }
            Self::DuplicateActor(id) => write!(f, "duplicate collaborator: {id}"),
            Self::DuplicateDelegation(id) => write!(f, "duplicate delegation: {id}"),
            Self::UnknownDelegation(id) => write!(f, "unknown delegation: {id}"),
            Self::InvalidStateTransition { from, to } => {
                write!(f, "invalid work item transition {from:?} -> {to:?}")
            }
            Self::InvalidDelegationTransition { from, to } => {
                write!(f, "invalid delegation transition {from:?} -> {to:?}")
            }
            Self::TerminalDelegationHasNoRun => {
                write!(f, "a terminal delegation must reference a runner run")
            }
        }
    }
}

impl std::error::Error for DomainError {}

impl WorkItem {
    pub fn validate(&self) -> Result<(), DomainError> {
        required("work item id", &self.id)?;
        required("project id", &self.project_id)?;
        required("title", &self.title)?;
        validate_human_owner(&self.accountable_owner)?;

        let mut actors = BTreeSet::new();
        actors.insert(self.accountable_owner.id.clone());
        for collaborator in &self.collaborators {
            required("collaborator id", &collaborator.id)?;
            if !actors.insert(collaborator.id.clone()) {
                return Err(DomainError::DuplicateActor(collaborator.id.clone()));
            }
        }

        let mut delegations = BTreeSet::new();
        for delegation in &self.delegations {
            delegation.validate()?;
            if !delegations.insert(delegation.id.clone()) {
                return Err(DomainError::DuplicateDelegation(delegation.id.clone()));
            }
        }
        Ok(())
    }

    pub fn transition(
        &mut self,
        next: WorkItemState,
        at_ms: i64,
    ) -> Result<WorkEventKind, DomainError> {
        if !can_transition_work_item(&self.state, &next) {
            return Err(DomainError::InvalidStateTransition {
                from: self.state.clone(),
                to: next,
            });
        }
        let from = std::mem::replace(&mut self.state, next.clone());
        self.version += 1;
        self.updated_at_ms = at_ms;
        Ok(WorkEventKind::StateChanged { from, to: next })
    }

    pub fn update_delegation(
        &mut self,
        delegation_id: &str,
        next: DelegationState,
        run_id: Option<String>,
        at_ms: i64,
    ) -> Result<WorkEventKind, DomainError> {
        let delegation = self
            .delegations
            .iter_mut()
            .find(|delegation| delegation.id == delegation_id)
            .ok_or_else(|| DomainError::UnknownDelegation(delegation_id.to_string()))?;
        if !can_transition_delegation(&delegation.state, &next) {
            return Err(DomainError::InvalidDelegationTransition {
                from: delegation.state.clone(),
                to: next,
            });
        }
        if run_id.is_some() {
            delegation.run_id = run_id;
        }
        if next.is_terminal() && delegation.run_id.is_none() {
            return Err(DomainError::TerminalDelegationHasNoRun);
        }
        let from = std::mem::replace(&mut delegation.state, next.clone());
        delegation.updated_at_ms = at_ms;
        self.version += 1;
        self.updated_at_ms = at_ms;
        Ok(WorkEventKind::DelegationStateChanged {
            delegation_id: delegation_id.to_string(),
            from,
            to: next,
        })
    }
}

impl AgentDelegation {
    pub fn validate(&self) -> Result<(), DomainError> {
        required("delegation id", &self.id)?;
        required("agent id", &self.agent.id)?;
        required("requester id", &self.requested_by.id)?;
        required("execution agent id", &self.execution.agent_id)?;
        required("execution model id", &self.execution.model_id)?;
        if self.agent.kind != ActorKind::Agent {
            return Err(DomainError::DelegatedActorMustBeAgent);
        }
        if self.requested_by.kind != ActorKind::Human {
            return Err(DomainError::DelegationRequesterMustBeHuman);
        }
        if self.state.is_terminal() && self.run_id.is_none() {
            return Err(DomainError::TerminalDelegationHasNoRun);
        }
        Ok(())
    }
}

fn required(name: &'static str, value: &str) -> Result<(), DomainError> {
    if value.trim().is_empty() {
        Err(DomainError::Empty(name))
    } else {
        Ok(())
    }
}

fn validate_human_owner(actor: &ActorRef) -> Result<(), DomainError> {
    required("accountable owner id", &actor.id)?;
    if actor.kind != ActorKind::Human {
        Err(DomainError::AccountableOwnerMustBeHuman)
    } else {
        Ok(())
    }
}

fn can_transition_work_item(from: &WorkItemState, to: &WorkItemState) -> bool {
    use WorkItemState::*;
    (from == &Backlog && matches!(to, Ready | Cancelled))
        || (from == &Ready && matches!(to, InProgress | Backlog | Cancelled))
        || (from == &InProgress && matches!(to, WaitingForReview | Ready | Cancelled))
        || (from == &WaitingForReview && matches!(to, Done | InProgress | Cancelled))
        || ((from == &Done || from == &Cancelled) && from == to)
}

fn can_transition_delegation(from: &DelegationState, to: &DelegationState) -> bool {
    use DelegationState::*;
    (from == &Queued && matches!(to, Running | Cancelled))
        || (from == &Running && matches!(to, WaitingForApproval | Succeeded | Failed | Cancelled))
        || (from == &WaitingForApproval && matches!(to, Running | Failed | Cancelled))
        || ((from == &Succeeded || from == &Failed || from == &Cancelled) && from == to)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn human(id: &str) -> ActorRef {
        ActorRef {
            id: id.into(),
            kind: ActorKind::Human,
            display_name: id.into(),
        }
    }

    fn agent(id: &str) -> ActorRef {
        ActorRef {
            id: id.into(),
            kind: ActorKind::Agent,
            display_name: id.into(),
        }
    }

    fn delegation() -> AgentDelegation {
        AgentDelegation {
            id: "del-1".into(),
            agent: agent("agent-coder"),
            requested_by: human("human-owner"),
            state: DelegationState::Queued,
            execution: AgentExecutionSpec::terra("coder"),
            run_id: None,
            created_at_ms: 1,
            updated_at_ms: 1,
        }
    }

    fn item() -> WorkItem {
        WorkItem {
            id: "work-1".into(),
            project_id: "project-1".into(),
            title: "Ship collaboration".into(),
            description: String::new(),
            state: WorkItemState::Ready,
            accountable_owner: human("human-owner"),
            collaborators: vec![agent("agent-coder")],
            delegations: vec![delegation()],
            version: 1,
            created_at_ms: 1,
            updated_at_ms: 1,
        }
    }

    #[test]
    fn human_owner_and_agent_delegate_are_enforced() {
        let work = item();
        assert!(work.validate().is_ok());

        let mut invalid = work.clone();
        invalid.accountable_owner = agent("agent-owner");
        assert_eq!(
            invalid.validate(),
            Err(DomainError::AccountableOwnerMustBeHuman)
        );

        let mut invalid_delegate = work;
        invalid_delegate.delegations[0].agent = human("human-helper");
        assert_eq!(
            invalid_delegate.validate(),
            Err(DomainError::DelegatedActorMustBeAgent)
        );
    }

    #[test]
    fn a_delegation_carries_terra_and_a_distinct_runner_run() {
        let mut work = item();
        assert_eq!(work.delegations[0].execution.model_id, "terra");
        let event = work
            .update_delegation("del-1", DelegationState::Running, Some("run-123".into()), 2)
            .unwrap();
        assert!(matches!(
            event,
            WorkEventKind::DelegationStateChanged {
                from: DelegationState::Queued,
                to: DelegationState::Running,
                ..
            }
        ));
        assert_eq!(work.delegations[0].run_id.as_deref(), Some("run-123"));
    }

    #[test]
    fn terminal_delegations_require_a_run_and_transitions_are_guarded() {
        let mut work = item();
        assert_eq!(
            work.update_delegation("del-1", DelegationState::Succeeded, None, 2),
            Err(DomainError::InvalidDelegationTransition {
                from: DelegationState::Queued,
                to: DelegationState::Succeeded,
            })
        );
        assert_eq!(
            work.transition(WorkItemState::Done, 2),
            Err(DomainError::InvalidStateTransition {
                from: WorkItemState::Ready,
                to: WorkItemState::Done,
            })
        );
    }
}
