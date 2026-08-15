use serde::{Deserialize, Serialize};

/// Structured file-edit diff attached to a clarification. Field names and
/// serde behavior are the existing Desktop `agent://event` contract.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EditDiffPayload {
    pub file: String,
    pub diff: String,
    pub truncated: bool,
}

/// Versioned event payload shared by every ALTAI host.
///
/// The snake_case tags are consumed directly by the Desktop webview. Keep
/// additions backwards compatible; this is a public host boundary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    RunStarted {
        run_id: String,
    },
    RunWarning {
        run_id: String,
        warning: serde_json::Value,
    },
    RunWarningCleared {
        run_id: String,
    },
    RunTerminated {
        run_id: String,
        outcome: serde_json::Value,
    },
    AgentMessage {
        content: String,
        role: String,
    },
    ToolCallStart {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolCallEnd {
        id: String,
        name: String,
        output: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    EditDiff {
        file: String,
        before: String,
        after: String,
        hunk_id: String,
    },
    ApprovalRequest {
        id: String,
        action: String,
        payload: serde_json::Value,
    },
    Thinking {
        content: String,
    },
    Clarification {
        content: String,
        choices: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        edit_diff: Option<EditDiffPayload>,
    },
    Usage {
        prompt_tokens: u32,
        completion_tokens: u32,
        total_tokens: u32,
        cache_read_tokens: u32,
        cache_creation_tokens: u32,
    },
    ExecutionRunFinished {
        provider_id: String,
        session_id: String,
        exit_code: Option<i32>,
        duration_ms: u64,
        stdout_len: usize,
        stderr_len: usize,
        artifact_count: usize,
        git_head: Option<String>,
        description: Option<String>,
    },
    ExecutionJobFinished {
        job_id: String,
        session_id: String,
        provider_id: String,
        status: String,
        exit_code: Option<i32>,
        duration_ms: u64,
        stdout_len: usize,
        stderr_len: usize,
        artifact_count: usize,
        description: Option<String>,
    },
    BackgroundJobUpdated {
        job_id: String,
        state: String,
        kind: String,
        detail: Option<String>,
    },
    NotificationCreated {
        notification_id: String,
        kind: String,
        title: String,
    },
    NotificationUpdated {
        notification_id: String,
        state: String,
    },
    SubagentSpawned {
        task_id: String,
        child_chat_id: String,
        display_name: Option<String>,
        agent_name: Option<String>,
        background_job_id: Option<String>,
    },
    SubagentFinished {
        task_id: String,
        child_chat_id: String,
        status: String,
        agent_name: Option<String>,
    },
    StreamDelta {
        chunk: serde_json::Value,
    },
    /// Server-owned snapshot of the current session state. `projection_seq`
    /// belongs to the projection stream, not the run-event journal sequence.
    SessionProjection {
        projection_seq: u64,
        timestamp_rfc3339: String,
        run_status: String,
        todos: Vec<serde_json::Value>,
        subagents: Vec<serde_json::Value>,
        jobs: Vec<serde_json::Value>,
    },
    NotebookOutput {
        notebook_id: String,
        cell_index: usize,
        output: serde_json::Value,
    },
    ExperimentResult {
        experiment_id: String,
        metrics: serde_json::Value,
        artifacts: Vec<String>,
    },
}

impl Event {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::RunTerminated { .. })
    }
}

/// Scope value serialized as the stable `scope` field in an agent envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentEventScope {
    Run,
    System,
}

impl AgentEventScope {
    fn as_str(self) -> &'static str {
        match self {
            Self::Run => "run",
            Self::System => "system",
        }
    }
}

/// Owned form of the public `agent://event` envelope.
///
/// Field ordering intentionally matches the historic Desktop serialization:
/// `version`, `scope`, `runId`, `seq`, `chatId`, `event`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentEventEnvelope {
    pub version: u8,
    pub scope: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seq: Option<u64>,
    pub chat_id: String,
    pub event: serde_json::Value,
}

impl AgentEventEnvelope {
    pub fn run(
        chat_id: impl Into<String>,
        run_id: impl Into<String>,
        seq: u64,
        event: serde_json::Value,
    ) -> Self {
        Self {
            version: 1,
            scope: AgentEventScope::Run.as_str().to_string(),
            run_id: Some(run_id.into()),
            seq: Some(seq),
            chat_id: chat_id.into(),
            event,
        }
    }

    pub fn system(chat_id: impl Into<String>, event: serde_json::Value) -> Self {
        Self {
            version: 1,
            scope: AgentEventScope::System.as_str().to_string(),
            run_id: None,
            seq: None,
            chat_id: chat_id.into(),
            event,
        }
    }

    pub fn is_terminal(&self) -> bool {
        self.event
            .get("type")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|kind| kind == "run_terminated")
    }
}
