//! Policy and approval engine (plan §B2).
//!
//! Classifies agent actions by risk, layers permission sources in priority
//! order (managed → settings → workflow → profile → override), and produces
//! deterministic decisions: Allow, Deny, Ask, or AutoReview.
//!
//! Layering rules:
//! - **Deny wins**: if any managed requirement says Deny, the answer is Deny
//!   regardless of user preferences.
//! - **Specific overrides win**: run > profile > workflow > settings > default.
//! - **Parent ceiling**: a child may choose a stricter mode but cannot exceed
//!   the authority inherited from its parent.
//! - **Managed floor**: hardcoded safety requirements that can never be
//!   relaxed by repository or app configuration.
//! - **Bypass gate**: the global bypass mode is preserved as a safety escape
//!   hatch, but managed requirements still override it.

use serde::Serialize;

use super::ledger::{approval_action_hash, ApprovalRecord, ApprovalState};
use super::workflow::PermissionMode;

// ---------------------------------------------------------------------------
// Risk classification
// ---------------------------------------------------------------------------

/// What kind of operation an action performs. Drives the default risk level.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionCategory {
    /// Read-only: search, list, inspect. Always safe.
    Read,
    /// Planning / todo updates. No side effects on the repo.
    Planning,
    /// Write to files in the workspace.
    WriteFile,
    /// Execute a shell command.
    RunCommand,
    /// Network egress to an external host.
    NetworkAccess,
    /// Git mutating operation (commit, push, etc.).
    GitOperation,
    /// Destructive: delete files, force operations, rm -rf.
    Destructive,
}

impl ActionCategory {
    /// The baseline risk for this category before policy layering.
    pub fn default_risk(self) -> RiskLevel {
        match self {
            Self::Read | Self::Planning => RiskLevel::None,
            Self::WriteFile => RiskLevel::Medium,
            Self::RunCommand | Self::NetworkAccess | Self::GitOperation | Self::Destructive => {
                RiskLevel::High
            }
        }
    }
}

/// How dangerous an action is. Higher levels require stricter approval.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    /// No approval needed (reads, searches).
    None,
    /// Auto-approved in AutoEdit/Bypass; requires Ask otherwise.
    Medium,
    /// Always requires human approval unless global bypass is active *and*
    /// the run is attended.
    High,
}

impl RiskLevel {
    pub fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

// ---------------------------------------------------------------------------
// Policy layers
// ---------------------------------------------------------------------------

/// Which policy layer produced the final decision. Every decision carries its
/// source so the user can see exactly why an action was allowed or blocked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionSource {
    Managed,
    Parent,
    Settings,
    Workflow,
    Profile,
    Override,
    Default,
}

impl DecisionSource {
    pub fn name(self) -> &'static str {
        match self {
            Self::Managed => "managed",
            Self::Parent => "parent",
            Self::Settings => "settings",
            Self::Workflow => "workflow",
            Self::Profile => "profile",
            Self::Override => "override",
            Self::Default => "default",
        }
    }
}

/// Managed (hardcoded) safety requirements. These are the non-negotiable floor:
/// no repository or app setting can relax them.
#[derive(Clone, Debug, Default)]
pub struct ManagedRequirements {
    /// Branch patterns that must never receive a force-push.
    pub protected_branches: Vec<String>,
    /// When true, destructive operations are always denied.
    pub deny_destructive: bool,
    /// When true, force pushes are always denied.
    pub deny_force_push: bool,
}

/// App-level policy settings (global user preferences).
#[derive(Clone, Debug, Default)]
pub struct AppPolicySettings {
    /// The global default permission mode when no layer overrides it.
    pub default_mode: Option<PermissionMode>,
    /// Whether the current run is attended (a human is present). Unattended
    /// runs cannot ask for approval — they auto-deny.
    pub attended: bool,
    /// Allow medium-risk AutoEdit actions immediately, but require a
    /// post-hoc reviewer pass.
    pub auto_review_medium: bool,
}

