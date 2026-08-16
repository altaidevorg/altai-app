//! Plugin manifest and capability contracts (package 071).
//!
//! A plugin is either **agent content** (instructions, prompts, skills an
//! agent consumes — no runtime of its own) or an **application** (a worker
//! with jobs, webhooks, scoped secrets or UI surfaces, packages 072–073).
//! The two kinds are distinct by construction: an agent-content plugin
//! declares no runtime capabilities, and the runtime capabilities are
//! reserved for application plugins.
//!
//! Upgrades must disclose capability expansion: [`PluginUpgradeDisclosure`]
//! computes what an upgrade adds and removes so the registry can refuse a
//! silent expansion (PR 2 enforces the disclosure on install/upgrade).
//!
//! Since 073 PR 1 the manifest may carry a UI declaration
//! ([`ui`](PluginManifest::ui)) — a schema the host renders, validated
//! here so registration is where an unsound UI dies.

use crate::PluginId;
use crate::plugin_ui::PluginUiDeclaration;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// What a plugin contributes to the system. Grounded in the runtime
/// packages: `072` (jobs, webhooks, scoped secrets) and `073`
/// (schema-driven UI). All of them assume a worker — so they are
/// application-plugin capabilities.
///
/// The derived `Ord` (declaration order: jobs, webhooks, scoped_secrets,
/// plugin_ui) is a wire-visible invariant: the upgrade disclosure lists
/// capabilities in this order, and the TS mirror sorts to match it. Do not
/// reorder the variants without updating `plugin.ts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginCapability {
    /// Background job execution.
    Jobs,
    /// Inbound/outbound webhook delivery.
    Webhooks,
    /// Access to secrets scoped to this plugin.
    ScopedSecrets,
    /// Schema-driven UI surfaces (073).
    PluginUi,
}

/// The plugin's classification. Mutually exclusive by design.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginKind {
    /// Content an agent consumes (instructions, prompts, skills). No
    /// runtime, no capabilities.
    AgentContent,
    /// An application with a worker process (072) and optional UI (073).
    Application,
}

impl PluginKind {
    /// The capabilities this kind may declare. Agent content declares none.
    pub fn allowed_capabilities(self) -> &'static [PluginCapability] {
        match self {
            Self::AgentContent => &[],
            Self::Application => &[
                PluginCapability::Jobs,
                PluginCapability::Webhooks,
                PluginCapability::ScopedSecrets,
                PluginCapability::PluginUi,
            ],
        }
    }
}

/// Numeric plugin version. Ordering is numeric per component, so
/// `1.2.0 < 1.10.0` (not string-lexicographic).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PluginVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl PluginVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }
}

impl std::fmt::Display for PluginVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// A plugin's declared identity and capabilities. The manifest is the
/// contract the registry (071 PR 2) verifies and the worker runtime (072)
/// enforces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginManifest {
    pub plugin_id: PluginId,
    pub kind: PluginKind,
    pub version: PluginVersion,
    pub display_name: String,
    pub capabilities: Vec<PluginCapability>,
    /// The plugin's declared UI surfaces (073). Optional and absent on
    /// the wire for plugins that predate it; a declaration requires the
    /// `PluginUi` capability and validates with the rest of the
    /// manifest, so registration refuses an unsound UI up front.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui: Option<PluginUiDeclaration>,
}

/// Typed manifest validation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginManifestError {
    /// A capability appears more than once.
    DuplicateCapability { capability: PluginCapability },
    /// The plugin's kind may not declare this capability.
    CapabilityNotAllowedForKind { kind: PluginKind, capability: PluginCapability },
    /// The display name is empty.
    EmptyDisplayName,
    /// The declared UI failed its own validation; the manifest is
    /// refused with the UI's reason.
    InvalidUi { reason: crate::plugin_ui::PluginUiError },
}

impl PluginManifest {
    /// Validate the manifest's internal consistency: non-empty name, no
    /// duplicate capabilities, capabilities permitted for the kind, and
    /// — when present — a UI declaration that is sound against them.
    pub fn validate(&self) -> Result<(), PluginManifestError> {
        if self.display_name.trim().is_empty() {
            return Err(PluginManifestError::EmptyDisplayName);
        }
        if let Some(ui) = &self.ui {
            ui.validate(&self.capabilities)
                .map_err(|reason| PluginManifestError::InvalidUi { reason })?;
        }
        let mut seen = BTreeSet::new();
        for capability in &self.capabilities {
            if !seen.insert(*capability) {
                return Err(PluginManifestError::DuplicateCapability {
                    capability: *capability,
                });
            }
            if !self.kind.allowed_capabilities().contains(capability) {
                return Err(PluginManifestError::CapabilityNotAllowedForKind {
                    kind: self.kind,
                    capability: *capability,
                });
            }
        }
        Ok(())
    }
}

/// What an upgrade changes about a plugin's declared capabilities. An
/// upgrade that adds capabilities must be disclosed — never installed
/// silently.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginUpgradeDisclosure {
    pub plugin_id: PluginId,
    pub from_version: PluginVersion,
    pub to_version: PluginVersion,
    pub added_capabilities: Vec<PluginCapability>,
    pub removed_capabilities: Vec<PluginCapability>,
}

