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
//! and the real client ([`GmailHttpClient`]) fetches — through the
//! SSRF-safe module HTTP client, with the account's scoped credential
//! (CP-08-76) supplied by the commands that own its lifetime.
//!
//! Gmail's list endpoints return ids only, so the client lists bounded
//! pages and then fetches each entry's metadata — everything listed is
//! enriched; nothing listed is silently dropped.

use crate::modules::net::{safe_http_request, HttpResponse};
use altai_control_plane::{ExternalObjectProvider, ProviderObject};
use serde::Deserialize;
use std::collections::HashMap;

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
        url: Some(format!(
            "https://mail.google.com/mail/u/0/#inbox/{}",
            raw.id
        )),
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

//
// The real client (package 074, PR 3). Gmail's list endpoints answer
// ids only — snippet, labels and `internalDate` arrive per entry — so a
// fetch is: list bounded pages, then fetch the metadata of everything
// listed. Entries Gmail reports gone between list and get (404/410)
// are skips, not failures: a mailbox that changed mid-run is normal.
//

const GMAIL_API_BASE: &str = "https://gmail.googleapis.com";
const GMAIL_USER_AGENT: &str = "altai-app";
/// One page of a list endpoint at the widest window the engine asks for.
const LIST_PAGE_SIZE: usize = 100;
/// Bound on pages per run per kind: a desktop sync must terminate even
/// against a very large mailbox. 50 pages ≈ 5 000 entries per run.
const MAX_LIST_PAGES: usize = 50;

/// Query string for a list endpoint: the window (`after:` accepts unix
/// seconds), the page size, and the page cursor when one exists.
fn list_url(kind: &str, since: Option<u64>, page_token: Option<&str>) -> Result<String, String> {
    let mut url = reqwest::Url::parse(&format!("{GMAIL_API_BASE}/gmail/v1/users/me/{kind}"))
        .map_err(|error| error.to_string())?;
    {
        let mut pairs = url.query_pairs_mut();
        pairs.append_pair("maxResults", &LIST_PAGE_SIZE.to_string());
        if let Some(since) = since {
            pairs.append_pair("q", &format!("after:{since}"));
        }
        if let Some(page_token) = page_token {
            pairs.append_pair("pageToken", page_token);
        }
    }
    Ok(url.to_string())
}

/// Metadata URL for one entry; `format=metadata` answers exactly the
/// mapped fields — id, snippet, and (for messages) thread binding,
/// labels and `internalDate`.
fn metadata_url(kind: &str, id: &str) -> Result<String, String> {
    let mut url = reqwest::Url::parse(&format!("{GMAIL_API_BASE}/gmail/v1/users/me/{kind}"))
        .map_err(|error| error.to_string())?;
    url.path_segments_mut()
        .map_err(|()| "Gmail metadata URL cannot be a base".to_string())?
        .push(id);
    url.query_pairs_mut().append_pair("format", "metadata");
    Ok(url.to_string())
}

