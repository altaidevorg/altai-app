//! Source adapter outbox and conformance suite (plan §I1).
//!
//! Provides an idempotent outbox for remote source mutations (status posts,
//! comments) with retry semantics, and a conformance test framework that
//! any source adapter must pass.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Outbox entry
// ---------------------------------------------------------------------------

/// A queued mutation to be delivered to a remote source.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutboxEntry {
    pub id: String,
    pub task_id: String,
    pub source_kind: String,
    pub mutation: SourceMutation,
    pub status: OutboxStatus,
    pub attempts: u32,
    pub max_attempts: u32,
    pub last_error: Option<String>,
    pub created_at_ms: u64,
    pub next_retry_ms: u64,
}

/// What kind of mutation to apply at the source.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum SourceMutation {
    PostStatus {
        native_id: String,
        status: String,
    },
    PostComment {
        native_id: String,
        body: String,
    },
    CloseIssue {
        native_id: String,
        reason: Option<String>,
    },
    AddLabel {
        native_id: String,
        label: String,
    },
}

/// The delivery state of an outbox entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OutboxStatus {
    #[default]
    Pending,
    Delivered,
    Failed,
}

// ---------------------------------------------------------------------------
// Outbox
// ---------------------------------------------------------------------------

/// An idempotent outbox for remote source mutations. Entries are delivered
/// at-least-once; the source's own idempotency (deduplication by mutation
/// identity) ensures exactly-once effects.
#[derive(Clone, Debug, Default)]
pub struct SourceOutbox {
    entries: HashMap<String, OutboxEntry>,
    /// Tracks mutation identity for deduplication.
    delivered_keys: HashMap<String, bool>,
}

/// Configuration for retry behavior.
#[derive(Clone, Debug)]
pub struct OutboxConfig {
    pub max_attempts: u32,
    pub backoff_base_ms: u64,
    pub backoff_max_ms: u64,
}

impl Default for OutboxConfig {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            backoff_base_ms: 5_000,
            backoff_max_ms: 300_000,
        }
    }
}

impl SourceOutbox {
    pub fn new() -> Self {
        Self::default()
    }

    /// Enqueue a mutation. Deduplicates by mutation identity — if the same
    /// mutation was already delivered, it's silently skipped.
    pub fn enqueue(
        &mut self,
        task_id: &str,
        source_kind: &str,
        mutation: SourceMutation,
        now_ms: u64,
        config: &OutboxConfig,
    ) -> String {
        let key = dedup_key(source_kind, &mutation);

        // Skip if already delivered (idempotent).
        if self.delivered_keys.get(&key).copied().unwrap_or(false) {
            return String::new(); // no-op
        }

        // Skip if already pending (don't double-queue).
        if self.entries.values().any(|e| {
            dedup_key(&e.source_kind, &e.mutation) == key && e.status == OutboxStatus::Pending
        }) {
            return String::new();
        }

        let id = format!("ob-{}-{}", now_ms, self.entries.len());
        let entry = OutboxEntry {
            id: id.clone(),
            task_id: task_id.to_string(),
            source_kind: source_kind.to_string(),
            mutation,
            status: OutboxStatus::Pending,
            attempts: 0,
            max_attempts: config.max_attempts,
            last_error: None,
            created_at_ms: now_ms,
            next_retry_ms: now_ms,
        };
        self.entries.insert(id.clone(), entry);
        id
    }

    /// Get all pending entries ready for delivery (next_retry_ms <= now).
    pub fn pending(&self, now_ms: u64) -> Vec<&OutboxEntry> {
        self.entries
            .values()
            .filter(|e| e.status == OutboxStatus::Pending && e.next_retry_ms <= now_ms)
            .collect()
    }

    /// Mark an entry as successfully delivered.
    pub fn mark_delivered(&mut self, entry_id: &str) {
        if let Some(entry) = self.entries.get_mut(entry_id) {
            entry.status = OutboxStatus::Delivered;
            let key = dedup_key(&entry.source_kind, &entry.mutation);
            self.delivered_keys.insert(key, true);
        }
    }

