//! Durable append-only activity stream. An [`ActivityEvent`] is an audit
//! fact — who did what, when, and why — so the store is insert-only:
//! re-appending the same `event_id` with identical payload is a no-op, and a
//! divergent payload for a known `event_id` is a conflict, never an update.
//! Events are org-scoped and ordered by a per-store monotonic sequence that
//! doubles as the pagination cursor, so pages are stable and resumable.

use altai_control_protocol::{
    ActivityEvent, ActivityQueryRequest, EventKind, PageResponse,
};
use rusqlite::{params, Connection, OptionalExtension};
use std::{path::Path, sync::Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivityEventError {
    Conflict { event_id: String },
    Internal { reason: String },
}

impl std::fmt::Display for ActivityEventError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Conflict { event_id } => {
                write!(f, "activity event conflict: {event_id}")
            }
            Self::Internal { reason } => write!(f, "activity stream failure: {reason}"),
        }
    }
}
impl std::error::Error for ActivityEventError {}

pub trait ActivityEventRepository: Send + Sync {
    /// Append one audit fact. Idempotent per `event_id`; a divergent payload
    /// for a known `event_id` is a conflict.
    fn append(&self, event: ActivityEvent) -> Result<(), ActivityEventError>;
    /// Query the organization's stream with optional kind/work-item filters
    /// and cursor pagination.
    fn query(
        &self,
        request: &ActivityQueryRequest,
    ) -> Result<PageResponse<ActivityEvent>, ActivityEventError>;
}

pub struct SqliteActivityEventRepository {
    connection: Mutex<Connection>,
}

fn kind_value(kind: &EventKind) -> Result<String, ActivityEventError> {
    serde_json::to_value(kind)
        .map_err(|e| ActivityEventError::Internal {
            reason: e.to_string(),
        })
        .and_then(|value| {
            value.as_str().map(str::to_string).ok_or(ActivityEventError::Internal {
                reason: "event kind did not serialize to a string".into(),
            })
        })
}

