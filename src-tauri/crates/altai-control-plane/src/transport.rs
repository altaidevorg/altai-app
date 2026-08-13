//! Authenticated loopback HTTP transport for the control-plane bootstrap.
//!
//! The daemon binds only a loopback address in this milestone. A deployed
//! transport requires a separate TLS/proxy and credential-custody design; this
//! module deliberately refuses to make an accidental unauthenticated listener.

use crate::{
    AgentRepository, AgentRepositoryError, ControlPlane, ControlPlaneError, RegistrationGrant,
    RunBindingError, RunBindingRepository, ScopeError, ScopeRepository, WakeError, WakeRepository,
    WorkGraphError, WorkGraphRepository,
};
use altai_control_protocol::{
    AgentInstance, AgentProfileRevision, ControlPlaneHealth, Goal, GoalId, HostRegistrationRequest,
    Organization, OrganizationId, Project, ProjectWorkspace, RegisteredHost, RunBinding,
    WakeRequest, WakeSource, WorkCheckoutLease, WorkComment, WorkDependency, WorkItemId,
};
use axum::{
    extract::{Path, State},
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
    scope_repository: Option<Arc<dyn ScopeRepository>>,
    agent_repository: Option<Arc<dyn AgentRepository>>,
    work_graph_repository: Option<Arc<dyn WorkGraphRepository>>,
    wake_repository: Option<Arc<dyn WakeRepository>>,
    run_binding_repository: Option<Arc<dyn RunBindingRepository>>,
}

/// Routes are versioned from their first public exposure. The health and grant
/// routes need bootstrap authentication; host registration authenticates using
/// its one-time grant in the body and therefore does not accept the bootstrap
/// credential as a substitute.
pub fn router(plane: Arc<ControlPlane>, bootstrap_credential: BootstrapCredential) -> Router {
    router_with_all_repositories(plane, bootstrap_credential, None, None, None)
}

/// Build the authenticated transport with an optional CP-04 scope store.
/// Scope routes are absent until the daemon has a durable repository.
pub fn router_with_scope_repository(
    plane: Arc<ControlPlane>,
    bootstrap_credential: BootstrapCredential,
    scope_repository: Option<Arc<dyn ScopeRepository>>,
) -> Router {
    router_with_all_repositories(plane, bootstrap_credential, scope_repository, None, None)
}

pub fn router_with_repositories(
    plane: Arc<ControlPlane>,
    bootstrap_credential: BootstrapCredential,
    scope_repository: Option<Arc<dyn ScopeRepository>>,
    agent_repository: Option<Arc<dyn AgentRepository>>,
) -> Router {
    router_with_all_repositories(
        plane,
        bootstrap_credential,
        scope_repository,
        agent_repository,
        None,
    )
}

pub fn router_with_all_repositories(
    plane: Arc<ControlPlane>,
    bootstrap_credential: BootstrapCredential,
    scope_repository: Option<Arc<dyn ScopeRepository>>,
    agent_repository: Option<Arc<dyn AgentRepository>>,
    work_graph_repository: Option<Arc<dyn WorkGraphRepository>>,
) -> Router {
    let state = ApiState {
        plane,
        bootstrap_credential: Arc::new(bootstrap_credential),
        scope_repository,
        agent_repository,
        work_graph_repository,
        wake_repository: None,
        run_binding_repository: None,
    };
    Router::new()
        .route("/v1/health", get(health))
        .route("/v1/registration-grants", post(issue_registration_grant))
        .route("/v1/hosts/register", post(register_host))
        .route("/v1/organizations", post(create_organization))
        .route("/v1/goals", post(create_goal))
        .route(
            "/v1/organizations/{organization_id}/goals/{goal_id}/ancestry",
            get(goal_ancestry),
        )
        .route("/v1/projects", post(create_project))
        .route("/v1/workspaces", post(create_workspace))
        .route("/v1/agent-profile-revisions", post(append_profile_revision))
        .route("/v1/agent-instances", post(create_agent_instance))
        .route(
            "/v1/work-graph/items/{work_item_id}",
            post(register_work_item),
        )
        .route("/v1/work-graph/parents", post(set_parent))
        .route("/v1/work-graph/dependencies", post(add_dependency))
        .route("/v1/work-graph/comments", post(add_comment))
        .with_state(state)
}