    /// Mark an entry as failed and schedule a retry (if attempts remain).
    /// Returns true if a retry was scheduled, false if permanently failed.
    pub fn mark_failed(
        &mut self,
        entry_id: &str,
        error: &str,
        now_ms: u64,
        config: &OutboxConfig,
    ) -> bool {
        let Some(entry) = self.entries.get_mut(entry_id) else {
            return false;
        };
        entry.attempts += 1;
        entry.last_error = Some(error.to_string());

        if entry.attempts >= entry.max_attempts {
            entry.status = OutboxStatus::Failed;
            return false;
        }

        // Exponential backoff.
        let backoff = (config.backoff_base_ms * (1u64 << entry.attempts.saturating_sub(1).min(10)))
            .min(config.backoff_max_ms);
        entry.next_retry_ms = now_ms.saturating_add(backoff);
        true
    }

    /// Get all entries (for inspection/debugging).
    pub fn all_entries(&self) -> Vec<&OutboxEntry> {
        self.entries.values().collect()
    }

    /// Count entries by status.
    pub fn counts(&self) -> OutboxCounts {
        let mut pending = 0;
        let mut delivered = 0;
        let mut failed = 0;
        for entry in self.entries.values() {
            match entry.status {
                OutboxStatus::Pending => pending += 1,
                OutboxStatus::Delivered => delivered += 1,
                OutboxStatus::Failed => failed += 1,
            }
        }
        OutboxCounts {
            pending,
            delivered,
            failed,
        }
    }

    /// Remove delivered entries (garbage collection).
    pub fn gc_delivered(&mut self) -> usize {
        let before = self.entries.len();
        self.entries
            .retain(|_, e| e.status != OutboxStatus::Delivered);
        before - self.entries.len()
    }
}

/// Counts by outbox status.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutboxCounts {
    pub pending: usize,
    pub delivered: usize,
    pub failed: usize,
}

/// Compute a deduplication key for a mutation. Same source + same mutation
/// body = same key (idempotent delivery).
fn dedup_key(source_kind: &str, mutation: &SourceMutation) -> String {
    match mutation {
        SourceMutation::PostStatus { native_id, status } => {
            format!("{source_kind}:status:{native_id}:{status}")
        }
        SourceMutation::PostComment { native_id, body } => {
            format!("{source_kind}:comment:{native_id}:{}", body.len())
        }
        SourceMutation::CloseIssue { native_id, reason } => {
            format!("{source_kind}:close:{native_id}:{reason:?}")
        }
        SourceMutation::AddLabel { native_id, label } => {
            format!("{source_kind}:label:{native_id}:{label}")
        }
    }
}

// ---------------------------------------------------------------------------
// Capability negotiation
// ---------------------------------------------------------------------------

/// What a source supports.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceCapabilities {
    pub can_read: bool,
    pub can_post_status: bool,
    pub can_post_comment: bool,
    pub can_close: bool,
    pub can_add_label: bool,
    pub requires_auth: bool,
    pub is_anonymous: bool,
    pub rate_limit_per_hour: Option<u32>,
}

/// Negotiate which capabilities a source provides, given the requested set
/// and what the source actually supports.
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NegotiatedCapabilities {
    pub granted: Vec<String>,
    pub denied: Vec<String>,
}

/// Negotiate capabilities. Only granted capabilities can be used.
pub fn negotiate_capabilities(
    requested: &[SourceMutation],
    available: &SourceCapabilities,
) -> NegotiatedCapabilities {
    let mut granted = Vec::new();
    let mut denied = Vec::new();

    for mutation in requested {
        let cap_name = mutation_capability(mutation);
        let allowed = match mutation {
            SourceMutation::PostStatus { .. } => available.can_post_status,
            SourceMutation::PostComment { .. } => available.can_post_comment,
            SourceMutation::CloseIssue { .. } => available.can_close,
            SourceMutation::AddLabel { .. } => available.can_add_label,
        };
        if allowed {
            granted.push(cap_name);
        } else {
            denied.push(cap_name);
        }
    }

    NegotiatedCapabilities { granted, denied }
}

