//! Content-addressed artifact / evidence store (plan §D3).
//!
//! Stores proof-of-work artifacts (diffs, logs, test output, screenshots,
//! metrics, summaries) in content-addressed blob storage with durable metadata
//! in the ledger. Enforces size limits, verifies checksums, and supports
//! pin/retention/cleanup policies.

use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

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
    /// A caller supplied a value that is not a lowercase SHA-256 digest.
    InvalidChecksum { checksum: String },
    /// Artifact provenance names an attempt that does not exist.
    UnknownAttempt { attempt_id: String },
    /// Artifact provenance names an attempt owned by another task.
    AttemptTaskMismatch { attempt_id: String, task_id: String },
    /// A blob path is a symlink or another unsupported filesystem object.
    UnsafeBlobPath { path: PathBuf },
    /// Filesystem operation failed.
    Io(std::io::Error),
    /// Export manifest serialization failed.
    Json(serde_json::Error),
    /// Export destination has no safe parent or file name.
    InvalidExportPath { path: PathBuf },
    /// Export never overwrites an existing file.
    ExportAlreadyExists { path: PathBuf },
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
            Self::InvalidChecksum { checksum } => {
                write!(f, "invalid SHA-256 checksum {checksum:?}")
            }
            Self::UnknownAttempt { attempt_id } => {
                write!(f, "unknown artifact producer attempt {attempt_id}")
            }
            Self::AttemptTaskMismatch {
                attempt_id,
                task_id,
            } => write!(f, "attempt {attempt_id} does not belong to task {task_id}"),
            Self::UnsafeBlobPath { path } => {
                write!(f, "unsafe artifact blob path {}", path.display())
            }
            Self::Io(err) => write!(f, "artifact storage I/O error: {err}"),
            Self::Json(err) => write!(f, "artifact export JSON error: {err}"),
            Self::InvalidExportPath { path } => {
                write!(f, "invalid artifact export path {}", path.display())
            }
            Self::ExportAlreadyExists { path } => {
                write!(f, "artifact export already exists at {}", path.display())
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

impl From<std::io::Error> for ArtifactError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<serde_json::Error> for ArtifactError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err)
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
        validate_checksum(checksum)?;
        self.blobs
            .lock()
            .map_err(|_| ArtifactError::BlobNotFound {
                checksum: "lock poisoned".into(),
            })?
            .insert(checksum.to_string(), content.to_vec());
        Ok(())
    }

    fn retrieve(&self, checksum: &str) -> ArtifactResult<Vec<u8>> {
        validate_checksum(checksum)?;
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
        validate_checksum(checksum)?;
        self.blobs
            .lock()
            .map_err(|_| ArtifactError::BlobNotFound {
                checksum: "lock poisoned".into(),
            })?
            .remove(checksum);
        Ok(())
    }
}

/// Durable, content-addressed blob store rooted in ALTAI application data.
/// Digest validation makes every resolved blob path a direct child of the
/// canonical root, and symlinks are rejected before reads or deletion.
pub struct FileBlobStore {
    root: PathBuf,
}

impl FileBlobStore {
    pub fn new(root: impl AsRef<Path>) -> ArtifactResult<Self> {
        let requested = root.as_ref();
        if requested.exists() {
            reject_symlink_or_non_directory(requested)?;
        } else {
            fs::create_dir_all(requested)?;
            reject_symlink_or_non_directory(requested)?;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(requested, fs::Permissions::from_mode(0o700))?;
        }
        Ok(Self {
            root: fs::canonicalize(requested)?,
        })
    }