impl PluginUpgradeDisclosure {
    /// Diff two manifests of the same plugin. The manifests themselves must
    /// already be valid; the diff only reports capability movement and
    /// version direction.
    pub fn diff(previous: &PluginManifest, next: &PluginManifest) -> Self {
        let previous_set: BTreeSet<_> = previous.capabilities.iter().copied().collect();
        let next_set: BTreeSet<_> = next.capabilities.iter().copied().collect();
        Self {
            plugin_id: next.plugin_id.clone(),
            from_version: previous.version,
            to_version: next.version,
            added_capabilities: next_set.difference(&previous_set).copied().collect(),
            removed_capabilities: previous_set.difference(&next_set).copied().collect(),
        }
    }

    /// True when the version strictly increases.
    pub fn is_version_advance(&self) -> bool {
        self.to_version > self.from_version
    }

    /// True when the upgrade expands the declared capability set — the case
    /// the registry must disclose (and the user must accept) explicitly.
    pub fn expands_capabilities(&self) -> bool {
        !self.added_capabilities.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(kind: PluginKind, capabilities: &[PluginCapability]) -> PluginManifest {
        PluginManifest {
            plugin_id: PluginId::new("plg_demo"),
            kind,
            version: PluginVersion::new(1, 0, 0),
            display_name: "Demo plugin".into(),
            capabilities: capabilities.to_vec(),
            ui: None,
        }
    }

    #[test]
    fn manifests_round_trip_with_snake_case_kinds() {
        let manifest = manifest(
            PluginKind::Application,
            &[PluginCapability::Jobs, PluginCapability::PluginUi],
        );
        let json = serde_json::to_string(&manifest).unwrap();
        assert!(json.contains(r#""kind":"application""#));
        assert!(json.contains(r#""plugin_ui""#));
        let parsed: PluginManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, manifest);
    }

    #[test]
    fn agent_content_and_application_are_distinct() {
        // An application plugin may declare every runtime capability.
        manifest(
            PluginKind::Application,
            &[
                PluginCapability::Jobs,
                PluginCapability::Webhooks,
                PluginCapability::ScopedSecrets,
                PluginCapability::PluginUi,
            ],
        )
        .validate()
        .unwrap();
        // An agent-content plugin may not declare any of them.
        for capability in PluginKind::Application.allowed_capabilities() {
            assert_eq!(
                manifest(PluginKind::AgentContent, &[*capability]).validate(),
                Err(PluginManifestError::CapabilityNotAllowedForKind {
                    kind: PluginKind::AgentContent,
                    capability: *capability,
                })
            );
        }
        // Bare agent content is valid.
        manifest(PluginKind::AgentContent, &[]).validate().unwrap();
    }

    #[test]
    fn duplicate_capabilities_are_rejected() {
        assert_eq!(
            manifest(
                PluginKind::Application,
                &[PluginCapability::Jobs, PluginCapability::Jobs]
            )
            .validate(),
            Err(PluginManifestError::DuplicateCapability {
                capability: PluginCapability::Jobs
            })
        );
    }

    #[test]
    fn empty_display_names_are_rejected() {
        let mut bare = manifest(PluginKind::Application, &[]);
        bare.display_name = "   ".into();
        assert_eq!(bare.validate(), Err(PluginManifestError::EmptyDisplayName));
    }

    #[test]
    fn versions_order_numerically() {
        assert!(PluginVersion::new(1, 2, 0) < PluginVersion::new(1, 10, 0));
        assert!(PluginVersion::new(1, 9, 9) < PluginVersion::new(2, 0, 0));
        assert_eq!(PluginVersion::new(1, 0, 3).to_string(), "1.0.3");
    }

    #[test]
    fn upgrade_diff_discloses_expansion() {
        let previous = manifest(PluginKind::Application, &[PluginCapability::Jobs]);
        let mut next = previous.clone();
        next.version = PluginVersion::new(1, 1, 0);
        next.capabilities = vec![PluginCapability::Jobs, PluginCapability::Webhooks];
        let disclosure = PluginUpgradeDisclosure::diff(&previous, &next);
        assert!(disclosure.is_version_advance());
        assert!(disclosure.expands_capabilities());
        assert_eq!(disclosure.added_capabilities, vec![PluginCapability::Webhooks]);
        assert!(disclosure.removed_capabilities.is_empty());

        // A removal is not an expansion; a pure version bump is neither.
        let mut shrunk = previous.clone();
        shrunk.version = PluginVersion::new(1, 1, 0);
        shrunk.capabilities = vec![];
        let removal = PluginUpgradeDisclosure::diff(&previous, &shrunk);
        assert!(!removal.expands_capabilities());
        assert_eq!(removal.removed_capabilities, vec![PluginCapability::Jobs]);

        let mut bumped = previous.clone();
        bumped.version = PluginVersion::new(1, 0, 1);
        let pure = PluginUpgradeDisclosure::diff(&previous, &bumped);
        assert!(pure.is_version_advance());
        assert!(!pure.expands_capabilities());

        // Downgrades are not version advances.
        let mut downgrade = previous.clone();
        downgrade.version = PluginVersion::new(0, 9, 0);
        assert!(!PluginUpgradeDisclosure::diff(&previous, &downgrade).is_version_advance());
    }
}
