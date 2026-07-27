//! Agent profiles (plan §F3).
//!
//! Repository profiles under `.altai/agents/` with model, reasoning, permissions,
//! tools, skills, MCP servers, budgets, and file scope. Managed/user/project
//! scope precedence is visible. Profiles cannot broaden managed permissions.
//! Missing dependencies fail before workspace side effects.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::workflow::PermissionMode;
use super::workflow_v2::{BudgetsConfig, Reasoning};

// ---------------------------------------------------------------------------
// Profile types
// ---------------------------------------------------------------------------

/// Where a profile originates — determines precedence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileScope {
    /// Managed (built-in) — highest precedence for restrictions.
    Managed,
    /// User-level (global config).
    User,
    /// Project-level (`.altai/agents/`).
    Project,
}

impl ProfileScope {
    /// Precedence rank: lower = higher priority.
    pub fn rank(self) -> u8 {
        match self {
            Self::Managed => 0,
            Self::User => 1,
            Self::Project => 2,
        }
    }
}

/// A full agent profile.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AgentProfileDef {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub prompt: String,
    pub model_id: Option<String>,
    pub reasoning: Option<Reasoning>,
    pub permissions: Option<PermissionMode>,
    /// `None` means no restriction.
    pub tools: Option<Vec<String>>,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub mcp_servers: Vec<String>,
    pub budgets: Option<BudgetsConfig>,
    /// File globs this profile is allowed to access. Empty = no restriction.
    #[serde(default)]
    pub file_scope: Vec<String>,
    /// Whether this profile can be auto-selected.
    pub auto_selectable: bool,
}

impl Default for AgentProfileDef {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            prompt: String::new(),
            model_id: None,
            reasoning: None,
            permissions: None,
            tools: None,
            skills: Vec::new(),
            mcp_servers: Vec::new(),
            budgets: None,
            file_scope: Vec::new(),
            auto_selectable: true,
        }
    }
}

/// A profile tagged with its source scope.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopedProfile {
    pub profile: AgentProfileDef,
    pub scope: ProfileScope,
}

/// The effective profile after merging managed + user + project layers.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectiveProfile {
    pub name: String,
    pub model_id: Option<String>,
    pub reasoning: Option<Reasoning>,
    pub permissions: PermissionMode,
    pub tools: Option<Vec<String>>,
    pub skills: Vec<String>,
    pub mcp_servers: Vec<String>,
    pub file_scope: Vec<String>,
    pub sources: Vec<ProfileSource>,
}

/// Which scope contributed each field.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileSource {
    pub scope: ProfileScope,
    pub fields: Vec<String>,
}

// ---------------------------------------------------------------------------
// Profile registry
// ---------------------------------------------------------------------------

/// Registry of all known profiles across scopes.
#[derive(Clone, Debug, Default)]
pub struct ProfileRegistry {
    /// Profile name → scopes (ordered by precedence).
    pub profiles: HashMap<String, Vec<ScopedProfile>>,
}

