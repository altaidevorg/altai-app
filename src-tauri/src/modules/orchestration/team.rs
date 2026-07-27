//! Team coordinator and mailbox (plan §F4).
//!
//! Parent/child task relationships, bounded agent-to-coordinator messages
//! with exactly-once delivery, shared task-list claim semantics, file
//! ownership conflict detection, and explicit approval before fan-out.

use std::collections::{HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ---------------------------------------------------------------------------
// Task hierarchy (parent/child)
// ---------------------------------------------------------------------------

/// Tracks parent/child relationships between tasks.
#[derive(Clone, Debug, Default)]
pub struct TaskHierarchy {
    /// child_task_id → parent_task_id
    parent_of: HashMap<String, String>,
    /// parent_task_id → ordered list of child_task_ids
    children_of: HashMap<String, Vec<String>>,
}

impl TaskHierarchy {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a parent→child relationship.
    /// Returns an error if the child already has a different parent.
    pub fn add_child(&mut self, parent_id: &str, child_id: &str) -> Result<(), HierarchyError> {
        if parent_id == child_id {
            return Err(HierarchyError::WouldCreateCycle {
                child_id: child_id.into(),
                parent_id: parent_id.into(),
            });
        }
        if let Some(existing) = self.parent_of.get(child_id) {
            if existing != parent_id {
                return Err(HierarchyError::AlreadyHasParent {
                    child_id: child_id.into(),
                    existing_parent: existing.clone(),
                    requested_parent: parent_id.into(),
                });
            }
            // Same parent — idempotent.
            return Ok(());
        }
        // Prevent cycles: adding parent→child is a cycle if the parent is
        // already a descendant of the child (child → ... → parent exists).
        if self.is_ancestor_of(child_id, parent_id) {
            return Err(HierarchyError::WouldCreateCycle {
                child_id: child_id.into(),
                parent_id: parent_id.into(),
            });
        }
        self.parent_of
            .insert(child_id.to_string(), parent_id.to_string());
        self.children_of
            .entry(parent_id.to_string())
            .or_default()
            .push(child_id.to_string());
        Ok(())
    }

    /// Get the parent of a task.
    pub fn parent_of(&self, task_id: &str) -> Option<&str> {
        self.parent_of.get(task_id).map(|s| s.as_str())
    }

    /// Get the children of a task.
    pub fn children_of(&self, task_id: &str) -> Vec<String> {
        self.children_of.get(task_id).cloned().unwrap_or_default()
    }

    /// Get all descendants of a task (recursive).
    pub fn descendants_of(&self, task_id: &str) -> HashSet<String> {
        let mut result = HashSet::new();
        let mut queue: VecDeque<String> = self.children_of(task_id).into_iter().collect();
        while let Some(child) = queue.pop_front() {
            if result.insert(child.clone()) {
                queue.extend(self.children_of(&child));
            }
        }
        result
    }

    /// Check if `ancestor` is an ancestor of `descendant` (descendant is in
    /// ancestor's subtree). This detects potential cycles before adding an edge.
    fn is_ancestor_of(&self, ancestor: &str, descendant: &str) -> bool {
        self.descendants_of(ancestor).contains(descendant)
    }

    /// Get the root ancestor of a task.
    pub fn root_of(&self, task_id: &str) -> Option<String> {
        let mut current = task_id.to_string();
        let mut visited = HashSet::new();
        loop {
            if !visited.insert(current.clone()) {
                return None; // cycle (shouldn't happen)
            }
            match self.parent_of.get(&current) {
                Some(parent) => current = parent.clone(),
                None => return Some(current),
            }
        }
    }

    /// Count all tasks in the tree rooted at the given task.
    pub fn tree_size(&self, root_id: &str) -> usize {
        1 + self.descendants_of(root_id).len()
    }
}

/// Error for hierarchy operations.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum HierarchyError {
    AlreadyHasParent {
        child_id: String,
        existing_parent: String,
        requested_parent: String,
    },
    WouldCreateCycle {
        child_id: String,
        parent_id: String,
    },
}

