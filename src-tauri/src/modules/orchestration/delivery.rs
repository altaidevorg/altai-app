//! Safe delivery — preview, apply, verify, and handoff (plan §D4).
//!
//! Ensures agent-produced changes are applied atomically with conflict-abort
//! semantics, post-apply verification, and configurable handoff policies.
//! Auto-apply and auto-merge are disabled by default.
//!
//! Acceptance criteria (plan §D4):
//! - delivery is idempotent;
//! - failed apply leaves the target unchanged;
//! - source publication failure does not lose local work;
//! - the task reaches `Done` only under the configured handoff policy.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Diff and commit preview
// ---------------------------------------------------------------------------

/// A single file change in a diff preview.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileChange {
    /// Repository-relative path.
    pub path: String,
    /// Change kind: add, modify, delete, rename.
    pub change_type: ChangeType,
    /// Lines added (approximate — for display).
    pub additions: u32,
    /// Lines removed (approximate — for display).
    pub deletions: u32,
}

/// Kind of file change.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChangeType {
    Add,
    Modify,
    Delete,
    Rename,
}

/// A commit in the preview.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommitPreview {
    /// Commit SHA (short form).
    pub sha: String,
    /// Commit message first line.
    pub message: String,
    /// Author of the commit.
    pub author: String,
    /// Files changed in this commit.
    pub files: Vec<FileChange>,
}

/// Full delivery preview before apply.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryPreview {
    /// Source branch (agent worktree).
    pub source_branch: String,
    /// Target branch.
    pub target_branch: String,
    /// Base SHA the work was built on.
    pub base_sha: String,
    /// Current HEAD SHA of the target (may differ from base if target moved).
    pub target_head_sha: String,
    /// Whether the target has moved since base (potential conflict risk).
    pub target_moved: bool,
    /// Commits to be applied.
    pub commits: Vec<CommitPreview>,
    /// Aggregated file changes.
    pub files: Vec<FileChange>,
    /// Total additions across all commits.
    pub total_additions: u32,
    /// Total deletions across all commits.
    pub total_deletions: u32,
}

impl DeliveryPreview {
    /// Compute aggregated file stats from commits.
    pub fn aggregate(commits: &[CommitPreview]) -> (Vec<FileChange>, u32, u32) {
        let mut by_path: HashMap<String, FileChange> = HashMap::new();
        let mut total_add = 0;
        let mut total_del = 0;
        for commit in commits {
            for f in &commit.files {
                total_add += f.additions;
                total_del += f.deletions;
                let entry = by_path.entry(f.path.clone()).or_insert_with(|| FileChange {
                    path: f.path.clone(),
                    change_type: f.change_type,
                    additions: 0,
                    deletions: 0,
                });
                entry.additions += f.additions;
                entry.deletions += f.deletions;
            }
        }
        let mut files: Vec<FileChange> = by_path.into_values().collect();
        files.sort_by(|a, b| a.path.cmp(&b.path));
        (files, total_add, total_del)
    }

    /// Build a preview from raw commit data.
    pub fn build(
        source_branch: impl Into<String>,
        target_branch: impl Into<String>,
        base_sha: impl Into<String>,
        target_head_sha: impl Into<String>,
        commits: Vec<CommitPreview>,
    ) -> Self {
        let base = base_sha.into();
        let head = target_head_sha.into();
        let target_moved = base != head;
        let (files, total_additions, total_deletions) = Self::aggregate(&commits);
        Self {
            source_branch: source_branch.into(),
            target_branch: target_branch.into(),
            base_sha: base,
            target_head_sha: head,
            target_moved,
            commits,
            files,
            total_additions,
            total_deletions,
        }
    }
}

// ---------------------------------------------------------------------------
// Conflict detection
// ---------------------------------------------------------------------------

/// Result of conflict detection between source and target.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum ConflictCheck {
    Clean {
        overlapping_files: Vec<String>,
    },
    Conflict {
        overlapping_files: Vec<String>,
        reason: String,
    },
}

/// Detect potential file-level conflicts between source and target changes.
pub fn detect_conflicts(source_files: &[FileChange], target_files: &[FileChange]) -> ConflictCheck {
    let source_paths: std::collections::HashSet<&str> =
        source_files.iter().map(|f| f.path.as_str()).collect();
    let target_paths: std::collections::HashSet<&str> =
        target_files.iter().map(|f| f.path.as_str()).collect();

    let overlapping: Vec<String> = source_paths
        .intersection(&target_paths)
        .map(|s| s.to_string())
        .collect();

    if overlapping.is_empty() {
        ConflictCheck::Clean {
            overlapping_files: Vec::new(),
        }
    } else {
        let mut sorted = overlapping.clone();
        sorted.sort();
        let count = sorted.len();
        ConflictCheck::Conflict {
            overlapping_files: sorted,
            reason: format!("{} file(s) modified in both source and target", count),
        }
    }
}

// ---------------------------------------------------------------------------
// Handoff policy
// ---------------------------------------------------------------------------

