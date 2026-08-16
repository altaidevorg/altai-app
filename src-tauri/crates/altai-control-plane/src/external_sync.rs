//! CP-08 external-object sync engine (package 070, PR 2). Drives one
//! integration's provider objects into the durable store (PR 1): fetch via
//! an injected [`ExternalObjectProvider`] — the engine itself is
//! transport-free — hash the adapter's canonical content, upsert, and
//! record what happened as `ExternalSync` activity events so a sync run is
//! auditable in the existing audit feed.
//!
//! The engine owns the three gate rules end to end: idempotency (an
//! unchanged provider payload never writes and never emits), authority
//! (the store's per-object rule decides apply vs. refuse, never write
//! order), and conflict reporting (a refused overwrite is surfaced in the
//! report and the audit feed, not swallowed).

use std::sync::Arc;

use altai_control_protocol::{
    Actor, ActivityEvent, EventKind, ExternalAuthority, ExternalObject, ExternalObjectId,
    OrganizationId,
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    ActivityEventRepository, ConflictResolution, ExternalObjectError, ExternalObjectRepository,
    ExternalSyncOutcome,
};

/// One object as the adapter mapped it from the provider's payload.
/// `content` is the canonical string the engine hashes: equal provider
/// content must produce an equal string, so adapters map — never format
/// display variants — before handing an object over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderObject {
    /// The provider's immutable id for this object (not its number).
    pub external_id: String,
    /// Provider object type (e.g. "issue", "pull_request").
    pub object_kind: String,
    pub url: Option<String>,
    pub title: String,
    /// Canonical mapped content; the engine's `content_hash` source.
    pub content: String,
    /// Provider-reported last change, if the provider reports one.
    pub external_updated_at_unix_seconds: Option<u64>,
}

/// The transport seam: a host implements one provider per integration
/// (GitHub lands in PR 3) and the engine stays free of network concerns.
pub trait ExternalObjectProvider: Send + Sync {
    fn integration(&self) -> &str;
    /// Objects changed at the provider since `since` (None = full sync).
    /// Objects within one page must carry distinct `external_id`s.
    fn fetch(&self, since: Option<u64>) -> Result<Vec<ProviderObject>, String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExternalSyncError {
    Provider { reason: String },
    Repository(ExternalObjectError),
    Activity { reason: String },
}

impl std::fmt::Display for ExternalSyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "external sync error: {self:?}")
    }
}
impl std::error::Error for ExternalSyncError {}

/// A refused overwrite: what the store holds, what the provider sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalSyncConflict {
    pub external_object_id: ExternalObjectId,
    pub stored_content_hash: String,
    pub incoming_content_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExternalSyncReport {
    pub inserted: usize,
    pub unchanged: usize,
    pub updated: usize,
    pub conflicts: Vec<ExternalSyncConflict>,
}

/// Content hash of an adapter's canonical mapped payload.
pub fn content_hash(content: &str) -> String {
    format!("{:x}", Sha256::digest(content.as_bytes()))
}

pub struct ExternalSyncService {
    organization_id: OrganizationId,
    provider: Arc<dyn ExternalObjectProvider>,
    repository: Arc<dyn ExternalObjectRepository>,
    activity: Arc<dyn ActivityEventRepository>,
    /// Authority recorded on objects this run inserts; the store's own
    /// rule decides every later run.
    default_authority: ExternalAuthority,
}

impl ExternalSyncService {
    pub fn new(
        organization_id: OrganizationId,
        provider: Arc<dyn ExternalObjectProvider>,
        repository: Arc<dyn ExternalObjectRepository>,
        activity: Arc<dyn ActivityEventRepository>,
        default_authority: ExternalAuthority,
    ) -> Self {
        Self {
            organization_id,
            provider,
            repository,
            activity,
            default_authority,
        }
    }