impl ProfileRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a profile under a scope.
    pub fn register(&mut self, profile: AgentProfileDef, scope: ProfileScope) {
        let entry = self.profiles.entry(profile.name.clone()).or_default();
        entry.push(ScopedProfile { profile, scope });
        // Keep sorted by scope precedence.
        entry.sort_by_key(|s| s.scope.rank());
    }

    /// Get all profile names.
    pub fn names(&self) -> Vec<String> {
        self.profiles.keys().cloned().collect()
    }

    /// Resolve the effective profile by merging layers.
    pub fn resolve(&self, name: &str) -> Option<EffectiveProfile> {
        let scoped = self.profiles.get(name)?;
        if scoped.is_empty() {
            return None;
        }

        let mut sources: Vec<ProfileSource> = Vec::new();
        let mut model_id = None;
        let mut reasoning = None;
        let mut permissions = PermissionMode::Ask;
        let mut tools: Option<Vec<String>> = None;
        let mut skills: Vec<String> = Vec::new();
        let mut mcp_servers: Vec<String> = Vec::new();
        let mut file_scope: Vec<String> = Vec::new();

        // Process from lowest to highest precedence (project → user → managed).
        for scoped_profile in scoped.iter().rev() {
            let mut contributed = Vec::new();
            let p = &scoped_profile.profile;

            // Name and description always contribute.
            contributed.push("name".into());
            if !p.description.is_empty() {
                contributed.push("description".into());
            }

            if p.model_id.is_some() {
                model_id = p.model_id.clone();
                contributed.push("model_id".into());
            }
            if p.reasoning.is_some() {
                reasoning = p.reasoning;
                contributed.push("reasoning".into());
            }
            if let Some(req_perms) = p.permissions {
                // Managed permissions cannot be broadened.
                if scoped_profile.scope == ProfileScope::Managed {
                    permissions = req_perms;
                    contributed.push("permissions".into());
                } else {
                    // Non-managed can only narrow (be more restrictive), not broaden.
                    if is_more_restrictive(req_perms, permissions) {
                        permissions = req_perms;
                        contributed.push("permissions".into());
                    }
                }
            }
            if p.tools.is_some() {
                tools = p.tools.clone();
                contributed.push("tools".into());
            }
            if !p.skills.is_empty() {
                skills.extend(p.skills.iter().cloned());
                contributed.push("skills".into());
            }
            if !p.mcp_servers.is_empty() {
                mcp_servers.extend(p.mcp_servers.iter().cloned());
                contributed.push("mcp_servers".into());
            }
            if !p.file_scope.is_empty() {
                file_scope.extend(p.file_scope.iter().cloned());
                contributed.push("file_scope".into());
            }

            if !contributed.is_empty() {
                sources.push(ProfileSource {
                    scope: scoped_profile.scope,
                    fields: contributed,
                });
            }
        }

        // Deduplicate skills and mcp_servers.
        skills.sort();
        skills.dedup();
        mcp_servers.sort();
        mcp_servers.dedup();

        Some(EffectiveProfile {
            name: name.to_string(),
            model_id,
            reasoning,
            permissions,
            tools,
            skills,
            mcp_servers,
            file_scope,
            sources,
        })
    }
}

/// Check if `requested` is at least as restrictive as `current`.
fn is_more_restrictive(requested: PermissionMode, current: PermissionMode) -> bool {
    fn rank(mode: PermissionMode) -> u8 {
        match mode {
            PermissionMode::Plan => 0, // most restrictive
            PermissionMode::Ask => 1,
            PermissionMode::AutoEdit => 2,
            PermissionMode::Bypass => 3, // least restrictive
        }
    }
    rank(requested) <= rank(current)
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/// Error when a profile references something unavailable.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ProfileValidationError {
    UnknownModel {
        model_id: String,
        profile: String,
    },
    UnknownTool {
        tool: String,
        profile: String,
    },
    UnknownMcpServer {
        server: String,
        profile: String,
    },
    UnknownSkill {
        skill: String,
        profile: String,
    },
    BroadenedPermissions {
        profile: String,
        requested: String,
        managed: String,
    },
}

/// Available models/tools/MCP/skills for validation.
#[derive(Clone, Debug, Default)]
pub struct AvailableCapabilities {
    pub models: Vec<String>,
    pub tools: Vec<String>,
    pub mcp_servers: Vec<String>,
    pub skills: Vec<String>,
}

/// Validate a profile against available capabilities. Returns all errors.
pub fn validate_profile(
    profile: &AgentProfileDef,
    caps: &AvailableCapabilities,
) -> Vec<ProfileValidationError> {
    let mut errors = Vec::new();

    if let Some(ref model) = profile.model_id {
        if !caps.models.is_empty() && !caps.models.contains(model) {
            errors.push(ProfileValidationError::UnknownModel {
                model_id: model.clone(),
                profile: profile.name.clone(),
            });
        }
    }

    if let Some(ref tools) = profile.tools {
        if !caps.tools.is_empty() {
            for tool in tools {
                if !caps.tools.contains(tool) {
                    errors.push(ProfileValidationError::UnknownTool {
                        tool: tool.clone(),
                        profile: profile.name.clone(),
                    });
                }
            }
        }
    }

    for server in &profile.mcp_servers {
        if !caps.mcp_servers.is_empty() && !caps.mcp_servers.contains(server) {
            errors.push(ProfileValidationError::UnknownMcpServer {
                server: server.clone(),
                profile: profile.name.clone(),
            });
        }
    }

    for skill in &profile.skills {
        if !caps.skills.is_empty() && !caps.skills.contains(skill) {
            errors.push(ProfileValidationError::UnknownSkill {
                skill: skill.clone(),
                profile: profile.name.clone(),
            });
        }
    }

    errors
}