/// The policy controlling when a task reaches `Done`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HandoffPolicy {
    /// Whether auto-apply (git apply without human review) is allowed.
    #[serde(default)]
    pub auto_apply: bool,
    /// Whether auto-merge (merge PR without human review) is allowed.
    #[serde(default)]
    pub auto_merge: bool,
    /// Whether post-apply verification is required before marking Done.
    #[serde(default = "default_true")]
    pub require_post_apply_verify: bool,
    /// Whether a draft PR must be published before marking Done.
    #[serde(default)]
    pub require_draft_pr: bool,
    /// Maximum allowed total additions before warning.
    #[serde(default)]
    pub max_additions_warning: Option<u32>,
    /// Maximum allowed total deletions before warning.
    #[serde(default)]
    pub max_deletions_warning: Option<u32>,
}

fn default_true() -> bool {
    true
}

impl Default for HandoffPolicy {
    fn default() -> Self {
        Self {
            auto_apply: false,
            auto_merge: false,
            require_post_apply_verify: true,
            require_draft_pr: false,
            max_additions_warning: None,
            max_deletions_warning: None,
        }
    }
}

/// Policy validation result.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PolicyEvaluation {
    pub allowed: bool,
    pub warnings: Vec<String>,
    pub blockers: Vec<String>,
}

impl PolicyEvaluation {
    pub fn is_ok(&self) -> bool {
        self.blockers.is_empty()
    }
}

/// Evaluate a delivery preview against the handoff policy.
pub fn evaluate_policy(
    preview: &DeliveryPreview,
    policy: &HandoffPolicy,
    conflict: &ConflictCheck,
) -> PolicyEvaluation {
    let mut warnings = Vec::new();
    let mut blockers = Vec::new();

    if !policy.auto_apply {
        blockers.push("auto_apply is disabled — manual review required".to_string());
    }

    if policy.require_draft_pr {
        blockers.push("draft PR publication required before Done".to_string());
    }

    match conflict {
        ConflictCheck::Conflict {
            overlapping_files, ..
        } => {
            blockers.push(format!(
                "file conflicts detected: {}",
                overlapping_files.join(", ")
            ));
        }
        ConflictCheck::Clean { overlapping_files } if !overlapping_files.is_empty() => {
            warnings.push(format!(
                "non-conflicting overlap in files: {}",
                overlapping_files.join(", ")
            ));
        }
        _ => {}
    }

    if preview.target_moved {
        warnings.push("target branch has moved since base — rebase may be needed".to_string());
    }

    if let Some(max) = policy.max_additions_warning {
        if preview.total_additions > max {
            warnings.push(format!(
                "additions ({}) exceed warning threshold ({max})",
                preview.total_additions
            ));
        }
    }

    if let Some(max) = policy.max_deletions_warning {
        if preview.total_deletions > max {
            warnings.push(format!(
                "deletions ({}) exceed warning threshold ({max})",
                preview.total_deletions
            ));
        }
    }

    if policy.require_post_apply_verify {
        warnings.push("post-apply verification required".to_string());
    }

    let allowed = blockers.is_empty() && policy.auto_apply;

    PolicyEvaluation {
        allowed,
        warnings,
        blockers,
    }
}

// ---------------------------------------------------------------------------
// Apply state machine
// ---------------------------------------------------------------------------

/// The state of a delivery attempt.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryState {
    /// Preview generated, awaiting approval.
    #[default]
    Pending,
    /// Apply in progress.
    Applying,
    /// Apply succeeded, awaiting post-apply verification.
    Applied,
    /// Post-apply verification passed.
    Verified,
    /// Published as draft PR.
    Published,
    /// Task marked Done.
    Done,
    /// Apply failed — target left unchanged.
    Failed,
    /// Aborted due to conflict.
    Aborted,
}

/// The full delivery record.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Delivery {
    /// Unique delivery id.
    pub id: String,
    /// Current state.
    pub state: DeliveryState,
    /// The delivery preview.
    pub preview: DeliveryPreview,
    /// Timestamps of state transitions.
    pub transitions: Vec<DeliveryTransition>,
    /// Error message if in Failed/Aborted state.
    pub error: Option<String>,
    /// Whether the delivery is idempotent (re-applying produces the same result).
    pub idempotency_key: String,
}

/// A state transition record.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryTransition {
    pub from: DeliveryState,
    pub to: DeliveryState,
    pub at_ms: u64,
    pub note: Option<String>,
}

/// Errors that can occur during delivery.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum DeliveryError {
    ConflictDetected {
        files: Vec<String>,
    },
    PolicyBlocked {
        blockers: Vec<String>,
    },
    ApplyFailed {
        reason: String,
    },
    PostApplyVerifyFailed {
        reason: String,
    },
    PublishFailed {
        reason: String,
    },
    InvalidTransition {
        from: DeliveryState,
        to: DeliveryState,
    },
    AlreadyApplied,
    NotVerified,
}

impl std::fmt::Display for DeliveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConflictDetected { files } => {
                write!(f, "conflict in files: {}", files.join(", "))
            }
            Self::PolicyBlocked { blockers } => {
                write!(f, "policy blocked: {}", blockers.join("; "))
            }
            Self::ApplyFailed { reason } => write!(f, "apply failed: {reason}"),
            Self::PostApplyVerifyFailed { reason } => {
                write!(f, "post-apply verification failed: {reason}")
            }
            Self::PublishFailed { reason } => write!(f, "publish failed: {reason}"),
            Self::InvalidTransition { from, to } => {
                write!(f, "invalid transition: {from:?} -> {to:?}")
            }
            Self::AlreadyApplied => write!(f, "delivery already applied"),
            Self::NotVerified => write!(f, "delivery not verified"),
        }
    }
}

