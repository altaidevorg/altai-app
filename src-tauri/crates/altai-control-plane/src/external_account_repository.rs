//! CP-08 external-account storage (package 074, PR 1). One row per
//! connected provider account, keyed by `(integration, account_ref)` so
//! reconnecting the same mailbox resolves to the same row — an account's
//! local identity survives reconnects, and two accounts at one
//! integration coexist as two rows. The account is the isolation
//! boundary objects and credentials are scoped to; this table holds
//! metadata only, never credential material.

use altai_control_protocol::{ExternalAccount, ExternalAccountId, OrganizationId};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use std::{path::Path, sync::Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExternalAccountError {
    NotFound { external_account_id: String },
    /// An account reference must be the provider's stable identity, not
    /// empty or whitespace.
    InvalidAccountRef { reason: String },
    Internal { reason: String },
}

impl std::fmt::Display for ExternalAccountError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "external account error: {self:?}")
    }
}
impl std::error::Error for ExternalAccountError {}

pub trait ExternalAccountRepository: Send + Sync {
    /// Idempotent per `(integration, account_ref)`. The first upsert of
    /// an account reference mints the row's `ExternalAccountId`; later
    /// upserts of the same reference update metadata and keep it, so an
    /// account's identity survives reconnects.
    fn upsert(&self, account: ExternalAccount) -> Result<ExternalAccount, ExternalAccountError>;
    fn get(&self, id: &ExternalAccountId) -> Result<Option<ExternalAccount>, ExternalAccountError>;
    /// The account a provider identity maps to, if it has been
    /// connected. The reconnect resolution path.
    fn find(
        &self,
        integration: &str,
        account_ref: &str,
    ) -> Result<Option<ExternalAccount>, ExternalAccountError>;
    /// Every connected account for one integration, oldest first.
    fn list_by_integration(
        &self,
        organization_id: &OrganizationId,
        integration: &str,
    ) -> Result<Vec<ExternalAccount>, ExternalAccountError>;
}

pub struct SqliteExternalAccountRepository {
    connection: Mutex<Connection>,
}

