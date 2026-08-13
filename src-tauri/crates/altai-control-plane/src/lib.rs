//! Federated control-plane bootstrap primitives.
//!
//! This crate deliberately starts at the boundary: version/health reporting
//! and one-time authenticated execution-host registration. It has no Work
//! repository and does not move any existing `work.db` mutation. A later M3
//! sync layer will connect the global control database to local execution
//! ledgers through the registered host identity.

pub mod postgres;
pub mod postgres_scope;
pub mod scope_repository;
mod service;
pub mod transport;

pub use altai_control_protocol::{
    ControlPlaneHealth, HostCapabilities, HostRegistration, HostRegistrationRequest, RegisteredHost,
};
pub use postgres::PostgresRegistrationRepository;
pub use postgres_scope::PostgresScopeRepository;
pub use scope_repository::{InMemoryScopeRepository, ScopeError, ScopeRepository};
pub use service::{
    ControlPlane, ControlPlaneConfig, ControlPlaneError, ControlPlaneStore, RegistrationCommit,
    RegistrationGrant, RegistrationRepository,
};
pub use transport::{router, router_with_scope_repository, BootstrapCredential, TransportError};
