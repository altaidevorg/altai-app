//! Gmail account commands (package 074, PR 3): connect, list,
//! disconnect, sync. Account metadata lives in the workspace's
//! `work.db`; the access token lives in platform secret storage under
//! the account's own scope ([`super::credentials`]) and never touches
//! the database. Everything SQLite opens on a blocking thread.

use super::credentials;
use super::external_sync::{GmailHttpClient, GmailProvider};
use crate::modules::secrets::SecretsState;
use crate::modules::work;
use altai_control_plane::{
    ExternalAccountRepository, ExternalSyncService, SqliteActivityEventRepository,
    SqliteExternalAccountRepository, SqliteExternalObjectRepository, SqliteScopeRepository,
};
use altai_control_protocol::{ExternalAccount, ExternalAccountId, ExternalAuthority, PluginId};
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, State};
use uuid::Uuid;

const GMAIL: &str = "gmail";

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GmailAccountDto {
    pub id: String,
    pub account_ref: String,
    pub display_name: String,
    pub created_at_unix_seconds: u64,
    pub updated_at_unix_seconds: u64,
}

impl GmailAccountDto {
    fn from_account(account: &ExternalAccount) -> Self {
        Self {
            id: account.id.value.clone(),
            account_ref: account.account_ref.clone(),
            display_name: account.display_name.clone(),
            created_at_unix_seconds: account.created_at_unix_seconds,
            updated_at_unix_seconds: account.updated_at_unix_seconds,
        }
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GmailSyncConflictDto {
    pub external_object_id: String,
    pub stored_content_hash: String,
    pub incoming_content_hash: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GmailSyncReportDto {
    pub inserted: usize,
    pub unchanged: usize,
    pub updated: usize,
    pub conflicts: Vec<GmailSyncConflictDto>,
}

fn now_unix_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0)
}

fn iso_from_unix_seconds(seconds: u64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp(seconds as i64, 0)
        .unwrap_or(chrono::DateTime::<chrono::Utc>::UNIX_EPOCH)
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// Connect (or reconnect) one Gmail account. The account row is keyed
/// by `(integration, account_ref)`: reconnecting the same address keeps
/// the stored identity and creation time; the token is stored under the
/// account's own scope, replacing any previous one.
#[tauri::command]
pub async fn gmail_connect_account(
    app: AppHandle,
    secrets: State<'_, SecretsState>,
    workspace_path: String,
    account_ref: String,
    display_name: String,
    access_token: String,
) -> Result<GmailAccountDto, String> {
    let database = work::resolve_work_db(&workspace_path)?;
    let account = tauri::async_runtime::spawn_blocking(move || {
        let scope = SqliteScopeRepository::open(&database)?;
        let organization = scope
            .ensure_default_local_organization()
            .map_err(|error| error.to_string())?;
        let accounts = SqliteExternalAccountRepository::open(&database)?;
        let now = now_unix_seconds();
        let account = ExternalAccount {
            id: ExternalAccountId::new(Uuid::new_v4().to_string()),
            organization_id: organization.id,
            integration: GMAIL.into(),
            account_ref,
            display_name,
            created_at_unix_seconds: now,
            updated_at_unix_seconds: now,
        };
        accounts.upsert(account).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("gmail connect join failed: {error}"))??;

    credentials::put_account_credential(
        &app,
        secrets.inner(),
        &PluginId::new(credentials::GMAIL_PLUGIN_ID),
        &account.id,
        credentials::ACCESS_TOKEN,
        altai_control_plane::SecretString::new(access_token),
    )?;
    Ok(GmailAccountDto::from_account(&account))
}

/// Every connected Gmail account, oldest first. Metadata only — no
/// credential is ever part of a listing.
#[tauri::command]
pub async fn gmail_accounts_list(workspace_path: String) -> Result<Vec<GmailAccountDto>, String> {
    let database = work::resolve_work_db(&workspace_path)?;
    tauri::async_runtime::spawn_blocking(move || {
        let scope = SqliteScopeRepository::open(&database)?;
        let organization = scope
            .ensure_default_local_organization()
            .map_err(|error| error.to_string())?;
        let accounts = SqliteExternalAccountRepository::open(&database)?;
        accounts
            .list_by_integration(&organization.id, GMAIL)
            .map_err(|error| error.to_string())
            .map(|list| list.iter().map(GmailAccountDto::from_account).collect())
    })
    .await
    .map_err(|error| format!("gmail accounts list join failed: {error}"))?
}

/// Disconnect one Gmail account: its credential is forgotten, scoped —
/// every other account's scope is untouched. The account row stays:
/// already-synced objects remain attributable to the account, and a
/// later reconnect reuses the same identity.
#[tauri::command]
pub async fn gmail_disconnect_account(
    app: AppHandle,
    secrets: State<'_, SecretsState>,
    workspace_path: String,
    account_id: String,
) -> Result<GmailAccountDto, String> {
    let database = work::resolve_work_db(&workspace_path)?;
    let account = tauri::async_runtime::spawn_blocking(move || {
        let accounts = SqliteExternalAccountRepository::open(&database)?;
        accounts
            .get(&ExternalAccountId::new(account_id))
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "Unknown Gmail account".to_string())
    })
    .await
    .map_err(|error| format!("gmail disconnect join failed: {error}"))??;

    credentials::remove_account_credential(
        &app,
        secrets.inner(),
        &PluginId::new(credentials::GMAIL_PLUGIN_ID),
        &account.id,
        credentials::ACCESS_TOKEN,
    )?;
    Ok(GmailAccountDto::from_account(&account))
}

/// Sync one Gmail account's threads and messages into the workspace's
/// external objects. The engine is account-scoped
/// (`ExternalSyncService::for_account`): objects carry the account, and
/// the sync window is this account's alone. Fails closed when the
/// account is unknown or its credential is gone — an unauthenticated
/// sync is never attempted.
#[tauri::command]
pub async fn gmail_sync_account(
    app: AppHandle,
    secrets: State<'_, SecretsState>,
    workspace_path: String,
    account_id: String,
) -> Result<GmailSyncReportDto, String> {
    let database = work::resolve_work_db(&workspace_path)?;
    let lookup_database = database.clone();
    let account = tauri::async_runtime::spawn_blocking(move || {
        let accounts = SqliteExternalAccountRepository::open(&lookup_database)?;
        accounts
            .get(&ExternalAccountId::new(account_id))
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "Unknown Gmail account".to_string())
    })
    .await
    .map_err(|error| format!("gmail sync join failed: {error}"))??;

    let access_token = credentials::get_account_credential(
        &app,
        secrets.inner(),
        &PluginId::new(credentials::GMAIL_PLUGIN_ID),
        &account.id,
        credentials::ACCESS_TOKEN,
    )?
    .ok_or_else(|| "Gmail account is not connected".to_string())?;

    // SQLite open, the client's runtime, and the engine run are
    // blocking work; the whole sync stays off the async workers.
    tauri::async_runtime::spawn_blocking(move || {
        let scope = SqliteScopeRepository::open(&database)?;
        let organization = scope
            .ensure_default_local_organization()
            .map_err(|error| error.to_string())?;
        let repository = Arc::new(SqliteExternalObjectRepository::open(&database)?);
        let activity = Arc::new(SqliteActivityEventRepository::open(&database)?);
        let client = GmailHttpClient::new(access_token.expose().to_string())?;
        let service = ExternalSyncService::for_account(
            account.id,
            organization.id,
            Arc::new(GmailProvider::new(client)),
            repository,
            activity,
            ExternalAuthority::External,
        );
        let now = now_unix_seconds();
        let report = service
            .run(now, &iso_from_unix_seconds(now))
            .map_err(|error| error.to_string())?;
        Ok(GmailSyncReportDto {
            inserted: report.inserted,
            unchanged: report.unchanged,
            updated: report.updated,
            conflicts: report
                .conflicts
                .iter()
                .map(|conflict| GmailSyncConflictDto {
                    external_object_id: conflict.external_object_id.value.clone(),
                    stored_content_hash: conflict.stored_content_hash.clone(),
                    incoming_content_hash: conflict.incoming_content_hash.clone(),
                })
                .collect(),
        })
    })
    .await
    .map_err(|error| format!("gmail sync join failed: {error}"))?
}