/// Add CP-07 wake/checkout routes without making the earlier repository
/// constructor signatures a breaking public API.
pub fn router_with_control_repositories(
    plane: Arc<ControlPlane>,
    bootstrap_credential: BootstrapCredential,
    scope_repository: Option<Arc<dyn ScopeRepository>>,
    agent_repository: Option<Arc<dyn AgentRepository>>,
    work_graph_repository: Option<Arc<dyn WorkGraphRepository>>,
    wake_repository: Arc<dyn WakeRepository>,
    run_binding_repository: Option<Arc<dyn RunBindingRepository>>,
) -> Router {
    let state = ApiState {
        plane,
        bootstrap_credential: Arc::new(bootstrap_credential),
        scope_repository,
        agent_repository,
        work_graph_repository,
        wake_repository: Some(wake_repository),
        run_binding_repository,
    };
    Router::new()
        .route("/v1/health", get(health))
        .route("/v1/registration-grants", post(issue_registration_grant))
        .route("/v1/hosts/register", post(register_host))
        .route("/v1/wakes", post(enqueue_wake))
        .route("/v1/wakes/{work_item_id}/claim", post(claim_wake))
        .route("/v1/work-checkouts", post(checkout_work))
        .route("/v1/work-checkouts/release", post(release_checkout))
        .route("/v1/runtime/run-bindings", post(bind_run))
        .with_state(state)
}

#[derive(serde::Deserialize)]
struct WakeMutation {
    work_item_id: WorkItemId,
    source: WakeSource,
    requested_at: String,
}
#[derive(serde::Deserialize)]
struct ClaimMutation {
    claimed_at: String,
}
#[derive(serde::Deserialize)]
struct CheckoutMutation {
    lease: WorkCheckoutLease,
    now_unix_seconds: u64,
}
#[derive(serde::Deserialize)]
struct ReleaseMutation {
    work_item_id: WorkItemId,
    attempt_id: altai_control_protocol::AttemptId,
}
async fn enqueue_wake(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(request): Json<WakeMutation>,
) -> Result<Json<WakeRequest>, ApiError> {
    require_bootstrap(&state, &headers)?;
    wake(&state)?
        .enqueue(request.work_item_id, request.source, request.requested_at)
        .map(Json)
        .map_err(ApiError::from)
}
async fn claim_wake(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(work_item_id): Path<String>,
    Json(request): Json<ClaimMutation>,
) -> Result<Json<WakeRequest>, ApiError> {
    require_bootstrap(&state, &headers)?;
    wake(&state)?
        .claim_wake(&WorkItemId::new(work_item_id), request.claimed_at)
        .map(Json)
        .map_err(ApiError::from)
}
async fn checkout_work(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(request): Json<CheckoutMutation>,
) -> Result<StatusCode, ApiError> {
    require_bootstrap(&state, &headers)?;
    wake(&state)?
        .checkout(request.lease, request.now_unix_seconds)
        .map_err(ApiError::from)?;
    Ok(StatusCode::CREATED)
}
async fn release_checkout(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(request): Json<ReleaseMutation>,
) -> Result<StatusCode, ApiError> {
    require_bootstrap(&state, &headers)?;
    wake(&state)?
        .release_checkout(&request.work_item_id, &request.attempt_id)
        .map_err(ApiError::from)?;
    Ok(StatusCode::NO_CONTENT)
}
async fn bind_run(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(binding): Json<RunBinding>,
) -> Result<Json<RunBinding>, ApiError> {
    require_bootstrap(&state, &headers)?;
    run_bindings(&state)?
        .bind(binding)
        .map(Json)
        .map_err(ApiError::from)
}

#[derive(serde::Deserialize)]
struct ParentMutation {
    work_item_id: WorkItemId,
    parent_work_item_id: Option<WorkItemId>,
}