/// The full set of layers consulted during policy evaluation. Layers are
/// consulted in priority order; see the module docs for the precedence rules.
#[derive(Clone, Debug)]
pub struct PolicyLayers {
    pub managed: ManagedRequirements,
    /// Maximum authority inherited from the parent agent/run. A child may
    /// choose a stricter mode, but can never exceed this one.
    pub parent_mode: Option<PermissionMode>,
    pub settings: AppPolicySettings,
    pub workflow_mode: Option<PermissionMode>,
    pub profile_mode: Option<PermissionMode>,
    pub run_override: Option<PermissionMode>,
}

impl Default for PolicyLayers {
    fn default() -> Self {
        Self {
            managed: ManagedRequirements::default(),
            parent_mode: None,
            settings: AppPolicySettings {
                attended: true,
                ..AppPolicySettings::default()
            },
            workflow_mode: None,
            profile_mode: None,
            run_override: None,
        }
    }
}

impl PolicyLayers {
    /// Resolve the effective permission mode after layering and applying the
    /// inherited parent-authority ceiling.
    pub fn effective_mode(&self) -> PermissionMode {
        self.effective_mode_with_source().0
    }

    /// Override > Profile > Workflow > Settings > Default(AutoEdit), then
    /// clamp the result to the parent mode when the child requested more
    /// authority.
    fn effective_mode_with_source(&self) -> (PermissionMode, DecisionSource) {
        let selected = if let Some(mode) = self.run_override {
            (mode, DecisionSource::Override)
        } else if let Some(mode) = self.profile_mode {
            (mode, DecisionSource::Profile)
        } else if let Some(mode) = self.workflow_mode {
            (mode, DecisionSource::Workflow)
        } else if let Some(mode) = self.settings.default_mode {
            (mode, DecisionSource::Settings)
        } else {
            (PermissionMode::AutoEdit, DecisionSource::Default)
        };

        match self.parent_mode {
            Some(parent) if authority_rank(selected.0) > authority_rank(parent) => {
                (parent, DecisionSource::Parent)
            }
            _ => selected,
        }
    }
}

fn authority_rank(mode: PermissionMode) -> u8 {
    match mode {
        PermissionMode::Plan => 0,
        PermissionMode::Ask => 1,
        PermissionMode::AutoEdit => 2,
        PermissionMode::Bypass => 3,
    }
}

// ---------------------------------------------------------------------------
// Decisions
// ---------------------------------------------------------------------------

/// The outcome of evaluating an action against the policy.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PolicyDecision {
    /// The action is permitted without further checks.
    Allow { source: DecisionSource },
    /// The action is blocked. No layer can override a Deny.
    Deny {
        reason: String,
        source: DecisionSource,
    },
    /// The action requires explicit human approval before proceeding.
    Ask {
        risk: RiskLevel,
        prompt: String,
        source: DecisionSource,
    },
    /// The action proceeds but will be reviewed post-hoc by the reviewer agent.
    AutoReview { source: DecisionSource },
}

impl PolicyDecision {
    pub fn source(&self) -> DecisionSource {
        match self {
            Self::Allow { source }
            | Self::Deny { source, .. }
            | Self::Ask { source, .. }
            | Self::AutoReview { source } => *source,
        }
    }
}

// ---------------------------------------------------------------------------
// Evaluation
// ---------------------------------------------------------------------------

/// A description of the action being evaluated.
#[derive(Clone, Debug)]
pub struct ActionDescriptor {
    pub category: ActionCategory,
    /// Human-readable description shown in approval prompts.
    pub description: String,
    /// For GitOperation: the branch being affected, if any.
    pub git_branch: Option<String>,
    /// For GitOperation: whether this is a force-push.
    pub git_force: bool,
}

impl ActionDescriptor {
    pub fn risk(&self) -> RiskLevel {
        self.category.default_risk()
    }
}

