//! Content-addressed artifact / evidence store (plan §D3).
//!
//! Stores proof-of-work artifacts (diffs, logs, test output, screenshots,
//! metrics, summaries) in content-addressed blob storage with durable metadata
//! in the ledger. Enforces size limits, verifies checksums, and supports
//! pin/retention/cleanup policies.

use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;

use super::ledger::{
    ArtifactKind, ArtifactRecord, CreateArtifactRequest, LedgerResult, OrchestrationLedger,
};

/// Default per-artifact size limit: 10 MB.
pub const DEFAULT_MAX_ARTIFACT_BYTES: u64 = 10 * 1024 * 1024;
/// Default per-task total artifact size limit: 100 MB.
pub const DEFAULT_MAX_TASK_ARTIFACT_BYTES: u64 = 100 * 1024 * 1024;

/// Errors from the artifact store.
#[derive(Debug)]
pub enum ArtifactError {
    /// Content exceeds the per-artifact size limit.
    ArtifactTooLarge { size: u64, limit: u64 },
    /// Task's total artifact storage exceeds the per-task limit.
    TaskQuotaExceeded {
        current: u64,
        attempted: u64,
        limit: u64,
    },
    /// Retrieved blob checksum does not match the recorded checksum.
    ChecksumMismatch { expected: String, actual: String },
    /// The blob was not found in content-addressed storage.
    BlobNotFound { checksum: String },
    /// Underlying ledger error.
    Ledger(super::ledger::LedgerError),
}

impl std::fmt::Display for ArtifactError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ArtifactTooLarge { size, limit } => {
                write!(f, "artifact {size} bytes exceeds limit {limit}")
            }
            Self::TaskQuotaExceeded {
                current,
                attempted,
                limit,
            } => {
                write!(
                    f,
                    "task artifact quota {current} + {attempted} exceeds limit {limit}"
                )
            }
            Self::ChecksumMismatch { expected, actual } => {
                write!(f, "checksum mismatch: expected {expected}, got {actual}")
            }
            Self::BlobNotFound { checksum } => {
                write!(f, "blob not found for checksum {checksum}")
            }
            Self::Ledger(err) => write!(f, "ledger error: {err}"),
        }
    }
}

impl std::error::Error for ArtifactError {}

impl From<super::ledger::LedgerError> for ArtifactError {
    fn from(err: super::ledger::LedgerError) -> Self {
        Self::Ledger(err)
    }
}

pub type ArtifactResult<T> = Result<T, ArtifactError>;

/// Content-addressed blob storage. Implementations store raw bytes keyed by
/// their SHA-256 checksum.
pub trait BlobStore: Send + Sync {
    fn store(&self, checksum: &str, content: &[u8]) -> ArtifactResult<()>;
    fn retrieve(&self, checksum: &str) -> ArtifactResult<Vec<u8>>;
    fn delete(&self, checksum: &str) -> ArtifactResult<()>;
}

/// In-memory blob store for tests.
#[derive(Default)]
pub struct InMemoryBlobStore {
    blobs: std::sync::Mutex<HashMap<String, Vec<u8>>>,
}

impl BlobStore for InMemoryBlobStore {
    fn store(&self, checksum: &str, content: &[u8]) -> ArtifactResult<()> {
        self.blobs
            .lock()
            .map_err(|_| ArtifactError::BlobNotFound {
                checksum: "lock poisoned".into(),
            })?
            .insert(checksum.to_string(), content.to_vec());
        Ok(())
    }

    fn retrieve(&self, checksum: &str) -> ArtifactResult<Vec<u8>> {
        self.blobs
            .lock()
            .map_err(|_| ArtifactError::BlobNotFound {
                checksum: "lock poisoned".into(),
            })?
            .get(checksum)
            .cloned()
            .ok_or_else(|| ArtifactError::BlobNotFound {
                checksum: checksum.to_string(),
            })
    }

    fn delete(&self, checksum: &str) -> ArtifactResult<()> {
        self.blobs
            .lock()
            .map_err(|_| ArtifactError::BlobNotFound {
                checksum: "lock poisoned".into(),
            })?
            .remove(checksum);
        Ok(())
    }
}

