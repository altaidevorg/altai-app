//! Smart routing engine (plan §H2).
//!
//! Routes task phases (planning, implementation, review) to the appropriate
//! model and runner based on agent profiles, routing overrides, and runner
//! availability. Produces explainable decisions with fallback chains and
//! routing-attribution so the user can see exactly why a model was chosen.

use serde::Serialize;

use super::workflow::PermissionMode;
use super::workflow_v2::{AgentsConfig, Reasoning, RoutingConfig, RunnerConfig, WorkflowConfigV2};

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

/// Fallback model used when no configuration specifies one.
pub const DEFAULT_MODEL: &str = "gpt-4o";

// ---------------------------------------------------------------------------
// Routing decision
// ---------------------------------------------------------------------------

/// Which configuration layer produced the model choice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingSource {
    /// Explicit model override in the `routing:` section of WORKFLOW.md.
    RoutingOverride,
    /// The `agents:` profile for this role.
    AgentProfile,
    /// Hardcoded default (no config specified a model).
    Default,
}

/// The resolved model, runner, and metadata for one task phase.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutingDecision {
    pub phase: TaskPhase,
    pub model_id: String,
    pub runner_kind: String,
    /// Reasoning effort from the agent profile, if specified.
    pub reasoning: Option<Reasoning>,
    /// Permission mode from the agent profile, if specified.
    pub permissions: Option<PermissionMode>,
    /// Models to try if the primary is unavailable (quality trade-off chain).
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
    /// The resolved runner is not in the `runner.allow` list.
    RunnerNotAllowed {
        runner_kind: String,
        allowed: Vec<String>,
    },
}