// ---------------------------------------------------------------------------
// Mailbox (bounded agent-to-coordinator messages)
// ---------------------------------------------------------------------------

/// The kind of message an agent sends to the coordinator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageKind {
    /// Child reports a structured result.
    Result,
    /// Progress/status update.
    Status,
    /// User steering applied to a child.
    Steering,
    /// Error report.
    Error,
    /// Request approval for an action.
    ApprovalRequest,
}

/// A single agent-to-coordinator message.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMessage {
    pub id: String,
    pub from_task: String,
    pub to_task: String,
    pub kind: MessageKind,
    pub payload: Value,
    pub timestamp_ms: u64,
}

/// A bounded mailbox with exactly-once delivery at the recipient boundary.
#[derive(Clone, Debug)]
pub struct Mailbox {
    messages: VecDeque<AgentMessage>,
    capacity: usize,
    /// IDs retained in the bounded exactly-once replay window.
    delivered: HashSet<String>,
    /// Delivery order used to evict the oldest replay-protection entry.
    delivered_order: VecDeque<String>,
    dedupe_capacity: usize,
    delivered_total: usize,
}

/// Error when the mailbox is full.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct MailboxFullError {
    pub capacity: usize,
}

impl Mailbox {
    pub fn new(capacity: usize) -> Self {
        Self::with_dedupe_capacity(capacity, capacity.max(1))
    }

    /// Create a mailbox with an explicit bounded replay-protection window.
    pub fn with_dedupe_capacity(capacity: usize, dedupe_capacity: usize) -> Self {
        Self {
            messages: VecDeque::new(),
            capacity,
            delivered: HashSet::new(),
            delivered_order: VecDeque::new(),
            dedupe_capacity: dedupe_capacity.max(1),
            delivered_total: 0,
        }
    }

    /// Post a message. Returns error if mailbox is full or message is a duplicate.
    pub fn post(&mut self, msg: AgentMessage) -> Result<(), MailboxFullError> {
        // Exactly-once within the bounded replay window. Also reject a
        // duplicate that is already pending so it cannot be delivered twice.
        if self.delivered.contains(&msg.id)
            || self.messages.iter().any(|pending| pending.id == msg.id)
        {
            return Ok(());
        }
        if self.messages.len() >= self.capacity {
            return Err(MailboxFullError {
                capacity: self.capacity,
            });
        }
        self.messages.push_back(msg);
        Ok(())
    }

    /// Try to deliver the next message. Returns None if empty.
    /// Marks the message as delivered (exactly-once at recipient boundary).
    pub fn deliver(&mut self) -> Option<AgentMessage> {
        let msg = self.messages.pop_front()?;
        self.remember_delivery(msg.id.clone());
        Some(msg)
    }

    fn remember_delivery(&mut self, message_id: String) {
        self.delivered_total = self.delivered_total.saturating_add(1);
        if !self.delivered.insert(message_id.clone()) {
            return;
        }
        self.delivered_order.push_back(message_id);
        while self.delivered_order.len() > self.dedupe_capacity {
            if let Some(expired) = self.delivered_order.pop_front() {
                self.delivered.remove(&expired);
            }
        }
    }

    /// Deliver all pending messages.
    pub fn drain(&mut self) -> Vec<AgentMessage> {
        let mut result = Vec::new();
        while let Some(msg) = self.deliver() {
            result.push(msg);
        }
        result
    }

    /// Number of pending (undelivered) messages.
    pub fn pending(&self) -> usize {
        self.messages.len()
    }

    /// Total number of messages delivered by this mailbox.
    pub fn delivered_count(&self) -> usize {
        self.delivered_total
    }

    /// Number of delivered IDs retained in the bounded replay window.
    pub fn dedupe_entries(&self) -> usize {
        self.delivered.len()
    }

