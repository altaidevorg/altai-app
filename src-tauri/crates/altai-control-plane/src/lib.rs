//! Federated control-plane bootstrap primitives.
//!
//! This crate deliberately starts at the boundary: version/health reporting
//! and one-time authenticated execution-host registration. It has no Work
//! repository and does not move any existing `work.db` mutation. A later M3
//! sync layer will connect the global control database to local execution
//! ledgers through the registered host identity.

mod service;

pub use service::{
    ControlPlane, ControlPlaneConfig, ControlPlaneError, ControlPlaneHealth, ControlPlaneStore,
    HostCapabilities, HostRegistration, RegisteredHost, RegistrationGrant, RegistrationRequest,
};
