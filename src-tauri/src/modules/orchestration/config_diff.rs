//! Visible config diff between repository config and per-run overrides (plan §6).
//!
//! Produces structured, field-scoped diff entries between two v2 workflow
//! configs. Used to surface exactly what changed when a run applies overrides
//! on top of the repository's WORKFLOW.md.
//!
//! Requirements satisfied (§6):
//! - visible diff between repository config and per-run overrides.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::workflow_v2::WorkflowConfigV2;

// ---------------------------------------------------------------------------
// Diff types
// ---------------------------------------------------------------------------

/// The kind of change detected at a single config path.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    /// Field exists in both but with different values.
    Modified,
    /// Field exists only in the override (added).
    Added,
    /// Field exists only in the repo config (removed by override).
    Removed,
}

/// A single field-level change between two configs.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConfigChange {
    /// Dotted path to the changed field (e.g. `orchestration.max_concurrent`).
    pub path: String,
    /// Kind of change.
    pub kind: ChangeKind,
    /// Value in the repo config (None if added by override).
    pub repo_value: Option<String>,
    /// Value in the override config (None if removed).
    pub override_value: Option<String>,
}

/// The full diff result between repo and override configs.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConfigDiff {
    /// All field-level changes, sorted by path.
    pub changes: Vec<ConfigChange>,
    /// Total number of changes.
    pub change_count: usize,
    /// Whether the configs are identical.
    pub identical: bool,
    /// Summary categories.
    pub modified_count: usize,
    pub added_count: usize,
    pub removed_count: usize,
}

impl ConfigDiff {
    /// Check if there are any changes.
    pub fn has_changes(&self) -> bool {
        !self.changes.is_empty()
    }

    /// Get changes filtered by a top-level section (e.g. "orchestration").
    pub fn changes_in_section(&self, section: &str) -> Vec<&ConfigChange> {
        let prefix = format!("{section}.");
        self.changes
            .iter()
            .filter(|c| c.path.starts_with(&prefix) || c.path == section)
            .collect()
    }

    /// Render a human-readable summary.
    pub fn summary(&self) -> String {
        if self.identical {
            return "No changes — override matches repository config.".to_string();
        }
        format!(
            "{} change(s): {} modified, {} added, {} removed",
            self.change_count, self.modified_count, self.added_count, self.removed_count
        )
    }
}

// ---------------------------------------------------------------------------
// Diff computation
// ---------------------------------------------------------------------------

/// Compute a structured diff between a repo config and an override config.
///
/// Both configs are serialized to JSON and compared recursively. The result
/// contains every field that differs, with the old (repo) and new (override)
/// values rendered as strings.
pub fn diff_config(repo: &WorkflowConfigV2, override_cfg: &WorkflowConfigV2) -> ConfigDiff {
    let repo_json = serde_json::to_value(repo).expect("WorkflowConfigV2 must serialize to JSON");
    let override_json =
        serde_json::to_value(override_cfg).expect("WorkflowConfigV2 must serialize to JSON");

    let mut changes = Vec::new();
    diff_values("", &repo_json, &override_json, &mut changes);

    changes.sort_by(|a, b| a.path.cmp(&b.path));

    let modified_count = changes
        .iter()
        .filter(|c| c.kind == ChangeKind::Modified)
        .count();
    let added_count = changes
        .iter()
        .filter(|c| c.kind == ChangeKind::Added)
        .count();
    let removed_count = changes
        .iter()
        .filter(|c| c.kind == ChangeKind::Removed)
        .count();

    ConfigDiff {
        change_count: changes.len(),
        identical: changes.is_empty(),
        changes,
        modified_count,
        added_count,
        removed_count,
    }
}

