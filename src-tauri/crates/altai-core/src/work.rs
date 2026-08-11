//! Durable Work / Attempt / Review store (Work OS Milestone 1).
//!
//! User-scoped SQLite beside the existing host — not a separate control-plane
//! daemon. Schema matches `altaidevorg/altai-agent-work-os` ENGINEERING.md.

use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use std::fmt;
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const SCHEMA_VERSION: i64 = 1;

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
    UnsupportedSchema(i64),
    NotFound(String),
    InvalidState(&'static str),
}

impl fmt::Display for WorkStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "work store io: {error}"),
            Self::Sqlite(error) => write!(f, "work store sqlite: {error}"),
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
        let mut tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
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
        append_event(&mut tx, &id, "created", "{}")?;
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

    pub fn transition(&self, id: &str, expected_revision: i64, next: WorkState) -> Result<WorkItemRecord> {
        let now = now_ms();
        let mut connection = self.connection.lock().expect("work store mutex");
        let mut tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
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
        if !is_allowed_transition(from, next) {
            return Err(WorkStoreError::InvalidState("transition not allowed"));
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
        append_event(&mut tx, id, "state_changed", &payload)?;
        tx.commit()?;
        drop(connection);
        self.get_work(id)?
            .ok_or_else(|| WorkStoreError::NotFound(id.to_string()))
    }

    /// Move Work into `in_progress` and open attempt N+1 (queued).
    /// Backlog is promoted through ready in the same transaction.
    pub fn start_attempt(&self, id: &str, expected_revision: i64) -> Result<WorkItemRecord> {
        let now = now_ms();
        let mut connection = self.connection.lock().expect("work store mutex");
        let mut tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
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
            WorkState::Backlog | WorkState::Ready | WorkState::InProgress => {}
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
                &mut tx,
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
            append_event(&mut tx, id, "state_changed", &payload)?;
        }

        let next_number: i64 = tx.query_row(
            "SELECT COALESCE(MAX(number), 0) + 1 FROM attempts WHERE work_id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        let attempt_id = new_id("attempt");
        tx.execute(
            r#"
            INSERT INTO attempts (
                id, work_id, number, role, phase, created_at_ms, updated_at_ms
            ) VALUES (?1, ?2, ?3, 'executor', 'queued', ?4, ?4)
            "#,
            params![attempt_id, id, next_number, now as i64],
        )?;
        let payload = format!(r#"{{"attemptId":"{attempt_id}","number":{next_number}}}"#);
        append_event(&mut tx, id, "attempt_started", &payload)?;
        tx.commit()?;
        drop(connection);
        self.get_work(id)?
            .ok_or_else(|| WorkStoreError::NotFound(id.to_string()))
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
        let mut tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
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
        let attempt_id: String = tx.query_row(
            r#"
            SELECT id FROM attempts WHERE work_id = ?1
            ORDER BY number DESC LIMIT 1
            "#,
            params![id],
            |row| row.get(0),
        )?;
        tx.execute(
            r#"
            UPDATE attempts
            SET phase = 'succeeded', updated_at_ms = ?1
            WHERE id = ?2
            "#,
            params![now as i64, attempt_id],
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
            &mut tx,
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
        let now = now_ms();
        let mut connection = self.connection.lock().expect("work store mutex");
        let mut tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
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
        let next = if accept {
            WorkState::Done
        } else {
            WorkState::Ready
        };
        let status = if accept { "complete" } else { "incomplete" };
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
        let payload = format!(
            r#"{{"reviewId":"{review_id}","to":"{}"}}"#,
            next.as_str()
        );
        append_event(&mut tx, id, kind, &payload)?;
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

fn is_allowed_transition(from: WorkState, to: WorkState) -> bool {
    use WorkState::*;
    matches!(
        (from, to),
        (Backlog, Ready)
            | (Ready, InProgress)
            | (Ready, Cancelled)
            | (InProgress, InReview)
            | (InProgress, Ready) // return path before review record
            | (InProgress, Cancelled)
            | (InReview, Done)
            | (InReview, Ready)
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
    let rand = (millis ^ (millis << 13)) % 1_000_000;
    format!("{prefix}_{millis}_{rand:06}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_db() -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("altai-work-store-{nanos}.db"))
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
