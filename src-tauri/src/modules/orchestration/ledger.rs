//! O2 — SQLite orchestration ledger.
//!
//! Persists the O1 domain model (tasks, attempts, leases, and the append-only
//! orchestration event log) using the same migration and idempotency approach
//! as the agent run journal (`crate::altai::agent::event_journal`):
//!
//! - versioned `orchestration_migrations` table, applied in an `IMMEDIATE`
//!   transaction, refusing schemas newer than supported;
//! - append-only event log with per-task sequencing and idempotent event IDs;
//! - attempts are never updated into history and never deleted (retries create
//!   a new attempt identity);
//! - duplicate idempotency keys never create duplicate work.
//!
//! O2 does **not** start the coordinator and exposes no Tauri commands. See
//! `docs/AGENT_OPERATIONS_IMPLEMENTATION_PLAN.md` §5 and §11.2 (O2).

use super::domain::{AttemptState, Lease, TaskState};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde_json::Value;
use std::fmt;
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const SCHEMA_VERSION: i64 = 1;
const MAX_EVENT_LIMIT: usize = 1_000;
const MAX_EVENT_PAYLOAD_BYTES: usize = 256 * 1024;

const MIGRATION_V1: &str = r#"
CREATE TABLE IF NOT EXISTS orchestration_tasks (
    task_id        TEXT PRIMARY KEY,
    workspace_key  TEXT NOT NULL,
    source_kind    TEXT NOT NULL,
    source_ref     TEXT NOT NULL,
    title          TEXT NOT NULL,
    state          TEXT NOT NULL,
    created_at_ms  INTEGER NOT NULL CHECK (created_at_ms >= 0),
    updated_at_ms  INTEGER NOT NULL CHECK (updated_at_ms >= 0)
) WITHOUT ROWID;

CREATE INDEX IF NOT EXISTS orchestration_tasks_workspace_state
    ON orchestration_tasks (workspace_key, state);

CREATE TABLE IF NOT EXISTS orchestration_attempts (
    attempt_id          TEXT PRIMARY KEY,
    task_id             TEXT NOT NULL REFERENCES orchestration_tasks(task_id),
    attempt_no          INTEGER NOT NULL CHECK (attempt_no > 0),
    runner_kind         TEXT NOT NULL,
    lease_owner         TEXT,
    lease_generation    INTEGER,
    lease_expires_at_ms INTEGER,
    state               TEXT NOT NULL,
    terminal_outcome    TEXT,
    idempotency_key     TEXT NOT NULL UNIQUE,
    created_at_ms       INTEGER NOT NULL CHECK (created_at_ms >= 0),
    started_at_ms       INTEGER CHECK (started_at_ms IS NULL OR started_at_ms >= 0),
    heartbeat_ms        INTEGER CHECK (heartbeat_ms IS NULL OR heartbeat_ms >= 0),
    terminal_at_ms      INTEGER CHECK (terminal_at_ms IS NULL OR terminal_at_ms >= 0),
    CHECK (
        (lease_owner IS NULL AND lease_generation IS NULL AND lease_expires_at_ms IS NULL)
        OR
        (
            lease_owner IS NOT NULL
            AND lease_generation IS NOT NULL
            AND lease_expires_at_ms IS NOT NULL
            AND length(trim(lease_owner)) > 0
            AND lease_generation >= 0
            AND lease_expires_at_ms >= 0
        )
    ),
    UNIQUE (task_id, attempt_no)
) WITHOUT ROWID;

CREATE INDEX IF NOT EXISTS orchestration_attempts_task_no
    ON orchestration_attempts (task_id, attempt_no);

CREATE TABLE IF NOT EXISTS orchestration_events (
    event_id      TEXT PRIMARY KEY,
    task_id       TEXT NOT NULL REFERENCES orchestration_tasks(task_id),
    seq           INTEGER NOT NULL CHECK (seq > 0),
    kind          TEXT NOT NULL,
    payload_json  TEXT NOT NULL CHECK (json_valid(payload_json)),
    recorded_at_ms INTEGER NOT NULL CHECK (recorded_at_ms >= 0),
    UNIQUE (task_id, seq)
) WITHOUT ROWID;

CREATE INDEX IF NOT EXISTS orchestration_events_task_seq
    ON orchestration_events (task_id, seq);
"#;

// ---------------------------------------------------------------------------
// Records
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct TaskRecord {
    pub task_id: String,
    pub workspace_key: String,
    pub source_kind: String,
    pub source_ref: String,
    pub title: String,
    pub state: TaskState,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AttemptRecord {
    pub attempt_id: String,
    pub task_id: String,
    pub attempt_no: u32,
    pub runner_kind: String,
    pub lease: Option<Lease>,
    pub state: AttemptState,
    pub terminal_outcome: Option<String>,
    pub idempotency_key: String,
    pub created_at_ms: u64,
    pub started_at_ms: Option<u64>,
    pub heartbeat_ms: Option<u64>,
    pub terminal_at_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct OrchestrationEvent {
    pub event_id: String,
    pub task_id: String,
    pub seq: u64,
    pub kind: String,
    pub payload: Value,
    pub recorded_at_ms: u64,
}

/// Input for creating an attempt. The `idempotency_key` is the deduplication
/// token (§5.4): a replay with the same key returns the existing attempt and
/// performs no new work.
#[derive(Debug, Clone)]
pub struct CreateAttemptRequest {
    pub attempt_id: String,
    pub task_id: String,
    pub attempt_no: u32,
    pub runner_kind: String,
    pub lease: Option<Lease>,
    pub idempotency_key: String,
    pub now_ms: u64,
}

// ---------------------------------------------------------------------------
// Results
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteStatus {
    Written,
    Duplicate,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AttemptOutcome {
    pub attempt_id: String,
    pub status: WriteStatus,
}

#[derive(Debug)]
pub enum LedgerError {
    Sqlite(rusqlite::Error),
    Io(std::io::Error),
    Json(serde_json::Error),
    InvalidField(&'static str),
    NumericOverflow(&'static str),
    UnsupportedSchema(i64),
    UnknownTask {
        task_id: String,
    },
    UnknownAttempt {
        attempt_id: String,
    },
    EventConflict {
        event_id: String,
    },
    TaskTerminalAlreadyCommitted {
        task_id: String,
        state: TaskState,
    },
    AttemptTerminalAlreadyCommitted {
        attempt_id: String,
        state: AttemptState,
    },
    LeaseMismatch {
        attempt_id: String,
    },
    AttemptNumberConflict {
        task_id: String,
        attempt_no: u32,
    },
    LockPoisoned,
}

impl fmt::Display for LedgerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LedgerError::Sqlite(error) => write!(f, "SQLite ledger error: {error}"),
            LedgerError::Io(error) => write!(f, "ledger I/O error: {error}"),
            LedgerError::Json(error) => write!(f, "ledger JSON error: {error}"),
            LedgerError::InvalidField(field) => write!(f, "invalid ledger field: {field}"),
            LedgerError::NumericOverflow(field) => {
                write!(f, "ledger value exceeds SQLite: {field}")
            }
            LedgerError::UnsupportedSchema(version) => {
                write!(f, "ledger schema {version} is newer than supported")
            }
            LedgerError::UnknownTask { task_id } => write!(f, "unknown task {task_id}"),
            LedgerError::UnknownAttempt { attempt_id } => {
                write!(f, "unknown attempt {attempt_id}")
            }
            LedgerError::EventConflict { event_id } => {
                write!(f, "conflicting orchestration event {event_id}")
            }
            LedgerError::TaskTerminalAlreadyCommitted { task_id, state } => write!(
                f,
                "task {task_id} already committed terminal state {}",
                state.name()
            ),
            LedgerError::AttemptTerminalAlreadyCommitted { attempt_id, state } => write!(
                f,
                "attempt {attempt_id} already committed terminal state {}",
                state.name()
            ),
            LedgerError::LeaseMismatch { attempt_id } => {
                write!(
                    f,
                    "attempt {attempt_id} lease owner or generation does not match"
                )
            }
            LedgerError::AttemptNumberConflict {
                task_id,
                attempt_no,
            } => write!(
                f,
                "attempt number {attempt_no} already exists for task {task_id}"
            ),
            LedgerError::LockPoisoned => write!(f, "ledger connection lock is poisoned"),
        }
    }
}

impl std::error::Error for LedgerError {}

impl From<rusqlite::Error> for LedgerError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Sqlite(value)
    }
}

