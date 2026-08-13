//! Federated control-plane bootstrap primitives.
//!
//! This crate deliberately starts at the boundary: version/health reporting
//! and one-time authenticated execution-host registration. It has no Work
//! repository and does not move any existing `work.db` mutation. A later M3
//! sync layer will connect the global control database to local execution
//! ledgers through the registered host identity.

pub mod agent_repository;
pub mod attempt_repository;
pub mod dispatch_eligibility;
pub mod execution_repository;
pub mod legacy_work_bridge;
pub mod recovery_repository;
pub mod run_binding_repository;
pub mod run_context;
pub mod scheduler;
pub mod scope_repository;
mod service;
pub mod sqlite_agent;
pub mod sqlite_registration;
pub mod sqlite_scope;
pub mod sqlite_wake;
pub mod sqlite_work_graph;
pub mod transport;
pub mod wake_repository;
pub mod work_graph_repository;
pub mod work_item_repository;

pub use agent_repository::{AgentRepository, AgentRepositoryError, InMemoryAgentRepository};
pub use altai_control_protocol::{
    ControlPlaneHealth, HostCapabilities, HostRegistration, HostRegistrationRequest, RegisteredHost,
};
pub use attempt_repository::{AttemptError, AttemptRepository, SqliteAttemptRepository};
pub use dispatch_eligibility::{
    DispatchBlocker, DispatchEligibility, DispatchEligibilityEngine, DispatchEligibilityError,
};
pub use execution_repository::{
    ExecutionSnapshot, ExecutionSnapshotError, ExecutionSnapshotRepository,
    SqliteExecutionSnapshotRepository,
};
pub use legacy_work_bridge::{LegacyWorkBridge, LegacyWorkBridgeError};
pub use recovery_repository::{RecoveryError, RecoveryRepository, SqliteRecoveryRepository};
pub use run_binding_repository::{
    RunBindingError, RunBindingRepository, SqliteRunBindingRepository,
};
pub use run_context::{
    assemble_bounded_run_context, build_bounded_run_context, load_attempt_bound_run_context,
    load_bounded_run_context, BoundedRunContext, RunContextError, RunContextInput,
    MAX_RUN_CONTEXT_BYTES,
};
pub use scheduler::{ScheduleResult, SchedulerError, SingleWriterScheduler};
pub use scope_repository::{InMemoryScopeRepository, ScopeError, ScopeRepository};
pub use service::{
    ControlPlane, ControlPlaneConfig, ControlPlaneError, ControlPlaneStore, RegistrationCommit,
    RegistrationGrant, RegistrationRepository,
};
pub use sqlite_agent::SqliteAgentRepository;
pub use sqlite_registration::SqliteRegistrationRepository;
pub use sqlite_scope::SqliteScopeRepository;
pub use sqlite_wake::SqliteWakeRepository;
pub use sqlite_work_graph::SqliteWorkGraphRepository;
pub use transport::{
    router, router_with_all_repositories, router_with_control_repositories,
    router_with_repositories, router_with_scope_repository, BootstrapCredential, TransportError,
};
pub use wake_repository::{InMemoryWakeRepository, WakeError, WakeRepository};
pub use work_graph_repository::{InMemoryWorkGraphRepository, WorkGraphError, WorkGraphRepository};
pub use work_item_repository::{
    SqliteWorkItemRepository, WorkItemRepository, WorkItemRepositoryError,
};
