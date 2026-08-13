//! Host-neutral service boundary for ALTAI agent surfaces.
//!
//! This crate deliberately knows nothing about Tauri, stdin/stdout, or a UI.
//! Desktop and future hosts own their respective adapters; shared event DTOs,
//! bounded sink semantics, workspace-local durable state, and the long-lived
//! IsanAgent lifecycle live here.

pub mod acks;
pub mod channel;
pub mod compaction;
pub mod delivery;
pub mod attempt_executor;
pub mod event;
pub mod event_map;
pub mod host;
pub mod instance;
pub mod instance_builder;
pub mod instance_registry;
pub mod mcp;
pub mod permission;
pub mod replay;
pub mod routing;
pub mod service;
pub mod sink;
pub mod workspace_services;

pub use acks::{CancelAck, DocumentPart, ManualCompactionAck, SendAck, SteerAck};
pub use attempt_executor::{
    AttemptExecutionRequest, AttemptExecutionStatus, AttemptExecutor, AttemptExecutorError,
    ExecutionBinding,
};
pub use channel::ServiceChannel;
pub use compaction::CompactionArg;
pub use delivery::{
    deliver_next_run_event, is_system_event, parse_edit_diff, persist_and_deliver_run_event,
    persist_and_deliver_to_renderer, persist_run_event, persist_run_payload,
    redacted_event_payload, trusted_inbound, RunEventDeliveryError, RunEventTransition,
};
pub use event::{AgentEventEnvelope, AgentEventScope, EditDiffPayload, Event};
pub use event_map::{map_lifecycle_to_event, map_telemetry_to_event, telemetry_chat_id};
pub use host::{
    BuildInstanceRequest, BuiltInstance, HostAdapter, HostControlPlane, WorkspaceBundle,
};
pub use instance::{
    secret_identity, stop_instance, FallbackFingerprint, Instance, RuntimeFingerprint,
};
pub use instance_builder::{build_shared_instance, register_existing_claw_tools, SharedInstanceHooks};
pub use instance_registry::{AgentInstanceRegistry, AgentInstanceRegistryError};
pub use permission::{permission_mode_to_edit_mode, permission_mode_to_shell_mode};
pub use replay::{
    AgentReplayEventEnvelope, AgentRunReplayCursor, ReplayError, ReplayService, SessionIdentity,
};
pub use routing::{
    admit_queued_user_message, admit_run, admit_user_message, coordinator_guard, queue_run,
    rollback_run_admission, RunAdmission, RunCoordinator, RunPhase, RunTransitionError,
    SharedRunCoordinator,
};
pub use service::AgentService;
pub use sink::{AgentEventSink, AgentEventSinkError, SequencedEventDispatcher};
pub use workspace_services::{
    classify_runs_abandoned_by_restart, WorkspaceServiceError, WorkspaceServices,
};