/// Configuration for the artifact store.
#[derive(Clone, Debug)]
pub struct ArtifactStoreConfig {
    pub max_artifact_bytes: u64,
    pub max_task_artifact_bytes: u64,
}

impl Default for ArtifactStoreConfig {
    fn default() -> Self {
        Self {
            max_artifact_bytes: DEFAULT_MAX_ARTIFACT_BYTES,
            max_task_artifact_bytes: DEFAULT_MAX_TASK_ARTIFACT_BYTES,
        }
    }
}

/// The artifact store combines the durable ledger (metadata) with a blob
/// store (content). It enforces size limits and verifies checksums.
pub struct ArtifactStore<'a> {
    ledger: &'a OrchestrationLedger,
    blobs: &'a dyn BlobStore,
    config: ArtifactStoreConfig,
}

impl<'a> ArtifactStore<'a> {
    pub fn new(ledger: &'a OrchestrationLedger, blobs: &'a dyn BlobStore) -> Self {
        Self {
            ledger,
            blobs,
            config: ArtifactStoreConfig::default(),
        }
    }

    pub fn with_config(
        ledger: &'a OrchestrationLedger,
        blobs: &'a dyn BlobStore,
        config: ArtifactStoreConfig,
    ) -> Self {
        Self {
            ledger,
            blobs,
            config,
        }
    }

    /// Store an artifact. Computes the SHA-256 checksum, enforces size limits,
    /// writes the blob, and records metadata in the ledger.
    #[allow(clippy::too_many_arguments)]
    pub fn store(
        &self,
        task_id: &str,
        attempt_id: &str,
        kind: ArtifactKind,
        producer: &str,
        content: &[u8],
        now_ms: u64,
        description: &str,
    ) -> ArtifactResult<ArtifactRecord> {
        let size = content.len() as u64;

        // Per-artifact size limit.
        if size > self.config.max_artifact_bytes {
            return Err(ArtifactError::ArtifactTooLarge {
                size,
                limit: self.config.max_artifact_bytes,
            });
        }

        // Per-task quota check.
        let existing = self.ledger.artifacts_for_task(task_id)?;
        let current_total: u64 = existing.iter().map(|a| a.size_bytes).sum();
        if current_total + size > self.config.max_task_artifact_bytes {
            return Err(ArtifactError::TaskQuotaExceeded {
                current: current_total,
                attempted: size,
                limit: self.config.max_task_artifact_bytes,
            });
        }

        // Content-addressed storage.
        let checksum = sha256_hex(content);
        self.blobs.store(&checksum, content)?;

        // Ledger metadata.
        let artifact_id = format!("{task_id}:{attempt_id}:{}:{checksum}", kind.name());
        // Record metadata (idempotent — duplicate artifact_id returns existing).
        self.ledger.create_artifact(&CreateArtifactRequest {
            artifact_id: artifact_id.clone(),
            task_id: task_id.to_string(),
            attempt_id: attempt_id.to_string(),
            kind,
            checksum: checksum.clone(),
            size_bytes: size,
            producer: producer.to_string(),
            created_at_ms: now_ms,
            description: description.to_string(),
        })?;

        let record = self
            .ledger
            .artifact(&artifact_id)?
            .ok_or(ArtifactError::BlobNotFound {
                checksum: checksum.clone(),
            })?;

        Ok(record)
    }

    /// Retrieve artifact content, verifying the checksum matches.
    pub fn retrieve(&self, artifact_id: &str) -> ArtifactResult<Vec<u8>> {
        let record = self
            .ledger
            .artifact(artifact_id)?
            .ok_or(ArtifactError::BlobNotFound {
                checksum: artifact_id.to_string(),
            })?;

        let content = self.blobs.retrieve(&record.checksum)?;

        // Verify checksum integrity.
        let actual = sha256_hex(&content);
        if actual != record.checksum {
            return Err(ArtifactError::ChecksumMismatch {
                expected: record.checksum,
                actual,
            });
        }

        Ok(content)
    }

