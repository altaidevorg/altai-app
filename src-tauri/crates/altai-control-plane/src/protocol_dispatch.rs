//! The single protocol seam between every ALTAI surface and the control plane.
//!
//! [`ProtocolDispatcher`] turns a [`ProtocolRequest<ProtocolCommand>`] into a
//! [`ProtocolResponse<ProtocolOutcome>`]; the axum transport calls the very
//! same [`ProtocolDispatcher::execute`] that an in-process (local) caller
//! calls, so "command/query/event conformance across local and deployed
//! transports" is structural — there is no second implementation to drift.
//!
//! Capabilities are derived from what the deployment actually serves
//! ([`capabilities_from_wiring`]); nothing advertises what it cannot serve.
//! Commands whose producers do not exist yet (activity queries, event replay —
//! package 060) answer a typed `PolicyDenied` identically on every transport
//! rather than guessing or silently 404ing.

use altai_control_protocol::{
    CapabilityNegotiationRequest, CapabilityNegotiationResponse, ControlPlaneCapabilities,
    ControlErrorCode, DeploymentMode, ProtocolCommand, ProtocolError, ProtocolOutcome,
    ProtocolRequest, ProtocolResponse, ProtocolVersion,
};

/// Capabilities honestly derived from the repositories wired into the
/// transport. Domains with protocol-facing routes advertise `true`; budgets,
/// evidence, activity audit, event replay and workspace scopes stay `false`
/// until they have serving.
pub fn capabilities_from_wiring(
    scope_repository: bool,
    agent_repository: bool,
    work_graph_repository: bool,
    attempt_repository: bool,
    routine_repository: bool,
    approval_repository: bool,
) -> ControlPlaneCapabilities {
    ControlPlaneCapabilities {
        organizations: scope_repository,
        goals: scope_repository,
        projects: scope_repository,
        workspaces: scope_repository,
        agents: agent_repository,
        work_graph: work_graph_repository,
        attempts: attempt_repository,
        routines: routine_repository,
        approvals: approval_repository,
        budgets: false,
        evidence: false,
        activity_audit: false,
        event_replay: false,
        workspace_scopes: false,
    }
}

/// Dispatcher for the public versioned control protocol. Holds only what it
/// serves; it never mutates and reaches no domain store beyond negotiation
/// state (commands whose producers are absent answer typed errors).
pub struct ProtocolDispatcher {
    deployment_mode: DeploymentMode,
    capabilities: ControlPlaneCapabilities,
}

impl ProtocolDispatcher {
    pub fn new(
        deployment_mode: DeploymentMode,
        capabilities: ControlPlaneCapabilities,
    ) -> Self {
        Self {
            deployment_mode,
            capabilities,
        }
    }

    pub fn capabilities(&self) -> &ControlPlaneCapabilities {
        &self.capabilities
    }

    /// Answer a capability negotiation request: wire compatibility plus the
    /// subset of required capabilities this deployment cannot serve.
    pub fn negotiate(
        &self,
        request: &CapabilityNegotiationRequest,
    ) -> CapabilityNegotiationResponse {
        CapabilityNegotiationResponse::evaluate(
            ProtocolVersion::CURRENT,
            self.deployment_mode,
            self.capabilities.clone(),
            request,
        )
    }

    /// Execute one framed protocol request. The protocol major version is
    /// gated first (typed `PolicyDenied` on mismatch), then the command
    /// dispatches. Domain failures are values inside the envelope — transport
    /// status codes stay reserved for transport problems.
    pub fn execute(
        &self,
        request: &ProtocolRequest<ProtocolCommand>,
    ) -> ProtocolResponse<ProtocolOutcome> {
        if !request.version.is_compatible_with(&ProtocolVersion::CURRENT) {
            return ProtocolResponse::error(
                request.id.clone(),
                ProtocolError::new(
                    ControlErrorCode::PolicyDenied,
                    format!(
                        "protocol major version {} is not supported (server: {}.{})",
                        request.version.major,
                        ProtocolVersion::CURRENT.major,
                        ProtocolVersion::CURRENT.minor
                    ),
                ),
            );
        }
        let outcome = match &request.payload {
            ProtocolCommand::NegotiateCapabilities(inner) => {
                Ok(ProtocolOutcome::Negotiated(self.negotiate(inner)))
            }
            ProtocolCommand::QueryActivity(_) => {
                // The activity producer arrives with package 060; until then
                // the dispatcher honestly denies instead of guessing.
                Err(self.unsupported("activity_audit"))
            }
            ProtocolCommand::ReplayEvents(_) => Err(self.unsupported("event_replay")),
        };
        match outcome {
            Ok(value) => ProtocolResponse::ok(request.id.clone(), value),
            Err(error) => ProtocolResponse::error(request.id.clone(), error),
        }
    }