impl From<std::io::Error> for LedgerError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for LedgerError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

pub type LedgerResult<T> = Result<T, LedgerError>;

// ---------------------------------------------------------------------------
// Ledger
// ---------------------------------------------------------------------------

/// Durable store for orchestration state. Like the agent run journal, it
/// exposes no update/delete API for events: state projections advance by
/// appending events, and attempts are immutable history.
pub struct OrchestrationLedger {
    connection: Mutex<Connection>,
}

impl OrchestrationLedger {
    pub fn open(path: impl AsRef<Path>) -> LedgerResult<Self> {
        create_private_file(path.as_ref())?;
        let connection = Connection::open(path)?;
        Self::from_connection(connection)
    }

    #[cfg(test)]
    pub(crate) fn open_in_memory() -> LedgerResult<Self> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(mut connection: Connection) -> LedgerResult<Self> {
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        migrate(&mut connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    #[cfg(test)]
    fn schema_version(&self) -> LedgerResult<i64> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| LedgerError::LockPoisoned)?;
        current_schema_version(&connection)
    }

    // ----- tasks -----------------------------------------------------------

    /// Insert or refresh task metadata. Idempotent by `task_id`: a second call
    /// updates source metadata and title but never bypasses the event-backed
    /// state transition path.
    pub fn upsert_task(&self, task: &TaskRecord) -> LedgerResult<WriteStatus> {
        validate_nonempty(&task.task_id, "task_id")?;
        validate_nonempty(&task.workspace_key, "workspace_key")?;
        validate_nonempty(&task.source_kind, "source_kind")?;
        validate_nonempty(&task.title, "title")?;
        let created_at_ms = sqlite_u64(task.created_at_ms, "created_at_ms")?;
        let updated_at_ms = sqlite_u64(task.updated_at_ms, "updated_at_ms")?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| LedgerError::LockPoisoned)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        transaction.execute(
            "INSERT INTO orchestration_tasks
                (task_id, workspace_key, source_kind, source_ref, title, state,
                 created_at_ms, updated_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(task_id) DO UPDATE SET
                workspace_key = excluded.workspace_key,
                source_kind   = excluded.source_kind,
                source_ref    = excluded.source_ref,
                title         = excluded.title,
                updated_at_ms = excluded.updated_at_ms",
            params![
                task.task_id,
                task.workspace_key,
                task.source_kind,
                task.source_ref,
                task.title,
                task.state.name(),
                created_at_ms,
                updated_at_ms,
            ],
        )?;
        transaction.commit()?;
        Ok(WriteStatus::Written)
    }

    /// Update the authoritative task state projection and append a state-change
    /// event in one transaction. The transition itself must already have been
    /// validated through the O1 domain model by the caller.
    pub fn set_task_state(
        &self,
        task_id: &str,
        new_state: TaskState,
        event_id: &str,
        now_ms: u64,
    ) -> LedgerResult<WriteStatus> {
        validate_nonempty(task_id, "task_id")?;
        validate_nonempty(event_id, "event_id")?;
        let now = sqlite_u64(now_ms, "now_ms")?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| LedgerError::LockPoisoned)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let event = OrchestrationEvent {
            event_id: event_id.to_string(),
            task_id: task_id.to_string(),
            seq: 0,
            kind: format!("task.{}", new_state.name()),
            payload: serde_json::json!({ "state": new_state.name() }),
            recorded_at_ms: now_ms,
        };
        if append_event_tx(&transaction, &event)? == WriteStatus::Duplicate {
            transaction.rollback()?;
            return Ok(WriteStatus::Duplicate);
        }
        let current_name: String = transaction
            .query_row(
                "SELECT state FROM orchestration_tasks WHERE task_id = ?1",
                params![task_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| LedgerError::UnknownTask {
                task_id: task_id.to_string(),
            })?;
        let current = TaskState::from_name(&current_name)
            .ok_or(LedgerError::InvalidField("persisted task state"))?;
        if current.is_terminal() {
            return Err(LedgerError::TaskTerminalAlreadyCommitted {
                task_id: task_id.to_string(),
                state: current,
            });
        }
        let changed = transaction.execute(
            "UPDATE orchestration_tasks SET state = ?2, updated_at_ms = ?3
             WHERE task_id = ?1
               AND state NOT IN ('done','cancelled','failed','abandoned')",
            params![task_id, new_state.name(), now],
        )?;
        if changed == 0 {
            return Err(LedgerError::TaskTerminalAlreadyCommitted {
                task_id: task_id.to_string(),
                state: current,
            });
        }
        transaction.commit()?;
        Ok(WriteStatus::Written)
    }

