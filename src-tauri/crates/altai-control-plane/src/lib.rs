//! Federated control-plane bootstrap primitives.
//!
//! This crate deliberately starts at the boundary: version/health reporting
//! and one-time authenticated execution-host registration. It has no Work
//! repository and does not move any existing `work.db` mutation. A later M3
//! sync layer will connect the global control database to local execution
//! ledgers through the registered host identity.

pub mod agent_repository;
pub mod dispatch_eligibility;
pub mod recovery_repository;
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

pub use agent_repository::{AgentRepository, AgentRepositoryError, InMemoryAgentRepository};
pub use altai_control_protocol::{
    ControlPlaneHealth, HostCapabilities, HostRegistration, HostRegistrationRequest, RegisteredHost,
};
pub use dispatch_eligibility::{
    DispatchBlocker, DispatchEligibility, DispatchEligibilityEngine, DispatchEligibilityError,
};
pub use recovery_repository::{RecoveryError, RecoveryRepository, SqliteRecoveryRepository};
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
