//! WORKFLOW.md version 2 schema (plan §6).
//!
//! A v2 document has an explicit `version: 2` front-matter field and a strictly
//! parsed, typed config. A missing `version` means v1 (handled by
//! [`crate::modules::orchestration::workflow`]); v2 is opt-in. Every section
//! rejects unknown fields so the schema surfaces typos instead of silently
//! dropping them.
//!
//! This slice delivers the typed schema, version detection, validation, and v1
//! migration. The prompt template variables and secret-reference resolution
//! (§6 requirements) are follow-ups; the schema reserves no fields for them yet
//! so the v2 contract stays minimal and stable.

use serde::{Deserialize, Serialize};

/// v2 workflow version marker. Anything other than `2` is a parse error at the
/// version-detection layer.
pub const V2_VERSION: u32 = 2;

/// Reasoning effort for an agent role. Matched leniently (kebab-case) to the
/// WORKFLOW.md surface.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Reasoning {
    Low,
    Medium,
    High,
}

/// The full v2 WORKFLOW.md front matter. `deny_unknown_fields` on every layer
/// enforces the strict-schema requirement (§6).
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowConfigV2 {
    pub version: u32,
    #[serde(default)]
    pub orchestration: OrchestrationConfig,
    #[serde(default)]
    pub runner: RunnerConfig,
    #[serde(default)]
    pub agents: AgentsConfig,
    #[serde(default)]
    pub environment: EnvironmentConfig,
    #[serde(default)]
    pub quality: QualityConfig,
    #[serde(default)]
    pub budgets: BudgetsConfig,
    #[serde(default)]
    pub hooks: Option<HooksConfig>,
    #[serde(default)]
    pub routing: Option<RoutingConfig>,
    #[serde(default)]
    pub handoff: Option<HandoffConfig>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct OrchestrationConfig {
    pub max_concurrent: usize,
    pub max_attempts: u32,
    pub poll_interval_seconds: u64,
    pub stall_timeout_seconds: u64,
    pub active_states: Vec<String>,
    pub terminal_states: Vec<String>,
}

impl Default for OrchestrationConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 2,
            max_attempts: 4,
            poll_interval_seconds: 15,
            stall_timeout_seconds: 300,
            active_states: vec!["todo".into(), "in_progress".into()],
            terminal_states: vec!["done".into(), "cancelled".into()],
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct RunnerConfig {
    pub default: String,
    pub allow: Vec<String>,
}

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            default: "native".into(),
            allow: vec!["native".into()],
        }
    }
}

