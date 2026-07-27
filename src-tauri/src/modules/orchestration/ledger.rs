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
use sha2::{Digest, Sha256};
use std::fmt;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const SCHEMA_VERSION: i64 = 4;
const MAX_EVENT_LIMIT: usize = 1_000;
const MAX_EVENT_PAYLOAD_BYTES: usize = 256 * 1024;
const MAX_APPROVAL_ID_CHARS: usize = 512;
const MAX_APPROVAL_DESCRIPTION_CHARS: usize = 4_096;
const MAX_APPROVAL_ACTION_BYTES: usize = 64 * 1024;
const MAX_APPROVAL_REASON_CHARS: usize = 4_096;

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

const MIGRATION_V2: &str = r#"
ALTER TABLE orchestration_tasks
ADD COLUMN description TEXT NOT NULL DEFAULT '';
"#;

const MIGRATION_V3: &str = r#"
CREATE UNIQUE INDEX IF NOT EXISTS orchestration_attempts_id_task
    ON orchestration_attempts (attempt_id, task_id);

CREATE TABLE IF NOT EXISTS orchestration_approvals (
    approval_id      TEXT PRIMARY KEY
                     CHECK (length(trim(approval_id)) BETWEEN 1 AND 512),
    task_id          TEXT NOT NULL
                     REFERENCES orchestration_tasks(task_id),
    attempt_id       TEXT NOT NULL,
    action_desc      TEXT NOT NULL
                     CHECK (length(trim(action_desc)) BETWEEN 1 AND 4096),
    action_payload_json TEXT NOT NULL
                     CHECK (json_valid(action_payload_json)
                            AND length(action_payload_json) <= 65536),
    action_hash      TEXT NOT NULL
                     CHECK (length(action_hash) = 64),
    risk_level       TEXT NOT NULL CHECK (risk_level IN ('none','medium','high')),
    policy_source    TEXT NOT NULL
                     CHECK (policy_source IN ('managed','parent','settings','workflow','profile','override','default')),
    state            TEXT NOT NULL CHECK (state IN ('pending','approved','denied','expired','auto_resolved')),
    requested_at_ms  INTEGER NOT NULL CHECK (requested_at_ms >= 0),
    expires_at_ms    INTEGER NOT NULL
                     CHECK (expires_at_ms > requested_at_ms),
    decided_at_ms    INTEGER CHECK (decided_at_ms IS NULL OR decided_at_ms >= 0),
    decided_by       TEXT CHECK (decided_by IS NULL OR decided_by IN ('human','timeout','policy')),
    decision_reason  TEXT CHECK (decision_reason IS NULL OR length(decision_reason) <= 4096),
    FOREIGN KEY (attempt_id, task_id)
        REFERENCES orchestration_attempts(attempt_id, task_id),
    CHECK (
        (state = 'pending' AND decided_at_ms IS NULL AND decided_by IS NULL)
        OR
        (state != 'pending' AND decided_at_ms >= requested_at_ms AND decided_by IS NOT NULL)
    )
) WITHOUT ROWID;

CREATE INDEX IF NOT EXISTS orchestration_approvals_task
    ON orchestration_approvals (task_id);

CREATE INDEX IF NOT EXISTS orchestration_approvals_state
    ON orchestration_approvals (state, expires_at_ms);
"#;

const MIGRATION_V4: &str = r#"
CREATE TABLE IF NOT EXISTS orchestration_artifacts (
    artifact_id    TEXT PRIMARY KEY
                   CHECK (length(trim(artifact_id)) BETWEEN 1 AND 512),
    task_id        TEXT NOT NULL
                   REFERENCES orchestration_tasks(task_id),
    attempt_id     TEXT NOT NULL,
    kind           TEXT NOT NULL
                   CHECK (kind IN ('diff','log','test_output','screenshot','metrics','summary','other')),
    checksum       TEXT NOT NULL
                   CHECK (length(checksum) = 64
                          AND checksum NOT GLOB '*[^0-9a-f]*'),
    size_bytes     INTEGER NOT NULL CHECK (size_bytes >= 0),
    producer       TEXT NOT NULL
                   CHECK (length(trim(producer)) BETWEEN 1 AND 256),
    created_at_ms  INTEGER NOT NULL CHECK (created_at_ms >= 0),
    pinned         INTEGER NOT NULL DEFAULT 0 CHECK (pinned IN (0,1)),
    description    TEXT NOT NULL DEFAULT ''
                   CHECK (length(description) <= 4096),
    FOREIGN KEY (attempt_id, task_id)
        REFERENCES orchestration_attempts(attempt_id, task_id)
) WITHOUT ROWID;

CREATE INDEX IF NOT EXISTS orchestration_artifacts_task
    ON orchestration_artifacts (task_id);

CREATE INDEX IF NOT EXISTS orchestration_artifacts_cleanup
    ON orchestration_artifacts (pinned, created_at_ms);
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
    pub description: String,
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

// ---------------------------------------------------------------------------
// Approvals (B2 — policy & approval engine)
// ---------------------------------------------------------------------------

/// Lifecycle of an approval request persisted to the ledger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalState {
    Pending,
    Approved,
    Denied,
    Expired,
    AutoResolved,
}

impl ApprovalState {
    pub fn name(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Denied => "denied",
            Self::Expired => "expired",
            Self::AutoResolved => "auto_resolved",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "pending" => Self::Pending,
            "approved" => Self::Approved,
            "denied" => Self::Denied,
            "expired" => Self::Expired,
            "auto_resolved" => Self::AutoResolved,
            _ => return None,
        })
    }

    pub fn is_terminal(self) -> bool {
        !matches!(self, Self::Pending)
    }
}

/// Who or what resolved an approval.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecidedBy {
    Human,
    Timeout,
    Policy,
}

impl ApprovalDecidedBy {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Human => "human",
            Self::Timeout => "timeout",
            Self::Policy => "policy",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "human" => Self::Human,
            "timeout" => Self::Timeout,
            "policy" => Self::Policy,
            _ => return None,
        })
    }
}

/// The durable record of an approval request and its resolution (B2 §4).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ApprovalRecord {
    pub approval_id: String,
    pub task_id: String,
    pub attempt_id: String,
    pub action_desc: String,
    pub action_payload: Value,
    pub action_hash: String,
    pub risk_level: String,
    pub policy_source: String,
    pub state: ApprovalState,
    pub requested_at_ms: u64,
    pub expires_at_ms: u64,
    pub decided_at_ms: Option<u64>,
    pub decided_by: Option<ApprovalDecidedBy>,
    pub decision_reason: Option<String>,
}

/// Input for creating an approval request.
#[derive(Debug, Clone)]
pub struct CreateApprovalRequest {
    pub approval_id: String,
    pub event_id: String,
    pub task_id: String,
    pub attempt_id: String,
    pub action_desc: String,
    pub action_payload: Value,
    pub risk_level: String,
    pub policy_source: String,
    pub expires_at_ms: u64,
    pub now_ms: u64,
}

/// Input for resolving a pending approval (first decision wins).
#[derive(Debug, Clone)]
pub struct ResolveApprovalRequest {
    pub approval_id: String,
    pub event_id: String,
    pub approved: bool,
    pub decided_by: ApprovalDecidedBy,
    pub reason: Option<String>,
    pub now_ms: u64,
}

// ---------------------------------------------------------------------------
// Artifacts (D3 — evidence store)
// ---------------------------------------------------------------------------

/// What kind of evidence an artifact represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    Diff,
    Log,
    TestOutput,
    Screenshot,
    Metrics,
    Summary,
    Other,
}