    /// Whether the mailbox is at capacity.
    pub fn is_full(&self) -> bool {
        self.messages.len() >= self.capacity
    }

    /// Messages for a specific recipient task.
    pub fn pending_for(&self, to_task: &str) -> usize {
        self.messages
            .iter()
            .filter(|m| m.to_task == to_task)
            .count()
    }
}

// ---------------------------------------------------------------------------
// File ownership conflict detection
// ---------------------------------------------------------------------------

/// A task's claim on a set of files.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileOwnership {
    pub task_id: String,
    pub file_globs: Vec<String>,
}

/// A conflict between two tasks over overlapping files.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileConflict {
    pub task_a: String,
    pub task_b: String,
    pub overlapping_globs: Vec<String>,
}

/// Detect file ownership conflicts between tasks.
/// Two tasks conflict if their file globs overlap.
pub fn detect_conflicts(ownerships: &[FileOwnership]) -> Vec<FileConflict> {
    let mut conflicts = Vec::new();
    for i in 0..ownerships.len() {
        for j in (i + 1)..ownerships.len() {
            let a = &ownerships[i];
            let b = &ownerships[j];
            let overlap = find_overlapping_globs(&a.file_globs, &b.file_globs);
            if !overlap.is_empty() {
                conflicts.push(FileConflict {
                    task_a: a.task_id.clone(),
                    task_b: b.task_id.clone(),
                    overlapping_globs: overlap,
                });
            }
        }
    }
    conflicts
}

#[derive(Clone, Copy, Debug)]
enum GlobToken {
    Literal(char),
    AnyNonSeparator,
    Star { crosses_separator: bool },
}

/// Check whether two supported path globs can match at least one common path.
///
/// `*` and `?` stay within one path segment; `**` may cross `/`. Literal
/// directory claims also cover descendants, preserving the ownership shorthand
/// used by this module.
fn glob_overlap(a: &str, b: &str) -> bool {
    let a = normalize_glob(a);
    let b = normalize_glob(b);

    if a.is_empty() || b.is_empty() {
        return false;
    }
    if a == b {
        return true;
    }

    if literal_directory_contains(&a, &b) || literal_directory_contains(&b, &a) {
        return true;
    }

    glob_languages_intersect(&tokenize_glob(&a), &tokenize_glob(&b))
}

fn normalize_glob(pattern: &str) -> String {
    pattern
        .replace('\\', "/")
        .trim_start_matches("./")
        .trim_end_matches('/')
        .to_string()
}

fn literal_directory_contains(directory: &str, candidate: &str) -> bool {
    if directory.is_empty() || directory.contains('*') || directory.contains('?') {
        return false;
    }
    candidate
        .strip_prefix(directory)
        .is_some_and(|suffix| suffix.starts_with('/'))
}

fn tokenize_glob(pattern: &str) -> Vec<GlobToken> {
    let chars: Vec<char> = pattern.chars().collect();
    let mut tokens = Vec::with_capacity(chars.len());
    let mut index = 0;
    while index < chars.len() {
        match chars[index] {
            '*' if chars.get(index + 1) == Some(&'*') => {
                tokens.push(GlobToken::Star {
                    crosses_separator: true,
                });
                index += 2;
            }
            '*' => {
                tokens.push(GlobToken::Star {
                    crosses_separator: false,
                });
                index += 1;
            }
            '?' => {
                tokens.push(GlobToken::AnyNonSeparator);
                index += 1;
            }
            literal => {
                tokens.push(GlobToken::Literal(literal));
                index += 1;
            }
        }
    }
    tokens
}

