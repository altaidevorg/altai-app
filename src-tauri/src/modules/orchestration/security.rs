//! Shared security primitives (plan §8 de-fragmentation).
//!
//! Provides unified, tested implementations of the security requirements that
//! were previously duplicated across multiple orchestration modules:
//!
//! - **Path containment**: normalize paths and verify they stay within a root
//!   (§8 #1, #11 — path normalization, symlink/traversal prevention).
//! - **Secret redaction**: unified pattern registry and text sanitization
//!   (§8 #4 — redact before logs/events/artifacts are persisted).
//! - **Payload bounding**: shared size limits for content, lines, and fields
//!   (§8 #5 — bounded payloads, output, line sizes, artifact sizes).
//!
//! These primitives are the canonical implementation; existing per-module
//! copies can be progressively replaced.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

// ===========================================================================
// Path containment (§8 #1, #11)
// ===========================================================================

/// Errors from path sandbox validation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum PathError {
    TraversalDetected { path: String },
    OutsideRoot { path: String, root: String },
    EmptyPath,
    AbsoluteRequired,
}

impl std::fmt::Display for PathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TraversalDetected { path } => {
                write!(f, "path traversal detected: '{path}' contains '..'")
            }
            Self::OutsideRoot { path, root } => {
                write!(f, "path '{path}' is outside root '{root}'")
            }
            Self::EmptyPath => write!(f, "path is empty"),
            Self::AbsoluteRequired => write!(f, "absolute path required"),
        }
    }
}

impl std::error::Error for PathError {}

/// Normalize a path string by resolving `.` and `..` segments lexically
/// (without touching the filesystem). This catches traversal attempts before
/// any canonicalization that might follow symlinks.
///
/// Returns the normalized absolute path, or an error if traversal escapes
/// the root.
pub fn normalize_path(path: &str) -> Result<String, PathError> {
    if path.trim().is_empty() {
        return Err(PathError::EmptyPath);
    }

    let mut segments: Vec<&str> = Vec::new();

    for component in path.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                if segments.pop().is_none() {
                    return Err(PathError::TraversalDetected {
                        path: path.to_string(),
                    });
                }
            }
            other => segments.push(other),
        }
    }

    let normalized = if path.starts_with('/') {
        format!("/{}", segments.join("/"))
    } else {
        segments.join("/")
    };

    Ok(if normalized.is_empty() {
        "/".to_string()
    } else {
        normalized
    })
}

/// Check if a (already normalized) path is within the given root.
/// Both paths must be absolute and normalized.
pub fn is_within_root(path: &str, root: &str) -> bool {
    if path == root {
        return true;
    }
    if root == "/" {
        return path.starts_with('/');
    }
    path.starts_with(root) && path[root.len()..].starts_with('/')
}

/// Validate that a path is within the sandbox root after normalization.
///
/// This is the canonical path containment check. It:
/// 1. Rejects empty paths.
/// 2. Rejects `..` traversal that escapes the root.
/// 3. Ensures the normalized path is within the root boundary.
pub fn validate_path(path: &str, root: &str) -> Result<String, PathError> {
    let normalized = normalize_path(path)?;
    if !normalized.starts_with('/') {
        return Err(PathError::AbsoluteRequired);
    }
    if !is_within_root(&normalized, root) {
        return Err(PathError::OutsideRoot {
            path: normalized,
            root: root.to_string(),
        });
    }
    Ok(normalized)
}

/// A path sandbox that validates multiple paths against a fixed root.
#[derive(Clone, Debug)]
pub struct PathSandbox {
    root: String,
}

impl PathSandbox {
    pub fn new(root: impl Into<String>) -> Result<Self, PathError> {
        let root = root.into();
        Ok(Self {
            root: normalize_path(&root)?,
        })
    }

    /// Validate and normalize a path within this sandbox.
    pub fn check(&self, path: &str) -> Result<String, PathError> {
        validate_path(path, &self.root)
    }

    /// Check if a path is within this sandbox without returning the normalized form.
    pub fn contains(&self, path: &str) -> bool {
        self.check(path).is_ok()
    }

    /// Get the root path.
    pub fn root(&self) -> &str {
        &self.root
    }
}

// ===========================================================================
// Secret redaction (§8 #4)
// ===========================================================================