impl ArtifactKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Diff => "diff",
            Self::Log => "log",
            Self::TestOutput => "test_output",
            Self::Screenshot => "screenshot",
            Self::Metrics => "metrics",
            Self::Summary => "summary",
            Self::Other => "other",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "diff" => Self::Diff,
            "log" => Self::Log,
            "test_output" => Self::TestOutput,
            "screenshot" => Self::Screenshot,
            "metrics" => Self::Metrics,
            "summary" => Self::Summary,
            "other" => Self::Other,
            _ => return None,
        })
    }
}

/// Durable metadata for a stored artifact. The blob content lives in
/// content-addressed storage; this record holds the checksum, size, and
/// provenance.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ArtifactRecord {
    pub artifact_id: String,
    pub task_id: String,
    pub attempt_id: String,
    pub kind: ArtifactKind,
    pub checksum: String,
    pub size_bytes: u64,
    pub producer: String,
    pub created_at_ms: u64,
    pub pinned: bool,
    pub description: String,
}

/// Input for recording an artifact's metadata in the ledger.
#[derive(Debug, Clone)]
pub struct CreateArtifactRequest {
    pub artifact_id: String,
    pub task_id: String,
    pub attempt_id: String,
    pub kind: ArtifactKind,
    pub checksum: String,
    pub size_bytes: u64,
    pub producer: String,
    pub created_at_ms: u64,
    pub description: String,
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

impl WriteStatus {
    /// True when the write actually persisted (not a duplicate no-op).
    pub fn is_written(self) -> bool {
        matches!(self, WriteStatus::Written)
    }
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
    UnknownApproval {
        approval_id: String,
    },
    ApprovalAlreadyResolved {
        approval_id: String,
        state: String,
    },
    ApprovalConflict {
        approval_id: String,
    },
    ApprovalAttemptMismatch {
        task_id: String,
        attempt_id: String,
    },
    ArtifactConflict {
        artifact_id: String,
    },
    ArtifactQuotaExceeded {
        current: u64,
        attempted: u64,
        limit: u64,
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
            LedgerError::UnknownApproval { approval_id } => {
                write!(f, "unknown approval {approval_id}")
            }
            LedgerError::ApprovalAlreadyResolved { approval_id, state } => {
                write!(f, "approval {approval_id} already resolved as {state}")
            }
            LedgerError::ApprovalConflict { approval_id } => {
                write!(
                    f,
                    "approval {approval_id} conflicts with the persisted request"
                )
            }
            LedgerError::ApprovalAttemptMismatch {
                task_id,
                attempt_id,
            } => write!(
                f,
                "approval attempt {attempt_id} does not belong to task {task_id}"
            ),
            LedgerError::ArtifactConflict { artifact_id } => {
                write!(
                    f,
                    "artifact {artifact_id} conflicts with persisted metadata"
                )
            }
            LedgerError::ArtifactQuotaExceeded {
                current,
                attempted,
                limit,
            } => write!(
                f,
                "task artifact quota {current} + {attempted} exceeds limit {limit}"
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
    artifact_operations: Mutex<()>,
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
            artifact_operations: Mutex::new(()),
        })
    }

    /// Serialize blob + metadata operations that cannot share one SQLite
    /// transaction. This prevents cleanup from deleting a checksum while a
    /// concurrent store is adding a new reference to the same blob.
    pub(crate) fn lock_artifact_operation(&self) -> LedgerResult<MutexGuard<'_, ()>> {
        self.artifact_operations
            .lock()
            .map_err(|_| LedgerError::LockPoisoned)
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
    /// updates source metadata, title, and description but never bypasses the event-backed
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
                (task_id, workspace_key, source_kind, source_ref, title, description, state,
                 created_at_ms, updated_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(task_id) DO UPDATE SET
                workspace_key = excluded.workspace_key,
                source_kind   = excluded.source_kind,
                source_ref    = excluded.source_ref,
                title         = excluded.title,
                description   = excluded.description,
                updated_at_ms = excluded.updated_at_ms",
            params![
                task.task_id,
                task.workspace_key,
                task.source_kind,
                task.source_ref,
                task.title,
                task.description,
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
                "SELECT task_id, workspace_key, source_kind, source_ref, title, description, state,
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
            "SELECT task_id, workspace_key, source_kind, source_ref, title, description, state,
                    created_at_ms, updated_at_ms
             FROM orchestration_tasks
             WHERE workspace_key = ?1 AND state NOT IN ('done','cancelled','failed','abandoned')
             ORDER BY task_id ASC",
        )?;
        let rows = statement.query_map(params![workspace_key], decode_task)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(LedgerError::from)
    }

