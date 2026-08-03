//! Lifecycle/telemetry → shared Event mappers (host-neutral).

use crate::event::Event;

/// Map typed reasoning-run lifecycle events to the stable ALTAI event surface.
/// The runtime adds the versioned envelope and monotonic sequence number.
pub fn map_lifecycle_to_event(lifecycle: &isanagent::bus::RunLifecycleEvent) -> Event {
    use isanagent::bus::RunLifecycleEvent;

    match lifecycle {
        RunLifecycleEvent::Started { run_id, .. } => Event::RunStarted {
            run_id: run_id.clone(),
        },
        RunLifecycleEvent::Warning {
            run_id, warning, ..
        } => Event::RunWarning {
            run_id: run_id.clone(),
            warning: serde_json::to_value(warning)
                .expect("run lifecycle warnings are serializable by contract"),
        },
        RunLifecycleEvent::WarningCleared { run_id, .. } => Event::RunWarningCleared {
            run_id: run_id.clone(),
        },
        RunLifecycleEvent::Terminated {
            run_id, outcome, ..
        } => Event::RunTerminated {
            run_id: run_id.clone(),
            outcome: serde_json::to_value(outcome)
                .expect("run lifecycle outcomes are serializable by contract"),
        },
    }
}

/// Forward IsanAgent telemetry events to the Tauri frontend.
///
/// Called from the bus router in runtime.rs to map `TelemetryEvent`
/// variants to the `Event` enum the frontend already understands.
pub fn map_telemetry_to_event(telemetry: &isanagent::bus::TelemetryEvent) -> Option<Event> {
    use isanagent::bus::TelemetryEvent;
    match telemetry {
        TelemetryEvent::ToolCall {
            tool_name,
            tool_call_id,
            args,
            ..
        } => Some(Event::ToolCallStart {
            id: tool_call_id
                .clone()
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            name: tool_name.clone(),
            input: serde_json::from_str(args).unwrap_or(serde_json::Value::String(args.clone())),
        }),
        TelemetryEvent::ToolResult {
            tool_name,
            tool_call_id,
            result,
            is_error,
            ..
        } => Some(Event::ToolCallEnd {
            id: tool_call_id.clone().unwrap_or_else(|| tool_name.clone()),
            name: tool_name.clone(),
            output: serde_json::Value::String(result.clone()),
            // isanagent sets `is_error` accurately for both in-band tool
            // failures (e.g. `edit_file` "old_text not found") and non-zero
            // `exec`/`python_run` exit codes. Forward it so the UI renders a
            // failed tool call in its error state instead of as successful
            // output. When `error` is set the frontend uses it as the error
            // body and omits `output`, so the text isn't duplicated.
            error: is_error.then(|| result.clone()),
        }),
        TelemetryEvent::AgentThought { thought, .. } => Some(Event::Thinking {
            content: thought.clone(),
        }),
        TelemetryEvent::AgentUsage {
            prompt_tokens,
            completion_tokens,
            total_tokens,
            cache_read_tokens,
            cache_creation_tokens,
            ..
        } => Some(Event::Usage {
            prompt_tokens: *prompt_tokens,
            completion_tokens: *completion_tokens,
            total_tokens: *total_tokens,
            cache_read_tokens: *cache_read_tokens,
            cache_creation_tokens: *cache_creation_tokens,
        }),
        TelemetryEvent::ToolProgress {
            tool_name, message, ..
        } => Some(Event::Thinking {
            content: format!("[{}] {}", tool_name, message),
        }),
        TelemetryEvent::ExecutionRunFinished {
            provider_id,
            session_id,
            exit_code,
            duration_ms,
            stdout_len,
            stderr_len,
            artifact_count,
            git_head,
            description,
            ..
        } => Some(Event::ExecutionRunFinished {
            provider_id: provider_id.clone(),
            session_id: session_id.clone(),
            exit_code: *exit_code,
            duration_ms: *duration_ms,
            stdout_len: *stdout_len,
            stderr_len: *stderr_len,
            artifact_count: *artifact_count,
            git_head: git_head.clone(),
            description: description.clone(),
        }),
        TelemetryEvent::ExecutionJobFinished {
            job_id,
            session_id,
            provider_id,
            status,
            exit_code,
            duration_ms,
            stdout_len,
            stderr_len,
            artifact_count,
            description,
            ..
        } => Some(Event::ExecutionJobFinished {
            job_id: job_id.clone(),
            session_id: session_id.clone(),
            provider_id: provider_id.clone(),
            status: status.clone(),
            exit_code: *exit_code,
            duration_ms: *duration_ms,
            stdout_len: *stdout_len,
            stderr_len: *stderr_len,
            artifact_count: *artifact_count,
            description: description.clone(),
        }),
        TelemetryEvent::BackgroundJobUpdated {
            job_id,
            state,
            kind,
            detail,
            ..
        } => Some(Event::BackgroundJobUpdated {
            job_id: job_id.clone(),
            state: state.clone(),
            kind: kind.clone(),
            detail: detail.clone(),
        }),
        TelemetryEvent::NotificationCreated {
            notification_id,
            channel,
            kind,
            title,
            ..
        } if channel == "tauri" || channel == "stdio" => Some(Event::NotificationCreated {
            notification_id: notification_id.clone(),
            kind: kind.clone(),
            title: title.clone(),
        }),
        TelemetryEvent::NotificationUpdated {
            notification_id,
            channel,
            state,
            ..
        } if channel == "tauri" || channel == "stdio" => Some(Event::NotificationUpdated {
            notification_id: notification_id.clone(),
            state: state.clone(),
        }),
        TelemetryEvent::SubagentSpawned {
            child_chat_id,
            task_id,
            display_name,
            agent_name,
            background_job_id,
            ..
        } => Some(Event::SubagentSpawned {
            task_id: task_id.clone(),
            child_chat_id: child_chat_id.clone(),
            display_name: display_name.clone(),
            agent_name: agent_name.clone(),
            background_job_id: background_job_id.clone(),
        }),
        TelemetryEvent::SubagentFinished {
            child_chat_id,
            task_id,
            status,
            agent_name,
            ..
        } => Some(Event::SubagentFinished {
            task_id: task_id.clone(),
            child_chat_id: child_chat_id.clone(),
            status: status.clone(),
            agent_name: agent_name.clone(),
        }),
        _ => None,
    }
}

/// The originating `chat_id` of a telemetry event, for per-chat event routing.
/// Returns `None` for variants that aren't scoped to a chat.
pub fn telemetry_chat_id(telemetry: &isanagent::bus::TelemetryEvent) -> Option<&str> {
    use isanagent::bus::TelemetryEvent::*;
    match telemetry {
        ToolCall { chat_id, .. }
        | ToolResult { chat_id, .. }
        | AgentThought { chat_id, .. }
        | AgentUsage { chat_id, .. }
        | ToolProgress { chat_id, .. }
        | ExecutionRunFinished { chat_id, .. }
        | ExecutionJobFinished { chat_id, .. }
        | BackgroundJobUpdated { chat_id, .. }
        | NotificationCreated { chat_id, .. }
        | NotificationUpdated { chat_id, .. } => Some(chat_id.as_str()),
        // Subagent events are scoped to the *parent* chat — that's the session
        // the UI filters on, so route them by `parent_chat_id`.
        SubagentSpawned { parent_chat_id, .. } | SubagentFinished { parent_chat_id, .. } => {
            Some(parent_chat_id.as_str())
        }
        _ => None,
    }
}