    pub fn task(&self, task_id: &str) -> LedgerResult<Option<TaskRecord>> {
        validate_nonempty(task_id, "task_id")?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| LedgerError::LockPoisoned)?;
        connection
            .query_row(
                "SELECT task_id, workspace_key, source_kind, source_ref, title, state,
                        created_at_ms, updated_at_ms
                 FROM orchestration_tasks WHERE task_id = ?1",
                params![task_id],
                decode_task,
            )
            .optional()
            .map_err(LedgerError::from)
    }

    /// Restart-safe query: every task that is not terminal, i.e. work the
    /// coordinator must reconcile on startup.
    pub fn active_tasks(&self, workspace_key: &str) -> LedgerResult<Vec<TaskRecord>> {
        validate_nonempty(workspace_key, "workspace_key")?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| LedgerError::LockPoisoned)?;
        let mut statement = connection.prepare(
            "SELECT task_id, workspace_key, source_kind, source_ref, title, state,
                    created_at_ms, updated_at_ms
             FROM orchestration_tasks
             WHERE workspace_key = ?1 AND state NOT IN ('done','cancelled','failed','abandoned')
             ORDER BY task_id ASC",
        )?;
        let rows = statement.query_map(params![workspace_key], decode_task)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(LedgerError::from)
    }

    // ----- attempts --------------------------------------------------------

    /// Create an attempt idempotently. A duplicate `idempotency_key` returns
    /// the existing attempt and performs no new work.
    pub fn create_attempt(&self, request: &CreateAttemptRequest) -> LedgerResult<AttemptOutcome> {
        validate_nonempty(&request.attempt_id, "attempt_id")?;
        validate_nonempty(&request.task_id, "task_id")?;
        validate_nonempty(&request.runner_kind, "runner_kind")?;
        validate_nonempty(&request.idempotency_key, "idempotency_key")?;
        let attempt_no = i64::from(request.attempt_no);
        if attempt_no == 0 {
            return Err(LedgerError::InvalidField("attempt_no"));
        }
        let created_at_ms = sqlite_u64(request.now_ms, "now_ms")?;
        let (lease_owner, lease_generation, lease_expires) = encode_lease(&request.lease)?;

        let mut connection = self
            .connection
            .lock()
            .map_err(|_| LedgerError::LockPoisoned)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if !task_exists(&transaction, &request.task_id)? {
            return Err(LedgerError::UnknownTask {
                task_id: request.task_id.clone(),
            });
        }
        if let Some(existing) = attempt_for_key(&transaction, &request.idempotency_key)? {
            transaction.rollback()?;
            return Ok(AttemptOutcome {
                attempt_id: existing,
                status: WriteStatus::Duplicate,
            });
        }
        // Guard attempt numbering independently of the idempotency key.
        if attempt_no_exists(&transaction, &request.task_id, attempt_no)? {
            return Err(LedgerError::AttemptNumberConflict {
                task_id: request.task_id.clone(),
                attempt_no: request.attempt_no,
            });
        }

        let inserted = transaction.execute(
            "INSERT INTO orchestration_attempts
                (attempt_id, task_id, attempt_no, runner_kind,
                 lease_owner, lease_generation, lease_expires_at_ms,
                 state, terminal_outcome, idempotency_key,
                 created_at_ms, started_at_ms, heartbeat_ms, terminal_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, ?9, ?10, NULL, NULL, NULL)",
            params![
                request.attempt_id,
                request.task_id,
                attempt_no,
                request.runner_kind,
                lease_owner,
                lease_generation,
                lease_expires,
                AttemptState::Created.name(),
                request.idempotency_key,
                created_at_ms,
            ],
        )?;
        debug_assert_eq!(
            inserted, 1,
            "attempt insert guarded by prior existence checks"
        );

        append_event_tx(
            &transaction,
            &OrchestrationEvent {
                event_id: format!("{}:attempt.created", request.attempt_id),
                task_id: request.task_id.clone(),
                seq: 0,
                kind: "attempt.created".to_string(),
                payload: serde_json::json!({ "attempt_id": request.attempt_id }),
                recorded_at_ms: request.now_ms,
            },
        )?;
        transaction.commit()?;
        Ok(AttemptOutcome {
            attempt_id: request.attempt_id.clone(),
            status: WriteStatus::Written,
        })
    }

    /// Update the authoritative attempt state projection. `terminal_outcome` is
    /// recorded only when the new state is terminal.
    pub fn set_attempt_state(
        &self,
        attempt_id: &str,
        new_state: AttemptState,
        terminal_outcome: Option<&str>,
        event_id: &str,
        renewed_lease: Option<&Lease>,
        now_ms: u64,
    ) -> LedgerResult<WriteStatus> {
        validate_nonempty(attempt_id, "attempt_id")?;
        validate_nonempty(event_id, "event_id")?;
        if new_state == AttemptState::Heartbeat && renewed_lease.is_none() {
            return Err(LedgerError::InvalidField("renewed_lease"));
        }
        if new_state != AttemptState::Heartbeat && renewed_lease.is_some() {
            return Err(LedgerError::InvalidField("renewed_lease"));
        }
        let now = sqlite_u64(now_ms, "now_ms")?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| LedgerError::LockPoisoned)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (task_id, current_name, lease_owner, lease_generation, lease_expires): (
            String,
            String,
            Option<String>,
            Option<i64>,
            Option<i64>,
        ) = transaction
            .query_row(
                "SELECT task_id, state, lease_owner, lease_generation, lease_expires_at_ms
                 FROM orchestration_attempts WHERE attempt_id = ?1",
                params![attempt_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| LedgerError::UnknownAttempt {
                attempt_id: attempt_id.to_string(),
            })?;
        let current = AttemptState::from_name(&current_name)
            .ok_or(LedgerError::InvalidField("persisted attempt state"))?;
        let payload = match renewed_lease {
            Some(lease) => serde_json::json!({
                "attempt_id": attempt_id,
                "state": new_state.name(),
                "lease": lease,
            }),
            None => serde_json::json!({
                "attempt_id": attempt_id,
                "state": new_state.name()
            }),
        };
        let event = OrchestrationEvent {
            event_id: event_id.to_string(),
            task_id: task_id.clone(),
            seq: 0,
            kind: format!("attempt.{}", new_state.name()),
            payload,
            recorded_at_ms: now_ms,
        };
        if append_event_tx(&transaction, &event)? == WriteStatus::Duplicate {
            transaction.rollback()?;
            return Ok(WriteStatus::Duplicate);
        }
        if current.is_terminal() {
            return Err(LedgerError::AttemptTerminalAlreadyCommitted {
                attempt_id: attempt_id.to_string(),
                state: current,
            });
        }

        let changed = if new_state.is_terminal() {
            transaction.execute(
                "UPDATE orchestration_attempts
                 SET state = ?2, terminal_outcome = ?3, terminal_at_ms = ?4
                 WHERE attempt_id = ?1 AND terminal_at_ms IS NULL",
                params![attempt_id, new_state.name(), terminal_outcome, now],
            )?
        } else if new_state == AttemptState::Started {
            transaction.execute(
                "UPDATE orchestration_attempts
                 SET state = ?2, started_at_ms = COALESCE(started_at_ms, ?3)
                 WHERE attempt_id = ?1 AND terminal_at_ms IS NULL",
                params![attempt_id, new_state.name(), now],
            )?
        } else if new_state == AttemptState::Stalled {
            transaction.execute(
                "UPDATE orchestration_attempts
                 SET state = ?2,
                     lease_owner = NULL,
                     lease_generation = NULL,
                     lease_expires_at_ms = NULL
                 WHERE attempt_id = ?1 AND terminal_at_ms IS NULL",
                params![attempt_id, new_state.name()],
            )?
        } else if let Some(lease) = renewed_lease {
            let stored_lease = decode_lease_values(
                lease_owner,
                lease_generation,
                lease_expires,
                "persisted lease",
            )?
            .ok_or_else(|| LedgerError::LeaseMismatch {
                attempt_id: attempt_id.to_string(),
            })?;
            if stored_lease.owner != lease.owner
                || stored_lease.generation != lease.generation
                || stored_lease.is_expired(now_ms)
                || lease.expires_at_ms <= now_ms
                || lease.expires_at_ms <= stored_lease.expires_at_ms
            {
                return Err(LedgerError::LeaseMismatch {
                    attempt_id: attempt_id.to_string(),
                });
            }
            let renewed_expiry = sqlite_u64(lease.expires_at_ms, "lease_expires_at_ms")?;
            transaction.execute(
                "UPDATE orchestration_attempts
                 SET state = ?2, heartbeat_ms = ?3, lease_expires_at_ms = ?4
                 WHERE attempt_id = ?1
                   AND lease_owner = ?5
                   AND lease_generation = ?6
                   AND terminal_at_ms IS NULL",
                params![
                    attempt_id,
                    new_state.name(),
                    now,
                    renewed_expiry,
                    lease.owner,
                    i64::try_from(lease.generation)
                        .map_err(|_| LedgerError::NumericOverflow("lease_generation"))?,
                ],
            )?
        } else {
            transaction.execute(
                "UPDATE orchestration_attempts
                 SET state = ?2
                 WHERE attempt_id = ?1 AND terminal_at_ms IS NULL",
                params![attempt_id, new_state.name()],
            )?
        };
        if changed == 0 {
            return Err(LedgerError::AttemptTerminalAlreadyCommitted {
                attempt_id: attempt_id.to_string(),
                state: current,
            });
        }
        transaction.commit()?;
        Ok(WriteStatus::Written)
    }

    pub fn latest_attempt(&self, task_id: &str) -> LedgerResult<Option<AttemptRecord>> {
        validate_nonempty(task_id, "task_id")?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| LedgerError::LockPoisoned)?;
        connection
            .query_row(
                "SELECT attempt_id, task_id, attempt_no, runner_kind,
                        lease_owner, lease_generation, lease_expires_at_ms,
                        state, terminal_outcome, idempotency_key,
                        created_at_ms, started_at_ms, heartbeat_ms, terminal_at_ms
                 FROM orchestration_attempts
                 WHERE task_id = ?1
                 ORDER BY attempt_no DESC LIMIT 1",
                params![task_id],
                decode_attempt,
            )
            .optional()
            .map_err(LedgerError::from)
    }

    /// Full attempt history for a task, oldest first. Retries never overwrite
    /// prior attempts.
    pub fn attempts_for_task(&self, task_id: &str) -> LedgerResult<Vec<AttemptRecord>> {
        validate_nonempty(task_id, "task_id")?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| LedgerError::LockPoisoned)?;
        let mut statement = connection.prepare(
            "SELECT attempt_id, task_id, attempt_no, runner_kind,
                    lease_owner, lease_generation, lease_expires_at_ms,
                    state, terminal_outcome, idempotency_key,
                    created_at_ms, started_at_ms, heartbeat_ms, terminal_at_ms
             FROM orchestration_attempts
             WHERE task_id = ?1
             ORDER BY attempt_no ASC",
        )?;
        let rows = statement.query_map(params![task_id], decode_attempt)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(LedgerError::from)
    }

    pub fn attempt(&self, attempt_id: &str) -> LedgerResult<Option<AttemptRecord>> {
        validate_nonempty(attempt_id, "attempt_id")?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| LedgerError::LockPoisoned)?;
        connection
            .query_row(
                "SELECT attempt_id, task_id, attempt_no, runner_kind,
                        lease_owner, lease_generation, lease_expires_at_ms,
                        state, terminal_outcome, idempotency_key,
                        created_at_ms, started_at_ms, heartbeat_ms, terminal_at_ms
                 FROM orchestration_attempts WHERE attempt_id = ?1",
                params![attempt_id],
                decode_attempt,
            )
            .optional()
            .map_err(LedgerError::from)
    }

    /// Attempts whose lease has lapsed but which are not yet terminal. The
    /// coordinator reconciles these on startup/tick (lost-lease recovery).
    pub fn expired_lease_attempts(&self, now_ms: u64) -> LedgerResult<Vec<AttemptRecord>> {
        let now = sqlite_u64(now_ms, "now_ms")?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| LedgerError::LockPoisoned)?;
        let mut statement = connection.prepare(
            "SELECT attempt_id, task_id, attempt_no, runner_kind,
                    lease_owner, lease_generation, lease_expires_at_ms,
                    state, terminal_outcome, idempotency_key,
                    created_at_ms, started_at_ms, heartbeat_ms, terminal_at_ms
             FROM orchestration_attempts
             WHERE state NOT IN ('completed','failed','cancelled','stalled')
               AND lease_expires_at_ms IS NOT NULL
               AND lease_expires_at_ms <= ?1
             ORDER BY attempt_id ASC",
        )?;
        let rows = statement.query_map(params![now], decode_attempt)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(LedgerError::from)
    }

    // ----- events ----------------------------------------------------------

    /// Append an orchestration event. Idempotent by `event_id`: a replayed
    /// event returns [`WriteStatus::Duplicate`] and consumes no sequence.
    pub fn record_event(&self, event: &OrchestrationEvent) -> LedgerResult<WriteStatus> {
        validate_nonempty(&event.event_id, "event_id")?;
        validate_nonempty(&event.task_id, "task_id")?;
        validate_nonempty(&event.kind, "kind")?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| LedgerError::LockPoisoned)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let status = append_event_tx(&transaction, event)?;
        transaction.commit()?;
        Ok(status)
    }

    pub fn events_for_task(
        &self,
        task_id: &str,
        after_seq: u64,
        limit: usize,
    ) -> LedgerResult<Vec<OrchestrationEvent>> {
        validate_nonempty(task_id, "task_id")?;
        if limit == 0 || limit > MAX_EVENT_LIMIT {
            return Err(LedgerError::InvalidField("limit"));
        }
        let after_seq = sqlite_u64(after_seq, "after_seq")?;
        let limit = i64::try_from(limit).map_err(|_| LedgerError::NumericOverflow("limit"))?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| LedgerError::LockPoisoned)?;
        let mut statement = connection.prepare(
            "SELECT event_id, task_id, seq, kind, payload_json, recorded_at_ms
             FROM orchestration_events
             WHERE task_id = ?1 AND seq > ?2
             ORDER BY seq ASC LIMIT ?3",
        )?;
        let rows = statement.query_map(params![task_id, after_seq, limit], decode_event)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(LedgerError::from)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn append_event_tx(
    transaction: &Transaction<'_>,
    event: &OrchestrationEvent,
) -> LedgerResult<WriteStatus> {
    validate_nonempty(&event.event_id, "event_id")?;
    validate_nonempty(&event.task_id, "task_id")?;
    validate_nonempty(&event.kind, "kind")?;
    let payload_json = serde_json::to_string(&event.payload)?;
    if payload_json.len() > MAX_EVENT_PAYLOAD_BYTES {
        return Err(LedgerError::InvalidField("payload"));
    }
    if let Some(existing) = find_event(transaction, &event.event_id)? {
        if existing
            == (StoredEvent {
                task_id: event.task_id.clone(),
                kind: event.kind.clone(),
                payload_json,
            })
        {
            return Ok(WriteStatus::Duplicate);
        }
        return Err(LedgerError::EventConflict {
            event_id: event.event_id.clone(),
        });
    }
    if !task_exists(transaction, &event.task_id)? {
        return Err(LedgerError::UnknownTask {
            task_id: event.task_id.clone(),
        });
    }
    let seq = next_seq_for_task(transaction, &event.task_id)?;
    let recorded_at_ms = sqlite_u64(event.recorded_at_ms, "recorded_at_ms")?;
    transaction.execute(
        "INSERT INTO orchestration_events
            (event_id, task_id, seq, kind, payload_json, recorded_at_ms)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            event.event_id,
            event.task_id,
            seq,
            event.kind,
            payload_json,
            recorded_at_ms
        ],
    )?;
    Ok(WriteStatus::Written)
}

