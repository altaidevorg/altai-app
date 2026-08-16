//! CP-08 org-scoped plugin registry. Stores validated [`PluginManifest`]
//! records per organization and enforces the upgrade disclosure rules from the
//! 071 contract: versions move forward (downgrades and divergent same-version
//! rewrites are refused) and capability expansion only commits when the caller
//! explicitly accepts it. An installed manifest is durable truth — installing
//! is not a client-side assertion.

use altai_control_protocol::{
    OrganizationId, PluginId, PluginManifest, PluginManifestError, PluginUpgradeDisclosure,
};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use std::{path::Path, sync::Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginRegistryError {
    /// The manifest failed contract validation; nothing was stored.
    InvalidManifest { reason: PluginManifestError },
    /// The target version is lower than the installed one, or equal with a
    /// divergent manifest. Stored manifests move forward or not at all.
    VersionDidNotAdvance { from_version: String, to_version: String },
    /// The upgrade adds capabilities and the caller did not accept the
    /// disclosure; the stored row is untouched.
    ExpansionNotAccepted { disclosure: PluginUpgradeDisclosure },
    Internal { reason: String },
}

impl std::fmt::Display for PluginRegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "plugin registry error: {self:?}")
    }
}
impl std::error::Error for PluginRegistryError {}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginRegistryOutcome {
    /// The manifest is installed as-is: a first registration or a
    /// byte-identical re-install.
    Installed { manifest: PluginManifest },
    /// A strictly newer version was committed. `disclosure` records what
    /// changed; if capabilities were added, the caller accepted the expansion.
    Upgraded { disclosure: PluginUpgradeDisclosure, manifest: PluginManifest },
}

pub trait PluginRegistry: Send + Sync {
    /// Install (first registration or upgrade) a manifest for an organization.
    /// `accept_expansion` is only consulted when the upgrade adds
    /// capabilities; a first install needs no consent flag because installing
    /// IS the consent.
    fn install(
        &self,
        organization_id: &OrganizationId,
        manifest: PluginManifest,
        accept_expansion: bool,
    ) -> Result<PluginRegistryOutcome, PluginRegistryError>;
    fn get(
        &self,
        organization_id: &OrganizationId,
        plugin_id: &PluginId,
    ) -> Result<Option<PluginManifest>, PluginRegistryError>;
    /// Every installed manifest in an organization (org equality filter).
    fn list_in_org(&self, organization_id: &OrganizationId) -> Result<Vec<PluginManifest>, PluginRegistryError>;
}

pub struct SqlitePluginRegistry {
    connection: Mutex<Connection>,
}

