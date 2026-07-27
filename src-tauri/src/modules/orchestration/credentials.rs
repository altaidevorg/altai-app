//! Credential broker and provider-native tools (plan §I2).
//!
//! Keeps credentials in a managed store (OS keychain abstraction), exposes
//! narrow host-side tools bound to the current task/repository context,
//! redacts secrets from events, and enforces immediate revocation. Runner
//! child processes never receive raw tracker credentials.

use std::collections::{HashMap, HashSet};
use std::sync::RwLock;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

// ---------------------------------------------------------------------------
// Credential store
// ---------------------------------------------------------------------------

/// A key identifying a stored credential.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialKey {
    /// The source/integration this credential belongs to (e.g., "github", "linear").
    pub source: String,
    /// The credential name (e.g., "token", "api_key").
    pub name: String,
}

impl CredentialKey {
    pub fn new(source: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            name: name.into(),
        }
    }
}

/// A stored credential with metadata.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredCredential {
    pub key: CredentialKey,
    /// The actual secret value — never exposed to runners.
    #[serde(skip_serializing)]
    pub value: String,
    pub created_at_ms: u64,
    pub revoked: bool,
    pub revoked_at_ms: Option<u64>,
}

/// Trait for credential storage backends (OS keychain, env, in-memory).
pub trait CredentialStore: Send + Sync {
    fn store(&self, key: &CredentialKey, value: &str) -> Result<(), CredentialError>;
    fn retrieve(&self, key: &CredentialKey) -> Result<Option<String>, CredentialError>;
    fn revoke(&self, key: &CredentialKey) -> Result<bool, CredentialError>;
    fn is_revoked(&self, key: &CredentialKey) -> Result<bool, CredentialError>;
    fn keys(&self) -> Result<Vec<CredentialKey>, CredentialError>;
}

/// Error for credential operations.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum CredentialError {
    NotFound { source: String, name: String },
    AlreadyRevoked { source: String, name: String },
    StoreError { detail: String },
}

impl std::fmt::Display for CredentialError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound { source, name } => {
                write!(f, "Credential not found: {source}/{name}")
            }
            Self::AlreadyRevoked { source, name } => {
                write!(f, "Credential already revoked: {source}/{name}")
            }
            Self::StoreError { detail } => write!(f, "Credential store error: {detail}"),
        }
    }
}

impl std::error::Error for CredentialError {}

/// In-memory credential store for testing and development.
pub struct InMemoryCredentialStore {
    inner: RwLock<HashMap<CredentialKey, StoredCredential>>,
}

impl InMemoryCredentialStore {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryCredentialStore {
    fn default() -> Self {
        Self::new()
    }
}

impl CredentialStore for InMemoryCredentialStore {
    fn store(&self, key: &CredentialKey, value: &str) -> Result<(), CredentialError> {
        let mut inner = self.inner.write().unwrap();
        inner.insert(
            key.clone(),
            StoredCredential {
                key: key.clone(),
                value: value.to_string(),
                created_at_ms: now_ms(),
                revoked: false,
                revoked_at_ms: None,
            },
        );
        Ok(())
    }

    fn retrieve(&self, key: &CredentialKey) -> Result<Option<String>, CredentialError> {
        let inner = self.inner.read().unwrap();
        match inner.get(key) {
            Some(cred) if !cred.revoked => Ok(Some(cred.value.clone())),
            Some(_) => Ok(None), // revoked → treated as not found
            None => Ok(None),
        }
    }

    fn revoke(&self, key: &CredentialKey) -> Result<bool, CredentialError> {
        let mut inner = self.inner.write().unwrap();
        match inner.get_mut(key) {
            Some(cred) if !cred.revoked => {
                cred.revoked = true;
                cred.revoked_at_ms = Some(now_ms());
                Ok(true)
            }
            Some(_) => Err(CredentialError::AlreadyRevoked {
                source: key.source.clone(),
                name: key.name.clone(),
            }),
            None => Err(CredentialError::NotFound {
                source: key.source.clone(),
                name: key.name.clone(),
            }),
        }
    }