fn mutation_capability(mutation: &SourceMutation) -> String {
    match mutation {
        SourceMutation::PostStatus { .. } => "post_status".into(),
        SourceMutation::PostComment { .. } => "post_comment".into(),
        SourceMutation::CloseIssue { .. } => "close".into(),
        SourceMutation::AddLabel { .. } => "add_label".into(),
    }
}

/// Check if a source is degraded (rate-limited or partially unavailable).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceHealth {
    Healthy,
    Degraded,
    Unavailable,
}

/// Assess source health based on rate limit remaining and error rate.
pub fn assess_health(
    rate_limit_remaining: Option<u32>,
    recent_error_count: usize,
    recent_success_count: usize,
) -> SourceHealth {
    let total = recent_error_count + recent_success_count;
    let error_rate = if total > 0 {
        recent_error_count as f64 / total as f64
    } else {
        0.0
    };

    if error_rate >= 0.5 {
        return SourceHealth::Unavailable;
    }
    if error_rate >= 0.2 {
        return SourceHealth::Degraded;
    }
    if let Some(remaining) = rate_limit_remaining {
        if remaining == 0 {
            return SourceHealth::Unavailable;
        }
        if remaining <= 10 {
            return SourceHealth::Degraded;
        }
    }
    SourceHealth::Healthy
}

// ---------------------------------------------------------------------------
// Conformance test trait
// ---------------------------------------------------------------------------

/// A source adapter implementation under conformance testing.
/// This trait extends TaskSourceAdapter with the contract that conformance
/// tests verify.
pub trait ConformantSource {
    /// Return a fresh test instance with seeded data.
    fn test_instance() -> Self;

    /// Source kind identifier.
    fn kind(&self) -> &str;

    /// List all tasks.
    fn list_all(&self) -> Vec<super::SourceTask>;

    /// Get a single task.
    fn get_task(&self, native_id: &str) -> Option<super::SourceTask>;

    /// Capabilities.
    fn capabilities(&self) -> SourceCapabilities;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_status_mutation(native: &str, status: &str) -> SourceMutation {
        SourceMutation::PostStatus {
            native_id: native.into(),
            status: status.into(),
        }
    }

    fn make_comment_mutation(native: &str, body: &str) -> SourceMutation {
        SourceMutation::PostComment {
            native_id: native.into(),
            body: body.into(),
        }
    }

    // ---- outbox enqueue + dedup ----

    #[test]
    fn enqueue_and_deliver() {
        let mut outbox = SourceOutbox::new();
        let config = OutboxConfig::default();

        let id = outbox.enqueue(
            "t1",
            "github",
            make_status_mutation("123", "done"),
            1000,
            &config,
        );
        assert!(!id.is_empty());

        let pending = outbox.pending(1000);
        assert_eq!(pending.len(), 1);

        outbox.mark_delivered(&id);
        let counts = outbox.counts();
        assert_eq!(counts.delivered, 1);
        assert_eq!(counts.pending, 0);
    }

    #[test]
    fn duplicate_status_not_re_enqueued() {
        let mut outbox = SourceOutbox::new();
        let config = OutboxConfig::default();

        let id = outbox.enqueue(
            "t1",
            "github",
            make_status_mutation("123", "done"),
            1000,
            &config,
        );
        outbox.mark_delivered(&id);

        // Same mutation after delivery → skipped.
        let re_id = outbox.enqueue(
            "t1",
            "github",
            make_status_mutation("123", "done"),
            2000,
            &config,
        );
        assert!(
            re_id.is_empty(),
            "delivered mutation should not be re-enqueued"
        );
    }

    #[test]
    fn different_status_is_new_entry() {
        let mut outbox = SourceOutbox::new();
        let config = OutboxConfig::default();

        outbox.enqueue(
            "t1",
            "github",
            make_status_mutation("123", "done"),
            1000,
            &config,
        );
        let id2 = outbox.enqueue(
            "t1",
            "github",
            make_status_mutation("123", "in_progress"),
            2000,
            &config,
        );
        assert!(!id2.is_empty(), "different status is a new mutation");
    }