fn next_seq_for_task(transaction: &Transaction<'_>, task_id: &str) -> LedgerResult<i64> {
    let max_seq: Option<i64> = transaction
        .query_row(
            "SELECT MAX(seq) FROM orchestration_events WHERE task_id = ?1",
            params![task_id],
            |row| row.get(0),
        )
        .optional()?
        .flatten();
    max_seq
        .unwrap_or(0)
        .checked_add(1)
        .ok_or(LedgerError::NumericOverflow("seq"))
}

fn task_exists(transaction: &Transaction<'_>, task_id: &str) -> LedgerResult<bool> {
    Ok(transaction
        .query_row(
            "SELECT 1 FROM orchestration_tasks WHERE task_id = ?1",
            params![task_id],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

#[derive(Debug, PartialEq, Eq)]
struct StoredEvent {
    task_id: String,
    kind: String,
    payload_json: String,
}

fn find_event(transaction: &Transaction<'_>, event_id: &str) -> LedgerResult<Option<StoredEvent>> {
    transaction
        .query_row(
            "SELECT task_id, kind, payload_json
             FROM orchestration_events WHERE event_id = ?1",
            params![event_id],
            |row| {
                Ok(StoredEvent {
                    task_id: row.get(0)?,
                    kind: row.get(1)?,
                    payload_json: row.get(2)?,
                })
            },
        )
        .optional()
        .map_err(LedgerError::from)
}

fn attempt_for_key(transaction: &Transaction<'_>, key: &str) -> LedgerResult<Option<String>> {
    Ok(transaction
        .query_row(
            "SELECT attempt_id FROM orchestration_attempts WHERE idempotency_key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()?)
}

fn attempt_no_exists(
    transaction: &Transaction<'_>,
    task_id: &str,
    attempt_no: i64,
) -> LedgerResult<bool> {
    Ok(transaction
        .query_row(
            "SELECT 1 FROM orchestration_attempts
             WHERE task_id = ?1 AND attempt_no = ?2",
            params![task_id, attempt_no],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

fn encode_lease(lease: &Option<Lease>) -> LedgerResult<(Option<String>, Option<i64>, Option<i64>)> {
    let Some(lease) = lease else {
        return Ok((None, None, None));
    };
    validate_nonempty(&lease.owner, "lease_owner")?;
    let generation = sqlite_u64(lease.generation, "lease_generation")?;
    let expiry = sqlite_u64(lease.expires_at_ms, "lease_expires_at_ms")?;
    Ok((Some(lease.owner.clone()), Some(generation), Some(expiry)))
}

fn decode_lease_values(
    owner: Option<String>,
    generation: Option<i64>,
    expires: Option<i64>,
    field: &'static str,
) -> LedgerResult<Option<Lease>> {
    match (owner, generation, expires) {
        (None, None, None) => Ok(None),
        (Some(owner), Some(generation), Some(expires)) if !owner.trim().is_empty() => {
            Ok(Some(Lease {
                owner,
                generation: u64::try_from(generation)
                    .map_err(|_| LedgerError::InvalidField(field))?,
                expires_at_ms: u64::try_from(expires)
                    .map_err(|_| LedgerError::InvalidField(field))?,
            }))
        }
        _ => Err(LedgerError::InvalidField(field)),
    }
}

fn from_sql_failure(
    index: usize,
    value_type: rusqlite::types::Type,
    message: String,
) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        index,
        value_type,
        std::io::Error::new(std::io::ErrorKind::InvalidData, message).into(),
    )
}

fn decode_u64(row: &rusqlite::Row<'_>, index: usize, field: &'static str) -> rusqlite::Result<u64> {
    let value = row.get::<_, i64>(index)?;
    u64::try_from(value).map_err(|_| {
        from_sql_failure(
            index,
            rusqlite::types::Type::Integer,
            format!("invalid {field}: {value}"),
        )
    })
}

fn decode_optional_u64(
    row: &rusqlite::Row<'_>,
    index: usize,
    field: &'static str,
) -> rusqlite::Result<Option<u64>> {
    row.get::<_, Option<i64>>(index)?
        .map(|value| {
            u64::try_from(value).map_err(|_| {
                from_sql_failure(
                    index,
                    rusqlite::types::Type::Integer,
                    format!("invalid {field}: {value}"),
                )
            })
        })
        .transpose()
}

fn decode_task(row: &rusqlite::Row<'_>) -> rusqlite::Result<TaskRecord> {
    let state_name: String = row.get(5)?;
    let state = TaskState::from_name(&state_name).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            5,
            rusqlite::types::Type::Text,
            format!("unknown task state `{state_name}`").into(),
        )
    })?;
    Ok(TaskRecord {
        task_id: row.get(0)?,
        workspace_key: row.get(1)?,
        source_kind: row.get(2)?,
        source_ref: row.get(3)?,
        title: row.get(4)?,
        state,
        created_at_ms: decode_u64(row, 6, "created_at_ms")?,
        updated_at_ms: decode_u64(row, 7, "updated_at_ms")?,
    })
}

fn decode_attempt(row: &rusqlite::Row<'_>) -> rusqlite::Result<AttemptRecord> {
    let state_name: String = row.get(7)?;
    let state = AttemptState::from_name(&state_name).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            7,
            rusqlite::types::Type::Text,
            format!("unknown attempt state `{state_name}`").into(),
        )
    })?;
    let lease_owner: Option<String> = row.get(4)?;
    let lease_generation: Option<i64> = row.get(5)?;
    let lease_expires: Option<i64> = row.get(6)?;
    let lease = decode_lease_values(
        lease_owner,
        lease_generation,
        lease_expires,
        "persisted lease",
    )
    .map_err(|error| from_sql_failure(4, rusqlite::types::Type::Integer, error.to_string()))?;
    let attempt_no = decode_u64(row, 2, "attempt_no").and_then(|value| {
        u32::try_from(value).map_err(|_| {
            from_sql_failure(
                2,
                rusqlite::types::Type::Integer,
                format!("invalid attempt_no: {value}"),
            )
        })
    })?;
    Ok(AttemptRecord {
        attempt_id: row.get(0)?,
        task_id: row.get(1)?,
        attempt_no,
        runner_kind: row.get(3)?,
        lease,
        state,
        terminal_outcome: row.get(8)?,
        idempotency_key: row.get(9)?,
        created_at_ms: decode_u64(row, 10, "created_at_ms")?,
        started_at_ms: decode_optional_u64(row, 11, "started_at_ms")?,
        heartbeat_ms: decode_optional_u64(row, 12, "heartbeat_ms")?,
        terminal_at_ms: decode_optional_u64(row, 13, "terminal_at_ms")?,
    })
}

