//! Durable, Tauri-independent SQLite event journal shared by ALTAI Desktop
//! and the ALTAI CLI. Every host adapter that runs an agent turn appends the
//! same sequenced, append-only records here so restart-discovery and
//! inspection tooling work identically regardless of which surface produced
//! a run.

use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde_json::Value;
use std::fmt;
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const SCHEMA_VERSION: i64 = 3;
const MAX_FETCH_LIMIT: usize = 1_000;

const MIGRATION_V1: &str = r#"
CREATE TABLE IF NOT EXISTS agent_event_journal_events (
    run_id          TEXT NOT NULL,
    seq             INTEGER NOT NULL CHECK (seq > 0),
    version         INTEGER NOT NULL CHECK (version > 0),
    chat_id         TEXT NOT NULL,
    recorded_at_ms  INTEGER NOT NULL CHECK (recorded_at_ms >= 0),
    kind            TEXT NOT NULL,
    payload_json    TEXT NOT NULL CHECK (json_valid(payload_json)),
    is_terminal     INTEGER NOT NULL CHECK (is_terminal IN (0, 1)),
    PRIMARY KEY (run_id, seq)
) WITHOUT ROWID;

CREATE INDEX IF NOT EXISTS agent_event_journal_events_chat_time
    ON agent_event_journal_events (chat_id, recorded_at_ms, run_id, seq);

CREATE TABLE IF NOT EXISTS agent_event_journal_runs (
    run_id                 TEXT PRIMARY KEY,
    chat_id                TEXT NOT NULL,
    last_seq               INTEGER NOT NULL CHECK (last_seq >= 0),
    terminal_seq           INTEGER,
    terminal_kind          TEXT,
    terminal_payload_json  TEXT CHECK (
        terminal_payload_json IS NULL OR json_valid(terminal_payload_json)
    ),
    CHECK (
        (terminal_seq IS NULL AND terminal_kind IS NULL AND terminal_payload_json IS NULL)
        OR
        (terminal_seq IS NOT NULL AND terminal_kind IS NOT NULL AND terminal_payload_json IS NOT NULL)
    )
);
"#;

// Session records live beside the append-only run journal. They intentionally
// contain only host-neutral presentation metadata: runtime state and message
// bodies continue to belong to the agent service and memory store.
const MIGRATION_V2: &str = r#"
CREATE TABLE IF NOT EXISTS agent_event_journal_sessions (
    chat_id       TEXT PRIMARY KEY,
    title         TEXT NOT NULL,
    archived      INTEGER NOT NULL DEFAULT 0 CHECK (archived IN (0, 1)),
    updated_at_ms INTEGER NOT NULL CHECK (updated_at_ms >= 0)
);
"#;

const MIGRATION_V3: &str = r#"
CREATE TABLE IF NOT EXISTS agent_event_journal_task_runs (
    chat_id       TEXT PRIMARY KEY,
    title         TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL CHECK (created_at_ms >= 0)
);
"#;

#[derive(Debug, Clone, PartialEq)]
pub struct JournalEvent {
    pub version: u32,
    pub run_id: String,
    pub seq: u64,
    pub chat_id: String,
    pub recorded_at_ms: u64,
    pub kind: String,
    pub payload: Value,
}