    /// Every task for a workspace, including terminal ones. Used by projections
    /// to build workspace-scoped metrics and read-models.
    pub fn tasks_for_workspace(&self, workspace_key: &str) -> LedgerResult<Vec<TaskRecord>> {
        validate_nonempty(workspace_key, "workspace_key")?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| LedgerError::LockPoisoned)?;
        let mut statement = connection.prepare(
            "SELECT task_id, workspace_key, source_kind, source_ref, title, description, state,
                    created_at_ms, updated_at_ms
             FROM orchestration_tasks
             WHERE workspace_key = ?1
             ORDER BY task_id ASC",
        )?;
        let rows = statement.query_map(params![workspace_key], decode_task)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(LedgerError::from)
    }

    /// All non-terminal tasks across every workspace. Used by startup recovery
    /// (O5) to reconcile orphaned or unresolved tasks without guessing.
    pub fn non_terminal_tasks(&self) -> LedgerResult<Vec<TaskRecord>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| LedgerError::LockPoisoned)?;
        let mut statement = connection.prepare(
            "SELECT task_id, workspace_key, source_kind, source_ref, title, description, state,
                    created_at_ms, updated_at_ms
             FROM orchestration_tasks
             WHERE state NOT IN ('done','cancelled','failed','abandoned')
             ORDER BY task_id ASC",
        )?;
        let rows = statement.query_map([], decode_task)?;
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

    /// Non-terminal attempts with expired leases, restricted to one workspace.
    pub fn expired_lease_attempts_for_workspace(
        &self,
        workspace_key: &str,
        now_ms: u64,
    ) -> LedgerResult<Vec<AttemptRecord>> {
        validate_nonempty(workspace_key, "workspace_key")?;
        let now = sqlite_u64(now_ms, "now_ms")?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| LedgerError::LockPoisoned)?;
        let mut statement = connection.prepare(
            "SELECT attempts.attempt_id, attempts.task_id, attempts.attempt_no,
                    attempts.runner_kind, attempts.lease_owner,
                    attempts.lease_generation, attempts.lease_expires_at_ms,
                    attempts.state, attempts.terminal_outcome,
                    attempts.idempotency_key, attempts.created_at_ms,
                    attempts.started_at_ms, attempts.heartbeat_ms,
                    attempts.terminal_at_ms
             FROM orchestration_attempts AS attempts
             INNER JOIN orchestration_tasks AS tasks
                     ON tasks.task_id = attempts.task_id
             WHERE tasks.workspace_key = ?1
               AND attempts.state NOT IN ('completed','failed','cancelled','stalled')
               AND attempts.lease_expires_at_ms IS NOT NULL
               AND attempts.lease_expires_at_ms <= ?2
             ORDER BY attempts.attempt_id ASC",
        )?;
        let rows = statement.query_map(params![workspace_key, now], decode_attempt)?;
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

    // ----- approvals -------------------------------------------------------

    /// Create a pending approval request. Returns `Duplicate` if an approval
    /// with the same id already exists (idempotent replay).
    pub fn create_approval_request(
        &self,
        request: &CreateApprovalRequest,
    ) -> LedgerResult<WriteStatus> {
        validate_bounded(&request.approval_id, "approval_id", MAX_APPROVAL_ID_CHARS)?;
        validate_nonempty(&request.event_id, "event_id")?;
        validate_nonempty(&request.task_id, "task_id")?;
        validate_nonempty(&request.attempt_id, "attempt_id")?;
        validate_bounded(
            &request.action_desc,
            "action_desc",
            MAX_APPROVAL_DESCRIPTION_CHARS,
        )?;
        if !matches!(request.risk_level.as_str(), "none" | "medium" | "high") {
            return Err(LedgerError::InvalidField("risk_level"));
        }
        if !matches!(
            request.policy_source.as_str(),
            "managed" | "parent" | "settings" | "workflow" | "profile" | "override" | "default"
        ) {
            return Err(LedgerError::InvalidField("policy_source"));
        }
        let now = sqlite_u64(request.now_ms, "now_ms")?;
        if request.expires_at_ms <= request.now_ms {
            return Err(LedgerError::InvalidField("expires_at_ms"));
        }
        let expires = sqlite_u64(request.expires_at_ms, "expires_at_ms")?;
        let action_payload_json = canonical_approval_action(&request.action_payload)?;
        let action_hash = approval_action_hash_from_json(&action_payload_json);

        let mut connection = self
            .connection
            .lock()
            .map_err(|_| LedgerError::LockPoisoned)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(existing) = find_approval_tx(&transaction, &request.approval_id)? {
            if approval_matches_request(&existing, request, &action_hash, request.expires_at_ms) {
                transaction.rollback()?;
                return Ok(WriteStatus::Duplicate);
            }
            return Err(LedgerError::ApprovalConflict {
                approval_id: request.approval_id.clone(),
            });
        }
        if !task_exists(&transaction, &request.task_id)? {
            return Err(LedgerError::UnknownTask {
                task_id: request.task_id.clone(),
            });
        }
        if !attempt_belongs_to_task(&transaction, &request.attempt_id, &request.task_id)? {
            return Err(LedgerError::ApprovalAttemptMismatch {
                task_id: request.task_id.clone(),
                attempt_id: request.attempt_id.clone(),
            });
        }

        transaction.execute(
            "INSERT INTO orchestration_approvals
                (approval_id, task_id, attempt_id, action_desc, action_payload_json,
                 action_hash, risk_level, policy_source, state, requested_at_ms,
                 expires_at_ms, decided_at_ms, decided_by, decision_reason)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'pending', ?9, ?10, NULL, NULL, NULL)",
            params![
                request.approval_id,
                request.task_id,
                request.attempt_id,
                request.action_desc,
                action_payload_json,
                action_hash,
                request.risk_level,
                request.policy_source,
                now,
                expires,
            ],
        )?;
        append_event_tx(
            &transaction,
            &OrchestrationEvent {
                event_id: request.event_id.clone(),
                task_id: request.task_id.clone(),
                seq: 0,
                kind: "policy.approval_required".into(),
                payload: serde_json::json!({
                    "approval_id": request.approval_id,
                    "attempt_id": request.attempt_id,
                    "action_hash": action_hash,
                    "risk_level": request.risk_level,
                    "policy_source": request.policy_source,
                    "expires_at_ms": request.expires_at_ms,
                }),
                recorded_at_ms: request.now_ms,
            },
        )?;
        transaction.commit()?;
        Ok(WriteStatus::Written)
    }

    /// Resolve a pending approval. First decision wins: a second call on an
    /// already-resolved approval returns `ApprovalAlreadyResolved`.
    pub fn resolve_approval(
        &self,
        request: &ResolveApprovalRequest,
    ) -> LedgerResult<ApprovalRecord> {
        validate_nonempty(&request.approval_id, "approval_id")?;
        validate_nonempty(&request.event_id, "event_id")?;
        if let Some(reason) = request.reason.as_deref() {
            if reason.chars().count() > MAX_APPROVAL_REASON_CHARS {
                return Err(LedgerError::InvalidField("reason"));
            }
        }
        let now = sqlite_u64(request.now_ms, "now_ms")?;
        let new_state = if request.approved {
            "approved"
        } else {
            "denied"
        };
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| LedgerError::LockPoisoned)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let Some(current) = find_approval_tx(&transaction, &request.approval_id)? else {
            return Err(LedgerError::UnknownApproval {
                approval_id: request.approval_id.clone(),
            });
        };
        if current.state != ApprovalState::Pending {
            return Err(LedgerError::ApprovalAlreadyResolved {
                approval_id: request.approval_id.clone(),
                state: current.state.name().into(),
            });
        }
        if request.now_ms < current.requested_at_ms {
            return Err(LedgerError::InvalidField("now_ms"));
        }
        if request.now_ms >= current.expires_at_ms {
            transaction.execute(
                "UPDATE orchestration_approvals
                    SET state = 'expired', decided_at_ms = ?1, decided_by = 'timeout',
                        decision_reason = 'approval expired before decision'
                  WHERE approval_id = ?2 AND state = 'pending'",
                params![now, request.approval_id],
            )?;
            append_event_tx(
                &transaction,
                &OrchestrationEvent {
                    event_id: approval_event_id("expired", &current.approval_id),
                    task_id: current.task_id,
                    seq: 0,
                    kind: "policy.approval_expired".into(),
                    payload: serde_json::json!({
                        "approval_id": current.approval_id,
                        "attempt_id": current.attempt_id,
                        "action_hash": current.action_hash,
                    }),
                    recorded_at_ms: request.now_ms,
                },
            )?;
            transaction.commit()?;
            return Err(LedgerError::ApprovalAlreadyResolved {
                approval_id: request.approval_id.clone(),
                state: ApprovalState::Expired.name().into(),
            });
        }
        let updated = transaction.execute(
            "UPDATE orchestration_approvals
                SET state = ?1, decided_at_ms = ?2, decided_by = ?3, decision_reason = ?4
              WHERE approval_id = ?5 AND state = 'pending'",
            params![
                new_state,
                now,
                request.decided_by.name(),
                request.reason,
                request.approval_id,
            ],
        )?;
        if updated == 0 {
            return Err(LedgerError::ApprovalAlreadyResolved {
                approval_id: request.approval_id.clone(),
                state: "pending".into(),
            });
        }
        let record = fetch_approval_tx(&transaction, &request.approval_id)?;
        append_event_tx(
            &transaction,
            &OrchestrationEvent {
                event_id: request.event_id.clone(),
                task_id: record.task_id.clone(),
                seq: 0,
                kind: if request.approved {
                    "policy.approved".into()
                } else {
                    "policy.denied".into()
                },
                payload: serde_json::json!({
                    "approval_id": record.approval_id,
                    "attempt_id": record.attempt_id,
                    "action_hash": record.action_hash,
                    "decided_by": request.decided_by.name(),
                }),
                recorded_at_ms: request.now_ms,
            },
        )?;
        transaction.commit()?;
        Ok(record)
    }

    /// Mark all pending approvals whose expiry has passed as `expired`.
    /// Returns the IDs that transitioned.
    pub fn expire_pending_approvals(&self, now_ms: u64) -> LedgerResult<Vec<String>> {
        let now = sqlite_u64(now_ms, "now_ms")?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| LedgerError::LockPoisoned)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let expiring = {
            let mut statement = transaction.prepare(
                "SELECT approval_id, task_id, attempt_id, action_desc, action_payload_json,
                        action_hash, risk_level, policy_source, state, requested_at_ms,
                        expires_at_ms, decided_at_ms, decided_by, decision_reason
                 FROM orchestration_approvals
                 WHERE state = 'pending' AND expires_at_ms <= ?1
                 ORDER BY approval_id ASC",
            )?;
            let records = statement
                .query_map(params![now], decode_approval)?
                .collect::<Result<Vec<_>, _>>()?;
            records
        };
        let mut ids = Vec::with_capacity(expiring.len());
        for record in expiring {
            let updated = transaction.execute(
                "UPDATE orchestration_approvals
                    SET state = 'expired', decided_at_ms = ?1, decided_by = 'timeout',
                        decision_reason = 'approval expired'
                  WHERE approval_id = ?2 AND state = 'pending'",
                params![now, &record.approval_id],
            )?;
            if updated == 0 {
                continue;
            }
            append_event_tx(
                &transaction,
                &OrchestrationEvent {
                    event_id: approval_event_id("expired", &record.approval_id),
                    task_id: record.task_id,
                    seq: 0,
                    kind: "policy.approval_expired".into(),
                    payload: serde_json::json!({
                        "approval_id": &record.approval_id,
                        "attempt_id": &record.attempt_id,
                        "action_hash": &record.action_hash,
                    }),
                    recorded_at_ms: now_ms,
                },
            )?;
            ids.push(record.approval_id);
        }
        transaction.commit()?;
        Ok(ids)
    }

    /// Fetch a single approval by id.
    pub fn approval(&self, approval_id: &str) -> LedgerResult<Option<ApprovalRecord>> {
        validate_nonempty(approval_id, "approval_id")?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| LedgerError::LockPoisoned)?;
        let mut statement = connection.prepare(
            "SELECT approval_id, task_id, attempt_id, action_desc, action_payload_json,
                    action_hash, risk_level, policy_source, state, requested_at_ms,
                    expires_at_ms, decided_at_ms, decided_by, decision_reason
             FROM orchestration_approvals WHERE approval_id = ?1",
        )?;
        statement
            .query_row(params![approval_id], decode_approval)
            .optional()
            .map_err(LedgerError::from)
    }

    /// All pending approvals, ordered by request time (oldest first).
    pub fn pending_approvals(&self) -> LedgerResult<Vec<ApprovalRecord>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| LedgerError::LockPoisoned)?;
        let mut statement = connection.prepare(
            "SELECT approval_id, task_id, attempt_id, action_desc, action_payload_json,
                    action_hash, risk_level, policy_source, state, requested_at_ms,
                    expires_at_ms, decided_at_ms, decided_by, decision_reason
             FROM orchestration_approvals
             WHERE state = 'pending'
             ORDER BY requested_at_ms ASC",
        )?;
        let rows = statement.query_map([], decode_approval)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(LedgerError::from)
    }

    /// All approvals for a task (pending and resolved), ordered by request time.
    pub fn approvals_for_task(&self, task_id: &str) -> LedgerResult<Vec<ApprovalRecord>> {
        validate_nonempty(task_id, "task_id")?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| LedgerError::LockPoisoned)?;
        let mut statement = connection.prepare(
            "SELECT approval_id, task_id, attempt_id, action_desc, action_payload_json,
                    action_hash, risk_level, policy_source, state, requested_at_ms,
                    expires_at_ms, decided_at_ms, decided_by, decision_reason
             FROM orchestration_approvals
             WHERE task_id = ?1
             ORDER BY requested_at_ms ASC",
        )?;
        let rows = statement.query_map(params![task_id], decode_approval)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(LedgerError::from)
    }

    // ----- artifacts (D3) --------------------------------------------------

    /// Record artifact metadata. Idempotent by artifact_id.
    pub fn create_artifact(&self, request: &CreateArtifactRequest) -> LedgerResult<WriteStatus> {
        self.create_artifact_with_quota(request, i64::MAX as u64)
    }

    /// Record artifact metadata while atomically enforcing a per-task byte
    /// quota. Both the quota check and insert run under one IMMEDIATE
    /// transaction, so concurrent writers cannot over-commit the task.
    pub fn create_artifact_with_quota(
        &self,
        request: &CreateArtifactRequest,
        max_task_bytes: u64,
    ) -> LedgerResult<WriteStatus> {
        validate_nonempty(&request.artifact_id, "artifact_id")?;
        validate_nonempty(&request.task_id, "task_id")?;
        validate_nonempty(&request.attempt_id, "attempt_id")?;
        validate_nonempty(&request.producer, "producer")?;
        validate_bounded(&request.artifact_id, "artifact_id", 512)?;
        validate_bounded(&request.producer, "producer", 256)?;
        if request.description.chars().count() > 4_096 {
            return Err(LedgerError::InvalidField("description"));
        }
        if request.checksum.len() != 64
            || !request
                .checksum
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(LedgerError::InvalidField("checksum"));
        }
        let now = sqlite_u64(request.created_at_ms, "created_at_ms")?;
        let size = sqlite_u64(request.size_bytes, "size_bytes")?;
        let limit = sqlite_u64(max_task_bytes, "max_task_bytes")?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| LedgerError::LockPoisoned)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;

        let existing = transaction
            .query_row(
                "SELECT artifact_id, task_id, attempt_id, kind, checksum, size_bytes,
                        producer, created_at_ms, pinned, description
                 FROM orchestration_artifacts WHERE artifact_id = ?1",
                params![request.artifact_id],
                decode_artifact,
            )
            .optional()?;
        if let Some(existing) = existing {
            let matches = existing.task_id == request.task_id
                && existing.attempt_id == request.attempt_id
                && existing.kind == request.kind
                && existing.checksum == request.checksum
                && existing.size_bytes == request.size_bytes
                && existing.producer == request.producer
                && existing.created_at_ms == request.created_at_ms
                && existing.description == request.description;
            transaction.rollback()?;
            return if matches {
                Ok(WriteStatus::Duplicate)
            } else {
                Err(LedgerError::ArtifactConflict {
                    artifact_id: request.artifact_id.clone(),
                })
            };
        }

        let current: i64 = transaction.query_row(
            "SELECT COALESCE(SUM(size_bytes), 0)
             FROM orchestration_artifacts WHERE task_id = ?1",
            params![request.task_id],
            |row| row.get(0),
        )?;
        let total = current
            .checked_add(size)
            .ok_or(LedgerError::NumericOverflow("task_artifact_bytes"))?;
        if total > limit {
            transaction.rollback()?;
            return Err(LedgerError::ArtifactQuotaExceeded {
                current: u64::try_from(current)
                    .map_err(|_| LedgerError::NumericOverflow("task_artifact_bytes"))?,
                attempted: request.size_bytes,
                limit: max_task_bytes,
            });
        }

        transaction.execute(
            "INSERT INTO orchestration_artifacts
                (artifact_id, task_id, attempt_id, kind, checksum, size_bytes,
                 producer, created_at_ms, pinned, description)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0, ?9)",
            params![
                request.artifact_id,
                request.task_id,
                request.attempt_id,
                request.kind.name(),
                request.checksum,
                size,
                request.producer,
                now,
                request.description,
            ],
        )?;
        transaction.commit()?;
        Ok(WriteStatus::Written)
    }

    /// Fetch a single artifact by id.
    pub fn artifact(&self, artifact_id: &str) -> LedgerResult<Option<ArtifactRecord>> {
        validate_nonempty(artifact_id, "artifact_id")?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| LedgerError::LockPoisoned)?;
        let mut statement = connection.prepare(
            "SELECT artifact_id, task_id, attempt_id, kind, checksum, size_bytes,
                    producer, created_at_ms, pinned, description
             FROM orchestration_artifacts WHERE artifact_id = ?1",
        )?;
        statement
            .query_row(params![artifact_id], decode_artifact)
            .optional()
            .map_err(LedgerError::from)
    }

    /// All artifacts for a task, ordered by creation time.
    pub fn artifacts_for_task(&self, task_id: &str) -> LedgerResult<Vec<ArtifactRecord>> {
        validate_nonempty(task_id, "task_id")?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| LedgerError::LockPoisoned)?;
        let mut statement = connection.prepare(
            "SELECT artifact_id, task_id, attempt_id, kind, checksum, size_bytes,
                    producer, created_at_ms, pinned, description
             FROM orchestration_artifacts
             WHERE task_id = ?1
             ORDER BY created_at_ms ASC",
        )?;
        let rows = statement.query_map(params![task_id], decode_artifact)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(LedgerError::from)
    }

    /// Pin an artifact so it is never auto-cleaned.
    pub fn pin_artifact(&self, artifact_id: &str) -> LedgerResult<WriteStatus> {
        validate_nonempty(artifact_id, "artifact_id")?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| LedgerError::LockPoisoned)?;
        let updated = connection.execute(
            "UPDATE orchestration_artifacts SET pinned = 1 WHERE artifact_id = ?1",
            params![artifact_id],
        )?;
        if updated == 0 {
            Ok(WriteStatus::Duplicate)
        } else {
            Ok(WriteStatus::Written)
        }
    }

    /// Unpin an artifact (allow auto-cleanup).
    pub fn unpin_artifact(&self, artifact_id: &str) -> LedgerResult<WriteStatus> {
        validate_nonempty(artifact_id, "artifact_id")?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| LedgerError::LockPoisoned)?;
        let updated = connection.execute(
            "UPDATE orchestration_artifacts SET pinned = 0 WHERE artifact_id = ?1",
            params![artifact_id],
        )?;
        if updated == 0 {
            Ok(WriteStatus::Duplicate)
        } else {
            Ok(WriteStatus::Written)
        }
    }

    /// Unpinned artifacts older than `cutoff_ms` — candidates for cleanup.
    pub fn cleanup_candidates(
        &self,
        cutoff_ms: u64,
        limit: usize,
    ) -> LedgerResult<Vec<ArtifactRecord>> {
        let cutoff = sqlite_u64(cutoff_ms, "cutoff_ms")?;
        let limit = i64::try_from(limit).unwrap_or(1000);
        let connection = self
            .connection
            .lock()
            .map_err(|_| LedgerError::LockPoisoned)?;
        let mut statement = connection.prepare(
            "SELECT artifact_id, task_id, attempt_id, kind, checksum, size_bytes,
                    producer, created_at_ms, pinned, description
             FROM orchestration_artifacts
             WHERE pinned = 0 AND created_at_ms < ?1
             ORDER BY created_at_ms ASC LIMIT ?2",
        )?;
        let rows = statement.query_map(params![cutoff, limit], decode_artifact)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(LedgerError::from)
    }

    /// Delete an artifact record (does not delete the blob). Returns whether
    /// a row was actually removed.
    pub fn delete_artifact(&self, artifact_id: &str) -> LedgerResult<bool> {
        validate_nonempty(artifact_id, "artifact_id")?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| LedgerError::LockPoisoned)?;
        let deleted = connection.execute(
            "DELETE FROM orchestration_artifacts WHERE artifact_id = ?1",
            params![artifact_id],
        )?;
        Ok(deleted > 0)
    }

    /// Delete an artifact only if it is still an eligible cleanup candidate.
    /// Returns `Some(true)` when the removed row was the final reference to its
    /// checksum, `Some(false)` when other artifacts still share the blob, and
    /// `None` when the row was pinned, too new, or already absent.
    pub fn delete_cleanup_candidate(
        &self,
        artifact_id: &str,
        cutoff_ms: u64,
    ) -> LedgerResult<Option<bool>> {
        validate_nonempty(artifact_id, "artifact_id")?;
        let cutoff = sqlite_u64(cutoff_ms, "cutoff_ms")?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| LedgerError::LockPoisoned)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let checksum = transaction
            .query_row(
                "SELECT checksum FROM orchestration_artifacts
                 WHERE artifact_id = ?1 AND pinned = 0 AND created_at_ms < ?2",
                params![artifact_id, cutoff],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        let Some(checksum) = checksum else {
            transaction.rollback()?;
            return Ok(None);
        };
        let deleted = transaction.execute(
            "DELETE FROM orchestration_artifacts
             WHERE artifact_id = ?1 AND pinned = 0 AND created_at_ms < ?2",
            params![artifact_id, cutoff],
        )?;
        if deleted == 0 {
            transaction.rollback()?;
            return Ok(None);
        }
        let remaining: i64 = transaction.query_row(
            "SELECT COUNT(*) FROM orchestration_artifacts WHERE checksum = ?1",
            params![checksum],
            |row| row.get(0),
        )?;
        transaction.commit()?;
        Ok(Some(remaining == 0))
    }
}

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
    let state_name: String = row.get(6)?;
    let state = TaskState::from_name(&state_name).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            6,
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
        description: row.get(5)?,
        state,
        created_at_ms: decode_u64(row, 7, "created_at_ms")?,
        updated_at_ms: decode_u64(row, 8, "updated_at_ms")?,
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

