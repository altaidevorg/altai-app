//! Host-neutral service boundary for ALTAI agent surfaces.
//!
//! This crate deliberately knows nothing about Tauri, stdin/stdout, or a UI.
//! Desktop and future hosts own their respective adapters; shared event DTOs,
//! bounded sink semantics, and workspace-local durable state live here.

pub mod event;
pub mod instance_registry;
pub mod replay;
pub mod routing;
pub mod sink;
pub mod workspace_services;

pub use event::{AgentEventEnvelope, AgentEventScope, EditDiffPayload, Event};
pub use instance_registry::{AgentInstanceRegistry, AgentInstanceRegistryError};
pub use replay::{
    AgentReplayEventEnvelope, AgentRunReplayCursor, ReplayError, ReplayService, SessionIdentity,
};
pub use routing::{
    admit_queued_user_message, admit_run, admit_user_message, coordinator_guard, queue_run,
    rollback_run_admission, RunAdmission, RunCoordinator, RunPhase, RunTransitionError,
    SharedRunCoordinator,
};
pub use sink::{AgentEventSink, AgentEventSinkError, SequencedEventDispatcher};
pub use workspace_services::{
    classify_runs_abandoned_by_restart, WorkspaceServiceError, WorkspaceServices,
};