    fn unsupported(&self, capability: &str) -> ProtocolError {
        ProtocolError::new(
            ControlErrorCode::PolicyDenied,
            format!("capability not served by this deployment: {capability}"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BootstrapCredential, ControlPlane, ControlPlaneConfig, ControlPlaneStore,
        InMemoryAgentRepository, InMemoryScopeRepository, InMemoryWakeRepository,
        InMemoryWorkGraphRepository, ProtocolDispatcher, SqliteAttemptRepository,
        SqliteApprovalRepository, SqliteRoutineRepository, SqliteRunBindingRepository,
        router_with_control_repositories,
    };
    use altai_control_protocol::{Actor, OrganizationId, PageRequest};
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode, header::AUTHORIZATION},
    };
    use std::sync::Arc;
    use tower::ServiceExt;

    const BOOTSTRAP_TOKEN: &str = "test-bootstrap-token";

    struct Harness {
        _dir: tempfile::TempDir,
        app: Router,
        dispatcher: Arc<ProtocolDispatcher>,
    }

    fn harness() -> Harness {
        let dir = tempfile::tempdir().unwrap();
        let plane = Arc::new(
            ControlPlane::bootstrap(ControlPlaneConfig {
                service_version: "0.1.0".to_string(),
                store: ControlPlaneStore::Sqlite {
                    database_path: dir.path().join("work.db").display().to_string(),
                },
                registration_ttl_seconds: 60,
            })
            .unwrap(),
        );
        let capabilities = capabilities_from_wiring(true, true, true, true, true, true);
        let dispatcher = Arc::new(ProtocolDispatcher::new(
            DeploymentMode::LocalDaemon,
            capabilities,
        ));
        // The router builder constructs its own dispatcher from the same
        // wiring (all optional repositories present), so the local and
        // deployed sides below share one capability truth by construction.
        let app = router_with_control_repositories(
            plane,
            BootstrapCredential::from_plaintext(BOOTSTRAP_TOKEN).unwrap(),
            Some(Arc::new(InMemoryScopeRepository::default())),
            Some(Arc::new(InMemoryAgentRepository::default())),
            Some(Arc::new(InMemoryWorkGraphRepository::default())),
            Arc::new(InMemoryWakeRepository::default()),
            Some(Arc::new(
                SqliteRunBindingRepository::open(&dir.path().join("bindings.db")).unwrap(),
            )),
            Some(Arc::new(
                SqliteAttemptRepository::open(&dir.path().join("attempts.db")).unwrap(),
            )),
            Some(Arc::new(
                SqliteRoutineRepository::open(&dir.path().join("routines.db")).unwrap(),
            )),
            Some(Arc::new(
                SqliteApprovalRepository::open(&dir.path().join("approvals.db")).unwrap(),
            )),
        );
        Harness {
            _dir: dir,
            app,
            dispatcher,
        }
    }

    fn request(id: &str, version_major: u16, payload: ProtocolCommand) -> ProtocolRequest<ProtocolCommand> {
        ProtocolRequest {
            id: id.to_string(),
            version: ProtocolVersion::new(version_major, 0),
            actor: Actor::System {
                component: "conformance-test".to_string(),
            },
            payload,
        }
    }

    fn negotiate_payload() -> ProtocolCommand {
        ProtocolCommand::NegotiateCapabilities(CapabilityNegotiationRequest {
            client_version: ProtocolVersion::CURRENT,
            client_name: "altai-conformance".to_string(),
            required_capabilities: vec!["organizations".to_string()],
        })
    }