async fn register_work_item(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(work_item_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    require_bootstrap(&state, &headers)?;
    work_graph(&state)?
        .register_work_item(WorkItemId::new(work_item_id))
        .map_err(ApiError::from)?;
    Ok(StatusCode::CREATED)
}

async fn set_parent(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(request): Json<ParentMutation>,
) -> Result<StatusCode, ApiError> {
    require_bootstrap(&state, &headers)?;
    work_graph(&state)?
        .set_parent(request.work_item_id, request.parent_work_item_id)
        .map_err(ApiError::from)?;
    Ok(StatusCode::NO_CONTENT)
}
async fn add_dependency(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(dependency): Json<WorkDependency>,
) -> Result<StatusCode, ApiError> {
    require_bootstrap(&state, &headers)?;
    work_graph(&state)?
        .add_dependency(dependency)
        .map_err(ApiError::from)?;
    Ok(StatusCode::CREATED)
}
async fn add_comment(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(comment): Json<WorkComment>,
) -> Result<StatusCode, ApiError> {
    require_bootstrap(&state, &headers)?;
    work_graph(&state)?
        .add_comment(comment)
        .map_err(ApiError::from)?;
    Ok(StatusCode::CREATED)
}

async fn append_profile_revision(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(revision): Json<AgentProfileRevision>,
) -> Result<StatusCode, ApiError> {
    require_bootstrap(&state, &headers)?;
    agent_repository(&state)?
        .append_profile_revision(revision)
        .map_err(ApiError::from)?;
    Ok(StatusCode::CREATED)
}

async fn create_agent_instance(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(instance): Json<AgentInstance>,
) -> Result<StatusCode, ApiError> {
    require_bootstrap(&state, &headers)?;
    agent_repository(&state)?
        .create_instance(instance)
        .map_err(ApiError::from)?;
    Ok(StatusCode::CREATED)
}

async fn goal_ancestry(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path((organization_id, goal_id)): Path<(String, String)>,
) -> Result<Json<Vec<Goal>>, ApiError> {
    require_bootstrap(&state, &headers)?;
    scope(&state)?
        .goal_ancestry(&OrganizationId::new(organization_id), &GoalId::new(goal_id))
        .map(Json)
        .map_err(ApiError::from)
}

async fn create_organization(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(organization): Json<Organization>,
) -> Result<StatusCode, ApiError> {
    require_bootstrap(&state, &headers)?;
    scope(&state)?
        .create_organization(organization)
        .map_err(ApiError::from)?;
    Ok(StatusCode::CREATED)
}

async fn create_goal(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(goal): Json<Goal>,
) -> Result<StatusCode, ApiError> {
    require_bootstrap(&state, &headers)?;
    scope(&state)?.create_goal(goal).map_err(ApiError::from)?;
    Ok(StatusCode::CREATED)
}

async fn create_project(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(project): Json<Project>,
) -> Result<StatusCode, ApiError> {
    require_bootstrap(&state, &headers)?;
    scope(&state)?
        .create_project(project)
        .map_err(ApiError::from)?;
    Ok(StatusCode::CREATED)
}

async fn create_workspace(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(workspace): Json<ProjectWorkspace>,
) -> Result<StatusCode, ApiError> {
    require_bootstrap(&state, &headers)?;
    scope(&state)?
        .create_workspace(workspace)
        .map_err(ApiError::from)?;
    Ok(StatusCode::CREATED)
}

fn scope(state: &ApiState) -> Result<&Arc<dyn ScopeRepository>, ApiError> {
    state.scope_repository.as_ref().ok_or(ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "scope_repository_unavailable",
    })
}

fn agent_repository(state: &ApiState) -> Result<&Arc<dyn AgentRepository>, ApiError> {
    state.agent_repository.as_ref().ok_or(ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "agent_repository_unavailable",
    })
}

fn work_graph(state: &ApiState) -> Result<&Arc<dyn WorkGraphRepository>, ApiError> {
    state.work_graph_repository.as_ref().ok_or(ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "work_graph_repository_unavailable",
    })
}
fn wake(state: &ApiState) -> Result<&Arc<dyn WakeRepository>, ApiError> {
    state.wake_repository.as_ref().ok_or(ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "wake_repository_unavailable",
    })
}
fn run_bindings(state: &ApiState) -> Result<&Arc<dyn RunBindingRepository>, ApiError> {
    state.run_binding_repository.as_ref().ok_or(ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "run_binding_repository_unavailable",
    })
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

impl From<ScopeError> for ApiError {
    fn from(value: ScopeError) -> Self {
        match value {
            ScopeError::AlreadyExists { .. } => Self {
                status: StatusCode::CONFLICT,
                code: "already_exists",
            },
            ScopeError::NotFound { .. } => Self {
                status: StatusCode::NOT_FOUND,
                code: "not_found",
            },
            ScopeError::CrossOrganization { .. } => Self {
                status: StatusCode::FORBIDDEN,
                code: "cross_organization",
            },
            ScopeError::GoalCycle { .. } => Self {
                status: StatusCode::CONFLICT,
                code: "goal_cycle",
            },
            ScopeError::Internal { .. } => Self {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                code: "scope_repository_unavailable",
            },
        }
    }
}