impl JournalEvent {
    pub fn now(
        version: u32,
        run_id: impl Into<String>,
        seq: u64,
        chat_id: impl Into<String>,
        kind: impl Into<String>,
        payload: Value,
    ) -> Self {
        let recorded_at_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .min(u128::from(u64::MAX)) as u64;
        Self {
            version,
            run_id: run_id.into(),
            seq,
            chat_id: chat_id.into(),
            recorded_at_ms,
            kind: kind.into(),
            payload,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunJournalSummary {
    pub run_id: String,
    pub chat_id: String,
    pub last_seq: u64,
    pub terminal_seq: Option<u64>,
    pub terminal_kind: Option<String>,
    pub terminal_payload: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChatJournalSummary {
    pub chat_id: String,
    pub latest_run_id: String,
    pub last_seq: u64,
    pub terminal_seq: Option<u64>,
    pub updated_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionJournalMetadata {
    pub chat_id: String,
    pub title: String,
    pub archived: bool,
    pub updated_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskRunJournalMetadata {
    pub chat_id: String,
    pub title: String,
    pub created_at_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppendStatus {
    Appended,
    Duplicate,
}

#[derive(Debug)]
pub enum JournalError {
    Sqlite(rusqlite::Error),
    Io(std::io::Error),
    Json(serde_json::Error),
    InvalidField(&'static str),
    NumericOverflow(&'static str),
    UnsupportedSchema(i64),
    EventConflict {
        run_id: String,
        seq: u64,
    },
    RunChatMismatch {
        run_id: String,
    },
    OutOfOrder {
        run_id: String,
        expected: u64,
        actual: u64,
    },
    RunAlreadyTerminated {
        run_id: String,
    },
    TerminalAlreadyCommitted {
        run_id: String,
    },
    LockPoisoned,
}

impl fmt::Display for JournalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlite(error) => write!(f, "SQLite journal error: {error}"),
            Self::Io(error) => write!(f, "event journal I/O error: {error}"),
            Self::Json(error) => write!(f, "journal JSON error: {error}"),
            Self::InvalidField(field) => write!(f, "invalid journal field: {field}"),
            Self::NumericOverflow(field) => write!(f, "journal value exceeds SQLite: {field}"),
            Self::UnsupportedSchema(version) => {
                write!(f, "journal schema {version} is newer than supported")
            }
            Self::EventConflict { run_id, seq } => {
                write!(f, "conflicting event for run {run_id} sequence {seq}")
            }
            Self::RunChatMismatch { run_id } => {
                write!(f, "run {run_id} is already owned by a different chat")
            }
            Self::OutOfOrder {
                run_id,
                expected,
                actual,
            } => write!(
                f,
                "out-of-order event for run {run_id}: expected {expected}, got {actual}"
            ),
            Self::RunAlreadyTerminated { run_id } => {
                write!(f, "run {run_id} already has a terminal event")
            }
            Self::TerminalAlreadyCommitted { run_id } => {
                write!(f, "run {run_id} already committed a terminal outcome")
            }
            Self::LockPoisoned => write!(f, "event journal connection lock is poisoned"),
        }
    }
}

impl std::error::Error for JournalError {}

impl From<rusqlite::Error> for JournalError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Sqlite(value)
    }
}

impl From<std::io::Error> for JournalError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for JournalError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

pub type JournalResult<T> = Result<T, JournalError>;

/// Minimal durable store for sequenced run lifecycle events.
///
/// The journal deliberately exposes no update/delete API. A run advances by
/// appending exactly the next sequence, and its first terminal append wins a
/// transactional compare-and-set in `agent_event_journal_runs`.
pub struct EventJournal {
    connection: Mutex<Connection>,
}

impl EventJournal {
    pub fn open(path: impl AsRef<Path>) -> JournalResult<Self> {
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        create_private_file(path.as_ref())?;
        let connection = Connection::open(path)?;
        Self::from_connection(connection)
    }

    #[cfg(test)]
    fn open_in_memory() -> JournalResult<Self> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(mut connection: Connection) -> JournalResult<Self> {
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        migrate(&mut connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub fn append(&self, event: &JournalEvent) -> JournalResult<AppendStatus> {
        self.append_inner(event, false)
    }

    pub fn append_terminal(&self, event: &JournalEvent) -> JournalResult<AppendStatus> {
        self.append_inner(event, true)
    }

    fn append_inner(&self, event: &JournalEvent, terminal: bool) -> JournalResult<AppendStatus> {
        validate_event(event)?;
        let seq = sqlite_u64(event.seq, "seq")?;
        let version = i64::from(event.version);
        let recorded_at_ms = sqlite_u64(event.recorded_at_ms, "recorded_at_ms")?;
        let payload_json = serde_json::to_string(&event.payload)?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| JournalError::LockPoisoned)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(existing) = find_event(&transaction, &event.run_id, seq)? {
            if existing == StoredEvent::from_input(event, terminal, &payload_json) {
                transaction.rollback()?;
                return Ok(AppendStatus::Duplicate);
            }
            if terminal && run_has_terminal(&transaction, &event.run_id)? {
                return Err(JournalError::TerminalAlreadyCommitted {
                    run_id: event.run_id.clone(),
                });
            }
            return Err(JournalError::EventConflict {
                run_id: event.run_id.clone(),
                seq: event.seq,
            });
        }

        transaction.execute(
            "INSERT INTO agent_event_journal_runs (run_id, chat_id, last_seq)
             VALUES (?1, ?2, 0)
             ON CONFLICT(run_id) DO NOTHING",
            params![event.run_id, event.chat_id],
        )?;

        let (stored_chat_id, last_seq, terminal_seq): (String, i64, Option<i64>) = transaction
            .query_row(
                "SELECT chat_id, last_seq, terminal_seq
                 FROM agent_event_journal_runs WHERE run_id = ?1",
                params![event.run_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )?;
        if stored_chat_id != event.chat_id {
            return Err(JournalError::RunChatMismatch {
                run_id: event.run_id.clone(),
            });
        }
        if terminal_seq.is_some() {
            return Err(if terminal {
                JournalError::TerminalAlreadyCommitted {
                    run_id: event.run_id.clone(),
                }
            } else {
                JournalError::RunAlreadyTerminated {
                    run_id: event.run_id.clone(),
                }
            });
        }
        let expected = last_seq
            .checked_add(1)
            .ok_or(JournalError::NumericOverflow("last_seq"))?;
        if seq != expected {
            return Err(JournalError::OutOfOrder {
                run_id: event.run_id.clone(),
                expected: expected as u64,
                actual: event.seq,
            });
        }

        if terminal {
            let changed = transaction.execute(
                "UPDATE agent_event_journal_runs
                 SET terminal_seq = ?2, terminal_kind = ?3, terminal_payload_json = ?4
                 WHERE run_id = ?1 AND terminal_seq IS NULL",
                params![event.run_id, seq, event.kind, payload_json],
            )?;
            if changed != 1 {
                return Err(JournalError::TerminalAlreadyCommitted {
                    run_id: event.run_id.clone(),
                });
            }
        }

        transaction.execute(
            "INSERT INTO agent_event_journal_events
             (run_id, seq, version, chat_id, recorded_at_ms, kind, payload_json, is_terminal)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                event.run_id,
                seq,
                version,
                event.chat_id,
                recorded_at_ms,
                event.kind,
                payload_json,
                terminal
            ],
        )?;
        transaction.execute(
            "UPDATE agent_event_journal_runs SET last_seq = ?2 WHERE run_id = ?1",
            params![event.run_id, seq],
        )?;
        transaction.commit()?;
        Ok(AppendStatus::Appended)
    }

    pub fn fetch_after(
        &self,
        run_id: &str,
        after_seq: u64,
        limit: usize,
    ) -> JournalResult<Vec<JournalEvent>> {
        if run_id.trim().is_empty() {
            return Err(JournalError::InvalidField("run_id"));
        }
        if limit == 0 || limit > MAX_FETCH_LIMIT {
            return Err(JournalError::InvalidField("limit"));
        }
        let after_seq = sqlite_u64(after_seq, "after_seq")?;
        let limit = i64::try_from(limit).map_err(|_| JournalError::NumericOverflow("limit"))?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| JournalError::LockPoisoned)?;
        let mut statement = connection.prepare(
            "SELECT version, run_id, seq, chat_id, recorded_at_ms, kind, payload_json
             FROM agent_event_journal_events
             WHERE run_id = ?1 AND seq > ?2
             ORDER BY seq ASC LIMIT ?3",
        )?;
        let rows = statement.query_map(params![run_id, after_seq, limit], |row| {
            let version: i64 = row.get(0)?;
            let seq: i64 = row.get(2)?;
            let recorded_at_ms: i64 = row.get(4)?;
            let payload_json: String = row.get(6)?;
            Ok((
                version,
                row.get::<_, String>(1)?,
                seq,
                row.get::<_, String>(3)?,
                recorded_at_ms,
                row.get::<_, String>(5)?,
                payload_json,
            ))
        })?;
        rows.map(|row| {
            let (version, run_id, seq, chat_id, recorded_at_ms, kind, payload_json) = row?;
            Ok(JournalEvent {
                version: u32::try_from(version)
                    .map_err(|_| JournalError::NumericOverflow("version"))?,
                run_id,
                seq: u64::try_from(seq).map_err(|_| JournalError::NumericOverflow("seq"))?,
                chat_id,
                recorded_at_ms: u64::try_from(recorded_at_ms)
                    .map_err(|_| JournalError::NumericOverflow("recorded_at_ms"))?,
                kind,
                payload: serde_json::from_str(&payload_json)?,
            })
        })
        .collect()
    }

    pub fn run_summary(&self, run_id: &str) -> JournalResult<Option<RunJournalSummary>> {
        if run_id.trim().is_empty() {
            return Err(JournalError::InvalidField("run_id"));
        }
        let connection = self
            .connection
            .lock()
            .map_err(|_| JournalError::LockPoisoned)?;
        let stored = connection
            .query_row(
                "SELECT run_id, chat_id, last_seq, terminal_seq, terminal_kind,
                        terminal_payload_json
                 FROM agent_event_journal_runs WHERE run_id = ?1",
                params![run_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, Option<i64>>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, Option<String>>(5)?,
                    ))
                },
            )
            .optional()?;
        stored.map(decode_run_summary).transpose()
    }

    /// Most recently recorded run for one chat. This is the renderer's
    /// restart-discovery cursor; it exposes no payload beyond the terminal
    /// lifecycle value already returned by `run_summary`.
    pub fn latest_run_summary_for_chat(
        &self,
        chat_id: &str,
    ) -> JournalResult<Option<RunJournalSummary>> {
        if chat_id.trim().is_empty() {
            return Err(JournalError::InvalidField("chat_id"));
        }
        let connection = self
            .connection
            .lock()
            .map_err(|_| JournalError::LockPoisoned)?;
        let stored = connection
            .query_row(
                "SELECT runs.run_id, runs.chat_id, runs.last_seq, runs.terminal_seq,
                        runs.terminal_kind, runs.terminal_payload_json
                 FROM agent_event_journal_runs AS runs
                 JOIN agent_event_journal_events AS events
                   ON events.run_id = runs.run_id AND events.seq = runs.last_seq
                 WHERE runs.chat_id = ?1
                 ORDER BY events.recorded_at_ms DESC, runs.run_id DESC
                 LIMIT 1",
                params![chat_id],
                read_stored_run_summary,
            )
            .optional()?;
        stored.map(decode_run_summary).transpose()
    }

    /// Lists one latest run per chat, newest first. Session titles remain a UI
    /// concern until the shared session metadata schema lands.
    pub fn list_chat_summaries(&self, limit: usize) -> JournalResult<Vec<ChatJournalSummary>> {
        if limit == 0 || limit > MAX_FETCH_LIMIT {
            return Err(JournalError::InvalidField("limit"));
        }
        let connection = self
            .connection
            .lock()
            .map_err(|_| JournalError::LockPoisoned)?;
        let mut statement = connection.prepare(
            "SELECT runs.chat_id, runs.run_id, runs.last_seq, runs.terminal_seq,
                    events.recorded_at_ms
             FROM agent_event_journal_runs AS runs
             JOIN agent_event_journal_events AS events
               ON events.run_id = runs.run_id AND events.seq = runs.last_seq
             ORDER BY events.recorded_at_ms DESC, runs.run_id DESC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, Option<i64>>(3)?,
                row.get::<_, i64>(4)?,
            ))
        })?;
        let mut seen = std::collections::HashSet::new();
        let mut summaries = Vec::new();
        for row in rows {
            let (chat_id, latest_run_id, last_seq, terminal_seq, updated_at_ms) = row?;
            if !seen.insert(chat_id.clone()) {
                continue;
            }
            summaries.push(ChatJournalSummary {
                chat_id,
                latest_run_id,
                last_seq: u64::try_from(last_seq)
                    .map_err(|_| JournalError::NumericOverflow("last_seq"))?,
                terminal_seq: terminal_seq
                    .map(u64::try_from)
                    .transpose()
                    .map_err(|_| JournalError::NumericOverflow("terminal_seq"))?,
                updated_at_ms: u64::try_from(updated_at_ms)
                    .map_err(|_| JournalError::NumericOverflow("recorded_at_ms"))?,
            });
            if summaries.len() >= limit {
                break;
            }
        }
        Ok(summaries)
    }

    /// Lists explicit sessions as well as legacy chats discovered from the
    /// event journal. This lets existing conversations gain durable metadata
    /// lazily without losing history created before the session table existed.
    pub fn list_session_metadata(
        &self,
        limit: usize,
    ) -> JournalResult<Vec<SessionJournalMetadata>> {
        if limit == 0 || limit > MAX_FETCH_LIMIT {
            return Err(JournalError::InvalidField("limit"));
        }
        let limit = i64::try_from(limit).map_err(|_| JournalError::NumericOverflow("limit"))?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| JournalError::LockPoisoned)?;
        let mut statement = connection.prepare(
            "WITH event_sessions AS (
                 SELECT chat_id, MAX(recorded_at_ms) AS updated_at_ms
                 FROM agent_event_journal_events
                 GROUP BY chat_id
             ), all_sessions AS (
                 SELECT sessions.chat_id,
                        sessions.title,
                        sessions.archived,
                        MAX(sessions.updated_at_ms, COALESCE(events.updated_at_ms, 0)) AS updated_at_ms
                 FROM agent_event_journal_sessions AS sessions
                 LEFT JOIN event_sessions AS events ON events.chat_id = sessions.chat_id
                 UNION ALL
                 SELECT events.chat_id, events.chat_id, 0, events.updated_at_ms
                 FROM event_sessions AS events
                 WHERE NOT EXISTS (
                     SELECT 1 FROM agent_event_journal_sessions AS sessions
                     WHERE sessions.chat_id = events.chat_id
                 )
             )
             SELECT chat_id, title, archived, updated_at_ms
             FROM all_sessions
             ORDER BY updated_at_ms DESC, chat_id DESC
             LIMIT ?1",
        )?;
        let rows = statement.query_map(params![limit], |row| {
            Ok(SessionJournalMetadata {
                chat_id: row.get(0)?,
                title: row.get(1)?,
                archived: row.get::<_, i64>(2)? != 0,
                updated_at_ms: u64::try_from(row.get::<_, i64>(3)?)
                    .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(3, 0))?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn session_metadata(&self, chat_id: &str) -> JournalResult<Option<SessionJournalMetadata>> {
        validate_session_field(chat_id, "chat_id")?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| JournalError::LockPoisoned)?;
        connection
            .query_row(
                "WITH event_session AS (
                     SELECT MAX(recorded_at_ms) AS updated_at_ms
                     FROM agent_event_journal_events WHERE chat_id = ?1
                 )
                 SELECT sessions.chat_id, sessions.title, sessions.archived,
                        MAX(sessions.updated_at_ms, COALESCE(events.updated_at_ms, 0))
                 FROM agent_event_journal_sessions AS sessions
                 LEFT JOIN event_session AS events ON true
                 WHERE sessions.chat_id = ?1
                 UNION ALL
                 SELECT ?1, ?1, 0, updated_at_ms FROM event_session
                 WHERE updated_at_ms IS NOT NULL
                   AND NOT EXISTS (SELECT 1 FROM agent_event_journal_sessions WHERE chat_id = ?1)
                 LIMIT 1",
                params![chat_id],
                |row| {
                    Ok(SessionJournalMetadata {
                        chat_id: row.get(0)?,
                        title: row.get(1)?,
                        archived: row.get::<_, i64>(2)? != 0,
                        updated_at_ms: u64::try_from(row.get::<_, i64>(3)?)
                            .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(3, 0))?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn create_session(
        &self,
        chat_id: &str,
        title: &str,
    ) -> JournalResult<SessionJournalMetadata> {
        validate_session_field(chat_id, "chat_id")?;
        validate_session_field(title, "title")?;
        let updated_at_ms = now_ms();
        let connection = self
            .connection
            .lock()
            .map_err(|_| JournalError::LockPoisoned)?;
        connection.execute(
            "INSERT INTO agent_event_journal_sessions (chat_id, title, archived, updated_at_ms)
             VALUES (?1, ?2, 0, ?3)",
            params![chat_id, title, updated_at_ms],
        )?;
        Ok(SessionJournalMetadata {
            chat_id: chat_id.to_string(),
            title: title.to_string(),
            archived: false,
            updated_at_ms: u64::try_from(updated_at_ms)
                .map_err(|_| JournalError::NumericOverflow("updated_at_ms"))?,
        })
    }

    pub fn rename_session(
        &self,
        chat_id: &str,
        title: &str,
    ) -> JournalResult<Option<SessionJournalMetadata>> {
        self.update_session_metadata(chat_id, Some(title), None)
    }

    pub fn archive_session(&self, chat_id: &str) -> JournalResult<Option<SessionJournalMetadata>> {
        self.update_session_metadata(chat_id, None, Some(true))
    }

    /// Removes all journal records for this chat. The host adapter is also
    /// responsible for clearing the agent-memory thread before calling this.
    pub fn delete_session(&self, chat_id: &str) -> JournalResult<bool> {
        validate_session_field(chat_id, "chat_id")?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| JournalError::LockPoisoned)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let deleted_events = transaction.execute(
            "DELETE FROM agent_event_journal_events WHERE chat_id = ?1",
            params![chat_id],
        )?;
        let deleted_runs = transaction.execute(
            "DELETE FROM agent_event_journal_runs WHERE chat_id = ?1",
            params![chat_id],
        )?;
        let deleted_metadata = transaction.execute(
            "DELETE FROM agent_event_journal_sessions WHERE chat_id = ?1",
            params![chat_id],
        )?;
        let deleted_task = transaction.execute(
            "DELETE FROM agent_event_journal_task_runs WHERE chat_id = ?1",
            params![chat_id],
        )?;
        transaction.commit()?;
        Ok(deleted_events != 0 || deleted_runs != 0 || deleted_metadata != 0 || deleted_task != 0)
    }

    pub fn create_task_run(&self, chat_id: &str, title: &str) -> JournalResult<TaskRunJournalMetadata> {
        validate_session_field(chat_id, "chat_id")?;
        validate_session_field(title, "title")?;
        let created_at_ms = now_ms();
        let connection = self.connection.lock().map_err(|_| JournalError::LockPoisoned)?;
        connection.execute(
            "INSERT INTO agent_event_journal_task_runs (chat_id, title, created_at_ms) VALUES (?1, ?2, ?3)",
            params![chat_id, title, created_at_ms],
        )?;
        Ok(TaskRunJournalMetadata { chat_id: chat_id.to_string(), title: title.to_string(), created_at_ms: u64::try_from(created_at_ms).map_err(|_| JournalError::NumericOverflow("created_at_ms"))? })
    }

    pub fn list_task_runs(&self, limit: usize) -> JournalResult<Vec<TaskRunJournalMetadata>> {
        if limit == 0 || limit > MAX_FETCH_LIMIT { return Err(JournalError::InvalidField("limit")); }
        let limit = i64::try_from(limit).map_err(|_| JournalError::NumericOverflow("limit"))?;
        let connection = self.connection.lock().map_err(|_| JournalError::LockPoisoned)?;
        let mut statement = connection.prepare("SELECT chat_id, title, created_at_ms FROM agent_event_journal_task_runs ORDER BY created_at_ms DESC, chat_id DESC LIMIT ?1")?;
        let rows = statement.query_map(params![limit], |row| Ok(TaskRunJournalMetadata {
            chat_id: row.get(0)?, title: row.get(1)?,
            created_at_ms: u64::try_from(row.get::<_, i64>(2)?).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(2, 0))?,
        }))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn remove_task_run(&self, chat_id: &str) -> JournalResult<bool> {
        validate_session_field(chat_id, "chat_id")?;
        let connection = self.connection.lock().map_err(|_| JournalError::LockPoisoned)?;
        Ok(connection.execute("DELETE FROM agent_event_journal_task_runs WHERE chat_id = ?1", params![chat_id])? != 0)
    }

    fn update_session_metadata(
        &self,
        chat_id: &str,
        title: Option<&str>,
        archived: Option<bool>,
    ) -> JournalResult<Option<SessionJournalMetadata>> {
        validate_session_field(chat_id, "chat_id")?;
        if let Some(title) = title {
            validate_session_field(title, "title")?;
        }
        let updated_at_ms = now_ms();
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| JournalError::LockPoisoned)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let exists = transaction.query_row(
            "SELECT EXISTS(
                 SELECT 1 FROM agent_event_journal_sessions WHERE chat_id = ?1
                 UNION ALL
                 SELECT 1 FROM agent_event_journal_runs WHERE chat_id = ?1
             )",
            params![chat_id],
            |row| row.get::<_, i64>(0),
        )? != 0;
        if !exists {
            transaction.rollback()?;
            return Ok(None);
        }
        transaction.execute(
            "INSERT INTO agent_event_journal_sessions (chat_id, title, archived, updated_at_ms)
             VALUES (?1, COALESCE(?2, ?1), COALESCE(?3, 0), ?4)
             ON CONFLICT(chat_id) DO UPDATE SET
                title = COALESCE(?2, agent_event_journal_sessions.title),
                archived = COALESCE(?3, agent_event_journal_sessions.archived),
                updated_at_ms = excluded.updated_at_ms",
            params![
                chat_id,
                title,
                archived.map(|value| if value { 1_i64 } else { 0_i64 }),
                updated_at_ms
            ],
        )?;
        transaction.commit()?;
        drop(connection);
        self.session_metadata(chat_id)
    }

    /// Snapshot unfinished runs from a previous host process. Callers may
    /// classify each one only by appending its next terminal sequence.
    pub fn incomplete_run_summaries(&self) -> JournalResult<Vec<RunJournalSummary>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| JournalError::LockPoisoned)?;
        let mut statement = connection.prepare(
            "SELECT run_id, chat_id, last_seq, terminal_seq, terminal_kind,
                    terminal_payload_json
             FROM agent_event_journal_runs
             WHERE terminal_seq IS NULL
             ORDER BY run_id ASC",
        )?;
        let rows = statement.query_map([], read_stored_run_summary)?;
        rows.map(|row| decode_run_summary(row?)).collect()
    }

    #[cfg(test)]
    fn schema_version(&self) -> JournalResult<i64> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| JournalError::LockPoisoned)?;
        current_schema_version(&connection)
    }
}