    /// Sync one integration. `now_unix_seconds` / `timestamp` are supplied
    /// by the caller so runs are deterministic under test. The provider is
    /// asked for changes since the newest provider-reported change already
    /// in the store — the watermark travels with the provider's clock, not
    /// ours, so an unsynced period never widens into a full refetch.
    pub fn run(
        &self,
        now_unix_seconds: u64,
        timestamp: &str,
    ) -> Result<ExternalSyncReport, ExternalSyncError> {
        let integration = self.provider.integration();
        let since = self
            .repository
            .list_by_integration(&self.organization_id, integration)
            .map_err(ExternalSyncError::Repository)?
            .iter()
            .filter_map(|object| object.external_updated_at_unix_seconds)
            .max();
        let fetched = self
            .provider
            .fetch(since)
            .map_err(|reason| ExternalSyncError::Provider { reason })?;

        let mut report = ExternalSyncReport::default();
        for provider_object in fetched {
            let object = ExternalObject {
                id: ExternalObjectId::new(Uuid::new_v4().to_string()),
                organization_id: self.organization_id.clone(),
                integration: integration.to_string(),
                // Unattributed: this engine serves single-account
                // integrations. Account-backed sync (074) carries the
                // account on every object it maps.
                account_id: None,
                external_id: provider_object.external_id.clone(),
                object_kind: provider_object.object_kind.clone(),
                url: provider_object.url.clone(),
                title: provider_object.title.clone(),
                content_hash: content_hash(&provider_object.content),
                authority: self.default_authority,
                refused_content_hash: None,
                declined_content_hash: None,
                linked_work_item_id: None,
                external_updated_at_unix_seconds: provider_object
                    .external_updated_at_unix_seconds,
                last_synced_at_unix_seconds: now_unix_seconds,
                created_at_unix_seconds: now_unix_seconds,
                updated_at_unix_seconds: now_unix_seconds,
            };
            match self
                .repository
                .upsert(object)
                .map_err(ExternalSyncError::Repository)?
            {
                ExternalSyncOutcome::Inserted => {
                    report.inserted += 1;
                    self.record(
                        timestamp,
                        integration,
                        &provider_object,
                        format!(
                            "sync inserted {} {}",
                            provider_object.object_kind, provider_object.title
                        ),
                    )?;
                }
                ExternalSyncOutcome::Unchanged => report.unchanged += 1,
                ExternalSyncOutcome::Updated => {
                    report.updated += 1;
                    self.record(
                        timestamp,
                        integration,
                        &provider_object,
                        format!(
                            "sync updated {} {}",
                            provider_object.object_kind, provider_object.title
                        ),
                    )?;
                }
                ExternalSyncOutcome::Conflict {
                    external_object_id,
                    stored_content_hash,
                    incoming_content_hash,
                } => {
                    report.conflicts.push(ExternalSyncConflict {
                        external_object_id: external_object_id.clone(),
                        stored_content_hash: stored_content_hash.clone(),
                        incoming_content_hash: incoming_content_hash.clone(),
                    });
                    self.record(
                        timestamp,
                        integration,
                        &provider_object,
                        format!(
                            "sync refused {} {}: local authority holds {}",
                            provider_object.object_kind,
                            provider_object.title,
                            external_object_id.value
                        ),
                    )?;
                }
            }
        }
        Ok(report)
    }

    fn record(
        &self,
        timestamp: &str,
        integration: &str,
        provider_object: &ProviderObject,
        summary: String,
    ) -> Result<(), ExternalSyncError> {
        self.activity
            .append(ActivityEvent {
                event_id: format!("evt_{}", Uuid::new_v4()),
                kind: EventKind::ExternalSync,
                actor: Actor::External {
                    integration: integration.to_string(),
                    external_actor_id: "sync".into(),
                },
                timestamp: timestamp.to_string(),
                organization_id: self.organization_id.clone(),
                project_id: None,
                work_item_id: None,
                attempt_id: None,
                summary,
                correlation_id: Some(provider_object.external_id.clone()),
                causation_id: None,
            })
            .map_err(|error| ExternalSyncError::Activity {
                reason: error.to_string(),
            })
    }
}

