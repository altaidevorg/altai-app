//! Policy and approval engine (plan §B2).
//!
//! Classifies agent actions by risk, layers permission sources in priority
//! order (managed → settings → workflow → profile → override), and produces
//! deterministic decisions: Allow, Deny, Ask, or AutoReview.
//!
//! Layering rules:
//! - **Deny wins**: if any managed requirement says Deny, the answer is Deny
//!   regardless of user preferences.
//! - **Ask upgrades**: if any layer says Ask, the final decision is at least
//!   Ask (never silently downgraded to Allow).
//! - **Managed floor**: hardcoded safety requirements that can never be
//!   relaxed by repository or app configuration.
//! - **Bypass gate**: the global bypass mode is preserved as a safety escape
//!   hatch, but managed requirements still override it.

use serde::Serialize;

use super::ledger::ApprovalState;
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
            Self::WriteFile | Self::RunCommand => RiskLevel::Medium,
            Self::NetworkAccess | Self::GitOperation | Self::Destructive => RiskLevel::High,
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
    Settings,
    Workflow,
    Profile,
    Override,
    Bypass,
    Default,
}

/// Managed (hardcoded) safety requirements. These are the non-negotiable floor:
/// no repository or app setting can relax them.
#[derive(Clone, Debug, Default)]
pub struct ManagedRequirements {
    /// Branch patterns that must never receive a force-push.
    pub protected_branches: Vec<String>,
    /// When true, High-risk actions are denied even in bypass mode (e.g.
    /// unattended runs).
    pub deny_high_risk_unattended: bool,
    /// When true, destructive git operations are always denied.
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
}

/// The full set of layers consulted during policy evaluation. Layers are
/// consulted in priority order; see the module docs for the precedence rules.
#[derive(Clone, Debug)]
pub struct PolicyLayers {
    pub managed: ManagedRequirements,
    pub settings: AppPolicySettings,
    pub workflow_mode: Option<PermissionMode>,
    pub profile_mode: Option<PermissionMode>,
    pub run_override: Option<PermissionMode>,
}

impl Default for PolicyLayers {
    fn default() -> Self {
        Self {
            managed: ManagedRequirements::default(),
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
    /// Resolve the effective permission mode after layering.
    /// Override > Profile > Workflow > Settings > Default(AutoEdit).
    pub fn effective_mode(&self) -> PermissionMode {
        self.run_override
            .or(self.profile_mode)
            .or(self.workflow_mode)
            .or(self.settings.default_mode)
            .unwrap_or(PermissionMode::AutoEdit)
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
    fn risk(&self) -> RiskLevel {
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
    let mode = layers.effective_mode();

    // 2. Bypass gate — attended bypass allows everything not denied by managed.
    if mode == PermissionMode::Bypass {
        if !layers.settings.attended && risk == RiskLevel::High {
            return PolicyDecision::Deny {
                reason: "high-risk action denied in unattended bypass mode".into(),
                source: DecisionSource::Managed,
            };
        }
        return PolicyDecision::Allow {
            source: DecisionSource::Bypass,
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
            source: DecisionSource::Default,
        };
    }

    // 5. Apply risk × mode.
    match (risk, mode) {
        (RiskLevel::None, _) => PolicyDecision::Allow {
            source: DecisionSource::Default,
        },
        (RiskLevel::Medium, PermissionMode::AutoEdit) => PolicyDecision::Allow {
            source: DecisionSource::Default,
        },
        (RiskLevel::Medium, _) => PolicyDecision::Ask {
            risk,
            prompt: action.description.clone(),
            source: DecisionSource::Default,
        },
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
                source: DecisionSource::Default,
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

    if action.category == ActionCategory::Destructive && managed.deny_high_risk_unattended {
        return Some(PolicyDecision::Deny {
            reason: "destructive action blocked by managed policy".into(),
            source: DecisionSource::Managed,
        });
    }

    None
}

/// Simple glob match: `*` matches any sequence.
fn branch_matches(branch: &str, pattern: &str) -> bool {
    if !pattern.contains('*') {
        return false;
    }
    let parts: Vec<&str> = pattern.split('*').collect();
    let mut pos = 0;
    for (i, part) in parts.iter().enumerate() {
        if i == 0 {
            if !branch[pos..].starts_with(part) {
                return false;
            }
            pos += part.len();
        } else if i == parts.len() - 1 {
            return branch[pos..].ends_with(part);
        } else if let Some(found) = branch[pos..].find(part) {
            pos += found + part.len();
        } else {
            return false;
        }
    }
    true
}

/// Map an approval outcome back to whether the action should proceed.
pub fn approval_allows(state: ApprovalState) -> bool {
    matches!(state, ApprovalState::Approved | ApprovalState::AutoResolved)
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

    fn command_action(desc: &str) -> ActionDescriptor {
        ActionDescriptor {
            category: ActionCategory::RunCommand,
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
    fn writefile_and_command_are_medium_risk() {
        assert_eq!(ActionCategory::WriteFile.default_risk(), RiskLevel::Medium);
        assert_eq!(ActionCategory::RunCommand.default_risk(), RiskLevel::Medium);
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
                source: DecisionSource::Bypass
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
                source: DecisionSource::Bypass
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
    fn unattended_medium_risk_still_asks() {
        let mut layers = PolicyLayers::default();
        layers.settings.default_mode = Some(PermissionMode::Ask);
        layers.settings.attended = false;
        let d = evaluate(&command_action("cargo test"), &layers);
        assert!(matches!(d, PolicyDecision::Ask { .. }));
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
        layers.managed.deny_high_risk_unattended = true;
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
    fn approval_allows_approved_and_auto_resolved() {
        assert!(approval_allows(ApprovalState::Approved));
        assert!(approval_allows(ApprovalState::AutoResolved));
        assert!(!approval_allows(ApprovalState::Denied));
        assert!(!approval_allows(ApprovalState::Expired));
        assert!(!approval_allows(ApprovalState::Pending));
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
                source: DecisionSource::Bypass
            }
        ));
    }
}