type StoredRunSummary = (
    String,
    String,
    i64,
    Option<i64>,
    Option<String>,
    Option<String>,
);

fn read_stored_run_summary(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredRunSummary> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
    ))
}

fn decode_run_summary(stored: StoredRunSummary) -> JournalResult<RunJournalSummary> {
    let (run_id, chat_id, last_seq, terminal_seq, terminal_kind, terminal_payload) = stored;
    Ok(RunJournalSummary {
        run_id,
        chat_id,
        last_seq: u64::try_from(last_seq).map_err(|_| JournalError::NumericOverflow("last_seq"))?,
        terminal_seq: terminal_seq
            .map(u64::try_from)
            .transpose()
            .map_err(|_| JournalError::NumericOverflow("terminal_seq"))?,
        terminal_kind,
        terminal_payload: terminal_payload
            .map(|payload| serde_json::from_str(&payload))
            .transpose()?,
    })
}

#[derive(Debug, PartialEq)]
struct StoredEvent {
    version: i64,
    chat_id: String,
    kind: String,
    payload_json: String,
    terminal: bool,
}

impl StoredEvent {
    fn from_input(event: &JournalEvent, terminal: bool, payload_json: &str) -> Self {
        Self {
            version: i64::from(event.version),
            chat_id: event.chat_id.clone(),
            kind: event.kind.clone(),
            payload_json: payload_json.to_string(),
            terminal,
        }
    }
}