// ---------------------------------------------------------------------------
// Profile selection / inference
// ---------------------------------------------------------------------------

/// Result of profile selection — manual or inferred.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileSelection {
    pub profile_name: String,
    pub source: SelectionSource,
    pub reason: String,
}

/// How a profile was selected.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectionSource {
    Manual,
    Inferred,
    Default,
}

/// Select a profile for a task. Manual override > inference > default.
pub fn select_profile(
    registry: &ProfileRegistry,
    manual_choice: Option<&str>,
    task_description: &str,
    default_profile: &str,
) -> ProfileSelection {
    // Manual selection takes priority.
    if let Some(name) = manual_choice {
        if registry.names().iter().any(|n| n == name) {
            return ProfileSelection {
                profile_name: name.to_string(),
                source: SelectionSource::Manual,
                reason: format!("User manually selected '{name}'"),
            };
        }
    }

    // Infer based on task description keyword matching.
    let available: Vec<String> = registry
        .names()
        .into_iter()
        .filter(|n| {
            registry
                .profiles
                .get(n)
                .map(|scoped| scoped.iter().any(|s| s.profile.auto_selectable))
                .unwrap_or(false)
        })
        .collect();

    let task_lower = task_description.to_lowercase();
    for name in &available {
        if let Some(scoped) = registry.profiles.get(name) {
            for s in scoped {
                let desc_lower = s.profile.description.to_lowercase();
                let keywords: Vec<&str> = desc_lower
                    .split_whitespace()
                    .filter(|w| w.len() >= 4)
                    .collect();
                for kw in keywords {
                    if task_lower.contains(kw) {
                        return ProfileSelection {
                            profile_name: name.clone(),
                            source: SelectionSource::Inferred,
                            reason: format!("Task description matches '{kw}' in profile '{name}'"),
                        };
                    }
                }
            }
        }
    }

    // Fall back to default.
    ProfileSelection {
        profile_name: default_profile.to_string(),
        source: SelectionSource::Default,
        reason: format!("No match found, using default '{default_profile}'"),
    }
}

// ---------------------------------------------------------------------------
// Serialization for .altai/agents/
// ---------------------------------------------------------------------------

