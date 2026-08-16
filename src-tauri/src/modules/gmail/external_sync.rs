//! Gmail provider for the account-scoped external-object sync engine
//! (package 074, PR 2). Maps Gmail threads and messages onto the
//! engine's `ProviderObject`s: the provider-assigned id is the
//! immutable provider identity, and the canonical content is the mapped
//! payload — equal mailbox entries must hash equal, so label order is
//! sorted (presentation, not identity) and volatile ordering fields are
//! excluded from content entirely.
//!
//! Threads report no change clock; they carry no watermark and never
//! advance the account's sync window — messages do, through Gmail's
//! `internalDate`. A message without a thread binding is refused, not
//! silently mapped: it cannot be placed in a conversation.
//!
//! Transport is a seam ([`GmailClient`]): this module maps and bounds,
//! the real client (and its account-scoped credentials, CP-08-76)
//! lands with the wiring PR.
//!
//! Nothing outside this module consumes the mappings yet — the account
//! sync command arrives with that wiring, and until then the compiler
//! would see only unreferenced contracts. The allow is that gap, not
//! neglect; it leaves with the wiring PR.
#![allow(dead_code)]

use altai_control_plane::{ExternalObjectProvider, ProviderObject};
use serde::Deserialize;

/// Title bound: a snippet is presentational, a title is a label.
const TITLE_MAX_CHARS: usize = 80;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ThreadRaw {
    pub id: String,
    #[serde(default)]
    pub snippet: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct MessageRaw {
    pub id: String,
    #[serde(rename = "threadId")]
    pub thread_id: String,
    #[serde(default)]
    pub snippet: Option<String>,
    #[serde(rename = "labelIds", default)]
    pub label_ids: Vec<String>,
    /// Gmail's `internalDate`: epoch milliseconds as a string.
    #[serde(rename = "internalDate", default)]
    pub internal_date: Option<String>,
}

/// Canonical mapped content. Built through `serde_json::json!` so key
/// order is fixed by code, which makes the string — and therefore the
/// engine's content hash — stable.
pub fn thread_content(raw: &ThreadRaw) -> String {
    serde_json::json!({ "snippet": raw.snippet }).to_string()
}

pub fn message_content(raw: &MessageRaw) -> String {
    let mut labels = raw.label_ids.clone();
    labels.sort();
    serde_json::json!({
        "thread_id": raw.thread_id,
        "snippet": raw.snippet,
        "label_ids": labels,
    })
    .to_string()
}

/// Map one thread. Threads have no stable web link (Gmail's is per
/// logged-in account index) and no change clock; both stay absent.
pub fn map_thread(raw: &ThreadRaw) -> ProviderObject {
    ProviderObject {
        external_id: raw.id.clone(),
        object_kind: "thread".into(),
        url: None,
        title: title_from_snippet(raw.snippet.as_deref()),
        content: thread_content(raw),
        external_updated_at_unix_seconds: None,
    }
}

/// Map one message. `None` when the entry lacks a thread binding — a
/// different gap, not silently the same object.
pub fn map_message(raw: &MessageRaw) -> Option<ProviderObject> {
    if raw.thread_id.trim().is_empty() {
        return None;
    }
    Some(ProviderObject {
        external_id: raw.id.clone(),
        object_kind: "message".into(),
        url: Some(format!("https://mail.google.com/mail/u/0/#inbox/{}", raw.id)),
        title: title_from_snippet(raw.snippet.as_deref()),
        content: message_content(raw),
        external_updated_at_unix_seconds: unix_seconds_from_internal_date(
            raw.internal_date.as_deref(),
        ),
    })
}

fn title_from_snippet(snippet: Option<&str>) -> String {
    snippet
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(|value| value.chars().take(TITLE_MAX_CHARS).collect::<String>())
        .unwrap_or_else(|| "(no snippet)".into())
}

/// `internalDate` (epoch milliseconds, string) to the engine's unix
/// seconds. `None` when Gmail did not report one — absent, not zero.
fn unix_seconds_from_internal_date(value: Option<&str>) -> Option<u64> {
    value?
        .parse::<u64>()
        .ok()
        .map(|milliseconds| milliseconds / 1_000)
}

/// The transport seam. The real client fetches per account with
/// account-scoped credentials and translates the engine's watermark
/// into Gmail's query; fakes answer from fixtures.
pub trait GmailClient: Send + Sync {
    fn fetch_threads(&self, since: Option<u64>) -> Result<Vec<ThreadRaw>, String>;
    fn fetch_messages(&self, since: Option<u64>) -> Result<Vec<MessageRaw>, String>;
}

pub struct GmailProvider<C: GmailClient> {
    client: C,
}

impl<C: GmailClient> GmailProvider<C> {
    pub fn new(client: C) -> Self {
        Self { client }
    }
}

impl<C: GmailClient> ExternalObjectProvider for GmailProvider<C> {
    fn integration(&self) -> &str {
        "gmail"
    }

    fn fetch(&self, since: Option<u64>) -> Result<Vec<ProviderObject>, String> {
        let threads = self.client.fetch_threads(since)?;
        let messages = self.client.fetch_messages(since)?;
        let mut objects: Vec<ProviderObject> = threads.iter().map(map_thread).collect();
        objects.extend(messages.iter().filter_map(map_message));
        Ok(objects)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use altai_control_plane::content_hash;
    use std::sync::Mutex;

    fn thread(id: &str, snippet: &str) -> ThreadRaw {
        ThreadRaw {
            id: id.into(),
            snippet: Some(snippet.into()),
        }
    }

    fn message(id: &str, thread_id: &str, snippet: &str, labels: &[&str]) -> MessageRaw {
        MessageRaw {
            id: id.into(),
            thread_id: thread_id.into(),
            snippet: Some(snippet.into()),
            label_ids: labels.iter().map(|label| label.to_string()).collect(),
            internal_date: Some(1_755_300_000_000_u64.to_string()),
        }
    }

    #[test]
    fn equal_payloads_hash_equal() {
        let mapped_first = map_thread(&thread("thr_1", "Same snippet"));
        let mapped_second = map_thread(&thread("thr_1", "Same snippet"));
        assert_eq!(
            content_hash(&mapped_first.content),
            content_hash(&mapped_second.content),
            "an unchanged mailbox entry must never write"
        );
        assert_eq!(mapped_first.external_id, "thr_1");
        assert_eq!(mapped_first.object_kind, "thread");
        assert_eq!(mapped_first.title, "Same snippet");
        assert_eq!(mapped_first.external_updated_at_unix_seconds, None);
    }

    #[test]
    fn label_order_is_presentation_not_identity() {
        let ordered = map_message(&message("msg_1", "thr_1", "Hello", &["INBOX", "UNREAD"])).unwrap();
        let shuffled =
            map_message(&message("msg_1", "thr_1", "Hello", &["UNREAD", "INBOX"])).unwrap();
        assert_eq!(
            content_hash(&ordered.content),
            content_hash(&shuffled.content)
        );
    }

    #[test]
    fn a_message_without_a_thread_is_refused() {
        let mut orphan = message("msg_1", "thr_1", "Hello", &["INBOX"]);
        orphan.thread_id = "  ".into();
        assert_eq!(map_message(&orphan), None);
    }

    #[test]
    fn internal_date_maps_to_unix_seconds_or_nothing() {
        assert_eq!(unix_seconds_from_internal_date(Some("1755300000000")), Some(1_755_300_000));
        assert_eq!(unix_seconds_from_internal_date(Some("not-a-date")), None);
        assert_eq!(unix_seconds_from_internal_date(None), None);
    }

    #[test]
    fn a_long_snippet_is_bounded_for_presentation_only() {
        let long = "x".repeat(200);
        let mapped = map_message(&message("msg_1", "thr_1", &long, &[]));
        assert_eq!(mapped.unwrap().title.chars().count(), TITLE_MAX_CHARS);
    }

    struct FakeClient {
        threads: Vec<ThreadRaw>,
        messages: Vec<MessageRaw>,
        asked: std::sync::Arc<Mutex<Vec<Option<u64>>>>,
    }

    impl GmailClient for FakeClient {
        fn fetch_threads(&self, since: Option<u64>) -> Result<Vec<ThreadRaw>, String> {
            self.asked.lock().unwrap().push(since);
            Ok(self.threads.clone())
        }

        fn fetch_messages(&self, since: Option<u64>) -> Result<Vec<MessageRaw>, String> {
            self.asked.lock().unwrap().push(since);
            Ok(self.messages.clone())
        }
    }

    #[test]
    fn the_provider_maps_threads_and_messages_through_the_client() {
        let client = FakeClient {
            threads: vec![thread("thr_1", "A conversation")],
            messages: vec![message("msg_1", "thr_1", "Hello", &["INBOX"])],
            asked: std::sync::Arc::new(Mutex::new(Vec::new())),
        };
        let provider = GmailProvider::new(client);

        assert_eq!(provider.integration(), "gmail");
        let objects = provider.fetch(None).unwrap();
        assert_eq!(objects.len(), 2);
        assert_eq!(objects[0].object_kind, "thread");
        assert_eq!(objects[1].object_kind, "message");
        assert_eq!(objects[1].external_updated_at_unix_seconds, Some(1_755_300_000));
    }

    #[test]
    fn the_client_sees_the_watermark_untouched() {
        let asked = std::sync::Arc::new(Mutex::new(Vec::new()));
        let client = FakeClient {
            threads: Vec::new(),
            messages: Vec::new(),
            asked: asked.clone(),
        };
        GmailProvider::new(client)
            .fetch(Some(1_755_300_000))
            .unwrap();
        // The engine's watermark is the client's to translate; the
        // provider passes it through without reinterpreting it.
        assert_eq!(
            *asked.lock().unwrap(),
            vec![Some(1_755_300_000), Some(1_755_300_000)]
        );
    }
}