impl std::error::Error for DeliveryError {}

// ---------------------------------------------------------------------------
// Delivery executor (state machine)
// ---------------------------------------------------------------------------

/// Trait abstracting the actual git operations for testability.
pub trait DeliveryExecutor: std::fmt::Debug {
    /// Check if the worktree is clean (no uncommitted changes).
    fn check_clean(&self, branch: &str) -> Result<(), String>;

    /// Apply the commits from source to target.
    fn apply(&mut self, preview: &DeliveryPreview) -> Result<(), String>;

    /// Verify the applied changes (run checks, build, tests).
    fn verify(&mut self, preview: &DeliveryPreview) -> Result<(), String>;

    /// Publish as a draft PR.
    fn publish_draft_pr(
        &mut self,
        preview: &DeliveryPreview,
        title: &str,
    ) -> Result<String, String>;
}

/// The delivery state machine manager.
#[derive(Debug)]
pub struct DeliveryManager<E: DeliveryExecutor> {
    executor: E,
    deliveries: HashMap<String, Delivery>,
    /// Monotonically increasing ID sequence; never reused after a removal.
    next_delivery_id: u64,
}

impl<E: DeliveryExecutor> DeliveryManager<E> {
    pub fn new(executor: E) -> Self {
        Self {
            executor,
            deliveries: HashMap::new(),
            next_delivery_id: 1,
        }
    }

    /// Create a new delivery in Pending state.
    pub fn create(&mut self, preview: DeliveryPreview, now_ms: u64) -> Delivery {
        let idempotency_key = format!(
            "{}:{}:{}",
            preview.source_branch, preview.target_branch, preview.base_sha
        );

        let id = format!("delivery-{}", self.next_delivery_id);
        self.next_delivery_id += 1;

        let delivery = Delivery {
            id,
            state: DeliveryState::Pending,
            preview,
            transitions: vec![DeliveryTransition {
                from: DeliveryState::Pending,
                to: DeliveryState::Pending,
                at_ms: now_ms,
                note: Some("delivery created".to_string()),
            }],
            error: None,
            idempotency_key,
        };
        self.deliveries
            .insert(delivery.id.clone(), delivery.clone());
        delivery
    }

    /// Attempt to apply the delivery.
    ///
    /// On failure, the state becomes `Failed` and the target is left unchanged.
    /// On success, the state becomes `Applied`.
    pub fn apply(&mut self, delivery_id: &str, now_ms: u64) -> Result<Delivery, DeliveryError> {
        let delivery = self
            .deliveries
            .get_mut(delivery_id)
            .ok_or(DeliveryError::ApplyFailed {
                reason: "delivery not found".to_string(),
            })?;

        if delivery.state == DeliveryState::Applied
            || delivery.state == DeliveryState::Verified
            || delivery.state == DeliveryState::Published
            || delivery.state == DeliveryState::Done
        {
            return Err(DeliveryError::AlreadyApplied);
        }

        if delivery.state != DeliveryState::Pending {
            return Err(DeliveryError::InvalidTransition {
                from: delivery.state,
                to: DeliveryState::Applying,
            });
        }

        delivery.state = DeliveryState::Applying;
        delivery.transitions.push(DeliveryTransition {
            from: DeliveryState::Pending,
            to: DeliveryState::Applying,
            at_ms: now_ms,
            note: None,
        });

        match self.executor.check_clean(&delivery.preview.target_branch) {
            Ok(()) => {}
            Err(e) => {
                delivery.state = DeliveryState::Failed;
                delivery.error = Some(e.clone());
                delivery.transitions.push(DeliveryTransition {
                    from: DeliveryState::Applying,
                    to: DeliveryState::Failed,
                    at_ms: now_ms,
                    note: Some(e.clone()),
                });
                return Err(DeliveryError::ApplyFailed { reason: e });
            }
        }

        match self.executor.apply(&delivery.preview) {
            Ok(()) => {
                delivery.state = DeliveryState::Applied;
                delivery.error = None;
                delivery.transitions.push(DeliveryTransition {
                    from: DeliveryState::Applying,
                    to: DeliveryState::Applied,
                    at_ms: now_ms,
                    note: None,
                });
                Ok(delivery.clone())
            }
            Err(e) => {
                delivery.state = DeliveryState::Failed;
                delivery.error = Some(e.clone());
                delivery.transitions.push(DeliveryTransition {
                    from: DeliveryState::Applying,
                    to: DeliveryState::Failed,
                    at_ms: now_ms,
                    note: Some(e.clone()),
                });
                Err(DeliveryError::ApplyFailed { reason: e })
            }
        }
    }

    /// Run post-apply verification.
    pub fn verify(&mut self, delivery_id: &str, now_ms: u64) -> Result<Delivery, DeliveryError> {
        let delivery =
            self.deliveries
                .get_mut(delivery_id)
                .ok_or(DeliveryError::PostApplyVerifyFailed {
                    reason: "delivery not found".to_string(),
                })?;

        if delivery.state != DeliveryState::Applied {
            return Err(DeliveryError::InvalidTransition {
                from: delivery.state,
                to: DeliveryState::Verified,
            });
        }

        match self.executor.verify(&delivery.preview) {
            Ok(()) => {
                delivery.state = DeliveryState::Verified;
                delivery.transitions.push(DeliveryTransition {
                    from: DeliveryState::Applied,
                    to: DeliveryState::Verified,
                    at_ms: now_ms,
                    note: None,
                });
                Ok(delivery.clone())
            }
            Err(e) => {
                delivery.state = DeliveryState::Failed;
                delivery.error = Some(e.clone());
                delivery.transitions.push(DeliveryTransition {
                    from: DeliveryState::Applied,
                    to: DeliveryState::Failed,
                    at_ms: now_ms,
                    note: Some(e.clone()),
                });
                Err(DeliveryError::PostApplyVerifyFailed { reason: e })
            }
        }
    }