/// A redaction pattern definition.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RedactionPattern {
    /// Human-readable name for the pattern (e.g. "AWS Access Key").
    pub name: String,
    /// Regex-like pattern (simplified: prefix + min length).
    pub prefix: String,
    /// Minimum length of the secret value after the prefix.
    pub min_length: usize,
}

/// Registry of known secret patterns for redaction.
#[derive(Clone, Debug, Default)]
pub struct RedactionRegistry {
    patterns: Vec<RedactionPattern>,
    /// Additional literal strings to redact (e.g. known API keys, tokens).
    literals: HashSet<String>,
}

impl RedactionRegistry {
    /// Create a registry pre-loaded with common secret patterns.
    pub fn with_defaults() -> Self {
        let mut registry = Self::default();
        for pattern in default_patterns() {
            registry.add_pattern(pattern);
        }
        registry
    }

    /// Add a custom redaction pattern.
    pub fn add_pattern(&mut self, pattern: RedactionPattern) {
        self.patterns.push(pattern);
    }

    /// Add a literal string to redact (e.g. a known token value).
    pub fn add_literal(&mut self, literal: impl Into<String>) {
        let lit = literal.into();
        if !lit.is_empty() {
            self.literals.insert(lit);
        }
    }

    /// Redact all known secrets from the given text.
    pub fn redact(&self, text: &str) -> String {
        let mut result = text.to_string();

        for pattern in &self.patterns {
            result = redact_pattern(&result, pattern);
        }

        for literal in &self.literals {
            if !literal.is_empty() && result.contains(literal.as_str()) {
                result = result.replace(literal, "[REDACTED]");
            }
        }

        result
    }

    /// Check if the text contains any known secrets.
    pub fn contains_secrets(&self, text: &str) -> bool {
        for pattern in &self.patterns {
            if find_pattern(text, pattern).is_some() {
                return true;
            }
        }
        for literal in &self.literals {
            if !literal.is_empty() && text.contains(literal.as_str()) {
                return true;
            }
        }
        false
    }

    /// Number of registered patterns.
    pub fn pattern_count(&self) -> usize {
        self.patterns.len()
    }

    /// Number of registered literals.
    pub fn literal_count(&self) -> usize {
        self.literals.len()
    }
}

/// Default secret patterns covering common credential types.
fn default_patterns() -> Vec<RedactionPattern> {
    vec![
        RedactionPattern {
            name: "AWS Access Key ID".to_string(),
            prefix: "AKIA".to_string(),
            min_length: 12,
        },
        RedactionPattern {
            name: "GitHub Token (ghp_)".to_string(),
            prefix: "ghp_".to_string(),
            min_length: 20,
        },
        RedactionPattern {
            name: "GitHub Token (github_pat_)".to_string(),
            prefix: "github_pat_".to_string(),
            min_length: 20,
        },
        RedactionPattern {
            name: "OpenAI API Key".to_string(),
            prefix: "sk-".to_string(),
            min_length: 20,
        },
        RedactionPattern {
            name: "Anthropic API Key".to_string(),
            prefix: "sk-ant-".to_string(),
            min_length: 20,
        },
        RedactionPattern {
            name: "Generic Bearer Token".to_string(),
            prefix: "Bearer ".to_string(),
            min_length: 10,
        },
    ]
}

/// Find the first occurrence of a pattern-based secret in text.
fn find_pattern(text: &str, pattern: &RedactionPattern) -> Option<(usize, usize)> {
    let mut search_start = 0;
    while let Some(pos) = text[search_start..].find(&pattern.prefix) {
        let abs_pos = search_start + pos;
        let end = abs_pos + pattern.prefix.len() + pattern.min_length;
        if end <= text.len() {
            return Some((abs_pos, end));
        }
        search_start = abs_pos + 1;
    }
    None
}

/// Redact a pattern-based secret from text.
fn redact_pattern(text: &str, pattern: &RedactionPattern) -> String {
    let mut redacted = String::new();
    let mut cursor = 0;

    while let Some((start, end)) = find_pattern(&text[cursor..], pattern) {
        let start = cursor + start;
        let end = cursor + end;
        redacted.push_str(&text[cursor..start + pattern.prefix.len()]);
        redacted.push_str("[REDACTED]");
        cursor = end;
    }

    redacted.push_str(&text[cursor..]);
    redacted
}

