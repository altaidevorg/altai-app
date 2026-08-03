//! Desktop channel type is the shared [`ServiceChannel`] configured for `"tauri"`.

/// Historical name for the Desktop host channel.
#[allow(dead_code)]
pub type TauriChannel = altai_agent_service::ServiceChannel;

#[cfg(test)]
mod tests {
    use altai_agent_service::{
        map_lifecycle_to_event, map_telemetry_to_event, telemetry_chat_id, Event,
    };
    use isanagent::bus::{RunLifecycleEvent, TelemetryEvent};

    // --- map_telemetry_to_event exhaustive coverage ---

    fn event_type(e: &Event) -> &str {
        match e {
            Event::RunStarted { .. } => "run_started",
            Event::RunWarning { .. } => "run_warning",
            Event::RunWarningCleared { .. } => "run_warning_cleared",
            Event::RunTerminated { .. } => "run_terminated",
            Event::AgentMessage { .. } => "agent_message",
            Event::ToolCallStart { .. } => "tool_call_start",
            Event::ToolCallEnd { .. } => "tool_call_end",
            Event::EditDiff { .. } => "edit_diff",
            Event::ApprovalRequest { .. } => "approval_request",
            Event::Thinking { .. } => "thinking",
            Event::Clarification { .. } => "clarification",
            Event::Usage { .. } => "usage",
            Event::ExecutionRunFinished { .. } => "execution_run_finished",
            Event::ExecutionJobFinished { .. } => "execution_job_finished",
            Event::BackgroundJobUpdated { .. } => "background_job_updated",
            Event::NotificationCreated { .. } => "notification_created",
            Event::NotificationUpdated { .. } => "notification_updated",
            Event::SubagentSpawned { .. } => "subagent_spawned",
            Event::SubagentFinished { .. } => "subagent_finished",
            Event::NotebookOutput { .. } => "notebook_output",
            Event::ExperimentResult { .. } => "experiment_result",
        }
    }

    fn te_tool_call() -> isanagent::bus::TelemetryEvent {
        isanagent::bus::TelemetryEvent::ToolCall {
            chat_id: "c1".into(),
            channel: "tauri".into(),
            tool_name: "read_file".into(),
            args: r#"{"path":"/x"}"#.into(),
            tool_call_id: Some("tc1".into()),
            background_job_id: None,
        }
    }

    fn te_tool_result() -> isanagent::bus::TelemetryEvent {
        isanagent::bus::TelemetryEvent::ToolResult {
            chat_id: "c1".into(),
            channel: "tauri".into(),
            tool_name: "read_file".into(),
            result: "hello".into(),
            is_error: false,
            tool_call_id: Some("tc1".into()),
            background_job_id: None,
        }
    }

    fn te_agent_thought() -> isanagent::bus::TelemetryEvent {
        isanagent::bus::TelemetryEvent::AgentThought {
            chat_id: "c1".into(),
            thought: "hmm".into(),
            background_job_id: None,
        }
    }

    fn te_agent_usage() -> isanagent::bus::TelemetryEvent {
        isanagent::bus::TelemetryEvent::AgentUsage {
            chat_id: "c1".into(),
            model: "gpt-4".into(),
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
            cache_read_tokens: 0,
            cache_creation_tokens: 0,
            background_job_id: None,
        }
    }

    fn te_tool_progress() -> isanagent::bus::TelemetryEvent {
        isanagent::bus::TelemetryEvent::ToolProgress {
            chat_id: "c1".into(),
            channel: "tauri".into(),
            tool_name: "execution_run".into(),
            tool_call_id: Some("tc2".into()),
            message: "installing deps".into(),
            background_job_id: None,
        }
    }

    fn te_execution_run_finished() -> isanagent::bus::TelemetryEvent {
        isanagent::bus::TelemetryEvent::ExecutionRunFinished {
            chat_id: "c1".into(),
            channel: "tauri".into(),
            provider_id: "local".into(),
            session_id: "s1".into(),
            exit_code: Some(0),
            duration_ms: 1200,
            stdout_len: 42,
            stderr_len: 0,
            artifact_count: 3,
            git_head: Some("abc123".into()),
            description: Some("train".into()),
        }
    }

