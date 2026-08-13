//! Host-neutral IsanAgent channel backed by [`AgentEventSink`].

use std::any::Any;
use std::sync::Arc;

use async_trait::async_trait;
use isanagent::bus::{BusMessage, OutboundMessage, METADATA_RUN_ID};
use isanagent::channels::Channel;
use log::info;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;

use altai_core::journal::EventJournal;

use crate::acks::{DocumentPart, SendAck};
use crate::delivery::deliver_next_run_event;
use crate::event::Event;
use crate::host::HostControlPlane;
use crate::routing::{
    admit_queued_user_message, admit_user_message, rollback_run_admission, SharedRunCoordinator,
};
use crate::sink::AgentEventSink;

/// Long-lived channel shared by Desktop (`tauri`) and stdio (`stdio`) hosts.
pub struct ServiceChannel {
    event_sink: Arc<dyn AgentEventSink>,
    chat_id: String,
    owner_id: String,
    channel_name: &'static str,
    sender_id: &'static str,
    run_coordinator: SharedRunCoordinator,
    event_journal: Arc<EventJournal>,
    bus_tx: Mutex<Option<Sender<BusMessage>>>,
}

impl ServiceChannel {
    pub fn new(
        event_sink: Arc<dyn AgentEventSink>,
        chat_id: String,
        owner_id: String,
        run_coordinator: SharedRunCoordinator,
        event_journal: Arc<EventJournal>,
        channel_name: &'static str,
        sender_id: &'static str,
    ) -> Self {
        Self {
            event_sink,
            chat_id,
            owner_id,
            channel_name,
            sender_id,
            run_coordinator,
            event_journal,
            bus_tx: Mutex::new(None),
        }
    }

    pub fn tauri(
        event_sink: Arc<dyn AgentEventSink>,
        chat_id: String,
        owner_id: String,
        run_coordinator: SharedRunCoordinator,
        event_journal: Arc<EventJournal>,
    ) -> Self {
        Self::new(
            event_sink,
            chat_id,
            owner_id,
            run_coordinator,
            event_journal,
            "tauri",
            "tauri_user",
        )
    }

    pub fn stdio(
        event_sink: Arc<dyn AgentEventSink>,
        chat_id: String,
        owner_id: String,
        run_coordinator: SharedRunCoordinator,
        event_journal: Arc<EventJournal>,
    ) -> Self {
        Self::new(
            event_sink,
            chat_id,
            owner_id,
            run_coordinator,
            event_journal,
            "stdio",
            "stdio_user",
        )
    }

    pub fn chat_id(&self) -> &str {
        &self.chat_id
    }

    pub fn owner_id(&self) -> &str {
        &self.owner_id
    }
}

#[async_trait]
impl Channel for ServiceChannel {
    fn name(&self) -> &str {
        self.channel_name
    }

    async fn start(&self, bus_tx: Sender<BusMessage>) -> Result<(), String> {
        let mut guard = self.bus_tx.lock().await;
        *guard = Some(bus_tx);
        info!(
            "ServiceChannel ({}) started for chat_id={}",
            self.channel_name, self.chat_id
        );
        Ok(())
    }

    async fn stop(&self) -> Result<(), String> {
        let mut guard = self.bus_tx.lock().await;
        *guard = None;
        info!("ServiceChannel ({}) stopped", self.channel_name);
        Ok(())
    }