fn glob_languages_intersect(a: &[GlobToken], b: &[GlobToken]) -> bool {
    let mut pending = VecDeque::from([(0_usize, 0_usize)]);
    let mut visited = HashSet::new();

    while let Some((a_index, b_index)) = pending.pop_front() {
        if !visited.insert((a_index, b_index)) {
            continue;
        }
        if a_index == a.len() && b_index == b.len() {
            return true;
        }

        if matches!(a.get(a_index), Some(GlobToken::Star { .. })) {
            pending.push_back((a_index + 1, b_index));
        }
        if matches!(b.get(b_index), Some(GlobToken::Star { .. })) {
            pending.push_back((a_index, b_index + 1));
        }

        if let (Some(a_token), Some(b_token)) = (a.get(a_index), b.get(b_index)) {
            if tokens_share_character(*a_token, *b_token) {
                pending.push_back((
                    consumed_index(a_index, *a_token),
                    consumed_index(b_index, *b_token),
                ));
            }
        }
    }
    false
}

fn consumed_index(index: usize, token: GlobToken) -> usize {
    match token {
        GlobToken::Star { .. } => index,
        GlobToken::Literal(_) | GlobToken::AnyNonSeparator => index + 1,
    }
}

fn tokens_share_character(a: GlobToken, b: GlobToken) -> bool {
    match (a, b) {
        (GlobToken::Literal(a), GlobToken::Literal(b)) => a == b,
        (GlobToken::Literal(literal), token) | (token, GlobToken::Literal(literal)) => {
            token_accepts(token, literal)
        }
        _ => true,
    }
}

fn token_accepts(token: GlobToken, character: char) -> bool {
    match token {
        GlobToken::Literal(expected) => expected == character,
        GlobToken::AnyNonSeparator => character != '/',
        GlobToken::Star { crosses_separator } => crosses_separator || character != '/',
    }
}

fn find_overlapping_globs(a: &[String], b: &[String]) -> Vec<String> {
    let mut overlap = Vec::new();
    for ga in a {
        for gb in b {
            if glob_overlap(ga, gb) {
                // Record the overlap once (avoid duplicates).
                let label = if ga == gb {
                    ga.clone()
                } else {
                    format!("{ga} ∩ {gb}")
                };
                if !overlap.contains(&label) {
                    overlap.push(label);
                }
            }
        }
    }
    overlap
}

// ---------------------------------------------------------------------------
// Claim semantics (shared task list)
// ---------------------------------------------------------------------------

/// Result of claiming tasks from a shared list.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimResult {
    pub claimed: Vec<String>,
    pub already_claimed: Vec<String>,
}

/// Claims tasks atomically. Tasks already claimed are returned separately.
pub fn claim_tasks(
    claim_state: &mut HashMap<String, String>,
    claimant: &str,
    task_ids: &[String],
) -> ClaimResult {
    let mut claimed = Vec::new();
    let mut already_claimed = Vec::new();
    for task_id in task_ids {
        match claim_state.get(task_id) {
            Some(owner) if owner == claimant => {
                // Already claimed by same claimant — idempotent.
                claimed.push(task_id.clone());
            }
            Some(_) => {
                already_claimed.push(task_id.clone());
            }
            None => {
                claim_state.insert(task_id.clone(), claimant.to_string());
                claimed.push(task_id.clone());
            }
        }
    }
    ClaimResult {
        claimed,
        already_claimed,
    }
}

/// Release a task claim.
pub fn release_claim(claim_state: &mut HashMap<String, String>, task_id: &str) -> bool {
    claim_state.remove(task_id).is_some()
}

/// Transfer a claim from one claimant to another (for failover).
pub fn transfer_claim(
    claim_state: &mut HashMap<String, String>,
    task_id: &str,
    from: &str,
    to: &str,
) -> bool {
    if claim_state.get(task_id) == Some(&from.to_string()) {
        claim_state.insert(task_id.to_string(), to.to_string());
        true
    } else {
        false
    }
}

// ---------------------------------------------------------------------------
// Fan-out approval
// ---------------------------------------------------------------------------

/// Whether a fan-out requires explicit approval.
pub fn requires_approval(child_count: usize, config: &FanOutConfig) -> bool {
    child_count >= config.approval_threshold
}