#[derive(Debug, Clone, Deserialize)]
struct ThreadIdsPage {
    #[serde(default)]
    threads: Vec<ThreadIdEntry>,
    #[serde(rename = "nextPageToken", default)]
    next_page_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ThreadIdEntry {
    id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct MessageIdsPage {
    #[serde(default)]
    messages: Vec<MessageIdEntry>,
    #[serde(rename = "nextPageToken", default)]
    next_page_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct MessageIdEntry {
    id: String,
}

/// The engine is synchronous, so the client owns a private Tokio
/// runtime and blocks on the module HTTP client inside it — callers run
/// the whole sync on a blocking thread (`spawn_blocking`), never on a
/// runtime worker.
pub struct GmailHttpClient {
    access_token: String,
    runtime: tokio::runtime::Runtime,
}

impl GmailHttpClient {
    pub fn new(access_token: String) -> Result<Self, String> {
        let runtime = tokio::runtime::Runtime::new().map_err(|error| error.to_string())?;
        Ok(Self {
            access_token,
            runtime,
        })
    }

    fn request(&self, url: &str) -> Result<HttpResponse, String> {
        let mut headers = HashMap::new();
        headers.insert(
            "authorization".to_string(),
            format!("Bearer {}", self.access_token),
        );
        headers.insert("accept".to_string(), "application/json".to_string());
        headers.insert("user-agent".to_string(), GMAIL_USER_AGENT.to_string());
        let request_url = url.to_string();
        self.runtime.block_on(async {
            safe_http_request(&request_url, "GET", Some(headers), None, false).await
        })
    }

    fn list_page<T: serde::de::DeserializeOwned>(
        &self,
        kind: &str,
        since: Option<u64>,
        page_token: Option<&str>,
    ) -> Result<T, String> {
        let url = list_url(kind, since, page_token)?;
        let response = self.request(&url)?;
        if !(200..300).contains(&response.status) {
            return Err(format!(
                "Gmail {kind} list failed with status {}",
                response.status
            ));
        }
        serde_json::from_slice(&response.body)
            .map_err(|error| format!("Gmail {kind} list response did not decode: {error}"))
    }

    /// One entry's metadata. `Ok(None)` when Gmail reports the entry
    /// gone (404/410): deleted between list and get is a skip.
    fn metadata(&self, kind: &str, id: &str) -> Result<Option<serde_json::Value>, String> {
        let url = metadata_url(kind, id)?;
        let response = self.request(&url)?;
        match response.status {
            200 => {}
            404 | 410 => return Ok(None),
            status => return Err(format!("Gmail {kind} fetch failed with status {status}")),
        }
        serde_json::from_slice(&response.body)
            .map(Some)
            .map_err(|error| format!("Gmail {kind} metadata response did not decode: {error}"))
    }

    /// List thread ids page by page. The bound is on pages listed per
    /// run; everything listed is then enriched, so the bound never
    /// silently drops part of what a run showed.
    fn list_thread_ids(&self, since: Option<u64>) -> Result<Vec<ThreadIdEntry>, String> {
        let mut ids = Vec::new();
        let mut page_token: Option<String> = None;
        for _ in 0..MAX_LIST_PAGES {
            let page: ThreadIdsPage = self.list_page("threads", since, page_token.as_deref())?;
            ids.extend(page.threads);
            page_token = page.next_page_token;
            if page_token.is_none() {
                break;
            }
        }
        Ok(ids)
    }

    fn list_message_ids(&self, since: Option<u64>) -> Result<Vec<MessageIdEntry>, String> {
        let mut ids = Vec::new();
        let mut page_token: Option<String> = None;
        for _ in 0..MAX_LIST_PAGES {
            let page: MessageIdsPage = self.list_page("messages", since, page_token.as_deref())?;
            ids.extend(page.messages);
            page_token = page.next_page_token;
            if page_token.is_none() {
                break;
            }
        }
        Ok(ids)
    }
}

impl GmailClient for GmailHttpClient {
    fn fetch_threads(&self, since: Option<u64>) -> Result<Vec<ThreadRaw>, String> {
        let mut threads = Vec::new();
        for entry in self.list_thread_ids(since)? {
            if let Some(metadata) = self.metadata("threads", &entry.id)? {
                threads.push(
                    serde_json::from_value::<ThreadRaw>(metadata).map_err(|error| {
                        format!("Gmail thread metadata did not decode: {error}")
                    })?,
                );
            }
        }
        Ok(threads)
    }

    fn fetch_messages(&self, since: Option<u64>) -> Result<Vec<MessageRaw>, String> {
        let mut messages = Vec::new();
        for entry in self.list_message_ids(since)? {
            if let Some(metadata) = self.metadata("messages", &entry.id)? {
                messages.push(
                    serde_json::from_value::<MessageRaw>(metadata).map_err(|error| {
                        format!("Gmail message metadata did not decode: {error}")
                    })?,
                );
            }
        }
        Ok(messages)
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
        let ordered =
            map_message(&message("msg_1", "thr_1", "Hello", &["INBOX", "UNREAD"])).unwrap();
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
        assert_eq!(
            unix_seconds_from_internal_date(Some("1755300000000")),
            Some(1_755_300_000)
        );
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
        assert_eq!(
            objects[1].external_updated_at_unix_seconds,
            Some(1_755_300_000)
        );
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

    fn query_pairs(url: &str) -> Vec<(String, String)> {
        reqwest::Url::parse(url)
            .unwrap()
            .query_pairs()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect()
    }

    #[test]
    fn the_list_url_carries_the_window_size_and_cursor() {
        let url = list_url("messages", Some(1_755_300_000), None).unwrap();
        assert!(url.starts_with("https://gmail.googleapis.com/gmail/v1/users/me/messages?"));
        let pairs = query_pairs(&url);
        assert!(pairs.contains(&("maxResults".to_string(), "100".to_string())));
        assert!(pairs.contains(&("q".to_string(), "after:1755300000".to_string())));
        assert!(!pairs.iter().any(|(key, _)| key == "pageToken"));

        // No watermark yet (first run) asks for no window; the page
        // cursor survives the round trip whatever it contains.
        let url = list_url("threads", None, Some("cursor 1&2")).unwrap();
        let pairs = query_pairs(&url);
        assert!(!pairs.iter().any(|(key, _)| key == "q"));
        assert!(pairs.contains(&("pageToken".to_string(), "cursor 1&2".to_string())));
    }

    #[test]
    fn the_metadata_url_addresses_one_entry_as_metadata() {
        let url = metadata_url("messages", "18c9f4a1b2c3d4e5").unwrap();
        assert_eq!(
            url,
            "https://gmail.googleapis.com/gmail/v1/users/me/messages/18c9f4a1b2c3d4e5?format=metadata"
        );
    }
}