fn decode_approval(row: &rusqlite::Row<'_>) -> rusqlite::Result<ApprovalRecord> {
    let action_payload_json: String = row.get(4)?;
    let action_payload = serde_json::from_str(&action_payload_json)
        .map_err(|error| from_sql_failure(4, rusqlite::types::Type::Text, error.to_string()))?;
    let state_str: String = row.get(8)?;
    let state = ApprovalState::parse(&state_str).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            8,
            rusqlite::types::Type::Text,
            format!("unknown approval state: {state_str}").into(),
        )
    })?;
    let decided_by_str: Option<String> = row.get(12)?;
    let decided_by = decided_by_str
        .map(|value| {
            ApprovalDecidedBy::parse(&value).ok_or_else(|| {
                from_sql_failure(
                    12,
                    rusqlite::types::Type::Text,
                    format!("unknown approval decision source: {value}"),
                )
            })
        })
        .transpose()?;
    Ok(ApprovalRecord {
        approval_id: row.get(0)?,
        task_id: row.get(1)?,
        attempt_id: row.get(2)?,
        action_desc: row.get(3)?,
        action_payload,
        action_hash: row.get(5)?,
        risk_level: row.get(6)?,
        policy_source: row.get(7)?,
        state,
        requested_at_ms: decode_u64(row, 9, "requested_at_ms")?,
        expires_at_ms: decode_u64(row, 10, "expires_at_ms")?,
        decided_at_ms: decode_optional_u64(row, 11, "decided_at_ms")?,
        decided_by,
        decision_reason: row.get(13)?,
    })
}