    async fn post_command(
        h: &Harness,
        body: &ProtocolRequest<ProtocolCommand>,
    ) -> ProtocolResponse<ProtocolOutcome> {
        let response = h
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/protocol/commands")
                    .header(
                        AUTHORIZATION,
                        format!("Bearer {BOOTSTRAP_TOKEN}"),
                    )
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), 1_048_576)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[test]
    fn capabilities_reflect_wired_repositories() {
        let wired = capabilities_from_wiring(true, true, true, true, true, true);
        assert!(wired.organizations && wired.agents && wired.work_graph && wired.attempts);
        assert!(!wired.budgets && !wired.evidence && !wired.activity_audit);
        assert!(!wired.event_replay && !wired.workspace_scopes);

        let bare = capabilities_from_wiring(false, false, false, false, false, false);
        assert!(!bare.organizations && !bare.attempts);
    }

    #[tokio::test]
    async fn negotiation_conforms_across_local_and_deployed_transports() {
        let h = harness();
        let body = request("req-1", 1, negotiate_payload());
        let local = h.dispatcher.execute(&body);
        let deployed = post_command(&h, &body).await;
        assert_eq!(local, deployed);
        match local.result {
            Ok(ProtocolOutcome::Negotiated(response)) => {
                assert!(response.compatible);
                assert_eq!(response.deployment_mode, DeploymentMode::LocalDaemon);
            }
            other => panic!("expected negotiated outcome, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn version_mismatch_is_denied_identically_on_both_transports() {
        let h = harness();
        let body = request("req-2", 2, negotiate_payload());
        let local = h.dispatcher.execute(&body);
        let deployed = post_command(&h, &body).await;
        assert_eq!(local, deployed);
        match local.result {
            Err(error) => {
                assert_eq!(error.code, ControlErrorCode::PolicyDenied);
                assert!(error.message.contains("major version 2"));
            }
            other => panic!("expected typed error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn event_replay_is_typed_denied_conformantly() {
        let h = harness();
        let body = request(
            "req-3",
            1,
            ProtocolCommand::ReplayEvents(altai_control_protocol::EventReplayRequest::new(
                OrganizationId::new("org"),
                0,
                None,
            )),
        );
        let local = h.dispatcher.execute(&body);
        let deployed = post_command(&h, &body).await;
        assert_eq!(local, deployed);
        match local.result {
            Err(error) => {
                assert_eq!(error.code, ControlErrorCode::PolicyDenied);
                assert!(error.message.contains("event_replay"));
            }
            other => panic!("expected typed error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn activity_query_is_typed_denied_conformantly() {
        let h = harness();
        let body = request(
            "req-4",
            1,
            ProtocolCommand::QueryActivity(altai_control_protocol::ActivityQueryRequest {
                organization_id: OrganizationId::new("org"),
                page: PageRequest::default(),
                kind: None,
                work_item_id: None,
            }),
        );
        let local = h.dispatcher.execute(&body);
        let deployed = post_command(&h, &body).await;
        assert_eq!(local, deployed);
        match local.result {
            Err(error) => {
                assert_eq!(error.code, ControlErrorCode::PolicyDenied);
                assert!(error.message.contains("activity_audit"));
            }
            other => panic!("expected typed error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn standalone_negotiate_route_matches_the_dispatched_command() {
        let h = harness();
        let negotiation = CapabilityNegotiationRequest {
            client_version: ProtocolVersion::CURRENT,
            client_name: "altai-conformance".to_string(),
            required_capabilities: vec!["organizations".to_string()],
        };
        let response = h
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/protocol/negotiate")
                    .header(
                        AUTHORIZATION,
                        format!("Bearer {BOOTSTRAP_TOKEN}"),
                    )
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&negotiation).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), 1_048_576)
            .await
            .unwrap();
        let standalone: CapabilityNegotiationResponse = serde_json::from_slice(&bytes).unwrap();
        let via_command = h
            .dispatcher
            .execute(&request("req-5", 1, negotiate_payload()));
        match via_command.result {
            Ok(ProtocolOutcome::Negotiated(framed)) => assert_eq!(standalone, framed),
            other => panic!("expected negotiated outcome, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn protocol_routes_require_bootstrap_bearer() {
        let h = harness();
        let response = h
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/protocol/commands")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(&request("req-6", 1, negotiate_payload())).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