impl SqliteActivityEventRepository {
    pub fn open(path: &Path) -> Result<Self, String> {
        let connection = Connection::open(path).map_err(|e| e.to_string())?;
        connection
            .execute_batch(
                "PRAGMA busy_timeout = 5000; CREATE TABLE IF NOT EXISTS control_plane_activity_events (sequence INTEGER PRIMARY KEY AUTOINCREMENT, organization_id TEXT NOT NULL, event_id TEXT NOT NULL UNIQUE, kind TEXT NOT NULL, work_item_id TEXT, payload_json TEXT NOT NULL); CREATE INDEX IF NOT EXISTS control_plane_activity_events_org_sequence ON control_plane_activity_events (organization_id, sequence);",
            )
            .map_err(|e| e.to_string())?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, ActivityEventError> {
        self.connection.lock().map_err(|_| ActivityEventError::Internal {
            reason: "sqlite activity lock poisoned".into(),
        })
    }
    fn db(e: rusqlite::Error) -> ActivityEventError {
        ActivityEventError::Internal { reason: e.to_string() }
    }
    fn decode(payload: String) -> Result<ActivityEvent, ActivityEventError> {
        serde_json::from_str(&payload).map_err(|e| ActivityEventError::Internal {
            reason: e.to_string(),
        })
    }
}

impl ActivityEventRepository for SqliteActivityEventRepository {
    fn append(&self, event: ActivityEvent) -> Result<(), ActivityEventError> {
        let payload =
            serde_json::to_string(&event).map_err(|e| ActivityEventError::Internal {
                reason: e.to_string(),
            })?;
        let connection = self.lock()?;
        connection
            .execute(
                "INSERT INTO control_plane_activity_events (organization_id, event_id, kind, work_item_id, payload_json) VALUES (?1, ?2, ?3, ?4, ?5) ON CONFLICT(event_id) DO NOTHING",
                params![
                    event.organization_id.value,
                    event.event_id,
                    kind_value(&event.kind)?,
                    event.work_item_id.as_ref().map(|id| id.value.clone()),
                    payload,
                ],
            )
            .map_err(Self::db)?;
        let stored: Option<String> = connection
            .query_row(
                "SELECT payload_json FROM control_plane_activity_events WHERE event_id = ?1",
                [&event.event_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(Self::db)?;
        match stored {
            Some(stored) if stored == payload => Ok(()),
            Some(_) => Err(ActivityEventError::Conflict {
                event_id: event.event_id,
            }),
            None => Err(ActivityEventError::Internal {
                reason: "activity event vanished after insert".into(),
            }),
        }
    }

    fn query(
        &self,
        request: &ActivityQueryRequest,
    ) -> Result<PageResponse<ActivityEvent>, ActivityEventError> {
        let cursor = request
            .page
            .cursor
            .as_deref()
            .and_then(|value| value.parse::<i64>().ok())
            .unwrap_or(0);
        let limit = request.page.effective_limit() as i64;
        let kind = match request.kind {
            Some(kind) => Some(kind_value(&kind)?),
            None => None,
        };
        let work_item = request
            .work_item_id
            .as_ref()
            .map(|id| id.value.clone());
        let connection = self.lock()?;
        let mut stmt = connection
            .prepare(
                "SELECT sequence, payload_json FROM control_plane_activity_events WHERE organization_id = ?1 AND sequence > ?2 AND (?3 IS NULL OR kind = ?3) AND (?4 IS NULL OR work_item_id = ?4) ORDER BY sequence LIMIT ?5",
            )
            .map_err(Self::db)?;
        let rows = stmt
            .query_map(
                params![
                    request.organization_id.value,
                    cursor,
                    kind,
                    work_item,
                    limit + 1,
                ],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
            )
            .map_err(Self::db)?;
        let mut items = Vec::new();
        let mut last_sequence = cursor;
        let mut has_more = false;
        for row in rows {
            let (sequence, payload) = row.map_err(Self::db)?;
            if (items.len() as i64) < limit {
                last_sequence = sequence;
                items.push(Self::decode(payload)?);
            } else {
                // The probe row exists, so a next page does too; the cursor
                // stays on the last included row so nothing is skipped.
                has_more = true;
                break;
            }
        }
        let next_cursor = if has_more {
            Some(last_sequence.to_string())
        } else {
            None
        };
        Ok(PageResponse::new(items, next_cursor, has_more, None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use altai_control_protocol::{Actor, OrganizationId, PageRequest, WorkItemId};

    fn event(org: &str, event_id: &str, kind: EventKind, work_item: Option<&str>) -> ActivityEvent {
        ActivityEvent {
            event_id: event_id.to_string(),
            kind,
            actor: Actor::System {
                component: "test".into(),
            },
            timestamp: "2026-08-15T00:00:00Z".into(),
            organization_id: OrganizationId::new(org),
            project_id: None,
            work_item_id: work_item.map(WorkItemId::new),
            attempt_id: None,
            summary: "test event".into(),
            correlation_id: None,
            causation_id: None,
        }
    }

    fn request(org: &str, cursor: Option<String>, limit: Option<u32>) -> ActivityQueryRequest {
        ActivityQueryRequest {
            organization_id: OrganizationId::new(org),
            page: PageRequest::new(cursor, limit),
            kind: None,
            work_item_id: None,
        }
    }

    #[test]
    fn appends_are_idempotent_and_conflicts_are_typed() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteActivityEventRepository::open(&dir.path().join("work.db")).unwrap();
        repo.append(event("org", "evt_1", EventKind::Created, None))
            .unwrap();
        // Identical re-append is a no-op.
        repo.append(event("org", "evt_1", EventKind::Created, None))
            .unwrap();
        // Divergent payload for a known event id conflicts.
        let divergent = event("org", "evt_1", EventKind::Updated, None);
        assert_eq!(
            repo.append(divergent),
            Err(ActivityEventError::Conflict {
                event_id: "evt_1".into()
            })
        );
        let page = repo.query(&request("org", None, None)).unwrap();
        assert_eq!(page.items.len(), 1);
    }

    #[test]
    fn queries_are_org_isolated() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteActivityEventRepository::open(&dir.path().join("work.db")).unwrap();
        repo.append(event("org", "evt_1", EventKind::Created, None))
            .unwrap();
        repo.append(event("other", "evt_2", EventKind::Created, None))
            .unwrap();
        let page = repo.query(&request("org", None, None)).unwrap();
        let items = &page.items;
        let ids: Vec<&str> = items.iter().map(|e| e.event_id.as_str()).collect();
        assert_eq!(ids, vec!["evt_1"]);
    }

    #[test]
    fn queries_filter_by_kind_and_work_item() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteActivityEventRepository::open(&dir.path().join("work.db")).unwrap();
        repo.append(event("org", "evt_1", EventKind::Created, Some("wi_1")))
            .unwrap();
        repo.append(event("org", "evt_2", EventKind::StatusChanged, Some("wi_1")))
            .unwrap();
        repo.append(event("org", "evt_3", EventKind::Created, Some("wi_2")))
            .unwrap();
        let mut by_kind = request("org", None, None);
        by_kind.kind = Some(EventKind::Created);
        let created = repo.query(&by_kind).unwrap();
        let ids: Vec<&str> = created
            .items
            .iter()
            .map(|e| e.event_id.as_str())
            .collect();
        assert_eq!(ids, vec!["evt_1", "evt_3"]);
        let mut by_work = request("org", None, None);
        by_work.work_item_id = Some(WorkItemId::new("wi_1"));
        let for_work = repo.query(&by_work).unwrap();
        let ids: Vec<&str> = for_work
            .items
            .iter()
            .map(|e| e.event_id.as_str())
            .collect();
        assert_eq!(ids, vec!["evt_1", "evt_2"]);
    }

    #[test]
    fn pagination_is_stable_and_resumable() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteActivityEventRepository::open(&dir.path().join("work.db")).unwrap();
        for index in 1..=5 {
            repo.append(event("org", &format!("evt_{index}"), EventKind::Created, None))
                .unwrap();
        }
        let first = repo.query(&request("org", None, Some(2))).unwrap();
        assert_eq!(first.items.len(), 2);
        assert!(first.has_more);
        assert!(first.next_cursor.is_some());
        let second = repo
            .query(&request("org", first.next_cursor.clone(), Some(2)))
            .unwrap();
        assert_eq!(second.items.len(), 2);
        assert!(second.has_more);
        let third = repo
            .query(&request("org", second.next_cursor.clone(), Some(2)))
            .unwrap();
        assert_eq!(third.items.len(), 1);
        assert!(!third.has_more);
        assert!(third.next_cursor.is_none());
        let all: Vec<String> = [first, second, third]
            .into_iter()
            .flat_map(|page| page.items.into_iter().map(|e| e.event_id))
            .collect();
        assert_eq!(
            all,
            vec!["evt_1", "evt_2", "evt_3", "evt_4", "evt_5"]
        );
    }
}