/// Apply an explicit decision to a refused overwrite and record it as an
/// `ExternalSync` activity event, so a resolution is as auditable as the
/// refusal it answers. Standalone from [`ExternalSyncService`] on purpose:
/// a resolution needs no provider — the user decides on stored state.
pub fn resolve_external_conflict(
    organization_id: &OrganizationId,
    repository: &dyn ExternalObjectRepository,
    activity: &dyn ActivityEventRepository,
    id: &ExternalObjectId,
    resolution: ConflictResolution,
    timestamp: &str,
) -> Result<ExternalObject, ExternalSyncError> {
    let resolved = repository
        .resolve_conflict(id, resolution)
        .map_err(ExternalSyncError::Repository)?;
    let summary = match resolution {
        ConflictResolution::TakeExternal => format!(
            "resolution took the external version of {} {}",
            resolved.object_kind, resolved.title
        ),
        ConflictResolution::KeepLocal => format!(
            "resolution kept local content for {} {}",
            resolved.object_kind, resolved.title
        ),
    };
    activity
        .append(ActivityEvent {
            event_id: format!("evt_{}", Uuid::new_v4()),
            kind: EventKind::ExternalSync,
            actor: Actor::System {
                component: "external-sync".into(),
            },
            timestamp: timestamp.to_string(),
            organization_id: organization_id.clone(),
            project_id: None,
            work_item_id: resolved.linked_work_item_id.clone(),
            attempt_id: None,
            summary,
            correlation_id: Some(resolved.external_id.clone()),
            causation_id: None,
        })
        .map_err(|error| ExternalSyncError::Activity {
            reason: error.to_string(),
        })?;
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SqliteActivityEventRepository, SqliteExternalObjectRepository};
    use altai_control_protocol::{ActivityQueryRequest, PageRequest};

    /// One page per fetch, in order; an exhausted provider serves nothing.
    /// Tests swap pages between runs to simulate provider-side changes.
    struct FakeProvider {
        integration: &'static str,
        pages: std::sync::Mutex<std::collections::VecDeque<Result<Vec<ProviderObject>, String>>>,
        fetched: std::sync::Mutex<Vec<Option<u64>>>,
    }

    impl FakeProvider {
        fn new(integration: &'static str, pages: Vec<Result<Vec<ProviderObject>, String>>) -> Self {
            Self {
                integration,
                pages: std::sync::Mutex::new(pages.into()),
                fetched: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn page(objects: Vec<ProviderObject>) -> Result<Vec<ProviderObject>, String> {
            Ok(objects)
        }
    }

    impl ExternalObjectProvider for FakeProvider {
        fn integration(&self) -> &str {
            self.integration
        }

        fn fetch(&self, since: Option<u64>) -> Result<Vec<ProviderObject>, String> {
            self.fetched.lock().unwrap().push(since);
            self.pages
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or(Ok(Vec::new()))
        }
    }

    fn provider_object(external_id: &str, title: &str, content: &str) -> ProviderObject {
        ProviderObject {
            external_id: external_id.into(),
            object_kind: "issue".into(),
            url: Some(format!("https://example.invalid/{external_id}")),
            title: title.into(),
            content: content.into(),
            external_updated_at_unix_seconds: Some(1_000),
        }
    }

    struct Harness {
        service: ExternalSyncService,
        repository: Arc<SqliteExternalObjectRepository>,
        activity: Arc<SqliteActivityEventRepository>,
    }

    fn harness(
        provider: Arc<FakeProvider>,
        repository: Arc<SqliteExternalObjectRepository>,
        activity: Arc<SqliteActivityEventRepository>,
    ) -> Harness {
        let service = ExternalSyncService::new(
            OrganizationId::new("org"),
            provider,
            repository.clone(),
            activity.clone(),
            ExternalAuthority::External,
        );
        Harness {
            service,
            repository,
            activity,
        }
    }

    fn stores() -> (Arc<SqliteExternalObjectRepository>, Arc<SqliteActivityEventRepository>, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let repository = Arc::new(
            SqliteExternalObjectRepository::open(&dir.path().join("work.db")).unwrap(),
        );
        let activity = Arc::new(
            SqliteActivityEventRepository::open(&dir.path().join("activity.db")).unwrap(),
        );
        (repository, activity, dir)
    }

    fn sync_events(
        activity: &SqliteActivityEventRepository,
        organization_id: &OrganizationId,
    ) -> Vec<ActivityEvent> {
        activity
            .query(&ActivityQueryRequest {
                organization_id: organization_id.clone(),
                page: PageRequest {
                    limit: 50,
                    cursor: None,
                },
                kind: Some(EventKind::ExternalSync),
                work_item_id: None,
            })
            .unwrap()
            .items
    }

    #[test]
    fn a_first_run_inserts_everything_and_records_each_insert() {
        let (repository, activity, _dir) = stores();
        let harness = harness(
            Arc::new(FakeProvider::new(
                "github",
                vec![FakeProvider::page(vec![
                    provider_object("node_1", "First issue", "content-a"),
                    provider_object("node_2", "Second issue", "content-b"),
                ])],
            )),
            repository,
            activity,
        );
        let organization_id = OrganizationId::new("org");

        let report = harness.service.run(2_000, "2026-08-15T00:00:00Z").unwrap();

        assert_eq!(report.inserted, 2);
        assert_eq!(report.unchanged, 0);
        let events = sync_events(&harness.activity, &organization_id);
        assert_eq!(events.len(), 2);
        assert!(events
            .iter()
            .all(|event| event.kind == EventKind::ExternalSync));
        assert!(events
            .iter()
            .all(|event| matches!(event.actor.clone(), Actor::External { .. })));
        assert!(events.iter().all(|event| event
            .correlation_id
            == Some("node_1".into())
            || event.correlation_id == Some("node_2".into())));
    }

    #[test]
    fn a_rerun_with_the_same_content_writes_and_records_nothing() {
        let (repository, activity, _dir) = stores();
        let provider = Arc::new(FakeProvider::new(
            "github",
            vec![
                FakeProvider::page(vec![provider_object("node_1", "First issue", "content-a")]),
                FakeProvider::page(vec![provider_object("node_1", "First issue", "content-a")]),
            ],
        ));
        let harness = harness(provider, repository, activity);

        harness.service.run(2_000, "2026-08-15T00:00:00Z").unwrap();
        let report = harness.service.run(3_000, "2026-08-15T01:00:00Z").unwrap();

        assert_eq!(report.inserted, 0);
        assert_eq!(report.unchanged, 1);
        assert_eq!(report.updated, 0);
        assert_eq!(
            sync_events(&harness.activity, &OrganizationId::new("org")).len(),
            1
        );
    }

    #[test]
    fn changed_content_applies_under_external_authority_and_is_recorded() {
        let (repository, activity, _dir) = stores();
        let harness = harness(
            Arc::new(FakeProvider::new(
                "github",
                vec![
                    FakeProvider::page(vec![provider_object("node_1", "First issue", "content-a")]),
                    FakeProvider::page(vec![provider_object(
                        "node_1",
                        "First issue renamed",
                        "content-a2",
                    )]),
                ],
            )),
            repository,
            activity,
        );

        harness.service.run(2_000, "2026-08-15T00:00:00Z").unwrap();
        let report = harness.service.run(3_000, "2026-08-15T01:00:00Z").unwrap();

        assert_eq!(report.updated, 1);
        assert_eq!(
            sync_events(&harness.activity, &OrganizationId::new("org")).len(),
            2
        );
        let stored = harness
            .repository
            .find("github", None, "node_1")
            .unwrap()
            .unwrap();
        assert_eq!(stored.title, "First issue renamed");
    }

    #[test]
    fn a_refused_overwrite_under_local_authority_is_reported_and_recorded() {
        let (repository, activity, _dir) = stores();
        // The local side declared itself authoritative for this object
        // before the provider changed it.
        let local = ExternalObject {
            id: ExternalObjectId::new("seed"),
            organization_id: OrganizationId::new("org"),
            integration: "github".into(),
            account_id: None,
            external_id: "node_1".into(),
            object_kind: "issue".into(),
            url: None,
            title: "First issue".into(),
            content_hash: content_hash("content-a"),
            authority: ExternalAuthority::Local,
            refused_content_hash: None,
            declined_content_hash: None,
            linked_work_item_id: None,
            external_updated_at_unix_seconds: Some(1_000),
            last_synced_at_unix_seconds: 1_000,
            created_at_unix_seconds: 1_000,
            updated_at_unix_seconds: 1_000,
        };
        repository.upsert(local.clone()).unwrap();
        let harness = harness(
            Arc::new(FakeProvider::new(
                "github",
                vec![FakeProvider::page(vec![provider_object(
                    "node_1",
                    "First issue renamed",
                    "content-a2",
                )])],
            )),
            repository,
            activity,
        );

        let report = harness.service.run(3_000, "2026-08-15T01:00:00Z").unwrap();

        assert_eq!(report.conflicts.len(), 1);
        assert_eq!(
            report.conflicts[0].stored_content_hash,
            content_hash("content-a")
        );
        assert_eq!(
            report.conflicts[0].incoming_content_hash,
            content_hash("content-a2")
        );
        assert_eq!(report.conflicts[0].external_object_id, local.id);
        // The refusal left the stored content untouched.
        let after = harness
            .repository
            .find("github", None, "node_1")
            .unwrap()
            .unwrap();
        assert_eq!(after.title, "First issue");
        assert_eq!(after.content_hash, content_hash("content-a"));
        // The refusal itself is one audit fact.
        let events = sync_events(&harness.activity, &OrganizationId::new("org"));
        assert_eq!(events.len(), 1);
        assert!(events[0].summary.contains("refused"));
    }

    #[test]
    fn a_keep_local_resolution_quiets_the_same_conflict_on_the_next_run() {
        let (repository, activity, _dir) = stores();
        let local = ExternalObject {
            id: ExternalObjectId::new("ext_1"),
            organization_id: OrganizationId::new("org"),
            integration: "github".into(),
            account_id: None,
            external_id: "node_1".into(),
            object_kind: "issue".into(),
            url: None,
            title: "First issue".into(),
            content_hash: content_hash("content-a"),
            authority: ExternalAuthority::Local,
            refused_content_hash: None,
            declined_content_hash: None,
            linked_work_item_id: None,
            external_updated_at_unix_seconds: Some(1_000),
            last_synced_at_unix_seconds: 1_000,
            created_at_unix_seconds: 1_000,
            updated_at_unix_seconds: 1_000,
        };
        repository.upsert(local.clone()).unwrap();
        let changed_page = || {
            Arc::new(FakeProvider::new(
                "github",
                vec![FakeProvider::page(vec![provider_object(
                    "node_1",
                    "First issue renamed",
                    "content-a2",
                )])],
            ))
        };

        let first_run = harness(changed_page(), repository.clone(), activity.clone());
        let first = first_run.service.run(3_000, "2026-08-15T01:00:00Z").unwrap();
        assert_eq!(first.conflicts.len(), 1);

        resolve_external_conflict(
            &OrganizationId::new("org"),
            repository.as_ref(),
            activity.as_ref(),
            &local.id,
            ConflictResolution::KeepLocal,
            "2026-08-15T02:00:00Z",
        )
        .unwrap();

        // The same external content no longer conflicts; nothing was written.
        let second_run = harness(changed_page(), repository.clone(), activity.clone());
        let second = second_run.service.run(4_000, "2026-08-15T03:00:00Z").unwrap();
        assert_eq!(second.conflicts.len(), 0);
        assert_eq!(second.unchanged, 1);

        // One audit fact for the refusal, one for the decision, none for the
        // quieted run.
        let events = sync_events(&activity, &OrganizationId::new("org"));
        assert_eq!(events.len(), 2);
        assert!(events
            .iter()
            .any(|event| event.summary.contains("kept local content")));
    }

    #[test]
    fn the_since_watermark_is_the_newest_provider_reported_change() {
        let (repository, activity, _dir) = stores();
        let provider = Arc::new(FakeProvider::new(
            "github",
            vec![
                FakeProvider::page(vec![
                    provider_object("node_1", "First issue", "content-a"),
                    ProviderObject {
                        external_updated_at_unix_seconds: Some(5_000),
                        ..provider_object("node_2", "Second issue", "content-b")
                    },
                ]),
                FakeProvider::page(Vec::new()),
            ],
        ));
        let harness = harness(provider.clone(), repository, activity);

        harness.service.run(2_000, "2026-08-15T00:00:00Z").unwrap();
        harness.service.run(9_000, "2026-08-15T02:00:00Z").unwrap();

        // The second run asked for changes since 5_000 — the newest
        // external_updated_at the store recorded — not since our own sync
        // time (2_000).
        assert_eq!(
            *provider.fetched.lock().unwrap(),
            vec![None, Some(5_000)]
        );
    }

    #[test]
    fn a_provider_failure_surfaces_without_writes() {
        let (repository, activity, _dir) = stores();
        let provider = Arc::new(FakeProvider::new(
            "github",
            vec![Err("github offline".into())],
        ));
        let harness = harness(provider, repository, activity);

        let error = harness.service.run(2_000, "2026-08-15T00:00:00Z").unwrap_err();

        assert_eq!(
            error,
            ExternalSyncError::Provider {
                reason: "github offline".into()
            }
        );
        assert!(harness
            .repository
            .list_by_integration(&OrganizationId::new("org"), "github")
            .unwrap()
            .is_empty());
        assert!(sync_events(&harness.activity, &OrganizationId::new("org")).is_empty());
    }
}