impl SqlitePluginRegistry {
    pub fn open(path: &Path) -> Result<Self, String> {
        let connection = Connection::open(path).map_err(|e| e.to_string())?;
        connection.execute_batch(
            "PRAGMA busy_timeout = 5000;
             CREATE TABLE IF NOT EXISTS control_plane_plugins (
                 organization_id TEXT NOT NULL,
                 plugin_id TEXT NOT NULL,
                 manifest_json TEXT NOT NULL,
                 PRIMARY KEY (organization_id, plugin_id)
             );",
        )
        .map_err(|e| e.to_string())?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, PluginRegistryError> {
        self.connection.lock().map_err(|_| PluginRegistryError::Internal {
            reason: "sqlite plugin registry lock poisoned".into(),
        })
    }
    fn db(e: rusqlite::Error) -> PluginRegistryError {
        PluginRegistryError::Internal { reason: e.to_string() }
    }
    fn encode(manifest: &PluginManifest) -> Result<String, PluginRegistryError> {
        serde_json::to_string(manifest).map_err(|e| PluginRegistryError::Internal { reason: e.to_string() })
    }
    fn decode(payload: &str) -> Result<PluginManifest, PluginRegistryError> {
        serde_json::from_str(payload)
            .map_err(|e| PluginRegistryError::Internal { reason: e.to_string() })
    }
    fn read_manifest(
        connection: &Connection,
        organization_id: &OrganizationId,
        plugin_id: &PluginId,
    ) -> Result<Option<PluginManifest>, PluginRegistryError> {
        let payload: Option<String> = connection
            .query_row(
                "SELECT manifest_json FROM control_plane_plugins WHERE organization_id=?1 AND plugin_id=?2",
                params![organization_id.value, plugin_id.value],
                |row| row.get(0),
            )
            .optional()
            .map_err(Self::db)?;
        payload.as_deref().map(Self::decode).transpose()
    }
}

impl PluginRegistry for SqlitePluginRegistry {
    fn install(
        &self,
        organization_id: &OrganizationId,
        manifest: PluginManifest,
        accept_expansion: bool,
    ) -> Result<PluginRegistryOutcome, PluginRegistryError> {
        manifest
            .validate()
            .map_err(|reason| PluginRegistryError::InvalidManifest { reason })?;
        let payload = Self::encode(&manifest)?;
        let plugin_id = manifest.plugin_id.value.clone();
        let mut connection = self.lock()?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(Self::db)?;
        let previous = Self::read_manifest(&tx, organization_id, &manifest.plugin_id)?;
        let outcome = match previous {
            None => PluginRegistryOutcome::Installed { manifest },
            Some(previous) => {
                // Byte-identical re-install is idempotent.
                if previous == manifest {
                    PluginRegistryOutcome::Installed { manifest }
                } else {
                    let disclosure = PluginUpgradeDisclosure::diff(&previous, &manifest);
                    if !disclosure.is_version_advance() {
                        return Err(PluginRegistryError::VersionDidNotAdvance {
                            from_version: disclosure.from_version.to_string(),
                            to_version: disclosure.to_version.to_string(),
                        });
                    }
                    if disclosure.expands_capabilities() && !accept_expansion {
                        return Err(PluginRegistryError::ExpansionNotAccepted { disclosure });
                    }
                    PluginRegistryOutcome::Upgraded { disclosure, manifest }
                }
            }
        };
        // INSERT ... ON CONFLICT DO UPDATE keeps first registration and
        // upgrade on one statement; the rules above already decided which.
        tx.execute(
            "INSERT INTO control_plane_plugins (organization_id, plugin_id, manifest_json) VALUES (?1, ?2, ?3)
             ON CONFLICT (organization_id, plugin_id) DO UPDATE SET manifest_json=excluded.manifest_json",
            params![organization_id.value, plugin_id, payload],
        )
        .map_err(Self::db)?;
        tx.commit().map_err(Self::db)?;
        Ok(outcome)
    }

    fn get(
        &self,
        organization_id: &OrganizationId,
        plugin_id: &PluginId,
    ) -> Result<Option<PluginManifest>, PluginRegistryError> {
        let connection = self.lock()?;
        Self::read_manifest(&connection, organization_id, plugin_id)
    }