fn find_event(
    transaction: &Transaction<'_>,
    run_id: &str,
    seq: i64,
) -> JournalResult<Option<StoredEvent>> {
    transaction
        .query_row(
            "SELECT version, chat_id, kind, payload_json, is_terminal
             FROM agent_event_journal_events WHERE run_id = ?1 AND seq = ?2",
            params![run_id, seq],
            |row| {
                Ok(StoredEvent {
                    version: row.get(0)?,
                    chat_id: row.get(1)?,
                    kind: row.get(2)?,
                    payload_json: row.get(3)?,
                    terminal: row.get(4)?,
                })
            },
        )
        .optional()
        .map_err(JournalError::from)
}

fn run_has_terminal(transaction: &Transaction<'_>, run_id: &str) -> JournalResult<bool> {
    Ok(transaction
        .query_row(
            "SELECT terminal_seq IS NOT NULL FROM agent_event_journal_runs WHERE run_id = ?1",
            params![run_id],
            |row| row.get(0),
        )
        .optional()?
        .unwrap_or(false))
}

fn validate_event(event: &JournalEvent) -> JournalResult<()> {
    if event.version == 0 {
        return Err(JournalError::InvalidField("version"));
    }
    if event.run_id.trim().is_empty() {
        return Err(JournalError::InvalidField("run_id"));
    }
    if event.seq == 0 {
        return Err(JournalError::InvalidField("seq"));
    }
    if event.chat_id.trim().is_empty() {
        return Err(JournalError::InvalidField("chat_id"));
    }
    if event.kind.trim().is_empty() {
        return Err(JournalError::InvalidField("kind"));
    }
    Ok(())
}

