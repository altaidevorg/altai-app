//! Authenticated loopback HTTP transport for the control-plane bootstrap.
//!
//! The daemon binds only a loopback address in this milestone. A deployed
//! transport requires a separate TLS/proxy and credential-custody design; this
//! module deliberately refuses to make an accidental unauthenticated listener.

use crate::{ControlPlane, ControlPlaneError, RegistrationGrant};
use altai_control_protocol::{ControlPlaneHealth, HostRegistrationRequest, RegisteredHost};
use axum::{
    extract::State,
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use subtle::ConstantTimeEq;

/// Bootstrap bearer credential used by an administrator to inspect health and
/// issue a one-time host-registration grant. The plaintext value is never
/// retained after construction and this type intentionally does not implement
/// `Debug`.
pub struct BootstrapCredential {
    digest: [u8; 32],
}

impl BootstrapCredential {
    pub fn from_plaintext(token: &str) -> Result<Self, TransportError> {
        if token.len() < 16 {
            return Err(TransportError::UnsafeConfiguration {
                reason: "bootstrap token must contain at least 16 bytes".to_string(),
            });
        }
        Ok(Self {
            digest: digest(token),
        })
    }

    fn authorizes(&self, headers: &HeaderMap) -> bool {
        let Some(value) = headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
        else {
            return false;
        };
        let Some(token) = value.strip_prefix("Bearer ") else {
            return false;
        };
        self.digest.ct_eq(&digest(token)).into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportError {
    UnsafeConfiguration { reason: String },
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsafeConfiguration { reason } => {
                write!(f, "unsafe control-plane transport configuration: {reason}")
            }
        }
    }
}

impl std::error::Error for TransportError {}

#[derive(Clone)]
struct ApiState {
    plane: Arc<ControlPlane>,
    bootstrap_credential: Arc<BootstrapCredential>,
}

/// Routes are versioned from their first public exposure. The health and grant
/// routes need bootstrap authentication; host registration authenticates using
/// its one-time grant in the body and therefore does not accept the bootstrap
/// credential as a substitute.
pub fn router(plane: Arc<ControlPlane>, bootstrap_credential: BootstrapCredential) -> Router {
    let state = ApiState {
        plane,
        bootstrap_credential: Arc::new(bootstrap_credential),
    };
    Router::new()
        .route("/v1/health", get(health))
        .route("/v1/registration-grants", post(issue_registration_grant))
        .route("/v1/hosts/register", post(register_host))
        .with_state(state)
}

async fn health(
    State(state): State<ApiState>,
    headers: HeaderMap,
) -> Result<Json<ControlPlaneHealth>, ApiError> {
    require_bootstrap(&state, &headers)?;
    state.plane.health().map(Json).map_err(ApiError::from)
}

async fn issue_registration_grant(
    State(state): State<ApiState>,
    headers: HeaderMap,
) -> Result<Json<RegistrationGrant>, ApiError> {
    require_bootstrap(&state, &headers)?;
    state
        .plane
        .issue_registration_grant()
        .map(Json)
        .map_err(ApiError::from)
}

async fn register_host(
    State(state): State<ApiState>,
    Json(request): Json<HostRegistrationRequest>,
) -> Result<Json<RegisteredHost>, ApiError> {
    state
        .plane
        .register_host(request)
        .map(Json)
        .map_err(ApiError::from)
}

fn require_bootstrap(state: &ApiState, headers: &HeaderMap) -> Result<(), ApiError> {
    if state.bootstrap_credential.authorizes(headers) {
        Ok(())
    } else {
        Err(ApiError::unauthorized())
    }
}

#[derive(Serialize)]
struct ErrorBody {
    code: &'static str,
}

struct ApiError {
    status: StatusCode,
    code: &'static str,
}

impl ApiError {
    fn unauthorized() -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            code: "unauthorized",
        }
    }
}

impl From<ControlPlaneError> for ApiError {
    fn from(value: ControlPlaneError) -> Self {
        match value {
            ControlPlaneError::InvalidGrant | ControlPlaneError::ExpiredGrant => {
                Self::unauthorized()
            }
            ControlPlaneError::UnsupportedProtocol { .. } => Self {
                status: StatusCode::CONFLICT,
                code: "unsupported_protocol",
            },
            ControlPlaneError::DuplicateWorkspace { .. } => Self {
                status: StatusCode::BAD_REQUEST,
                code: "duplicate_workspace",
            },
            ControlPlaneError::InvalidConfig { .. } | ControlPlaneError::Internal { .. } => Self {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                code: "control_plane_unavailable",
            },
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (self.status, Json(ErrorBody { code: self.code })).into_response()
    }
}

fn digest(value: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ControlPlaneConfig, ControlPlaneStore};
    use altai_control_protocol::{
        AgentInstanceId, HostCapabilities, HostRegistration, WorkspaceId,
        CONTROL_PLANE_PROTOCOL_MAJOR,
    };
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    fn app() -> Router {
        let plane = Arc::new(
            ControlPlane::bootstrap(ControlPlaneConfig {
                service_version: "0.1.0".to_string(),
                store: ControlPlaneStore::Pglite {
                    data_dir: "/tmp/control-plane-test".to_string(),
                },
                registration_ttl_seconds: 60,
            })
            .unwrap(),
        );
        router(
            plane,
            BootstrapCredential::from_plaintext("test-bootstrap-token").unwrap(),
        )
    }

    #[tokio::test]
    async fn health_requires_bootstrap_bearer() {
        let response = app()
            .oneshot(
                Request::builder()
                    .uri("/v1/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn grant_registers_a_host_without_exposing_bootstrap_credential() {
        let app = app();
        let grant_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/registration-grants")
                    .header(AUTHORIZATION, "Bearer test-bootstrap-token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(grant_response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(grant_response.into_body(), 4096)
            .await
            .unwrap();
        let grant: RegistrationGrant = serde_json::from_slice(&bytes).unwrap();

        let request = HostRegistrationRequest {
            grant_token: grant.token,
            host: HostRegistration {
                agent_instance_id: AgentInstanceId::new("host-a"),
                workspaces: vec![WorkspaceId::new("workspace-a")],
                capabilities: HostCapabilities::default(),
                protocol_major: CONTROL_PLANE_PROTOCOL_MAJOR,
            },
        };
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/hosts/register")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn short_bootstrap_tokens_are_rejected() {
        assert!(BootstrapCredential::from_plaintext("too-short").is_err());
    }
}