    /// List all artifacts for a task.
    pub fn for_task(&self, task_id: &str) -> ArtifactResult<Vec<ArtifactRecord>> {
        Ok(self.ledger.artifacts_for_task(task_id)?)
    }

    /// Pin an artifact (protect from cleanup).
    pub fn pin(&self, artifact_id: &str) -> ArtifactResult<()> {
        self.ledger.pin_artifact(artifact_id)?;
        Ok(())
    }

    /// Unpin an artifact (allow cleanup).
    pub fn unpin(&self, artifact_id: &str) -> ArtifactResult<()> {
        self.ledger.unpin_artifact(artifact_id)?;
        Ok(())
    }

    /// Delete unpinned artifacts older than `cutoff_ms`. Returns the deleted
    /// artifact IDs and frees their blobs.
    pub fn cleanup(&self, cutoff_ms: u64, limit: usize) -> ArtifactResult<Vec<String>> {
        let candidates = self.ledger.cleanup_candidates(cutoff_ms, limit)?;
        let mut deleted = Vec::with_capacity(candidates.len());
        for artifact in candidates {
            if self.ledger.delete_artifact(&artifact.artifact_id)? {
                let _ = self.blobs.delete(&artifact.checksum);
                deleted.push(artifact.artifact_id);
            }
        }
        Ok(deleted)
    }
}

/// A proof-of-work summary aggregating all evidence for a task.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofOfWork {
    pub task_id: String,
    pub artifact_count: usize,
    pub total_size_bytes: u64,
    pub kinds: Vec<ArtifactSummary>,
}

/// Per-kind artifact summary within a proof-of-work bundle.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactSummary {
    pub kind: ArtifactKind,
    pub count: usize,
    pub total_size_bytes: u64,
}

/// Generate a proof-of-work summary for a task from its stored artifacts.
pub fn proof_of_work(ledger: &OrchestrationLedger, task_id: &str) -> LedgerResult<ProofOfWork> {
    let artifacts = ledger.artifacts_for_task(task_id)?;
    let mut by_kind: HashMap<ArtifactKind, (usize, u64)> = HashMap::new();
    let mut total = 0u64;

    for artifact in &artifacts {
        total += artifact.size_bytes;
        let entry = by_kind.entry(artifact.kind).or_insert((0, 0));
        entry.0 += 1;
        entry.1 += artifact.size_bytes;
    }

    let mut kinds: Vec<ArtifactSummary> = by_kind
        .into_iter()
        .map(|(kind, (count, size))| ArtifactSummary {
            kind,
            count,
            total_size_bytes: size,
        })
        .collect();
    kinds.sort_by_key(|k| k.kind.name());

    Ok(ProofOfWork {
        task_id: task_id.to_string(),
        artifact_count: artifacts.len(),
        total_size_bytes: total,
        kinds,
    })
}