/// Configuration for fan-out approval.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FanOutConfig {
    /// Number of children that triggers approval requirement.
    pub approval_threshold: usize,
    /// Maximum children allowed without approval.
    pub max_without_approval: usize,
}

impl Default for FanOutConfig {
    fn default() -> Self {
        Self {
            approval_threshold: 3,
            max_without_approval: 5,
        }
    }
}

/// Result of evaluating a fan-out request.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FanOutDecision {
    Allowed { child_count: usize },
    NeedsApproval { child_count: usize, reason: String },
    Denied { child_count: usize, reason: String },
}

/// Evaluate whether a fan-out is allowed.
pub fn evaluate_fan_out(child_count: usize, config: &FanOutConfig) -> FanOutDecision {
    if child_count == 0 {
        return FanOutDecision::Allowed { child_count: 0 };
    }
    if child_count > config.max_without_approval {
        return FanOutDecision::Denied {
            child_count,
            reason: format!(
                "Fan-out of {child_count} exceeds maximum without approval ({})",
                config.max_without_approval
            ),
        };
    }
    if requires_approval(child_count, config) {
        return FanOutDecision::NeedsApproval {
            child_count,
            reason: format!(
                "Fan-out of {child_count} requires explicit approval (threshold: {})",
                config.approval_threshold
            ),
        };
    }
    FanOutDecision::Allowed { child_count }
}

// ---------------------------------------------------------------------------
// Worktree isolation check
// ---------------------------------------------------------------------------

/// Verify that parallel agents never share a worktree.
/// Returns conflicts if two tasks map to the same worktree path.
pub fn check_worktree_isolation(task_worktrees: &HashMap<String, String>) -> Vec<WorktreeConflict> {
    let mut path_to_tasks: HashMap<&str, Vec<&str>> = HashMap::new();
    for (task_id, path) in task_worktrees {
        path_to_tasks
            .entry(path.as_str())
            .or_default()
            .push(task_id.as_str());
    }
    path_to_tasks
        .into_iter()
        .filter(|(_, tasks)| tasks.len() > 1)
        .map(|(path, tasks)| WorktreeConflict {
            worktree_path: path.to_string(),
            task_ids: tasks.iter().map(|s| s.to_string()).collect(),
        })
        .collect()
}