// ===========================================================================
// Payload bounding (§8 #5)
// ===========================================================================

/// Shared payload size limits.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayloadLimits {
    /// Maximum total bytes for a single payload (default: 1 MB).
    pub max_payload_bytes: usize,
    /// Maximum bytes for a single line (default: 64 KB).
    pub max_line_bytes: usize,
    /// Maximum number of lines (default: 10_000).
    pub max_lines: usize,
    /// Maximum bytes for an artifact (default: 10 MB).
    pub max_artifact_bytes: usize,
    /// Maximum total artifacts per task (default: 100 MB).
    pub max_total_artifact_bytes: usize,
}

impl Default for PayloadLimits {
    fn default() -> Self {
        Self {
            max_payload_bytes: 1_048_576, // 1 MB
            max_line_bytes: 65_536,       // 64 KB
            max_lines: 10_000,
            max_artifact_bytes: 10_485_760,        // 10 MB
            max_total_artifact_bytes: 104_857_600, // 100 MB
        }
    }
}

/// Result of checking a payload against limits.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum LimitExceeded {
    PayloadTooLarge { actual: usize, limit: usize },
    LineTooLong { actual: usize, limit: usize },
    TooManyLines { actual: usize, limit: usize },
    ArtifactTooLarge { actual: usize, limit: usize },
    TotalArtifactsTooLarge { actual: usize, limit: usize },
}

impl std::fmt::Display for LimitExceeded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PayloadTooLarge { actual, limit } => {
                write!(f, "payload too large: {actual} bytes (limit {limit})")
            }
            Self::LineTooLong { actual, limit } => {
                write!(f, "line too long: {actual} bytes (limit {limit})")
            }
            Self::TooManyLines { actual, limit } => {
                write!(f, "too many lines: {actual} (limit {limit})")
            }
            Self::ArtifactTooLarge { actual, limit } => {
                write!(f, "artifact too large: {actual} bytes (limit {limit})")
            }
            Self::TotalArtifactsTooLarge { actual, limit } => {
                write!(
                    f,
                    "total artifacts too large: {actual} bytes (limit {limit})"
                )
            }
        }
    }
}

impl std::error::Error for LimitExceeded {}

impl PayloadLimits {
    /// Check if a payload byte count is within limits.
    pub fn check_payload(&self, bytes: usize) -> Result<(), LimitExceeded> {
        if bytes > self.max_payload_bytes {
            return Err(LimitExceeded::PayloadTooLarge {
                actual: bytes,
                limit: self.max_payload_bytes,
            });
        }
        Ok(())
    }

    /// Check if a text payload is within byte and line limits.
    pub fn check_text(&self, text: &str) -> Result<(), LimitExceeded> {
        self.check_payload(text.len())?;
        let line_count = text.lines().count();
        if line_count > self.max_lines {
            return Err(LimitExceeded::TooManyLines {
                actual: line_count,
                limit: self.max_lines,
            });
        }
        for line in text.lines() {
            if line.len() > self.max_line_bytes {
                return Err(LimitExceeded::LineTooLong {
                    actual: line.len(),
                    limit: self.max_line_bytes,
                });
            }
        }
        Ok(())
    }

    /// Check if a single artifact is within size limits.
    pub fn check_artifact(&self, bytes: usize) -> Result<(), LimitExceeded> {
        if bytes > self.max_artifact_bytes {
            return Err(LimitExceeded::ArtifactTooLarge {
                actual: bytes,
                limit: self.max_artifact_bytes,
            });
        }
        Ok(())
    }

    /// Check if the total artifact bytes are within cumulative limits.
    pub fn check_total_artifacts(&self, total_bytes: usize) -> Result<(), LimitExceeded> {
        if total_bytes > self.max_total_artifact_bytes {
            return Err(LimitExceeded::TotalArtifactsTooLarge {
                actual: total_bytes,
                limit: self.max_total_artifact_bytes,
            });
        }
        Ok(())
    }

    /// Truncate text to fit within line limits, keeping the first N lines.
    pub fn truncate_lines(&self, text: &str) -> String {
        let lines: Vec<&str> = text.lines().take(self.max_lines).collect();
        lines.join("\n")
    }