/// Evaluate an action against the full policy stack.
///
/// Evaluation order:
/// 1. **Managed floor** — hardcoded safety requirements (force-push to
///    protected branches, destructive ops). Deny wins.
/// 2. **Bypass gate** — global bypass allows everything except managed Denies,
///    but only when attended.
/// 3. **Effective mode** — layer-resolved permission mode applied to the
///    action's risk level.
pub fn evaluate(action: &ActionDescriptor, layers: &PolicyLayers) -> PolicyDecision {
    // 1. Managed floor — never relaxable.
    if let Some(deny) = managed_check(action, &layers.managed) {
        return deny;
    }

    let risk = action.risk();
    let (mode, mode_source) = layers.effective_mode_with_source();

    // 2. Bypass gate — attended bypass allows everything not denied by managed.
    if mode == PermissionMode::Bypass {
        if !layers.settings.attended && risk == RiskLevel::High {
            return PolicyDecision::Deny {
                reason: "high-risk action denied in unattended bypass mode".into(),
                source: DecisionSource::Managed,
            };
        }
        return PolicyDecision::Allow {
            source: mode_source,
        };
    }

    // 3. Read-only actions (reads, planning) are always allowed regardless of
    //    mode — they have no side effects.
    if risk == RiskLevel::None {
        return PolicyDecision::Allow {
            source: DecisionSource::Default,
        };
    }

    // 4. Plan mode — no side-effecting actions.
    if mode == PermissionMode::Plan {
        return PolicyDecision::Deny {
            reason: "plan mode blocks all side-effecting actions".into(),
            source: mode_source,
        };
    }

    // 5. Apply risk × mode.
    match (risk, mode) {
        (RiskLevel::None, _) => PolicyDecision::Allow {
            source: DecisionSource::Default,
        },
        (RiskLevel::Medium, PermissionMode::AutoEdit) => {
            if layers.settings.auto_review_medium {
                PolicyDecision::AutoReview {
                    source: DecisionSource::Settings,
                }
            } else {
                PolicyDecision::Allow {
                    source: mode_source,
                }
            }
        }
        (RiskLevel::Medium, _) => {
            if !layers.settings.attended {
                return PolicyDecision::Deny {
                    reason: "approval-required action denied: run is unattended".into(),
                    source: DecisionSource::Settings,
                };
            }
            PolicyDecision::Ask {
                risk,
                prompt: action.description.clone(),
                source: mode_source,
            }
        }
        (RiskLevel::High, _) => {
            // Unattended runs cannot ask — auto-deny instead of stalling.
            if !layers.settings.attended {
                return PolicyDecision::Deny {
                    reason: "high-risk action denied: run is unattended".into(),
                    source: DecisionSource::Managed,
                };
            }
            PolicyDecision::Ask {
                risk,
                prompt: action.description.clone(),
                source: mode_source,
            }
        }
    }
}

/// Check managed (hardcoded) requirements. Returns `Some(Deny)` if a managed
/// rule blocks the action, `None` otherwise.
fn managed_check(
    action: &ActionDescriptor,
    managed: &ManagedRequirements,
) -> Option<PolicyDecision> {
    if action.git_force {
        if managed.deny_force_push {
            return Some(PolicyDecision::Deny {
                reason: "force-push is blocked by managed policy".into(),
                source: DecisionSource::Managed,
            });
        }
        if let Some(branch) = &action.git_branch {
            if managed
                .protected_branches
                .iter()
                .any(|p| branch == p || branch_matches(branch, p))
            {
                return Some(PolicyDecision::Deny {
                    reason: format!("force-push to protected branch '{branch}' is blocked"),
                    source: DecisionSource::Managed,
                });
            }
        }
    }

    if action.category == ActionCategory::Destructive && managed.deny_destructive {
        return Some(PolicyDecision::Deny {
            reason: "destructive action blocked by managed policy".into(),
            source: DecisionSource::Managed,
        });
    }

    None
}

