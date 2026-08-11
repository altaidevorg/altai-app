//! Durable Work / Attempt / Review store (Work OS Milestone 1).
//!
//! User-scoped SQLite beside the existing host — not a separate control-plane
//! daemon. Schema matches `altaidevorg/altai-agent-work-os` ENGINEERING.md.

use crate::journal::{EventJournal, JournalError, RunJournalSummary};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use std::fmt;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const SCHEMA_VERSION: i64 = 2;
static NEXT_ID_SEQUENCE: AtomicU64 = AtomicU64::new(1);

const MIGRATION_V1: &str = r#"
CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    workspace_ref TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL CHECK (created_at_ms >= 0),
    updated_at_ms INTEGER NOT NULL CHECK (updated_at_ms >= 0)
);

CREATE TABLE IF NOT EXISTS work_items (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    acceptance_criteria TEXT NOT NULL DEFAULT '',
    state TEXT NOT NULL CHECK (state IN (
        'backlog', 'ready', 'in_progress', 'in_review', 'done', 'cancelled'
    )),
    assignee_ref TEXT,
    blocker TEXT,
    revision INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0),
    managed_mode TEXT CHECK (
        managed_mode IS NULL OR managed_mode IN ('audit', 'assist', 'auto')
    ),
    managed_max_rounds INTEGER CHECK (
        managed_max_rounds IS NULL OR managed_max_rounds > 0
    ),
    created_at_ms INTEGER NOT NULL CHECK (created_at_ms >= 0),
    updated_at_ms INTEGER NOT NULL CHECK (updated_at_ms >= 0)
);

CREATE INDEX IF NOT EXISTS work_items_project_state
    ON work_items (project_id, state, updated_at_ms DESC);

CREATE TABLE IF NOT EXISTS attempts (
    id TEXT PRIMARY KEY,
    work_id TEXT NOT NULL REFERENCES work_items(id),
    number INTEGER NOT NULL CHECK (number > 0),
    role TEXT NOT NULL DEFAULT 'executor',
    phase TEXT NOT NULL CHECK (phase IN (
        'queued', 'running', 'waiting', 'succeeded', 'failed', 'cancelled'
    )),
    chat_id TEXT,
    session_id TEXT,
    run_id TEXT,
    input_json TEXT,
    result_json TEXT,
    created_at_ms INTEGER NOT NULL CHECK (created_at_ms >= 0),
    updated_at_ms INTEGER NOT NULL CHECK (updated_at_ms >= 0),
    UNIQUE (work_id, number)
);

CREATE INDEX IF NOT EXISTS attempts_work_number
    ON attempts (work_id, number DESC);

CREATE TABLE IF NOT EXISTS reviews (
    id TEXT PRIMARY KEY,
    attempt_id TEXT NOT NULL REFERENCES attempts(id),
    reviewer_kind TEXT NOT NULL CHECK (reviewer_kind IN ('human', 'agent')),
    status TEXT NOT NULL CHECK (status IN (
        'complete', 'incomplete', 'blocked'
    )),
    integrity TEXT NOT NULL DEFAULT 'ok',
    acceptance_aligned INTEGER NOT NULL DEFAULT 0 CHECK (acceptance_aligned IN (0, 1)),
    evidence_json TEXT NOT NULL DEFAULT '[]',
    missing_json TEXT NOT NULL DEFAULT '[]',
    guidance TEXT NOT NULL DEFAULT '',
    created_at_ms INTEGER NOT NULL CHECK (created_at_ms >= 0)
);

CREATE INDEX IF NOT EXISTS reviews_attempt
    ON reviews (attempt_id, created_at_ms DESC);

CREATE TABLE IF NOT EXISTS work_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    work_id TEXT NOT NULL REFERENCES work_items(id),
    kind TEXT NOT NULL,
    payload_json TEXT NOT NULL DEFAULT '{}',
    created_at_ms INTEGER NOT NULL CHECK (created_at_ms >= 0)
);

CREATE INDEX IF NOT EXISTS work_events_work_time
    ON work_events (work_id, created_at_ms DESC);
"#;

const MIGRATION_V2: &str = r#"
CREATE UNIQUE INDEX IF NOT EXISTS attempts_one_active_per_work
    ON attempts (work_id)
    WHERE phase IN ('queued', 'running', 'waiting');

CREATE UNIQUE INDEX IF NOT EXISTS attempts_one_binding_per_run
    ON attempts (run_id)
    WHERE run_id IS NOT NULL;
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkState {
    Backlog,
    Ready,
    InProgress,
    InReview,
    Done,
    Cancelled,
}

impl WorkState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Backlog => "backlog",
            Self::Ready => "ready",
            Self::InProgress => "in_progress",
            Self::InReview => "in_review",
            Self::Done => "done",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "backlog" => Some(Self::Backlog),
            "ready" => Some(Self::Ready),
            "in_progress" => Some(Self::InProgress),
            "in_review" => Some(Self::InReview),
            "done" => Some(Self::Done),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttemptPhase {
    Queued,
    Running,
    Waiting,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttemptReconcileMode {
    Live,
    RestartRecovery,
}

impl AttemptPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Waiting => "waiting",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "queued" => Some(Self::Queued),
            "running" => Some(Self::Running),
            "waiting" => Some(Self::Waiting),
            "succeeded" => Some(Self::Succeeded),
            "failed" => Some(Self::Failed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkItemRecord {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub description: String,
    pub acceptance_criteria: String,
    pub state: WorkState,
    pub assignee_ref: Option<String>,
    pub blocker: Option<String>,
    pub revision: i64,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttemptRecord {
    pub id: String,
    pub work_id: String,
    pub number: i64,
    pub role: String,
    pub phase: AttemptPhase,
    pub chat_id: Option<String>,
    pub session_id: Option<String>,
    pub run_id: Option<String>,
    pub input_json: Option<String>,
    pub result_json: Option<String>,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkInboxKind {
    ReviewRequired,
    Approval,
    Question,
    FailedAttempt,
    Blocked,
}

impl WorkInboxKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReviewRequired => "review_required",
            Self::Approval => "approval",
            Self::Question => "question",
            Self::FailedAttempt => "failed_attempt",
            Self::Blocked => "blocked",
        }
    }
}

/// One actionable Work condition. This is a projection over canonical source
/// records, never a mutable notification record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkInboxRecord {
    pub id: String,
    pub work_id: String,
    pub kind: WorkInboxKind,
    pub title: String,
    pub why: String,
    pub created_at_ms: u64,
    pub attempt_id: Option<String>,
    pub chat_id: Option<String>,
    pub run_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkAttemptStart {
    pub work: WorkItemRecord,
    pub attempt: AttemptRecord,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateWorkInput {
    pub project_id: String,
    pub title: String,
    pub description: String,
    pub acceptance_criteria: String,
    pub assignee_ref: Option<String>,
}

#[derive(Debug)]
pub enum WorkStoreError {
    Io(std::io::Error),
    Sqlite(rusqlite::Error),
    Journal(JournalError),
    UnsupportedSchema(i64),
    NotFound(String),
    InvalidState(&'static str),
}

impl fmt::Display for WorkStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "work store io: {error}"),
            Self::Sqlite(error) => write!(f, "work store sqlite: {error}"),
            Self::Journal(error) => write!(f, "work attempt journal: {error}"),
            Self::UnsupportedSchema(version) => {
                write!(f, "work store schema version {version} is newer than supported")
            }
            Self::NotFound(id) => write!(f, "work item not found: {id}"),
            Self::InvalidState(message) => write!(f, "invalid work transition: {message}"),
        }
    }
}

impl std::error::Error for WorkStoreError {}

impl From<rusqlite::Error> for WorkStoreError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Sqlite(value)
    }
}

impl From<JournalError> for WorkStoreError {
    fn from(value: JournalError) -> Self {
        Self::Journal(value)
    }
}

impl From<std::io::Error> for WorkStoreError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

type Result<T> = std::result::Result<T, WorkStoreError>;

/// Durable Work database shared by Desktop and CLI hosts.
pub struct WorkStore {
    connection: Mutex<Connection>,
}

