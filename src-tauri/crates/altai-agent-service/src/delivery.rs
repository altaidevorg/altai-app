//! Journal-then-sink delivery for host-neutral agent events.

use altai_core::journal::{AppendStatus, EventJournal, JournalEvent};

use crate::event::{AgentEventEnvelope, EditDiffPayload, Event};
use crate::routing::{coordinator_guard, SharedRunCoordinator};
use crate::sink::AgentEventSink;

#[derive(Debug)]
pub enum RunEventDeliveryError {
    Serialization,
    Transition(String),
    Persistence(String),
    Renderer(String),
}

impl std::fmt::Display for RunEventDeliveryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Serialization => formatter.write_str("agent_event_serialization_failed"),
            Self::Transition(detail) => {
                write!(formatter, "agent_event_transition_rejected: {detail}")
            }
            Self::Persistence(detail) => {
                write!(formatter, "agent_event_persistence_failed: {detail}")
            }
            Self::Renderer(detail) => {
                write!(formatter, "agent_event_renderer_unavailable: {detail}")
            }
        }
    }
}

pub fn emit_event(
    sink: &dyn AgentEventSink,
    chat_id: &str,
    event: &Event,
    run: Option<(String, u64)>,
) -> Result<(), RunEventDeliveryError> {
    let payload = redacted_event_payload(event)?;
    emit_payload(sink, chat_id, &payload, run)
}

pub fn emit_payload(
    sink: &dyn AgentEventSink,
    chat_id: &str,
    payload: &serde_json::Value,
    run: Option<(String, u64)>,
) -> Result<(), RunEventDeliveryError> {
    let envelope = match run {
        Some((run_id, seq)) => AgentEventEnvelope::run(chat_id, run_id, seq, payload.clone()),
        None => AgentEventEnvelope::system(chat_id, payload.clone()),
    };
    sink.try_send(envelope)
        .map_err(|error| RunEventDeliveryError::Renderer(error.to_string()))
}

pub fn redacted_event_payload(event: &Event) -> Result<serde_json::Value, RunEventDeliveryError> {
    let mut payload =
        serde_json::to_value(event).map_err(|_| RunEventDeliveryError::Serialization)?;
    isanagent::redact::shared().redact_json(&mut payload);
    Ok(payload)
}