fn validate_session_field(value: &str, field: &'static str) -> JournalResult<()> {
    if value.trim().is_empty() || value.len() > 256 {
        return Err(JournalError::InvalidField(field));
    }
    Ok(())
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
}

fn sqlite_u64(value: u64, field: &'static str) -> JournalResult<i64> {
    i64::try_from(value).map_err(|_| JournalError::NumericOverflow(field))
}

fn migrate(connection: &mut Connection) -> JournalResult<()> {
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    transaction.execute_batch(
        "CREATE TABLE IF NOT EXISTS agent_event_journal_migrations (
             version       INTEGER PRIMARY KEY,
             applied_at_ms INTEGER NOT NULL CHECK (applied_at_ms >= 0)
         );",
    )?;
    let current = current_schema_version(&transaction)?;
    if current > SCHEMA_VERSION {
        return Err(JournalError::UnsupportedSchema(current));
    }
    if current < 1 {
        let applied_at_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .min(i64::MAX as u128) as i64;
        transaction.execute_batch(MIGRATION_V1)?;
        transaction.execute(
            "INSERT INTO agent_event_journal_migrations (version, applied_at_ms)
             VALUES (1, ?1)",
            params![applied_at_ms],
        )?;
    }
    if current < 2 {
        transaction.execute_batch(MIGRATION_V2)?;
        transaction.execute(
            "INSERT INTO agent_event_journal_migrations (version, applied_at_ms)
             VALUES (2, ?1)",
            params![now_ms()],
        )?;
    }
    if current < 3 {
        transaction.execute_batch(MIGRATION_V3)?;
        transaction.execute("INSERT INTO agent_event_journal_migrations (version, applied_at_ms) VALUES (3, ?1)", params![now_ms()])?;
    }
    transaction.commit()?;
    Ok(())
}