    fn is_revoked(&self, key: &CredentialKey) -> Result<bool, CredentialError> {
        let inner = self.inner.read().unwrap();
        Ok(inner.get(key).is_some_and(|c| c.revoked))
    }

    fn keys(&self) -> Result<Vec<CredentialKey>, CredentialError> {
        let inner = self.inner.read().unwrap();
        Ok(inner.keys().cloned().collect())
    }
}

// ---------------------------------------------------------------------------
// Tool binding (task/repository context isolation)
// ---------------------------------------------------------------------------

/// What scope a tool is allowed to operate in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolScope {
    /// Read files in the bound repository.
    Read,
    /// Write files in the bound repository.
    Write,
    /// Execute commands in the bound worktree.
    Execute,
    /// Make network requests on behalf of the bound source.
    Network,
    /// Apply changes (git apply/commit) in the bound repository.
    Apply,
    /// Push/publish to the bound source.
    Publish,
}

/// A tool bound to a specific task and repository context.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolBinding {
    pub tool_name: String,
    pub task_id: String,
    pub repository: String,
    pub source: String,
    pub allowed_scopes: Vec<ToolScope>,
}

/// Validate that a tool invocation stays within its bound context.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ContextViolation {
    WrongTask {
        bound_task: String,
        requested_task: String,
    },
    WrongRepository {
        bound_repo: String,
        requested_repo: String,
    },
    MissingScope {
        required: String,
        allowed: Vec<String>,
    },
}