pub enum RunEventTransition<'a> {
    Started(&'a str),
    Next,
    NextForRun(&'a str),
    Terminated(&'a str),
}

pub fn persist_run_event(
    coordinator: &SharedRunCoordinator,
    journal: &EventJournal,
    chat_id: &str,
    owner_id: &str,
    event: &Event,
    transition: RunEventTransition<'_>,
) -> Result<(String, u64), RunEventDeliveryError> {
    let payload = redacted_event_payload(event)?;
    persist_run_payload(
        coordinator,
        journal,
        chat_id,
        owner_id,
        &payload,
        transition,
    )
}

pub fn persist_run_payload(
    coordinator: &SharedRunCoordinator,
    journal: &EventJournal,
    chat_id: &str,
    owner_id: &str,
    payload: &serde_json::Value,
    transition: RunEventTransition<'_>,
) -> Result<(String, u64), RunEventDeliveryError> {
    let kind = payload
        .get("type")
        .and_then(serde_json::Value::as_str)
        .ok_or(RunEventDeliveryError::Serialization)?
        .to_string();

    // Sequence assignment and durable append are one coordinator transition.
    // SQLite I/O is intentionally performed while the coordinator is locked:
    // otherwise two producers could reserve the same next sequence or a later
    // event could overtake a failed append and leave a permanent journal gap.
    let mut coordinator = coordinator_guard(coordinator);
    let before = coordinator.clone();
    let run = match transition {
        RunEventTransition::Started(run_id) => coordinator.started(chat_id, run_id, owner_id),
        RunEventTransition::Next => coordinator.next(chat_id, owner_id),
        RunEventTransition::NextForRun(run_id) => {
            coordinator.next_for_run(chat_id, run_id, owner_id)
        }
        RunEventTransition::Terminated(run_id) => coordinator.terminated(chat_id, run_id, owner_id),
    }
    .map_err(|error| RunEventDeliveryError::Transition(format!("{error:?}")))?;

    let journal_event = JournalEvent::now(1, run.0.clone(), run.1, chat_id, kind, payload.clone());
    let append = if matches!(transition, RunEventTransition::Terminated(_)) {
        journal.append_terminal(&journal_event)
    } else {
        journal.append(&journal_event)
    };
    match append {
        Ok(AppendStatus::Appended | AppendStatus::Duplicate) => Ok(run),
        Err(error) => {
            *coordinator = before;
            Err(RunEventDeliveryError::Persistence(error.to_string()))
        }
    }
}

pub fn deliver_next_run_event(
    sink: &dyn AgentEventSink,
    journal: &EventJournal,
    coordinator: &SharedRunCoordinator,
    chat_id: &str,
    owner_id: &str,
    event: &Event,
) -> Result<(), String> {
    let result = persist_and_deliver_to_renderer(
        sink,
        journal,
        coordinator,
        chat_id,
        owner_id,
        event,
        RunEventTransition::Next,
    );
    match result {
        Ok(_) => Ok(()),
        // Persistence already advanced the durable sequence. A disconnected
        // renderer recovers it via replay, so surfacing delivery failure to
        // IsanAgent would only invite a duplicate semantic event.
        Err(error @ RunEventDeliveryError::Renderer(_)) => {
            log::warn!("Agent event for chat {chat_id} awaits replay: {error}");
            Ok(())
        }
        Err(error) => Err(error.to_string()),
    }
}

pub fn persist_and_deliver_to_renderer(
    sink: &dyn AgentEventSink,
    journal: &EventJournal,
    coordinator: &SharedRunCoordinator,
    chat_id: &str,
    owner_id: &str,
    event: &Event,
    transition: RunEventTransition<'_>,
) -> Result<(String, u64), RunEventDeliveryError> {
    persist_and_deliver_run_event(
        coordinator,
        journal,
        chat_id,
        owner_id,
        event,
        transition,
        |run, payload| emit_payload(sink, chat_id, payload, Some(run.clone())),
    )
}

pub fn persist_and_deliver_run_event<F>(
    coordinator: &SharedRunCoordinator,
    journal: &EventJournal,
    chat_id: &str,
    owner_id: &str,
    event: &Event,
    transition: RunEventTransition<'_>,
    deliver: F,
) -> Result<(String, u64), RunEventDeliveryError>
where
    F: FnOnce(&(String, u64), &serde_json::Value) -> Result<(), RunEventDeliveryError>,
{
    let payload = redacted_event_payload(event)?;
    let run = persist_run_payload(
        coordinator,
        journal,
        chat_id,
        owner_id,
        &payload,
        transition,
    )?;
    deliver(&run, &payload)?;
    Ok(run)
}

pub fn is_system_event(event: &Event) -> bool {
    matches!(
        event,
        Event::BackgroundJobUpdated { .. }
            | Event::NotificationCreated { .. }
            | Event::NotificationUpdated { .. }
    )
}

pub fn parse_edit_diff(value: &serde_json::Value) -> Option<EditDiffPayload> {
    let obj = value.as_object()?;
    let file = obj.get("file").and_then(|v| v.as_str())?.to_string();
    let diff = obj.get("diff").and_then(|v| v.as_str())?.to_string();
    let truncated = obj
        .get("truncated")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    Some(EditDiffPayload {
        file,
        diff,
        truncated,
    })
}


/// Inject a fresh run id for trusted host-owned synthetic inbound turns.
pub fn trusted_inbound(
    mut inbound: isanagent::bus::InboundMessage,
) -> isanagent::bus::InboundMessage {
    inbound.metadata.insert(
        isanagent::bus::METADATA_RUN_ID.to_string(),
        serde_json::Value::String(uuid::Uuid::new_v4().to_string()),
    );
    inbound
}

pub fn inbound_run_id(inbound: &isanagent::bus::InboundMessage) -> Option<&str> {
    inbound
        .metadata
        .get(isanagent::bus::METADATA_RUN_ID)
        .and_then(serde_json::Value::as_str)
        .filter(|run_id| !run_id.trim().is_empty())
}
