//! Durable repository-scope allowlist. A [`RepositoryScope`] row is an
//! explicit org-scoped permission for one repository URL; absence of a row is
//! the denying condition. The store is insert-only: re-granting the same
//! (org, URL) pair is idempotent, and there is no revoke path yet —
//! revocation is an approval-backed governance action for a later package.

use altai_control_protocol::{OrganizationId, RepositoryScope};
use rusqlite::{params, Connection};
use std::{path::Path, sync::Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepositoryScopeError {
    Internal { reason: String },
}

impl std::fmt::Display for RepositoryScopeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Internal { reason } => write!(f, "repository scope failure: {reason}"),
        }
    }
}
impl std::error::Error for RepositoryScopeError {}

pub trait RepositoryScopeRepository: Send + Sync {
    /// Grant `repository_url` to an organization. Idempotent: re-granting the
    /// same pair succeeds without change.
    fn grant(&self, scope: RepositoryScope) -> Result<(), RepositoryScopeError>;
    /// Every URL the organization has permitted, ordered by URL.
    fn list_in_org(&self, organization_id: &OrganizationId) -> Result<Vec<RepositoryScope>, RepositoryScopeError>;
    /// Whether the organization has an explicit grant for exactly this URL.
    fn is_granted(
        &self,
        organization_id: &OrganizationId,
        repository_url: &str,
    ) -> Result<bool, RepositoryScopeError>;
}

pub struct SqliteRepositoryScopeRepository {
    connection: Mutex<Connection>,
}

impl SqliteRepositoryScopeRepository {
    pub fn open(path: &Path) -> Result<Self, String> {
        let connection = Connection::open(path).map_err(|e| e.to_string())?;
        connection
            .execute_batch(
                "PRAGMA busy_timeout = 5000; CREATE TABLE IF NOT EXISTS control_plane_repository_scopes (organization_id TEXT NOT NULL, repository_url TEXT NOT NULL, payload_json TEXT NOT NULL, PRIMARY KEY(organization_id, repository_url));",
            )
            .map_err(|e| e.to_string())?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, RepositoryScopeError> {
        self.connection.lock().map_err(|_| RepositoryScopeError::Internal {
            reason: "sqlite repository-scope lock poisoned".into(),
        })
    }
    fn db(e: rusqlite::Error) -> RepositoryScopeError {
        RepositoryScopeError::Internal { reason: e.to_string() }
    }
    fn decode(payload: String) -> Result<RepositoryScope, RepositoryScopeError> {
        serde_json::from_str(&payload).map_err(|e| RepositoryScopeError::Internal {
            reason: e.to_string(),
        })
    }
}

impl RepositoryScopeRepository for SqliteRepositoryScopeRepository {
    fn grant(&self, scope: RepositoryScope) -> Result<(), RepositoryScopeError> {
        self.lock()?
            .execute(
                "INSERT INTO control_plane_repository_scopes (organization_id, repository_url, payload_json) VALUES (?1, ?2, ?3) ON CONFLICT(organization_id, repository_url) DO NOTHING",
                params![scope.organization_id.value, scope.repository_url, serde_json::to_string(&scope).map_err(|e| RepositoryScopeError::Internal { reason: e.to_string() })?],
            )
            .map_err(Self::db)?;
        Ok(())
    }

    fn list_in_org(&self, organization_id: &OrganizationId) -> Result<Vec<RepositoryScope>, RepositoryScopeError> {
        let connection = self.lock()?;
        let mut stmt = connection
            .prepare("SELECT payload_json FROM control_plane_repository_scopes WHERE organization_id = ?1 ORDER BY repository_url")
            .map_err(Self::db)?;
        let payloads = stmt
            .query_map([&organization_id.value], |row| row.get::<_, String>(0))
            .map_err(Self::db)?;
        let mut scopes = Vec::new();
        for payload in payloads {
            scopes.push(Self::decode(payload.map_err(Self::db)?)?);
        }
        Ok(scopes)
    }

    fn is_granted(
        &self,
        organization_id: &OrganizationId,
        repository_url: &str,
    ) -> Result<bool, RepositoryScopeError> {
        Ok(self
            .lock()?
            .query_row(
                "SELECT 1 FROM control_plane_repository_scopes WHERE organization_id = ?1 AND repository_url = ?2",
                params![organization_id.value, repository_url],
                |_| Ok(()),
            )
            .map(|_| true)
            .unwrap_or(false))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope(org: &str, url: &str) -> RepositoryScope {
        RepositoryScope {
            organization_id: OrganizationId::new(org),
            repository_url: url.into(),
            granted_at_unix_seconds: 10,
        }
    }

    #[test]
    fn grants_are_durable_idempotent_and_org_scoped() {
        let dir = tempfile::tempdir().unwrap();
        let database = dir.path().join("work.db");
        let repo = SqliteRepositoryScopeRepository::open(&database).unwrap();
        repo.grant(scope("org", "https://github.com/altaidevorg/altai-app")).unwrap();
        // Re-granting the same pair is idempotent.
        repo.grant(scope("org", "https://github.com/altaidevorg/altai-app")).unwrap();
        repo.grant(scope("other", "https://github.com/altaidevorg/altai-app")).unwrap();

        let reopened = SqliteRepositoryScopeRepository::open(&database).unwrap();
        let org_scopes = reopened.list_in_org(&OrganizationId::new("org")).unwrap();
        let org_urls: Vec<&str> = org_scopes
            .iter()
            .map(|s| s.repository_url.as_str())
            .collect();
        assert_eq!(org_urls, vec!["https://github.com/altaidevorg/altai-app"]);
        assert!(reopened
            .is_granted(&OrganizationId::new("org"), "https://github.com/altaidevorg/altai-app")
            .unwrap());
        // Fail closed: another org's grant does not leak.
        assert!(!reopened
            .is_granted(&OrganizationId::new("third"), "https://github.com/altaidevorg/altai-app")
            .unwrap());
    }

    #[test]
    fn ungranted_urls_are_not_granted() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteRepositoryScopeRepository::open(&dir.path().join("work.db")).unwrap();
        repo.grant(scope("org", "https://github.com/altaidevorg/altai-app")).unwrap();

        assert!(!repo
            .is_granted(&OrganizationId::new("org"), "https://github.com/example/other")
            .unwrap());
        assert!(!repo.is_granted(&OrganizationId::new("org"), "").unwrap());
    }
}