fn decode_event(row: &rusqlite::Row<'_>) -> rusqlite::Result<OrchestrationEvent> {
    let payload_json: String = row.get(4)?;
    Ok(OrchestrationEvent {
        event_id: row.get(0)?,
        task_id: row.get(1)?,
        seq: decode_u64(row, 2, "seq")?,
        kind: row.get(3)?,
        payload: serde_json::from_str(&payload_json)
            .map_err(|error| from_sql_failure(4, rusqlite::types::Type::Text, error.to_string()))?,
        recorded_at_ms: decode_u64(row, 5, "recorded_at_ms")?,
    })
}

fn validate_nonempty(value: &str, field: &'static str) -> LedgerResult<()> {
    if value.trim().is_empty() {
        return Err(LedgerError::InvalidField(field));
    }
    Ok(())
}

fn sqlite_u64(value: u64, field: &'static str) -> LedgerResult<i64> {
    i64::try_from(value).map_err(|_| LedgerError::NumericOverflow(field))
}

fn migrate(connection: &mut Connection) -> LedgerResult<()> {
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    transaction.execute_batch(
        "CREATE TABLE IF NOT EXISTS orchestration_migrations (
             version       INTEGER PRIMARY KEY,
             applied_at_ms INTEGER NOT NULL CHECK (applied_at_ms >= 0)
         );",
    )?;
    let current = current_schema_version(&transaction)?;
    if current > SCHEMA_VERSION {
        return Err(LedgerError::UnsupportedSchema(current));
    }
    if current < 1 {
        let applied_at_ms = now_ms();
        transaction.execute_batch(MIGRATION_V1)?;
        transaction.execute(
            "INSERT INTO orchestration_migrations (version, applied_at_ms) VALUES (1, ?1)",
            params![applied_at_ms],
        )?;
    }
    transaction.commit()?;
    Ok(())
}

