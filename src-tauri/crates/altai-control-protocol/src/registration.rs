//! Versioned control-plane health and execution-host registration contracts.
//!
//! The registration grant is deliberately a plain request field. A transport
//! can accept it without retaining it; callers must redact the field in logs
//! and errors.

use crate::{AgentInstanceId, WorkspaceId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const CONTROL_PLANE_PROTOCOL_MAJOR: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct HostCapabilities {
    pub values: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostRegistration {
    pub agent_instance_id: AgentInstanceId,
    pub workspaces: Vec<WorkspaceId>,
    pub capabilities: HostCapabilities,
    pub protocol_major: u16,
}

/// Transport request; deliberately does not implement `Debug`.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostRegistrationRequest {
    pub grant_token: String,
    pub host: HostRegistration,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisteredHost {
    pub agent_instance_id: AgentInstanceId,
    pub workspaces: Vec<WorkspaceId>,
    pub capabilities: HostCapabilities,
    pub registered_at_unix_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlPlaneHealth {
    pub service_version: String,
    pub protocol_major: u16,
    pub store_kind: String,
    pub registered_host_count: usize,
    pub database_adapter_ready: bool,
}