fn decode_artifact(row: &rusqlite::Row<'_>) -> rusqlite::Result<ArtifactRecord> {
    let kind_str: String = row.get(3)?;
    let kind = ArtifactKind::parse(&kind_str).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            3,
            rusqlite::types::Type::Text,
            format!("unknown artifact kind: {kind_str}").into(),
        )
    })?;
    let pinned: i64 = row.get(8)?;
    Ok(ArtifactRecord {
        artifact_id: row.get(0)?,
        task_id: row.get(1)?,
        attempt_id: row.get(2)?,
        kind,
        checksum: row.get(4)?,
        size_bytes: decode_u64(row, 5, "size_bytes")?,
        producer: row.get(6)?,
        created_at_ms: decode_u64(row, 7, "created_at_ms")?,
        pinned: pinned != 0,
        description: row.get(9)?,
    })
}

fn fetch_approval_tx(
    transaction: &Transaction<'_>,
    approval_id: &str,
) -> LedgerResult<ApprovalRecord> {
    transaction
        .query_row(
            "SELECT approval_id, task_id, attempt_id, action_desc, action_payload_json,
                    action_hash, risk_level, policy_source, state, requested_at_ms,
                    expires_at_ms, decided_at_ms, decided_by, decision_reason
             FROM orchestration_approvals WHERE approval_id = ?1",
            params![approval_id],
            decode_approval,
        )
        .map_err(LedgerError::from)
}

