//! Fail-closed workspace/repository permission gate. [`WorkspaceScopeGate::permits`]
//! answers "may this workspace operate on its repository?" — **deny unless an
//! explicit grant exists**. A workspace with no repository binding denies; a
//! URL without a grant row denies; only a grant in the workspace's own
//! organization (resolved through its project, never trusted from the caller)
//! permits. The gate reads and answers; clone/fetch/delivery executors are
//! required to ask before acting.

use std::sync::Arc;

use altai_control_protocol::{OrganizationId, ProjectWorkspace};

use crate::{RepositoryScopeError, RepositoryScopeRepository, ScopeError, ScopeRepository};

pub struct WorkspaceScopeGate {
    grants: Arc<dyn RepositoryScopeRepository>,
    scopes: Arc<dyn ScopeRepository>,
}

impl WorkspaceScopeGate {
    pub fn new(
        grants: Arc<dyn RepositoryScopeRepository>,
        scopes: Arc<dyn ScopeRepository>,
    ) -> Self {
        Self { grants, scopes }
    }

    /// Decide whether `workspace` may operate on its repository. The
    /// workspace's organization is resolved from its project; an unreadable
    /// project is an error, not a permit.
    pub fn permits(
        &self,
        workspace: &ProjectWorkspace,
    ) -> Result<ScopePermit, WorkspaceScopeError> {
        let Some(repository_url) = workspace.repository_url.as_deref() else {
            return Ok(ScopePermit::Denied(DenialReason::WorkspaceNotBound));
        };
        let organization_id = self
            .scopes
            .get_project(&workspace.project_id)
            .map_err(WorkspaceScopeError::Scope)?
            .organization_id;
        if self
            .grants
            .is_granted(&organization_id, repository_url)
            .map_err(WorkspaceScopeError::RepositoryScope)?
        {
            return Ok(ScopePermit::Permitted {
                organization_id,
                repository_url: repository_url.to_string(),
            });
        }
        Ok(ScopePermit::Denied(DenialReason::RepositoryNotGranted {
            repository_url: repository_url.to_string(),
        }))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopePermit {
    Permitted {
        organization_id: OrganizationId,
        repository_url: String,
    },
    Denied(DenialReason),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DenialReason {
    /// The workspace has no repository binding to permit.
    WorkspaceNotBound,
    /// No grant exists for this URL in the workspace's organization.
    RepositoryNotGranted { repository_url: String },
}

#[derive(Debug)]
pub enum WorkspaceScopeError {
    Scope(ScopeError),
    RepositoryScope(RepositoryScopeError),
}

impl std::fmt::Display for WorkspaceScopeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Scope(e) => write!(f, "workspace scope project lookup failed: {e}"),
            Self::RepositoryScope(e) => write!(f, "workspace scope grant lookup failed: {e}"),
        }
    }
}
impl std::error::Error for WorkspaceScopeError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InMemoryScopeRepository, SqliteRepositoryScopeRepository};
    use altai_control_protocol::{
        Organization, Project, ProjectId, ProjectStatus, RepositoryScope, Revision, WorkspaceId,
    };

    const URL: &str = "https://github.com/altaidevorg/altai-app";

    struct Harness {
        _dir: tempfile::TempDir,
        gate: WorkspaceScopeGate,
        _scopes: Arc<InMemoryScopeRepository>,
        grants: Arc<SqliteRepositoryScopeRepository>,
    }

    fn harness() -> Harness {
        let dir = tempfile::tempdir().unwrap();
        let scopes = Arc::new(InMemoryScopeRepository::default());
        scopes
            .create_organization(Organization {
                id: OrganizationId::new("org"),
                name: "Org".into(),
                revision: Revision::INITIAL,
                created_at: "now".into(),
                updated_at: "now".into(),
            })
            .unwrap();
        scopes
            .create_project(Project {
                id: ProjectId::new("project"),
                organization_id: OrganizationId::new("org"),
                goal_ids: vec![],
                name: "Project".into(),
                description: String::new(),
                status: ProjectStatus::Active,
                revision: Revision::INITIAL,
                created_at: "now".into(),
                updated_at: "now".into(),
            })
            .unwrap();
        let grants = Arc::new(
            SqliteRepositoryScopeRepository::open(&dir.path().join("work.db")).unwrap(),
        );
        let gate = WorkspaceScopeGate::new(grants.clone(), scopes.clone());
        Harness {
            _dir: dir,
            gate,
            _scopes: scopes,
            grants,
        }
    }

    fn workspace(url: Option<&str>) -> ProjectWorkspace {
        ProjectWorkspace {
            id: WorkspaceId::new("ws"),
            project_id: ProjectId::new("project"),
            name: "Checkout".into(),
            repository_url: url.map(str::to_string),
            local_path_hint: None,
            revision: Revision::INITIAL,
            created_at: "now".into(),
            updated_at: "now".into(),
        }
    }

    fn grant(h: &Harness, org: &str, url: &str) {
        h.grants
            .grant(RepositoryScope {
                organization_id: OrganizationId::new(org),
                repository_url: url.into(),
                granted_at_unix_seconds: 10,
            })
            .unwrap();
    }

    #[test]
    fn permits_denies_by_default_with_no_grant() {
        let h = harness();
        match h.gate.permits(&workspace(Some(URL))).unwrap() {
            ScopePermit::Denied(reason) => assert_eq!(
                reason,
                DenialReason::RepositoryNotGranted {
                    repository_url: URL.to_string()
                }
            ),
            other => panic!("expected Denied, got {other:?}"),
        }
    }

    #[test]
    fn permits_denies_an_unbound_workspace() {
        let h = harness();
        grant(&h, "org", URL);
        match h.gate.permits(&workspace(None)).unwrap() {
            ScopePermit::Denied(reason) => assert_eq!(reason, DenialReason::WorkspaceNotBound),
            other => panic!("expected Denied, got {other:?}"),
        }
    }

    #[test]
    fn permits_allows_an_explicitly_granted_repository() {
        let h = harness();
        grant(&h, "org", URL);
        match h.gate.permits(&workspace(Some(URL))).unwrap() {
            ScopePermit::Permitted {
                organization_id,
                repository_url,
            } => {
                assert_eq!(organization_id, OrganizationId::new("org"));
                assert_eq!(repository_url, URL);
            }
            other => panic!("expected Permitted, got {other:?}"),
        }
    }

    #[test]
    fn permits_are_isolated_by_organization() {
        let h = harness();
        // A different org's grant does not permit this workspace's project org.
        grant(&h, "other", URL);
        match h.gate.permits(&workspace(Some(URL))).unwrap() {
            ScopePermit::Denied(reason) => assert_eq!(
                reason,
                DenialReason::RepositoryNotGranted {
                    repository_url: URL.to_string()
                }
            ),
            other => panic!("expected Denied, got {other:?}"),
        }
    }

    #[test]
    fn permits_errors_when_the_project_is_unknown() {
        let h = harness();
        grant(&h, "org", URL);
        let mut orphan = workspace(Some(URL));
        orphan.project_id = ProjectId::new("missing");
        assert!(matches!(
            h.gate.permits(&orphan),
            Err(WorkspaceScopeError::Scope(
                ScopeError::NotFound { .. }
            ))
        ));
    }
}