impl SqliteExternalAccountRepository {
    pub fn open(path: &Path) -> Result<Self, String> {
        let connection = Connection::open(path).map_err(|e| e.to_string())?;
        connection
            .execute_batch(
                "PRAGMA busy_timeout = 5000;
                 CREATE TABLE IF NOT EXISTS control_plane_external_accounts (
                   external_account_id TEXT PRIMARY KEY,
                   organization_id TEXT NOT NULL,
                   integration TEXT NOT NULL,
                   account_ref TEXT NOT NULL,
                   payload_json TEXT NOT NULL,
                   UNIQUE(integration, account_ref)
                 );",
            )
            .map_err(|e| e.to_string())?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    fn lock(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, Connection>, ExternalAccountError> {
        self.connection
            .lock()
            .map_err(|_| ExternalAccountError::Internal {
                reason: "sqlite external account lock poisoned".into(),
            })
    }

    fn db(e: rusqlite::Error) -> ExternalAccountError {
        ExternalAccountError::Internal { reason: e.to_string() }
    }

    fn decode(payload: String) -> Result<ExternalAccount, ExternalAccountError> {
        serde_json::from_str(&payload).map_err(|e| ExternalAccountError::Internal {
            reason: format!("external account payload decode failed: {e}"),
        })
    }
}

impl ExternalAccountRepository for SqliteExternalAccountRepository {
    fn upsert(&self, account: ExternalAccount) -> Result<ExternalAccount, ExternalAccountError> {
        if account.account_ref.trim().is_empty() {
            return Err(ExternalAccountError::InvalidAccountRef {
                reason: "account_ref must be the provider's stable identity".into(),
            });
        }
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(Self::db)?;
        let stored: Option<String> = transaction
            .query_row(
                "SELECT payload_json FROM control_plane_external_accounts
                 WHERE integration = ?1 AND account_ref = ?2",
                params![account.integration, account.account_ref],
                |row| row.get(0),
            )
            .optional()
            .map_err(Self::db)?;

        let stored = match stored {
            None => account,
            Some(payload) => {
                let existing = Self::decode(payload)?;
                // The stored id stays: reconnecting the same account
                // reference must not mint a second identity.
                ExternalAccount {
                    id: existing.id,
                    created_at_unix_seconds: existing.created_at_unix_seconds,
                    ..account
                }
            }
        };
        let payload = serde_json::to_string(&stored).map_err(|e| ExternalAccountError::Internal {
            reason: e.to_string(),
        })?;
        transaction
            .execute(
                "INSERT INTO control_plane_external_accounts
                 (external_account_id, organization_id, integration, account_ref, payload_json)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(integration, account_ref) DO UPDATE SET
                   payload_json = excluded.payload_json",
                params![
                    stored.id.value,
                    stored.organization_id.value,
                    stored.integration,
                    stored.account_ref,
                    payload
                ],
            )
            .map_err(Self::db)?;
        transaction.commit().map_err(Self::db)?;
        Ok(stored)
    }

    fn get(&self, id: &ExternalAccountId) -> Result<Option<ExternalAccount>, ExternalAccountError> {
        self.lock()?
            .query_row(
                "SELECT payload_json FROM control_plane_external_accounts
                 WHERE external_account_id = ?1",
                params![id.value],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(Self::db)?
            .map(Self::decode)
            .transpose()
    }

    fn find(
        &self,
        integration: &str,
        account_ref: &str,
    ) -> Result<Option<ExternalAccount>, ExternalAccountError> {
        self.lock()?
            .query_row(
                "SELECT payload_json FROM control_plane_external_accounts
                 WHERE integration = ?1 AND account_ref = ?2",
                params![integration, account_ref],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(Self::db)?
            .map(Self::decode)
            .transpose()
    }

    fn list_by_integration(
        &self,
        organization_id: &OrganizationId,
        integration: &str,
    ) -> Result<Vec<ExternalAccount>, ExternalAccountError> {
        let connection = self.lock()?;
        let mut statement = connection
            .prepare(
                "SELECT payload_json FROM control_plane_external_accounts
                 WHERE organization_id = ?1 AND integration = ?2
                 ORDER BY json_extract(payload_json, '$.created_at_unix_seconds')",
            )
            .map_err(Self::db)?;
        let rows = statement
            .query_map(
                params![organization_id.value, integration],
                |row| row.get::<_, String>(0),
            )
            .map_err(Self::db)?;
        rows.map(|row| Self::decode(row.map_err(Self::db)?))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account(integration: &str, account_ref: &str, display: &str) -> ExternalAccount {
        ExternalAccount {
            id: ExternalAccountId::new(format!("{integration}-{account_ref}")),
            organization_id: OrganizationId::new("org"),
            integration: integration.into(),
            account_ref: account_ref.into(),
            display_name: display.into(),
            created_at_unix_seconds: 1_000,
            updated_at_unix_seconds: 1_000,
        }
    }

    /// The TempDir must outlive the repository: returning it keeps the
    /// database writable for the whole test.
    fn repository() -> (SqliteExternalAccountRepository, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let repository = SqliteExternalAccountRepository::open(&dir.path().join("work.db")).unwrap();
        (repository, dir)
    }

    #[test]
    fn connecting_the_same_account_again_keeps_its_identity() {
        let (repository, _dir) = repository();
        let first = repository
            .upsert(account("gmail", "user@example.com", "Work"))
            .unwrap();
        let mut reconnect = account("gmail", "user@example.com", "Work (renamed)");
        reconnect.id = ExternalAccountId::new("plg_minted_elsewhere");
        reconnect.updated_at_unix_seconds = 2_000;
        let second = repository.upsert(reconnect).unwrap();

        assert_eq!(second.id, first.id, "reconnect keeps the stored identity");
        assert_eq!(second.display_name, "Work (renamed)");
        assert_eq!(second.created_at_unix_seconds, 1_000);
        assert_eq!(
            repository.list_by_integration(&OrganizationId::new("org"), "gmail").unwrap(),
            vec![second],
            "one reference, one row"
        );
    }

    #[test]
    fn two_accounts_at_one_integration_coexist() {
        let (repository, _dir) = repository();
        let work = repository
            .upsert(account("gmail", "work@example.com", "Work"))
            .unwrap();
        let personal = repository
            .upsert(account("gmail", "me@example.com", "Personal"))
            .unwrap();

        assert_ne!(work.id, personal.id);
        let listed = repository
            .list_by_integration(&OrganizationId::new("org"), "gmail")
            .unwrap();
        assert_eq!(listed.len(), 2);
        assert_eq!(
            repository.find("gmail", "me@example.com").unwrap().unwrap().id,
            personal.id
        );
    }

    #[test]
    fn an_account_is_scoped_to_its_integration() {
        let (repository, _dir) = repository();
        repository
            .upsert(account("gmail", "shared@example.com", "Gmail"))
            .unwrap();
        assert!(
            repository.find("github", "shared@example.com").unwrap().is_none(),
            "the same provider identity at another integration is another account"
        );
    }

    #[test]
    fn an_empty_account_reference_is_refused() {
        let (repository, _dir) = repository();
        let result = repository.upsert(account("gmail", "  ", "Nope"));
        assert!(matches!(
            result,
            Err(ExternalAccountError::InvalidAccountRef { .. })
        ));
        assert_eq!(
            repository
                .list_by_integration(&OrganizationId::new("org"), "gmail")
                .unwrap()
                .len(),
            0
        );
    }

    #[test]
    fn a_missing_account_is_not_found_not_an_error() {
        let (repository, _dir) = repository();
        assert!(repository
            .get(&ExternalAccountId::new("exta_never"))
            .unwrap()
            .is_none());
        assert!(repository.find("gmail", "never@example.com").unwrap().is_none());
    }
}
