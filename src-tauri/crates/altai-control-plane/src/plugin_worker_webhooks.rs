//! CP-08 webhook delivery to plugin workers (package 072, PR 5). A
//! plugin with the `Webhooks` capability can be handed inbound webhook
//! events; these are the frame types that travel on the
//! [`worker transport`](crate::plugin_worker_transport). Delivery uses
//! the same at-most-once contract as jobs — one
//! [`DispatchLedger`](crate::plugin_worker_jobs::DispatchLedger) per
//! family, so a delivery id is sent once, ever.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Host → worker: one inbound webhook delivery. `delivery_id` is the
/// idempotency key.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebhookDelivery {
    pub delivery_id: String,
    pub event: String,
    pub payload: Value,
}

/// Worker → host: the delivery was processed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebhookAck {
    pub delivery_id: String,
    pub ok: bool,
}
