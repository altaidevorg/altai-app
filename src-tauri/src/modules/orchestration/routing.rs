//! Smart routing engine (plan §H2).
//!
//! Routes task phases (planning, implementation, review) to configured agent
//! profiles and runners. Model identifiers stay provider-agnostic: this module
//! never invents a model when the selected profile has none.

use serde::Serialize;

use super::workflow::PermissionMode;
use super::workflow_v2::{
    AgentProfile, AgentRole, AgentsConfig, Reasoning, RoutingConfig, RunnerConfig,
    WorkflowConfigV2, NATIVE_RUNNER,
};

// ---------------------------------------------------------------------------
// Task phase
// ---------------------------------------------------------------------------

/// Which stage of the task lifecycle this routing decision applies to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskPhase {
    Planning,
    Implementation,
    Review,
}

impl TaskPhase {
    pub fn name(self) -> &'static str {
        match self {
            Self::Planning => "planning",
            Self::Implementation => "implementation",
            Self::Review => "review",
        }
    }
}

// ---------------------------------------------------------------------------
// Routing decision
// ---------------------------------------------------------------------------

/// Which configuration layer selected the agent profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingSource {
    /// Explicit profile selection in the `routing:` section of WORKFLOW.md.
    RoutingOverride,
    /// The phase's conventional profile (planner, worker, or reviewer).
    PhaseDefault,
}

/// The resolved model, runner, and metadata for one task phase.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutingDecision {
    pub phase: TaskPhase,
    pub profile: AgentRole,
    /// Optional workflow-level override. `None` delegates to the runtime's
    /// configured provider/model selection.
    pub model_id: Option<String>,
    pub runner_kind: String,
    /// Reasoning effort from the agent profile, if specified.
    pub reasoning: Option<Reasoning>,
    /// Permission mode from the agent profile, if specified.
    pub permissions: Option<PermissionMode>,
    /// Reserved for explicit, policy-checked fallbacks. The current schema has
    /// no fallback policy, so this is intentionally empty.
    pub fallback_models: Vec<String>,
    pub source: RoutingSource,
    /// Human-readable explanation of why this model was selected.
    pub reason: String,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoutingError {
    /// The selected profile is absent from the `agents:` section.
    ProfileNotConfigured {
        phase: TaskPhase,
        profile: AgentRole,
    },
    /// The resolved runner is not in the `runner.allow` list.
    RunnerNotAllowed {
        runner_kind: String,
        allowed: Vec<String>,
    },
}

impl std::fmt::Display for RoutingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProfileNotConfigured { phase, profile } => write!(
                f,
                "no {:?} agent profile is configured for the {} phase",
                profile,
                phase.name()
            ),
            Self::RunnerNotAllowed {
                runner_kind,
                allowed,
            } => write!(
                f,
                "runner '{runner_kind}' is not in the allowed list: {allowed:?}"
            ),
        }
    }
}

impl std::error::Error for RoutingError {}

pub type RoutingResult = Result<RoutingDecision, RoutingError>;

// ---------------------------------------------------------------------------
// Routing engine
// ---------------------------------------------------------------------------

/// Resolves routing decisions from the v2 workflow configuration. Stateless
/// and deterministic: the same config + phase always produces the same decision.
#[derive(Clone, Debug, Default)]
pub struct RoutingEngine {
    agents: AgentsConfig,
    routing: RoutingConfig,
    runner: RunnerConfig,
}

impl RoutingEngine {
    pub fn new(agents: AgentsConfig, routing: RoutingConfig, runner: RunnerConfig) -> Self {
        Self {
            agents,
            routing,
            runner,
        }
    }

    /// Construct from a parsed v2 workflow config.
    pub fn from_config_v2(config: &WorkflowConfigV2) -> Self {
        Self::new(
            config.agents.clone(),
            config.routing.clone().unwrap_or_default(),
            config.runner.clone(),
        )
    }