/// Recursively diff two JSON values, appending changes to the output.
/// `Value::Null` is treated as "field absent" (serde serializes `None` as null).
fn diff_values(path: &str, repo: &Value, override_val: &Value, out: &mut Vec<ConfigChange>) {
    match (repo, override_val) {
        (Value::Null, Value::Null) => {}
        (Value::Null, ov) => {
            out.push(ConfigChange {
                path: path.to_string(),
                kind: ChangeKind::Added,
                repo_value: None,
                override_value: Some(value_to_string(ov)),
            });
        }
        (rv, Value::Null) => {
            out.push(ConfigChange {
                path: path.to_string(),
                kind: ChangeKind::Removed,
                repo_value: Some(value_to_string(rv)),
                override_value: None,
            });
        }
        (Value::Object(repo_obj), Value::Object(override_obj)) => {
            let all_keys: std::collections::BTreeSet<&String> =
                repo_obj.keys().chain(override_obj.keys()).collect();

            for key in all_keys {
                let child_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                match (repo_obj.get(key), override_obj.get(key)) {
                    (Some(rv), Some(ov)) => {
                        diff_values(&child_path, rv, ov, out);
                    }
                    (Some(rv), None) => {
                        out.push(ConfigChange {
                            path: child_path,
                            kind: ChangeKind::Removed,
                            repo_value: Some(value_to_string(rv)),
                            override_value: None,
                        });
                    }
                    (None, Some(ov)) => {
                        out.push(ConfigChange {
                            path: child_path,
                            kind: ChangeKind::Added,
                            repo_value: None,
                            override_value: Some(value_to_string(ov)),
                        });
                    }
                    (None, None) => {}
                }
            }
        }
        (Value::Array(repo_arr), Value::Array(override_arr)) => {
            if repo_arr.len() != override_arr.len() {
                out.push(ConfigChange {
                    path: path.to_string(),
                    kind: ChangeKind::Modified,
                    repo_value: Some(value_to_string(repo)),
                    override_value: Some(value_to_string(override_val)),
                });
            } else {
                for (i, (rv, ov)) in repo_arr.iter().zip(override_arr.iter()).enumerate() {
                    let child_path = format!("{path}[{i}]");
                    diff_values(&child_path, rv, ov, out);
                }
            }
        }
        (rv, ov) if rv != ov => {
            out.push(ConfigChange {
                path: path.to_string(),
                kind: ChangeKind::Modified,
                repo_value: Some(value_to_string(rv)),
                override_value: Some(value_to_string(ov)),
            });
        }
        _ => {}
    }
}

/// Render a JSON value as a compact string for display.
fn value_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        other => other.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Structured validation diagnostics
// ---------------------------------------------------------------------------

/// Severity of a validation diagnostic.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverity {
    Error,
    Warning,
}

/// A structured validation diagnostic with field context.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    /// Severity level.
    pub severity: DiagnosticSeverity,
    /// Dotted path to the field (e.g. `orchestration.max_concurrent`).
    pub field: String,
    /// Human-readable message.
    pub message: String,
}

impl Diagnostic {
    pub fn error(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: DiagnosticSeverity::Error,
            field: field.into(),
            message: message.into(),
        }
    }

    pub fn warning(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: DiagnosticSeverity::Warning,
            field: field.into(),
            message: message.into(),
        }
    }
}

/// A collection of validation diagnostics.
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticList {
    pub diagnostics: Vec<Diagnostic>,
}

impl DiagnosticList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, diag: Diagnostic) {
        self.diagnostics.push(diag);
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == DiagnosticSeverity::Error)
    }

    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == DiagnosticSeverity::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == DiagnosticSeverity::Warning)
            .count()
    }

    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

impl std::fmt::Display for DiagnosticList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for diag in &self.diagnostics {
            let level = match diag.severity {
                DiagnosticSeverity::Error => "ERROR",
                DiagnosticSeverity::Warning => "WARN",
            };
            writeln!(f, "[{level}] {}: {}", diag.field, diag.message)?;
        }
        Ok(())
    }
}