/// One agent role's runtime profile. `tools: None` means "no restriction".
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct AgentProfile {
    pub model_id: Option<String>,
    pub reasoning: Option<Reasoning>,
    pub permissions: Option<crate::modules::orchestration::workflow::PermissionMode>,
    pub tools: Option<Vec<String>>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct AgentsConfig {
    pub planner: Option<AgentProfile>,
    pub worker: Option<AgentProfile>,
    pub reviewer: Option<AgentProfile>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct EnvironmentConfig {
    pub executor: String,
    pub install: Option<String>,
    pub start: Option<String>,
    pub terminals: Vec<TerminalConfig>,
    pub healthcheck: Option<String>,
}

impl Default for EnvironmentConfig {
    fn default() -> Self {
        Self {
            executor: "local-worktree".into(),
            install: None,
            start: None,
            terminals: Vec::new(),
            healthcheck: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalConfig {
    pub name: String,
    pub command: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct QualityConfig {
    pub commands: Vec<String>,
    pub require_clean_worktree: bool,
    pub require_review: bool,
    pub require_plan_approval: bool,
    pub browser: BrowserQualityConfig,
}

impl Default for QualityConfig {
    fn default() -> Self {
        Self {
            commands: Vec::new(),
            require_clean_worktree: true,
            require_review: true,
            require_plan_approval: false,
            browser: BrowserQualityConfig::default(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct BrowserQualityConfig {
    pub enabled: bool,
    pub routes: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct BudgetsConfig {
    pub max_task_minutes: Option<u64>,
    pub max_attempt_tokens: Option<u64>,
    pub max_task_cost_usd: Option<f64>,
    pub warn_at_percent: u8,
}

impl Default for BudgetsConfig {
    fn default() -> Self {
        Self {
            max_task_minutes: Some(120),
            max_attempt_tokens: Some(200_000),
            max_task_cost_usd: None,
            warn_at_percent: 80,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct HooksConfig {
    pub after_create: Option<String>,
    pub before_run: Option<String>,
    pub after_run: Option<String>,
    pub timeout_seconds: u64,
}

impl Default for HooksConfig {
    fn default() -> Self {
        Self {
            after_create: None,
            before_run: None,
            after_run: None,
            timeout_seconds: 60,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct RoutingConfig {
    pub planner: Option<String>,
    pub implementation: Option<String>,
    pub review: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct HandoffConfig {
    pub target: String,
    pub auto_apply: bool,
    pub auto_publish_draft_pr: bool,
}

impl Default for HandoffConfig {
    fn default() -> Self {
        Self {
            target: "human-review".into(),
            auto_apply: false,
            auto_publish_draft_pr: false,
        }
    }
}

/// Validate a parsed v2 config against the §6 constraints. Returns a
/// field-scoped error message on failure (the caller attaches line context).
pub fn validate(config: &WorkflowConfigV2) -> Result<(), String> {
    let o = &config.orchestration;
    if !(1..=8).contains(&o.max_concurrent) {
        return Err("orchestration.max_concurrent must be between 1 and 8.".into());
    }
    if !(1..=10).contains(&o.max_attempts) {
        return Err("orchestration.max_attempts must be between 1 and 10.".into());
    }
    if o.poll_interval_seconds == 0 || o.poll_interval_seconds > 3_600 {
        return Err("orchestration.poll_interval_seconds must be between 1 and 3600.".into());
    }
    if o.stall_timeout_seconds == 0 || o.stall_timeout_seconds > 86_400 {
        return Err("orchestration.stall_timeout_seconds must be between 1 and 86400.".into());
    }
    if o.active_states.is_empty() {
        return Err("orchestration.active_states must list at least one state.".into());
    }
    if o.terminal_states.is_empty() {
        return Err("orchestration.terminal_states must list at least one state.".into());
    }
    let default_runner = config.runner.default.trim();
    if default_runner.is_empty() {
        return Err("runner.default must be a non-empty runner name.".into());
    }
    if !config.runner.allow.iter().any(|r| r == default_runner) {
        return Err("runner.default must be listed in runner.allow.".into());
    }
    if config.runner.allow.is_empty() {
        return Err("runner.allow must list at least one runner.".into());
    }
    for (role, profile) in [
        ("agents.planner", &config.agents.planner),
        ("agents.worker", &config.agents.worker),
        ("agents.reviewer", &config.agents.reviewer),
    ] {
        if let Some(profile) = profile {
            if let Some(model) = profile.model_id.as_deref() {
                if model.trim().is_empty() || model.len() > 128 {
                    return Err(format!(
                        "{role}.model_id must be non-empty and under 128 chars."
                    ));
                }
            }
        }
    }
    if config.environment.executor.trim().is_empty() {
        return Err("environment.executor must be non-empty.".into());
    }
    for (index, terminal) in config.environment.terminals.iter().enumerate() {
        if terminal.name.trim().is_empty() {
            return Err(format!(
                "environment.terminals[{index}].name must be non-empty."
            ));
        }
        if terminal.command.trim().is_empty() {
            return Err(format!(
                "environment.terminals[{index}].command must be non-empty."
            ));
        }
        if config.environment.terminals[..index]
            .iter()
            .any(|other| other.name == terminal.name)
        {
            return Err(format!(
                "environment.terminals contains duplicate name `{}`.",
                terminal.name
            ));
        }
    }
    if config.budgets.warn_at_percent > 100 {
        return Err("budgets.warn_at_percent must be between 0 and 100.".into());
    }
    if let Some(cost) = config.budgets.max_task_cost_usd {
        if !cost.is_finite() || cost < 0.0 {
            return Err("budgets.max_task_cost_usd must be a non-negative finite number.".into());
        }
    }
    Ok(())
}

/// Parse the front-matter YAML for a v2 document. The caller has already
/// confirmed `version == 2`; this enforces strict parsing + validation.
pub fn parse(yaml: &str) -> Result<WorkflowConfigV2, String> {
    let config = serde_yaml::from_str::<WorkflowConfigV2>(yaml)
        .map_err(|error| format!("Invalid v2 WORKFLOW.md front matter: {error}"))?;
    if config.version != V2_VERSION {
        return Err(format!(
            "WORKFLOW.md version must be exactly {V2_VERSION} for the v2 schema (got {}).",
            config.version
        ));
    }
    validate(&config)?;
    Ok(config)
}

/// Downgrade a v2 config into the v1 shape for backward-compatible consumers.
/// Lossy by design: v2-only knobs (poll interval, stall timeout, budgets,
/// quality) have no v1 home and are dropped; retry back-off falls back to the
/// v1 defaults since v2 expresses cadence differently.
pub fn to_v1(config: &WorkflowConfigV2) -> crate::modules::orchestration::workflow::WorkflowConfig {
    use crate::modules::orchestration::workflow::{AgentConfig, SchedulerConfig, WorkflowConfig};
    let scheduler = SchedulerConfig {
        max_concurrent: config.orchestration.max_concurrent,
        max_attempts: config.orchestration.max_attempts,
        ..SchedulerConfig::default()
    };
    let agent = config
        .agents
        .worker
        .as_ref()
        .map(|w| AgentConfig {
            model_id: w.model_id.clone(),
            permission_mode: w.permissions,
        })
        .unwrap_or_default();
    WorkflowConfig {
        orchestration: scheduler,
        agent,
    }
}

/// Normalize a v1 [`workflow::WorkflowConfig`] into a v2 config. The v2 schema is
/// a strict superset of intent; v1 fields map onto the v2 orchestration/agent
/// sections and every other section takes its default.
pub fn migrate_from_v1(
    v1: &crate::modules::orchestration::workflow::WorkflowConfig,
) -> WorkflowConfigV2 {
    let mut config = WorkflowConfigV2 {
        version: V2_VERSION,
        orchestration: OrchestrationConfig {
            max_concurrent: v1.orchestration.max_concurrent,
            max_attempts: v1.orchestration.max_attempts,
            poll_interval_seconds: OrchestrationConfig::default().poll_interval_seconds,
            stall_timeout_seconds: OrchestrationConfig::default().stall_timeout_seconds,
            active_states: OrchestrationConfig::default().active_states,
            terminal_states: OrchestrationConfig::default().terminal_states,
        },
        runner: RunnerConfig::default(),
        agents: AgentsConfig::default(),
        environment: EnvironmentConfig::default(),
        quality: QualityConfig::default(),
        budgets: BudgetsConfig::default(),
        hooks: None,
        routing: None,
        handoff: None,
    };
    if v1.agent.model_id.is_some() || v1.agent.permission_mode.is_some() {
        config.agents.worker = Some(AgentProfile {
            model_id: v1.agent.model_id.clone(),
            // v1 has no reasoning field, so migration must not invent one.
            reasoning: None,
            permissions: v1.agent.permission_mode,
            tools: None,
        });
    }
    config
}

#[cfg(test)]
mod tests {
    use super::*;

    const V2_DOC: &str = "\
version: 2

orchestration:
  max_concurrent: 4
  max_attempts: 4
  poll_interval_seconds: 15
  stall_timeout_seconds: 300
  active_states: [todo, in_progress]
  terminal_states: [done, cancelled]

runner:
  default: native
  allow: [native, codex-app-server]

agents:
  planner:
    model_id: null
    reasoning: medium
    permissions: plan
    tools: [read, search]
  worker:
    model_id: null
    reasoning: high
    permissions: auto-edit
  reviewer:
    model_id: null
    reasoning: high
    permissions: plan

environment:
  executor: local-worktree
  install: pnpm install
  start: pnpm dev
  terminals:
    - name: app
      command: pnpm dev
  healthcheck: http://127.0.0.1:1420

quality:
  commands:
    - npm run lint
    - npm test -- --run
  require_clean_worktree: true
  require_review: true
  require_plan_approval: false
  browser:
    enabled: false
    routes: []

budgets:
  max_task_minutes: 120
  max_attempt_tokens: 200000
  max_task_cost_usd: null
  warn_at_percent: 80
";

    #[test]
    fn parses_full_v2_document() {
        let config = parse(V2_DOC).expect("parse");
        assert_eq!(config.version, 2);
        assert_eq!(config.orchestration.max_concurrent, 4);
        assert_eq!(config.runner.default, "native");
        assert_eq!(config.runner.allow, vec!["native", "codex-app-server"]);
        assert_eq!(
            config.agents.planner.as_ref().unwrap().reasoning,
            Some(Reasoning::Medium)
        );
        assert_eq!(config.environment.terminals.len(), 1);
        assert_eq!(config.environment.terminals[0].name, "app");
        assert_eq!(config.quality.commands.len(), 2);
        assert_eq!(config.budgets.warn_at_percent, 80);
    }

    #[test]
    fn applies_defaults_for_omitted_sections() {
        let yaml = "version: 2\n";
        let config = parse(yaml).expect("parse minimal");
        assert_eq!(config.orchestration.max_concurrent, 2);
        assert_eq!(config.runner.default, "native");
        assert_eq!(config.environment.executor, "local-worktree");
        assert!(config.hooks.is_none());
        assert!(config.routing.is_none());
    }

    #[test]
    fn rejects_unknown_field() {
        let yaml = "version: 2\ntypo_field: true\n";
        assert!(parse(yaml).is_err());
        let yaml = "version: 2\norchestration:\n  bogus: 1\n";
        assert!(parse(yaml).is_err());
    }

    #[test]
    fn rejects_wrong_version() {
        let yaml = "version: 3\n";
        let err = parse(yaml).unwrap_err();
        assert!(err.contains("must be exactly 2"), "{err}");
    }

    #[test]
    fn validates_ranges() {
        let yaml = "version: 2\norchestration:\n  max_concurrent: 99\n";
        let err = parse(yaml).unwrap_err();
        assert!(err.contains("max_concurrent"), "{err}");
    }

    #[test]
    fn default_runner_must_be_allowed() {
        let yaml = "version: 2\nrunner:\n  default: codex\n  allow: [native]\n";
        let err = parse(yaml).unwrap_err();
        assert!(err.contains("runner.default"), "{err}");
    }

    #[test]
    fn active_and_terminal_states_required() {
        let yaml = "version: 2\norchestration:\n  active_states: []\n";
        let err = parse(yaml).unwrap_err();
        assert!(err.contains("active_states"), "{err}");
    }

    #[test]
    fn rejects_non_finite_budget() {
        let yaml = "version: 2\nbudgets:\n  max_task_cost_usd: .inf\n";
        assert!(parse(yaml).is_err());
    }

    #[test]
    fn migrate_from_v1_preserves_scheduling_and_agent() {
        use crate::modules::orchestration::workflow::{
            AgentConfig, PermissionMode, SchedulerConfig, WorkflowConfig,
        };
        let v1 = WorkflowConfig {
            orchestration: SchedulerConfig {
                max_concurrent: 3,
                max_attempts: 5,
                retry_base_seconds: 5,
                retry_max_seconds: 300,
            },
            agent: AgentConfig {
                model_id: Some("gemini-2.5-pro".into()),
                permission_mode: Some(PermissionMode::AutoEdit),
            },
        };
        let v2 = migrate_from_v1(&v1);
        assert_eq!(v2.version, 2);
        assert_eq!(v2.orchestration.max_concurrent, 3);
        assert_eq!(v2.orchestration.max_attempts, 5);
        let worker = v2.agents.worker.expect("worker migrated");
        assert_eq!(worker.model_id.as_deref(), Some("gemini-2.5-pro"));
        assert_eq!(worker.permissions, Some(PermissionMode::AutoEdit));
        assert_eq!(worker.reasoning, None);
    }

    #[test]
    fn migration_preserves_model_without_permission_mode() {
        use crate::modules::orchestration::workflow::{AgentConfig, WorkflowConfig};
        let v1 = WorkflowConfig {
            agent: AgentConfig {
                model_id: Some("glm-5".into()),
                permission_mode: None,
            },
            ..WorkflowConfig::default()
        };

        let v2 = migrate_from_v1(&v1);
        let worker = v2.agents.worker.expect("model-only worker migrated");
        assert_eq!(worker.model_id.as_deref(), Some("glm-5"));
        assert_eq!(worker.permissions, None);
        assert_eq!(worker.reasoning, None);
    }

    #[test]
    fn rejects_invalid_permission_and_terminal_definitions() {
        let permission = "version: 2\nagents:\n  worker:\n    permissions: unrestricted\n";
        assert!(parse(permission).is_err());

        let empty_command =
            "version: 2\nenvironment:\n  terminals:\n    - name: app\n      command: ''\n";
        let error = parse(empty_command).unwrap_err();
        assert!(error.contains("terminals[0].command"), "{error}");

        let duplicate = "version: 2\nenvironment:\n  terminals:\n    - name: app\n      command: first\n    - name: app\n      command: second\n";
        let error = parse(duplicate).unwrap_err();
        assert!(error.contains("duplicate name `app`"), "{error}");
    }
}