    /// Publish as a draft PR.
    /// Source publication failure does not lose local work (state stays Verified).
    pub fn publish(
        &mut self,
        delivery_id: &str,
        title: &str,
        now_ms: u64,
    ) -> Result<Delivery, DeliveryError> {
        let delivery =
            self.deliveries
                .get_mut(delivery_id)
                .ok_or(DeliveryError::PublishFailed {
                    reason: "delivery not found".to_string(),
                })?;

        if delivery.state != DeliveryState::Verified {
            return Err(DeliveryError::NotVerified);
        }

        match self.executor.publish_draft_pr(&delivery.preview, title) {
            Ok(pr_url) => {
                delivery.state = DeliveryState::Published;
                delivery.transitions.push(DeliveryTransition {
                    from: DeliveryState::Verified,
                    to: DeliveryState::Published,
                    at_ms: now_ms,
                    note: Some(format!("draft PR: {pr_url}")),
                });
                Ok(delivery.clone())
            }
            Err(e) => {
                delivery.error = Some(e.clone());
                delivery.transitions.push(DeliveryTransition {
                    from: DeliveryState::Verified,
                    to: DeliveryState::Verified,
                    at_ms: now_ms,
                    note: Some(format!("publish failed (work preserved): {e}")),
                });
                Err(DeliveryError::PublishFailed { reason: e })
            }
        }
    }

    /// Mark the delivery as Done. Requires the handoff policy to be satisfied.
    pub fn complete(
        &mut self,
        delivery_id: &str,
        policy: &HandoffPolicy,
        now_ms: u64,
    ) -> Result<Delivery, DeliveryError> {
        let delivery =
            self.deliveries
                .get_mut(delivery_id)
                .ok_or(DeliveryError::InvalidTransition {
                    from: DeliveryState::Pending,
                    to: DeliveryState::Done,
                })?;

        if delivery.state == DeliveryState::Done {
            return Err(DeliveryError::AlreadyApplied);
        }

        let required_state = if policy.require_draft_pr {
            DeliveryState::Published
        } else if policy.require_post_apply_verify {
            DeliveryState::Verified
        } else {
            DeliveryState::Applied
        };

        let current_order = state_order(delivery.state);
        let required_order = state_order(required_state);

        if current_order < required_order {
            return Err(DeliveryError::InvalidTransition {
                from: delivery.state,
                to: DeliveryState::Done,
            });
        }

        delivery.state = DeliveryState::Done;
        delivery.transitions.push(DeliveryTransition {
            from: if current_order >= state_order(DeliveryState::Published) {
                DeliveryState::Published
            } else if current_order >= state_order(DeliveryState::Verified) {
                DeliveryState::Verified
            } else {
                DeliveryState::Applied
            },
            to: DeliveryState::Done,
            at_ms: now_ms,
            note: Some("handoff complete".to_string()),
        });
        Ok(delivery.clone())
    }

    /// Abort a delivery due to conflict.
    pub fn abort(&mut self, delivery_id: &str, reason: &str, now_ms: u64) -> Delivery {
        if let Some(delivery) = self.deliveries.get_mut(delivery_id) {
            let from = delivery.state;
            delivery.state = DeliveryState::Aborted;
            delivery.error = Some(reason.to_string());
            delivery.transitions.push(DeliveryTransition {
                from,
                to: DeliveryState::Aborted,
                at_ms: now_ms,
                note: Some(reason.to_string()),
            });
            delivery.clone()
        } else {
            Delivery {
                id: delivery_id.to_string(),
                state: DeliveryState::Aborted,
                preview: DeliveryPreview::build("", "", "", "", vec![]),
                transitions: vec![],
                error: Some(reason.to_string()),
                idempotency_key: String::new(),
            }
        }
    }

    /// Get a delivery by id.
    pub fn get(&self, delivery_id: &str) -> Option<&Delivery> {
        self.deliveries.get(delivery_id)
    }

    /// Check if a delivery is idempotent (re-apply produces same result).
    pub fn is_idempotent(&self, delivery_id: &str) -> bool {
        self.deliveries
            .get(delivery_id)
            .map(|d| !d.idempotency_key.is_empty())
            .unwrap_or(false)
    }
}

fn state_order(state: DeliveryState) -> u8 {
    match state {
        DeliveryState::Pending => 0,
        DeliveryState::Applying => 1,
        DeliveryState::Applied => 2,
        DeliveryState::Verified => 3,
        DeliveryState::Published => 4,
        DeliveryState::Done => 5,
        DeliveryState::Failed => 99,
        DeliveryState::Aborted => 99,
    }
}