fn find_approval_tx(
    transaction: &Transaction<'_>,
    approval_id: &str,
) -> LedgerResult<Option<ApprovalRecord>> {
    transaction
        .query_row(
            "SELECT approval_id, task_id, attempt_id, action_desc, action_payload_json,
                    action_hash, risk_level, policy_source, state, requested_at_ms,
                    expires_at_ms, decided_at_ms, decided_by, decision_reason
             FROM orchestration_approvals WHERE approval_id = ?1",
            params![approval_id],
            decode_approval,
        )
        .optional()
        .map_err(LedgerError::from)
}

fn approval_matches_request(
    record: &ApprovalRecord,
    request: &CreateApprovalRequest,
    action_hash: &str,
    expires_at_ms: u64,
) -> bool {
    record.task_id == request.task_id
        && record.attempt_id == request.attempt_id
        && record.action_desc == request.action_desc
        && record.action_payload == request.action_payload
        && record.action_hash == action_hash
        && record.risk_level == request.risk_level
        && record.policy_source == request.policy_source
        && record.requested_at_ms == request.now_ms
        && record.expires_at_ms == expires_at_ms
}

fn attempt_belongs_to_task(
    transaction: &Transaction<'_>,
    attempt_id: &str,
    task_id: &str,
) -> LedgerResult<bool> {
    Ok(transaction
        .query_row(
            "SELECT 1 FROM orchestration_attempts
             WHERE attempt_id = ?1 AND task_id = ?2",
            params![attempt_id, task_id],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

/// Stable SHA-256 identity for the exact structured action covered by an
/// approval. Object keys are sorted recursively so semantically identical JSON
/// does not produce a different authorization identity.
pub fn approval_action_hash(action: &Value) -> LedgerResult<String> {
    let canonical = canonical_approval_action(action)?;
    Ok(approval_action_hash_from_json(&canonical))
}

fn canonical_approval_action(action: &Value) -> LedgerResult<String> {
    let mut output = String::new();
    write_canonical_json(action, &mut output)?;
    if output.len() > MAX_APPROVAL_ACTION_BYTES {
        return Err(LedgerError::InvalidField("action_payload"));
    }
    Ok(output)
}

fn write_canonical_json(value: &Value, output: &mut String) -> Result<(), serde_json::Error> {
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {
            output.push_str(&serde_json::to_string(value)?);
        }
        Value::Array(values) => {
            output.push('[');
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_canonical_json(value, output)?;
            }
            output.push(']');
        }
        Value::Object(values) => {
            output.push('{');
            let mut entries: Vec<_> = values.iter().collect();
            entries.sort_unstable_by(|left, right| left.0.cmp(right.0));
            for (index, (key, value)) in entries.into_iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                output.push_str(&serde_json::to_string(key)?);
                output.push(':');
                write_canonical_json(value, output)?;
            }
            output.push('}');
        }
    }
    Ok(())
}

fn approval_action_hash_from_json(canonical_json: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(canonical_json.as_bytes());
    hex::encode(hash.finalize())
}

fn approval_event_id(kind: &str, approval_id: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(b"orchestration-approval-event\0");
    hash.update(kind.as_bytes());
    hash.update(b"\0");
    hash.update(approval_id.as_bytes());
    format!("approval:{}", hex::encode(hash.finalize()))
}

fn validate_nonempty(value: &str, field: &'static str) -> LedgerResult<()> {
    if value.trim().is_empty() {
        return Err(LedgerError::InvalidField(field));
    }
    Ok(())
}

