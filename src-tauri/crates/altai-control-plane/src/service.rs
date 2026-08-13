use altai_control_protocol::{
    ControlPlaneHealth, HostRegistrationRequest, RegisteredHost, CONTROL_PLANE_PROTOCOL_MAJOR,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Selected global control-database topology. Connection/bootstrap is a later
/// integration concern; this contract validates that callers cannot confuse
/// the two supported deployment modes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlPlaneStore {
    Postgres { connection_url: String },
    Pglite { data_dir: String },
}

/// Immutable service configuration supplied by the daemon/bootstrap layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlPlaneConfig {
    pub service_version: String,
    pub store: ControlPlaneStore,
    /// Registration credentials expire quickly and may be used exactly once.
    pub registration_ttl_seconds: u64,
}

impl ControlPlaneConfig {
    pub fn validate(&self) -> Result<(), ControlPlaneError> {
        if self.service_version.trim().is_empty() {
            return Err(ControlPlaneError::InvalidConfig {
                reason: "service_version must not be empty".to_string(),
            });
        }
        if self.registration_ttl_seconds == 0 {
            return Err(ControlPlaneError::InvalidConfig {
                reason: "registration_ttl_seconds must be greater than zero".to_string(),
            });
        }
        match &self.store {
            ControlPlaneStore::Postgres { connection_url }
                if !connection_url.starts_with("postgres://")
                    && !connection_url.starts_with("postgresql://") =>
            {
                Err(ControlPlaneError::InvalidConfig {
                    reason: "Postgres connection_url must use postgres:// or postgresql://"
                        .to_string(),
                })
            }
            ControlPlaneStore::Pglite { data_dir } if data_dir.trim().is_empty() => {
                Err(ControlPlaneError::InvalidConfig {
                    reason: "PGlite data_dir must not be empty".to_string(),
                })
            }
            _ => Ok(()),
        }
    }
}

/// One-time credential issued by an authenticated administrator/bootstrapper.
/// The plaintext token is returned exactly once and is never retained.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistrationGrant {
    pub token: String,
    pub expires_at_unix_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlPlaneError {
    InvalidConfig { reason: String },
    UnsupportedProtocol { expected: u16, got: u16 },
    InvalidGrant,
    ExpiredGrant,
    DuplicateWorkspace { workspace_id: String },
    Internal { reason: String },
}

impl std::fmt::Display for ControlPlaneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidConfig { reason } => {
                write!(f, "invalid control-plane configuration: {reason}")
            }
            Self::UnsupportedProtocol { expected, got } => {
                write!(
                    f,
                    "unsupported control-plane protocol: expected {expected}, got {got}"
                )
            }
            Self::InvalidGrant => write!(f, "invalid or already consumed registration grant"),
            Self::ExpiredGrant => write!(f, "expired registration grant"),
            Self::DuplicateWorkspace { workspace_id } => {
                write!(
                    f,
                    "duplicate workspace in host registration: {workspace_id}"
                )
            }
            Self::Internal { reason } => write!(f, "control-plane internal error: {reason}"),
        }
    }
}

impl std::error::Error for ControlPlaneError {}

#[derive(Debug, Clone)]
struct PendingGrant {
    expires_at_unix_seconds: u64,
}

/// In-memory bootstrap service. Persistence and transport are deliberately
/// separate follow-up concerns; callers cannot accidentally treat this type as
/// the global Work repository.
pub struct ControlPlane {
    config: ControlPlaneConfig,
    pending_grants: Mutex<HashMap<String, PendingGrant>>,
    hosts: Mutex<HashMap<String, RegisteredHost>>,
}

impl ControlPlane {
    pub fn bootstrap(config: ControlPlaneConfig) -> Result<Self, ControlPlaneError> {
        config.validate()?;
        Ok(Self {
            config,
            pending_grants: Mutex::new(HashMap::new()),
            hosts: Mutex::new(HashMap::new()),
        })
    }

    pub fn health(&self) -> Result<ControlPlaneHealth, ControlPlaneError> {
        let hosts = self.hosts.lock().map_err(|_| ControlPlaneError::Internal {
            reason: "host registry lock poisoned".to_string(),
        })?;
        Ok(ControlPlaneHealth {
            service_version: self.config.service_version.clone(),
            protocol_major: CONTROL_PLANE_PROTOCOL_MAJOR,
            store_kind: match self.config.store {
                ControlPlaneStore::Postgres { .. } => "postgres".to_string(),
                ControlPlaneStore::Pglite { .. } => "pglite".to_string(),
            },
            registered_host_count: hosts.len(),
            database_adapter_ready: false,
        })
    }

    pub fn issue_registration_grant(&self) -> Result<RegistrationGrant, ControlPlaneError> {
        let token = format!("cpr_{}", Uuid::new_v4());
        let expires_at_unix_seconds = now_unix_seconds()
            .checked_add(self.config.registration_ttl_seconds)
            .ok_or_else(|| ControlPlaneError::InvalidConfig {
                reason: "registration_ttl_seconds overflows unix timestamp".to_string(),
            })?;
        let mut grants = self
            .pending_grants
            .lock()
            .map_err(|_| ControlPlaneError::Internal {
                reason: "registration grant lock poisoned".to_string(),
            })?;
        grants.insert(
            token_digest(&token),
            PendingGrant {
                expires_at_unix_seconds,
            },
        );
        Ok(RegistrationGrant {
            token,
            expires_at_unix_seconds,
        })
    }