/// A worktree isolation violation.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeConflict {
    pub worktree_path: String,
    pub task_ids: Vec<String>,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ---- task hierarchy ----

    #[test]
    fn add_child_basic() {
        let mut h = TaskHierarchy::new();
        h.add_child("parent", "child1").unwrap();
        h.add_child("parent", "child2").unwrap();
        assert_eq!(h.children_of("parent"), vec!["child1", "child2"]);
        assert_eq!(h.parent_of("child1"), Some("parent"));
    }

    #[test]
    fn child_cannot_have_two_parents() {
        let mut h = TaskHierarchy::new();
        h.add_child("p1", "c1").unwrap();
        let err = h.add_child("p2", "c1").unwrap_err();
        assert!(matches!(err, HierarchyError::AlreadyHasParent { .. }));
    }

    #[test]
    fn same_parent_is_idempotent() {
        let mut h = TaskHierarchy::new();
        h.add_child("p1", "c1").unwrap();
        h.add_child("p1", "c1").unwrap(); // no error
        assert_eq!(h.children_of("p1").len(), 1);
    }

    #[test]
    fn cycle_prevented() {
        let mut h = TaskHierarchy::new();
        h.add_child("A", "B").unwrap();
        h.add_child("B", "C").unwrap();
        // C → A would create a cycle.
        let err = h.add_child("C", "A").unwrap_err();
        assert!(matches!(err, HierarchyError::WouldCreateCycle { .. }));
    }

    #[test]
    fn task_cannot_be_its_own_parent() {
        let mut h = TaskHierarchy::new();
        let err = h.add_child("A", "A").unwrap_err();
        assert!(matches!(err, HierarchyError::WouldCreateCycle { .. }));
        assert!(h.parent_of("A").is_none());
    }

    #[test]
    fn descendants_recursive() {
        let mut h = TaskHierarchy::new();
        h.add_child("A", "B").unwrap();
        h.add_child("B", "C").unwrap();
        h.add_child("B", "D").unwrap();
        let desc = h.descendants_of("A");
        assert!(desc.contains("B"));
        assert!(desc.contains("C"));
        assert!(desc.contains("D"));
        assert_eq!(desc.len(), 3);
    }

    #[test]
    fn root_of_finds_topmost() {
        let mut h = TaskHierarchy::new();
        h.add_child("A", "B").unwrap();
        h.add_child("B", "C").unwrap();
        assert_eq!(h.root_of("C"), Some("A".into()));
        assert_eq!(h.root_of("A"), Some("A".into()));
    }

    #[test]
    fn tree_size_counts_all() {
        let mut h = TaskHierarchy::new();
        h.add_child("A", "B").unwrap();
        h.add_child("A", "C").unwrap();
        h.add_child("C", "D").unwrap();
        assert_eq!(h.tree_size("A"), 4); // A + B + C + D
    }

    // ---- mailbox ----

    fn make_msg(id: &str, from: &str, to: &str) -> AgentMessage {
        AgentMessage {
            id: id.into(),
            from_task: from.into(),
            to_task: to.into(),
            kind: MessageKind::Status,
            payload: json!({}),
            timestamp_ms: 1000,
        }
    }

    #[test]
    fn post_and_deliver() {
        let mut mb = Mailbox::new(10);
        mb.post(make_msg("m1", "child", "parent")).unwrap();
        assert_eq!(mb.pending(), 1);

        let msg = mb.deliver().unwrap();
        assert_eq!(msg.id, "m1");
        assert_eq!(mb.pending(), 0);
        assert_eq!(mb.delivered_count(), 1);
    }

    #[test]
    fn exactly_once_delivery() {
        let mut mb = Mailbox::new(10);
        mb.post(make_msg("m1", "child", "parent")).unwrap();
        mb.deliver().unwrap();
        // Re-posting the same ID should be silently skipped.
        mb.post(make_msg("m1", "child", "parent")).unwrap();
        assert_eq!(mb.pending(), 0);
        assert_eq!(mb.delivered_count(), 1);
    }

    #[test]
    fn duplicate_pending_message_is_delivered_once() {
        let mut mb = Mailbox::new(10);
        mb.post(make_msg("m1", "child", "parent")).unwrap();
        mb.post(make_msg("m1", "child", "parent")).unwrap();

        assert_eq!(mb.pending(), 1);
        assert_eq!(mb.drain().len(), 1);
    }

    #[test]
    fn delivered_replay_window_is_bounded() {
        let mut mb = Mailbox::with_dedupe_capacity(10, 2);
        for id in ["m1", "m2", "m3"] {
            mb.post(make_msg(id, "child", "parent")).unwrap();
            mb.deliver().unwrap();
        }

        assert_eq!(mb.delivered_count(), 3);
        assert_eq!(mb.dedupe_entries(), 2);

        // A recent replay is still suppressed.
        mb.post(make_msg("m3", "child", "parent")).unwrap();
        assert_eq!(mb.pending(), 0);

        // The oldest ID has left the explicitly bounded replay window.
        mb.post(make_msg("m1", "child", "parent")).unwrap();
        assert_eq!(mb.pending(), 1);
    }

    #[test]
    fn mailbox_full_errors() {
        let mut mb = Mailbox::new(2);
        mb.post(make_msg("m1", "a", "b")).unwrap();
        mb.post(make_msg("m2", "a", "b")).unwrap();
        let err = mb.post(make_msg("m3", "a", "b")).unwrap_err();
        assert_eq!(err.capacity, 2);
    }

    #[test]
    fn drain_delivers_all() {
        let mut mb = Mailbox::new(10);
        mb.post(make_msg("m1", "a", "b")).unwrap();
        mb.post(make_msg("m2", "a", "b")).unwrap();
        let msgs = mb.drain();
        assert_eq!(msgs.len(), 2);
        assert_eq!(mb.pending(), 0);
        assert_eq!(mb.delivered_count(), 2);
    }

    #[test]
    fn pending_for_counts_recipient() {
        let mut mb = Mailbox::new(10);
        mb.post(make_msg("m1", "a", "parent")).unwrap();
        mb.post(make_msg("m2", "b", "parent")).unwrap();
        mb.post(make_msg("m3", "c", "other")).unwrap();
        assert_eq!(mb.pending_for("parent"), 2);
        assert_eq!(mb.pending_for("other"), 1);
    }

    // ---- file conflict detection ----

    #[test]
    fn exact_path_conflict() {
        let ownerships = vec![
            FileOwnership {
                task_id: "t1".into(),
                file_globs: vec!["src/main.rs".into()],
            },
            FileOwnership {
                task_id: "t2".into(),
                file_globs: vec!["src/main.rs".into()],
            },
        ];
        let conflicts = detect_conflicts(&ownerships);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].task_a, "t1");
        assert_eq!(conflicts[0].task_b, "t2");
    }

    #[test]
    fn directory_prefix_conflict() {
        let ownerships = vec![
            FileOwnership {
                task_id: "t1".into(),
                file_globs: vec!["src/auth".into()],
            },
            FileOwnership {
                task_id: "t2".into(),
                file_globs: vec!["src/auth/login.rs".into()],
            },
        ];
        let conflicts = detect_conflicts(&ownerships);
        assert!(
            !conflicts.is_empty(),
            "src/auth and src/auth/login.rs should conflict"
        );
    }

    #[test]
    fn no_conflict_different_dirs() {
        let ownerships = vec![
            FileOwnership {
                task_id: "t1".into(),
                file_globs: vec!["src/auth".into()],
            },
            FileOwnership {
                task_id: "t2".into(),
                file_globs: vec!["src/db".into()],
            },
        ];
        let conflicts = detect_conflicts(&ownerships);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn wildcard_conflict() {
        let ownerships = vec![
            FileOwnership {
                task_id: "t1".into(),
                file_globs: vec!["src/*.rs".into()],
            },
            FileOwnership {
                task_id: "t2".into(),
                file_globs: vec!["src/main.rs".into()],
            },
        ];
        let conflicts = detect_conflicts(&ownerships);
        assert!(
            !conflicts.is_empty(),
            "src/*.rs should conflict with src/main.rs"
        );
    }

    #[test]
    fn wildcard_suffixes_must_be_compatible() {
        assert!(!glob_overlap("src/*.rs", "src/main.ts"));
        assert!(glob_overlap("src/auth*.rs", "src/authentication.rs"));
    }

    #[test]
    fn wildcard_matching_respects_directory_boundaries() {
        assert!(!glob_overlap(
            "src/auth/*.rs",
            "src/authentication/login.rs"
        ));
        assert!(!glob_overlap("src/*.rs", "src/nested/main.rs"));
        assert!(glob_overlap("src/**/*.rs", "src/nested/main.rs"));
    }

    #[test]
    fn empty_globs_do_not_claim_paths() {
        assert!(!glob_overlap("", ""));
        assert!(!glob_overlap("", "src/main.rs"));
    }

    // ---- claim semantics ----

    #[test]
    fn claim_atomic() {
        let mut state = HashMap::new();
        let result = claim_tasks(&mut state, "agent-1", &["t1".into(), "t2".into()]);
        assert_eq!(result.claimed.len(), 2);
        assert!(result.already_claimed.is_empty());
        assert_eq!(state.get("t1"), Some(&"agent-1".to_string()));
    }

    #[test]
    fn claim_already_claimed() {
        let mut state = HashMap::new();
        state.insert("t1".into(), "agent-1".into());
        let result = claim_tasks(&mut state, "agent-2", &["t1".into(), "t2".into()]);
        assert_eq!(result.claimed, vec!["t2"]);
        assert_eq!(result.already_claimed, vec!["t1"]);
    }

    #[test]
    fn claim_same_claimant_idempotent() {
        let mut state = HashMap::new();
        state.insert("t1".into(), "agent-1".into());
        let result = claim_tasks(&mut state, "agent-1", &["t1".into()]);
        assert!(result.claimed.contains(&"t1".to_string()));
        assert!(result.already_claimed.is_empty());
    }

    #[test]
    fn release_claim_works() {
        let mut state = HashMap::new();
        state.insert("t1".into(), "agent-1".into());
        assert!(release_claim(&mut state, "t1"));
        assert!(!state.contains_key("t1"));
    }

    #[test]
    fn transfer_claim_failover() {
        let mut state = HashMap::new();
        state.insert("t1".into(), "agent-1".into());
        assert!(transfer_claim(&mut state, "t1", "agent-1", "agent-2"));
        assert_eq!(state.get("t1"), Some(&"agent-2".to_string()));
    }

    #[test]
    fn transfer_claim_wrong_owner_fails() {
        let mut state = HashMap::new();
        state.insert("t1".into(), "agent-1".into());
        assert!(!transfer_claim(&mut state, "t1", "agent-wrong", "agent-2"));
    }

    // ---- fan-out approval ----

    #[test]
    fn small_fanout_allowed() {
        let decision = evaluate_fan_out(2, &FanOutConfig::default());
        assert!(matches!(decision, FanOutDecision::Allowed { .. }));
    }

    #[test]
    fn threshold_fanout_needs_approval() {
        let decision = evaluate_fan_out(3, &FanOutConfig::default());
        assert!(matches!(decision, FanOutDecision::NeedsApproval { .. }));
    }

    #[test]
    fn excessive_fanout_denied() {
        let decision = evaluate_fan_out(10, &FanOutConfig::default());
        assert!(matches!(decision, FanOutDecision::Denied { .. }));
    }

    #[test]
    fn zero_fanout_allowed() {
        let decision = evaluate_fan_out(0, &FanOutConfig::default());
        assert!(matches!(
            decision,
            FanOutDecision::Allowed { child_count: 0 }
        ));
    }

    // ---- worktree isolation ----

    #[test]
    fn shared_worktree_detected() {
        let mut map = HashMap::new();
        map.insert("t1".into(), "/repo/wt-1".into());
        map.insert("t2".into(), "/repo/wt-1".into()); // same worktree!
        map.insert("t3".into(), "/repo/wt-2".into());
        let conflicts = check_worktree_isolation(&map);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].task_ids.len(), 2);
    }

    #[test]
    fn isolated_worktrees_no_conflict() {
        let mut map = HashMap::new();
        map.insert("t1".into(), "/repo/wt-1".into());
        map.insert("t2".into(), "/repo/wt-2".into());
        let conflicts = check_worktree_isolation(&map);
        assert!(conflicts.is_empty());
    }

    // ---- F4 acceptance ----

    #[test]
    fn message_delivery_exactly_once() {
        let mut mb = Mailbox::new(10);
        mb.post(make_msg("m1", "child", "parent")).unwrap();
        let first = mb.deliver();
        let second = mb.deliver();
        assert!(first.is_some());
        assert!(second.is_none(), "message should be delivered exactly once");
    }

    #[test]
    fn parallel_writes_never_share_worktree() {
        let mut map = HashMap::new();
        map.insert("t1".into(), "/repo/wt-a".into());
        map.insert("t2".into(), "/repo/wt-a".into());
        let conflicts = check_worktree_isolation(&map);
        assert!(
            !conflicts.is_empty(),
            "two tasks in same worktree must be flagged"
        );
    }
}
