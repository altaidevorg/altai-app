//! Append-only event log for [`ControlEvent`] — the event-sourced half of
//! the activity stream. Each event belongs to one aggregate
//! (`aggregate` + `aggregate_id` + per-aggregate `sequence`), so that triple
//! (scoped to one organization) is the idempotency key: re-appending it with
//! an identical payload is a no-op, and a divergent payload is a conflict,
//! never an update. A separate AUTOINCREMENT `global_sequence` is the replay
//! cursor, so client checkpoints survive interleaved appends across
//! aggregates and replay windows are stable and resumable.

use altai_control_protocol::{ControlEvent, EventReplayRequest, EventReplayResponse};
use rusqlite::{params, Connection, OptionalExtension};
use std::{path::Path, sync::Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlEventError {
    Conflict {
        aggregate: String,
        aggregate_id: String,
        sequence: u64,
    },
    Internal {
        reason: String,
    },
}

impl std::fmt::Display for ControlEventError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Conflict {
                aggregate,
                aggregate_id,
                sequence,
            } => {
                write!(
                    f,
                    "control event conflict: {aggregate}/{aggregate_id}#{sequence}"
                )
            }
            Self::Internal { reason } => {
                write!(f, "control event log failure: {reason}")
            }
        }
    }
}
impl std::error::Error for ControlEventError {}

pub trait ControlEventRepository: Send + Sync {
    /// Append one aggregate event under an organization partition.
    /// Idempotent per `(organization, aggregate, aggregate_id, sequence)`;
    /// a divergent payload for a known key is a conflict.
    fn append(
        &self,
        organization_id: &altai_control_protocol::OrganizationId,
        event: &ControlEvent,
    ) -> Result<(), ControlEventError>;
    /// Replay the organization's log from a sequence checkpoint with an
    /// optional aggregate filter.
    fn replay(
        &self,
        request: &EventReplayRequest,
    ) -> Result<EventReplayResponse, ControlEventError>;
}

pub struct SqliteControlEventRepository {
    connection: Mutex<Connection>,
}

/// Canonical string form of an aggregate ID value, used both as the
/// idempotency-key component and the fold key. `serde_json` maps iterate in
/// sorted key order, so equal values always canonicalize equally.
fn aggregate_key(aggregate_id: &serde_json::Value) -> Result<String, ControlEventError> {
    serde_json::to_string(aggregate_id).map_err(|e| ControlEventError::Internal {
        reason: e.to_string(),
    })
}