    /// Resolve the routing for a single task phase.
    pub fn route(&self, phase: TaskPhase) -> RoutingResult {
        let (selected_role, source) = self.selected_role(phase);
        let profile = self.profile(selected_role);
        if source == RoutingSource::RoutingOverride && profile.is_none() {
            return Err(RoutingError::ProfileNotConfigured {
                phase,
                profile: selected_role,
            });
        }
        let model_id = profile
            .and_then(|profile| profile.model_id.as_deref())
            .filter(|model| !model.trim().is_empty())
            .map(str::to_owned);

        // Runner resolution uses the configured default. Production allows only
        // the ALTAI/IsanAgent native runner (MockRunner is tests-only).
        let runner_kind = self.runner.default.clone();
        if runner_kind != NATIVE_RUNNER {
            return Err(RoutingError::RunnerNotAllowed {
                runner_kind,
                allowed: vec![NATIVE_RUNNER.to_string()],
            });
        }
        if !self.runner.allow.is_empty() && !self.runner.allow.contains(&runner_kind) {
            return Err(RoutingError::RunnerNotAllowed {
                runner_kind,
                allowed: self.runner.allow.clone(),
            });
        }
        if self
            .runner
            .allow
            .iter()
            .any(|allowed| allowed != NATIVE_RUNNER)
        {
            return Err(RoutingError::RunnerNotAllowed {
                runner_kind,
                allowed: vec![NATIVE_RUNNER.to_string()],
            });
        }

        // Automatic cross-profile fallback would silently change reasoning and
        // permissions. Add fallbacks only once the schema can express and
        // validate those constraints.
        let fallback_models = Vec::new();
        let reasoning = profile.and_then(|profile| profile.reasoning);
        let permissions = profile.and_then(|profile| profile.permissions);

        let profile_reason = match source {
            RoutingSource::RoutingOverride => format!(
                "{} routing explicitly selected the {:?} profile",
                phase.name(),
                selected_role
            ),
            RoutingSource::PhaseDefault => format!(
                "{} uses its default {:?} profile",
                phase.name(),
                selected_role
            ),
        };
        let reason = match &model_id {
            Some(model_id) => format!("{profile_reason} with model override '{model_id}'"),
            None => format!(
                "{profile_reason}; provider and model resolve from the configured runtime target"
            ),
        };

        Ok(RoutingDecision {
            phase,
            profile: selected_role,
            model_id,
            runner_kind,
            reasoning,
            permissions,
            fallback_models,
            source,
            reason,
        })
    }

    /// Route all three phases at once.
    pub fn route_all(&self) -> Result<Vec<RoutingDecision>, RoutingError> {
        [
            TaskPhase::Planning,
            TaskPhase::Implementation,
            TaskPhase::Review,
        ]
        .into_iter()
        .map(|phase| self.route(phase))
        .collect()
    }

    // ----- helpers -----

    fn selected_role(&self, phase: TaskPhase) -> (AgentRole, RoutingSource) {
        let configured = match phase {
            TaskPhase::Planning => self.routing.planner,
            TaskPhase::Implementation => self.routing.implementation,
            TaskPhase::Review => self.routing.review,
        };
        match configured {
            Some(role) => (role, RoutingSource::RoutingOverride),
            None => (
                match phase {
                    TaskPhase::Planning => AgentRole::Planner,
                    TaskPhase::Implementation => AgentRole::Worker,
                    TaskPhase::Review => AgentRole::Reviewer,
                },
                RoutingSource::PhaseDefault,
            ),
        }
    }