/// Compute the SHA-256 hex digest of content.
pub fn sha256_hex(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    let result = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for byte in result {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

// Suppress unused import warning when Path is only used in production impls.
#[allow(dead_code)]
fn _path_used(_p: &Path) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::orchestration::domain::TaskState;
    use crate::modules::orchestration::ledger::TaskRecord;

    fn setup() -> (OrchestrationLedger, InMemoryBlobStore) {
        let ledger = OrchestrationLedger::open_in_memory().unwrap();
        ledger
            .upsert_task(&TaskRecord {
                task_id: "t-1".into(),
                workspace_key: "ws-1".into(),
                source_kind: "local".into(),
                source_ref: "t-1".into(),
                title: "Task 1".into(),
                description: "do work".into(),
                state: TaskState::Running,
                created_at_ms: 1_000,
                updated_at_ms: 1_000,
            })
            .unwrap();
        (ledger, InMemoryBlobStore::default())
    }

    // ---- store + retrieve ----

    #[test]
    fn store_and_retrieve_roundtrip() {
        let (ledger, blobs) = setup();
        let store = ArtifactStore::new(&ledger, &blobs);

        let content = b"diff --git a/foo.rs b/foo.rs\n+added line";
        let record = store
            .store(
                "t-1",
                "t-1-att-1",
                ArtifactKind::Diff,
                "runner",
                content,
                2_000,
                "changes",
            )
            .unwrap();

        assert_eq!(record.kind, ArtifactKind::Diff);
        assert_eq!(record.size_bytes, content.len() as u64);
        assert_eq!(record.checksum.len(), 64);

        let retrieved = store.retrieve(&record.artifact_id).unwrap();
        assert_eq!(retrieved, content);
    }

    // ---- checksum verification ----

    #[test]
    fn retrieve_detects_corruption() {
        let (ledger, blobs) = setup();
        let store = ArtifactStore::new(&ledger, &blobs);

        let record = store
            .store(
                "t-1",
                "t-1-att-1",
                ArtifactKind::Log,
                "runner",
                b"original log",
                2_000,
                "",
            )
            .unwrap();

        // Corrupt the blob in storage.
        blobs.store(&record.checksum, b"corrupted content").unwrap();

        let result = store.retrieve(&record.artifact_id);
        assert!(matches!(
            result,
            Err(ArtifactError::ChecksumMismatch { .. })
        ));
    }

    // ---- idempotent store ----

    #[test]
    fn store_same_content_is_idempotent() {
        let (ledger, blobs) = setup();
        let store = ArtifactStore::new(&ledger, &blobs);

        let content = b"test output\nall tests passed";
        let r1 = store
            .store(
                "t-1",
                "t-1-att-1",
                ArtifactKind::TestOutput,
                "runner",
                content,
                2_000,
                "",
            )
            .unwrap();
        let r2 = store
            .store(
                "t-1",
                "t-1-att-1",
                ArtifactKind::TestOutput,
                "runner",
                content,
                2_000,
                "",
            )
            .unwrap();

        assert_eq!(r1.artifact_id, r2.artifact_id);
        let artifacts = store.for_task("t-1").unwrap();
        assert_eq!(artifacts.len(), 1);
    }

    // ---- per-artifact size limit ----

    #[test]
    fn rejects_artifact_exceeding_size_limit() {
        let (ledger, blobs) = setup();
        let config = ArtifactStoreConfig {
            max_artifact_bytes: 10,
            max_task_artifact_bytes: 1000,
        };
        let store = ArtifactStore::with_config(&ledger, &blobs, config);

        let result = store.store(
            "t-1",
            "t-1-att-1",
            ArtifactKind::Log,
            "runner",
            b"this is more than 10 bytes",
            2_000,
            "",
        );
        assert!(matches!(
            result,
            Err(ArtifactError::ArtifactTooLarge { .. })
        ));
    }

    // ---- per-task quota ----

    #[test]
    fn rejects_when_task_quota_exceeded() {
        let (ledger, blobs) = setup();
        let config = ArtifactStoreConfig {
            max_artifact_bytes: 100,
            max_task_artifact_bytes: 50,
        };
        let store = ArtifactStore::with_config(&ledger, &blobs, config);

        let result = store.store(
            "t-1",
            "t-1-att-1",
            ArtifactKind::Log,
            "runner",
            b"this content is definitely more than fifty bytes long yes indeed",
            2_000,
            "",
        );
        assert!(matches!(
            result,
            Err(ArtifactError::TaskQuotaExceeded { .. })
        ));
    }

    // ---- pin/unpin ----

    #[test]
    fn pin_protects_from_cleanup() {
        let (ledger, blobs) = setup();
        let store = ArtifactStore::new(&ledger, &blobs);

        let unpinned = store
            .store(
                "t-1",
                "t-1-att-1",
                ArtifactKind::Log,
                "runner",
                b"old log",
                1_000,
                "",
            )
            .unwrap();
        let pinned = store
            .store(
                "t-1",
                "t-1-att-2",
                ArtifactKind::Log,
                "runner",
                b"important",
                2_000,
                "",
            )
            .unwrap();

        store.pin(&pinned.artifact_id).unwrap();

        // Cleanup everything older than 5_000 — but nothing qualifies (both
        // are younger). Use a future cutoff to test.
        let deleted = store.cleanup(5_000, 100).unwrap();
        assert!(deleted.contains(&unpinned.artifact_id));
        assert!(!deleted.contains(&pinned.artifact_id));

        // Pinned artifact survives.
        assert!(store.retrieve(&pinned.artifact_id).is_ok());
    }

    // ---- multiple kinds for one task ----

    #[test]
    fn multiple_artifact_kinds_for_one_task() {
        let (ledger, blobs) = setup();
        let store = ArtifactStore::new(&ledger, &blobs);

        store
            .store(
                "t-1",
                "t-1-att-1",
                ArtifactKind::Diff,
                "runner",
                b"diff",
                2_000,
                "",
            )
            .unwrap();
        store
            .store(
                "t-1",
                "t-1-att-1",
                ArtifactKind::Log,
                "runner",
                b"log",
                3_000,
                "",
            )
            .unwrap();
        store
            .store(
                "t-1",
                "t-1-att-1",
                ArtifactKind::TestOutput,
                "runner",
                b"tests",
                4_000,
                "",
            )
            .unwrap();
        store
            .store(
                "t-1",
                "t-1-att-1",
                ArtifactKind::Metrics,
                "runner",
                b"metrics",
                5_000,
                "",
            )
            .unwrap();

        let artifacts = store.for_task("t-1").unwrap();
        assert_eq!(artifacts.len(), 4);
    }

    // ---- proof of work ----

    #[test]
    fn proof_of_work_aggregates_artifacts() {
        let (ledger, blobs) = setup();
        let store = ArtifactStore::new(&ledger, &blobs);

        store
            .store(
                "t-1",
                "t-1-att-1",
                ArtifactKind::Diff,
                "runner",
                b"diff content",
                2_000,
                "",
            )
            .unwrap();
        store
            .store(
                "t-1",
                "t-1-att-1",
                ArtifactKind::Log,
                "runner",
                b"log content here",
                3_000,
                "",
            )
            .unwrap();
        store
            .store(
                "t-1",
                "t-1-att-1",
                ArtifactKind::TestOutput,
                "runner",
                b"tests passed",
                4_000,
                "",
            )
            .unwrap();
        store
            .store(
                "t-1",
                "t-1-att-2",
                ArtifactKind::Diff,
                "runner",
                b"second diff",
                5_000,
                "",
            )
            .unwrap();

        let pow = proof_of_work(&ledger, "t-1").unwrap();
        assert_eq!(pow.task_id, "t-1");
        assert_eq!(pow.artifact_count, 4);
        assert!(pow.total_size_bytes > 0);

        // Two diffs, one log, one test_output.
        let diff_kind = pow
            .kinds
            .iter()
            .find(|k| k.kind == ArtifactKind::Diff)
            .unwrap();
        assert_eq!(diff_kind.count, 2);
    }

    #[test]
    fn proof_of_work_empty_task() {
        let ledger = OrchestrationLedger::open_in_memory().unwrap();
        let pow = proof_of_work(&ledger, "nonexistent").unwrap();
        assert_eq!(pow.artifact_count, 0);
        assert_eq!(pow.total_size_bytes, 0);
        assert!(pow.kinds.is_empty());
    }

    // ---- sha256 ----

    #[test]
    fn sha256_is_deterministic() {
        let a = sha256_hex(b"hello");
        let b = sha256_hex(b"hello");
        assert_eq!(a, b);
        assert_eq!(a.len(), 64);
    }

    #[test]
    fn sha256_differs_for_different_content() {
        let a = sha256_hex(b"hello");
        let b = sha256_hex(b"world");
        assert_ne!(a, b);
    }

    // ---- migration v4 ----

    #[test]
    fn migration_v4_creates_artifacts_table() {
        let ledger = OrchestrationLedger::open_in_memory().unwrap();
        // If table exists, we can query it.
        let result = ledger.artifacts_for_task("any-task");
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