impl SqliteControlEventRepository {
    pub fn open(path: &Path) -> Result<Self, String> {
        let connection = Connection::open(path).map_err(|e| e.to_string())?;
        connection
            .execute_batch(
                "PRAGMA busy_timeout = 5000; CREATE TABLE IF NOT EXISTS control_plane_control_events (global_sequence INTEGER PRIMARY KEY AUTOINCREMENT, organization_id TEXT NOT NULL, aggregate TEXT NOT NULL, aggregate_id TEXT NOT NULL, sequence INTEGER NOT NULL, payload_json TEXT NOT NULL, UNIQUE (organization_id, aggregate, aggregate_id, sequence)); CREATE INDEX IF NOT EXISTS control_plane_control_events_org_sequence ON control_plane_control_events (organization_id, global_sequence);",
            )
            .map_err(|e| e.to_string())?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, ControlEventError> {
        self.connection.lock().map_err(|_| ControlEventError::Internal {
            reason: "sqlite control event lock poisoned".into(),
        })
    }
    fn db(e: rusqlite::Error) -> ControlEventError {
        ControlEventError::Internal { reason: e.to_string() }
    }
    fn decode(payload: String) -> Result<ControlEvent, ControlEventError> {
        serde_json::from_str(&payload).map_err(|e| ControlEventError::Internal {
            reason: e.to_string(),
        })
    }
}

impl ControlEventRepository for SqliteControlEventRepository {
    fn append(
        &self,
        organization_id: &altai_control_protocol::OrganizationId,
        event: &ControlEvent,
    ) -> Result<(), ControlEventError> {
        let payload = serde_json::to_string(event).map_err(|e| ControlEventError::Internal {
            reason: e.to_string(),
        })?;
        let aggregate_id = aggregate_key(&event.aggregate_id)?;
        let connection = self.lock()?;
        connection
            .execute(
                "INSERT INTO control_plane_control_events (organization_id, aggregate, aggregate_id, sequence, payload_json) VALUES (?1, ?2, ?3, ?4, ?5) ON CONFLICT(organization_id, aggregate, aggregate_id, sequence) DO NOTHING",
                params![
                    organization_id.value,
                    event.aggregate,
                    aggregate_id,
                    event.sequence as i64,
                    payload,
                ],
            )
            .map_err(Self::db)?;
        let stored: Option<String> = connection
            .query_row(
                "SELECT payload_json FROM control_plane_control_events WHERE organization_id = ?1 AND aggregate = ?2 AND aggregate_id = ?3 AND sequence = ?4",
                params![
                    organization_id.value,
                    event.aggregate,
                    aggregate_id,
                    event.sequence as i64,
                ],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(Self::db)?;
        match stored {
            Some(stored) if stored == payload => Ok(()),
            Some(_) => Err(ControlEventError::Conflict {
                aggregate: event.aggregate.clone(),
                aggregate_id,
                sequence: event.sequence,
            }),
            None => Err(ControlEventError::Internal {
                reason: "control event vanished after insert".into(),
            }),
        }
    }

    fn replay(
        &self,
        request: &EventReplayRequest,
    ) -> Result<EventReplayResponse, ControlEventError> {
        let cursor = request.since_sequence as i64;
        let limit = request.limit as i64;
        let connection = self.lock()?;
        let mut stmt = connection
            .prepare(
                "SELECT global_sequence, payload_json FROM control_plane_control_events WHERE organization_id = ?1 AND global_sequence > ?2 AND (?3 IS NULL OR aggregate = ?3) ORDER BY global_sequence LIMIT ?4",
            )
            .map_err(Self::db)?;
        let rows = stmt
            .query_map(
                params![
                    request.organization_id.value,
                    cursor,
                    request.aggregate,
                    limit + 1,
                ],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
            )
            .map_err(Self::db)?;
        let mut events = Vec::new();
        let mut last_sequence = cursor;
        let mut has_more = false;
        for row in rows {
            let (sequence, payload) = row.map_err(Self::db)?;
            if (events.len() as i64) < limit {
                last_sequence = sequence;
                events.push(Self::decode(payload)?);
            } else {
                // The probe row exists, so a next window does too; the
                // checkpoint stays on the last included row so nothing is
                // skipped.
                has_more = true;
                break;
            }
        }
        // The checkpoint is the last included global sequence (or the
        // incoming cursor for an empty window), so a stored checkpoint
        // resumes exactly after what the client has seen.
        Ok(EventReplayResponse {
            next_sequence: last_sequence as u64,
            events,
            has_more,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use altai_control_protocol::{
        Actor, EventKind, OrganizationId, Revision,
    };

    fn event(aggregate: &str, aggregate_id: &str, sequence: u64, summary: &str) -> ControlEvent {
        ControlEvent {
            aggregate: aggregate.to_string(),
            aggregate_id: serde_json::json!({ "value": aggregate_id }),
            sequence,
            kind: EventKind::Created,
            actor: Actor::System {
                component: "test".into(),
            },
            timestamp: "2026-08-15T00:00:00Z".into(),
            revision: Revision::new(sequence),
            payload: serde_json::json!({ "summary": summary }),
            correlation_id: None,
            causation_id: None,
        }
    }

    fn org(name: &str) -> OrganizationId {
        OrganizationId::new(name)
    }

    fn request(org_name: &str, since: u64, limit: Option<u32>) -> EventReplayRequest {
        EventReplayRequest::new(org(org_name), since, limit)
    }

    #[test]
    fn appends_are_idempotent_and_conflicts_are_typed() {
        let dir = tempfile::tempdir().unwrap();
        let repo =
            SqliteControlEventRepository::open(&dir.path().join("work.db")).unwrap();
        repo.append(&org("org"), &event("work_item", "wi_1", 1, "created"))
            .unwrap();
        // Identical re-append is a no-op.
        repo.append(&org("org"), &event("work_item", "wi_1", 1, "created"))
            .unwrap();
        // Divergent payload for a known key conflicts.
        let divergent = event("work_item", "wi_1", 1, "updated");
        assert_eq!(
            repo.append(&org("org"), &divergent),
            Err(ControlEventError::Conflict {
                aggregate: "work_item".into(),
                aggregate_id: r#"{"value":"wi_1"}"#.into(),
                sequence: 1,
            })
        );
        // The same sequence on a different aggregate or id does not collide.
        repo.append(&org("org"), &event("work_item", "wi_2", 1, "created"))
            .unwrap();
        repo.append(&org("org"), &event("attempt", "wi_1", 1, "created"))
            .unwrap();
        let response = repo.replay(&request("org", 0, None)).unwrap();
        assert_eq!(response.events.len(), 3);
    }

    #[test]
    fn replay_is_org_isolated() {
        let dir = tempfile::tempdir().unwrap();
        let repo =
            SqliteControlEventRepository::open(&dir.path().join("work.db")).unwrap();
        repo.append(&org("org"), &event("work_item", "wi_1", 1, "a"))
            .unwrap();
        repo.append(&org("other"), &event("work_item", "wi_2", 1, "b"))
            .unwrap();
        let response = repo.replay(&request("org", 0, None)).unwrap();
        let ids: Vec<String> = response
            .events
            .iter()
            .map(|e| e.aggregate_id["value"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(ids, vec!["wi_1"]);
    }

    #[test]
    fn replay_filters_by_aggregate() {
        let dir = tempfile::tempdir().unwrap();
        let repo =
            SqliteControlEventRepository::open(&dir.path().join("work.db")).unwrap();
        repo.append(&org("org"), &event("work_item", "wi_1", 1, "a"))
            .unwrap();
        repo.append(&org("org"), &event("attempt", "at_1", 1, "b"))
            .unwrap();
        repo.append(&org("org"), &event("work_item", "wi_2", 1, "c"))
            .unwrap();
        let mut filtered = request("org", 0, None);
        filtered.aggregate = Some("work_item".to_string());
        let response = repo.replay(&filtered).unwrap();
        let ids: Vec<String> = response
            .events
            .iter()
            .map(|e| e.aggregate_id["value"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(ids, vec!["wi_1", "wi_2"]);
    }

    #[test]
    fn replay_windows_are_stable_and_resumable_across_interleaves() {
        let dir = tempfile::tempdir().unwrap();
        let repo =
            SqliteControlEventRepository::open(&dir.path().join("work.db")).unwrap();
        // Interleave two aggregates: global order differs from per-aggregate
        // order, and checkpoints must still walk the global order.
        repo.append(&org("org"), &event("work_item", "wi_1", 1, "a"))
            .unwrap();
        repo.append(&org("org"), &event("attempt", "at_1", 1, "b"))
            .unwrap();
        repo.append(&org("org"), &event("work_item", "wi_1", 2, "c"))
            .unwrap();
        repo.append(&org("org"), &event("attempt", "at_1", 2, "d"))
            .unwrap();
        repo.append(&org("org"), &event("work_item", "wi_2", 1, "e"))
            .unwrap();
        let mut walked: Vec<String> = Vec::new();
        let mut since = 0u64;
        loop {
            let response = repo.replay(&request("org", since, Some(2))).unwrap();
            walked.extend(
                response
                    .events
                    .iter()
                    .map(|e| e.aggregate_id["value"].as_str().unwrap().to_string()),
            );
            since = response.next_sequence;
            if !response.has_more {
                break;
            }
        }
        assert_eq!(walked, vec!["wi_1", "at_1", "wi_1", "at_1", "wi_2"]);
    }
}