fn validate_bounded(value: &str, field: &'static str, max_chars: usize) -> LedgerResult<()> {
    validate_nonempty(value, field)?;
    if value.chars().count() > max_chars {
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
    if current < 2 {
        let applied_at_ms = now_ms();
        transaction.execute_batch(MIGRATION_V2)?;
        transaction.execute(
            "INSERT INTO orchestration_migrations (version, applied_at_ms) VALUES (2, ?1)",
            params![applied_at_ms],
        )?;
    }
    if current < 3 {
        let applied_at_ms = now_ms();
        transaction.execute_batch(MIGRATION_V3)?;
        transaction.execute(
            "INSERT INTO orchestration_migrations (version, applied_at_ms) VALUES (3, ?1)",
            params![applied_at_ms],
        )?;
    }
    if current < 4 {
        let applied_at_ms = now_ms();
        transaction.execute_batch(MIGRATION_V4)?;
        transaction.execute(
            "INSERT INTO orchestration_migrations (version, applied_at_ms) VALUES (4, ?1)",
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
            description: String::new(),
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
        assert_eq!(ledger.schema_version().expect("schema"), 4);
        drop(ledger);
        // Reopening must not re-apply or fail.
        let reopened = OrchestrationLedger::open(&path).expect("reopen");
        assert_eq!(reopened.schema_version().expect("schema"), 4);
    }

    #[test]
    fn migration_v2_preserves_v1_tasks_with_empty_description() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("v1.sqlite3");
        let connection = Connection::open(&path).expect("db");
        connection
            .execute_batch(
                "CREATE TABLE orchestration_migrations (
                     version INTEGER PRIMARY KEY,
                     applied_at_ms INTEGER NOT NULL
                 );",
            )
            .expect("migration table");
        connection.execute_batch(MIGRATION_V1).expect("v1 schema");
        connection
            .execute(
                "INSERT INTO orchestration_migrations (version, applied_at_ms) VALUES (1, 0)",
                [],
            )
            .expect("v1 marker");
        connection
            .execute(
                "INSERT INTO orchestration_tasks
                    (task_id, workspace_key, source_kind, source_ref, title, state,
                     created_at_ms, updated_at_ms)
                 VALUES ('t1', 'ws', 'local', 'native-1', 'Legacy task', 'queued', 1, 1)",
                [],
            )
            .expect("legacy task");
        drop(connection);

        let ledger = OrchestrationLedger::open(&path).expect("upgrade");
        assert_eq!(ledger.schema_version().expect("schema"), 4);
        let task = ledger.task("t1").expect("lookup").expect("task");
        assert_eq!(task.title, "Legacy task");
        assert_eq!(task.description, "");
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
        ledger
            .upsert_task(&task("needs-attention", TaskState::NeedsAttention))
            .expect("t");

        let active = ledger.active_tasks("ws-1").expect("active");
        let ids: Vec<_> = active.iter().map(|t| t.task_id.as_str()).collect();
        assert_eq!(ids, vec!["active", "needs-attention", "queued"]);

        let non_terminal = ledger.non_terminal_tasks().expect("non-terminal");
        let ids: Vec<_> = non_terminal
            .iter()
            .map(|task| task.task_id.as_str())
            .collect();
        assert_eq!(ids, vec!["active", "needs-attention", "queued"]);
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

    // ----- approval store tests (B2) ---------------------------------------

    fn approval_req(id: &str, task: &str, attempt: &str) -> CreateApprovalRequest {
        CreateApprovalRequest {
            approval_id: id.into(),
            event_id: format!("{id}:requested"),
            task_id: task.into(),
            attempt_id: attempt.into(),
            action_desc: "git push --force origin main".into(),
            action_payload: serde_json::json!({
                "kind": "git_push",
                "branch": "main",
                "force": true,
            }),
            risk_level: "high".into(),
            policy_source: "default".into(),
            expires_at_ms: 10_000,
            now_ms: 5_000,
        }
    }

    fn seed_task_attempt(ledger: &OrchestrationLedger, task_id: &str) {
        ledger
            .upsert_task(&task(task_id, TaskState::Running))
            .unwrap();
        ledger
            .create_attempt(&attempt_req(
                task_id,
                1,
                &format!("{task_id}:approval-test"),
            ))
            .unwrap();
    }

    #[test]
    fn create_and_fetch_approval() {
        let ledger = OrchestrationLedger::open_in_memory().unwrap();
        seed_task_attempt(&ledger, "t-1");

        let status = ledger
            .create_approval_request(&approval_req("ap-1", "t-1", "t-1-att-1"))
            .unwrap();
        assert_eq!(status, WriteStatus::Written);

        let rec = ledger.approval("ap-1").unwrap().expect("present");
        assert_eq!(rec.task_id, "t-1");
        assert_eq!(rec.state, ApprovalState::Pending);
        assert_eq!(rec.risk_level, "high");
        assert_eq!(
            rec.action_payload,
            serde_json::json!({"kind": "git_push", "branch": "main", "force": true})
        );
        assert_eq!(
            rec.action_hash,
            approval_action_hash(&rec.action_payload).unwrap()
        );
        assert!(rec.decided_at_ms.is_none());
        let events = ledger.events_for_task("t-1", 0, 100).unwrap();
        assert!(events
            .iter()
            .any(|event| event.kind == "policy.approval_required"));
    }

    #[test]
    fn create_approval_idempotent() {
        let ledger = OrchestrationLedger::open_in_memory().unwrap();
        seed_task_attempt(&ledger, "t-1");

        ledger
            .create_approval_request(&approval_req("ap-1", "t-1", "t-1-att-1"))
            .unwrap();
        let status = ledger
            .create_approval_request(&approval_req("ap-1", "t-1", "t-1-att-1"))
            .unwrap();
        assert_eq!(status, WriteStatus::Duplicate);
    }

    #[test]
    fn conflicting_approval_id_is_not_treated_as_idempotent() {
        let ledger = OrchestrationLedger::open_in_memory().unwrap();
        seed_task_attempt(&ledger, "t-1");

        let original = approval_req("ap-1", "t-1", "t-1-att-1");
        ledger.create_approval_request(&original).unwrap();
        let mut changed = original.clone();
        changed.action_payload["force"] = serde_json::json!(false);

        assert!(matches!(
            ledger.create_approval_request(&changed),
            Err(LedgerError::ApprovalConflict { approval_id }) if approval_id == "ap-1"
        ));
    }

    #[test]
    fn approval_requires_a_matching_attempt_and_valid_expiry() {
        let ledger = OrchestrationLedger::open_in_memory().unwrap();
        seed_task_attempt(&ledger, "t-1");
        seed_task_attempt(&ledger, "t-2");

        let wrong_task = approval_req("ap-wrong", "t-1", "t-2-att-1");
        assert!(matches!(
            ledger.create_approval_request(&wrong_task),
            Err(LedgerError::ApprovalAttemptMismatch {
                task_id,
                attempt_id
            }) if task_id == "t-1" && attempt_id == "t-2-att-1"
        ));

        let mut expired = approval_req("ap-expired", "t-1", "t-1-att-1");
        expired.expires_at_ms = expired.now_ms;
        assert!(matches!(
            ledger.create_approval_request(&expired),
            Err(LedgerError::InvalidField("expires_at_ms"))
        ));
    }

    #[test]
    fn invalid_approval_fields_are_errors_not_duplicate_writes() {
        let ledger = OrchestrationLedger::open_in_memory().unwrap();
        seed_task_attempt(&ledger, "t-1");
        let mut invalid = approval_req("ap-1", "t-1", "t-1-att-1");
        invalid.risk_level = "critical".into();
        assert!(matches!(
            ledger.create_approval_request(&invalid),
            Err(LedgerError::InvalidField("risk_level"))
        ));
        assert!(ledger.approval("ap-1").unwrap().is_none());
    }

    #[test]
    fn approval_and_event_write_are_atomic() {
        let ledger = OrchestrationLedger::open_in_memory().unwrap();
        seed_task_attempt(&ledger, "t-1");
        let mut request = approval_req("ap-1", "t-1", "t-1-att-1");
        request.event_id = "t-1-att-1:attempt.created".into();

        assert!(matches!(
            ledger.create_approval_request(&request),
            Err(LedgerError::EventConflict { .. })
        ));
        assert!(ledger.approval("ap-1").unwrap().is_none());
    }

    #[test]
    fn approval_resolution_rolls_back_when_its_event_conflicts() {
        let ledger = OrchestrationLedger::open_in_memory().unwrap();
        seed_task_attempt(&ledger, "t-1");
        ledger
            .create_approval_request(&approval_req("ap-1", "t-1", "t-1-att-1"))
            .unwrap();

        assert!(matches!(
            ledger.resolve_approval(&ResolveApprovalRequest {
                approval_id: "ap-1".into(),
                event_id: "t-1-att-1:attempt.created".into(),
                approved: true,
                decided_by: ApprovalDecidedBy::Human,
                reason: None,
                now_ms: 6_000,
            }),
            Err(LedgerError::EventConflict { .. })
        ));
        assert_eq!(
            ledger.approval("ap-1").unwrap().unwrap().state,
            ApprovalState::Pending
        );
    }

    #[test]
    fn approval_action_hash_is_canonical_and_action_specific() {
        let first = serde_json::json!({"branch": "main", "force": true});
        let mut second = serde_json::Map::new();
        second.insert("force".into(), serde_json::json!(true));
        second.insert("branch".into(), serde_json::json!("main"));

        assert_eq!(
            approval_action_hash(&first).unwrap(),
            approval_action_hash(&Value::Object(second)).unwrap()
        );
        assert_ne!(
            approval_action_hash(&first).unwrap(),
            approval_action_hash(&serde_json::json!({"branch": "main", "force": false})).unwrap()
        );
    }

    #[test]
    fn resolve_approval_first_decision_wins() {
        let ledger = OrchestrationLedger::open_in_memory().unwrap();
        seed_task_attempt(&ledger, "t-1");
        ledger
            .create_approval_request(&approval_req("ap-1", "t-1", "t-1-att-1"))
            .unwrap();

        let rec = ledger
            .resolve_approval(&ResolveApprovalRequest {
                approval_id: "ap-1".into(),
                event_id: "ap-1:approved".into(),
                approved: true,
                decided_by: ApprovalDecidedBy::Human,
                reason: Some("looks good".into()),
                now_ms: 6_000,
            })
            .unwrap();
        assert_eq!(rec.state, ApprovalState::Approved);
        assert_eq!(rec.decided_by, Some(ApprovalDecidedBy::Human));
        assert_eq!(rec.decision_reason, Some("looks good".into()));
        assert!(ledger
            .events_for_task("t-1", 0, 100)
            .unwrap()
            .iter()
            .any(|event| event.kind == "policy.approved"));

        // Second resolution fails.
        let err = ledger
            .resolve_approval(&ResolveApprovalRequest {
                approval_id: "ap-1".into(),
                event_id: "ap-1:second-decision".into(),
                approved: false,
                decided_by: ApprovalDecidedBy::Human,
                reason: None,
                now_ms: 7_000,
            })
            .unwrap_err();
        assert!(matches!(
            err,
            LedgerError::ApprovalAlreadyResolved { approval_id, state }
                if approval_id == "ap-1" && state == "approved"
        ));
    }

    #[test]
    fn resolve_unknown_approval_errors() {
        let ledger = OrchestrationLedger::open_in_memory().unwrap();
        let err = ledger
            .resolve_approval(&ResolveApprovalRequest {
                approval_id: "nope".into(),
                event_id: "nope:approved".into(),
                approved: true,
                decided_by: ApprovalDecidedBy::Human,
                reason: None,
                now_ms: 1_000,
            })
            .unwrap_err();
        assert!(matches!(
            err,
            LedgerError::UnknownApproval { approval_id } if approval_id == "nope"
        ));
    }

    #[test]
    fn deny_resolution_persists() {
        let ledger = OrchestrationLedger::open_in_memory().unwrap();
        seed_task_attempt(&ledger, "t-1");
        ledger
            .create_approval_request(&approval_req("ap-1", "t-1", "t-1-att-1"))
            .unwrap();

        let rec = ledger
            .resolve_approval(&ResolveApprovalRequest {
                approval_id: "ap-1".into(),
                event_id: "ap-1:denied".into(),
                approved: false,
                decided_by: ApprovalDecidedBy::Human,
                reason: Some("dangerous".into()),
                now_ms: 6_000,
            })
            .unwrap();
        assert_eq!(rec.state, ApprovalState::Denied);
    }

    #[test]
    fn pending_approvals_lists_only_pending() {
        let ledger = OrchestrationLedger::open_in_memory().unwrap();
        seed_task_attempt(&ledger, "t-1");

        for i in 1..=3 {
            ledger
                .create_approval_request(&approval_req(&format!("ap-{i}"), "t-1", "t-1-att-1"))
                .unwrap();
        }
        // Resolve one.
        ledger
            .resolve_approval(&ResolveApprovalRequest {
                approval_id: "ap-2".into(),
                event_id: "ap-2:approved".into(),
                approved: true,
                decided_by: ApprovalDecidedBy::Human,
                reason: None,
                now_ms: 6_000,
            })
            .unwrap();

        let pending = ledger.pending_approvals().unwrap();
        assert_eq!(pending.len(), 2);
        // Ordered by request time; both have the same now_ms so by id insertion.
        let ids: Vec<_> = pending.iter().map(|a| a.approval_id.as_str()).collect();
        assert!(ids.contains(&"ap-1"));
        assert!(ids.contains(&"ap-3"));
        assert!(!ids.contains(&"ap-2"));
    }

    #[test]
    fn approvals_for_task_includes_resolved() {
        let ledger = OrchestrationLedger::open_in_memory().unwrap();
        seed_task_attempt(&ledger, "t-1");
        seed_task_attempt(&ledger, "t-2");

        ledger
            .create_approval_request(&approval_req("ap-1", "t-1", "t-1-att-1"))
            .unwrap();
        ledger
            .create_approval_request(&approval_req("ap-2", "t-2", "t-2-att-1"))
            .unwrap();
        ledger
            .resolve_approval(&ResolveApprovalRequest {
                approval_id: "ap-1".into(),
                event_id: "ap-1:policy-approved".into(),
                approved: true,
                decided_by: ApprovalDecidedBy::Policy,
                reason: None,
                now_ms: 6_000,
            })
            .unwrap();

        let for_t1 = ledger.approvals_for_task("t-1").unwrap();
        assert_eq!(for_t1.len(), 1);
        assert_eq!(for_t1[0].state, ApprovalState::Approved);
        assert_eq!(for_t1[0].decided_by, Some(ApprovalDecidedBy::Policy));

        let for_t2 = ledger.approvals_for_task("t-2").unwrap();
        assert_eq!(for_t2.len(), 1);
        assert_eq!(for_t2[0].state, ApprovalState::Pending);
    }

    #[test]
    fn expire_pending_approvals_transitions_expired() {
        let ledger = OrchestrationLedger::open_in_memory().unwrap();
        seed_task_attempt(&ledger, "t-1");

        // Create one with an expiry.
        let mut req = approval_req("ap-1", "t-1", "t-1-att-1");
        req.expires_at_ms = 10_000;
        ledger.create_approval_request(&req).unwrap();

        // Create one whose required expiry is still in the future.
        let mut later = approval_req("ap-2", "t-1", "t-1-att-1");
        later.expires_at_ms = 30_000;
        ledger.create_approval_request(&later).unwrap();

        // Before expiry: nothing transitions.
        let expired = ledger.expire_pending_approvals(9_000).unwrap();
        assert!(expired.is_empty());

        // After expiry: only ap-1 transitions.
        let expired = ledger.expire_pending_approvals(11_000).unwrap();
        assert_eq!(expired, vec!["ap-1".to_string()]);

        let rec = ledger.approval("ap-1").unwrap().unwrap();
        assert_eq!(rec.state, ApprovalState::Expired);
        assert_eq!(rec.decided_by, Some(ApprovalDecidedBy::Timeout));
        assert!(ledger
            .events_for_task("t-1", 0, 100)
            .unwrap()
            .iter()
            .any(|event| event.kind == "policy.approval_expired"));

        // ap-2 is still pending.
        let rec2 = ledger.approval("ap-2").unwrap().unwrap();
        assert_eq!(rec2.state, ApprovalState::Pending);
    }

    #[test]
    fn resolved_approval_cannot_be_expired() {
        let ledger = OrchestrationLedger::open_in_memory().unwrap();
        seed_task_attempt(&ledger, "t-1");

        let mut req = approval_req("ap-1", "t-1", "t-1-att-1");
        req.expires_at_ms = 10_000;
        ledger.create_approval_request(&req).unwrap();

        // Approve it.
        ledger
            .resolve_approval(&ResolveApprovalRequest {
                approval_id: "ap-1".into(),
                event_id: "ap-1:approved".into(),
                approved: true,
                decided_by: ApprovalDecidedBy::Human,
                reason: None,
                now_ms: 6_000,
            })
            .unwrap();

        // Expiry sweep finds nothing.
        let expired = ledger.expire_pending_approvals(20_000).unwrap();
        assert!(expired.is_empty());
    }

    #[test]
    fn decision_after_expiry_fails_closed_and_persists_expiration() {
        let ledger = OrchestrationLedger::open_in_memory().unwrap();
        seed_task_attempt(&ledger, "t-1");
        ledger
            .create_approval_request(&approval_req("ap-1", "t-1", "t-1-att-1"))
            .unwrap();

        assert!(matches!(
            ledger.resolve_approval(&ResolveApprovalRequest {
                approval_id: "ap-1".into(),
                event_id: "ap-1:late-approval".into(),
                approved: true,
                decided_by: ApprovalDecidedBy::Human,
                reason: None,
                now_ms: 10_000,
            }),
            Err(LedgerError::ApprovalAlreadyResolved { state, .. }) if state == "expired"
        ));
        assert_eq!(
            ledger.approval("ap-1").unwrap().unwrap().state,
            ApprovalState::Expired
        );
    }

    #[test]
    fn migration_v3_adds_approvals_table() {
        let ledger = OrchestrationLedger::open_in_memory().unwrap();
        // If the table exists, we can query it without error.
        let pending = ledger.pending_approvals().unwrap();
        assert!(pending.is_empty());
    }
}