impl From<AgentRepositoryError> for ApiError {
    fn from(value: AgentRepositoryError) -> Self {
        match value {
            AgentRepositoryError::AlreadyExists { .. }
            | AgentRepositoryError::ReportingCycle { .. } => Self {
                status: StatusCode::CONFLICT,
                code: "agent_conflict",
            },
            AgentRepositoryError::NotFound { .. } => Self {
                status: StatusCode::NOT_FOUND,
                code: "not_found",
            },
            AgentRepositoryError::NotDispatchable { .. } => Self {
                status: StatusCode::CONFLICT,
                code: "agent_not_dispatchable",
            },
            AgentRepositoryError::Internal { .. } => Self {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                code: "agent_repository_unavailable",
            },
        }
    }
}

impl From<WorkGraphError> for ApiError {
    fn from(value: WorkGraphError) -> Self {
        match value {
            WorkGraphError::NotFound { .. } => Self {
                status: StatusCode::NOT_FOUND,
                code: "not_found",
            },
            WorkGraphError::AlreadyExists { .. } | WorkGraphError::ParentCycle { .. } => Self {
                status: StatusCode::CONFLICT,
                code: "work_graph_conflict",
            },
            WorkGraphError::Internal { .. } => Self {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                code: "work_graph_repository_unavailable",
            },
        }
    }
}
impl From<WakeError> for ApiError {
    fn from(value: WakeError) -> Self {
        match value {
            WakeError::ActiveCheckout { .. } | WakeError::AlreadyClaimed { .. } => Self {
                status: StatusCode::CONFLICT,
                code: "wake_conflict",
            },
            WakeError::NotFound { .. } => Self {
                status: StatusCode::NOT_FOUND,
                code: "not_found",
            },
            WakeError::Internal { .. } => Self {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                code: "wake_repository_unavailable",
            },
        }
    }
}