fn current_schema_version(connection: &Connection) -> LedgerResult<i64> {
    Ok(connection.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM orchestration_migrations",
        [],
        |row| row.get(0),
    )?)
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
}

fn create_private_file(path: &Path) -> LedgerResult<()> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true).write(true).create(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options
            .mode(0o600)
            .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW);
    }
    let _file = options.open(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        _file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn task(task_id: &str, state: TaskState) -> TaskRecord {
        TaskRecord {
            task_id: task_id.to_string(),
            workspace_key: "ws-1".to_string(),
            source_kind: "local".to_string(),
            source_ref: format!("local://{task_id}"),
            title: format!("Task {task_id}"),
            state,
            created_at_ms: 1_000,
            updated_at_ms: 1_000,
        }
    }

    fn attempt_req(task_id: &str, no: u32, key: &str) -> CreateAttemptRequest {
        CreateAttemptRequest {
            attempt_id: format!("{task_id}-att-{no}"),
            task_id: task_id.to_string(),
            attempt_no: no,
            runner_kind: "native".to_string(),
            lease: Some(Lease {
                owner: "coord-1".to_string(),
                generation: 1,
                expires_at_ms: 5_000,
            }),
            idempotency_key: key.to_string(),
            now_ms: 1_500,
        }
    }

    // --- Acceptance: deterministic schema + migration -----------------------

    #[test]
    fn migration_is_deterministic_and_idempotent() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("ledger.sqlite3");
        let ledger = OrchestrationLedger::open(&path).expect("open");
        assert_eq!(ledger.schema_version().expect("schema"), 1);
        drop(ledger);
        // Reopening must not re-apply or fail.
        let reopened = OrchestrationLedger::open(&path).expect("reopen");
        assert_eq!(reopened.schema_version().expect("schema"), 1);
    }

    #[test]
    fn migration_rejects_future_schema() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("future.sqlite3");
        let connection = Connection::open(&path).expect("db");
        connection
            .execute_batch(
                "CREATE TABLE orchestration_migrations (version INTEGER PRIMARY KEY, applied_at_ms INTEGER NOT NULL);
                 INSERT INTO orchestration_migrations VALUES (99, 0);",
            )
            .expect("marker");
        drop(connection);
        assert!(matches!(
            OrchestrationLedger::open(&path),
            Err(LedgerError::UnsupportedSchema(99))
        ));
    }

    // --- Acceptance: events append-only, attempts retain history ------------

    #[test]
    fn events_are_append_only_and_ordered() {
        let ledger = OrchestrationLedger::open_in_memory().expect("ledger");
        ledger
            .upsert_task(&task("t1", TaskState::Queued))
            .expect("task");
        let mk = |seq_offset: u64, id: &str| OrchestrationEvent {
            event_id: id.to_string(),
            task_id: "t1".to_string(),
            seq: 0,
            kind: "task.updated".to_string(),
            payload: serde_json::json!({ "n": seq_offset }),
            recorded_at_ms: 2_000 + seq_offset,
        };
        assert_eq!(
            ledger.record_event(&mk(1, "e1")).expect("e1"),
            WriteStatus::Written
        );
        assert_eq!(
            ledger.record_event(&mk(2, "e2")).expect("e2"),
            WriteStatus::Written
        );
        let events = ledger.events_for_task("t1", 0, 10).expect("events");
        assert_eq!(events.iter().map(|e| e.seq).collect::<Vec<_>>(), vec![1, 2]);
        assert_eq!(events[0].event_id, "e1");
        assert_eq!(events[1].payload, serde_json::json!({ "n": 2 }));
    }

    #[test]
    fn attempts_retain_history_across_retries() {
        let ledger = OrchestrationLedger::open_in_memory().expect("ledger");
        ledger
            .upsert_task(&task("t1", TaskState::Running))
            .expect("task");
        // First attempt fails terminally.
        ledger
            .create_attempt(&attempt_req("t1", 1, "k1"))
            .expect("attempt 1");
        ledger
            .set_attempt_state(
                "t1-att-1",
                AttemptState::Failed,
                Some("error"),
                "evt-attempt-1-failed",
                None,
                1_600,
            )
            .expect("fail");
        // A retry creates a NEW attempt identity; the old one is retained.
        ledger
            .create_attempt(&attempt_req("t1", 2, "k2"))
            .expect("attempt 2");

        let history = ledger.attempts_for_task("t1").expect("history");
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].attempt_no, 1);
        assert_eq!(history[0].state, AttemptState::Failed);
        assert_eq!(history[1].attempt_no, 2);
        assert_eq!(history[1].state, AttemptState::Created);
        assert_eq!(
            ledger
                .latest_attempt("t1")
                .expect("latest")
                .unwrap()
                .attempt_id,
            "t1-att-2"
        );
    }

    // --- Acceptance: duplicate idempotency keys -----------------------------

    #[test]
    fn duplicate_idempotency_key_creates_no_new_work() {
        let ledger = OrchestrationLedger::open_in_memory().expect("ledger");
        ledger
            .upsert_task(&task("t1", TaskState::Running))
            .expect("task");
        let first = ledger
            .create_attempt(&attempt_req("t1", 1, "dup-key"))
            .expect("first");
        assert_eq!(first.status, WriteStatus::Written);

        // Replay with the SAME key but a different attempt id/number: still a
        // duplicate, and no second row is created.
        let mut replay = attempt_req("t1", 2, "dup-key");
        replay.attempt_id = "should-not-exist".to_string();
        let outcome = ledger.create_attempt(&replay).expect("replay");
        assert_eq!(outcome.status, WriteStatus::Duplicate);
        assert_eq!(outcome.attempt_id, "t1-att-1");

        assert_eq!(ledger.attempts_for_task("t1").expect("history").len(), 1);
    }

    #[test]
    fn duplicate_event_id_is_idempotent() {
        let ledger = OrchestrationLedger::open_in_memory().expect("ledger");
        ledger
            .upsert_task(&task("t1", TaskState::Queued))
            .expect("task");
        let event = OrchestrationEvent {
            event_id: "evt-1".to_string(),
            task_id: "t1".to_string(),
            seq: 0,
            kind: "task.updated".to_string(),
            payload: serde_json::json!({ "v": 1 }),
            recorded_at_ms: 9_000,
        };
        assert_eq!(
            ledger.record_event(&event).expect("first"),
            WriteStatus::Written
        );
        assert_eq!(
            ledger.record_event(&event).expect("duplicate"),
            WriteStatus::Duplicate
        );
        assert_eq!(
            ledger.events_for_task("t1", 0, 10).expect("events").len(),
            1
        );
    }

    #[test]
    fn conflicting_duplicate_event_id_is_rejected() {
        let ledger = OrchestrationLedger::open_in_memory().expect("ledger");
        ledger
            .upsert_task(&task("t1", TaskState::Queued))
            .expect("task");
        let event = OrchestrationEvent {
            event_id: "evt-conflict".to_string(),
            task_id: "t1".to_string(),
            seq: 0,
            kind: "task.updated".to_string(),
            payload: serde_json::json!({ "v": 1 }),
            recorded_at_ms: 9_000,
        };
        ledger.record_event(&event).expect("first");
        let conflicting = OrchestrationEvent {
            kind: "task.blocked".to_string(),
            payload: serde_json::json!({ "v": 2 }),
            ..event
        };
        assert!(matches!(
            ledger.record_event(&conflicting),
            Err(LedgerError::EventConflict { event_id })
                if event_id == "evt-conflict"
        ));
        assert_eq!(
            ledger.events_for_task("t1", 0, 10).expect("events").len(),
            1
        );
    }

    // --- Acceptance: restart-safe queries -----------------------------------

    #[test]
    fn active_tasks_excludes_terminal_state() {
        let ledger = OrchestrationLedger::open_in_memory().expect("ledger");
        ledger
            .upsert_task(&task("active", TaskState::Running))
            .expect("t");
        ledger
            .upsert_task(&task("queued", TaskState::Queued))
            .expect("t");
        ledger
            .upsert_task(&task("done", TaskState::Done))
            .expect("t");
        ledger
            .upsert_task(&task("failed", TaskState::Failed))
            .expect("t");

        let active = ledger.active_tasks("ws-1").expect("active");
        let ids: Vec<_> = active.iter().map(|t| t.task_id.as_str()).collect();
        assert_eq!(ids, vec!["active", "queued"]);
    }

    #[test]
    fn restart_recovers_tasks_and_attempt_projection() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("restart.sqlite3");

        // First "process" writes state and drops the connection.
        {
            let ledger = OrchestrationLedger::open(&path).expect("open");
            ledger
                .upsert_task(&task("t1", TaskState::Running))
                .expect("task");
            ledger
                .create_attempt(&attempt_req("t1", 1, "k1"))
                .expect("attempt");
            ledger
                .set_task_state("t1", TaskState::Verifying, "evt-t1-verifying", 2_000)
                .expect("state");
        }
        // A fresh process reopens the same file and recovers projections.
        let recovered = OrchestrationLedger::open(&path).expect("reopen");
        let task = recovered.task("t1").expect("task").expect("present");
        assert_eq!(task.state, TaskState::Verifying);
        let attempt = recovered
            .latest_attempt("t1")
            .expect("latest")
            .expect("present");
        assert_eq!(attempt.state, AttemptState::Created);
        assert!(recovered
            .active_tasks("ws-1")
            .expect("active")
            .iter()
            .any(|t| t.task_id == "t1"));
        // Events survived restart.
        assert_eq!(
            recovered
                .events_for_task("t1", 0, 10)
                .expect("events")
                .len(),
            2
        );
    }

    // --- Round-trip & error paths ------------------------------------------

    #[test]
    fn task_state_round_trips_through_projection() {
        let ledger = OrchestrationLedger::open_in_memory().expect("ledger");
        ledger
            .upsert_task(&task("t1", TaskState::Draft))
            .expect("task");
        for (index, state) in [
            TaskState::Queued,
            TaskState::Running,
            TaskState::AwaitingApproval,
            TaskState::ReadyForHandoff,
            TaskState::Done,
        ]
        .into_iter()
        .enumerate()
        {
            ledger
                .set_task_state("t1", state, &format!("evt-state-{index}"), 3_000)
                .expect("set state");
            assert_eq!(ledger.task("t1").expect("read").unwrap().state, state);
        }
    }

    #[test]
    fn set_task_state_rejects_unknown_task() {
        let ledger = OrchestrationLedger::open_in_memory().expect("ledger");
        assert!(matches!(
            ledger.set_task_state("nope", TaskState::Running, "evt-unknown", 1),
            Err(LedgerError::UnknownTask { .. })
        ));
    }

    #[test]
    fn create_attempt_rejects_unknown_task() {
        let ledger = OrchestrationLedger::open_in_memory().expect("ledger");
        assert!(matches!(
            ledger.create_attempt(&attempt_req("ghost", 1, "k")),
            Err(LedgerError::UnknownTask { .. })
        ));
    }

    #[test]
    fn terminal_outcome_is_recorded_only_on_terminal_state() {
        let ledger = OrchestrationLedger::open_in_memory().expect("ledger");
        ledger
            .upsert_task(&task("t1", TaskState::Running))
            .expect("task");
        ledger
            .create_attempt(&attempt_req("t1", 1, "k1"))
            .expect("attempt");
        // Non-terminal move carries no outcome.
        ledger
            .set_attempt_state(
                "t1-att-1",
                AttemptState::Started,
                None,
                "evt-attempt-started",
                None,
                1_700,
            )
            .expect("start");
        let mid = ledger.latest_attempt("t1").expect("latest").unwrap();
        assert_eq!(mid.state, AttemptState::Started);
        assert_eq!(mid.started_at_ms, Some(1_700));
        assert!(mid.terminal_outcome.is_none() && mid.terminal_at_ms.is_none());
        // Terminal move records outcome + timestamp.
        ledger
            .set_attempt_state(
                "t1-att-1",
                AttemptState::Completed,
                Some("ok"),
                "evt-attempt-completed",
                None,
                1_800,
            )
            .expect("complete");
        let done = ledger.latest_attempt("t1").expect("latest").unwrap();
        assert_eq!(done.state, AttemptState::Completed);
        assert_eq!(done.terminal_outcome.as_deref(), Some("ok"));
        assert_eq!(done.terminal_at_ms, Some(1_800));
    }

    #[test]
    fn state_reentry_records_each_transition_occurrence() {
        let ledger = OrchestrationLedger::open_in_memory().expect("ledger");
        ledger
            .upsert_task(&task("t1", TaskState::Running))
            .expect("task");
        for (state, event_id) in [
            (TaskState::Retrying, "evt-retrying"),
            (TaskState::Running, "evt-running-after-retry"),
            (TaskState::Paused, "evt-paused"),
            (TaskState::Running, "evt-running-after-pause"),
        ] {
            assert_eq!(
                ledger
                    .set_task_state("t1", state, event_id, 2_000)
                    .expect("transition"),
                WriteStatus::Written
            );
        }
        let events = ledger.events_for_task("t1", 0, 10).expect("events");
        assert_eq!(events.len(), 4);
        assert_eq!(
            events
                .iter()
                .map(|event| event.event_id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "evt-retrying",
                "evt-running-after-retry",
                "evt-paused",
                "evt-running-after-pause",
            ]
        );
    }

    #[test]
    fn terminal_attempt_is_first_writer_wins_and_exact_replay_is_idempotent() {
        let ledger = OrchestrationLedger::open_in_memory().expect("ledger");
        ledger
            .upsert_task(&task("t1", TaskState::Running))
            .expect("task");
        ledger
            .create_attempt(&attempt_req("t1", 1, "k1"))
            .expect("attempt");
        assert_eq!(
            ledger
                .set_attempt_state(
                    "t1-att-1",
                    AttemptState::Completed,
                    Some("ok"),
                    "evt-terminal",
                    None,
                    2_000,
                )
                .expect("complete"),
            WriteStatus::Written
        );
        assert_eq!(
            ledger
                .set_attempt_state(
                    "t1-att-1",
                    AttemptState::Completed,
                    Some("ok"),
                    "evt-terminal",
                    None,
                    2_100,
                )
                .expect("exact replay"),
            WriteStatus::Duplicate
        );
        assert!(matches!(
            ledger.set_attempt_state(
                "t1-att-1",
                AttemptState::Failed,
                Some("late failure"),
                "evt-late-failure",
                None,
                2_200,
            ),
            Err(LedgerError::AttemptTerminalAlreadyCommitted {
                state: AttemptState::Completed,
                ..
            })
        ));
        let attempt = ledger.latest_attempt("t1").expect("latest").unwrap();
        assert_eq!(attempt.state, AttemptState::Completed);
        assert_eq!(attempt.terminal_outcome.as_deref(), Some("ok"));
        assert_eq!(attempt.terminal_at_ms, Some(2_000));
        assert_eq!(
            ledger.events_for_task("t1", 0, 10).expect("events").len(),
            2
        );
    }

    #[test]
    fn terminal_task_is_first_writer_wins() {
        let ledger = OrchestrationLedger::open_in_memory().expect("ledger");
        ledger
            .upsert_task(&task("t1", TaskState::ReadyForHandoff))
            .expect("task");
        ledger
            .set_task_state("t1", TaskState::Done, "evt-done", 2_000)
            .expect("done");
        assert!(matches!(
            ledger.set_task_state("t1", TaskState::Failed, "evt-late-failure", 2_100),
            Err(LedgerError::TaskTerminalAlreadyCommitted {
                state: TaskState::Done,
                ..
            })
        ));
        assert_eq!(
            ledger.task("t1").expect("task").unwrap().state,
            TaskState::Done
        );
        assert_eq!(
            ledger.events_for_task("t1", 0, 10).expect("events").len(),
            1
        );
    }

    #[test]
    fn heartbeat_updates_timestamp_and_renews_matching_lease() {
        let ledger = OrchestrationLedger::open_in_memory().expect("ledger");
        ledger
            .upsert_task(&task("t1", TaskState::Running))
            .expect("task");
        ledger
            .create_attempt(&attempt_req("t1", 1, "k1"))
            .expect("attempt");
        ledger
            .set_attempt_state(
                "t1-att-1",
                AttemptState::Started,
                None,
                "evt-started",
                None,
                1_600,
            )
            .expect("start");
        let renewed = Lease {
            owner: "coord-1".to_string(),
            generation: 1,
            expires_at_ms: 6_000,
        };
        ledger
            .set_attempt_state(
                "t1-att-1",
                AttemptState::Heartbeat,
                None,
                "evt-heartbeat-1",
                Some(&renewed),
                2_000,
            )
            .expect("heartbeat");
        let attempt = ledger.latest_attempt("t1").expect("latest").unwrap();
        assert_eq!(attempt.started_at_ms, Some(1_600));
        assert_eq!(attempt.heartbeat_ms, Some(2_000));
        assert_eq!(attempt.lease, Some(renewed));
    }

    #[test]
    fn heartbeat_rejects_stale_lease_generation() {
        let ledger = OrchestrationLedger::open_in_memory().expect("ledger");
        ledger
            .upsert_task(&task("t1", TaskState::Running))
            .expect("task");
        ledger
            .create_attempt(&attempt_req("t1", 1, "k1"))
            .expect("attempt");
        let stale = Lease {
            owner: "coord-1".to_string(),
            generation: 2,
            expires_at_ms: 6_000,
        };
        assert!(matches!(
            ledger.set_attempt_state(
                "t1-att-1",
                AttemptState::Heartbeat,
                None,
                "evt-stale-heartbeat",
                Some(&stale),
                2_000,
            ),
            Err(LedgerError::LeaseMismatch { .. })
        ));
        assert_eq!(
            ledger.events_for_task("t1", 0, 10).expect("events").len(),
            1
        );
    }

    #[test]
    fn lease_overflow_is_rejected_instead_of_dropped() {
        let ledger = OrchestrationLedger::open_in_memory().expect("ledger");
        ledger
            .upsert_task(&task("t1", TaskState::Running))
            .expect("task");
        let mut request = attempt_req("t1", 1, "k1");
        request.lease.as_mut().unwrap().generation = u64::MAX;
        assert!(matches!(
            ledger.create_attempt(&request),
            Err(LedgerError::NumericOverflow("lease_generation"))
        ));
        assert!(ledger.attempts_for_task("t1").expect("history").is_empty());
    }

    #[test]
    fn malformed_persisted_lease_is_reported_instead_of_clamped() {
        let ledger = OrchestrationLedger::open_in_memory().expect("ledger");
        ledger
            .upsert_task(&task("t1", TaskState::Running))
            .expect("task");
        ledger
            .create_attempt(&attempt_req("t1", 1, "k1"))
            .expect("attempt");
        {
            let connection = ledger.connection.lock().expect("connection");
            connection
                .pragma_update(None, "ignore_check_constraints", "ON")
                .expect("disable checks for corruption fixture");
            connection
                .execute(
                    "UPDATE orchestration_attempts
                     SET lease_generation = -1
                     WHERE attempt_id = 't1-att-1'",
                    [],
                )
                .expect("inject corrupt value");
            connection
                .pragma_update(None, "ignore_check_constraints", "OFF")
                .expect("restore checks");
        }
        assert!(matches!(
            ledger.latest_attempt("t1"),
            Err(LedgerError::Sqlite(
                rusqlite::Error::FromSqlConversionFailure(..)
            ))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn open_rejects_symlinked_database_path() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().expect("tempdir");
        let target = temp.path().join("target.sqlite3");
        std::fs::write(&target, b"not a database").expect("target");
        let link = temp.path().join("ledger.sqlite3");
        symlink(&target, &link).expect("symlink");

        assert!(matches!(
            OrchestrationLedger::open(&link),
            Err(LedgerError::Io(_))
        ));
        assert_eq!(
            std::fs::read(&target).expect("target remains readable"),
            b"not a database"
        );
    }
}
