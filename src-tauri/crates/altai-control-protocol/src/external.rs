//! Durable external-object contracts. An [`ExternalObject`] is one object a
//! tracker integration (GitHub, Gmail, …) owns — identified by the provider's
//! immutable id, not its mutable number or title — linked to at most one local
//! Work item. Package 070's sync is idempotent per `content_hash`, and
//! conflicts between the two sides are resolved by the object's recorded
//! [`ExternalAuthority`], never by write order. Nothing here performs network
//! I/O: adapters fetch, this model records.

use crate::{ExternalAccountId, ExternalObjectId, OrganizationId, WorkItemId};
use serde::{Deserialize, Serialize};

/// One connected account at an external provider (package 074). The
/// account is the isolation boundary: objects and credentials belong to
/// exactly one account, and a second account at the same integration
/// (a second Gmail mailbox) never sees the first account's data. This
/// type is metadata only — credential material is never carried here;
/// the host brokers it separately, scoped to the account.
///
/// `account_ref` is the provider's stable identity for the account
/// (for Gmail, the mailbox address), and `(integration, account_ref)`
/// is the upsert key: connecting the same account again resolves to the
/// same row, never a duplicate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalAccount {
    pub id: ExternalAccountId,
    pub organization_id: OrganizationId,
    /// Owning integration name (e.g. "gmail"); part of the upsert key.
    pub integration: String,
    /// The provider's stable identity for this account (e.g. the
    /// mailbox address), not a display name.
    pub account_ref: String,
    /// Human-readable label for surfaces; purely presentational.
    pub display_name: String,
    pub created_at_unix_seconds: u64,
    pub updated_at_unix_seconds: u64,
}

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
    /// The account this object belongs to (package 074). `None` marks
    /// an unattributed object of a single-account integration; an
    /// account-backed integration (Gmail) always sets it, making the
    /// upsert key `(integration, account, external_id)` — two accounts
    /// never collide, never share.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_id: Option<ExternalAccountId>,
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
    /// The external content the last sync refused, awaiting a decision.
    /// Presenting state for a resolver — it is not itself a decision.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refused_content_hash: Option<String>,
    /// The external content a `KeepLocal` resolution dismissed. An
    /// identical provider payload stops re-conflicting; a new hash does not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub declined_content_hash: Option<String>,
    /// The local Work item this object is linked to, if any.
    pub linked_work_item_id: Option<WorkItemId>,
    /// Provider-reported last change, for sync windows.
    pub external_updated_at_unix_seconds: Option<u64>,
    pub last_synced_at_unix_seconds: u64,
    pub created_at_unix_seconds: u64,
    pub updated_at_unix_seconds: u64,
}