    fn profile(&self, role: AgentRole) -> Option<&AgentProfile> {
        match role {
            AgentRole::Planner => self.agents.planner.as_ref(),
            AgentRole::Worker => self.agents.worker.as_ref(),
            AgentRole::Reviewer => self.agents.reviewer.as_ref(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::workflow_v2::AgentProfile;
    use super::*;

    fn profile(model: &str, reasoning: Option<Reasoning>) -> AgentProfile {
        AgentProfile {
            model_id: Some(model.into()),
            reasoning,
            permissions: None,
            tools: None,
        }
    }

    fn engine(agents: AgentsConfig, routing: RoutingConfig, runner: RunnerConfig) -> RoutingEngine {
        RoutingEngine::new(agents, routing, runner)
    }

    #[test]
    fn agent_profile_model_is_used() {
        let agents = AgentsConfig {
            worker: Some(profile("claude-sonnet-4", None)),
            ..AgentsConfig::default()
        };
        let eng = engine(agents, RoutingConfig::default(), RunnerConfig::default());
        let d = eng.route(TaskPhase::Implementation).unwrap();
        assert_eq!(d.model_id.as_deref(), Some("claude-sonnet-4"));
        assert_eq!(d.profile, AgentRole::Worker);
        assert_eq!(d.source, RoutingSource::PhaseDefault);
    }

    #[test]
    fn planner_profile_used_for_planning() {
        let agents = AgentsConfig {
            planner: Some(profile("o3", Some(Reasoning::High))),
            ..AgentsConfig::default()
        };
        let eng = engine(agents, RoutingConfig::default(), RunnerConfig::default());
        let d = eng.route(TaskPhase::Planning).unwrap();
        assert_eq!(d.model_id.as_deref(), Some("o3"));
        assert_eq!(d.reasoning, Some(Reasoning::High));
    }

    #[test]
    fn reviewer_profile_used_for_review() {
        let agents = AgentsConfig {
            reviewer: Some(profile("o3-mini", None)),
            ..AgentsConfig::default()
        };
        let eng = engine(agents, RoutingConfig::default(), RunnerConfig::default());
        let d = eng.route(TaskPhase::Review).unwrap();
        assert_eq!(d.model_id.as_deref(), Some("o3-mini"));
    }

    #[test]
    fn routing_override_selects_a_named_profile() {
        let agents = AgentsConfig {
            planner: Some(profile("gemini-2.5-pro", Some(Reasoning::High))),
            worker: Some(profile("glm-5", None)),
            ..AgentsConfig::default()
        };
        let routing = RoutingConfig {
            implementation: Some(AgentRole::Planner),
            ..RoutingConfig::default()
        };
        let eng = engine(agents, routing, RunnerConfig::default());
        let d = eng.route(TaskPhase::Implementation).unwrap();
        assert_eq!(d.model_id.as_deref(), Some("gemini-2.5-pro"));
        assert_eq!(d.profile, AgentRole::Planner);
        assert_eq!(d.reasoning, Some(Reasoning::High));
        assert_eq!(d.source, RoutingSource::RoutingOverride);
    }

    #[test]
    fn missing_profile_is_an_explicit_error() {
        let routing = RoutingConfig {
            implementation: Some(AgentRole::Planner),
            ..RoutingConfig::default()
        };
        let err = engine(AgentsConfig::default(), routing, RunnerConfig::default())
            .route(TaskPhase::Implementation)
            .unwrap_err();
        assert_eq!(
            err,
            RoutingError::ProfileNotConfigured {
                phase: TaskPhase::Implementation,
                profile: AgentRole::Planner,
            }
        );
    }

    #[test]
    fn missing_model_delegates_to_the_configured_runtime_target() {
        let decision = RoutingEngine::default()
            .route(TaskPhase::Implementation)
            .unwrap();
        assert_eq!(decision.profile, AgentRole::Worker);
        assert_eq!(decision.model_id, None);
        assert!(decision.reason.contains("configured runtime target"));
    }

    #[test]
    fn route_all_preserves_the_first_routing_error() {
        let routing = RoutingConfig {
            implementation: Some(AgentRole::Reviewer),
            ..RoutingConfig::default()
        };
        let err = engine(AgentsConfig::default(), routing, RunnerConfig::default())
            .route_all()
            .unwrap_err();
        assert_eq!(
            err,
            RoutingError::ProfileNotConfigured {
                phase: TaskPhase::Implementation,
                profile: AgentRole::Reviewer,
            }
        );
    }

    #[test]
    fn disallowed_runner_errors() {
        let agents = AgentsConfig {
            planner: Some(profile("glm-5", None)),
            ..AgentsConfig::default()
        };
        let runner = RunnerConfig {
            default: "external".into(),
            allow: vec!["native".into()],
        };
        let eng = engine(agents, RoutingConfig::default(), runner);
        let err = eng.route(TaskPhase::Planning).unwrap_err();
        assert!(matches!(
            err,
            RoutingError::RunnerNotAllowed { runner_kind, .. } if runner_kind == "external"
        ));
    }

    #[test]
    fn empty_allow_list_still_requires_native() {
        let agents = AgentsConfig {
            planner: Some(profile("glm-5", None)),
            ..AgentsConfig::default()
        };
        let runner = RunnerConfig {
            default: "custom-runner".into(),
            allow: vec![],
        };
        let eng = engine(agents, RoutingConfig::default(), runner);
        let err = eng.route(TaskPhase::Planning).unwrap_err();
        assert!(matches!(
            err,
            RoutingError::RunnerNotAllowed { runner_kind, .. } if runner_kind == "custom-runner"
        ));
    }

    #[test]
    fn allow_list_with_non_native_entry_errors() {
        let agents = AgentsConfig {
            planner: Some(profile("glm-5", None)),
            ..AgentsConfig::default()
        };
        let runner = RunnerConfig {
            default: "native".into(),
            allow: vec!["native".into(), "external".into()],
        };
        let eng = engine(agents, RoutingConfig::default(), runner);
        let err = eng.route(TaskPhase::Planning).unwrap_err();
        assert!(matches!(err, RoutingError::RunnerNotAllowed { .. }));
    }

    #[test]
    fn does_not_invent_cross_profile_fallbacks() {
        let agents = AgentsConfig {
            planner: Some(profile("gemini-2.5-pro", None)),
            worker: Some(profile("glm-5", None)),
            reviewer: Some(profile("claude-sonnet-4", None)),
        };
        let eng = engine(agents, RoutingConfig::default(), RunnerConfig::default());
        let d = eng.route(TaskPhase::Planning).unwrap();
        assert!(d.fallback_models.is_empty());
    }

    #[test]
    fn reason_for_routing_override() {
        let agents = AgentsConfig {
            planner: Some(profile("glm-5", None)),
            ..AgentsConfig::default()
        };
        let routing = RoutingConfig {
            implementation: Some(AgentRole::Planner),
            ..RoutingConfig::default()
        };
        let eng = engine(agents, routing, RunnerConfig::default());
        let d = eng.route(TaskPhase::Implementation).unwrap();
        assert!(d.reason.contains("explicitly selected"));
        assert!(d.reason.contains("glm-5"));
    }

    #[test]
    fn permissions_propagate_from_profile() {
        let agents = AgentsConfig {
            worker: Some(AgentProfile {
                model_id: Some("claude-sonnet-4".into()),
                reasoning: None,
                permissions: Some(PermissionMode::Ask),
                tools: None,
            }),
            ..AgentsConfig::default()
        };
        let eng = engine(agents, RoutingConfig::default(), RunnerConfig::default());
        let d = eng.route(TaskPhase::Implementation).unwrap();
        assert_eq!(d.permissions, Some(PermissionMode::Ask));
    }

    #[test]
    fn from_config_v2_routes_correctly() {
        let config = WorkflowConfigV2 {
            version: 2,
            orchestration: Default::default(),
            runner: RunnerConfig {
                default: "native".into(),
                allow: vec!["native".into()],
            },
            agents: AgentsConfig {
                planner: Some(profile("gemini-2.5-pro", Some(Reasoning::High))),
                worker: Some(profile("glm-5", None)),
                reviewer: Some(profile("claude-sonnet-4", None)),
            },
            environment: Default::default(),
            quality: Default::default(),
            budgets: Default::default(),
            hooks: None,
            routing: Some(RoutingConfig {
                implementation: Some(AgentRole::Planner),
                ..RoutingConfig::default()
            }),
            handoff: None,
        };
        let eng = RoutingEngine::from_config_v2(&config);

        // Planning: planner profile.
        let planning = eng.route(TaskPhase::Planning).unwrap();
        assert_eq!(planning.model_id.as_deref(), Some("gemini-2.5-pro"));
        assert_eq!(planning.reasoning, Some(Reasoning::High));

        // Implementation: explicit planner-profile selection.
        let impl_d = eng.route(TaskPhase::Implementation).unwrap();
        assert_eq!(impl_d.model_id.as_deref(), Some("gemini-2.5-pro"));
        assert_eq!(impl_d.source, RoutingSource::RoutingOverride);

        // Review: reviewer profile.
        let review = eng.route(TaskPhase::Review).unwrap();
        assert_eq!(review.model_id.as_deref(), Some("claude-sonnet-4"));
    }
}