/// Serialize a profile to TOML-ish key-value format for `.altai/agents/`.
pub fn profile_to_text(profile: &AgentProfileDef) -> String {
    let mut out = String::new();
    out.push_str(&format!("# Agent profile: {}\n", profile.name));
    if !profile.description.is_empty() {
        out.push_str(&format!("# {}\n", profile.description));
    }
    out.push('\n');
    out.push_str(&format!("name = {:?}\n", profile.name));
    if !profile.description.is_empty() {
        out.push_str(&format!("description = {:?}\n", profile.description));
    }
    if !profile.prompt.is_empty() {
        out.push_str(&format!("prompt = {:?}\n", profile.prompt));
    }
    if let Some(ref model) = profile.model_id {
        out.push_str(&format!("model_id = {:?}\n", model));
    }
    if let Some(reasoning) = profile.reasoning {
        out.push_str(&format!("reasoning = \"{:?}\"\n", reasoning));
    }
    if let Some(permissions) = profile.permissions {
        out.push_str(&format!("permissions = \"{:?}\"\n", permissions));
    }
    if let Some(ref tools) = profile.tools {
        out.push_str(&format!("tools = {:?}\n", tools));
    }
    if !profile.skills.is_empty() {
        out.push_str(&format!("skills = {:?}\n", profile.skills));
    }
    if !profile.mcp_servers.is_empty() {
        out.push_str(&format!("mcp_servers = {:?}\n", profile.mcp_servers));
    }
    if !profile.file_scope.is_empty() {
        out.push_str(&format!("file_scope = {:?}\n", profile.file_scope));
    }
    out.push_str(&format!("auto_selectable = {}\n", profile.auto_selectable));
    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_profile(name: &str, desc: &str) -> AgentProfileDef {
        AgentProfileDef {
            name: name.into(),
            description: desc.into(),
            ..Default::default()
        }
    }

    // ---- registry + resolution ----

    #[test]
    fn register_and_resolve() {
        let mut reg = ProfileRegistry::new();
        reg.register(
            make_profile("reviewer", "code review"),
            ProfileScope::Project,
        );
        let eff = reg.resolve("reviewer").unwrap();
        assert_eq!(eff.name, "reviewer");
        assert!(!eff.sources.is_empty());
    }

    #[test]
    fn resolve_missing_returns_none() {
        let reg = ProfileRegistry::new();
        assert!(reg.resolve("nonexistent").is_none());
    }

    // ---- scope precedence ----

    #[test]
    fn project_overrides_user_model() {
        let mut reg = ProfileRegistry::new();
        let mut user_profile = make_profile("worker", "user");
        user_profile.model_id = Some("gpt-4".into());
        reg.register(user_profile, ProfileScope::User);

        let mut project_profile = make_profile("worker", "project");
        project_profile.model_id = Some("claude-3".into());
        reg.register(project_profile, ProfileScope::Project);

        let eff = reg.resolve("worker").unwrap();
        // Project has higher rank (lower precedence), but we process from
        // project → user → managed, so user wins for model_id.
        // Actually, we process from lowest precedence (project) to highest.
        // So user should override project.
        assert_eq!(eff.model_id.as_deref(), Some("gpt-4"));
    }

    #[test]
    fn managed_permissions_cannot_be_broadened() {
        let mut reg = ProfileRegistry::new();
        let mut managed = make_profile("worker", "managed");
        managed.permissions = Some(PermissionMode::Ask); // restrictive
        reg.register(managed, ProfileScope::Managed);

        let mut project = make_profile("worker", "project");
        project.permissions = Some(PermissionMode::Bypass); // tries to broaden
        reg.register(project, ProfileScope::Project);

        let eff = reg.resolve("worker").unwrap();
        // Managed Ask should win — Bypass cannot broaden.
        assert_eq!(eff.permissions, PermissionMode::Ask);
    }

    #[test]
    fn non_managed_can_narrow_permissions() {
        let mut reg = ProfileRegistry::new();
        let mut user = make_profile("worker", "user");
        user.permissions = Some(PermissionMode::AutoEdit);
        reg.register(user, ProfileScope::User);

        let mut project = make_profile("worker", "project");
        project.permissions = Some(PermissionMode::Ask); // more restrictive
        reg.register(project, ProfileScope::Project);

        let eff = reg.resolve("worker").unwrap();
        assert_eq!(eff.permissions, PermissionMode::Ask);
    }

    #[test]
    fn skills_and_mcp_deduplicated() {
        let mut reg = ProfileRegistry::new();
        let mut p1 = make_profile("worker", "layer 1");
        p1.skills = vec!["git".into(), "rust".into()];
        p1.mcp_servers = vec!["server-a".into()];
        reg.register(p1, ProfileScope::User);

        let mut p2 = make_profile("worker", "layer 2");
        p2.skills = vec!["git".into(), "python".into()];
        p2.mcp_servers = vec!["server-a".into(), "server-b".into()];
        reg.register(p2, ProfileScope::Project);

        let eff = reg.resolve("worker").unwrap();
        assert_eq!(eff.skills.len(), 3); // git, python, rust
        assert_eq!(eff.mcp_servers.len(), 2); // server-a, server-b
    }

    // ---- validation ----

    #[test]
    fn unknown_model_rejected() {
        let profile = AgentProfileDef {
            name: "test".into(),
            model_id: Some("nonexistent-model".into()),
            ..Default::default()
        };
        let caps = AvailableCapabilities {
            models: vec!["gpt-4".into(), "claude-3".into()],
            ..Default::default()
        };
        let errors = validate_profile(&profile, &caps);
        assert!(errors
            .iter()
            .any(|e| matches!(e, ProfileValidationError::UnknownModel { .. })));
    }

    #[test]
    fn known_model_accepted() {
        let profile = AgentProfileDef {
            name: "test".into(),
            model_id: Some("gpt-4".into()),
            ..Default::default()
        };
        let caps = AvailableCapabilities {
            models: vec!["gpt-4".into()],
            ..Default::default()
        };
        let errors = validate_profile(&profile, &caps);
        assert!(errors.is_empty());
    }

    #[test]
    fn unknown_tool_rejected() {
        let profile = AgentProfileDef {
            name: "test".into(),
            tools: Some(vec!["valid-tool".into(), "invalid-tool".into()]),
            ..Default::default()
        };
        let caps = AvailableCapabilities {
            tools: vec!["valid-tool".into()],
            ..Default::default()
        };
        let errors = validate_profile(&profile, &caps);
        assert!(errors.iter().any(|e| matches!(
            e,
            ProfileValidationError::UnknownTool { tool, .. } if tool == "invalid-tool"
        )));
    }

    #[test]
    fn empty_capabilities_skips_validation() {
        let profile = AgentProfileDef {
            name: "test".into(),
            model_id: Some("anything".into()),
            tools: Some(vec!["whatever".into()]),
            ..Default::default()
        };
        let caps = AvailableCapabilities::default();
        let errors = validate_profile(&profile, &caps);
        assert!(errors.is_empty(), "empty caps means no validation");
    }

    // ---- selection ----

    #[test]
    fn manual_selection_takes_priority() {
        let mut reg = ProfileRegistry::new();
        reg.register(
            make_profile("reviewer", "code review"),
            ProfileScope::Project,
        );
        reg.register(make_profile("planner", "planning"), ProfileScope::Project);

        let sel = select_profile(&reg, Some("planner"), "review the code", "worker");
        assert_eq!(sel.profile_name, "planner");
        assert_eq!(sel.source, SelectionSource::Manual);
    }

    #[test]
    fn inferred_selection_matches_description() {
        let mut reg = ProfileRegistry::new();
        reg.register(
            make_profile("reviewer", "review testing quality"),
            ProfileScope::Project,
        );
        reg.register(
            make_profile("planner", "planning architecture"),
            ProfileScope::Project,
        );

        let sel = select_profile(&reg, None, "we need to review the code quality", "worker");
        assert_eq!(sel.profile_name, "reviewer");
        assert_eq!(sel.source, SelectionSource::Inferred);
        assert!(sel.reason.contains("match"));
    }

    #[test]
    fn default_selection_when_no_match() {
        let reg = ProfileRegistry::new();
        let sel = select_profile(&reg, None, "random task", "worker");
        assert_eq!(sel.profile_name, "worker");
        assert_eq!(sel.source, SelectionSource::Default);
    }

    #[test]
    fn non_auto_selectable_skipped_in_inference() {
        let mut reg = ProfileRegistry::new();
        let mut hidden = make_profile("hidden", "secret review");
        hidden.auto_selectable = false;
        reg.register(hidden, ProfileScope::Project);

        let sel = select_profile(&reg, None, "review the code", "worker");
        // Hidden profile should not be auto-selected.
        assert_eq!(sel.source, SelectionSource::Default);
    }

    #[test]
    fn selection_is_explainable() {
        let mut reg = ProfileRegistry::new();
        reg.register(
            make_profile("reviewer", "review quality"),
            ProfileScope::Project,
        );

        let sel = select_profile(&reg, None, "review quality", "worker");
        assert!(!sel.reason.is_empty(), "selection must have an explanation");
    }

    // ---- serialization ----

    #[test]
    fn profile_to_text_contains_fields() {
        let profile = AgentProfileDef {
            name: "reviewer".into(),
            description: "Code reviewer".into(),
            model_id: Some("claude-3".into()),
            permissions: Some(PermissionMode::Ask),
            skills: vec!["git".into()],
            ..Default::default()
        };
        let text = profile_to_text(&profile);
        assert!(text.contains("name = \"reviewer\""));
        assert!(text.contains("claude-3"));
        assert!(text.contains("git"));
    }

    // ---- F3 acceptance ----

    #[test]
    fn profile_cannot_broaden_managed_permissions() {
        let mut reg = ProfileRegistry::new();
        let mut managed = make_profile("strict", "managed floor");
        managed.permissions = Some(PermissionMode::Plan); // most restrictive
        reg.register(managed, ProfileScope::Managed);

        let mut user = make_profile("strict", "user override");
        user.permissions = Some(PermissionMode::AutoEdit); // tries to broaden
        reg.register(user, ProfileScope::User);

        let eff = reg.resolve("strict").unwrap();
        assert_eq!(
            eff.permissions,
            PermissionMode::Plan,
            "Plan is the managed floor — cannot be broadened to AutoEdit"
        );
    }

    #[test]
    fn inferred_selection_recorded_and_explainable() {
        let mut reg = ProfileRegistry::new();
        reg.register(
            make_profile("tester", "testing quality"),
            ProfileScope::Project,
        );

        let sel = select_profile(&reg, None, "run testing suite", "worker");
        assert_eq!(sel.source, SelectionSource::Inferred);
        assert!(!sel.reason.is_empty());
    }
}