impl std::fmt::Display for RoutingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
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
        let profile = self.profile_for(phase);
        let routing_override = self.routing_override_for(phase);

        // 1. Model resolution: routing override > agent profile > default.
        let (model_id, source) = if let Some(m) = routing_override {
            (m.to_string(), RoutingSource::RoutingOverride)
        } else if let Some(p) = profile {
            if let Some(m) = &p.model_id {
                (m.clone(), RoutingSource::AgentProfile)
            } else {
                (DEFAULT_MODEL.to_string(), RoutingSource::Default)
            }
        } else {
            (DEFAULT_MODEL.to_string(), RoutingSource::Default)
        };

        // 2. Runner resolution: always the configured default.
        let runner_kind = self.runner.default.clone();

        // 3. Validate runner is allowed.
        if !self.runner.allow.is_empty() && !self.runner.allow.contains(&runner_kind) {
            return Err(RoutingError::RunnerNotAllowed {
                runner_kind,
                allowed: self.runner.allow.clone(),
            });
        }

        // 4. Build fallback chain from other configured models (excluding primary).
        let fallback_models = self.collect_fallback_models(phase, &model_id);

        // 5. Extract profile metadata.
        let reasoning = profile.and_then(|p| p.reasoning);
        let permissions = profile.and_then(|p| p.permissions);

        let reason = match source {
            RoutingSource::RoutingOverride => {
                format!(
                    "model '{model_id}' from routing override for {}",
                    phase.name()
                )
            }
            RoutingSource::AgentProfile => {
                format!("model '{model_id}' from {} agent profile", phase.name())
            }
            RoutingSource::Default => {
                format!(
                    "no model configured for {}; using default '{DEFAULT_MODEL}'",
                    phase.name()
                )
            }
        };

        Ok(RoutingDecision {
            phase,
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
    pub fn route_all(&self) -> Vec<RoutingDecision> {
        [
            TaskPhase::Planning,
            TaskPhase::Implementation,
            TaskPhase::Review,
        ]
        .iter()
        .filter_map(|phase| self.route(*phase).ok())
        .collect()
    }

    // ----- helpers -----

    fn profile_for(&self, phase: TaskPhase) -> Option<&super::workflow_v2::AgentProfile> {
        match phase {
            TaskPhase::Planning => self.agents.planner.as_ref(),
            TaskPhase::Implementation => self.agents.worker.as_ref(),
            TaskPhase::Review => self.agents.reviewer.as_ref(),
        }
    }

    fn routing_override_for(&self, phase: TaskPhase) -> Option<&str> {
        match phase {
            TaskPhase::Planning => self.routing.planner.as_deref(),
            TaskPhase::Implementation => self.routing.implementation.as_deref(),
            TaskPhase::Review => self.routing.review.as_deref(),
        }
    }

    /// Collect all distinct configured models across all roles, excluding the
    /// primary model, as a fallback chain.
    fn collect_fallback_models(&self, _phase: TaskPhase, primary: &str) -> Vec<String> {
        let mut models: Vec<String> = Vec::new();

        for profile in [
            self.agents.planner.as_ref(),
            self.agents.worker.as_ref(),
            self.agents.reviewer.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            if let Some(m) = &profile.model_id {
                if m != primary && !models.contains(m) {
                    models.push(m.clone());
                }
            }
        }

        // Also include routing overrides as fallbacks.
        for m in [
            self.routing.planner.as_deref(),
            self.routing.implementation.as_deref(),
            self.routing.review.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            if m != primary && !models.iter().any(|existing: &String| existing == m) {
                models.push(m.to_string());
            }
        }

        models
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

    // ---- default routing ----

    #[test]
    fn default_engine_routes_to_default_model() {
        let eng = RoutingEngine::default();
        let d = eng.route(TaskPhase::Implementation).unwrap();
        assert_eq!(d.model_id, DEFAULT_MODEL);
        assert_eq!(d.source, RoutingSource::Default);
        assert_eq!(d.runner_kind, "native");
    }

    #[test]
    fn default_engine_routes_all_three_phases() {
        let eng = RoutingEngine::default();
        let decisions = eng.route_all();
        assert_eq!(decisions.len(), 3);
        assert!(decisions.iter().all(|d| d.model_id == DEFAULT_MODEL));
    }

    // ---- agent profile ----

    #[test]
    fn agent_profile_model_is_used() {
        let agents = AgentsConfig {
            worker: Some(profile("claude-sonnet-4", None)),
            ..AgentsConfig::default()
        };
        let eng = engine(agents, RoutingConfig::default(), RunnerConfig::default());
        let d = eng.route(TaskPhase::Implementation).unwrap();
        assert_eq!(d.model_id, "claude-sonnet-4");
        assert_eq!(d.source, RoutingSource::AgentProfile);
    }

    #[test]
    fn planner_profile_used_for_planning() {
        let agents = AgentsConfig {
            planner: Some(profile("o3", Some(Reasoning::High))),
            ..AgentsConfig::default()
        };
        let eng = engine(agents, RoutingConfig::default(), RunnerConfig::default());
        let d = eng.route(TaskPhase::Planning).unwrap();
        assert_eq!(d.model_id, "o3");
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
        assert_eq!(d.model_id, "o3-mini");
    }

    // ---- routing override takes priority ----

    #[test]
    fn routing_override_beats_agent_profile() {
        let agents = AgentsConfig {
            worker: Some(profile("claude-sonnet-4", None)),
            ..AgentsConfig::default()
        };
        let routing = RoutingConfig {
            implementation: Some("gpt-5".into()),
            ..RoutingConfig::default()
        };
        let eng = engine(agents, routing, RunnerConfig::default());
        let d = eng.route(TaskPhase::Implementation).unwrap();
        assert_eq!(d.model_id, "gpt-5");
        assert_eq!(d.source, RoutingSource::RoutingOverride);
    }

    // ---- runner validation ----

    #[test]
    fn disallowed_runner_errors() {
        let runner = RunnerConfig {
            default: "codex".into(),
            allow: vec!["native".into()],
        };
        let eng = engine(AgentsConfig::default(), RoutingConfig::default(), runner);
        let err = eng.route(TaskPhase::Planning).unwrap_err();
        assert!(matches!(
            err,
            RoutingError::RunnerNotAllowed { runner_kind, .. } if runner_kind == "codex"
        ));
    }

    #[test]
    fn empty_allow_list_allows_any_runner() {
        let runner = RunnerConfig {
            default: "custom-runner".into(),
            allow: vec![],
        };
        let eng = engine(AgentsConfig::default(), RoutingConfig::default(), runner);
        let d = eng.route(TaskPhase::Planning).unwrap();
        assert_eq!(d.runner_kind, "custom-runner");
    }

    // ---- fallback chain ----

    #[test]
    fn fallback_chain_excludes_primary_and_deduplicates() {
        let agents = AgentsConfig {
            planner: Some(profile("o3", None)),
            worker: Some(profile("claude-sonnet-4", None)),
            reviewer: Some(profile("o3", None)), // duplicate
        };
        let eng = engine(agents, RoutingConfig::default(), RunnerConfig::default());
        let d = eng.route(TaskPhase::Planning).unwrap();
        // Primary is o3; fallback should be claude-sonnet-4 only (no dup).
        assert_eq!(d.model_id, "o3");
        assert_eq!(d.fallback_models, vec!["claude-sonnet-4".to_string()]);
    }

    #[test]
    fn fallback_includes_routing_overrides() {
        let agents = AgentsConfig {
            worker: Some(profile("claude-sonnet-4", None)),
            ..AgentsConfig::default()
        };
        let routing = RoutingConfig {
            planner: Some("o3".into()),
            ..RoutingConfig::default()
        };
        let eng = engine(agents, routing, RunnerConfig::default());
        let d = eng.route(TaskPhase::Implementation).unwrap();
        // Primary is claude-sonnet-4; fallback includes o3 from routing override.
        assert!(d.fallback_models.contains(&"o3".to_string()));
    }

    #[test]
    fn no_fallbacks_when_only_one_model() {
        let agents = AgentsConfig {
            worker: Some(profile("claude-sonnet-4", None)),
            ..AgentsConfig::default()
        };
        let eng = engine(agents, RoutingConfig::default(), RunnerConfig::default());
        let d = eng.route(TaskPhase::Implementation).unwrap();
        assert!(d.fallback_models.is_empty());
    }

    // ---- reason is human-readable ----

    #[test]
    fn reason_for_routing_override() {
        let routing = RoutingConfig {
            implementation: Some("gpt-5".into()),
            ..RoutingConfig::default()
        };
        let eng = engine(AgentsConfig::default(), routing, RunnerConfig::default());
        let d = eng.route(TaskPhase::Implementation).unwrap();
        assert!(d.reason.contains("routing override"));
        assert!(d.reason.contains("gpt-5"));
    }

    #[test]
    fn reason_for_default() {
        let eng = RoutingEngine::default();
        let d = eng.route(TaskPhase::Review).unwrap();
        assert!(d.reason.contains("no model configured"));
        assert!(d.reason.contains(DEFAULT_MODEL));
    }

    // ---- permissions propagate ----

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

    // ---- from_config_v2 ----

    #[test]
    fn from_config_v2_routes_correctly() {
        let config = WorkflowConfigV2 {
            version: 2,
            orchestration: Default::default(),
            runner: RunnerConfig {
                default: "native".into(),
                allow: vec!["native".into(), "codex".into()],
            },
            agents: AgentsConfig {
                planner: Some(profile("o3", Some(Reasoning::High))),
                worker: Some(profile("claude-sonnet-4", None)),
                reviewer: None,
            },
            environment: Default::default(),
            quality: Default::default(),
            budgets: Default::default(),
            hooks: None,
            routing: Some(RoutingConfig {
                implementation: Some("gpt-5".into()),
                ..RoutingConfig::default()
            }),
            handoff: None,
        };
        let eng = RoutingEngine::from_config_v2(&config);

        // Planning: planner profile.
        let planning = eng.route(TaskPhase::Planning).unwrap();
        assert_eq!(planning.model_id, "o3");
        assert_eq!(planning.reasoning, Some(Reasoning::High));

        // Implementation: routing override.
        let impl_d = eng.route(TaskPhase::Implementation).unwrap();
        assert_eq!(impl_d.model_id, "gpt-5");
        assert_eq!(impl_d.source, RoutingSource::RoutingOverride);

        // Review: no profile → default.
        let review = eng.route(TaskPhase::Review).unwrap();
        assert_eq!(review.model_id, DEFAULT_MODEL);
    }
}