/// Simple glob match: `*` matches any sequence. Character indices avoid
/// slicing at invalid UTF-8 boundaries and the backtracking cursor is always
/// bounded by the branch length.
fn branch_matches(branch: &str, pattern: &str) -> bool {
    let branch: Vec<char> = branch.chars().collect();
    let pattern: Vec<char> = pattern.chars().collect();
    let (mut branch_index, mut pattern_index) = (0, 0);
    let (mut last_star, mut star_match) = (None, 0);

    while branch_index < branch.len() {
        if pattern_index < pattern.len()
            && pattern[pattern_index] != '*'
            && pattern[pattern_index] == branch[branch_index]
        {
            branch_index += 1;
            pattern_index += 1;
        } else if pattern_index < pattern.len() && pattern[pattern_index] == '*' {
            last_star = Some(pattern_index);
            star_match = branch_index;
            pattern_index += 1;
        } else if let Some(star_index) = last_star {
            star_match += 1;
            branch_index = star_match;
            pattern_index = star_index + 1;
        } else {
            return false;
        }
    }

    pattern[pattern_index..]
        .iter()
        .all(|character| *character == '*')
}

/// Confirm that a durable approval authorizes this exact task, attempt, action,
/// and time. Any mismatch or hashing failure denies execution.
pub fn approval_allows(
    approval: &ApprovalRecord,
    task_id: &str,
    attempt_id: &str,
    action: &serde_json::Value,
    now_ms: u64,
) -> bool {
    matches!(
        approval.state,
        ApprovalState::Approved | ApprovalState::AutoResolved
    ) && approval.task_id == task_id
        && approval.attempt_id == attempt_id
        && now_ms < approval.expires_at_ms
        && approval_action_hash(action)
            .map(|hash| hash == approval.action_hash)
            .unwrap_or(false)
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;

    fn read_action(desc: &str) -> ActionDescriptor {
        ActionDescriptor {
            category: ActionCategory::Read,
            description: desc.into(),
            git_branch: None,
            git_force: false,
        }
    }

    fn write_action(desc: &str) -> ActionDescriptor {
        ActionDescriptor {
            category: ActionCategory::WriteFile,
            description: desc.into(),
            git_branch: None,
            git_force: false,
        }
    }

    fn git_action(branch: Option<&str>, force: bool) -> ActionDescriptor {
        ActionDescriptor {
            category: ActionCategory::GitOperation,
            description: "git push".into(),
            git_branch: branch.map(String::from),
            git_force: force,
        }
    }

    fn destructive_action() -> ActionDescriptor {
        ActionDescriptor {
            category: ActionCategory::Destructive,
            description: "rm -rf node_modules".into(),
            git_branch: None,
            git_force: false,
        }
    }

    fn network_action() -> ActionDescriptor {
        ActionDescriptor {
            category: ActionCategory::NetworkAccess,
            description: "curl https://example.com".into(),
            git_branch: None,
            git_force: false,
        }
    }

    // ---- risk classification ----

    #[test]
    fn read_and_planning_are_none_risk() {
        assert_eq!(ActionCategory::Read.default_risk(), RiskLevel::None);
        assert_eq!(ActionCategory::Planning.default_risk(), RiskLevel::None);
    }

    #[test]
    fn writes_are_medium_but_commands_are_high_risk() {
        assert_eq!(ActionCategory::WriteFile.default_risk(), RiskLevel::Medium);
        assert_eq!(ActionCategory::RunCommand.default_risk(), RiskLevel::High);
    }

    #[test]
    fn git_network_destructive_are_high_risk() {
        assert_eq!(ActionCategory::GitOperation.default_risk(), RiskLevel::High);
        assert_eq!(
            ActionCategory::NetworkAccess.default_risk(),
            RiskLevel::High
        );
        assert_eq!(ActionCategory::Destructive.default_risk(), RiskLevel::High);
    }

    // ---- layering precedence ----

    #[test]
    fn override_beats_profile_beats_workflow_beats_settings() {
        let mut layers = PolicyLayers::default();
        layers.settings.default_mode = Some(PermissionMode::Ask);
        assert_eq!(layers.effective_mode(), PermissionMode::Ask);

        layers.workflow_mode = Some(PermissionMode::AutoEdit);
        assert_eq!(layers.effective_mode(), PermissionMode::AutoEdit);

        layers.profile_mode = Some(PermissionMode::Plan);
        assert_eq!(layers.effective_mode(), PermissionMode::Plan);

        layers.run_override = Some(PermissionMode::Bypass);
        assert_eq!(layers.effective_mode(), PermissionMode::Bypass);
    }

    #[test]
    fn default_mode_is_auto_edit() {
        assert_eq!(
            PolicyLayers::default().effective_mode(),
            PermissionMode::AutoEdit
        );
    }

    #[test]
    fn parent_authority_clamps_a_more_permissive_child_override() {
        let mut layers = PolicyLayers::default();
        layers.parent_mode = Some(PermissionMode::Ask);
        layers.run_override = Some(PermissionMode::Bypass);

        assert_eq!(layers.effective_mode(), PermissionMode::Ask);
        let decision = evaluate(&write_action("edit file.rs"), &layers);
        assert!(matches!(
            decision,
            PolicyDecision::Ask {
                source: DecisionSource::Parent,
                ..
            }
        ));
    }

    #[test]
    fn child_may_choose_a_stricter_mode_than_its_parent() {
        let mut layers = PolicyLayers::default();
        layers.parent_mode = Some(PermissionMode::Bypass);
        layers.run_override = Some(PermissionMode::Plan);
        assert_eq!(layers.effective_mode(), PermissionMode::Plan);
    }

    // ---- evaluate: read actions always allowed ----

    #[test]
    fn read_action_always_allowed() {
        let layers = PolicyLayers::default();
        let d = evaluate(&read_action("ls"), &layers);
        assert!(matches!(d, PolicyDecision::Allow { .. }));
    }

    // ---- evaluate: medium risk + auto-edit → allow ----

    #[test]
    fn medium_risk_auto_edit_allows() {
        let layers = PolicyLayers::default(); // AutoEdit
        let d = evaluate(&write_action("edit file.rs"), &layers);
        assert!(matches!(d, PolicyDecision::Allow { .. }));
    }

    // ---- evaluate: medium risk + ask → ask ----

    #[test]
    fn medium_risk_ask_mode_asks() {
        let mut layers = PolicyLayers::default();
        layers.settings.default_mode = Some(PermissionMode::Ask);
        let d = evaluate(&write_action("edit file.rs"), &layers);
        assert!(matches!(
            d,
            PolicyDecision::Ask {
                risk: RiskLevel::Medium,
                ..
            }
        ));
    }

    // ---- evaluate: high risk always asks (attended) ----

    #[test]
    fn high_risk_auto_edit_asks() {
        let layers = PolicyLayers::default(); // attended + AutoEdit
        let d = evaluate(&network_action(), &layers);
        assert!(matches!(
            d,
            PolicyDecision::Ask {
                risk: RiskLevel::High,
                ..
            }
        ));
    }

    // ---- evaluate: plan mode blocks everything except reads ----

    #[test]
    fn plan_mode_blocks_writes() {
        let mut layers = PolicyLayers::default();
        layers.run_override = Some(PermissionMode::Plan);
        let d = evaluate(&write_action("edit file.rs"), &layers);
        assert!(matches!(d, PolicyDecision::Deny { .. }));
    }

    #[test]
    fn plan_mode_allows_reads() {
        let mut layers = PolicyLayers::default();
        layers.run_override = Some(PermissionMode::Plan);
        let d = evaluate(&read_action("ls"), &layers);
        assert!(matches!(d, PolicyDecision::Allow { .. }));
    }

    // ---- evaluate: bypass gate ----

    #[test]
    fn bypass_allows_high_risk_when_attended() {
        let mut layers = PolicyLayers::default();
        layers.run_override = Some(PermissionMode::Bypass);
        layers.settings.attended = true;
        let d = evaluate(&network_action(), &layers);
        assert!(matches!(
            d,
            PolicyDecision::Allow {
                source: DecisionSource::Override
            }
        ));
    }

    #[test]
    fn bypass_denies_high_risk_when_unattended() {
        let mut layers = PolicyLayers::default();
        layers.run_override = Some(PermissionMode::Bypass);
        layers.settings.attended = false;
        let d = evaluate(&network_action(), &layers);
        assert!(matches!(
            d,
            PolicyDecision::Deny {
                source: DecisionSource::Managed,
                ..
            }
        ));
    }

    #[test]
    fn bypass_allows_medium_risk_when_unattended() {
        let mut layers = PolicyLayers::default();
        layers.run_override = Some(PermissionMode::Bypass);
        layers.settings.attended = false;
        let d = evaluate(&write_action("edit file.rs"), &layers);
        assert!(matches!(
            d,
            PolicyDecision::Allow {
                source: DecisionSource::Override
            }
        ));
    }

    // ---- evaluate: unattended non-bypass high-risk → deny ----

    #[test]
    fn unattended_high_risk_auto_denies() {
        let mut layers = PolicyLayers::default();
        layers.settings.attended = false;
        let d = evaluate(&network_action(), &layers);
        assert!(matches!(
            d,
            PolicyDecision::Deny {
                source: DecisionSource::Managed,
                ..
            }
        ));
    }

    #[test]
    fn unattended_medium_risk_that_needs_approval_is_denied() {
        let mut layers = PolicyLayers::default();
        layers.settings.default_mode = Some(PermissionMode::Ask);
        layers.settings.attended = false;
        let d = evaluate(&write_action("edit file.rs"), &layers);
        assert!(matches!(
            d,
            PolicyDecision::Deny {
                source: DecisionSource::Settings,
                ..
            }
        ));
    }

    // ---- managed floor ----

    #[test]
    fn managed_force_push_to_protected_branch_denied() {
        let mut layers = PolicyLayers::default();
        layers.run_override = Some(PermissionMode::Bypass);
        layers.settings.attended = true;
        layers.managed.protected_branches = vec!["main".into(), "release/*".into()];
        let d = evaluate(&git_action(Some("main"), true), &layers);
        assert!(matches!(
            d,
            PolicyDecision::Deny {
                source: DecisionSource::Managed,
                ..
            }
        ));
    }

    #[test]
    fn managed_force_push_glob_pattern_denied() {
        let mut layers = PolicyLayers::default();
        layers.run_override = Some(PermissionMode::Bypass);
        layers.managed.protected_branches = vec!["release/*".into()];
        let d = evaluate(&git_action(Some("release/v2"), true), &layers);
        assert!(matches!(d, PolicyDecision::Deny { .. }));
    }

    #[test]
    fn managed_deny_force_push_blocks_all_force() {
        let mut layers = PolicyLayers::default();
        layers.run_override = Some(PermissionMode::Bypass);
        layers.managed.deny_force_push = true;
        let d = evaluate(&git_action(Some("feature/x"), true), &layers);
        assert!(matches!(
            d,
            PolicyDecision::Deny {
                source: DecisionSource::Managed,
                ..
            }
        ));
    }

    #[test]
    fn non_force_git_push_not_blocked_by_managed() {
        let mut layers = PolicyLayers::default();
        layers.managed.protected_branches = vec!["main".into()];
        let d = evaluate(&git_action(Some("main"), false), &layers);
        // Not a force push → managed floor doesn't apply → falls through to
        // normal risk evaluation (High → Ask).
        assert!(matches!(
            d,
            PolicyDecision::Ask {
                risk: RiskLevel::High,
                ..
            }
        ));
    }

    #[test]
    fn managed_destructive_unattended_denied() {
        let mut layers = PolicyLayers::default();
        layers.run_override = Some(PermissionMode::Bypass);
        layers.settings.attended = true;
        layers.managed.deny_destructive = true;
        let d = evaluate(&destructive_action(), &layers);
        assert!(matches!(
            d,
            PolicyDecision::Deny {
                source: DecisionSource::Managed,
                ..
            }
        ));
    }

    // ---- approval_allows helper ----

    #[test]
    fn approval_allows_only_the_exact_unexpired_action() {
        let action = serde_json::json!({"command": "git push"});
        let mut approval = ApprovalRecord {
            approval_id: "ap-1".into(),
            task_id: "task-1".into(),
            attempt_id: "attempt-1".into(),
            action_desc: "Push changes".into(),
            action_payload: action.clone(),
            action_hash: approval_action_hash(&action).unwrap(),
            risk_level: "high".into(),
            policy_source: "default".into(),
            state: ApprovalState::Approved,
            requested_at_ms: 1_000,
            expires_at_ms: 2_000,
            decided_at_ms: Some(1_100),
            decided_by: None,
            decision_reason: None,
        };

        assert!(approval_allows(
            &approval,
            "task-1",
            "attempt-1",
            &action,
            1_500
        ));
        assert!(!approval_allows(
            &approval,
            "another-task",
            "attempt-1",
            &action,
            1_500
        ));
        assert!(!approval_allows(
            &approval,
            "task-1",
            "attempt-1",
            &serde_json::json!({"command": "git push --force"}),
            1_500
        ));
        assert!(!approval_allows(
            &approval,
            "task-1",
            "attempt-1",
            &action,
            2_000
        ));
        approval.state = ApprovalState::Denied;
        assert!(!approval_allows(
            &approval,
            "task-1",
            "attempt-1",
            &action,
            1_500
        ));
    }

    // ---- decision source attribution ----

    #[test]
    fn decision_carries_source() {
        let mut layers = PolicyLayers::default();
        layers.run_override = Some(PermissionMode::Bypass);
        let d = evaluate(&write_action("edit"), &layers);
        assert!(matches!(
            d,
            PolicyDecision::Allow {
                source: DecisionSource::Override
            }
        ));
    }

    #[test]
    fn decisions_preserve_the_selected_layer_source() {
        let mut layers = PolicyLayers::default();
        layers.settings.default_mode = Some(PermissionMode::Ask);
        assert!(matches!(
            evaluate(&write_action("settings"), &layers),
            PolicyDecision::Ask {
                source: DecisionSource::Settings,
                ..
            }
        ));

        layers.workflow_mode = Some(PermissionMode::Ask);
        assert!(matches!(
            evaluate(&write_action("workflow"), &layers),
            PolicyDecision::Ask {
                source: DecisionSource::Workflow,
                ..
            }
        ));

        layers.profile_mode = Some(PermissionMode::Ask);
        assert!(matches!(
            evaluate(&write_action("profile"), &layers),
            PolicyDecision::Ask {
                source: DecisionSource::Profile,
                ..
            }
        ));

        layers.run_override = Some(PermissionMode::Ask);
        assert!(matches!(
            evaluate(&write_action("override"), &layers),
            PolicyDecision::Ask {
                source: DecisionSource::Override,
                ..
            }
        ));
    }

    #[test]
    fn auto_review_is_reachable_for_medium_risk_auto_edit() {
        let mut layers = PolicyLayers::default();
        layers.settings.auto_review_medium = true;
        assert!(matches!(
            evaluate(&write_action("review this edit"), &layers),
            PolicyDecision::AutoReview {
                source: DecisionSource::Settings
            }
        ));
    }

    #[test]
    fn glob_matching_handles_leading_trailing_repeated_and_unicode_stars() {
        assert!(branch_matches("feature/main", "*main"));
        assert!(branch_matches("release/v2", "release/*"));
        assert!(branch_matches("feature/ödeme-v2", "*ödeme*"));
        assert!(branch_matches("abc", "a**c"));
        assert!(branch_matches("anything", "*"));
        assert!(!branch_matches("feature/main-old", "*main"));
        assert!(!branch_matches("main", "release/*"));
    }
}