    pub fn register_host(
        &self,
        request: HostRegistrationRequest,
    ) -> Result<RegisteredHost, ControlPlaneError> {
        if request.host.protocol_major != CONTROL_PLANE_PROTOCOL_MAJOR {
            return Err(ControlPlaneError::UnsupportedProtocol {
                expected: CONTROL_PLANE_PROTOCOL_MAJOR,
                got: request.host.protocol_major,
            });
        }
        let mut workspace_values = BTreeSet::new();
        for workspace in &request.host.workspaces {
            if !workspace_values.insert(workspace.value.clone()) {
                return Err(ControlPlaneError::DuplicateWorkspace {
                    workspace_id: workspace.value.clone(),
                });
            }
        }

        let now = now_unix_seconds();
        let digest = token_digest(&request.grant_token);
        let grant = self
            .pending_grants
            .lock()
            .map_err(|_| ControlPlaneError::Internal {
                reason: "registration grant lock poisoned".to_string(),
            })?
            .remove(&digest)
            .ok_or(ControlPlaneError::InvalidGrant)?;
        if grant.expires_at_unix_seconds < now {
            return Err(ControlPlaneError::ExpiredGrant);
        }

        let registered = RegisteredHost {
            agent_instance_id: request.host.agent_instance_id,
            workspaces: request.host.workspaces,
            capabilities: request.host.capabilities,
            registered_at_unix_seconds: now,
        };
        self.hosts
            .lock()
            .map_err(|_| ControlPlaneError::Internal {
                reason: "host registry lock poisoned".to_string(),
            })?
            .insert(
                registered.agent_instance_id.value.clone(),
                registered.clone(),
            );
        Ok(registered)
    }
}

fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs()
}

fn token_digest(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use altai_control_protocol::{HostCapabilities, HostRegistration};

    fn config() -> ControlPlaneConfig {
        ControlPlaneConfig {
            service_version: "0.1.0".to_string(),
            store: ControlPlaneStore::Pglite {
                data_dir: "/tmp/altai-control".to_string(),
            },
            registration_ttl_seconds: 60,
        }
    }

    fn host() -> HostRegistration {
        HostRegistration {
            agent_instance_id: altai_control_protocol::AgentInstanceId::new("host-a"),
            workspaces: vec![altai_control_protocol::WorkspaceId::new("workspace-a")],
            capabilities: HostCapabilities {
                values: ["isanagent".to_string(), "work.execute".to_string()]
                    .into_iter()
                    .collect(),
            },
            protocol_major: CONTROL_PLANE_PROTOCOL_MAJOR,
        }
    }

    #[test]
    fn bootstrap_reports_non_secret_health() {
        let plane = ControlPlane::bootstrap(config()).unwrap();
        assert_eq!(
            plane.health().unwrap(),
            ControlPlaneHealth {
                service_version: "0.1.0".to_string(),
                protocol_major: 1,
                store_kind: "pglite".to_string(),
                registered_host_count: 0,
                database_adapter_ready: false,
            }
        );
    }

    #[test]
    fn registration_grant_is_single_use_and_updates_health() {
        let plane = ControlPlane::bootstrap(config()).unwrap();
        let grant = plane.issue_registration_grant().unwrap();
        let registered = plane
            .register_host(HostRegistrationRequest {
                grant_token: grant.token.clone(),
                host: host(),
            })
            .unwrap();
        assert_eq!(
            registered.agent_instance_id,
            altai_control_protocol::AgentInstanceId::new("host-a")
        );
        assert_eq!(plane.health().unwrap().registered_host_count, 1);
        assert_eq!(
            plane.register_host(HostRegistrationRequest {
                grant_token: grant.token,
                host: host(),
            }),
            Err(ControlPlaneError::InvalidGrant)
        );
    }

    #[test]
    fn rejected_registration_does_not_consume_grant_before_validation() {
        let plane = ControlPlane::bootstrap(config()).unwrap();
        let grant = plane.issue_registration_grant().unwrap();
        let mut incompatible = host();
        incompatible.protocol_major = 2;
        assert_eq!(
            plane.register_host(HostRegistrationRequest {
                grant_token: grant.token.clone(),
                host: incompatible,
            }),
            Err(ControlPlaneError::UnsupportedProtocol {
                expected: 1,
                got: 2,
            })
        );
        assert!(plane
            .register_host(HostRegistrationRequest {
                grant_token: grant.token,
                host: host(),
            })
            .is_ok());
    }

    #[test]
    fn rejects_unsupported_store_configuration() {
        let mut invalid = config();
        invalid.store = ControlPlaneStore::Postgres {
            connection_url: "sqlite:///not-global.db".to_string(),
        };
        assert!(matches!(
            ControlPlane::bootstrap(invalid),
            Err(ControlPlaneError::InvalidConfig { .. })
        ));
    }
}