    fn te_execution_job_finished() -> isanagent::bus::TelemetryEvent {
        isanagent::bus::TelemetryEvent::ExecutionJobFinished {
            chat_id: "c1".into(),
            channel: "tauri".into(),
            job_id: "j1".into(),
            session_id: "s1".into(),
            provider_id: "local".into(),
            status: "completed".into(),
            duration_ms: 5000,
            exit_code: Some(0),
            stdout_len: 100,
            stderr_len: 5,
            artifact_count: 1,
            description: Some("bg job".into()),
        }
    }

    fn te_background_job_updated() -> isanagent::bus::TelemetryEvent {
        isanagent::bus::TelemetryEvent::BackgroundJobUpdated {
            job_id: "j1".into(),
            chat_id: "c1".into(),
            channel: "tauri".into(),
            state: "running".into(),
            kind: "execution".into(),
            detail: Some("step 2/5".into()),
        }
    }

    fn te_subagent_spawned() -> isanagent::bus::TelemetryEvent {
        isanagent::bus::TelemetryEvent::SubagentSpawned {
            parent_chat_id: "c1".into(),
            child_chat_id: "c2".into(),
            task_id: "t1".into(),
            // Distinct values so the mapping test proves display_name and
            // agent_name aren't cross-wired.
            display_name: Some("Research run #2".into()),
            agent_name: Some("researcher".into()),
            background_job_id: None,
        }
    }

    fn te_subagent_finished() -> isanagent::bus::TelemetryEvent {
        isanagent::bus::TelemetryEvent::SubagentFinished {
            parent_chat_id: "c1".into(),
            child_chat_id: "c2".into(),
            task_id: "t1".into(),
            status: "completed".into(),
            agent_name: Some("researcher".into()),
        }
    }

    // Variants that should hit _ => None
    fn te_cron_trigger() -> isanagent::bus::TelemetryEvent {
        isanagent::bus::TelemetryEvent::CronTrigger {
            job_id: "cj1".into(),
            message: "tick".into(),
        }
    }

    fn te_notification_created() -> isanagent::bus::TelemetryEvent {
        isanagent::bus::TelemetryEvent::NotificationCreated {
            notification_id: "n1".into(),
            chat_id: "c1".into(),
            channel: "tauri".into(),
            kind: "info".into(),
            title: "Job done".into(),
        }
    }

    fn te_notification_updated() -> isanagent::bus::TelemetryEvent {
        isanagent::bus::TelemetryEvent::NotificationUpdated {
            notification_id: "n1".into(),
            chat_id: "c1".into(),
            channel: "tauri".into(),
            state: "seen".into(),
        }
    }

