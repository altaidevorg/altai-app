//! Wire-facing acknowledgements for run control plane methods.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendAck {
    pub chat_id: String,
    pub run_id: String,
    pub queued: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelAck {
    pub chat_id: String,
    pub run_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SteerAck {
    pub chat_id: String,
    pub run_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualCompactionAck {
    pub chat_id: String,
}

/// Attachment document forwarded into the host channel.
#[derive(Debug, Clone)]
pub struct DocumentPart {
    pub data: String,
    pub media_type: String,
    pub name: Option<String>,
}