impl From<RunBindingError> for ApiError {
    fn from(value: RunBindingError) -> Self {
        match value {
            RunBindingError::Conflict { .. } => Self {
                status: StatusCode::CONFLICT,
                code: "run_binding_conflict",
            },
            RunBindingError::Internal { .. } => Self {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                code: "run_binding_repository_unavailable",
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
    use crate::{
        ControlPlaneConfig, ControlPlaneStore, InMemoryAgentRepository, InMemoryScopeRepository,
        InMemoryWakeRepository, SqliteRunBindingRepository,
    };
    use altai_control_protocol::{
        AgentInstanceId, AttemptId, HostCapabilities, HostRegistration, Organization,
        OrganizationId, Revision, RunBinding, RunId, WorkItemId, WorkspaceId,
        CONTROL_PLANE_PROTOCOL_MAJOR,
    };
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    fn app() -> Router {
        let plane = Arc::new(
            ControlPlane::bootstrap(ControlPlaneConfig {
                service_version: "0.1.0".to_string(),
                store: ControlPlaneStore::Sqlite {
                    database_path: "/tmp/control-plane-test.db".to_string(),
                },
                registration_ttl_seconds: 60,
            })
            .unwrap(),
        );
        router_with_repositories(
            plane,
            BootstrapCredential::from_plaintext("test-bootstrap-token").unwrap(),
            Some(Arc::new(InMemoryScopeRepository::default())),
            Some(Arc::new(InMemoryAgentRepository::default())),
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

    #[tokio::test]
    async fn scope_mutations_require_bootstrap_and_reject_duplicates() {
        let organization = Organization {
            id: OrganizationId::new("transport-test"),
            name: "Transport test".to_string(),
            revision: Revision::INITIAL,
            created_at: "2026-08-13T00:00:00Z".to_string(),
            updated_at: "2026-08-13T00:00:00Z".to_string(),
        };
        let request = || {
            Request::builder()
                .method("POST")
                .uri("/v1/organizations")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&organization).unwrap()))
                .unwrap()
        };
        assert_eq!(
            app().clone().oneshot(request()).await.unwrap().status(),
            StatusCode::UNAUTHORIZED
        );

        let authorized = |request: Request<Body>| {
            let (mut parts, body) = request.into_parts();
            parts.headers.insert(
                AUTHORIZATION,
                "Bearer test-bootstrap-token".parse().unwrap(),
            );
            Request::from_parts(parts, body)
        };
        let app = app();
        assert_eq!(
            app.clone()
                .oneshot(authorized(request()))
                .await
                .unwrap()
                .status(),
            StatusCode::CREATED
        );
        assert_eq!(
            app.oneshot(authorized(request())).await.unwrap().status(),
            StatusCode::CONFLICT
        );
    }

    #[test]
    fn short_bootstrap_tokens_are_rejected() {
        assert!(BootstrapCredential::from_plaintext("too-short").is_err());
    }

    #[tokio::test]
    async fn wake_mutations_require_bearer_and_enqueue_when_authorized() {
        let plane = Arc::new(
            ControlPlane::bootstrap(ControlPlaneConfig {
                service_version: "0.1.0".into(),
                store: ControlPlaneStore::Sqlite {
                    database_path: "/tmp/control-plane-test.db".into(),
                },
                registration_ttl_seconds: 60,
            })
            .unwrap(),
        );
        let app = router_with_control_repositories(
            plane,
            BootstrapCredential::from_plaintext("test-bootstrap-token").unwrap(),
            None,
            None,
            None,
            Arc::new(InMemoryWakeRepository::default()),
            None,
        );
        let body = serde_json::json!({ "work_item_id": { "type": "work_item_id", "value": "work-1" }, "source": "manual", "requested_at": "now" }).to_string();
        let request = || {
            Request::builder()
                .method("POST")
                .uri("/v1/wakes")
                .header("content-type", "application/json")
                .body(Body::from(body.clone()))
                .unwrap()
        };
        assert_eq!(
            app.clone().oneshot(request()).await.unwrap().status(),
            StatusCode::UNAUTHORIZED
        );
        let mut authorized = request();
        authorized.headers_mut().insert(
            AUTHORIZATION,
            "Bearer test-bootstrap-token".parse().unwrap(),
        );
        assert_eq!(
            app.oneshot(authorized).await.unwrap().status(),
            StatusCode::OK
        );
    }

    #[tokio::test]
    async fn run_bindings_require_bearer_and_are_immutable() {
        let directory = tempfile::tempdir().unwrap();
        let plane = Arc::new(
            ControlPlane::bootstrap(ControlPlaneConfig {
                service_version: "0.1.0".into(),
                store: ControlPlaneStore::Sqlite {
                    database_path: directory.path().join("control.db").display().to_string(),
                },
                registration_ttl_seconds: 60,
            })
            .unwrap(),
        );
        let app = router_with_control_repositories(
            plane,
            BootstrapCredential::from_plaintext("test-bootstrap-token").unwrap(),
            None,
            None,
            None,
            Arc::new(InMemoryWakeRepository::default()),
            Some(Arc::new(
                SqliteRunBindingRepository::open(&directory.path().join("work.db")).unwrap(),
            )),
        );
        let binding = RunBinding {
            attempt_id: AttemptId::new("attempt-1"),
            work_item_id: WorkItemId::new("work-1"),
            owner_agent_instance_id: AgentInstanceId::new("agent-1"),
            run_id: RunId::new("run-1"),
            bound_at_unix_seconds: 1,
        };
        let request = |binding: RunBinding, authenticated: bool| {
            let mut request = Request::builder()
                .method("POST")
                .uri("/v1/runtime/run-bindings")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&binding).unwrap()))
                .unwrap();
            if authenticated {
                request.headers_mut().insert(
                    AUTHORIZATION,
                    "Bearer test-bootstrap-token".parse().unwrap(),
                );
            }
            request
        };
        assert_eq!(
            app.clone()
                .oneshot(request(binding.clone(), false))
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            app.clone()
                .oneshot(request(binding.clone(), true))
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
        assert_eq!(
            app.clone()
                .oneshot(request(binding.clone(), true))
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
        let mut conflicting = binding;
        conflicting.run_id = RunId::new("run-2");
        assert_eq!(
            app.oneshot(request(conflicting, true))
                .await
                .unwrap()
                .status(),
            StatusCode::CONFLICT
        );
    }
}