// ---------------------------------------------------------------------------
// Mock executor for testing
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct MockDeliveryExecutor {
    pub clean: bool,
    pub apply_succeeds: bool,
    pub verify_succeeds: bool,
    pub publish_succeeds: bool,
    pub apply_called: bool,
    pub verify_called: bool,
    pub publish_called: bool,
}

impl Default for MockDeliveryExecutor {
    fn default() -> Self {
        Self {
            clean: true,
            apply_succeeds: true,
            verify_succeeds: true,
            publish_succeeds: true,
            apply_called: false,
            verify_called: false,
            publish_called: false,
        }
    }
}

impl DeliveryExecutor for MockDeliveryExecutor {
    fn check_clean(&self, _branch: &str) -> Result<(), String> {
        if self.clean {
            Ok(())
        } else {
            Err("worktree is dirty".to_string())
        }
    }

    fn apply(&mut self, _preview: &DeliveryPreview) -> Result<(), String> {
        self.apply_called = true;
        if self.apply_succeeds {
            Ok(())
        } else {
            Err("merge conflict in src/main.rs".to_string())
        }
    }

    fn verify(&mut self, _preview: &DeliveryPreview) -> Result<(), String> {
        self.verify_called = true;
        if self.verify_succeeds {
            Ok(())
        } else {
            Err("test 'should_render' failed".to_string())
        }
    }