/// Check that a tool invocation is within its bound context.
pub fn validate_tool_context(
    binding: &ToolBinding,
    requested_task: &str,
    requested_repo: &str,
    required_scope: ToolScope,
) -> Result<(), ContextViolation> {
    if binding.task_id != requested_task {
        return Err(ContextViolation::WrongTask {
            bound_task: binding.task_id.clone(),
            requested_task: requested_task.to_string(),
        });
    }
    if binding.repository != requested_repo {
        return Err(ContextViolation::WrongRepository {
            bound_repo: binding.repository.clone(),
            requested_repo: requested_repo.to_string(),
        });
    }
    if !binding.allowed_scopes.contains(&required_scope) {
        return Err(ContextViolation::MissingScope {
            required: format!("{required_scope:?}"),
            allowed: binding
                .allowed_scopes
                .iter()
                .map(|s| format!("{s:?}"))
                .collect(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Credential injection (masked values for runner environments)
// ---------------------------------------------------------------------------

/// A masked credential reference — runners see this, not the raw value.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MaskedCredential {
    /// Reference ID the host can resolve.
    pub ref_id: String,
    /// Human-readable label for UI.
    pub label: String,
    /// The source this credential belongs to.
    pub source: String,
}

/// Produce masked credential references for a runner environment.
/// The actual values are resolved host-side only.
pub fn mask_credentials(
    store: &dyn CredentialStore,
    keys: &[CredentialKey],
) -> Vec<MaskedCredential> {
    keys.iter()
        .filter_map(|key| {
            // Only mask non-revoked credentials.
            match store.is_revoked(key) {
                Ok(false) => Some(MaskedCredential {
                    ref_id: format!("cred:{}:{}", key.source, key.name),
                    label: format!("{} {}", key.source, key.name),
                    source: key.source.clone(),
                }),
                _ => None,
            }
        })
        .collect()
}

/// Resolve a masked credential reference to its actual value (host-side only).
pub fn resolve_credential(
    store: &dyn CredentialStore,
    ref_id: &str,
) -> Result<String, CredentialError> {
    // Parse "cred:source:name" format.
    let parts: Vec<&str> = ref_id
        .strip_prefix("cred:")
        .unwrap_or(ref_id)
        .splitn(2, ':')
        .collect();
    if parts.len() != 2 {
        return Err(CredentialError::StoreError {
            detail: format!("Invalid credential ref: {ref_id}"),
        });
    }
    let key = CredentialKey::new(parts[0], parts[1]);
    store.retrieve(&key)?.ok_or(CredentialError::NotFound {
        source: parts[0].to_string(),
        name: parts[1].to_string(),
    })
}

// ---------------------------------------------------------------------------
// Credential redaction
// ---------------------------------------------------------------------------

/// A pattern for redacting secrets from event payloads.
#[derive(Clone, Debug)]
pub struct RedactionPattern {
    pub key_contains: String,
}

/// Default redaction patterns for common credential field names.
pub fn default_redaction_patterns() -> Vec<RedactionPattern> {
    [
        "token",
        "secret",
        "password",
        "api_key",
        "apikey",
        "access_key",
        "private_key",
        "authorization",
        "credential",
        "bearer",
        "client_secret",
        "refresh_token",
        "session_token",
    ]
    .into_iter()
    .map(|k| RedactionPattern {
        key_contains: k.to_string(),
    })
    .collect()
}

/// Redact sensitive values from a JSON payload recursively.
pub fn redact_credentials(value: &mut Value, patterns: &[RedactionPattern]) {
    match value {
        Value::Object(map) => redact_map(map, patterns),
        Value::Array(arr) => {
            for v in arr.iter_mut() {
                redact_credentials(v, patterns);
            }
        }
        Value::String(s) => {
            // Also check for known credential prefixes in string values.
            for prefix in &["sk-", "ghp_", "gho_", "ghs_", "AKIA", "xoxb-"] {
                if s.starts_with(prefix) && s.len() > prefix.len() + 4 {
                    *s = "[REDACTED]".to_string();
                    return;
                }
            }
        }
        _ => {}
    }
}

fn redact_map(map: &mut Map<String, Value>, patterns: &[RedactionPattern]) {
    for (key, val) in map.iter_mut() {
        let key_lower = key.to_lowercase();
        let matched = patterns.iter().any(|p| key_lower.contains(&p.key_contains));
        if matched {
            match val {
                Value::String(s) if !s.is_empty() => {
                    *val = Value::String("[REDACTED]".to_string());
                }
                Value::Object(_) | Value::Array(_) => {
                    redact_credentials(val, patterns);
                }
                _ => {}
            }
        } else {
            redact_credentials(val, patterns);
        }
    }
}

// ---------------------------------------------------------------------------
// Revocation gate
// ---------------------------------------------------------------------------

/// Check if a credential is still valid (not revoked). Enforces immediate
/// revocation — revoked credentials are treated as non-existent.
pub fn check_revocation(store: &dyn CredentialStore, key: &CredentialKey) -> RevocationStatus {
    match store.is_revoked(key) {
        Ok(true) => RevocationStatus::Revoked,
        Ok(false) => RevocationStatus::Active,
        Err(_) => RevocationStatus::Unknown,
    }
}

/// Result of a revocation check.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RevocationStatus {
    Active,
    Revoked,
    Unknown,
}

/// Gate a tool invocation against credential revocation. Returns false if
/// any required credential is revoked.
pub fn gate_on_credentials(store: &dyn CredentialStore, required: &[CredentialKey]) -> bool {
    required
        .iter()
        .all(|key| matches!(check_revocation(store, key), RevocationStatus::Active))
}

// ---------------------------------------------------------------------------
// Audit log
// ---------------------------------------------------------------------------

/// An audit entry for credential access.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialAuditEntry {
    pub timestamp_ms: u64,
    pub action: CredentialAction,
    pub key: CredentialKey,
    pub task_id: Option<String>,
    pub tool_name: Option<String>,
}

/// What happened to a credential.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialAction {
    Stored,
    Retrieved,
    Revoked,
    AccessDenied,
}

/// Simple in-memory audit log.
#[derive(Clone, Debug, Default)]
pub struct CredentialAuditLog {
    entries: Vec<CredentialAuditEntry>,
}

impl CredentialAuditLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, entry: CredentialAuditEntry) {
        self.entries.push(entry);
    }

    pub fn for_source(&self, source: &str) -> Vec<&CredentialAuditEntry> {
        self.entries
            .iter()
            .filter(|e| e.key.source == source)
            .collect()
    }

    pub fn for_key(&self, key: &CredentialKey) -> Vec<&CredentialAuditEntry> {
        self.entries.iter().filter(|e| e.key == *key).collect()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn actions(&self) -> HashSet<CredentialAction> {
        self.entries.iter().map(|e| e.action).collect()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ---- credential store ----

    #[test]
    fn store_and_retrieve() {
        let store = InMemoryCredentialStore::new();
        let key = CredentialKey::new("github", "token");
        store.store(&key, "ghp_abc123").unwrap();

        let value = store.retrieve(&key).unwrap();
        assert_eq!(value.as_deref(), Some("ghp_abc123"));
    }

    #[test]
    fn retrieve_missing_returns_none() {
        let store = InMemoryCredentialStore::new();
        let key = CredentialKey::new("github", "token");
        assert_eq!(store.retrieve(&key).unwrap(), None);
    }

    #[test]
    fn revoke_makes_credential_inaccessible() {
        let store = InMemoryCredentialStore::new();
        let key = CredentialKey::new("github", "token");
        store.store(&key, "ghp_abc123").unwrap();
        assert!(store.revoke(&key).unwrap());

        // After revocation, retrieve returns None.
        assert_eq!(store.retrieve(&key).unwrap(), None);
        assert!(store.is_revoked(&key).unwrap());
    }

    #[test]
    fn revoke_missing_errors() {
        let store = InMemoryCredentialStore::new();
        let key = CredentialKey::new("github", "token");
        assert!(store.revoke(&key).is_err());
    }

    #[test]
    fn double_revoke_errors() {
        let store = InMemoryCredentialStore::new();
        let key = CredentialKey::new("github", "token");
        store.store(&key, "val").unwrap();
        store.revoke(&key).unwrap();
        assert!(store.revoke(&key).is_err());
    }

    #[test]
    fn keys_lists_all() {
        let store = InMemoryCredentialStore::new();
        store
            .store(&CredentialKey::new("github", "token"), "a")
            .unwrap();
        store
            .store(&CredentialKey::new("linear", "api_key"), "b")
            .unwrap();
        let keys = store.keys().unwrap();
        assert_eq!(keys.len(), 2);
    }

    // ---- tool binding ----

    #[test]
    fn valid_context_passes() {
        let binding = ToolBinding {
            tool_name: "git_apply".into(),
            task_id: "t1".into(),
            repository: "owner/repo".into(),
            source: "github".into(),
            allowed_scopes: vec![ToolScope::Read, ToolScope::Apply],
        };
        assert!(validate_tool_context(&binding, "t1", "owner/repo", ToolScope::Apply).is_ok());
    }

    #[test]
    fn wrong_task_rejected() {
        let binding = ToolBinding {
            tool_name: "tool".into(),
            task_id: "t1".into(),
            repository: "repo".into(),
            source: "github".into(),
            allowed_scopes: vec![ToolScope::Write],
        };
        let err = validate_tool_context(&binding, "t2", "repo", ToolScope::Write).unwrap_err();
        assert!(matches!(err, ContextViolation::WrongTask { .. }));
    }

    #[test]
    fn wrong_repo_rejected() {
        let binding = ToolBinding {
            tool_name: "tool".into(),
            task_id: "t1".into(),
            repository: "repo-a".into(),
            source: "github".into(),
            allowed_scopes: vec![ToolScope::Write],
        };
        let err = validate_tool_context(&binding, "t1", "repo-b", ToolScope::Write).unwrap_err();
        assert!(matches!(err, ContextViolation::WrongRepository { .. }));
    }

    #[test]
    fn missing_scope_rejected() {
        let binding = ToolBinding {
            tool_name: "tool".into(),
            task_id: "t1".into(),
            repository: "repo".into(),
            source: "github".into(),
            allowed_scopes: vec![ToolScope::Read],
        };
        let err = validate_tool_context(&binding, "t1", "repo", ToolScope::Publish).unwrap_err();
        assert!(matches!(err, ContextViolation::MissingScope { .. }));
    }

    // ---- masking ----

    #[test]
    fn mask_produces_refs_not_values() {
        let store = InMemoryCredentialStore::new();
        let key = CredentialKey::new("github", "token");
        store.store(&key, "ghp_secret").unwrap();

        let masked = mask_credentials(&store, std::slice::from_ref(&key));
        assert_eq!(masked.len(), 1);
        assert!(masked[0].ref_id.contains("github"));
        assert!(!masked[0].ref_id.contains("ghp_secret"));
    }

    #[test]
    fn mask_excludes_revoked() {
        let store = InMemoryCredentialStore::new();
        let key = CredentialKey::new("github", "token");
        store.store(&key, "val").unwrap();
        store.revoke(&key).unwrap();

        let masked = mask_credentials(&store, &[key]);
        assert!(
            masked.is_empty(),
            "revoked credentials should not be masked"
        );
    }

    #[test]
    fn resolve_credential_roundtrip() {
        let store = InMemoryCredentialStore::new();
        let key = CredentialKey::new("github", "token");
        store.store(&key, "ghp_value").unwrap();

        let masked = mask_credentials(&store, &[key])[0].clone();
        let resolved = resolve_credential(&store, &masked.ref_id).unwrap();
        assert_eq!(resolved, "ghp_value");
    }

    #[test]
    fn resolve_revoked_fails() {
        let store = InMemoryCredentialStore::new();
        let key = CredentialKey::new("github", "token");
        store.store(&key, "val").unwrap();
        store.revoke(&key).unwrap();

        let result = resolve_credential(&store, "cred:github:token");
        assert!(result.is_err());
    }

    // ---- redaction ----

    #[test]
    fn redact_known_keys() {
        let patterns = default_redaction_patterns();
        let mut payload = json!({
            "api_key": "sk-1234567890",
            "data": "safe",
            "headers": {
                "authorization": "Bearer xyz",
                "content-type": "application/json"
            }
        });
        redact_credentials(&mut payload, &patterns);
        assert_eq!(payload["api_key"], "[REDACTED]");
        assert_eq!(payload["data"], "safe");
        assert_eq!(payload["headers"]["authorization"], "[REDACTED]");
        assert_eq!(payload["headers"]["content-type"], "application/json");
    }

    #[test]
    fn redact_nested_arrays() {
        let patterns = default_redaction_patterns();
        let mut payload = json!({
            "items": [
                {"token": "abc", "name": "ok"},
                {"password": "xyz", "id": 1}
            ]
        });
        redact_credentials(&mut payload, &patterns);
        assert_eq!(payload["items"][0]["token"], "[REDACTED]");
        assert_eq!(payload["items"][0]["name"], "ok");
        assert_eq!(payload["items"][1]["password"], "[REDACTED]");
    }

    #[test]
    fn redact_credential_prefixes_in_strings() {
        let patterns = default_redaction_patterns();
        let mut payload = json!({
            "url": "https://api.github.com",
            "raw_token": "ghp_1234567890abcdef",
            "slack": "xoxb-1234567890"
        });
        redact_credentials(&mut payload, &patterns);
        // "url" doesn't match any pattern, but the value has no known prefix.
        assert_eq!(payload["url"], "https://api.github.com");
        // "raw_token" matches "token" pattern → redacted.
        assert_eq!(payload["raw_token"], "[REDACTED]");
        // "slack" doesn't match a key pattern, but the value has "xoxb-" prefix.
        assert_eq!(payload["slack"], "[REDACTED]");
    }

    // ---- revocation gate ----

    #[test]
    fn gate_passes_with_active_credentials() {
        let store = InMemoryCredentialStore::new();
        let k1 = CredentialKey::new("github", "token");
        let k2 = CredentialKey::new("linear", "key");
        store.store(&k1, "a").unwrap();
        store.store(&k2, "b").unwrap();

        assert!(gate_on_credentials(&store, &[k1, k2]));
    }

    #[test]
    fn gate_fails_with_revoked_credential() {
        let store = InMemoryCredentialStore::new();
        let k1 = CredentialKey::new("github", "token");
        let k2 = CredentialKey::new("linear", "key");
        store.store(&k1, "a").unwrap();
        store.store(&k2, "b").unwrap();
        store.revoke(&k1).unwrap();

        // Revocation affects new calls immediately.
        assert!(!gate_on_credentials(&store, &[k1, k2]));
    }

    #[test]
    fn revocation_status_after_revoke() {
        let store = InMemoryCredentialStore::new();
        let key = CredentialKey::new("github", "token");
        store.store(&key, "val").unwrap();

        assert_eq!(check_revocation(&store, &key), RevocationStatus::Active);
        store.revoke(&key).unwrap();
        assert_eq!(check_revocation(&store, &key), RevocationStatus::Revoked);
    }

    // ---- audit log ----

    #[test]
    fn audit_log_records_actions() {
        let mut log = CredentialAuditLog::new();
        let key = CredentialKey::new("github", "token");

        log.record(CredentialAuditEntry {
            timestamp_ms: 1000,
            action: CredentialAction::Stored,
            key: key.clone(),
            task_id: None,
            tool_name: None,
        });
        log.record(CredentialAuditEntry {
            timestamp_ms: 2000,
            action: CredentialAction::Retrieved,
            key: key.clone(),
            task_id: Some("t1".into()),
            tool_name: Some("git_tool".into()),
        });

        assert_eq!(log.len(), 2);
        assert_eq!(log.for_key(&key).len(), 2);
        assert_eq!(log.for_source("github").len(), 2);
        assert!(log.actions().contains(&CredentialAction::Stored));
        assert!(log.actions().contains(&CredentialAction::Retrieved));
    }

    #[test]
    fn audit_log_empty() {
        let log = CredentialAuditLog::new();
        assert!(log.is_empty());
        assert_eq!(log.len(), 0);
    }

    // ---- I2 acceptance ----

    #[test]
    fn runner_never_receives_raw_credentials() {
        let store = InMemoryCredentialStore::new();
        let key = CredentialKey::new("github", "token");
        store.store(&key, "ghp_super_secret_value").unwrap();

        let masked = mask_credentials(&store, &[key]);
        // The masked reference should NOT contain the raw value.
        for m in &masked {
            assert!(!m.ref_id.contains("ghp_super_secret_value"));
            assert!(!m.label.contains("ghp_super_secret_value"));
        }
    }

    #[test]
    fn tool_cannot_mutate_different_task() {
        let binding = ToolBinding {
            tool_name: "git_apply".into(),
            task_id: "t1".into(),
            repository: "repo".into(),
            source: "github".into(),
            allowed_scopes: vec![ToolScope::Apply],
        };
        // Trying to apply to a different task → rejected.
        let result = validate_tool_context(&binding, "t2", "repo", ToolScope::Apply);
        assert!(result.is_err());
    }

    #[test]
    fn revocation_affects_new_calls_immediately() {
        let store = InMemoryCredentialStore::new();
        let key = CredentialKey::new("github", "token");
        store.store(&key, "val").unwrap();

        // Credential is active → gate passes.
        assert!(gate_on_credentials(&store, std::slice::from_ref(&key)));

        // Revoke → gate fails immediately.
        store.revoke(&key).unwrap();
        assert!(!gate_on_credentials(&store, &[key]));
    }
}