    #[test]
    fn tool_call_maps_to_tool_call_start() {
        let e = map_telemetry_to_event(&te_tool_call()).unwrap();
        assert_eq!(event_type(&e), "tool_call_start");
        if let Event::ToolCallStart { id, name, input } = e {
            assert_eq!(id, "tc1");
            assert_eq!(name, "read_file");
            assert_eq!(input, serde_json::json!({"path": "/x"}));
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn tool_result_maps_to_tool_call_end() {
        let e = map_telemetry_to_event(&te_tool_result()).unwrap();
        assert_eq!(event_type(&e), "tool_call_end");
        if let Event::ToolCallEnd {
            id,
            name,
            output,
            error,
        } = e
        {
            assert_eq!(id, "tc1");
            assert_eq!(name, "read_file");
            assert_eq!(output, serde_json::Value::String("hello".into()));
            // is_error: false → no error, output carried normally.
            assert!(error.is_none());
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn failed_tool_result_surfaces_error_text() {
        // is_error: true → the result text rides through as `error` so the
        // frontend renders the tool cell in its `output-error` state.
        let te = isanagent::bus::TelemetryEvent::ToolResult {
            chat_id: "c1".into(),
            channel: "tauri".into(),
            tool_name: "edit_file".into(),
            result: "Error: old_text not found".into(),
            is_error: true,
            tool_call_id: Some("tc1".into()),
            background_job_id: None,
        };
        let e = map_telemetry_to_event(&te).unwrap();
        if let Event::ToolCallEnd { error, output, .. } = e {
            assert_eq!(error.as_deref(), Some("Error: old_text not found"));
            // output still carries the same text; the frontend prefers `error`.
            assert_eq!(
                output,
                serde_json::Value::String("Error: old_text not found".into())
            );
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn agent_thought_maps_to_thinking() {
        let e = map_telemetry_to_event(&te_agent_thought()).unwrap();
        assert_eq!(event_type(&e), "thinking");
        if let Event::Thinking { content } = e {
            assert_eq!(content, "hmm");
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn agent_usage_maps_to_usage() {
        let e = map_telemetry_to_event(&te_agent_usage()).unwrap();
        assert_eq!(event_type(&e), "usage");
        if let Event::Usage {
            prompt_tokens,
            completion_tokens,
            total_tokens,
            ..
        } = e
        {
            assert_eq!(prompt_tokens, 100);
            assert_eq!(completion_tokens, 50);
            assert_eq!(total_tokens, 150);
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn tool_progress_maps_to_thinking_with_prefix() {
        let e = map_telemetry_to_event(&te_tool_progress()).unwrap();
        assert_eq!(event_type(&e), "thinking");
        if let Event::Thinking { content } = e {
            assert!(content.starts_with("[execution_run]"));
            assert!(content.contains("installing deps"));
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn execution_run_finished_round_trips_fields() {
        let e = map_telemetry_to_event(&te_execution_run_finished()).unwrap();
        assert_eq!(event_type(&e), "execution_run_finished");
        if let Event::ExecutionRunFinished {
            provider_id,
            session_id,
            exit_code,
            duration_ms,
            stdout_len,
            stderr_len,
            artifact_count,
            git_head,
            description,
        } = e
        {
            assert_eq!(provider_id, "local");
            assert_eq!(session_id, "s1");
            assert_eq!(exit_code, Some(0));
            assert_eq!(duration_ms, 1200);
            assert_eq!(stdout_len, 42);
            assert_eq!(stderr_len, 0);
            assert_eq!(artifact_count, 3);
            assert_eq!(git_head.unwrap(), "abc123");
            assert_eq!(description.unwrap(), "train");
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn execution_job_finished_round_trips_fields() {
        let e = map_telemetry_to_event(&te_execution_job_finished()).unwrap();
        assert_eq!(event_type(&e), "execution_job_finished");
        if let Event::ExecutionJobFinished {
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
        } = e
        {
            assert_eq!(job_id, "j1");
            assert_eq!(session_id, "s1");
            assert_eq!(provider_id, "local");
            assert_eq!(status, "completed");
            assert_eq!(exit_code, Some(0));
            assert_eq!(duration_ms, 5000);
            assert_eq!(stdout_len, 100);
            assert_eq!(stderr_len, 5);
            assert_eq!(artifact_count, 1);
            assert_eq!(description.unwrap(), "bg job");
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn background_job_updated_maps_correctly() {
        let e = map_telemetry_to_event(&te_background_job_updated()).unwrap();
        assert_eq!(event_type(&e), "background_job_updated");
        if let Event::BackgroundJobUpdated {
            job_id,
            state,
            kind,
            detail,
        } = e
        {
            assert_eq!(job_id, "j1");
            assert_eq!(state, "running");
            assert_eq!(kind, "execution");
            assert_eq!(detail.unwrap(), "step 2/5");
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn subagent_spawned_maps_to_subagent_spawned() {
        let e = map_telemetry_to_event(&te_subagent_spawned()).unwrap();
        assert_eq!(event_type(&e), "subagent_spawned");
        if let Event::SubagentSpawned {
            task_id,
            child_chat_id,
            display_name,
            agent_name,
            background_job_id,
        } = e
        {
            assert_eq!(task_id, "t1");
            assert_eq!(child_chat_id, "c2");
            assert_eq!(display_name.as_deref(), Some("Research run #2"));
            assert_eq!(agent_name.as_deref(), Some("researcher"));
            assert_eq!(background_job_id, None);
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn subagent_events_route_by_parent_chat_id() {
        // The UI filters per session on the parent chat, so routing must
        // resolve to `parent_chat_id` ("c1"), not the child ("c2").
        assert_eq!(telemetry_chat_id(&te_subagent_spawned()), Some("c1"));
        assert_eq!(telemetry_chat_id(&te_subagent_finished()), Some("c1"));
    }

    #[test]
    fn subagent_finished_maps_to_subagent_finished() {
        let e = map_telemetry_to_event(&te_subagent_finished()).unwrap();
        assert_eq!(event_type(&e), "subagent_finished");
        if let Event::SubagentFinished {
            task_id,
            child_chat_id,
            status,
            agent_name,
        } = e
        {
            assert_eq!(task_id, "t1");
            assert_eq!(child_chat_id, "c2");
            assert_eq!(status, "completed");
            assert_eq!(agent_name.as_deref(), Some("researcher"));
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn cron_trigger_falls_through_to_none() {
        assert!(map_telemetry_to_event(&te_cron_trigger()).is_none());
    }

    #[test]
    fn notification_created_maps_for_tauri() {
        let event = map_telemetry_to_event(&te_notification_created()).unwrap();
        assert_eq!(event_type(&event), "notification_created");
        if let Event::NotificationCreated {
            notification_id,
            kind,
            title,
        } = event
        {
            assert_eq!(notification_id, "n1");
            assert_eq!(kind, "info");
            assert_eq!(title, "Job done");
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn notification_updated_maps_for_tauri() {
        let event = map_telemetry_to_event(&te_notification_updated()).unwrap();
        assert_eq!(event_type(&event), "notification_updated");
        if let Event::NotificationUpdated {
            notification_id,
            state,
        } = event
        {
            assert_eq!(notification_id, "n1");
            assert_eq!(state, "seen");
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn notification_from_another_channel_is_not_forwarded() {
        let mut event = te_notification_created();
        if let isanagent::bus::TelemetryEvent::NotificationCreated { channel, .. } = &mut event {
            *channel = "slack".into();
        }
        assert!(map_telemetry_to_event(&event).is_none());
    }

    #[test]
    fn tool_call_without_id_generates_uuid() {
        let te = isanagent::bus::TelemetryEvent::ToolCall {
            chat_id: "c1".into(),
            channel: "tauri".into(),
            tool_name: "read_file".into(),
            args: "{}".into(),
            tool_call_id: None,
            background_job_id: None,
        };
        let e = map_telemetry_to_event(&te).unwrap();
        if let Event::ToolCallStart { id, .. } = e {
            assert!(id.len() >= 32, "expected uuid-length id, got {:?}", id);
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn tool_result_without_id_falls_back_to_name() {
        let te = isanagent::bus::TelemetryEvent::ToolResult {
            chat_id: "c1".into(),
            channel: "tauri".into(),
            tool_name: "bash".into(),
            result: "ok".into(),
            is_error: false,
            tool_call_id: None,
            background_job_id: None,
        };
        let e = map_telemetry_to_event(&te).unwrap();
        if let Event::ToolCallEnd { id, .. } = e {
            assert_eq!(id, "bash");
        } else {
            panic!("wrong variant");
        }
    }

    /// Every mapped event type must serialise/deserialise with the
    /// `"type"` tag intact — the frontend discriminates on it.
    #[test]
    fn all_mapped_events_serialize_with_discriminant() {
        let events: Vec<Event> = vec![
            map_telemetry_to_event(&te_tool_call()).unwrap(),
            map_telemetry_to_event(&te_tool_result()).unwrap(),
            map_telemetry_to_event(&te_agent_thought()).unwrap(),
            map_telemetry_to_event(&te_agent_usage()).unwrap(),
            map_telemetry_to_event(&te_tool_progress()).unwrap(),
            map_telemetry_to_event(&te_execution_run_finished()).unwrap(),
            map_telemetry_to_event(&te_execution_job_finished()).unwrap(),
            map_telemetry_to_event(&te_background_job_updated()).unwrap(),
            map_telemetry_to_event(&te_subagent_spawned()).unwrap(),
            map_telemetry_to_event(&te_subagent_finished()).unwrap(),
        ];

        for e in &events {
            let json = serde_json::to_string(e).unwrap();
            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
            let tag = parsed["type"].as_str().unwrap();
            assert_eq!(tag, event_type(e));
            // Round-trip through the Value representation
            let e2: Event = serde_json::from_value(parsed).unwrap();
            let json2 = serde_json::to_string(&e2).unwrap();
            assert_eq!(json, json2);
        }
    }
}