    /// Use the stable artifact location beneath the application's data
    /// directory. The caller supplies the platform-resolved app-data path.
    pub fn for_app_data(app_data_dir: impl AsRef<Path>) -> ArtifactResult<Self> {
        Self::new(
            app_data_dir
                .as_ref()
                .join("orchestration")
                .join("artifacts"),
        )
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn blob_path(&self, checksum: &str) -> ArtifactResult<PathBuf> {
        validate_checksum(checksum)?;
        Ok(self.root.join(checksum))
    }

    fn existing_blob(&self, path: &Path) -> ArtifactResult<Option<Vec<u8>>> {
        match fs::symlink_metadata(path) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || !metadata.is_file() {
                    return Err(ArtifactError::UnsafeBlobPath {
                        path: path.to_path_buf(),
                    });
                }
                Ok(Some(fs::read(path)?))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }
}

impl BlobStore for FileBlobStore {
    fn store(&self, checksum: &str, content: &[u8]) -> ArtifactResult<()> {
        let path = self.blob_path(checksum)?;
        if let Some(existing) = self.existing_blob(&path)? {
            return verify_blob(checksum, &existing);
        }
        verify_blob(checksum, content)?;

        let mut temporary = tempfile::NamedTempFile::new_in(&self.root)?;
        temporary.as_file_mut().write_all(content)?;
        temporary.as_file_mut().sync_all()?;
        match temporary.persist_noclobber(&path) {
            Ok(_) => Ok(()),
            Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {
                let existing =
                    self.existing_blob(&path)?
                        .ok_or_else(|| ArtifactError::BlobNotFound {
                            checksum: checksum.to_string(),
                        })?;
                verify_blob(checksum, &existing)
            }
            Err(error) => Err(ArtifactError::Io(error.error)),
        }
    }

    fn retrieve(&self, checksum: &str) -> ArtifactResult<Vec<u8>> {
        let path = self.blob_path(checksum)?;
        self.existing_blob(&path)?
            .ok_or_else(|| ArtifactError::BlobNotFound {
                checksum: checksum.to_string(),
            })
    }

