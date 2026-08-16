//! GitHub provider for the external-object sync engine (package 070, PR 3).
//! Maps GitHub issues onto the engine's `ProviderObject`s: the issue's
//! `node_id` is the immutable provider id, and the canonical content is the
//! mapped {title, state, body} triple — equal issues must hash equal.
//! Pull-request entries ride the issues endpoint with a `pull_request`
//! marker; they are excluded here so one object kind stays one mapping.
//!
//! The engine is synchronous, so the provider owns a private Tokio runtime
//! and blocks on the desktop GitHub client inside it — callers run the
//! whole sync on a blocking thread (`spawn_blocking`), never on a runtime
//! worker.

use altai_control_plane::{
    ExternalObjectProvider, ExternalSyncService, ProviderObject, SqliteActivityEventRepository,
    SqliteExternalObjectRepository, SqliteScopeRepository,
};
use altai_control_protocol::ExternalAuthority;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, State};

use super::api;
use crate::modules::{secrets::SecretsState, work};

/// One page of the issues endpoint at the widest window the engine asks
/// for; pagination stops on the first short page.
const ISSUES_PAGE_SIZE: usize = 100;
/// Bound on pages per run: a desktop sync must terminate even against a
/// very large repository. 50 pages ≈ 5 000 issues per run.
const MAX_ISSUES_PAGES: usize = 50;

#[derive(Debug, Clone, Deserialize)]
pub struct IssueRaw {
    pub node_id: String,
    pub title: String,
    pub state: String,
    pub html_url: String,
    pub updated_at: String,
    pub body: Option<String>,
    /// Present exactly when the entry is a pull request.
    pub pull_request: Option<serde_json::Value>,
}

/// Canonical mapped content: the fields the sync treats as the issue.
/// Built through `serde_json::json!` so key order is fixed by code, which
/// makes the string — and therefore the engine's content hash — stable.
fn issue_content(raw: &IssueRaw) -> String {
    serde_json::json!({
        "title": raw.title,
        "state": raw.state,
        "body": raw.body,
    })
    .to_string()
}

/// Map one endpoint entry. Pull requests answer `None` — they are a
/// different object kind with a different mapping, not silently the same.
pub fn map_issue(raw: &IssueRaw) -> Option<ProviderObject> {
    if raw.pull_request.is_some() {
        return None;
    }
    Some(ProviderObject {
        external_id: raw.node_id.clone(),
        object_kind: "issue".into(),
        url: Some(raw.html_url.clone()),
        title: raw.title.clone(),
        content: issue_content(raw),
        external_updated_at_unix_seconds: unix_seconds_from_iso(&raw.updated_at),
    })
}

/// The `since` query value for a provider-side watermark, in the endpoint's
/// RFC 3339 shape. `None` when the store has no watermark yet (full sync).
fn since_query(since: Option<u64>) -> Option<String> {
    since.map(iso_from_unix_seconds)
}

pub fn iso_from_unix_seconds(seconds: u64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp(seconds as i64, 0)
        .unwrap_or(chrono::DateTime::<chrono::Utc>::UNIX_EPOCH)
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

pub fn unix_seconds_from_iso(value: &str) -> Option<u64> {
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|parsed| parsed.timestamp().max(0) as u64)
}

pub struct GitHubIssueProvider {
    token: String,
    owner: String,
    repo: String,
    runtime: tokio::runtime::Runtime,
}

impl GitHubIssueProvider {
    pub fn new(token: String, owner: String, repo: String) -> Result<Self, String> {
        let runtime = tokio::runtime::Runtime::new().map_err(|error| error.to_string())?;
        Ok(Self {
            token,
            owner,
            repo,
            runtime,
        })
    }

    fn fetch_page(&self, page: usize, since: Option<&str>) -> Result<Vec<IssueRaw>, String> {
        let mut path = format!(
            "/repos/{}/{}/issues?state=all&per_page={ISSUES_PAGE_SIZE}&page={page}",
            self.owner, self.repo
        );
        if let Some(since) = since {
            path.push_str(&format!("&since={since}"));
        }
        let response = self.runtime.block_on(async {
            api::request(&self.token, "GET", &path, None, false).await
        })?;
        if response.status < 200 || response.status >= 300 {
            return Err(format!(
                "GitHub issues request failed with status {}",
                response.status
            ));
        }
        let body = String::from_utf8(response.body)
            .map_err(|error| format!("GitHub issues response was not UTF-8: {error}"))?;
        serde_json::from_str::<Vec<IssueRaw>>(&body)
            .map_err(|error| format!("GitHub issues response did not decode: {error}"))
    }
}

impl ExternalObjectProvider for GitHubIssueProvider {
    fn integration(&self) -> &str {
        "github"
    }

