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
    /// IDs of messages that have been delivered (exactly-once).
    delivered: HashSet<String>,
}

/// Error when the mailbox is full.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct MailboxFullError {
    pub capacity: usize,
}

impl Mailbox {
    pub fn new(capacity: usize) -> Self {
        Self {
            messages: VecDeque::with_capacity(capacity),
            capacity,
            delivered: HashSet::new(),
        }
    }

    /// Post a message. Returns error if mailbox is full or message is a duplicate.
    pub fn post(&mut self, msg: AgentMessage) -> Result<(), MailboxFullError> {
        // Exactly-once: skip if already delivered.
        if self.delivered.contains(&msg.id) {
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
        self.delivered.insert(msg.id.clone());
        Some(msg)
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

    /// Number of messages delivered so far.
    pub fn delivered_count(&self) -> usize {
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

/// Check if two glob patterns overlap (simplified: exact match or prefix match).
fn glob_overlap(a: &str, b: &str) -> bool {
    // Normalize: remove trailing /*
    let a_base = a.trim_end_matches("/*");
    let b_base = b.trim_end_matches("/*");

    // Exact match.
    if a_base == b_base {
        return true;
    }

    // One is a prefix of the other (directory containment).
    if a_base.starts_with(&format!("{b_base}/")) || b_base.starts_with(&format!("{a_base}/")) {
        return true;
    }

    // Wildcard overlap: if both have *, check if they could match the same file.
    if a_base.contains('*') || b_base.contains('*') {
        // Simplified: if one is a glob prefix of the other.
        let a_prefix = a_base.split('*').next().unwrap_or("");
        let b_prefix = b_base.split('*').next().unwrap_or("");
        if !a_prefix.is_empty() && !b_prefix.is_empty() {
            return a_base.starts_with(b_prefix) || b_base.starts_with(a_prefix);
        }
    }

    false
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