/// Validate a v2 config and return structured diagnostics instead of a single
/// error string. This wraps the existing [`workflow_v2::validate`] and augments
/// it with field-scoped diagnostics.
pub fn validate_with_diagnostics(config: &WorkflowConfigV2) -> DiagnosticList {
    let mut diags = DiagnosticList::new();

    let o = &config.orchestration;
    if !(1..=8).contains(&o.max_concurrent) {
        diags.push(Diagnostic::error(
            "orchestration.max_concurrent",
            "must be between 1 and 8",
        ));
    }
    if !(1..=10).contains(&o.max_attempts) {
        diags.push(Diagnostic::error(
            "orchestration.max_attempts",
            "must be between 1 and 10",
        ));
    }
    if o.poll_interval_seconds == 0 || o.poll_interval_seconds > 3_600 {
        diags.push(Diagnostic::error(
            "orchestration.poll_interval_seconds",
            "must be between 1 and 3600",
        ));
    }
    if o.stall_timeout_seconds == 0 || o.stall_timeout_seconds > 86_400 {
        diags.push(Diagnostic::error(
            "orchestration.stall_timeout_seconds",
            "must be between 1 and 86400",
        ));
    }
    if o.active_states.is_empty() {
        diags.push(Diagnostic::error(
            "orchestration.active_states",
            "must list at least one state",
        ));
    }
    if o.terminal_states.is_empty() {
        diags.push(Diagnostic::error(
            "orchestration.terminal_states",
            "must list at least one state",
        ));
    }

    let default_runner = config.runner.default.trim();
    if default_runner.is_empty() {
        diags.push(Diagnostic::error(
            "runner.default",
            "must be a non-empty runner name",
        ));
    } else if default_runner != super::workflow_v2::NATIVE_RUNNER {
        diags.push(Diagnostic::error(
            "runner.default",
            format!(
                "must be '{}' (ALTAI/IsanAgent only); got '{default_runner}'",
                super::workflow_v2::NATIVE_RUNNER
            ),
        ));
    }
    if config.runner.allow.is_empty() {
        diags.push(Diagnostic::error(
            "runner.allow",
            "must list at least one runner",
        ));
    }
    for runner in &config.runner.allow {
        if runner != super::workflow_v2::NATIVE_RUNNER {
            diags.push(Diagnostic::error(
                "runner.allow",
                format!(
                    "may only include '{}' (ALTAI/IsanAgent only); got '{runner}'",
                    super::workflow_v2::NATIVE_RUNNER
                ),
            ));
        }
    }

    for (role, profile) in [
        ("agents.planner", &config.agents.planner),
        ("agents.worker", &config.agents.worker),
        ("agents.reviewer", &config.agents.reviewer),
    ] {
        if let Some(profile) = profile {
            if let Some(model) = profile.model_id.as_deref() {
                if model.trim().is_empty() || model.len() > 128 {
                    diags.push(Diagnostic::error(
                        format!("{role}.model_id"),
                        "must be non-empty and under 128 chars",
                    ));
                }
            }
        }
    }

    if config.environment.executor.trim().is_empty() {
        diags.push(Diagnostic::error(
            "environment.executor",
            "must be non-empty",
        ));
    }

    for (index, terminal) in config.environment.terminals.iter().enumerate() {
        if terminal.name.trim().is_empty() {
            diags.push(Diagnostic::error(
                format!("environment.terminals[{index}].name"),
                "must be non-empty",
            ));
        }
        if terminal.command.trim().is_empty() {
            diags.push(Diagnostic::error(
                format!("environment.terminals[{index}].command"),
                "must be non-empty",
            ));
        }
    }

    if config.budgets.warn_at_percent > 100 {
        diags.push(Diagnostic::error(
            "budgets.warn_at_percent",
            "must be between 0 and 100",
        ));
    }
    if let Some(cost) = config.budgets.max_task_cost_usd {
        if !cost.is_finite() || cost < 0.0 {
            diags.push(Diagnostic::error(
                "budgets.max_task_cost_usd",
                "must be a non-negative finite number",
            ));
        }
    }

    if let Some(hooks) = &config.hooks {
        if hooks.timeout_seconds == 0 || hooks.timeout_seconds > 3_600 {
            diags.push(Diagnostic::error(
                "hooks.timeout_seconds",
                "must be between 1 and 3600",
            ));
        }
        if hooks.lifecycle.len() > 64 {
            diags.push(Diagnostic::error(
                "hooks.lifecycle",
                "cannot contain more than 64 hooks",
            ));
        }
    }

    diags
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::super::workflow_v2::*;
    use super::*;

    fn base_config() -> WorkflowConfigV2 {
        WorkflowConfigV2 {
            version: 2,
            orchestration: OrchestrationConfig {
                max_concurrent: 4,
                max_attempts: 4,
                poll_interval_seconds: 15,
                stall_timeout_seconds: 300,
                active_states: vec!["todo".into(), "in_progress".into()],
                terminal_states: vec!["done".into(), "cancelled".into()],
            },
            runner: RunnerConfig::default(),
            agents: AgentsConfig::default(),
            environment: EnvironmentConfig::default(),
            quality: QualityConfig::default(),
            budgets: BudgetsConfig::default(),
            hooks: None,
            routing: None,
            handoff: None,
        }
    }

    // ---- Config diff ----

    #[test]
    fn identical_configs_produce_empty_diff() {
        let repo = base_config();
        let override_cfg = base_config();
        let diff = diff_config(&repo, &override_cfg);
        assert!(diff.identical);
        assert!(!diff.has_changes());
        assert_eq!(diff.change_count, 0);
    }

    #[test]
    fn modified_field_detected() {
        let repo = base_config();
        let mut override_cfg = base_config();
        override_cfg.orchestration.max_concurrent = 8;

        let diff = diff_config(&repo, &override_cfg);
        assert!(!diff.identical);
        assert_eq!(diff.modified_count, 1);

        let change = &diff.changes[0];
        assert_eq!(change.path, "orchestration.max_concurrent");
        assert_eq!(change.kind, ChangeKind::Modified);
        assert_eq!(change.repo_value.as_deref(), Some("4"));
        assert_eq!(change.override_value.as_deref(), Some("8"));
    }

    #[test]
    fn added_field_detected() {
        let repo = base_config();
        let mut override_cfg = base_config();
        override_cfg.handoff = Some(HandoffConfig {
            target: "auto-merge".into(),
            auto_apply: true,
            auto_publish_draft_pr: false,
        });

        let diff = diff_config(&repo, &override_cfg);
        assert!(diff
            .changes
            .iter()
            .any(|c| c.path == "handoff" && c.kind == ChangeKind::Added));
    }

    #[test]
    fn removed_field_detected() {
        let mut repo = base_config();
        repo.hooks = Some(HooksConfig::default());
        let override_cfg = base_config();

        let diff = diff_config(&repo, &override_cfg);
        assert!(diff
            .changes
            .iter()
            .any(|c| c.path == "hooks" && c.kind == ChangeKind::Removed));
    }

    #[test]
    fn multiple_changes_detected() {
        let repo = base_config();
        let mut override_cfg = base_config();
        override_cfg.orchestration.max_concurrent = 6;
        override_cfg.orchestration.max_attempts = 8;
        override_cfg.budgets.warn_at_percent = 90;

        let diff = diff_config(&repo, &override_cfg);
        assert_eq!(diff.modified_count, 3);
        assert_eq!(diff.change_count, 3);
    }

    #[test]
    fn changes_sorted_by_path() {
        let repo = base_config();
        let mut override_cfg = base_config();
        override_cfg.budgets.warn_at_percent = 90;
        override_cfg.orchestration.max_concurrent = 6;
        override_cfg.runner.default = "native".into();

        let diff = diff_config(&repo, &override_cfg);
        let paths: Vec<&str> = diff.changes.iter().map(|c| c.path.as_str()).collect();
        assert!(paths.windows(2).all(|w| w[0] <= w[1]));
    }

    #[test]
    fn changes_in_section_filter_works() {
        let repo = base_config();
        let mut override_cfg = base_config();
        override_cfg.orchestration.max_concurrent = 6;
        override_cfg.orchestration.max_attempts = 8;
        override_cfg.budgets.warn_at_percent = 90;

        let diff = diff_config(&repo, &override_cfg);
        let orch_changes = diff.changes_in_section("orchestration");
        assert_eq!(orch_changes.len(), 2);
        assert!(orch_changes
            .iter()
            .all(|c| c.path.starts_with("orchestration.")));
    }

    #[test]
    fn summary_reports_correct_counts() {
        let repo = base_config();
        let mut override_cfg = base_config();
        override_cfg.orchestration.max_concurrent = 6;
        override_cfg.handoff = Some(HandoffConfig::default());

        let diff = diff_config(&repo, &override_cfg);
        let summary = diff.summary();
        assert!(summary.contains("modified"));
    }

    #[test]
    fn summary_identical_configs() {
        let repo = base_config();
        let diff = diff_config(&repo, &repo);
        assert!(diff.summary().contains("No changes"));
    }

    #[test]
    fn array_length_change_detected() {
        let repo = base_config();
        let mut override_cfg = base_config();
        override_cfg.orchestration.active_states =
            vec!["todo".into(), "in_progress".into(), "review".into()];

        let diff = diff_config(&repo, &override_cfg);
        assert!(diff
            .changes
            .iter()
            .any(|c| c.path == "orchestration.active_states" && c.kind == ChangeKind::Modified));
    }

    #[test]
    fn nested_object_changes_detected() {
        let repo = base_config();
        let mut override_cfg = base_config();
        override_cfg.agents.worker = Some(AgentProfile {
            model_id: Some("gpt-4".into()),
            reasoning: Some(Reasoning::High),
            permissions: None,
            tools: None,
        });

        let diff = diff_config(&repo, &override_cfg);
        assert!(diff
            .changes
            .iter()
            .any(|c| c.path.starts_with("agents.worker")));
    }

    // ---- Diagnostics ----

    #[test]
    fn diagnostics_clean_for_valid_config() {
        let config = base_config();
        let diags = validate_with_diagnostics(&config);
        assert!(!diags.has_errors());
        assert_eq!(diags.error_count(), 0);
    }

    #[test]
    fn diagnostics_detect_invalid_max_concurrent() {
        let mut config = base_config();
        config.orchestration.max_concurrent = 0;
        let diags = validate_with_diagnostics(&config);
        assert!(diags.has_errors());
        assert_eq!(diags.error_count(), 1);
        assert_eq!(diags.diagnostics[0].field, "orchestration.max_concurrent");
    }

    #[test]
    fn diagnostics_detect_multiple_errors() {
        let mut config = base_config();
        config.orchestration.max_concurrent = 0;
        config.orchestration.max_attempts = 0;
        config.budgets.warn_at_percent = 150;
        let diags = validate_with_diagnostics(&config);
        assert_eq!(diags.error_count(), 3);
    }

    #[test]
    fn diagnostics_detect_invalid_runner() {
        let mut config = base_config();
        config.runner.default = "codex".into();
        let diags = validate_with_diagnostics(&config);
        assert!(diags.has_errors());
        assert!(diags
            .diagnostics
            .iter()
            .any(|d| d.field == "runner.default"));
    }

    #[test]
    fn diagnostics_detect_empty_executor() {
        let mut config = base_config();
        config.environment.executor = "".into();
        let diags = validate_with_diagnostics(&config);
        assert!(diags
            .diagnostics
            .iter()
            .any(|d| d.field == "environment.executor"));
    }

    #[test]
    fn diagnostics_field_path_is_specific() {
        let mut config = base_config();
        config.environment.terminals.push(TerminalConfig {
            name: "".to_string(),
            command: "echo hi".to_string(),
        });
        let diags = validate_with_diagnostics(&config);
        assert!(diags
            .diagnostics
            .iter()
            .any(|d| d.field.contains("environment.terminals[0].name")));
    }

    #[test]
    fn diagnostics_display_formats_correctly() {
        let mut diags = DiagnosticList::new();
        diags.push(Diagnostic::error(
            "orchestration.max_concurrent",
            "must be between 1 and 8",
        ));
        let display = format!("{diags}");
        assert!(display.contains("ERROR"));
        assert!(display.contains("orchestration.max_concurrent"));
        assert!(display.contains("must be between 1 and 8"));
    }

    #[test]
    fn diagnostic_constructors() {
        let err = Diagnostic::error("field", "bad");
        assert_eq!(err.severity, DiagnosticSeverity::Error);
        assert_eq!(err.field, "field");

        let warn = Diagnostic::warning("field", "ehh");
        assert_eq!(warn.severity, DiagnosticSeverity::Warning);
    }

    #[test]
    fn diagnostics_empty_list() {
        let diags = DiagnosticList::new();
        assert!(!diags.has_errors());
        assert_eq!(diags.error_count(), 0);
        assert_eq!(diags.warning_count(), 0);
        assert!(diags.is_empty());
    }

    #[test]
    fn diagnostics_consistent_with_legacy_validate() {
        let mut config = base_config();
        config.orchestration.max_concurrent = 0;
        config.budgets.warn_at_percent = 150;

        let legacy_result = super::super::workflow_v2::validate(&config);
        let diags = validate_with_diagnostics(&config);

        assert!(legacy_result.is_err());
        assert!(diags.has_errors());
        assert!(diags.error_count() >= 2);
    }
}