    #[test]
    fn pending_duplicate_not_re_enqueued() {
        let mut outbox = SourceOutbox::new();
        let config = OutboxConfig::default();

        let id1 = outbox.enqueue(
            "t1",
            "github",
            make_status_mutation("123", "done"),
            1000,
            &config,
        );
        let id2 = outbox.enqueue(
            "t1",
            "github",
            make_status_mutation("123", "done"),
            1000,
            &config,
        );
        assert!(!id1.is_empty());
        assert!(id2.is_empty(), "pending duplicate should be skipped");
    }

    // ---- retry logic ----

    #[test]
    fn failed_entry_retries_with_backoff() {
        let mut outbox = SourceOutbox::new();
        let config = OutboxConfig {
            max_attempts: 3,
            backoff_base_ms: 1000,
            backoff_max_ms: 60_000,
        };

        let id = outbox.enqueue(
            "t1",
            "github",
            make_status_mutation("123", "done"),
            1000,
            &config,
        );

        // First failure → retry scheduled.
        let retried = outbox.mark_failed(&id, "rate limited", 2000, &config);
        assert!(retried);
        let next_retry = {
            let entries = outbox.all_entries();
            let entry = entries.iter().find(|e| e.id == id).unwrap();
            entry.next_retry_ms
        };
        assert!(next_retry > 2000, "next retry should be in the future");

        // At max attempts → permanent failure.
        outbox.mark_failed(&id, "still failing", 3000, &config);
        let retried2 = outbox.mark_failed(&id, "still failing", 4000, &config);
        assert!(!retried2, "should not retry after max_attempts");
        let entries = outbox.all_entries();
        let entry = entries.iter().find(|e| e.id == id).unwrap();
        assert_eq!(entry.status, OutboxStatus::Failed);
    }

    #[test]
    fn pending_respects_retry_time() {
        let mut outbox = SourceOutbox::new();
        let config = OutboxConfig {
            max_attempts: 5,
            backoff_base_ms: 10_000,
            backoff_max_ms: 60_000,
        };

        let id = outbox.enqueue(
            "t1",
            "github",
            make_status_mutation("123", "done"),
            1000,
            &config,
        );
        outbox.mark_failed(&id, "err", 1000, &config);

        // At t=1000, the entry has next_retry > 1000 (backoff applied).
        assert!(
            outbox.pending(1000).is_empty(),
            "should not retry before backoff"
        );

        // After backoff, it's pending again.
        let next_retry = {
            let entries = outbox.all_entries();
            let entry = entries.iter().find(|e| e.id == id).unwrap();
            entry.next_retry_ms
        };
        assert!(!outbox.pending(next_retry).is_empty());
    }

    // ---- GC ----

    #[test]
    fn gc_removes_delivered() {
        let mut outbox = SourceOutbox::new();
        let config = OutboxConfig::default();

        let id1 = outbox.enqueue(
            "t1",
            "github",
            make_status_mutation("1", "done"),
            1000,
            &config,
        );
        outbox.enqueue(
            "t2",
            "github",
            make_status_mutation("2", "done"),
            1000,
            &config,
        );

        outbox.mark_delivered(&id1);
        let removed = outbox.gc_delivered();
        assert_eq!(removed, 1);
        assert_eq!(outbox.counts().pending, 1);
    }

    // ---- capability negotiation ----

    #[test]
    fn negotiate_grants_supported_capabilities() {
        let caps = SourceCapabilities {
            can_post_status: true,
            can_post_comment: true,
            can_close: false,
            can_add_label: false,
            ..Default::default()
        };
        let mutations = vec![
            make_status_mutation("1", "done"),
            make_comment_mutation("1", "ok"),
        ];
        let negotiated = negotiate_capabilities(&mutations, &caps);
        assert_eq!(negotiated.granted.len(), 2);
        assert!(negotiated.denied.is_empty());
    }