    fn delete(&self, checksum: &str) -> ArtifactResult<()> {
        let path = self.blob_path(checksum)?;
        match fs::symlink_metadata(&path) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || !metadata.is_file() {
                    return Err(ArtifactError::UnsafeBlobPath { path });
                }
                fs::remove_file(path)?;
                Ok(())
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }
}

/// Configuration for the artifact store.
#[derive(Clone, Debug)]
pub struct ArtifactStoreConfig {
    pub max_artifact_bytes: u64,
    pub max_task_artifact_bytes: u64,
    /// Exact secret values to replace before content or descriptions are
    /// persisted. Empty values are ignored.
    pub secret_patterns: Vec<String>,
}

impl Default for ArtifactStoreConfig {
    fn default() -> Self {
        Self {
            max_artifact_bytes: DEFAULT_MAX_ARTIFACT_BYTES,
            max_task_artifact_bytes: DEFAULT_MAX_TASK_ARTIFACT_BYTES,
            secret_patterns: Vec::new(),
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
        let _operation = self.ledger.lock_artifact_operation()?;
        let attempt =
            self.ledger
                .attempt(attempt_id)?
                .ok_or_else(|| ArtifactError::UnknownAttempt {
                    attempt_id: attempt_id.to_string(),
                })?;
        if attempt.task_id != task_id {
            return Err(ArtifactError::AttemptTaskMismatch {
                attempt_id: attempt_id.to_string(),
                task_id: task_id.to_string(),
            });
        }

        let content = redact_bytes(content, &self.config.secret_patterns);
        let description = redact_text(description, &self.config.secret_patterns);
        let producer = redact_text(producer, &self.config.secret_patterns);
        let size = u64::try_from(content.len()).map_err(|_| ArtifactError::ArtifactTooLarge {
            size: u64::MAX,
            limit: self.config.max_artifact_bytes,
        })?;

        // Per-artifact size limit.
        if size > self.config.max_artifact_bytes {
            return Err(ArtifactError::ArtifactTooLarge {
                size,
                limit: self.config.max_artifact_bytes,
            });
        }

        // Content-addressed storage.
        let checksum = sha256_hex(&content);
        let artifact_id = format!("{task_id}:{attempt_id}:{}:{checksum}", kind.name());

        // Fast idempotent path: a retry must not consume quota again, but still
        // verifies/repairs the content-addressed blob.
        if let Some(existing) = self.ledger.artifact(&artifact_id)? {
            if existing.checksum != checksum || existing.size_bytes != size {
                return Err(ArtifactError::Ledger(
                    super::ledger::LedgerError::ArtifactConflict { artifact_id },
                ));
            }
            self.blobs.store(&checksum, &content)?;
            return Ok(existing);
        }
        self.blobs.store(&checksum, &content)?;

        // Ledger metadata.
        let request = CreateArtifactRequest {
            artifact_id: artifact_id.clone(),
            task_id: task_id.to_string(),
            attempt_id: attempt_id.to_string(),
            kind,
            checksum: checksum.clone(),
            size_bytes: size,
            producer,
            created_at_ms: now_ms,
            description,
        };
        match self
            .ledger
            .create_artifact_with_quota(&request, self.config.max_task_artifact_bytes)
        {
            Ok(_) => {}
            Err(super::ledger::LedgerError::ArtifactQuotaExceeded {
                current,
                attempted,
                limit,
            }) => {
                return Err(ArtifactError::TaskQuotaExceeded {
                    current,
                    attempted,
                    limit,
                });
            }
            Err(error) => return Err(error.into()),
        }

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
        let _operation = self.ledger.lock_artifact_operation()?;
        self.retrieve_unlocked(artifact_id)
    }

    fn retrieve_unlocked(&self, artifact_id: &str) -> ArtifactResult<Vec<u8>> {
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
        let _operation = self.ledger.lock_artifact_operation()?;
        self.ledger.pin_artifact(artifact_id)?;
        Ok(())
    }

    /// Unpin an artifact (allow cleanup).
    pub fn unpin(&self, artifact_id: &str) -> ArtifactResult<()> {
        let _operation = self.ledger.lock_artifact_operation()?;
        self.ledger.unpin_artifact(artifact_id)?;
        Ok(())
    }

    /// Export a task's verified, already-redacted evidence as a tar archive.
    /// Blob names are validated SHA-256 digests and an existing destination is
    /// never followed or overwritten.
    pub fn export(&self, task_id: &str, destination: &Path) -> ArtifactResult<ArtifactExport> {
        let _operation = self.ledger.lock_artifact_operation()?;
        let file_name =
            destination
                .file_name()
                .ok_or_else(|| ArtifactError::InvalidExportPath {
                    path: destination.to_path_buf(),
                })?;
        let parent = destination
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        reject_symlink_or_non_directory(parent)?;
        let canonical_parent = fs::canonicalize(parent)?;
        let destination = canonical_parent.join(file_name);
        match fs::symlink_metadata(&destination) {
            Ok(_) => return Err(ArtifactError::ExportAlreadyExists { path: destination }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }

        let artifacts = self.for_task(task_id)?;
        let manifest = ArtifactExportManifest {
            proof_of_work: proof_of_work(self.ledger, task_id)?,
            artifacts: artifacts.clone(),
        };
        let manifest_bytes = serde_json::to_vec_pretty(&manifest)?;
        let mut unique_checksums = HashSet::new();
        let mut temporary = tempfile::NamedTempFile::new_in(&canonical_parent)?;
        {
            let mut archive = tar::Builder::new(temporary.as_file_mut());
            append_tar_bytes(&mut archive, "manifest.json", &manifest_bytes)?;
            for artifact in &artifacts {
                if unique_checksums.insert(artifact.checksum.clone()) {
                    let content = self.retrieve_unlocked(&artifact.artifact_id)?;
                    append_tar_bytes(
                        &mut archive,
                        &format!("blobs/{}", artifact.checksum),
                        &content,
                    )?;
                }
            }
            archive.finish()?;
        }
        temporary.as_file_mut().sync_all()?;
        match temporary.persist_noclobber(&destination) {
            Ok(_) => Ok(ArtifactExport {
                path: destination,
                artifact_count: artifacts.len(),
                unique_blob_count: unique_checksums.len(),
            }),
            Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {
                Err(ArtifactError::ExportAlreadyExists { path: destination })
            }
            Err(error) => Err(ArtifactError::Io(error.error)),
        }
    }

    /// Delete unpinned artifacts older than `cutoff_ms`. Returns the deleted
    /// artifact IDs and frees their blobs.
    pub fn cleanup(&self, cutoff_ms: u64, limit: usize) -> ArtifactResult<Vec<String>> {
        let _operation = self.ledger.lock_artifact_operation()?;
        let candidates = self.ledger.cleanup_candidates(cutoff_ms, limit)?;
        let mut deleted = Vec::with_capacity(candidates.len());
        for artifact in candidates {
            if let Some(last_reference) = self
                .ledger
                .delete_cleanup_candidate(&artifact.artifact_id, cutoff_ms)?
            {
                if last_reference {
                    self.blobs.delete(&artifact.checksum)?;
                }
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

/// Result of exporting a task evidence archive.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactExport {
    pub path: PathBuf,
    pub artifact_count: usize,
    pub unique_blob_count: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ArtifactExportManifest {
    proof_of_work: ProofOfWork,
    artifacts: Vec<ArtifactRecord>,
}

/// Generate a proof-of-work summary for a task from its stored artifacts.
pub fn proof_of_work(ledger: &OrchestrationLedger, task_id: &str) -> LedgerResult<ProofOfWork> {
    let artifacts = ledger.artifacts_for_task(task_id)?;
    let mut by_kind: HashMap<ArtifactKind, (usize, u64)> = HashMap::new();
    let mut total = 0u64;

    for artifact in &artifacts {
        total = total.checked_add(artifact.size_bytes).ok_or(
            super::ledger::LedgerError::NumericOverflow("proof_of_work.total_size_bytes"),
        )?;
        let entry = by_kind.entry(artifact.kind).or_insert((0, 0));
        entry.0 += 1;
        entry.1 = entry.1.checked_add(artifact.size_bytes).ok_or(
            super::ledger::LedgerError::NumericOverflow("proof_of_work.kind_size_bytes"),
        )?;
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

fn validate_checksum(checksum: &str) -> ArtifactResult<()> {
    if checksum.len() == 64
        && checksum
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(ArtifactError::InvalidChecksum {
            checksum: checksum.to_string(),
        })
    }
}

fn verify_blob(expected: &str, content: &[u8]) -> ArtifactResult<()> {
    let actual = sha256_hex(content);
    if actual == expected {
        Ok(())
    } else {
        Err(ArtifactError::ChecksumMismatch {
            expected: expected.to_string(),
            actual,
        })
    }
}

fn reject_symlink_or_non_directory(path: &Path) -> ArtifactResult<()> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        Err(ArtifactError::UnsafeBlobPath {
            path: path.to_path_buf(),
        })
    } else {
        Ok(())
    }
}

fn redact_text(value: &str, patterns: &[String]) -> String {
    let mut redacted = value.to_string();
    for pattern in patterns {
        if !pattern.is_empty() {
            redacted = redacted.replace(pattern, "[REDACTED]");
        }
    }
    redacted
}

fn redact_bytes(value: &[u8], patterns: &[String]) -> Vec<u8> {
    let mut redacted = value.to_vec();
    for pattern in patterns {
        if pattern.is_empty() {
            continue;
        }
        redacted = replace_bytes(&redacted, pattern.as_bytes(), b"[REDACTED]");
    }
    redacted
}

fn replace_bytes(value: &[u8], needle: &[u8], replacement: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(value.len());
    let mut cursor = 0;
    while let Some(offset) = value[cursor..]
        .windows(needle.len())
        .position(|window| window == needle)
    {
        let match_start = cursor + offset;
        output.extend_from_slice(&value[cursor..match_start]);
        output.extend_from_slice(replacement);
        cursor = match_start + needle.len();
    }
    output.extend_from_slice(&value[cursor..]);
    output
}

fn append_tar_bytes(
    archive: &mut tar::Builder<&mut fs::File>,
    path: &str,
    content: &[u8],
) -> ArtifactResult<()> {
    let mut header = tar::Header::new_gnu();
    header.set_size(
        u64::try_from(content.len())
            .map_err(|_| ArtifactError::InvalidExportPath { path: path.into() })?,
    );
    header.set_mode(0o600);
    header.set_mtime(0);
    header.set_cksum();
    archive.append_data(&mut header, path, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::orchestration::domain::TaskState;
    use crate::modules::orchestration::ledger::{CreateAttemptRequest, TaskRecord};

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
        for attempt_no in 1..=2 {
            ledger
                .create_attempt(&CreateAttemptRequest {
                    attempt_id: format!("t-1-att-{attempt_no}"),
                    task_id: "t-1".into(),
                    attempt_no,
                    runner_kind: "native".into(),
                    lease: None,
                    idempotency_key: format!("attempt-{attempt_no}"),
                    now_ms: 1_500,
                })
                .unwrap();
        }
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
            secret_patterns: Vec::new(),
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
            secret_patterns: Vec::new(),
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

    #[test]
    fn cleanup_preserves_a_blob_shared_by_another_artifact() {
        let (ledger, blobs) = setup();
        let store = ArtifactStore::new(&ledger, &blobs);
        let first = store
            .store(
                "t-1",
                "t-1-att-1",
                ArtifactKind::Log,
                "runner",
                b"shared",
                1_000,
                "",
            )
            .unwrap();
        let second = store
            .store(
                "t-1",
                "t-1-att-2",
                ArtifactKind::Log,
                "runner",
                b"shared",
                2_000,
                "",
            )
            .unwrap();
        store.pin(&second.artifact_id).unwrap();

        assert_eq!(store.cleanup(5_000, 100).unwrap(), vec![first.artifact_id]);
        assert_eq!(store.retrieve(&second.artifact_id).unwrap(), b"shared");
    }

    #[test]
    fn idempotent_retry_does_not_consume_quota_twice() {
        let (ledger, blobs) = setup();
        let content = b"exact quota";
        let store = ArtifactStore::with_config(
            &ledger,
            &blobs,
            ArtifactStoreConfig {
                max_artifact_bytes: content.len() as u64,
                max_task_artifact_bytes: content.len() as u64,
                secret_patterns: Vec::new(),
            },
        );
        let first = store
            .store(
                "t-1",
                "t-1-att-1",
                ArtifactKind::Log,
                "runner",
                content,
                2_000,
                "",
            )
            .unwrap();
        let retry = store
            .store(
                "t-1",
                "t-1-att-1",
                ArtifactKind::Log,
                "runner",
                content,
                2_000,
                "",
            )
            .unwrap();
        assert_eq!(retry.artifact_id, first.artifact_id);
    }

    #[test]
    fn redacts_known_secrets_before_persistence() {
        let (ledger, blobs) = setup();
        let store = ArtifactStore::with_config(
            &ledger,
            &blobs,
            ArtifactStoreConfig {
                secret_patterns: vec!["sk-sensitive".into()],
                ..ArtifactStoreConfig::default()
            },
        );
        let record = store
            .store(
                "t-1",
                "t-1-att-1",
                ArtifactKind::Log,
                "runner-sk-sensitive",
                b"token=sk-sensitive",
                2_000,
                "used sk-sensitive",
            )
            .unwrap();

        assert_eq!(
            store.retrieve(&record.artifact_id).unwrap(),
            b"token=[REDACTED]"
        );
        assert_eq!(record.producer, "runner-[REDACTED]");
        assert_eq!(record.description, "used [REDACTED]");
    }

    #[test]
    fn export_contains_verified_redacted_evidence_and_never_overwrites() {
        let (ledger, blobs) = setup();
        let store = ArtifactStore::with_config(
            &ledger,
            &blobs,
            ArtifactStoreConfig {
                secret_patterns: vec!["raw-credential".into()],
                ..ArtifactStoreConfig::default()
            },
        );
        store
            .store(
                "t-1",
                "t-1-att-1",
                ArtifactKind::Log,
                "runner",
                b"token=raw-credential",
                2_000,
                "credential removed",
            )
            .unwrap();

        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("evidence.tar");
        let exported = store.export("t-1", &destination).unwrap();
        assert_eq!(exported.artifact_count, 1);
        assert_eq!(exported.unique_blob_count, 1);

        let bytes = fs::read(&destination).unwrap();
        assert!(!bytes
            .windows(b"raw-credential".len())
            .any(|window| window == b"raw-credential"));
        assert!(bytes
            .windows(b"[REDACTED]".len())
            .any(|window| window == b"[REDACTED]"));
        assert!(matches!(
            store.export("t-1", &destination),
            Err(ArtifactError::ExportAlreadyExists { .. })
        ));
    }

    #[test]
    fn file_blob_store_survives_reopen() {
        let directory = tempfile::tempdir().unwrap();
        let checksum = sha256_hex(b"durable evidence");
        {
            let blobs = FileBlobStore::new(directory.path()).unwrap();
            blobs.store(&checksum, b"durable evidence").unwrap();
        }
        let reopened = FileBlobStore::new(directory.path()).unwrap();
        assert_eq!(reopened.retrieve(&checksum).unwrap(), b"durable evidence");
    }

    #[test]
    fn artifact_metadata_and_content_survive_restart() {
        let directory = tempfile::tempdir().unwrap();
        let ledger_path = directory.path().join("orchestration.sqlite");
        let blob_root = directory.path().join("artifacts");
        let artifact_id = {
            let ledger = OrchestrationLedger::open(&ledger_path).unwrap();
            ledger
                .upsert_task(&TaskRecord {
                    task_id: "durable-task".into(),
                    workspace_key: "ws-1".into(),
                    source_kind: "local".into(),
                    source_ref: "durable-task".into(),
                    title: "Durable task".into(),
                    description: String::new(),
                    state: TaskState::Running,
                    created_at_ms: 1_000,
                    updated_at_ms: 1_000,
                })
                .unwrap();
            ledger
                .create_attempt(&CreateAttemptRequest {
                    attempt_id: "durable-attempt".into(),
                    task_id: "durable-task".into(),
                    attempt_no: 1,
                    runner_kind: "native".into(),
                    lease: None,
                    idempotency_key: "durable-attempt-key".into(),
                    now_ms: 1_500,
                })
                .unwrap();
            let blobs = FileBlobStore::new(&blob_root).unwrap();
            ArtifactStore::new(&ledger, &blobs)
                .store(
                    "durable-task",
                    "durable-attempt",
                    ArtifactKind::TestOutput,
                    "runner",
                    b"tests passed after restart",
                    2_000,
                    "",
                )
                .unwrap()
                .artifact_id
        };

        let reopened_ledger = OrchestrationLedger::open(&ledger_path).unwrap();
        let reopened_blobs = FileBlobStore::new(&blob_root).unwrap();
        assert_eq!(
            ArtifactStore::new(&reopened_ledger, &reopened_blobs)
                .retrieve(&artifact_id)
                .unwrap(),
            b"tests passed after restart"
        );
    }

    #[test]
    fn file_blob_store_rejects_untrusted_digest_paths() {
        let directory = tempfile::tempdir().unwrap();
        let blobs = FileBlobStore::new(directory.path()).unwrap();
        let result = blobs.retrieve("../outside");
        assert!(matches!(result, Err(ArtifactError::InvalidChecksum { .. })));
    }

    #[test]
    fn artifact_requires_an_attempt_belonging_to_the_task() {
        let (ledger, blobs) = setup();
        let result = ArtifactStore::new(&ledger, &blobs).store(
            "t-1",
            "unknown-attempt",
            ArtifactKind::Log,
            "runner",
            b"log",
            2_000,
            "",
        );
        assert!(matches!(result, Err(ArtifactError::UnknownAttempt { .. })));
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
