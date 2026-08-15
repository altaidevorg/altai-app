//! Durable external-object contracts. An [`ExternalObject`] is one object a
//! tracker integration (GitHub, Gmail, …) owns — identified by the provider's
//! immutable id, not its mutable number or title — linked to at most one local
//! Work item. Package 070's sync is idempotent per `content_hash`, and
//! conflicts between the two sides are resolved by the object's recorded
//! [`ExternalAuthority`], never by write order. Nothing here performs network
//! I/O: adapters fetch, this model records.

use crate::{ExternalObjectId, OrganizationId, WorkItemId};
use serde::{Deserialize, Serialize};

/// Which side is the source of truth when the same external object changed
/// locally and at the provider. Authority is recorded per object, so a sync
/// run resolves conflicts by rule instead of by arrival order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalAuthority {
    /// The provider wins: local edits are advisory and are overwritten.
    External,
    /// The local Work item wins: provider changes are surfaced as conflicts,
    /// never applied.
    Local,
}

/// One provider-owned object, linked to at most one local Work item.
/// `content_hash` is the adapter's hash of the mapped provider payload —
/// the idempotency token a sync run compares before writing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalObject {
    pub id: ExternalObjectId,
    pub organization_id: OrganizationId,
    /// Owning integration name (e.g. "github"); part of the upsert key.
    pub integration: String,
    /// The provider's immutable id for this object (not its number).
    pub external_id: String,
    /// Provider object type (e.g. "issue", "pull_request").
    pub object_kind: String,
    pub url: Option<String>,
    pub title: String,
    /// Hash of the mapped provider payload; equal hashes mean equal content.
    pub content_hash: String,
    /// Conflict-resolution rule for this object.
    pub authority: ExternalAuthority,
    /// The local Work item this object is linked to, if any.
    pub linked_work_item_id: Option<WorkItemId>,
    /// Provider-reported last change, for sync windows.
    pub external_updated_at_unix_seconds: Option<u64>,
    pub last_synced_at_unix_seconds: u64,
    pub created_at_unix_seconds: u64,
    pub updated_at_unix_seconds: u64,
}