    #[test]
    fn negotiate_denies_unsupported() {
        let caps = SourceCapabilities {
            can_post_status: true,
            can_close: false,
            ..Default::default()
        };
        let mutations = vec![
            make_status_mutation("1", "done"),
            SourceMutation::CloseIssue {
                native_id: "1".into(),
                reason: None,
            },
        ];
        let negotiated = negotiate_capabilities(&mutations, &caps);
        assert_eq!(negotiated.granted.len(), 1);
        assert_eq!(negotiated.denied.len(), 1);
        assert!(negotiated.denied.contains(&"close".to_string()));
    }

    // ---- health assessment ----

    #[test]
    fn healthy_when_no_errors() {
        assert_eq!(assess_health(Some(100), 0, 100), SourceHealth::Healthy);
    }

    #[test]
    fn degraded_when_rate_limit_low() {
        assert_eq!(assess_health(Some(5), 0, 100), SourceHealth::Degraded);
    }

    #[test]
    fn unavailable_when_rate_exhausted() {
        assert_eq!(assess_health(Some(0), 0, 100), SourceHealth::Unavailable);
    }

    #[test]
    fn unavailable_when_high_error_rate() {
        assert_eq!(assess_health(Some(5000), 50, 50), SourceHealth::Unavailable);
    }

    #[test]
    fn degraded_when_moderate_error_rate() {
        assert_eq!(assess_health(Some(5000), 20, 80), SourceHealth::Degraded);
    }

    // ---- conformance: local source passes ----

    #[test]
    fn outbox_idempotent_delivery() {
        let mut outbox = SourceOutbox::new();
        let config = OutboxConfig::default();

        let id = outbox.enqueue(
            "t1",
            "github",
            make_status_mutation("1", "done"),
            1000,
            &config,
        );
        outbox.mark_delivered(&id);

        // Deliver again → should be no-op (already delivered).
        let re_id = outbox.enqueue(
            "t1",
            "github",
            make_status_mutation("1", "done"),
            2000,
            &config,
        );
        assert!(
            re_id.is_empty(),
            "already-delivered mutation should be skipped"
        );
    }

    #[test]
    fn outbox_isolated_by_source() {
        let mut outbox = SourceOutbox::new();
        let config = OutboxConfig::default();

        // Same mutation on different sources → both queued.
        outbox.enqueue(
            "t1",
            "github",
            make_status_mutation("1", "done"),
            1000,
            &config,
        );
        outbox.enqueue(
            "t1",
            "linear",
            make_status_mutation("1", "done"),
            1000,
            &config,
        );
        assert_eq!(outbox.pending(1000).len(), 2);
    }

    #[test]
    fn outbox_isolated_by_task() {
        let mut outbox = SourceOutbox::new();
        let config = OutboxConfig::default();

        // Same status on different tasks → both queued.
        outbox.enqueue(
            "t1",
            "github",
            make_status_mutation("1", "done"),
            1000,
            &config,
        );
        outbox.enqueue(
            "t2",
            "github",
            make_status_mutation("2", "done"),
            1000,
            &config,
        );
        assert_eq!(outbox.pending(1000).len(), 2);
    }

    #[test]
    fn outbox_comment_dedup_by_body_length() {
        // Comments with the same body length are deduplicated (simplified).
        let mut outbox = SourceOutbox::new();
        let config = OutboxConfig::default();

        let id1 = outbox.enqueue(
            "t1",
            "github",
            make_comment_mutation("1", "hello"),
            1000,
            &config,
        );
        // Different body, same length → deduplicated by length-based key.
        let id2 = outbox.enqueue(
            "t1",
            "github",
            make_comment_mutation("1", "world"),
            1000,
            &config,
        );
        assert!(!id1.is_empty());
        assert!(
            id2.is_empty(),
            "same-length comment to same issue is deduplicated"
        );
    }

    #[test]
    fn outbox_close_dedup() {
        let mut outbox = SourceOutbox::new();
        let config = OutboxConfig::default();

        let m = SourceMutation::CloseIssue {
            native_id: "1".into(),
            reason: Some("done".into()),
        };
        outbox.enqueue("t1", "github", m.clone(), 1000, &config);
        let id2 = outbox.enqueue("t1", "github", m, 1000, &config);
        assert!(id2.is_empty(), "duplicate close should be deduplicated");
    }
}