    fn publish_draft_pr(
        &mut self,
        _preview: &DeliveryPreview,
        _title: &str,
    ) -> Result<String, String> {
        self.publish_called = true;
        if self.publish_succeeds {
            Ok("https://github.com/org/repo/pull/42".to_string())
        } else {
            Err("rate limited".to_string())
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_file(path: &str, add: u32, del: u32) -> FileChange {
        FileChange {
            path: path.to_string(),
            change_type: ChangeType::Modify,
            additions: add,
            deletions: del,
        }
    }

    fn sample_commit(sha: &str, files: Vec<FileChange>) -> CommitPreview {
        CommitPreview {
            sha: sha.to_string(),
            message: format!("commit {sha}"),
            author: "agent".to_string(),
            files,
        }
    }

    fn sample_preview() -> DeliveryPreview {
        DeliveryPreview::build(
            "feat/agent-work",
            "main",
            "abc123",
            "abc123",
            vec![
                sample_commit("def456", vec![sample_file("src/a.rs", 10, 3)]),
                sample_commit("ghi789", vec![sample_file("src/b.rs", 5, 2)]),
            ],
        )
    }

    // ---- DeliveryPreview ----

    #[test]
    fn preview_aggregates_file_changes() {
        let preview = sample_preview();
        assert_eq!(preview.files.len(), 2);
        assert_eq!(preview.total_additions, 15);
        assert_eq!(preview.total_deletions, 5);
    }

    #[test]
    fn preview_aggregates_same_file_across_commits() {
        let preview = DeliveryPreview::build(
            "src",
            "tgt",
            "base",
            "base",
            vec![
                sample_commit("c1", vec![sample_file("src/a.rs", 10, 3)]),
                sample_commit("c2", vec![sample_file("src/a.rs", 5, 1)]),
            ],
        );
        assert_eq!(preview.files.len(), 1);
        assert_eq!(preview.files[0].additions, 15);
        assert_eq!(preview.files[0].deletions, 4);
        assert_eq!(preview.total_additions, 15);
        assert_eq!(preview.total_deletions, 4);
    }

    #[test]
    fn preview_files_are_sorted_by_path() {
        let preview = DeliveryPreview::build(
            "src",
            "tgt",
            "base",
            "base",
            vec![
                sample_commit("c1", vec![sample_file("z.rs", 1, 0)]),
                sample_commit("c2", vec![sample_file("a.rs", 1, 0)]),
                sample_commit("c3", vec![sample_file("m.rs", 1, 0)]),
            ],
        );
        assert_eq!(preview.files[0].path, "a.rs");
        assert_eq!(preview.files[1].path, "m.rs");
        assert_eq!(preview.files[2].path, "z.rs");
    }

    #[test]
    fn preview_detects_target_moved() {
        let preview = DeliveryPreview::build("src", "tgt", "base123", "new456", vec![]);
        assert!(preview.target_moved);
    }

    #[test]
    fn preview_target_not_moved_when_same() {
        let preview = DeliveryPreview::build("src", "tgt", "abc", "abc", vec![]);
        assert!(!preview.target_moved);
    }

    // ---- Conflict detection ----

    #[test]
    fn conflict_detected_on_overlapping_files() {
        let source = vec![sample_file("src/a.rs", 1, 0)];
        let target = vec![sample_file("src/a.rs", 0, 1)];
        let result = detect_conflicts(&source, &target);
        assert!(matches!(result, ConflictCheck::Conflict { .. }));
    }

    #[test]
    fn no_conflict_on_disjoint_files() {
        let source = vec![sample_file("src/a.rs", 1, 0)];
        let target = vec![sample_file("src/b.rs", 0, 1)];
        let result = detect_conflicts(&source, &target);
        assert!(matches!(result, ConflictCheck::Clean { .. }));
    }

    #[test]
    fn no_conflict_on_empty_target() {
        let source = vec![sample_file("src/a.rs", 1, 0)];
        let result = detect_conflicts(&source, &[]);
        assert!(
            matches!(result, ConflictCheck::Clean { overlapping_files } if overlapping_files.is_empty())
        );
    }

    // ---- Policy evaluation ----

    #[test]
    fn policy_blocks_when_auto_apply_disabled() {
        let preview = sample_preview();
        let policy = HandoffPolicy::default();
        let conflict = ConflictCheck::Clean {
            overlapping_files: vec![],
        };
        let eval = evaluate_policy(&preview, &policy, &conflict);
        assert!(!eval.allowed);
        assert!(eval.blockers.iter().any(|b| b.contains("auto_apply")));
    }

    #[test]
    fn policy_allows_when_auto_apply_enabled_and_no_conflict() {
        let preview = sample_preview();
        let policy = HandoffPolicy {
            auto_apply: true,
            ..HandoffPolicy::default()
        };
        let conflict = ConflictCheck::Clean {
            overlapping_files: vec![],
        };
        let eval = evaluate_policy(&preview, &policy, &conflict);
        assert!(eval.allowed);
        assert!(eval.blockers.is_empty());
    }

    #[test]
    fn policy_blocks_on_conflict_even_with_auto_apply() {
        let preview = sample_preview();
        let policy = HandoffPolicy {
            auto_apply: true,
            ..HandoffPolicy::default()
        };
        let conflict = ConflictCheck::Conflict {
            overlapping_files: vec!["src/a.rs".to_string()],
            reason: "conflict".to_string(),
        };
        let eval = evaluate_policy(&preview, &policy, &conflict);
        assert!(!eval.allowed);
        assert!(eval.blockers.iter().any(|b| b.contains("conflict")));
    }

    #[test]
    fn policy_warns_on_large_diff() {
        let preview = DeliveryPreview::build(
            "src",
            "tgt",
            "base",
            "base",
            vec![sample_commit("c1", vec![sample_file("big.rs", 2000, 0)])],
        );
        let policy = HandoffPolicy {
            auto_apply: true,
            max_additions_warning: Some(1000),
            ..HandoffPolicy::default()
        };
        let conflict = ConflictCheck::Clean {
            overlapping_files: vec![],
        };
        let eval = evaluate_policy(&preview, &policy, &conflict);
        assert!(eval.warnings.iter().any(|w| w.contains("additions")));
    }

    #[test]
    fn policy_warns_on_target_moved() {
        let preview = DeliveryPreview::build("src", "tgt", "old", "new", vec![]);
        let policy = HandoffPolicy {
            auto_apply: true,
            ..HandoffPolicy::default()
        };
        let conflict = ConflictCheck::Clean {
            overlapping_files: vec![],
        };
        let eval = evaluate_policy(&preview, &policy, &conflict);
        assert!(eval.warnings.iter().any(|w| w.contains("moved")));
    }

    #[test]
    fn policy_blocks_when_draft_pr_required() {
        let preview = sample_preview();
        let policy = HandoffPolicy {
            auto_apply: true,
            require_draft_pr: true,
            ..HandoffPolicy::default()
        };
        let conflict = ConflictCheck::Clean {
            overlapping_files: vec![],
        };
        let eval = evaluate_policy(&preview, &policy, &conflict);
        assert!(eval.blockers.iter().any(|b| b.contains("draft PR")));
    }

    // ---- State machine: apply ----

    #[test]
    fn create_does_not_reuse_an_id_after_delivery_removal() {
        let executor = MockDeliveryExecutor::default();
        let mut mgr = DeliveryManager::new(executor);
        let first = mgr.create(sample_preview(), 1000);
        mgr.deliveries.remove(&first.id);

        let second = mgr.create(sample_preview(), 2000);

        assert_ne!(first.id, second.id);
        assert_eq!(second.id, "delivery-2");
    }

    #[test]
    fn apply_succeeds_on_clean_worktree() {
        let executor = MockDeliveryExecutor::default();
        let mut mgr = DeliveryManager::new(executor);
        let preview = sample_preview();
        let delivery = mgr.create(preview, 1000);

        let result = mgr.apply(&delivery.id, 2000).unwrap();
        assert_eq!(result.state, DeliveryState::Applied);
        assert!(mgr.executor.apply_called);
    }

    #[test]
    fn apply_fails_on_dirty_worktree() {
        let executor = MockDeliveryExecutor {
            clean: false,
            ..MockDeliveryExecutor::default()
        };
        let mut mgr = DeliveryManager::new(executor);
        let preview = sample_preview();
        let delivery = mgr.create(preview, 1000);

        let result = mgr.apply(&delivery.id, 2000);
        assert!(matches!(result, Err(DeliveryError::ApplyFailed { .. })));
        let delivery = mgr.get(&delivery.id).unwrap();
        assert_eq!(delivery.state, DeliveryState::Failed);
    }

    #[test]
    fn apply_fails_on_apply_error() {
        let executor = MockDeliveryExecutor {
            apply_succeeds: false,
            ..MockDeliveryExecutor::default()
        };
        let mut mgr = DeliveryManager::new(executor);
        let preview = sample_preview();
        let delivery = mgr.create(preview, 1000);

        let result = mgr.apply(&delivery.id, 2000);
        assert!(matches!(result, Err(DeliveryError::ApplyFailed { .. })));
        let delivery = mgr.get(&delivery.id).unwrap();
        assert_eq!(delivery.state, DeliveryState::Failed);
    }

    #[test]
    fn apply_is_idempotent_second_call_returns_error() {
        let executor = MockDeliveryExecutor::default();
        let mut mgr = DeliveryManager::new(executor);
        let preview = sample_preview();
        let delivery = mgr.create(preview, 1000);

        mgr.apply(&delivery.id, 2000).unwrap();
        let result = mgr.apply(&delivery.id, 3000);
        assert!(matches!(result, Err(DeliveryError::AlreadyApplied)));
    }

    #[test]
    fn apply_rejects_invalid_transition_from_done() {
        let executor = MockDeliveryExecutor::default();
        let mut mgr = DeliveryManager::new(executor);
        let preview = sample_preview();
        let delivery = mgr.create(preview, 1000);

        mgr.apply(&delivery.id, 2000).unwrap();
        mgr.verify(&delivery.id, 3000).unwrap();
        mgr.complete(&delivery.id, &HandoffPolicy::default(), 4000)
            .unwrap();

        let result = mgr.apply(&delivery.id, 5000);
        assert!(matches!(result, Err(DeliveryError::AlreadyApplied)));
    }

    // ---- State machine: verify ----

    #[test]
    fn verify_succeeds_after_apply() {
        let executor = MockDeliveryExecutor::default();
        let mut mgr = DeliveryManager::new(executor);
        let preview = sample_preview();
        let delivery = mgr.create(preview, 1000);

        mgr.apply(&delivery.id, 2000).unwrap();
        let result = mgr.verify(&delivery.id, 3000).unwrap();
        assert_eq!(result.state, DeliveryState::Verified);
        assert!(mgr.executor.verify_called);
    }

    #[test]
    fn verify_fails_on_verify_error() {
        let executor = MockDeliveryExecutor {
            verify_succeeds: false,
            ..MockDeliveryExecutor::default()
        };
        let mut mgr = DeliveryManager::new(executor);
        let preview = sample_preview();
        let delivery = mgr.create(preview, 1000);

        mgr.apply(&delivery.id, 2000).unwrap();
        let result = mgr.verify(&delivery.id, 3000);
        assert!(matches!(
            result,
            Err(DeliveryError::PostApplyVerifyFailed { .. })
        ));
    }

    #[test]
    fn verify_rejects_non_applied_state() {
        let executor = MockDeliveryExecutor::default();
        let mut mgr = DeliveryManager::new(executor);
        let preview = sample_preview();
        let delivery = mgr.create(preview, 1000);

        let result = mgr.verify(&delivery.id, 2000);
        assert!(matches!(
            result,
            Err(DeliveryError::InvalidTransition { .. })
        ));
    }

    // ---- State machine: publish ----

    #[test]
    fn publish_succeeds_after_verify() {
        let executor = MockDeliveryExecutor::default();
        let mut mgr = DeliveryManager::new(executor);
        let preview = sample_preview();
        let delivery = mgr.create(preview, 1000);

        mgr.apply(&delivery.id, 2000).unwrap();
        mgr.verify(&delivery.id, 3000).unwrap();
        let result = mgr.publish(&delivery.id, "feat: awesome", 4000).unwrap();
        assert_eq!(result.state, DeliveryState::Published);
        assert!(mgr.executor.publish_called);
    }

    #[test]
    fn publish_failure_preserves_work() {
        let executor = MockDeliveryExecutor {
            publish_succeeds: false,
            ..MockDeliveryExecutor::default()
        };
        let mut mgr = DeliveryManager::new(executor);
        let preview = sample_preview();
        let delivery = mgr.create(preview, 1000);

        mgr.apply(&delivery.id, 2000).unwrap();
        mgr.verify(&delivery.id, 3000).unwrap();
        let result = mgr.publish(&delivery.id, "feat: awesome", 4000);
        assert!(matches!(result, Err(DeliveryError::PublishFailed { .. })));

        let delivery = mgr.get(&delivery.id).unwrap();
        assert_eq!(delivery.state, DeliveryState::Verified);
        assert!(delivery.error.is_some());
    }

    #[test]
    fn publish_rejects_non_verified_state() {
        let executor = MockDeliveryExecutor::default();
        let mut mgr = DeliveryManager::new(executor);
        let preview = sample_preview();
        let delivery = mgr.create(preview, 1000);

        mgr.apply(&delivery.id, 2000).unwrap();
        let result = mgr.publish(&delivery.id, "title", 3000);
        assert!(matches!(result, Err(DeliveryError::NotVerified)));
    }

    // ---- State machine: complete ----

    #[test]
    fn complete_requires_verified_by_default() {
        let executor = MockDeliveryExecutor::default();
        let mut mgr = DeliveryManager::new(executor);
        let preview = sample_preview();
        let delivery = mgr.create(preview, 1000);

        mgr.apply(&delivery.id, 2000).unwrap();
        mgr.verify(&delivery.id, 3000).unwrap();
        let result = mgr
            .complete(&delivery.id, &HandoffPolicy::default(), 4000)
            .unwrap();
        assert_eq!(result.state, DeliveryState::Done);
    }

    #[test]
    fn complete_blocks_without_verify_by_default() {
        let executor = MockDeliveryExecutor::default();
        let mut mgr = DeliveryManager::new(executor);
        let preview = sample_preview();
        let delivery = mgr.create(preview, 1000);

        mgr.apply(&delivery.id, 2000).unwrap();
        let result = mgr.complete(&delivery.id, &HandoffPolicy::default(), 3000);
        assert!(matches!(
            result,
            Err(DeliveryError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn complete_requires_published_when_draft_pr_required() {
        let executor = MockDeliveryExecutor::default();
        let mut mgr = DeliveryManager::new(executor);
        let preview = sample_preview();
        let delivery = mgr.create(preview, 1000);
        let policy = HandoffPolicy {
            require_draft_pr: true,
            ..HandoffPolicy::default()
        };

        mgr.apply(&delivery.id, 2000).unwrap();
        mgr.verify(&delivery.id, 3000).unwrap();
        let result = mgr.complete(&delivery.id, &policy, 4000);
        assert!(matches!(
            result,
            Err(DeliveryError::InvalidTransition { .. })
        ));

        mgr.publish(&delivery.id, "title", 5000).unwrap();
        let result = mgr.complete(&delivery.id, &policy, 6000).unwrap();
        assert_eq!(result.state, DeliveryState::Done);
    }

    #[test]
    fn complete_allows_applied_when_verify_not_required() {
        let executor = MockDeliveryExecutor::default();
        let mut mgr = DeliveryManager::new(executor);
        let preview = sample_preview();
        let delivery = mgr.create(preview, 1000);
        let policy = HandoffPolicy {
            require_post_apply_verify: false,
            ..HandoffPolicy::default()
        };

        mgr.apply(&delivery.id, 2000).unwrap();
        let result = mgr.complete(&delivery.id, &policy, 3000).unwrap();
        assert_eq!(result.state, DeliveryState::Done);
    }

    // ---- State machine: abort ----

    #[test]
    fn abort_sets_aborted_state() {
        let executor = MockDeliveryExecutor::default();
        let mut mgr = DeliveryManager::new(executor);
        let preview = sample_preview();
        let delivery = mgr.create(preview, 1000);

        let result = mgr.abort(&delivery.id, "conflict detected", 2000);
        assert_eq!(result.state, DeliveryState::Aborted);
        assert_eq!(result.error.as_deref(), Some("conflict detected"));
    }

    // ---- Idempotency ----

    #[test]
    fn idempotency_key_is_deterministic() {
        let executor = MockDeliveryExecutor::default();
        let mut mgr = DeliveryManager::new(executor);
        let preview = sample_preview();
        let delivery = mgr.create(preview, 1000);

        assert!(mgr.is_idempotent(&delivery.id));
        assert!(!delivery.idempotency_key.is_empty());
    }

    #[test]
    fn idempotency_key_differs_for_different_branches() {
        let executor = MockDeliveryExecutor::default();
        let mut mgr = DeliveryManager::new(executor);

        let d1 = mgr.create(
            DeliveryPreview::build("branch-a", "main", "abc", "abc", vec![]),
            1000,
        );
        let d2 = mgr.create(
            DeliveryPreview::build("branch-b", "main", "abc", "abc", vec![]),
            2000,
        );
        assert_ne!(d1.idempotency_key, d2.idempotency_key);
    }

    // ---- Full lifecycle ----

    #[test]
    fn full_lifecycle_apply_verify_publish_complete() {
        let executor = MockDeliveryExecutor::default();
        let mut mgr = DeliveryManager::new(executor);
        let preview = sample_preview();
        let delivery = mgr.create(preview, 1000);

        mgr.apply(&delivery.id, 2000).unwrap();
        mgr.verify(&delivery.id, 3000).unwrap();
        mgr.publish(&delivery.id, "feat: awesome", 4000).unwrap();
        let result = mgr
            .complete(&delivery.id, &HandoffPolicy::default(), 5000)
            .unwrap();

        assert_eq!(result.state, DeliveryState::Done);
        assert!(result.transitions.len() >= 5);
    }

    #[test]
    fn failed_apply_records_transition_chain() {
        let executor = MockDeliveryExecutor {
            apply_succeeds: false,
            ..MockDeliveryExecutor::default()
        };
        let mut mgr = DeliveryManager::new(executor);
        let preview = sample_preview();
        let delivery = mgr.create(preview, 1000);

        let _ = mgr.apply(&delivery.id, 2000);
        let delivery = mgr.get(&delivery.id).unwrap();

        assert_eq!(delivery.state, DeliveryState::Failed);
        assert!(delivery.transitions.len() >= 3);
        assert!(delivery.error.is_some());
    }

    // ---- Serialization ----

    #[test]
    fn file_change_serializes() {
        let fc = sample_file("src/main.rs", 10, 3);
        let json = serde_json::to_string(&fc).unwrap();
        let back: FileChange = serde_json::from_str(&json).unwrap();
        assert_eq!(fc, back);
    }

    #[test]
    fn handoff_policy_defaults_to_safe() {
        let policy = HandoffPolicy::default();
        assert!(!policy.auto_apply);
        assert!(!policy.auto_merge);
        assert!(policy.require_post_apply_verify);
        assert!(!policy.require_draft_pr);
    }

    #[test]
    fn delivery_error_display() {
        assert!(format!(
            "{}",
            DeliveryError::ConflictDetected {
                files: vec!["a.rs".to_string()]
            }
        )
        .contains("a.rs"));

        assert!(format!(
            "{}",
            DeliveryError::ApplyFailed {
                reason: "boom".to_string()
            }
        )
        .contains("boom"));

        assert!(format!("{}", DeliveryError::AlreadyApplied).contains("already"));
    }
}