impl WorkStore {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            let _ = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(false)
                .mode(0o600)
                .open(path)?;
        }
        let mut connection = Connection::open(path)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        migrate(&mut connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub fn ensure_project(&self, id: &str, name: &str, workspace_ref: &str) -> Result<()> {
        let now = now_ms();
        let connection = self.connection.lock().expect("work store mutex");
        connection.execute(
            r#"
            INSERT INTO projects (id, name, workspace_ref, created_at_ms, updated_at_ms)
            VALUES (?1, ?2, ?3, ?4, ?4)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                workspace_ref = excluded.workspace_ref,
                updated_at_ms = excluded.updated_at_ms
            "#,
            params![id, name, workspace_ref, now as i64],
        )?;
        Ok(())
    }

    pub fn create_work(&self, input: CreateWorkInput) -> Result<WorkItemRecord> {
        let title = input.title.trim();
        if title.is_empty() {
            return Err(WorkStoreError::InvalidState("title is required"));
        }
        let now = now_ms();
        let id = new_id("work");
        let mut connection = self.connection.lock().expect("work store mutex");
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute(
            r#"
            INSERT INTO work_items (
                id, project_id, title, description, acceptance_criteria, state,
                assignee_ref, blocker, revision, created_at_ms, updated_at_ms
            ) VALUES (?1, ?2, ?3, ?4, ?5, 'backlog', ?6, NULL, 1, ?7, ?7)
            "#,
            params![
                id,
                input.project_id,
                title,
                input.description,
                input.acceptance_criteria,
                input.assignee_ref,
                now as i64
            ],
        )?;
        append_event(&tx, &id, "created", "{}")?;
        tx.commit()?;
        drop(connection);
        self.get_work(&id)?
            .ok_or_else(|| WorkStoreError::NotFound(id))
    }

    pub fn get_work(&self, id: &str) -> Result<Option<WorkItemRecord>> {
        let connection = self.connection.lock().expect("work store mutex");
        let mut statement = connection.prepare(
            r#"
            SELECT id, project_id, title, description, acceptance_criteria, state,
                   assignee_ref, blocker, revision, created_at_ms, updated_at_ms
            FROM work_items WHERE id = ?1
            "#,
        )?;
        let row = statement
            .query_row(params![id], map_work_row)
            .optional()?;
        Ok(row)
    }

    pub fn get_attempt(&self, id: &str) -> Result<Option<AttemptRecord>> {
        let connection = self.connection.lock().expect("work store mutex");
        let mut statement = connection.prepare(
            r#"
            SELECT id, work_id, number, role, phase, chat_id, session_id,
                   run_id, input_json, result_json, created_at_ms, updated_at_ms
            FROM attempts WHERE id = ?1
            "#,
        )?;
        Ok(statement
            .query_row(params![id], map_attempt_row)
            .optional()?)
    }

    pub fn list_attempts(&self, work_id: &str) -> Result<Vec<AttemptRecord>> {
        let connection = self.connection.lock().expect("work store mutex");
        let mut statement = connection.prepare(
            r#"
            SELECT id, work_id, number, role, phase, chat_id, session_id,
                   run_id, input_json, result_json, created_at_ms, updated_at_ms
            FROM attempts WHERE work_id = ?1
            ORDER BY number DESC
            "#,
        )?;
        let rows = statement.query_map(params![work_id], map_attempt_row)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn list_work(&self, project_id: &str, filter: WorkListFilter) -> Result<Vec<WorkItemRecord>> {
        let connection = self.connection.lock().expect("work store mutex");
        let (clause, params_vec) = filter.sql(project_id);
        let sql = format!(
            r#"
            SELECT id, project_id, title, description, acceptance_criteria, state,
                   assignee_ref, blocker, revision, created_at_ms, updated_at_ms
            FROM work_items
            WHERE {clause}
            ORDER BY updated_at_ms DESC
            "#
        );
        let mut statement = connection.prepare(&sql)?;
        let rows = statement.query_map(rusqlite::params_from_iter(params_vec), map_work_row)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// Project the M1 Work Inbox from authoritative Work and Attempt state.
    /// `blocked` is read-only here: it reflects the canonical Work owner's
    /// `work_items.blocker` value and opens that Work detail; this projection
    /// does not invent a renderer or Inbox-owned blocker setter.
    ///
    /// Approval and question are valid public kinds, but this query does not
    /// synthesize them: the current approval surface is not ID-addressable and
    /// persisted clarification tickets do not carry the exact bound run id.
    /// They can be added only when a host can join an unresolved durable source
    /// to the exact bound Attempt without guessing from renderer state.
    pub fn list_work_inbox(&self, project_id: &str) -> Result<Vec<WorkInboxRecord>> {
        let connection = self.connection.lock().expect("work store mutex");
        let mut rows = Vec::new();

        {
            let mut statement = connection.prepare(
                r#"
                SELECT w.id, w.title, w.updated_at_ms,
                       a.id, a.chat_id, a.run_id, a.updated_at_ms
                FROM work_items w
                JOIN attempts a ON a.id = (
                    SELECT latest.id FROM attempts latest
                    WHERE latest.work_id = w.id
                    ORDER BY latest.number DESC
                    LIMIT 1
                )
                WHERE w.project_id = ?1
                  AND w.state = 'in_review'
                  AND a.phase = 'succeeded'
                "#,
            )?;
            let projected = statement.query_map(params![project_id], |row| {
                let work_id: String = row.get(0)?;
                let work_updated_at_ms: i64 = row.get(2)?;
                let attempt_id: String = row.get(3)?;
                let attempt_updated_at_ms: i64 = row.get(6)?;
                Ok(WorkInboxRecord {
                    id: format!("review_required:{work_id}"),
                    work_id,
                    kind: WorkInboxKind::ReviewRequired,
                    title: row.get(1)?,
                    why: "Attempt finished — decide Accept or Return".to_string(),
                    created_at_ms: attempt_updated_at_ms.max(work_updated_at_ms).max(0) as u64,
                    attempt_id: Some(attempt_id),
                    chat_id: row.get(4)?,
                    run_id: row.get(5)?,
                })
            })?;
            for row in projected {
                rows.push(row?);
            }
        }

        {
            let mut statement = connection.prepare(
                r#"
                SELECT w.id, w.title, a.id, a.chat_id, a.run_id, a.updated_at_ms
                FROM work_items w
                JOIN attempts a ON a.id = (
                    SELECT latest.id FROM attempts latest
                    WHERE latest.work_id = w.id
                    ORDER BY latest.number DESC
                    LIMIT 1
                )
                WHERE w.project_id = ?1
                  AND w.state = 'ready'
                  AND a.phase = 'failed'
                "#,
            )?;
            let projected = statement.query_map(params![project_id], |row| {
                let work_id: String = row.get(0)?;
                let attempt_id: String = row.get(2)?;
                let updated_at_ms: i64 = row.get(5)?;
                Ok(WorkInboxRecord {
                    id: format!("failed_attempt:{attempt_id}"),
                    work_id,
                    kind: WorkInboxKind::FailedAttempt,
                    title: row.get(1)?,
                    why: "Attempt failed — inspect evidence and retry".to_string(),
                    created_at_ms: updated_at_ms.max(0) as u64,
                    attempt_id: Some(attempt_id),
                    chat_id: row.get(3)?,
                    run_id: row.get(4)?,
                })
            })?;
            for row in projected {
                rows.push(row?);
            }
        }

        {
            let mut statement = connection.prepare(
                r#"
                SELECT id, title, blocker, updated_at_ms
                FROM work_items
                WHERE project_id = ?1
                  AND state NOT IN ('done', 'cancelled')
                  AND blocker IS NOT NULL
                  AND length(trim(blocker)) > 0
                "#,
            )?;
            let projected = statement.query_map(params![project_id], |row| {
                let work_id: String = row.get(0)?;
                let blocker: String = row.get(2)?;
                let updated_at_ms: i64 = row.get(3)?;
                Ok(WorkInboxRecord {
                    id: format!("blocked:{work_id}"),
                    work_id,
                    kind: WorkInboxKind::Blocked,
                    title: row.get(1)?,
                    why: format!("Blocked: {}", blocker.trim()),
                    created_at_ms: updated_at_ms.max(0) as u64,
                    attempt_id: None,
                    chat_id: None,
                    run_id: None,
                })
            })?;
            for row in projected {
                rows.push(row?);
            }
        }

        rows.sort_by(|left, right| {
            right
                .created_at_ms
                .cmp(&left.created_at_ms)
                .then_with(|| left.id.cmp(&right.id))
        });
        Ok(rows)
    }

    pub fn transition(&self, id: &str, expected_revision: i64, next: WorkState) -> Result<WorkItemRecord> {
        let now = now_ms();
        let mut connection = self.connection.lock().expect("work store mutex");
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current: (String, i64) = tx.query_row(
            "SELECT state, revision FROM work_items WHERE id = ?1",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        if current.1 != expected_revision {
            return Err(WorkStoreError::InvalidState("revision mismatch"));
        }
        let from = WorkState::parse(&current.0)
            .ok_or(WorkStoreError::InvalidState("unknown current state"))?;
        let has_active_attempt: bool = tx.query_row(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM attempts
                WHERE work_id = ?1 AND phase IN ('queued', 'running', 'waiting')
            )
            "#,
            params![id],
            |row| row.get(0),
        )?;
        if has_active_attempt {
            return Err(WorkStoreError::InvalidState(
                "an active attempt owns the Work transition",
            ));
        }
        if !is_allowed_generic_transition(from, next) {
            return Err(WorkStoreError::InvalidState(
                "transition requires its canonical Work lifecycle command",
            ));
        }
        tx.execute(
            r#"
            UPDATE work_items
            SET state = ?1, revision = revision + 1, updated_at_ms = ?2
            WHERE id = ?3
            "#,
            params![next.as_str(), now as i64, id],
        )?;
        let payload = format!(
            r#"{{"from":"{}","to":"{}"}}"#,
            from.as_str(),
            next.as_str()
        );
        append_event(&tx, id, "state_changed", &payload)?;
        tx.commit()?;
        drop(connection);
        self.get_work(id)?
            .ok_or_else(|| WorkStoreError::NotFound(id.to_string()))
    }

    /// Move Work into `in_progress` and open attempt N+1 (queued).
    /// Backlog is promoted through ready in the same transaction.
    pub fn start_attempt(&self, id: &str, expected_revision: i64) -> Result<WorkItemRecord> {
        Ok(self.start_attempt_with_record(id, expected_revision)?.work)
    }

    /// CLI-compatible start without a preallocated execution session.
    pub fn start_attempt_with_record(
        &self,
        id: &str,
        expected_revision: i64,
    ) -> Result<WorkAttemptStart> {
        self.start_attempt_with_dispatch(id, expected_revision, None, None)
    }

    /// Start an Attempt with its execution session durably recorded before
    /// dispatch. A renderer crash can then recover the acknowledged run from
    /// the workspace journal instead of relying on volatile IPC state.
    pub fn start_attempt_with_dispatch(
        &self,
        id: &str,
        expected_revision: i64,
        chat_id: Option<&str>,
        session_id: Option<&str>,
    ) -> Result<WorkAttemptStart> {
        let normalized_chat = chat_id.map(str::trim).filter(|value| !value.is_empty());
        let normalized_session = session_id.map(str::trim).filter(|value| !value.is_empty());
        if normalized_session.is_some() && normalized_chat.is_none() {
            return Err(WorkStoreError::InvalidState(
                "an Attempt session requires a chat id",
            ));
        }
        let now = now_ms();
        let mut connection = self.connection.lock().expect("work store mutex");
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current: (String, i64, String, String, String) = tx.query_row(
            r#"
            SELECT state, revision, title, description, acceptance_criteria
            FROM work_items WHERE id = ?1
            "#,
            params![id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )?;
        if current.1 != expected_revision {
            return Err(WorkStoreError::InvalidState("revision mismatch"));
        }
        let from = WorkState::parse(&current.0)
            .ok_or(WorkStoreError::InvalidState("unknown current state"))?;
        let active_attempt: Option<String> = tx
            .query_row(
                r#"
                SELECT id FROM attempts
                WHERE work_id = ?1 AND phase IN ('queued', 'running', 'waiting')
                LIMIT 1
                "#,
                params![id],
                |row| row.get(0),
            )
            .optional()?;
        if active_attempt.is_some() {
            return Err(WorkStoreError::InvalidState(
                "work already has an active attempt",
            ));
        }
        match from {
            WorkState::InReview => {
                return Err(WorkStoreError::InvalidState(
                    "work is already in review; Accept or Return first",
                ));
            }
            WorkState::Done | WorkState::Cancelled => {
                return Err(WorkStoreError::InvalidState(
                    "cannot start an attempt from a terminal state",
                ));
            }
            WorkState::InProgress => {
                return Err(WorkStoreError::InvalidState(
                    "work is already in progress",
                ));
            }
            WorkState::Backlog | WorkState::Ready => {}
        }

        if from == WorkState::Backlog {
            tx.execute(
                r#"
                UPDATE work_items
                SET state = 'ready', revision = revision + 1, updated_at_ms = ?1
                WHERE id = ?2
                "#,
                params![now as i64, id],
            )?;
            append_event(
                &tx,
                id,
                "state_changed",
                r#"{"from":"backlog","to":"ready"}"#,
            )?;
        }

        if from != WorkState::InProgress {
            let prior = if from == WorkState::Backlog {
                "ready"
            } else {
                from.as_str()
            };
            tx.execute(
                r#"
                UPDATE work_items
                SET state = 'in_progress', revision = revision + 1, updated_at_ms = ?1
                WHERE id = ?2
                "#,
                params![now as i64, id],
            )?;
            let payload = format!(r#"{{"from":"{prior}","to":"in_progress"}}"#);
            append_event(&tx, id, "state_changed", &payload)?;
        }

        let next_number: i64 = tx.query_row(
            "SELECT COALESCE(MAX(number), 0) + 1 FROM attempts WHERE work_id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        let attempt_id = new_id("attempt");
        let input_json = serde_json::json!({
            "title": current.2,
            "description": current.3,
            "acceptanceCriteria": current.4,
        })
        .to_string();
        tx.execute(
            r#"
            INSERT INTO attempts (
                id, work_id, number, role, phase, chat_id, session_id, input_json,
                created_at_ms, updated_at_ms
            ) VALUES (?1, ?2, ?3, 'executor', 'queued', ?4, ?5, ?6, ?7, ?7)
            "#,
            params![
                attempt_id,
                id,
                next_number,
                normalized_chat,
                normalized_session,
                input_json,
                now as i64
            ],
        )?;
        let payload = format!(r#"{{"attemptId":"{attempt_id}","number":{next_number}}}"#);
        append_event(&tx, id, "attempt_started", &payload)?;
        tx.commit()?;
        drop(connection);
        let work = self
            .get_work(id)?
            .ok_or_else(|| WorkStoreError::NotFound(id.to_string()))?;
        let attempt = self
            .get_attempt(&attempt_id)?
            .ok_or_else(|| WorkStoreError::NotFound(attempt_id.clone()))?;
        Ok(WorkAttemptStart { work, attempt })
    }

    /// Bind a queued Attempt to the exact execution identity acknowledged by
    /// IsanAgent. Repeating the same bind is a no-op; rebinding is rejected.
    pub fn bind_attempt_run(
        &self,
        attempt_id: &str,
        chat_id: &str,
        session_id: Option<&str>,
        run_id: &str,
    ) -> Result<AttemptRecord> {
        if chat_id.trim().is_empty() || run_id.trim().is_empty() {
            return Err(WorkStoreError::InvalidState(
                "chat and run ids are required",
            ));
        }
        let now = now_ms();
        let mut connection = self.connection.lock().expect("work store mutex");
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current: (
            String,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
        ) = tx.query_row(
            r#"
                SELECT work_id, phase, chat_id, session_id, run_id
                FROM attempts WHERE id = ?1
                "#,
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
        )?;
        let normalized_session = session_id.map(str::trim).filter(|value| !value.is_empty());
        if current.2.as_deref() == Some(chat_id)
            && current.3.as_deref() == normalized_session
            && current.4.as_deref() == Some(run_id)
        {
            tx.commit()?;
            drop(connection);
            return self
                .get_attempt(attempt_id)?
                .ok_or_else(|| WorkStoreError::NotFound(attempt_id.to_string()));
        }
        if current.4.is_some()
            || current.2.as_deref().is_some_and(|bound| bound != chat_id)
            || (current.2.is_some() && current.3.as_deref() != normalized_session)
        {
            return Err(WorkStoreError::InvalidState(
                "attempt is already bound to another run",
            ));
        }
        let phase = AttemptPhase::parse(&current.1)
            .ok_or(WorkStoreError::InvalidState("unknown attempt phase"))?;
        if phase != AttemptPhase::Queued {
            return Err(WorkStoreError::InvalidState(
                "only a queued attempt can bind a run",
            ));
        }
        let duplicate: Option<String> = tx
            .query_row(
                "SELECT id FROM attempts WHERE run_id = ?1 LIMIT 1",
                params![run_id],
                |row| row.get(0),
            )
            .optional()?;
        if duplicate.is_some() {
            return Err(WorkStoreError::InvalidState(
                "run is already bound to another attempt",
            ));
        }
        tx.execute(
            r#"
            UPDATE attempts
            SET phase = 'running', chat_id = ?1, session_id = ?2, run_id = ?3,
                updated_at_ms = ?4
            WHERE id = ?5
            "#,
            params![chat_id, normalized_session, run_id, now as i64, attempt_id],
        )?;
        let payload = serde_json::json!({
            "attemptId": attempt_id,
            "chatId": chat_id,
            "sessionId": normalized_session,
            "runId": run_id,
        })
        .to_string();
        append_event(&tx, &current.0, "attempt_run_bound", &payload)?;
        tx.commit()?;
        drop(connection);
        self.get_attempt(attempt_id)?
            .ok_or_else(|| WorkStoreError::NotFound(attempt_id.to_string()))
    }

    pub fn finish_attempt_by_id(
        &self,
        attempt_id: &str,
        phase: AttemptPhase,
        result_json: &str,
    ) -> Result<WorkItemRecord> {
        self.finish_attempt(Some(attempt_id), None, phase, result_json)?
            .ok_or_else(|| WorkStoreError::NotFound(attempt_id.to_string()))
    }

    /// Apply a typed run terminal. Unknown run ids are ignored because every
    /// agent run passes through the shared event bridge, not only Work runs.
    pub fn finish_attempt_by_run(
        &self,
        run_id: &str,
        phase: AttemptPhase,
        result_json: &str,
    ) -> Result<Option<WorkItemRecord>> {
        self.finish_attempt(None, Some(run_id), phase, result_json)
    }

    fn finish_attempt(
        &self,
        attempt_id: Option<&str>,
        run_id: Option<&str>,
        phase: AttemptPhase,
        result_json: &str,
    ) -> Result<Option<WorkItemRecord>> {
        if !matches!(
            phase,
            AttemptPhase::Succeeded | AttemptPhase::Failed | AttemptPhase::Cancelled
        ) {
            return Err(WorkStoreError::InvalidState(
                "attempt finish requires a terminal phase",
            ));
        }
        let (column, value) = match (attempt_id, run_id) {
            (Some(id), None) if !id.trim().is_empty() => ("id", id),
            (None, Some(id)) if !id.trim().is_empty() => ("run_id", id),
            _ => {
                return Err(WorkStoreError::InvalidState(
                    "exactly one attempt or run id is required",
                ))
            }
        };
        let now = now_ms();
        let mut connection = self.connection.lock().expect("work store mutex");
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let sql = format!("SELECT id, work_id, phase FROM attempts WHERE {column} = ?1 LIMIT 1");
        let current: Option<(String, String, String)> = tx
            .query_row(&sql, params![value], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .optional()?;
        let Some((resolved_attempt_id, work_id, current_phase_raw)) = current else {
            tx.commit()?;
            return Ok(None);
        };
        let current_phase = AttemptPhase::parse(&current_phase_raw)
            .ok_or(WorkStoreError::InvalidState("unknown attempt phase"))?;
        if matches!(
            current_phase,
            AttemptPhase::Succeeded | AttemptPhase::Failed | AttemptPhase::Cancelled
        ) {
            if current_phase != phase {
                return Err(WorkStoreError::InvalidState(
                    "attempt already finished with another phase",
                ));
            }
            tx.commit()?;
            drop(connection);
            return self.get_work(&work_id);
        }

        let work_state_raw: String = tx.query_row(
            "SELECT state FROM work_items WHERE id = ?1",
            params![work_id],
            |row| row.get(0),
        )?;
        let work_state = WorkState::parse(&work_state_raw)
            .ok_or(WorkStoreError::InvalidState("unknown current state"))?;
        if work_state != WorkState::InProgress {
            return Err(WorkStoreError::InvalidState(
                "only in_progress work can finish an attempt",
            ));
        }
        tx.execute(
            r#"
            UPDATE attempts
            SET phase = ?1, result_json = ?2, updated_at_ms = ?3
            WHERE id = ?4
            "#,
            params![phase.as_str(), result_json, now as i64, resolved_attempt_id],
        )?;
        let next = if phase == AttemptPhase::Succeeded {
            WorkState::InReview
        } else {
            WorkState::Ready
        };
        tx.execute(
            r#"
            UPDATE work_items
            SET state = ?1, revision = revision + 1, updated_at_ms = ?2
            WHERE id = ?3
            "#,
            params![next.as_str(), now as i64, work_id],
        )?;
        let payload = serde_json::json!({
            "attemptId": resolved_attempt_id,
            "phase": phase.as_str(),
            "from": work_state.as_str(),
            "to": next.as_str(),
        })
        .to_string();
        append_event(&tx, &work_id, "attempt_finished", &payload)?;
        tx.commit()?;
        drop(connection);
        self.get_work(&work_id)
    }

    /// Reconcile every active Attempt against the workspace-owned durable
    /// agent journal. The host must open the journal through its long-lived
    /// workspace services first so inherited runs are restart-classified once.
    /// Only the process-lifetime `RestartRecovery` pass may fail a prebound
    /// Attempt with no journal run; live passes tolerate cold dispatch latency.
    pub fn reconcile_attempts_from_journal(
        &self,
        journal: &EventJournal,
        mode: AttemptReconcileMode,
    ) -> Result<Vec<String>> {
        let attempts = self.list_active_attempts()?;
        let mut changed_work_ids = Vec::new();
        for attempt in attempts {
            let Some(chat_id) = attempt.chat_id.as_deref() else {
                // CLI-created Attempts intentionally remain manually managed.
                continue;
            };
            let summary = if let Some(run_id) = attempt.run_id.as_deref() {
                journal.run_summary(run_id)?
            } else {
                journal.latest_run_summary_for_chat(chat_id)?
            };
            let Some(summary) = summary else {
                if mode == AttemptReconcileMode::RestartRecovery {
                    self.finish_attempt_by_id(
                        &attempt.id,
                        AttemptPhase::Failed,
                        &serde_json::json!({
                            "kind": "failed",
                            "failure": "No durable agent run was recorded for this Attempt.",
                            "retryable": true,
                        })
                        .to_string(),
                    )?;
                    push_unique(&mut changed_work_ids, &attempt.work_id);
                }
                continue;
            };
            if summary.chat_id != chat_id {
                return Err(WorkStoreError::InvalidState(
                    "journal run does not belong to the Attempt chat",
                ));
            }
            if let Some(bound_run_id) = attempt.run_id.as_deref() {
                if bound_run_id != summary.run_id {
                    return Err(WorkStoreError::InvalidState(
                        "journal run does not match the Attempt binding",
                    ));
                }
            } else {
                self.bind_attempt_run(
                    &attempt.id,
                    chat_id,
                    attempt.session_id.as_deref(),
                    &summary.run_id,
                )?;
                push_unique(&mut changed_work_ids, &attempt.work_id);
            }
            if let Some((phase, result_json)) = terminal_attempt_outcome(&summary)? {
                self.finish_attempt_by_run(&summary.run_id, phase, &result_json)?;
                push_unique(&mut changed_work_ids, &attempt.work_id);
            }
        }
        Ok(changed_work_ids)
    }

    fn list_active_attempts(&self) -> Result<Vec<AttemptRecord>> {
        let connection = self.connection.lock().expect("work store mutex");
        let mut statement = connection.prepare(
            r#"
            SELECT id, work_id, number, role, phase, chat_id, session_id,
                   run_id, input_json, result_json, created_at_ms, updated_at_ms
            FROM attempts
            WHERE phase IN ('queued', 'running', 'waiting')
            ORDER BY created_at_ms ASC, id ASC
            "#,
        )?;
        let rows = statement.query_map([], map_attempt_row)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// Mark the latest attempt succeeded and move Work to `in_review`.
    /// Does **not** mark Work done — human Accept/Return decides.
    pub fn mark_attempt_ready_for_review(
        &self,
        id: &str,
        expected_revision: i64,
    ) -> Result<WorkItemRecord> {
        let now = now_ms();
        let mut connection = self.connection.lock().expect("work store mutex");
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current: (String, i64) = tx.query_row(
            "SELECT state, revision FROM work_items WHERE id = ?1",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        if current.1 != expected_revision {
            return Err(WorkStoreError::InvalidState("revision mismatch"));
        }
        let from = WorkState::parse(&current.0)
            .ok_or(WorkStoreError::InvalidState("unknown current state"))?;
        if from != WorkState::InProgress {
            return Err(WorkStoreError::InvalidState(
                "only in_progress work can enter review",
            ));
        }
        let attempt: (String, String, Option<String>, Option<String>, Option<String>) = tx.query_row(
            r#"
            SELECT id, phase, chat_id, session_id, run_id
            FROM attempts WHERE work_id = ?1
            ORDER BY number DESC LIMIT 1
            "#,
            params![id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )?;
        if attempt.1 != AttemptPhase::Queued.as_str()
            || attempt.2.is_some()
            || attempt.3.is_some()
            || attempt.4.is_some()
        {
            return Err(WorkStoreError::InvalidState(
                "a bound Attempt can only finish from its typed run terminal",
            ));
        }
        tx.execute(
            r#"
            UPDATE attempts
            SET phase = 'succeeded', updated_at_ms = ?1
            WHERE id = ?2
            "#,
            params![now as i64, attempt.0],
        )?;
        tx.execute(
            r#"
            UPDATE work_items
            SET state = 'in_review', revision = revision + 1, updated_at_ms = ?1
            WHERE id = ?2
            "#,
            params![now as i64, id],
        )?;
        append_event(
            &tx,
            id,
            "state_changed",
            r#"{"from":"in_progress","to":"in_review"}"#,
        )?;
        tx.commit()?;
        drop(connection);
        self.get_work(id)?
            .ok_or_else(|| WorkStoreError::NotFound(id.to_string()))
    }

    /// Human Accept (`done`) or Return (`ready`) for Work in `in_review`.
    pub fn human_review(
        &self,
        id: &str,
        expected_revision: i64,
        accept: bool,
        guidance: &str,
    ) -> Result<WorkItemRecord> {
        if !accept && guidance.trim().is_empty() {
            return Err(WorkStoreError::InvalidState(
                "Return requires human guidance",
            ));
        }
        let now = now_ms();
        let mut connection = self.connection.lock().expect("work store mutex");
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current: (String, i64) = tx.query_row(
            "SELECT state, revision FROM work_items WHERE id = ?1",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let next = if accept {
            WorkState::Done
        } else {
            WorkState::Ready
        };
        let status = if accept { "complete" } else { "incomplete" };
        if expected_revision.checked_add(1) == Some(current.1) && current.0 == next.as_str() {
            let latest_review: Option<(String, String, i64, String)> = tx
                .query_row(
                    r#"
                SELECT review.id, review.status, review.acceptance_aligned, review.guidance
                FROM reviews AS review
                WHERE review.reviewer_kind = 'human'
                  AND review.attempt_id = (
                      SELECT attempt.id
                      FROM attempts AS attempt
                      WHERE attempt.work_id = ?1
                      ORDER BY attempt.number DESC
                      LIMIT 1
                  )
                ORDER BY review.created_at_ms DESC, review.id DESC
                LIMIT 1
                "#,
                    params![id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )
                .optional()?;
            let latest_event: Option<(String, String)> = tx
                .query_row(
                    r#"
                    SELECT kind, payload_json
                    FROM work_events
                    WHERE work_id = ?1
                    ORDER BY id DESC
                    LIMIT 1
                    "#,
                    params![id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?;
            let matching_retry = latest_review.as_ref().is_some_and(
                |(review_id, latest_status, acceptance_aligned, latest_guidance)| {
                    latest_status == status
                        && *acceptance_aligned == if accept { 1 } else { 0 }
                        && latest_guidance == guidance
                        && latest_event
                            .as_ref()
                            .is_some_and(|(event_kind, payload_json)| {
                                if event_kind != if accept { "accepted" } else { "returned" } {
                                    return false;
                                }
                                serde_json::from_str::<serde_json::Value>(payload_json)
                                    .ok()
                                    .is_some_and(|payload| {
                                        payload.get("reviewId").and_then(serde_json::Value::as_str)
                                            == Some(review_id.as_str())
                                            && payload.get("to").and_then(serde_json::Value::as_str)
                                                == Some(next.as_str())
                                            && payload
                                                .get("expectedRevision")
                                                .and_then(serde_json::Value::as_i64)
                                                == Some(expected_revision)
                                            && payload
                                                .get("resultRevision")
                                                .and_then(serde_json::Value::as_i64)
                                                == Some(current.1)
                                    })
                            })
                },
            );
            if matching_retry {
                drop(tx);
                drop(connection);
                return self
                    .get_work(id)?
                    .ok_or_else(|| WorkStoreError::NotFound(id.to_string()));
            }
        }
        if current.1 != expected_revision {
            return Err(WorkStoreError::InvalidState("revision mismatch"));
        }
        let from = WorkState::parse(&current.0)
            .ok_or(WorkStoreError::InvalidState("unknown current state"))?;
        if from != WorkState::InReview {
            return Err(WorkStoreError::InvalidState(
                "only in_review work can be Accepted or Returned",
            ));
        }
        let attempt_id: String = tx.query_row(
            r#"
            SELECT id FROM attempts WHERE work_id = ?1
            ORDER BY number DESC LIMIT 1
            "#,
            params![id],
            |row| row.get(0),
        )?;
        let review_id = new_id("review");
        tx.execute(
            r#"
            INSERT INTO reviews (
                id, attempt_id, reviewer_kind, status, integrity,
                acceptance_aligned, evidence_json, missing_json, guidance, created_at_ms
            ) VALUES (?1, ?2, 'human', ?3, 'ok', ?4, '[]', '[]', ?5, ?6)
            "#,
            params![
                review_id,
                attempt_id,
                status,
                if accept { 1 } else { 0 },
                guidance,
                now as i64
            ],
        )?;
        tx.execute(
            r#"
            UPDATE work_items
            SET state = ?1, revision = revision + 1, updated_at_ms = ?2
            WHERE id = ?3
            "#,
            params![next.as_str(), now as i64, id],
        )?;
        let kind = if accept { "accepted" } else { "returned" };
        let result_revision = expected_revision + 1;
        let payload = serde_json::json!({
            "reviewId": review_id,
            "to": next.as_str(),
            "expectedRevision": expected_revision,
            "resultRevision": result_revision,
        });
        append_event(&tx, id, kind, &payload.to_string())?;
        tx.commit()?;
        drop(connection);
        self.get_work(id)?
            .ok_or_else(|| WorkStoreError::NotFound(id.to_string()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkListFilter {
    MyActive,
    Review,
    Backlog,
    Done,
}

impl WorkListFilter {
    fn sql(self, project_id: &str) -> (String, Vec<String>) {
        match self {
            Self::MyActive => (
                "project_id = ?1 AND state IN ('ready', 'in_progress', 'in_review')".into(),
                vec![project_id.to_string()],
            ),
            Self::Review => (
                "project_id = ?1 AND state = 'in_review'".into(),
                vec![project_id.to_string()],
            ),
            Self::Backlog => (
                "project_id = ?1 AND state = 'backlog'".into(),
                vec![project_id.to_string()],
            ),
            Self::Done => (
                "project_id = ?1 AND state = 'done'".into(),
                vec![project_id.to_string()],
            ),
        }
    }
}

fn terminal_attempt_outcome(
    summary: &RunJournalSummary,
) -> Result<Option<(AttemptPhase, String)>> {
    if summary.terminal_seq.is_none() {
        return Ok(None);
    }
    if summary.terminal_kind.as_deref() != Some("run_terminated") {
        return Err(WorkStoreError::InvalidState(
            "journal terminal is not a typed run termination",
        ));
    }
    let outcome = summary
        .terminal_payload
        .as_ref()
        .and_then(|payload| payload.get("outcome"))
        .ok_or(WorkStoreError::InvalidState(
            "journal run termination has no outcome",
        ))?;
    let kind = outcome
        .get("kind")
        .and_then(serde_json::Value::as_str)
        .ok_or(WorkStoreError::InvalidState(
            "journal run termination has no outcome kind",
        ))?;
    let phase = match kind {
        "completed" => AttemptPhase::Succeeded,
        "cancelled" => AttemptPhase::Cancelled,
        "failed" | "stuck" | "budget_exhausted" => AttemptPhase::Failed,
        _ => {
            return Err(WorkStoreError::InvalidState(
                "journal run termination has an unknown outcome kind",
            ))
        }
    };
    Ok(Some((phase, outcome.to_string())))
}

fn push_unique(values: &mut Vec<String>, value: &str) {
    if !values.iter().any(|current| current == value) {
        values.push(value.to_string());
    }
}

/// Generic human-controlled state changes. Attempt and Review lifecycle edges
/// are deliberately absent: callers must use start/finish/review so the
/// corresponding durable records and evidence are updated atomically.
fn is_allowed_generic_transition(from: WorkState, to: WorkState) -> bool {
    use WorkState::*;
    matches!(
        (from, to),
        (Backlog, Ready)
            | (Ready, Cancelled)
            | (InProgress, Cancelled)
            | (InReview, Cancelled)
            | (Done, Ready)
            | (Cancelled, Backlog)
            | (Backlog, Cancelled)
    )
}

fn map_work_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<WorkItemRecord> {
    let state_raw: String = row.get(5)?;
    let state = WorkState::parse(&state_raw).unwrap_or(WorkState::Backlog);
    Ok(WorkItemRecord {
        id: row.get(0)?,
        project_id: row.get(1)?,
        title: row.get(2)?,
        description: row.get(3)?,
        acceptance_criteria: row.get(4)?,
        state,
        assignee_ref: row.get(6)?,
        blocker: row.get(7)?,
        revision: row.get(8)?,
        created_at_ms: row.get::<_, i64>(9)? as u64,
        updated_at_ms: row.get::<_, i64>(10)? as u64,
    })
}

fn map_attempt_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AttemptRecord> {
    let phase_raw: String = row.get(4)?;
    let phase = AttemptPhase::parse(&phase_raw).unwrap_or(AttemptPhase::Queued);
    Ok(AttemptRecord {
        id: row.get(0)?,
        work_id: row.get(1)?,
        number: row.get(2)?,
        role: row.get(3)?,
        phase,
        chat_id: row.get(5)?,
        session_id: row.get(6)?,
        run_id: row.get(7)?,
        input_json: row.get(8)?,
        result_json: row.get(9)?,
        created_at_ms: row.get::<_, i64>(10)? as u64,
        updated_at_ms: row.get::<_, i64>(11)? as u64,
    })
}

fn append_event(
    tx: &rusqlite::Transaction<'_>,
    work_id: &str,
    kind: &str,
    payload_json: &str,
) -> Result<()> {
    tx.execute(
        r#"
        INSERT INTO work_events (work_id, kind, payload_json, created_at_ms)
        VALUES (?1, ?2, ?3, ?4)
        "#,
        params![work_id, kind, payload_json, now_ms() as i64],
    )?;
    Ok(())
}

fn migrate(connection: &mut Connection) -> Result<()> {
    let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    tx.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS work_store_migrations (
            version INTEGER PRIMARY KEY,
            applied_at_ms INTEGER NOT NULL
        );
        "#,
    )?;
    let current: i64 = tx
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM work_store_migrations",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if current > SCHEMA_VERSION {
        return Err(WorkStoreError::UnsupportedSchema(current));
    }
    if current < 1 {
        tx.execute_batch(MIGRATION_V1)?;
        tx.execute(
            "INSERT INTO work_store_migrations (version, applied_at_ms) VALUES (1, ?1)",
            params![now_ms() as i64],
        )?;
    }
    if current < 2 {
        // Repair duplicate active Attempts created by the pre-v2 InProgress
        // retry bug, preserving only the newest Attempt as the recovery owner.
        tx.execute(
            r#"
            UPDATE attempts
            SET phase = 'failed',
                result_json = COALESCE(
                    result_json,
                    '{"kind":"failed","failure":"Superseded by a newer active Attempt during schema repair.","retryable":true}'
                ),
                updated_at_ms = ?1
            WHERE phase IN ('queued', 'running', 'waiting')
              AND EXISTS (
                  SELECT 1 FROM attempts AS newer
                  WHERE newer.work_id = attempts.work_id
                    AND newer.phase IN ('queued', 'running', 'waiting')
                    AND newer.number > attempts.number
              )
            "#,
            params![now_ms() as i64],
        )?;
        tx.execute_batch(MIGRATION_V2)?;
        tx.execute(
            "INSERT INTO work_store_migrations (version, applied_at_ms) VALUES (2, ?1)",
            params![now_ms() as i64],
        )?;
    }
    tx.commit()?;
    Ok(())
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64
}

fn new_id(prefix: &str) -> String {
    let millis = now_ms();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let sequence = NEXT_ID_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let nonce = nanos ^ sequence.rotate_left(17);
    format!("{prefix}_{millis}_{nonce:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::JournalEvent;
    use std::sync::{Arc, Barrier};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_db() -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let sequence = NEXT_ID_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("altai-work-store-{nanos}-{sequence}.db"))
    }

    #[test]
    fn create_list_and_transition_work() {
        let path = temp_db();
        let store = WorkStore::open(&path).expect("open");
        store
            .ensure_project("proj_1", "Demo", "/tmp/demo")
            .expect("project");
        let created = store
            .create_work(CreateWorkInput {
                project_id: "proj_1".into(),
                title: "Ship Work list".into(),
                description: "Build the list screen".into(),
                acceptance_criteria: "List filters work".into(),
                assignee_ref: None,
            })
            .expect("create");
        assert_eq!(created.state, WorkState::Backlog);
        assert_eq!(created.title, "Ship Work list");

        let listed = store
            .list_work("proj_1", WorkListFilter::Backlog)
            .expect("list");
        assert_eq!(listed.len(), 1);

        let ready = store
            .transition(&created.id, created.revision, WorkState::Ready)
            .expect("ready");
        assert_eq!(ready.state, WorkState::Ready);
        assert_eq!(ready.revision, created.revision + 1);

        let active = store
            .list_work("proj_1", WorkListFilter::MyActive)
            .expect("active");
        assert_eq!(active.len(), 1);

        let started = store
            .start_attempt(&ready.id, ready.revision)
            .expect("start");
        assert_eq!(started.state, WorkState::InProgress);

        let in_review = store
            .mark_attempt_ready_for_review(&started.id, started.revision)
            .expect("review-ready");
        assert_eq!(in_review.state, WorkState::InReview);

        let accepted = store
            .human_review(&in_review.id, in_review.revision, true, "")
            .expect("accept");
        assert_eq!(accepted.state, WorkState::Done);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn generic_transition_cannot_manufacture_attempt_or_review_lifecycle() {
        let path = temp_db();
        let store = WorkStore::open(&path).expect("open");
        store
            .ensure_project("proj_1", "Demo", "/tmp/demo")
            .expect("project");
        let created = store
            .create_work(CreateWorkInput {
                project_id: "proj_1".into(),
                title: "Keep lifecycle records atomic".into(),
                description: String::new(),
                acceptance_criteria: "Generic transitions never forge Attempt or Review state"
                    .into(),
                assignee_ref: None,
            })
            .expect("create");
        let ready = store
            .transition(&created.id, created.revision, WorkState::Ready)
            .expect("backlog to ready is generic");

        assert!(matches!(
            store.transition(&ready.id, ready.revision, WorkState::InProgress),
            Err(WorkStoreError::InvalidState(
                "transition requires its canonical Work lifecycle command"
            ))
        ));
        assert!(store.list_attempts(&ready.id).expect("attempts").is_empty());
        assert_eq!(store.get_work(&ready.id).expect("get"), Some(ready.clone()));

        let started = store
            .start_attempt(&ready.id, ready.revision)
            .expect("canonical start");
        assert!(store
            .transition(&started.id, started.revision, WorkState::InReview)
            .is_err());
        assert!(store
            .transition(&started.id, started.revision, WorkState::Ready)
            .is_err());
        let in_review = store
            .mark_attempt_ready_for_review(&started.id, started.revision)
            .expect("canonical review-ready");
        assert!(matches!(
            store.transition(&in_review.id, in_review.revision, WorkState::Done),
            Err(WorkStoreError::InvalidState(
                "transition requires its canonical Work lifecycle command"
            ))
        ));
        assert!(matches!(
            store.transition(&in_review.id, in_review.revision, WorkState::Ready),
            Err(WorkStoreError::InvalidState(
                "transition requires its canonical Work lifecycle command"
            ))
        ));
        let review_count_before_decision: i64 = store
            .connection
            .lock()
            .expect("connection")
            .query_row(
                r#"
                SELECT COUNT(*) FROM reviews AS review
                JOIN attempts AS attempt ON attempt.id = review.attempt_id
                WHERE attempt.work_id = ?1
                "#,
                params![created.id],
                |row| row.get(0),
            )
            .expect("review count before canonical decision");
        assert_eq!(review_count_before_decision, 0);

        let done = store
            .human_review(&in_review.id, in_review.revision, true, "Accepted")
            .expect("canonical review");
        let reopened = store
            .transition(&done.id, done.revision, WorkState::Ready)
            .expect("done can reopen to ready");
        let cancelled = store
            .transition(&reopened.id, reopened.revision, WorkState::Cancelled)
            .expect("ready can be cancelled");
        let backlog = store
            .transition(&cancelled.id, cancelled.revision, WorkState::Backlog)
            .expect("cancelled can reopen to backlog");
        assert_eq!(backlog.state, WorkState::Backlog);

        let connection = store.connection.lock().expect("connection");
        let (attempt_count, review_count): (i64, i64) = connection
            .query_row(
                r#"
                SELECT
                    (SELECT COUNT(*) FROM attempts WHERE work_id = ?1),
                    (SELECT COUNT(*) FROM reviews AS review
                     JOIN attempts AS attempt ON attempt.id = review.attempt_id
                     WHERE attempt.work_id = ?1)
                "#,
                params![created.id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("lifecycle counts");
        assert_eq!((attempt_count, review_count), (1, 1));
        drop(connection);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn work_inbox_projects_only_actionable_source_conditions() {
        let path = temp_db();
        let store = WorkStore::open(&path).expect("open");
        store
            .ensure_project("proj_1", "Demo", "/tmp/demo")
            .expect("project");

        let review = store
            .create_work(CreateWorkInput {
                project_id: "proj_1".into(),
                title: "Review evidence".into(),
                description: String::new(),
                acceptance_criteria: "Tests pass".into(),
                assignee_ref: None,
            })
            .expect("review work");
        let review_started = store
            .start_attempt_with_record(&review.id, review.revision)
            .expect("review attempt");
        let review_ready = store
            .mark_attempt_ready_for_review(&review.id, review_started.work.revision)
            .expect("review ready");

        let failed = store
            .create_work(CreateWorkInput {
                project_id: "proj_1".into(),
                title: "Retry failure".into(),
                description: String::new(),
                acceptance_criteria: String::new(),
                assignee_ref: None,
            })
            .expect("failed work");
        let failed_started = store
            .start_attempt_with_record(&failed.id, failed.revision)
            .expect("failed attempt");
        let failed_ready = store
            .finish_attempt_by_id(
                &failed_started.attempt.id,
                AttemptPhase::Failed,
                r#"{"kind":"failed"}"#,
            )
            .expect("finish failed attempt");

        store
            .ensure_project("proj_2", "Other", "/tmp/other")
            .expect("other project");
        let other = store
            .create_work(CreateWorkInput {
                project_id: "proj_2".into(),
                title: "Other project review".into(),
                description: String::new(),
                acceptance_criteria: String::new(),
                assignee_ref: None,
            })
            .expect("other project work");
        let other_started = store
            .start_attempt_with_record(&other.id, other.revision)
            .expect("other project attempt");
        store
            .mark_attempt_ready_for_review(&other.id, other_started.work.revision)
            .expect("other project review ready");

        store
            .connection
            .lock()
            .expect("connection")
            .execute(
                "UPDATE work_items SET blocker = ?1 WHERE id = ?2",
                params!["Need a human decision", review.id],
            )
            .expect("set blocker source");
        {
            let connection = store.connection.lock().expect("connection");
            connection
                .execute(
                    "UPDATE attempts SET updated_at_ms = 200 WHERE id = ?1",
                    params![review_started.attempt.id],
                )
                .expect("set review ordering time");
            connection
                .execute(
                    "UPDATE work_items SET updated_at_ms = 200 WHERE id = ?1",
                    params![review.id],
                )
                .expect("set blocker ordering time");
            connection
                .execute(
                    "UPDATE attempts SET updated_at_ms = 300 WHERE id = ?1",
                    params![failed_started.attempt.id],
                )
                .expect("set failure ordering time");
        }

        let rows = store.list_work_inbox("proj_1").expect("project Inbox");
        assert_eq!(rows.len(), 3);
        assert_eq!(
            rows.iter().map(|row| row.kind).collect::<Vec<_>>(),
            vec![
                WorkInboxKind::FailedAttempt,
                WorkInboxKind::Blocked,
                WorkInboxKind::ReviewRequired,
            ]
        );
        assert!(rows.iter().all(|row| row.work_id != other.id));
        assert!(rows.iter().any(|row| {
            row.kind == WorkInboxKind::ReviewRequired
                && row.work_id == review.id
                && row.attempt_id.as_deref() == Some(review_started.attempt.id.as_str())
        }));
        assert!(rows.iter().any(|row| {
            row.kind == WorkInboxKind::FailedAttempt
                && row.work_id == failed.id
                && row.attempt_id.as_deref() == Some(failed_started.attempt.id.as_str())
        }));
        assert!(rows.iter().any(|row| {
            row.kind == WorkInboxKind::Blocked
                && row.work_id == review.id
                && row.why == "Blocked: Need a human decision"
        }));
        assert!(rows.iter().all(|row| {
            row.kind != WorkInboxKind::Approval && row.kind != WorkInboxKind::Question
        }));

        let returned = store
            .human_review(&review.id, review_ready.revision, false, "Address the blocker")
            .expect("return review");
        store
            .start_attempt(&failed.id, failed_ready.revision)
            .expect("retry failed work");

        let remaining = store
            .list_work_inbox("proj_1")
            .expect("partially resolved Inbox");
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].kind, WorkInboxKind::Blocked);
        assert_eq!(remaining[0].work_id, review.id);

        store
            .transition(&review.id, returned.revision, WorkState::Cancelled)
            .expect("cancel blocked Work");

        assert!(store
            .list_work_inbox("proj_1")
            .expect("resolved Inbox")
            .is_empty());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn return_keeps_work_undone() {
        let path = temp_db();
        let store = WorkStore::open(&path).expect("open");
        store
            .ensure_project("proj_1", "Demo", "/tmp/demo")
            .expect("project");
        let created = store
            .create_work(CreateWorkInput {
                project_id: "proj_1".into(),
                title: "Needs return".into(),
                description: String::new(),
                acceptance_criteria: "tests".into(),
                assignee_ref: None,
            })
            .expect("create");
        let started = store
            .start_attempt(&created.id, created.revision)
            .expect("start");
        let in_review = store
            .mark_attempt_ready_for_review(&started.id, started.revision)
            .expect("review-ready");
        let returned = store
            .human_review(&in_review.id, in_review.revision, false, "add tests")
            .expect("return");
        assert_eq!(returned.state, WorkState::Ready);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn human_review_retries_are_idempotent_and_guidance_sensitive() {
        let path = temp_db();
        let store = WorkStore::open(&path).expect("open");
        store
            .ensure_project("proj_1", "Demo", "/tmp/demo")
            .expect("project");
        let created = store
            .create_work(CreateWorkInput {
                project_id: "proj_1".into(),
                title: "Retry the human decision".into(),
                description: String::new(),
                acceptance_criteria: "One auditable review per decision".into(),
                assignee_ref: None,
            })
            .expect("create");
        let started = store
            .start_attempt(&created.id, created.revision)
            .expect("start");
        let first_review = store
            .mark_attempt_ready_for_review(&started.id, started.revision)
            .expect("review-ready");

        assert!(store
            .human_review(&first_review.id, first_review.revision, false, "   ")
            .is_err());
        let returned = store
            .human_review(
                &first_review.id,
                first_review.revision,
                false,
                "Add restart coverage",
            )
            .expect("return");
        let repeated_return = store
            .human_review(
                &first_review.id,
                first_review.revision,
                false,
                "Add restart coverage",
            )
            .expect("same Return retry");
        assert_eq!(repeated_return, returned);
        assert!(store
            .human_review(
                &first_review.id,
                first_review.revision,
                false,
                "Different guidance",
            )
            .is_err());

        {
            let connection = store.connection.lock().expect("connection");
            let review_id: String = connection
                .query_row(
                    r#"
                    SELECT review.id FROM reviews AS review
                    JOIN attempts AS attempt ON attempt.id = review.attempt_id
                    WHERE attempt.work_id = ?1
                    ORDER BY review.created_at_ms DESC, review.id DESC
                    LIMIT 1
                    "#,
                    params![first_review.id],
                    |row| row.get(0),
                )
                .expect("latest human review id");
            let legacy_payload = serde_json::json!({
                "reviewId": review_id,
                "to": "ready",
            })
            .to_string();
            connection
                .execute(
                r#"
                UPDATE work_events
                SET payload_json = ?1
                WHERE id = (
                    SELECT id FROM work_events
                    WHERE work_id = ?2 AND kind = 'returned'
                    ORDER BY id DESC LIMIT 1
                )
                "#,
                    params![legacy_payload, first_review.id],
                )
                .expect("replace decision payload with the pre-revision shape");
        }
        assert!(store
            .human_review(
                &first_review.id,
                first_review.revision,
                false,
                "Add restart coverage",
            )
            .is_err());
        assert!(store
            .human_review(
                &first_review.id,
                first_review.revision,
                true,
                "Add restart coverage",
            )
            .is_err());

        let second_started = store
            .start_attempt(&returned.id, returned.revision)
            .expect("retry Attempt");
        let second_review = store
            .mark_attempt_ready_for_review(&second_started.id, second_started.revision)
            .expect("second review-ready");
        let accepted = store
            .human_review(&second_review.id, second_review.revision, true, "Evidence accepted")
            .expect("accept");
        let repeated_accept = store
            .human_review(&second_review.id, second_review.revision, true, "Evidence accepted")
            .expect("same Accept retry");
        assert_eq!(repeated_accept, accepted);
        assert_eq!(accepted.state, WorkState::Done);

        let connection = store.connection.lock().expect("work store mutex");
        let review_count: i64 = connection
            .query_row(
                r#"
                SELECT COUNT(*)
                FROM reviews AS review
                JOIN attempts AS attempt ON attempt.id = review.attempt_id
                WHERE attempt.work_id = ?1
                "#,
                params![created.id],
                |row| row.get(0),
            )
            .expect("review count");
        let decision_event_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM work_events WHERE work_id = ?1 AND kind IN ('returned', 'accepted')",
                params![created.id],
                |row| row.get(0),
            )
            .expect("decision event count");
        assert_eq!(review_count, 2);
        assert_eq!(decision_event_count, 2);
        drop(connection);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn human_review_retry_rejects_a_generic_transition_cycle_with_the_same_shape() {
        let path = temp_db();
        let store = WorkStore::open(&path).expect("open");
        store
            .ensure_project("proj_1", "Demo", "/tmp/demo")
            .expect("project");
        let created = store
            .create_work(CreateWorkInput {
                project_id: "proj_1".into(),
                title: "Do not confuse transition history".into(),
                description: String::new(),
                acceptance_criteria: "Only the matching decision event is retryable".into(),
                assignee_ref: None,
            })
            .expect("create");
        let started = store
            .start_attempt(&created.id, created.revision)
            .expect("start");
        let in_review = store
            .mark_attempt_ready_for_review(&started.id, started.revision)
            .expect("review-ready");
        let returned = store
            .human_review(
                &in_review.id,
                in_review.revision,
                false,
                "Keep this exact guidance",
            )
            .expect("return");

        let generic_cancelled = store
            .transition(&returned.id, returned.revision, WorkState::Cancelled)
            .expect("generic cancel");
        let generic_backlog = store
            .transition(
                &generic_cancelled.id,
                generic_cancelled.revision,
                WorkState::Backlog,
            )
            .expect("generic reopen to backlog");
        let generic_ready = store
            .transition(
                &generic_backlog.id,
                generic_backlog.revision,
                WorkState::Ready,
            )
            .expect("generic backlog to ready");

        let retry = store.human_review(
            &generic_ready.id,
            generic_ready.revision - 1,
            false,
            "Keep this exact guidance",
        );
        assert!(matches!(
            retry,
            Err(WorkStoreError::InvalidState("revision mismatch"))
        ));

        let connection = store.connection.lock().expect("connection");
        let review_count: i64 = connection
            .query_row(
                r#"
                SELECT COUNT(*) FROM reviews AS review
                JOIN attempts AS attempt ON attempt.id = review.attempt_id
                WHERE attempt.work_id = ?1
                "#,
                params![created.id],
                |row| row.get(0),
            )
            .expect("review count");
        assert_eq!(review_count, 1);
        drop(connection);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn concurrent_identical_human_reviews_commit_one_decision_and_both_succeed() {
        let path = temp_db();
        let setup = WorkStore::open(&path).expect("open setup");
        setup
            .ensure_project("proj_1", "Demo", "/tmp/demo")
            .expect("project");
        let created = setup
            .create_work(CreateWorkInput {
                project_id: "proj_1".into(),
                title: "Concurrent identical reviews".into(),
                description: String::new(),
                acceptance_criteria: "One durable human decision".into(),
                assignee_ref: None,
            })
            .expect("create");
        let started = setup
            .start_attempt(&created.id, created.revision)
            .expect("start");
        let in_review = setup
            .mark_attempt_ready_for_review(&started.id, started.revision)
            .expect("review-ready");
        drop(setup);

        let barrier = Arc::new(Barrier::new(2));
        let mut handles = Vec::new();
        for _ in 0..2 {
            let store = WorkStore::open(&path).expect("open concurrent store");
            let barrier = Arc::clone(&barrier);
            let work_id = in_review.id.clone();
            let revision = in_review.revision;
            handles.push(std::thread::spawn(move || {
                barrier.wait();
                store
                    .human_review(&work_id, revision, true, "Exact shared decision")
                    .map_err(|error| error.to_string())
            }));
        }
        let outcomes: Vec<_> = handles
            .into_iter()
            .map(|handle| handle.join().expect("review thread"))
            .collect();
        assert!(
            outcomes.iter().all(|result| result.is_ok()),
            "outcomes: {outcomes:?}"
        );
        assert_eq!(
            outcomes[0].as_ref().expect("first"),
            outcomes[1].as_ref().expect("second")
        );

        let store = WorkStore::open(&path).expect("reopen");
        let connection = store.connection.lock().expect("connection");
        let (review_count, event_count): (i64, i64) = connection
            .query_row(
                r#"
                SELECT
                    (SELECT COUNT(*) FROM reviews AS review
                     JOIN attempts AS attempt ON attempt.id = review.attempt_id
                     WHERE attempt.work_id = ?1),
                    (SELECT COUNT(*) FROM work_events
                     WHERE work_id = ?1 AND kind IN ('accepted', 'returned'))
                "#,
                params![created.id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("decision counts");
        assert_eq!((review_count, event_count), (1, 1));
        drop(connection);
        drop(store);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn concurrent_different_human_review_guidance_conflicts() {
        let path = temp_db();
        let setup = WorkStore::open(&path).expect("open setup");
        setup
            .ensure_project("proj_1", "Demo", "/tmp/demo")
            .expect("project");
        let created = setup
            .create_work(CreateWorkInput {
                project_id: "proj_1".into(),
                title: "Concurrent conflicting reviews".into(),
                description: String::new(),
                acceptance_criteria: "Conflicting guidance never aliases".into(),
                assignee_ref: None,
            })
            .expect("create");
        let started = setup
            .start_attempt(&created.id, created.revision)
            .expect("start");
        let in_review = setup
            .mark_attempt_ready_for_review(&started.id, started.revision)
            .expect("review-ready");
        drop(setup);

        let barrier = Arc::new(Barrier::new(2));
        let handles: Vec<_> = ["Add tests", "Add migration evidence"]
            .into_iter()
            .map(|guidance| {
                let store = WorkStore::open(&path).expect("open concurrent store");
                let barrier = Arc::clone(&barrier);
                let work_id = in_review.id.clone();
                let revision = in_review.revision;
                std::thread::spawn(move || {
                    barrier.wait();
                    store
                        .human_review(&work_id, revision, false, guidance)
                        .map_err(|error| error.to_string())
                })
            })
            .collect();
        let outcomes: Vec<_> = handles
            .into_iter()
            .map(|handle| handle.join().expect("review thread"))
            .collect();
        assert_eq!(outcomes.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(outcomes.iter().filter(|result| result.is_err()).count(), 1);
        assert!(outcomes
            .iter()
            .filter_map(|result| result.as_ref().err())
            .all(|error| error.contains("revision mismatch")));

        let store = WorkStore::open(&path).expect("reopen");
        let connection = store.connection.lock().expect("connection");
        let (review_count, event_count): (i64, i64) = connection
            .query_row(
                r#"
                SELECT
                    (SELECT COUNT(*) FROM reviews AS review
                     JOIN attempts AS attempt ON attempt.id = review.attempt_id
                     WHERE attempt.work_id = ?1),
                    (SELECT COUNT(*) FROM work_events
                     WHERE work_id = ?1 AND kind IN ('accepted', 'returned'))
                "#,
                params![created.id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("decision counts");
        assert_eq!((review_count, event_count), (1, 1));
        drop(connection);
        drop(store);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn attempt_run_binding_and_success_are_durable_and_idempotent() {
        let path = temp_db();
        let store = WorkStore::open(&path).expect("open");
        store
            .ensure_project("proj_1", "Demo", "/tmp/demo")
            .expect("project");
        let created = store
            .create_work(CreateWorkInput {
                project_id: "proj_1".into(),
                title: "Bind a real run".into(),
                description: "Use IsanAgent".into(),
                acceptance_criteria: "Run id survives restart".into(),
                assignee_ref: None,
            })
            .expect("create");
        let started = store
            .start_attempt_with_record(&created.id, created.revision)
            .expect("start");
        assert_eq!(started.work.state, WorkState::InProgress);
        assert_eq!(started.attempt.phase, AttemptPhase::Queued);
        assert!(started
            .attempt
            .input_json
            .as_deref()
            .is_some_and(|json| json.contains("Run id survives restart")));

        let bound = store
            .bind_attempt_run(
                &started.attempt.id,
                "chat-work",
                Some("chat-work"),
                "run-work",
            )
            .expect("bind");
        assert_eq!(bound.phase, AttemptPhase::Running);
        assert_eq!(bound.run_id.as_deref(), Some("run-work"));
        let rebound = store
            .bind_attempt_run(
                &started.attempt.id,
                "chat-work",
                Some("chat-work"),
                "run-work",
            )
            .expect("same bind is idempotent");
        assert_eq!(rebound, bound);
        assert!(store
            .bind_attempt_run(
                &started.attempt.id,
                "another-chat",
                Some("another-chat"),
                "another-run",
            )
            .is_err());

        drop(store);
        let reopened = WorkStore::open(&path).expect("reopen");
        let attempts = reopened.list_attempts(&created.id).expect("attempts");
        assert_eq!(attempts.len(), 1);
        assert_eq!(attempts[0].run_id.as_deref(), Some("run-work"));

        let in_review = reopened
            .finish_attempt_by_run(
                "run-work",
                AttemptPhase::Succeeded,
                r#"{"kind":"completed"}"#,
            )
            .expect("finish")
            .expect("bound work");
        assert_eq!(in_review.state, WorkState::InReview);
        let revision = in_review.revision;
        let repeated = reopened
            .finish_attempt_by_run(
                "run-work",
                AttemptPhase::Succeeded,
                r#"{"kind":"completed"}"#,
            )
            .expect("repeat finish")
            .expect("bound work");
        assert_eq!(repeated.revision, revision);
        assert_eq!(repeated.state, WorkState::InReview);
        let finished_attempt = reopened
            .get_attempt(&started.attempt.id)
            .expect("get attempt")
            .expect("finished attempt");
        assert_eq!(finished_attempt.phase, AttemptPhase::Succeeded);
        assert_eq!(
            finished_attempt.result_json.as_deref(),
            Some(r#"{"kind":"completed"}"#)
        );
        assert!(reopened
            .finish_attempt_by_run(
                "not-a-work-run",
                AttemptPhase::Succeeded,
                r#"{"kind":"completed"}"#,
            )
            .expect("unknown agent run is ignored")
            .is_none());

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn failed_attempt_returns_work_to_ready_without_done() {
        let path = temp_db();
        let store = WorkStore::open(&path).expect("open");
        store
            .ensure_project("proj_1", "Demo", "/tmp/demo")
            .expect("project");
        let created = store
            .create_work(CreateWorkInput {
                project_id: "proj_1".into(),
                title: "Retry failure".into(),
                description: String::new(),
                acceptance_criteria: String::new(),
                assignee_ref: None,
            })
            .expect("create");
        let started = store
            .start_attempt_with_record(&created.id, created.revision)
            .expect("start");
        let ready = store
            .finish_attempt_by_id(
                &started.attempt.id,
                AttemptPhase::Failed,
                r#"{"kind":"failed"}"#,
            )
            .expect("fail attempt");
        assert_eq!(ready.state, WorkState::Ready);
        let attempts = store.list_attempts(&created.id).expect("attempts");
        assert_eq!(attempts[0].phase, AttemptPhase::Failed);

        let retried = store
            .start_attempt_with_record(&ready.id, ready.revision)
            .expect("retry");
        assert_eq!(retried.attempt.number, 2);
        assert_eq!(retried.work.state, WorkState::InProgress);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn active_attempt_owns_in_progress_and_has_a_database_backstop() {
        let path = temp_db();
        let store = WorkStore::open(&path).expect("open");
        store
            .ensure_project("proj_1", "Demo", "/tmp/demo")
            .expect("project");
        let created = store
            .create_work(CreateWorkInput {
                project_id: "proj_1".into(),
                title: "Only one active attempt".into(),
                description: String::new(),
                acceptance_criteria: String::new(),
                assignee_ref: None,
            })
            .expect("create");
        let started = store
            .start_attempt_with_dispatch(
                &created.id,
                created.revision,
                Some("chat-owned"),
                Some("chat-owned"),
            )
            .expect("start");

        assert!(store
            .start_attempt_with_record(&created.id, started.work.revision)
            .is_err());
        assert!(store
            .transition(&created.id, started.work.revision, WorkState::Ready)
            .is_err());
        assert!(store
            .mark_attempt_ready_for_review(&created.id, started.work.revision)
            .is_err());

        let connection = store.connection.lock().expect("work store mutex");
        let duplicate = connection.execute(
            r#"
            INSERT INTO attempts (
                id, work_id, number, role, phase, created_at_ms, updated_at_ms
            ) VALUES ('attempt-duplicate', ?1, 2, 'executor', 'queued', 1, 1)
            "#,
            params![created.id],
        );
        assert!(duplicate.is_err());
        drop(connection);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn schema_v2_repairs_duplicate_active_attempts_before_unique_index() {
        let path = temp_db();
        let connection = Connection::open(&path).expect("legacy db");
        connection
            .execute_batch(
                r#"
                CREATE TABLE work_store_migrations (
                    version INTEGER PRIMARY KEY,
                    applied_at_ms INTEGER NOT NULL
                );
                INSERT INTO work_store_migrations VALUES (1, 1);
                "#,
            )
            .expect("migration table");
        connection.execute_batch(MIGRATION_V1).expect("v1 schema");
        connection
            .execute(
                r#"
                INSERT INTO projects
                    (id, name, workspace_ref, created_at_ms, updated_at_ms)
                VALUES ('proj_1', 'Demo', '/tmp/demo', 1, 1)
                "#,
                [],
            )
            .expect("project");
        connection
            .execute(
                r#"
                INSERT INTO work_items (
                    id, project_id, title, description, acceptance_criteria,
                    state, revision, created_at_ms, updated_at_ms
                ) VALUES ('work_1', 'proj_1', 'Repair', '', '', 'in_progress', 2, 1, 1)
                "#,
                [],
            )
            .expect("work");
        for (id, number) in [("attempt_old", 1), ("attempt_new", 2)] {
            connection
                .execute(
                    r#"
                    INSERT INTO attempts (
                        id, work_id, number, role, phase, created_at_ms, updated_at_ms
                    ) VALUES (?1, 'work_1', ?2, 'executor', 'queued', 1, 1)
                    "#,
                    params![id, number],
                )
                .expect("legacy active attempt");
        }
        drop(connection);

        let store = WorkStore::open(&path).expect("migrate v2");
        let attempts = store.list_attempts("work_1").expect("attempts");
        assert_eq!(attempts[0].phase, AttemptPhase::Queued);
        assert_eq!(attempts[1].phase, AttemptPhase::Failed);
        let connection = store.connection.lock().expect("work store mutex");
        assert!(connection
            .execute(
                r#"
                INSERT INTO attempts (
                    id, work_id, number, role, phase, created_at_ms, updated_at_ms
                ) VALUES ('attempt_third', 'work_1', 3, 'executor', 'queued', 1, 1)
                "#,
                [],
            )
            .is_err());
        drop(connection);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn journal_reconcile_recovers_binding_terminal_and_orphan() {
        let path = temp_db();
        let journal_path = path.with_extension("journal.db");
        let store = WorkStore::open(&path).expect("open");
        store
            .ensure_project("proj_1", "Demo", "/tmp/demo")
            .expect("project");
        let created = store
            .create_work(CreateWorkInput {
                project_id: "proj_1".into(),
                title: "Recover from journal".into(),
                description: String::new(),
                acceptance_criteria: String::new(),
                assignee_ref: None,
            })
            .expect("create");
        let started = store
            .start_attempt_with_dispatch(
                &created.id,
                created.revision,
                Some("chat-recovery"),
                Some("chat-recovery"),
            )
            .expect("start");
        assert_eq!(started.attempt.chat_id.as_deref(), Some("chat-recovery"));
        assert!(started.attempt.run_id.is_none());
        drop(store);

        let journal = EventJournal::open(&journal_path).expect("journal");
        let reopened = WorkStore::open(&path).expect("reopen");
        reopened
            .connection
            .lock()
            .expect("work store mutex")
            .execute(
                "UPDATE attempts SET created_at_ms = 1, updated_at_ms = 1 WHERE id = ?1",
                params![started.attempt.id],
            )
            .expect("age cold dispatch beyond the former timeout");
        assert!(reopened
            .reconcile_attempts_from_journal(&journal, AttemptReconcileMode::Live)
            .expect("current-process cold dispatch remains queued")
            .is_empty());
        assert_eq!(
            reopened
                .get_attempt(&started.attempt.id)
                .expect("attempt")
                .expect("cold attempt")
                .phase,
            AttemptPhase::Queued
        );
        journal
            .append(&JournalEvent::now(
                1,
                "run-recovery",
                1,
                "chat-recovery",
                "run_started",
                serde_json::json!({
                    "type": "run_started",
                    "run_id": "run-recovery"
                }),
            ))
            .expect("start event");

        let changed = reopened
            .reconcile_attempts_from_journal(&journal, AttemptReconcileMode::Live)
            .expect("bind from journal");
        assert_eq!(changed, vec![created.id.clone()]);
        let bound = reopened
            .get_attempt(&started.attempt.id)
            .expect("attempt")
            .expect("bound attempt");
        assert_eq!(bound.phase, AttemptPhase::Running);
        assert_eq!(bound.run_id.as_deref(), Some("run-recovery"));

        journal
            .append_terminal(&JournalEvent::now(
                1,
                "run-recovery",
                2,
                "chat-recovery",
                "run_terminated",
                serde_json::json!({
                    "type": "run_terminated",
                    "run_id": "run-recovery",
                    "outcome": { "kind": "completed" }
                }),
            ))
            .expect("terminal event");
        reopened
            .reconcile_attempts_from_journal(&journal, AttemptReconcileMode::Live)
            .expect("finish from journal");
        let reviewed = reopened
            .get_work(&created.id)
            .expect("work")
            .expect("reviewed work");
        assert_eq!(reviewed.state, WorkState::InReview);
        assert!(reopened
            .reconcile_attempts_from_journal(&journal, AttemptReconcileMode::Live)
            .expect("idempotent reconcile")
            .is_empty());

        let returned = reopened
            .human_review(&reviewed.id, reviewed.revision, false, "retry")
            .expect("return");
        let orphan = reopened
            .start_attempt_with_dispatch(
                &returned.id,
                returned.revision,
                Some("chat-orphan"),
                Some("chat-orphan"),
            )
            .expect("orphan start");
        assert!(reopened
            .reconcile_attempts_from_journal(&journal, AttemptReconcileMode::Live)
            .expect("live pass tolerates unjournaled dispatch")
            .is_empty());
        assert_eq!(
            reopened
                .get_attempt(&orphan.attempt.id)
                .expect("attempt")
                .expect("live orphan")
                .phase,
            AttemptPhase::Queued
        );
        reopened
            .reconcile_attempts_from_journal(
                &journal,
                AttemptReconcileMode::RestartRecovery,
            )
            .expect("fail orphan");
        let recovered = reopened
            .get_work(&created.id)
            .expect("work")
            .expect("ready work");
        assert_eq!(recovered.state, WorkState::Ready);
        assert_eq!(
            reopened
                .get_attempt(&orphan.attempt.id)
                .expect("attempt")
                .expect("orphan attempt")
                .phase,
            AttemptPhase::Failed
        );

        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(journal_path);
    }

    #[test]
    fn typed_journal_terminals_map_to_attempt_phases() {
        for (kind, expected) in [
            ("completed", AttemptPhase::Succeeded),
            ("cancelled", AttemptPhase::Cancelled),
            ("failed", AttemptPhase::Failed),
            ("stuck", AttemptPhase::Failed),
            ("budget_exhausted", AttemptPhase::Failed),
        ] {
            let summary = RunJournalSummary {
                run_id: "run-1".into(),
                chat_id: "chat-1".into(),
                last_seq: 2,
                terminal_seq: Some(2),
                terminal_kind: Some("run_terminated".into()),
                terminal_payload: Some(serde_json::json!({
                    "type": "run_terminated",
                    "run_id": "run-1",
                    "outcome": { "kind": kind }
                })),
            };
            let (actual, _) = terminal_attempt_outcome(&summary)
                .expect("typed terminal")
                .expect("terminal outcome");
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn rejects_empty_title() {
        let path = temp_db();
        let store = WorkStore::open(&path).expect("open");
        store
            .ensure_project("proj_1", "Demo", "/tmp/demo")
            .expect("project");
        let err = store
            .create_work(CreateWorkInput {
                project_id: "proj_1".into(),
                title: "   ".into(),
                description: String::new(),
                acceptance_criteria: String::new(),
                assignee_ref: None,
            })
            .expect_err("empty title");
        assert!(matches!(err, WorkStoreError::InvalidState(_)));
        let _ = std::fs::remove_file(path);
    }
}