    fn fetch(&self, since: Option<u64>) -> Result<Vec<ProviderObject>, String> {
        let since = since_query(since);
        let mut objects = Vec::new();
        for page in 1..=MAX_ISSUES_PAGES {
            let raw = self.fetch_page(page, since.as_deref())?;
            let short_page = raw.len() < ISSUES_PAGE_SIZE;
            objects.extend(raw.iter().filter_map(map_issue));
            if short_page {
                break;
            }
        }
        Ok(objects)
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExternalSyncConflictDto {
    pub external_object_id: String,
    pub stored_content_hash: String,
    pub incoming_content_hash: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExternalSyncReportDto {
    pub inserted: usize,
    pub unchanged: usize,
    pub updated: usize,
    pub conflicts: Vec<ExternalSyncConflictDto>,
}

/// Sync one GitHub repository's issues into the workspace's external
/// objects. Everything the run did — inserts, updates, refused overwrites —
/// lands in the returned report and, for the audit trail, in the
/// control-plane activity stream under an `Actor::External`.
#[tauri::command]
pub async fn external_sync_github_issues(
    app: AppHandle,
    secrets: State<'_, SecretsState>,
    workspace_path: String,
    owner: String,
    repo: String,
) -> Result<ExternalSyncReportDto, String> {
    let token = api::get_token(&app, secrets.inner())?
        .ok_or_else(|| "GitHub is not connected".to_string())?;
    let database = work::resolve_work_db(&workspace_path)?;

    // SQLite open + engine run are blocking work; the provider owns its own
    // runtime, so the whole run stays off the async workers.
    tauri::async_runtime::spawn_blocking(move || {
        let scope = SqliteScopeRepository::open(&database)?;
        let organization = scope
            .ensure_default_local_organization()
            .map_err(|error| error.to_string())?;
        let repository = Arc::new(SqliteExternalObjectRepository::open(&database)?);
        let activity = Arc::new(SqliteActivityEventRepository::open(&database)?);
        let provider = GitHubIssueProvider::new(token, owner, repo)?;
        let service = ExternalSyncService::new(
            organization.id,
            Arc::new(provider),
            repository,
            activity,
            ExternalAuthority::External,
        );
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_secs())
            .unwrap_or(0);
        let timestamp = iso_from_unix_seconds(now);
        let report = service.run(now, &timestamp).map_err(|error| error.to_string())?;
        Ok(ExternalSyncReportDto {
            inserted: report.inserted,
            unchanged: report.unchanged,
            updated: report.updated,
            conflicts: report
                .conflicts
                .iter()
                .map(|conflict| ExternalSyncConflictDto {
                    external_object_id: conflict.external_object_id.value.clone(),
                    stored_content_hash: conflict.stored_content_hash.clone(),
                    incoming_content_hash: conflict.incoming_content_hash.clone(),
                })
                .collect(),
        })
    })
    .await
    .map_err(|error| format!("external sync join failed: {error}"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn issue(node_id: &str, title: &str, state: &str) -> IssueRaw {
        IssueRaw {
            node_id: node_id.into(),
            title: title.into(),
            state: state.into(),
            html_url: "https://github.invalid/o/r/issues/7".into(),
            updated_at: "2026-08-15T00:00:00Z".into(),
            body: Some("Body".into()),
            pull_request: None,
        }
    }

    #[test]
    fn an_issue_maps_with_its_node_id_and_canonical_content() {
        let mapped = map_issue(&issue("node_1", "Fix the flux", "open")).unwrap();

        assert_eq!(mapped.external_id, "node_1");
        assert_eq!(mapped.object_kind, "issue");
        assert_eq!(mapped.title, "Fix the flux");
        assert_eq!(
            mapped.url.as_deref(),
            Some("https://github.invalid/o/r/issues/7")
        );
        assert_eq!(
            mapped.content,
            r#"{"title":"Fix the flux","state":"open","body":"Body"}"#
        );
    }

    #[test]
    fn a_missing_body_maps_to_a_null_body_not_an_absent_field() {
        let mut raw = issue("node_1", "Fix the flux", "open");
        raw.body = None;
        let mapped = map_issue(&raw).unwrap();

        assert_eq!(
            mapped.content,
            r#"{"title":"Fix the flux","state":"open","body":null}"#
        );
    }

    #[test]
    fn pull_request_entries_are_excluded_rather_than_mislabeled() {
        let mut raw = issue("node_pr", "Add the flux", "open");
        raw.pull_request = Some(serde_json::json!({"url": "…"}));

        assert_eq!(map_issue(&raw), None);
    }

    #[test]
    fn the_updated_at_watermark_round_trips_through_unix_seconds() {
        let iso = "2026-08-15T12:34:56Z";
        let unix = unix_seconds_from_iso(iso).unwrap();
        assert_eq!(iso_from_unix_seconds(unix), iso);
        assert_eq!(unix_seconds_from_iso("not a date"), None);
    }

    #[test]
    fn the_since_query_carries_the_watermark_or_nothing() {
        assert_eq!(since_query(None), None);
        assert_eq!(
            since_query(unix_seconds_from_iso("2026-08-15T12:34:56Z")),
            Some("2026-08-15T12:34:56Z".into())
        );
    }
}