fn current_schema_version(connection: &Connection) -> JournalResult<i64> {
    Ok(connection.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM agent_event_journal_migrations",
        [],
        |row| row.get(0),
    )?)
}

fn create_private_file(path: &Path) -> JournalResult<()> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true).write(true).create(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let _file = options.open(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        _file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};

    fn event(seq: u64, kind: &str, payload: Value) -> JournalEvent {
        JournalEvent {
            version: 1,
            run_id: "run-1".to_string(),
            seq,
            chat_id: "chat-1".to_string(),
            recorded_at_ms: 1_700_000_000_000 + seq,
            kind: kind.to_string(),
            payload,
        }
    }

    #[test]
    fn migration_is_idempotent_and_rejects_future_schema() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("events.sqlite3");
        let journal = EventJournal::open(&path).expect("open journal");
        assert_eq!(journal.schema_version().expect("schema version"), 3);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&path)
                    .expect("journal metadata")
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
        drop(journal);
        assert_eq!(
            EventJournal::open(&path)
                .expect("reopen migrated journal")
                .schema_version()
                .expect("schema version"),
            3
        );

        let future_path = temp.path().join("future.sqlite3");
        let connection = Connection::open(&future_path).expect("future db");
        connection
            .execute_batch(
                "CREATE TABLE agent_event_journal_migrations (
                    version INTEGER PRIMARY KEY,
                    applied_at_ms INTEGER NOT NULL
                 );
                 INSERT INTO agent_event_journal_migrations VALUES (99, 0);",
            )
            .expect("future schema marker");
        drop(connection);
        assert!(matches!(
            EventJournal::open(&future_path),
            Err(JournalError::UnsupportedSchema(99))
        ));
    }

    #[test]
    fn concurrent_first_open_applies_one_migration() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = Arc::new(temp.path().join("concurrent-migration.sqlite3"));
        let barrier = Arc::new(Barrier::new(3));
        let handles: Vec<_> = (0..2)
            .map(|_| {
                let path = path.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    EventJournal::open(path.as_ref())
                        .map(|journal| journal.schema_version().expect("schema version"))
                })
            })
            .collect();
        barrier.wait();

        for handle in handles {
            assert_eq!(handle.join().expect("migration thread").expect("open"), 3);
        }
        assert_eq!(
            EventJournal::open(path.as_ref())
                .expect("verify journal")
                .schema_version()
                .expect("schema version"),
            3
        );
    }

    #[test]
    fn append_fetch_after_and_summary_preserve_sequence_order() {
        let journal = EventJournal::open_in_memory().expect("journal");
        for seq in 1..=3 {
            assert_eq!(
                journal
                    .append(&event(seq, "thinking", serde_json::json!({ "seq": seq })))
                    .expect("append"),
                AppendStatus::Appended
            );
        }

        let fetched = journal.fetch_after("run-1", 1, 10).expect("fetch");
        assert_eq!(
            fetched.iter().map(|item| item.seq).collect::<Vec<_>>(),
            vec![2, 3]
        );
        assert_eq!(fetched[0].payload, serde_json::json!({ "seq": 2 }));
        assert_eq!(
            journal.run_summary("run-1").expect("summary"),
            Some(RunJournalSummary {
                run_id: "run-1".to_string(),
                chat_id: "chat-1".to_string(),
                last_seq: 3,
                terminal_seq: None,
                terminal_kind: None,
                terminal_payload: None,
            })
        );
    }

    #[test]
    fn chat_summaries_return_only_the_latest_run_per_chat() {
        let journal = EventJournal::open_in_memory().expect("journal");
        for (run_id, chat_id, recorded_at_ms) in [
            ("run-1", "chat-1", 10),
            ("run-2", "chat-1", 30),
            ("run-3", "chat-2", 20),
        ] {
            journal
                .append(&JournalEvent {
                    version: 1,
                    run_id: run_id.to_string(),
                    seq: 1,
                    chat_id: chat_id.to_string(),
                    recorded_at_ms,
                    kind: "run_started".to_string(),
                    payload: serde_json::json!({ "type": "run_started", "run_id": run_id }),
                })
                .expect("append");
        }
        let summaries = journal.list_chat_summaries(10).expect("session summaries");
        assert_eq!(
            summaries
                .iter()
                .map(|summary| (summary.chat_id.as_str(), summary.latest_run_id.as_str()))
                .collect::<Vec<_>>(),
            vec![("chat-1", "run-2"), ("chat-2", "run-3")]
        );
    }

    #[test]
    fn task_run_metadata_is_durable_and_removable() {
        let journal = EventJournal::open_in_memory().expect("journal");
        journal.create_task_run("task-chat", "Review pull request").expect("create task");
        assert_eq!(journal.list_task_runs(10).expect("list")[0].title, "Review pull request");
        assert!(journal.remove_task_run("task-chat").expect("remove task"));
        assert!(journal.list_task_runs(10).expect("list").is_empty());
    }

    #[test]
    fn session_metadata_persists_titles_archives_and_legacy_chats() {
        let journal = EventJournal::open_in_memory().expect("journal");
        let created = journal
            .create_session("empty-chat", "Empty chat")
            .expect("create session");
        assert_eq!(created.title, "Empty chat");
        assert!(!created.archived);

        journal
            .append(&JournalEvent {
                version: 1,
                run_id: "legacy-run".to_string(),
                seq: 1,
                chat_id: "legacy-chat".to_string(),
                recorded_at_ms: 42,
                kind: "run_started".to_string(),
                payload: serde_json::json!({}),
            })
            .expect("append legacy event");
        assert_eq!(
            journal
                .session_metadata("legacy-chat")
                .expect("legacy metadata")
                .expect("legacy session")
                .title,
            "legacy-chat"
        );

        let renamed = journal
            .rename_session("legacy-chat", "Renamed legacy chat")
            .expect("rename")
            .expect("renamed session");
        assert_eq!(renamed.title, "Renamed legacy chat");
        let archived = journal
            .archive_session("legacy-chat")
            .expect("archive")
            .expect("archived session");
        assert!(archived.archived);
        assert!(journal
            .delete_session("legacy-chat")
            .expect("delete session"));
        assert!(journal
            .session_metadata("legacy-chat")
            .expect("deleted metadata")
            .is_none());
        assert_eq!(
            journal
                .list_session_metadata(10)
                .expect("list sessions")
                .iter()
                .map(|session| session.chat_id.as_str())
                .collect::<Vec<_>>(),
            vec!["empty-chat"]
        );
    }

    #[test]
    fn duplicate_is_idempotent_but_conflicting_or_gapped_events_fail() {
        let journal = EventJournal::open_in_memory().expect("journal");
        let first = event(1, "run_started", serde_json::json!({ "run_id": "run-1" }));
        assert_eq!(
            journal.append(&first).expect("first"),
            AppendStatus::Appended
        );
        let mut replayed = first.clone();
        replayed.recorded_at_ms += 1;
        assert_eq!(
            journal.append(&replayed).expect("duplicate"),
            AppendStatus::Duplicate
        );

        let conflict = event(1, "thinking", serde_json::json!({ "different": true }));
        assert!(matches!(
            journal.append(&conflict),
            Err(JournalError::EventConflict { seq: 1, .. })
        ));
        assert!(matches!(
            journal.append(&event(3, "thinking", Value::Null)),
            Err(JournalError::OutOfOrder {
                expected: 2,
                actual: 3,
                ..
            })
        ));
    }

    #[test]
    fn identical_duplicate_rolls_back_without_mutating_event_or_summary() {
        let journal = EventJournal::open_in_memory().expect("journal");
        let first = event(1, "run_started", serde_json::json!({ "run_id": "run-1" }));
        journal.append(&first).expect("first");
        let events_before = journal.fetch_after("run-1", 0, 10).expect("events before");
        let summary_before = journal
            .run_summary("run-1")
            .expect("summary before")
            .expect("run summary before");

        let mut duplicate = first;
        duplicate.recorded_at_ms += 1;
        assert_eq!(
            journal.append(&duplicate).expect("duplicate"),
            AppendStatus::Duplicate
        );

        assert_eq!(
            journal.fetch_after("run-1", 0, 10).expect("events after"),
            events_before
        );
        assert_eq!(
            journal
                .run_summary("run-1")
                .expect("summary after")
                .expect("run summary after"),
            summary_before
        );
    }

    #[test]
    fn terminal_commit_updates_summary_and_blocks_later_events() {
        let journal = EventJournal::open_in_memory().expect("journal");
        journal
            .append(&event(1, "run_started", serde_json::json!({})))
            .expect("start");
        let terminal = event(
            2,
            "run_terminated",
            serde_json::json!({ "outcome": { "kind": "completed" } }),
        );
        assert_eq!(
            journal.append_terminal(&terminal).expect("terminal"),
            AppendStatus::Appended
        );
        assert_eq!(
            journal
                .append_terminal(&terminal)
                .expect("terminal duplicate"),
            AppendStatus::Duplicate
        );
        let summary = journal
            .run_summary("run-1")
            .expect("summary")
            .expect("run summary");
        assert_eq!(summary.last_seq, 2);
        assert_eq!(summary.terminal_seq, Some(2));
        assert_eq!(summary.terminal_kind.as_deref(), Some("run_terminated"));
        assert_eq!(summary.terminal_payload, Some(terminal.payload.clone()));
        assert!(matches!(
            journal.append(&event(3, "thinking", Value::Null)),
            Err(JournalError::RunAlreadyTerminated { .. })
        ));
        assert!(matches!(
            journal.append_terminal(&event(3, "run_terminated", Value::Null)),
            Err(JournalError::TerminalAlreadyCommitted { .. })
        ));
    }

    #[test]
    fn concurrent_terminal_race_commits_exactly_one_outcome() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("race.sqlite3");
        EventJournal::open(&path)
            .expect("seed journal")
            .append(&event(1, "run_started", serde_json::json!({})))
            .expect("seed event");
        let journal_a = EventJournal::open(&path).expect("journal a");
        let journal_b = EventJournal::open(&path).expect("journal b");
        let barrier = Arc::new(Barrier::new(3));
        let barrier_a = barrier.clone();
        let barrier_b = barrier.clone();
        let handle_a = std::thread::spawn(move || {
            barrier_a.wait();
            journal_a.append_terminal(&event(
                2,
                "run_terminated",
                serde_json::json!({ "winner": "a" }),
            ))
        });
        let handle_b = std::thread::spawn(move || {
            barrier_b.wait();
            journal_b.append_terminal(&event(
                2,
                "run_terminated",
                serde_json::json!({ "winner": "b" }),
            ))
        });
        barrier.wait();
        let outcomes = [
            handle_a.join().expect("thread a"),
            handle_b.join().expect("thread b"),
        ];

        assert_eq!(
            outcomes
                .iter()
                .filter(|outcome| matches!(outcome, Ok(AppendStatus::Appended)))
                .count(),
            1
        );
        assert_eq!(
            outcomes
                .iter()
                .filter(|outcome| matches!(
                    outcome,
                    Err(JournalError::TerminalAlreadyCommitted { .. })
                ))
                .count(),
            1
        );
        let summary = EventJournal::open(&path)
            .expect("verify journal")
            .run_summary("run-1")
            .expect("summary")
            .expect("run summary");
        assert_eq!(summary.terminal_seq, Some(2));
    }
}