    fn list_in_org(&self, organization_id: &OrganizationId) -> Result<Vec<PluginManifest>, PluginRegistryError> {
        let connection = self.lock()?;
        let mut statement = connection
            .prepare("SELECT manifest_json FROM control_plane_plugins WHERE organization_id=?1")
            .map_err(Self::db)?;
        let rows = statement
            .query_map([organization_id.value.as_str()], |row| row.get::<_, String>(0))
            .map_err(Self::db)?;
        let mut manifests = Vec::new();
        for payload in rows {
            manifests.push(Self::decode(&payload.map_err(Self::db)?)?);
        }
        Ok(manifests)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use altai_control_protocol::{
        PluginCapability, PluginKind, PluginManifestError, PluginUiAction, PluginUiDeclaration,
        PluginUiError, PluginUiNode, PluginUiSurface, PluginVersion,
    };

    fn temp_registry() -> (tempfile::TempDir, SqlitePluginRegistry) {
        let dir = tempfile::tempdir().expect("tempdir");
        let registry = SqlitePluginRegistry::open(&dir.path().join("plugins.db")).expect("open");
        (dir, registry)
    }

    fn org() -> OrganizationId {
        OrganizationId::new("org_test")
    }

    fn agent_manifest(version: (u32, u32, u32), capabilities: Vec<PluginCapability>) -> PluginManifest {
        PluginManifest {
            plugin_id: PluginId::new("plg_alpha"),
            kind: PluginKind::AgentContent,
            version: PluginVersion::new(version.0, version.1, version.2),
            display_name: "Alpha".into(),
            capabilities,
            ui: None,
        }
    }

    #[test]
    fn first_install_stores_and_returns_installed() {
        let (_dir, registry) = temp_registry();
        let manifest = agent_manifest((1, 0, 0), vec![]);
        let outcome = registry.install(&org(), manifest.clone(), false).expect("install");
        assert_eq!(outcome, PluginRegistryOutcome::Installed { manifest: manifest.clone() });
        assert_eq!(registry.get(&org(), &manifest.plugin_id).expect("get"), Some(manifest));
    }

    #[test]
    fn identical_reinstall_is_idempotent() {
        let (_dir, registry) = temp_registry();
        let manifest = agent_manifest((1, 0, 0), vec![]);
        registry.install(&org(), manifest.clone(), false).expect("first");
        let outcome = registry.install(&org(), manifest.clone(), false).expect("reinstall");
        assert_eq!(outcome, PluginRegistryOutcome::Installed { manifest });
    }

    #[test]
    fn divergent_same_version_manifest_is_refused() {
        let (_dir, registry) = temp_registry();
        registry.install(&org(), agent_manifest((1, 0, 0), vec![]), false).expect("first");
        let mut divergent = agent_manifest((1, 0, 0), vec![]);
        divergent.display_name = "Beta".into();
        let error = registry.install(&org(), divergent.clone(), true).expect_err("refused");
        assert_eq!(
            error,
            PluginRegistryError::VersionDidNotAdvance {
                from_version: "1.0.0".into(),
                to_version: "1.0.0".into(),
            }
        );
        // Stored row untouched.
        assert_eq!(registry.get(&org(), &divergent.plugin_id).expect("get").unwrap().display_name, "Alpha");
    }

    #[test]
    fn downgrade_is_refused() {
        let (_dir, registry) = temp_registry();
        registry.install(&org(), agent_manifest((2, 0, 0), vec![]), false).expect("first");
        let error = registry.install(&org(), agent_manifest((1, 9, 9), vec![]), true).expect_err("refused");
        assert_eq!(
            error,
            PluginRegistryError::VersionDidNotAdvance {
                from_version: "2.0.0".into(),
                to_version: "1.9.9".into(),
            }
        );
    }

    #[test]
    fn upgrade_without_expansion_commits() {
        let (_dir, registry) = temp_registry();
        registry.install(&org(), agent_manifest((1, 0, 0), vec![]), false).expect("first");
        let outcome = registry.install(&org(), agent_manifest((1, 1, 0), vec![]), false).expect("upgrade");
        let PluginRegistryOutcome::Upgraded { disclosure, manifest } = outcome else {
            panic!("expected Upgraded");
        };
        assert!(disclosure.is_version_advance());
        assert!(!disclosure.expands_capabilities());
        assert_eq!(manifest.version.to_string(), "1.1.0");
        assert_eq!(registry.get(&org(), &manifest.plugin_id).expect("get").unwrap().version.to_string(), "1.1.0");
    }

    #[test]
    fn expansion_without_consent_is_refused_and_row_untouched() {
        let (_dir, registry) = temp_registry();
        // Application plugin: only application plugins may declare capabilities.
        let mut application = agent_manifest((1, 0, 0), vec![PluginCapability::Jobs]);
        application.kind = PluginKind::Application;
        registry.install(&org(), application.clone(), false).expect("first");
        let mut expanded = application.clone();
        expanded.version = PluginVersion::new(1, 1, 0);
        expanded.capabilities.push(PluginCapability::Webhooks);
        let error = registry.install(&org(), expanded.clone(), false).expect_err("refused");
        let PluginRegistryError::ExpansionNotAccepted { disclosure } = error else {
            panic!("expected ExpansionNotAccepted, got {error:?}");
        };
        assert_eq!(disclosure.added_capabilities, vec![PluginCapability::Webhooks]);
        assert_eq!(disclosure.removed_capabilities, vec![]);
        // Stored row untouched: still the 1.0.0 manifest.
        let stored = registry.get(&org(), &expanded.plugin_id).expect("get").unwrap();
        assert_eq!(stored.version.to_string(), "1.0.0");
        assert_eq!(stored.capabilities, vec![PluginCapability::Jobs]);
    }

    #[test]
    fn expansion_with_consent_commits_and_discloses() {
        let (_dir, registry) = temp_registry();
        let mut application = agent_manifest((1, 0, 0), vec![PluginCapability::Jobs]);
        application.kind = PluginKind::Application;
        registry.install(&org(), application.clone(), false).expect("first");
        let mut expanded = application.clone();
        expanded.version = PluginVersion::new(2, 0, 0);
        expanded.capabilities.push(PluginCapability::Webhooks);
        let outcome = registry.install(&org(), expanded.clone(), true).expect("upgrade");
        let PluginRegistryOutcome::Upgraded { disclosure, .. } = outcome else {
            panic!("expected Upgraded");
        };
        assert_eq!(disclosure.added_capabilities, vec![PluginCapability::Webhooks]);
        let stored = registry.get(&org(), &expanded.plugin_id).expect("get").unwrap();
        assert_eq!(stored.capabilities, vec![PluginCapability::Jobs, PluginCapability::Webhooks]);
    }

    #[test]
    fn invalid_manifest_is_refused_before_storage() {
        let (_dir, registry) = temp_registry();
        let invalid = agent_manifest((1, 0, 0), vec![PluginCapability::Jobs]);
        let error = registry.install(&org(), invalid.clone(), true).expect_err("refused");
        assert!(matches!(
            error,
            PluginRegistryError::InvalidManifest {
                reason: PluginManifestError::CapabilityNotAllowedForKind { .. }
            }
        ));
        assert_eq!(registry.get(&org(), &invalid.plugin_id).expect("get"), None);
    }

    #[test]
    fn a_manifest_with_an_unsound_ui_is_refused_at_registration() {
        // The declaration rides in the manifest, so the registry's
        // existing validation path is where an unsound UI dies — before
        // any storage, and inside the upgrade rules for free.
        let (_dir, registry) = temp_registry();
        let mut manifest = agent_manifest((1, 0, 0), vec![PluginCapability::PluginUi]);
        manifest.kind = PluginKind::Application;
        manifest.ui = Some(PluginUiDeclaration {
            surfaces: vec![PluginUiSurface {
                surface_id: "main".into(),
                title: "Panel".into(),
                root: PluginUiNode::Action {
                    label: "Run".into(),
                    action: PluginUiAction::InvokeJob {
                        job_id: "job_refresh".into(),
                    },
                },
            }],
        });
        // The invoke-job action needs Jobs, which this manifest does not
        // declare: refused, nothing stored.
        let error = registry.install(&org(), manifest.clone(), true).expect_err("refused");
        assert!(matches!(
            error,
            PluginRegistryError::InvalidManifest {
                reason: PluginManifestError::InvalidUi {
                    reason: PluginUiError::ActionCapabilityMissing { .. }
                }
            }
        ));
        assert_eq!(registry.get(&org(), &manifest.plugin_id).expect("get"), None);

        // With Jobs declared the same declaration installs.
        let mut sound = manifest.clone();
        sound.capabilities.push(PluginCapability::Jobs);
        registry.install(&org(), sound.clone(), false).expect("install");
        assert_eq!(
            registry.get(&org(), &sound.plugin_id).expect("get"),
            Some(sound)
        );
    }

    #[test]
    fn registry_rows_are_org_isolated() {
        let (_dir, registry) = temp_registry();
        let manifest = agent_manifest((1, 0, 0), vec![]);
        registry.install(&org(), manifest.clone(), false).expect("first");
        let other = OrganizationId::new("org_other");
        assert_eq!(registry.get(&other, &manifest.plugin_id).expect("get"), None);
        assert_eq!(registry.list_in_org(&other).expect("list"), Vec::new());
        assert_eq!(registry.list_in_org(&org()).expect("list"), vec![manifest]);
    }
}