    async fn send(&self, msg: OutboundMessage) -> Result<(), String> {
        let chat_id = msg.chat_id;
        let event = Event::AgentMessage {
            content: msg.content,
            role: "assistant".to_string(),
        };
        deliver_next_run_event(
            self.event_sink.as_ref(),
            &self.event_journal,
            &self.run_coordinator,
            &chat_id,
            &self.owner_id,
            &event,
        )
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[async_trait]
impl HostControlPlane for ServiceChannel {
    async fn inject_user_message(
        &self,
        content: String,
        image_urls: Vec<String>,
        documents: Vec<DocumentPart>,
        chat_id: String,
        queue: bool,
        requested_run_id: Option<String>,
    ) -> Result<SendAck, String> {
        let guard = self.bus_tx.lock().await;
        let tx = guard
            .as_ref()
            .ok_or_else(|| format!("{} channel not started", self.channel_name))?;
        let mut attachments: Vec<_> = image_urls
            .into_iter()
            .map(|url| isanagent::utils::ContentPart::ImageUrl {
                image_url: isanagent::utils::ImageUrl { url, detail: None },
            })
            .collect();
        attachments.extend(documents.into_iter().map(|document| {
            isanagent::utils::ContentPart::Document {
                document: isanagent::utils::Document {
                    data: document.data,
                    media_type: document.media_type,
                    name: document.name,
                },
            }
        }));
        let chat_id = if chat_id.is_empty() {
            self.chat_id.clone()
        } else {
            chat_id
        };
        let requested_run_id = resolve_run_id(requested_run_id);
        let (run_id, queued) = if queue {
            admit_queued_user_message(
                &self.run_coordinator,
                &chat_id,
                &requested_run_id,
                &self.owner_id,
            )?
        } else {
            (
                admit_user_message(
                    &self.run_coordinator,
                    &chat_id,
                    &requested_run_id,
                    &self.owner_id,
                )?,
                false,
            )
        };
        let mut metadata = std::collections::HashMap::new();
        metadata.insert(
            METADATA_RUN_ID.to_string(),
            serde_json::json!(run_id.clone()),
        );
        let msg = isanagent::bus::InboundMessage {
            channel: self.name().to_string(),
            sender_id: self.sender_id.to_string(),
            chat_id: chat_id.clone(),
            thread_id: None,
            content,
            attachments,
            metadata,
        };
        let result = tx
            .send(BusMessage::Inbound(msg))
            .await
            .map_err(|e| format!("Failed to send to bus: {e}"));
        if result.is_err() {
            rollback_run_admission(&self.run_coordinator, &chat_id, &run_id, &self.owner_id);
        }
        result.map(|()| SendAck {
            chat_id,
            run_id,
            queued,
        })
    }

    async fn cancel_run(&self, chat_id: String, run_id: String) -> Result<(), String> {
        let guard = self.bus_tx.lock().await;
        let tx = guard
            .as_ref()
            .ok_or_else(|| format!("{} channel not started", self.channel_name))?;
        let chat_id = if chat_id.is_empty() {
            self.chat_id.clone()
        } else {
            chat_id
        };
        tx.send(BusMessage::CancelRun { chat_id, run_id })
            .await
            .map_err(|e| format!("Cancel failed: {e}"))
    }

    async fn steer_run(
        &self,
        chat_id: String,
        run_id: String,
        content: String,
    ) -> Result<(), String> {
        let guard = self.bus_tx.lock().await;
        let tx = guard
            .as_ref()
            .ok_or_else(|| format!("{} channel not started", self.channel_name))?;
        let chat_id = if chat_id.is_empty() {
            self.chat_id.clone()
        } else {
            chat_id
        };
        tx.send(BusMessage::Steer {
            chat_id,
            run_id,
            content,
        })
        .await
        .map_err(|e| format!("Steer failed: {e}"))
    }
}

fn resolve_run_id(requested_run_id: Option<String>) -> String {
    requested_run_id
        .map(|run_id| run_id.trim().to_string())
        .filter(|run_id| !run_id.is_empty())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
}

#[cfg(test)]
mod tests {
    use super::resolve_run_id;

    #[test]
    fn authorized_run_id_is_preserved_exactly() {
        assert_eq!(resolve_run_id(Some(" run_authorized ".into())), "run_authorized");
    }

    #[test]
    fn empty_requested_run_id_falls_back_to_a_new_identity() {
        assert!(!resolve_run_id(Some("   ".into())).trim().is_empty());
    }
}