    /// Truncate a byte buffer to fit within the payload limit.
    pub fn truncate_bytes<'a>(&self, bytes: &'a [u8]) -> &'a [u8] {
        &bytes[..bytes.len().min(self.max_payload_bytes)]
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Path normalization ----

    #[test]
    fn normalize_simple_path() {
        assert_eq!(normalize_path("/a/b/c").unwrap(), "/a/b/c");
    }

    #[test]
    fn normalize_resolves_dot() {
        assert_eq!(normalize_path("/a/./b").unwrap(), "/a/b");
    }

    #[test]
    fn normalize_resolves_double_dot() {
        assert_eq!(normalize_path("/a/b/../c").unwrap(), "/a/c");
    }

    #[test]
    fn normalize_rejects_traversal_above_root() {
        assert!(matches!(
            normalize_path("/a/../../b"),
            Err(PathError::TraversalDetected { .. })
        ));
    }

    #[test]
    fn normalize_handles_multiple_dots() {
        assert_eq!(normalize_path("/a/b/c/../../d").unwrap(), "/a/d");
    }

    #[test]
    fn normalize_rejects_empty_path() {
        assert!(matches!(normalize_path(""), Err(PathError::EmptyPath)));
        assert!(matches!(normalize_path("   "), Err(PathError::EmptyPath)));
    }

    #[test]
    fn normalize_handles_trailing_slash() {
        assert_eq!(normalize_path("/a/b/").unwrap(), "/a/b");
    }

    #[test]
    fn normalize_relative_path() {
        assert_eq!(normalize_path("a/b/c").unwrap(), "a/b/c");
        assert_eq!(normalize_path("a/../b").unwrap(), "b");
    }

    // ---- Path containment ----

    #[test]
    fn is_within_root_exact_match() {
        assert!(is_within_root("/workspace", "/workspace"));
    }

    #[test]
    fn is_within_root_child() {
        assert!(is_within_root("/workspace/src/main.rs", "/workspace"));
    }

    #[test]
    fn is_within_root_rejects_sibling() {
        assert!(!is_within_root("/other/file.txt", "/workspace"));
    }

    #[test]
    fn is_within_root_rejects_prefix_match_without_separator() {
        assert!(!is_within_root("/workspace-evil", "/workspace"));
    }

    #[test]
    fn is_within_root_accepts_children_of_filesystem_root() {
        assert!(is_within_root("/workspace/file.txt", "/"));
    }

    // ---- Path validation ----

    #[test]
    fn validate_accepts_path_within_root() {
        let result = validate_path("/workspace/src/main.rs", "/workspace");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "/workspace/src/main.rs");
    }

    #[test]
    fn validate_rejects_traversal() {
        let result = validate_path("/workspace/../etc/passwd", "/workspace");
        assert!(result.is_err());
    }

    #[test]
    fn validate_rejects_outside_root() {
        let result = validate_path("/etc/passwd", "/workspace");
        assert!(matches!(result, Err(PathError::OutsideRoot { .. })));
    }

    #[test]
    fn validate_normalizes_dots_before_checking() {
        let result = validate_path("/workspace/src/../src/main.rs", "/workspace");
        assert!(result.is_ok());
    }

    // ---- PathSandbox ----

    #[test]
    fn sandbox_validates_multiple_paths() {
        let sandbox = PathSandbox::new("/workspace").unwrap();
        assert!(sandbox.check("/workspace/a.rs").is_ok());
        assert!(sandbox.check("/workspace/sub/b.rs").is_ok());
        assert!(sandbox.check("/etc/passwd").is_err());
    }

    #[test]
    fn sandbox_contains_check() {
        let sandbox = PathSandbox::new("/workspace").unwrap();
        assert!(sandbox.contains("/workspace/file.txt"));
        assert!(!sandbox.contains("/other/file.txt"));
    }

    #[test]
    fn sandbox_rejects_traversal() {
        let sandbox = PathSandbox::new("/workspace").unwrap();
        assert!(!sandbox.contains("/workspace/../../etc/passwd"));
    }

    #[test]
    fn sandbox_rejects_invalid_root() {
        assert!(matches!(PathSandbox::new(""), Err(PathError::EmptyPath)));
    }

    // ---- Redaction: pattern matching ----

    #[test]
    fn redact_aws_access_key() {
        let registry = RedactionRegistry::with_defaults();
        let text = "key=AKIAIOSFODNN7EXAMPLE more text";
        let redacted = registry.redact(text);
        assert!(redacted.contains("[REDACTED]"));
        assert!(!redacted.contains("AKIAIOSFODNN7EXAMPLE"));
    }

    #[test]
    fn redact_github_token() {
        let registry = RedactionRegistry::with_defaults();
        let text = "token: ghp_1234567890abcdefghijklmnop";
        let redacted = registry.redact(text);
        assert!(redacted.contains("[REDACTED]"));
        assert!(!redacted.contains("ghp_1234567890abcdefghijklmnop"));
    }

    #[test]
    fn redact_openai_key() {
        let registry = RedactionRegistry::with_defaults();
        let text = "OPENAI_API_KEY=sk-1234567890abcdefghijklmnopqrstuvwxyz";
        let redacted = registry.redact(text);
        assert!(redacted.contains("[REDACTED]"));
    }

    #[test]
    fn redact_bearer_token() {
        let registry = RedactionRegistry::with_defaults();
        let text = "Authorization: Bearer eyJhbGciOiJIUzI1";
        let redacted = registry.redact(text);
        assert!(redacted.contains("[REDACTED]"));
    }

    #[test]
    fn redact_custom_literal() {
        let mut registry = RedactionRegistry::with_defaults();
        registry.add_literal("my-super-secret-token-12345");
        let text = "auth=my-super-secret-token-12345";
        let redacted = registry.redact(text);
        assert!(redacted.contains("[REDACTED]"));
        assert!(!redacted.contains("my-super-secret-token-12345"));
    }

    #[test]
    fn redact_preserves_clean_text() {
        let registry = RedactionRegistry::with_defaults();
        let text = "This is a normal log line with no secrets.";
        assert_eq!(registry.redact(text), text);
    }

    #[test]
    fn redact_multiple_secrets_in_one_text() {
        let mut registry = RedactionRegistry::with_defaults();
        registry.add_literal("literal-secret");
        let text = "aws=AKIAIOSFODNN7EXAMPLE and literal-secret";
        let redacted = registry.redact(text);
        assert!(redacted.matches("[REDACTED]").count() >= 2);
    }

    #[test]
    fn redact_all_occurrences_of_the_same_pattern() {
        let registry = RedactionRegistry::with_defaults();
        let text = "AKIAIOSFODNN7EXAMPLE then AKIAABCDEFGHIJKLMNOP";
        let redacted = registry.redact(text);
        assert_eq!(redacted.matches("[REDACTED]").count(), 2);
        assert!(!redacted.contains("AKIA"));
    }

    #[test]
    fn contains_secrets_detects_patterns() {
        let registry = RedactionRegistry::with_defaults();
        assert!(registry.contains_secrets("key=AKIAIOSFODNN7EXAMPLE"));
        assert!(!registry.contains_secrets("normal text"));
    }

    #[test]
    fn contains_secrets_detects_literals() {
        let mut registry = RedactionRegistry::with_defaults();
        registry.add_literal("hidden-token");
        assert!(registry.contains_secrets("using hidden-token here"));
    }

    #[test]
    fn redact_ignores_empty_literal() {
        let mut registry = RedactionRegistry::with_defaults();
        registry.add_literal("");
        let text = "some text";
        assert_eq!(registry.redact(text), text);
    }

    #[test]
    fn redact_ignores_short_prefix_match() {
        let registry = RedactionRegistry::with_defaults();
        let text = "sk-ab"; // too short for OpenAI pattern (min_length=20)
        assert_eq!(registry.redact(text), text);
    }

    #[test]
    fn redaction_pattern_count() {
        let registry = RedactionRegistry::with_defaults();
        assert!(registry.pattern_count() >= 6);
    }

    #[test]
    fn custom_pattern_detected() {
        let mut registry = RedactionRegistry::default();
        registry.add_pattern(RedactionPattern {
            name: "Custom".to_string(),
            prefix: "CUSTOM_".to_string(),
            min_length: 5,
        });
        let text = "val=CUSTOM_abcde12345";
        let redacted = registry.redact(text);
        assert!(redacted.contains("[REDACTED]"));
    }

    // ---- Payload limits ----

    #[test]
    fn check_payload_within_limit() {
        let limits = PayloadLimits::default();
        assert!(limits.check_payload(1000).is_ok());
    }

    #[test]
    fn check_payload_exceeds_limit() {
        let limits = PayloadLimits::default();
        assert!(matches!(
            limits.check_payload(2_000_000),
            Err(LimitExceeded::PayloadTooLarge { .. })
        ));
    }

    #[test]
    fn check_text_within_limits() {
        let limits = PayloadLimits::default();
        let text = "line1\nline2\nline3";
        assert!(limits.check_text(text).is_ok());
    }

    #[test]
    fn check_text_too_many_lines() {
        let limits = PayloadLimits {
            max_lines: 2,
            ..PayloadLimits::default()
        };
        let text = "a\nb\nc";
        assert!(matches!(
            limits.check_text(text),
            Err(LimitExceeded::TooManyLines { .. })
        ));
    }

    #[test]
    fn check_text_line_too_long() {
        let limits = PayloadLimits {
            max_line_bytes: 5,
            ..PayloadLimits::default()
        };
        let text = "short\nthis is too long";
        assert!(matches!(
            limits.check_text(text),
            Err(LimitExceeded::LineTooLong { .. })
        ));
    }

    #[test]
    fn check_artifact_within_limit() {
        let limits = PayloadLimits::default();
        assert!(limits.check_artifact(5_000_000).is_ok());
    }

    #[test]
    fn check_artifact_exceeds_limit() {
        let limits = PayloadLimits::default();
        assert!(matches!(
            limits.check_artifact(20_000_000),
            Err(LimitExceeded::ArtifactTooLarge { .. })
        ));
    }

    #[test]
    fn check_total_artifacts_within_limit() {
        let limits = PayloadLimits::default();
        assert!(limits.check_total_artifacts(50_000_000).is_ok());
    }

    #[test]
    fn check_total_artifacts_exceeds_limit() {
        let limits = PayloadLimits::default();
        assert!(matches!(
            limits.check_total_artifacts(200_000_000),
            Err(LimitExceeded::TotalArtifactsTooLarge { .. })
        ));
    }

    #[test]
    fn truncate_lines_caps_line_count() {
        let limits = PayloadLimits {
            max_lines: 2,
            ..PayloadLimits::default()
        };
        let text = "a\nb\nc\nd\ne";
        let truncated = limits.truncate_lines(text);
        assert_eq!(truncated, "a\nb");
    }

    #[test]
    fn truncate_bytes_caps_byte_count() {
        let limits = PayloadLimits {
            max_payload_bytes: 5,
            ..PayloadLimits::default()
        };
        let data = b"hello world";
        let truncated = limits.truncate_bytes(data);
        assert_eq!(truncated.len(), 5);
    }

    // ---- Error display ----

    #[test]
    fn path_error_display() {
        assert!(format!(
            "{}",
            PathError::TraversalDetected {
                path: "/x".to_string()
            }
        )
        .contains("traversal"));
        assert!(format!(
            "{}",
            PathError::OutsideRoot {
                path: "/x".to_string(),
                root: "/y".to_string()
            }
        )
        .contains("outside root"));
    }

    #[test]
    fn limit_exceeded_display() {
        assert!(format!(
            "{}",
            LimitExceeded::PayloadTooLarge {
                actual: 100,
                limit: 50
            }
        )
        .contains("payload"));
        assert!(format!(
            "{}",
            LimitExceeded::TooManyLines {
                actual: 100,
                limit: 50
            }
        )
        .contains("lines"));
    }

    // ---- Serialization ----

    #[test]
    fn payload_limits_serialize() {
        let limits = PayloadLimits::default();
        let json = serde_json::to_string(&limits).unwrap();
        let back: PayloadLimits = serde_json::from_str(&json).unwrap();
        assert_eq!(limits.max_payload_bytes, back.max_payload_bytes);
    }

    #[test]
    fn redaction_pattern_serialize() {
        let pattern = RedactionPattern {
            name: "test".to_string(),
            prefix: "PREFIX_".to_string(),
            min_length: 10,
        };
        let json = serde_json::to_string(&pattern).unwrap();
        let back: RedactionPattern = serde_json::from_str(&json).unwrap();
        assert_eq!(pattern, back);
    }
}
