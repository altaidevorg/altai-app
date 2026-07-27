//! Task graphs, dependency DAG, cycle detection, and decomposition (plan §F1–F2).
//!
//! F1: dependency CRUD, cycle detection (with useful path), priority/age/fairness
//! dispatch ordering, critical path, blocked-reason, transactional eligibility.
//! F2: planner output, plan validation, version-specific approval, plan diffing.

use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// F1: Dependency DAG
// ---------------------------------------------------------------------------

/// A single dependency edge: `task_id` depends on `depends_on`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDependency {
    pub task_id: String,
    pub depends_on: String,
}

/// A directed acyclic graph of tasks and their dependencies.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TaskGraph {
    pub nodes: HashSet<String>,
    /// All edges: child → depends_on.
    pub edges: Vec<TaskDependency>,
}

/// Error when a cycle is detected in the dependency graph.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CycleError {
    /// The cycle path, e.g., ["A", "B", "C", "A"].
    pub cycle: Vec<String>,
}

impl std::fmt::Display for CycleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Dependency cycle detected: {}", self.cycle.join(" → "))
    }
}

impl std::error::Error for CycleError {}

impl TaskGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a node (task) to the graph.
    pub fn add_node(&mut self, task_id: impl Into<String>) {
        self.nodes.insert(task_id.into());
    }

    /// Add a dependency edge. Returns an error if it would create a cycle.
    pub fn add_dependency(&mut self, task_id: &str, depends_on: &str) -> Result<(), CycleError> {
        if task_id == depends_on {
            return Err(CycleError {
                cycle: vec![task_id.into(), depends_on.into()],
            });
        }
        let edge = TaskDependency {
            task_id: task_id.to_string(),
            depends_on: depends_on.to_string(),
        };
        if self.edges.contains(&edge) {
            return Ok(());
        }
        // Temporarily add the edge, then check for cycles.
        self.edges.push(edge.clone());
        self.nodes.insert(task_id.to_string());
        self.nodes.insert(depends_on.to_string());
        if let Some(cycle) = self.detect_cycle() {
            // Remove the edge we just added.
            self.edges.retain(|e| e != &edge);
            return Err(CycleError { cycle });
        }
        Ok(())
    }

    /// Remove a dependency edge.
    pub fn remove_dependency(&mut self, task_id: &str, depends_on: &str) {
        self.edges
            .retain(|e| !(e.task_id == task_id && e.depends_on == depends_on));
    }

    /// Get all tasks that a given task depends on (direct dependencies).
    pub fn dependencies_of(&self, task_id: &str) -> Vec<String> {
        self.edges
            .iter()
            .filter(|e| e.task_id == task_id)
            .map(|e| e.depends_on.clone())
            .collect()
    }

    /// Get all tasks that depend on a given task (direct dependents).
    pub fn dependents_of(&self, task_id: &str) -> Vec<String> {
        self.edges
            .iter()
            .filter(|e| e.depends_on == task_id)
            .map(|e| e.task_id.clone())
            .collect()
    }

    /// Get all transitive dependencies of a task (the full ancestor set).
    pub fn transitive_dependencies(&self, task_id: &str) -> HashSet<String> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.extend(self.dependencies_of(task_id));
        while let Some(dep) = queue.pop_front() {
            if visited.insert(dep.clone()) {
                queue.extend(self.dependencies_of(&dep));
            }
        }
        visited
    }

    /// Detect a cycle in the graph. Returns the cycle path if found.
    pub fn detect_cycle(&self) -> Option<Vec<String>> {
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
        for node in &self.nodes {
            adj.entry(node).or_default();
        }
        for edge in &self.edges {
            adj.entry(&edge.task_id).or_default().push(&edge.depends_on);
        }

        let mut white: HashSet<&str> = self.nodes.iter().map(|s| s.as_str()).collect();
        let mut gray: HashSet<&str> = HashSet::new();
        let mut black: HashSet<&str> = HashSet::new();
        let mut path: Vec<&str> = Vec::new();

        for node in &self.nodes {
            if white.contains(node.as_str()) {
                if let Some(cycle) =
                    dfs_cycle(node, &adj, &mut white, &mut gray, &mut black, &mut path)
                {
                    return Some(cycle.iter().map(|s| s.to_string()).collect());
                }
            }
        }
        None
    }

    /// Compute a topological order. Returns an error if a cycle exists.
    pub fn topological_order(&self) -> Result<Vec<String>, CycleError> {
        if let Some(cycle) = self.detect_cycle() {
            return Err(CycleError { cycle });
        }
        // Kahn's algorithm.
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
        for node in &self.nodes {
            in_degree.entry(node).or_insert(0);
            adj.entry(node).or_default();
        }
        for edge in &self.edges {
            // edge: task_id depends_on depends_on
            // In topological order, depends_on comes before task_id.
            adj.entry(&edge.depends_on).or_default().push(&edge.task_id);
            *in_degree.entry(&edge.task_id).or_insert(0) += 1;
        }
        let mut queue: BTreeSet<&str> = in_degree
            .iter()
            .filter(|(_, &d)| d == 0)
            .map(|(&n, _)| n)
            .collect();
        let mut result = Vec::new();
        while let Some(node) = queue.pop_first() {
            result.push(node.to_string());
            if let Some(neighbors) = adj.get(node) {
                let mut neighbors = neighbors.clone();
                neighbors.sort_unstable();
                for n in neighbors {
                    if let Some(d) = in_degree.get_mut(n) {
                        *d -= 1;
                        if *d == 0 {
                            queue.insert(n);
                        }
                    }
                }
            }
        }
        Ok(result)
    }

    /// Compute which tasks are eligible to run (all dependencies completed).
    pub fn eligible_tasks(&self, completed: &HashSet<String>) -> Vec<String> {
        self.nodes
            .iter()
            .filter(|node| !completed.contains(*node))
            .filter(|node| {
                let deps = self.dependencies_of(node);
                deps.iter().all(|d| completed.contains(d))
            })
            .cloned()
            .collect()
    }

    /// Get the blocked reason for a task (which dependencies are not yet done).
    pub fn blocked_reason(
        &self,
        task_id: &str,
        completed: &HashSet<String>,
    ) -> Option<Vec<String>> {
        let unmet: Vec<String> = self
            .dependencies_of(task_id)
            .into_iter()
            .filter(|d| !completed.contains(d))
            .collect();
        if unmet.is_empty() {
            None
        } else {
            Some(unmet)
        }
    }

    /// Compute the critical path through the graph given task durations.
    /// Returns the longest chain of tasks by total duration.
    pub fn critical_path(&self, durations: &HashMap<String, u64>) -> Vec<String> {
        if self.nodes.is_empty() {
            return Vec::new();
        }
        // Topological order (safe — we checked for cycles at construction).
        let order = self.topological_order().unwrap_or_default();

        // Longest path ending at each node.
        let mut dist: HashMap<String, u64> = HashMap::new();
        let mut parent: HashMap<String, Option<String>> = HashMap::new();

        for node in &self.nodes {
            dist.insert(node.clone(), *durations.get(node).unwrap_or(&0));
            parent.insert(node.clone(), None);
        }

        // Process in topological order (depends_on before task_id).
        // For each edge depends_on → task_id:
        //   if dist(depends_on) + dur(task_id) > dist(task_id), update.
        for node in &order {
            for dependent in self.dependents_of(node) {
                let candidate =
                    dist.get(node).copied().unwrap_or(0) + durations.get(&dependent).unwrap_or(&0);
                if candidate > dist.get(&dependent).copied().unwrap_or(0) {
                    dist.insert(dependent.clone(), candidate);
                    parent.insert(dependent.clone(), Some(node.clone()));
                }
            }
        }

        // Find the node with the maximum distance.
        let end = dist
            .iter()
            .max_by_key(|(_, &d)| d)
            .map(|(n, _)| n.clone())
            .unwrap_or_default();

        // Backtrack from end to build the path.
        let mut path = Vec::new();
        let mut current = Some(end);
        while let Some(node) = current {
            path.push(node.clone());
            current = parent.get(&node).cloned().flatten();
        }
        path.reverse();
        path
    }
}

fn dfs_cycle<'a>(
    node: &'a str,
    adj: &HashMap<&str, Vec<&'a str>>,
    white: &mut HashSet<&'a str>,
    gray: &mut HashSet<&'a str>,
    black: &mut HashSet<&'a str>,
    path: &mut Vec<&'a str>,
) -> Option<Vec<&'a str>> {
    white.remove(node);
    gray.insert(node);
    path.push(node);

    if let Some(neighbors) = adj.get(node) {
        for &n in neighbors {
            if black.contains(n) {
                continue;
            }
            if gray.contains(n) {
                // Found a cycle — extract it from the path.
                let start_idx = path.iter().position(|&p| p == n).unwrap_or(0);
                let mut cycle = path[start_idx..].to_vec();
                cycle.push(n); // close the cycle
                return Some(cycle);
            }
            if white.contains(n) {
                if let Some(cycle) = dfs_cycle(n, adj, white, gray, black, path) {
                    return Some(cycle);
                }
            }
        }
    }

    gray.remove(node);
    black.insert(node);
    path.pop();
    None
}

// ---------------------------------------------------------------------------
// F1: Dispatch ordering
// ---------------------------------------------------------------------------

/// Priority levels for task dispatch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Low,
    #[default]
    Normal,
    High,
    Critical,
}

impl Priority {
    pub fn rank(self) -> u8 {
        match self {
            Self::Critical => 0,
            Self::High => 1,
            Self::Normal => 2,
            Self::Low => 3,
        }
    }
}

/// A task with scheduling metadata for dispatch ordering.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchedulableTask {
    pub task_id: String,
    pub priority: Priority,
    pub created_at_ms: u64,
}

/// Order eligible tasks by priority (desc), then age (oldest first), then ID
/// for deterministic fairness.
pub fn dispatch_order(
    graph: &TaskGraph,
    completed: &HashSet<String>,
    tasks: &[SchedulableTask],
) -> Vec<String> {
    let eligible: HashSet<String> = graph.eligible_tasks(completed).into_iter().collect();
    let mut ordered: Vec<&SchedulableTask> = tasks
        .iter()
        .filter(|t| eligible.contains(&t.task_id))
        .collect();
    ordered.sort_by(|a, b| {
        a.priority
            .rank()
            .cmp(&b.priority.rank())
            .then(a.created_at_ms.cmp(&b.created_at_ms))
            .then(a.task_id.cmp(&b.task_id))
    });
    ordered.iter().map(|t| t.task_id.clone()).collect()
}

// ---------------------------------------------------------------------------
// F2: Planning and decomposition
// ---------------------------------------------------------------------------

/// A planned task in a decomposition.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedTask {
    pub task_id: String,
    pub title: String,
    pub description: String,
    pub estimated_duration_ms: Option<u64>,
    pub priority: Priority,
}

/// A structured plan produced by the planner.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskPlan {
    pub id: String,
    pub version: u32,
    pub tasks: Vec<PlannedTask>,
    pub dependencies: Vec<TaskDependency>,
    pub approved: bool,
    pub approval_version: u32,
}

/// Errors during plan validation.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "kind")]
pub enum PlanError {
    Cycle { cycle: Vec<String> },
    UnknownTask { task_id: String },
    DuplicateTask { task_id: String },
    SelfDependency { task_id: String },
    EmptyPlan,
}

impl std::fmt::Display for PlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cycle { cycle } => {
                write!(f, "Plan has a dependency cycle: {}", cycle.join(" → "))
            }
            Self::UnknownTask { task_id } => {
                write!(f, "Dependency references unknown task: {task_id}")
            }
            Self::DuplicateTask { task_id } => write!(f, "Duplicate task ID: {task_id}"),
            Self::SelfDependency { task_id } => write!(f, "Task depends on itself: {task_id}"),
            Self::EmptyPlan => write!(f, "Plan has no tasks"),
        }
    }
}

impl std::error::Error for PlanError {}

/// Validate a plan: check for duplicates, self-deps, unknown deps, and cycles.
pub fn validate_plan(plan: &TaskPlan) -> Result<(), PlanError> {
    if plan.tasks.is_empty() {
        return Err(PlanError::EmptyPlan);
    }

    // Check for duplicate task IDs.
    let mut seen = HashSet::new();
    for task in &plan.tasks {
        if !seen.insert(task.task_id.clone()) {
            return Err(PlanError::DuplicateTask {
                task_id: task.task_id.clone(),
            });
        }
    }

    // Build a graph and validate edges.
    let mut graph = TaskGraph::new();
    for task in &plan.tasks {
        graph.add_node(task.task_id.clone());
    }
    for dep in &plan.dependencies {
        // Self-dependency.
        if dep.task_id == dep.depends_on {
            return Err(PlanError::SelfDependency {
                task_id: dep.task_id.clone(),
            });
        }
        // Unknown task references.
        if !graph.nodes.contains(&dep.task_id) {
            return Err(PlanError::UnknownTask {
                task_id: dep.task_id.clone(),
            });
        }
        if !graph.nodes.contains(&dep.depends_on) {
            return Err(PlanError::UnknownTask {
                task_id: dep.depends_on.clone(),
            });
        }
        // add_dependency checks for cycles.
        if let Err(cycle_err) = graph.add_dependency(&dep.task_id, &dep.depends_on) {
            return Err(PlanError::Cycle {
                cycle: cycle_err.cycle,
            });
        }
    }

    Ok(())
}

/// Changes between two plan versions.
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanChanges {
    pub added_tasks: Vec<String>,
    pub removed_tasks: Vec<String>,
    pub added_deps: Vec<TaskDependency>,
    pub removed_deps: Vec<TaskDependency>,
    pub version_changed: bool,
}

/// Compute the diff between two plan versions.
pub fn plan_diff(old: &TaskPlan, new: &TaskPlan) -> PlanChanges {
    let old_task_ids: HashSet<&str> = old.tasks.iter().map(|t| t.task_id.as_str()).collect();
    let new_task_ids: HashSet<&str> = new.tasks.iter().map(|t| t.task_id.as_str()).collect();

    let added_tasks = new_task_ids
        .difference(&old_task_ids)
        .map(|s| s.to_string())
        .collect();
    let removed_tasks = old_task_ids
        .difference(&new_task_ids)
        .map(|s| s.to_string())
        .collect();

    let old_deps: HashSet<&TaskDependency> = old.dependencies.iter().collect();
    let new_deps: HashSet<&TaskDependency> = new.dependencies.iter().collect();

    let added_deps: Vec<TaskDependency> = new_deps
        .difference(&old_deps)
        .map(|d| (*d).clone())
        .collect();
    let removed_deps: Vec<TaskDependency> = old_deps
        .difference(&new_deps)
        .map(|d| (*d).clone())
        .collect();

    PlanChanges {
        added_tasks,
        removed_tasks,
        added_deps,
        removed_deps,
        version_changed: old.version != new.version,
    }
}

/// Check whether a plan approval is valid for the current version.
/// Editing a plan (version change) invalidates the previous approval.
pub fn is_approval_valid(plan: &TaskPlan) -> bool {
    plan.approved && plan.approval_version == plan.version
}

/// Approve a plan at the current version.
pub fn approve_plan(plan: &mut TaskPlan) {
    plan.approved = true;
    plan.approval_version = plan.version;
}

/// Bump the plan version (called when tasks or dependencies change).
/// This invalidates any existing approval.
pub fn bump_version(plan: &mut TaskPlan) {
    plan.version += 1;
    plan.approved = false;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- cycle detection ----

    #[test]
    fn no_cycle_in_dag() {
        let mut g = TaskGraph::new();
        g.add_dependency("C", "B").unwrap();
        g.add_dependency("B", "A").unwrap();
        assert!(g.detect_cycle().is_none());
    }

    #[test]
    fn detects_simple_cycle() {
        let mut g = TaskGraph::new();
        g.add_dependency("A", "B").unwrap();
        let err = g.add_dependency("B", "A").unwrap_err();
        assert_eq!(err.cycle.len(), 3); // B → A → B
    }

    #[test]
    fn detects_longer_cycle() {
        let mut g = TaskGraph::new();
        g.add_dependency("A", "B").unwrap();
        g.add_dependency("B", "C").unwrap();
        g.add_dependency("C", "D").unwrap();
        let err = g.add_dependency("D", "B").unwrap_err();
        // Should detect D → B → C → D
        assert!(!err.cycle.is_empty());
    }

    #[test]
    fn self_dependency_rejected() {
        let mut g = TaskGraph::new();
        let err = g.add_dependency("A", "A").unwrap_err();
        assert_eq!(err.cycle, vec!["A", "A"]);
    }

    #[test]
    fn add_dependency_rolls_back_on_cycle() {
        let mut g = TaskGraph::new();
        g.add_dependency("A", "B").unwrap();
        assert!(g.add_dependency("B", "A").is_err());
        // The B→A edge should NOT be in the graph.
        assert_eq!(g.edges.len(), 1);
    }

    #[test]
    fn adding_the_same_dependency_is_idempotent() {
        let mut g = TaskGraph::new();
        g.add_dependency("B", "A").unwrap();
        g.add_dependency("B", "A").unwrap();

        assert_eq!(g.edges.len(), 1);
    }

    // ---- topological order ----

    #[test]
    fn topological_order_linear() {
        let mut g = TaskGraph::new();
        g.add_dependency("C", "B").unwrap();
        g.add_dependency("B", "A").unwrap();
        let order = g.topological_order().unwrap();
        let a_pos = order.iter().position(|s| s == "A").unwrap();
        let b_pos = order.iter().position(|s| s == "B").unwrap();
        let c_pos = order.iter().position(|s| s == "C").unwrap();
        assert!(a_pos < b_pos);
        assert!(b_pos < c_pos);
    }

    #[test]
    fn topological_order_with_cycle_errors() {
        let mut g = TaskGraph::new();
        g.add_dependency("A", "B").unwrap();
        g.add_dependency("B", "A").unwrap_err();
        // The graph should still have only one edge (B→A was rejected).
        let result = g.topological_order();
        assert!(result.is_ok()); // only A→B, no cycle
    }

    #[test]
    fn topological_order_is_deterministic_for_independent_nodes() {
        let mut g = TaskGraph::new();
        for node in ["C", "A", "B"] {
            g.add_node(node);
        }

        assert_eq!(g.topological_order().unwrap(), vec!["A", "B", "C"]);
    }

    // ---- eligibility ----

    #[test]
    fn eligible_tasks_respect_dependencies() {
        let mut g = TaskGraph::new();
        g.add_dependency("C", "B").unwrap();
        g.add_dependency("B", "A").unwrap();
        g.add_node("A");
        g.add_node("D"); // no dependencies

        let eligible = g.eligible_tasks(&HashSet::new());
        assert!(eligible.iter().any(|s| s == "A"));
        assert!(eligible.iter().any(|s| s == "D"));
        assert!(!eligible.iter().any(|s| s == "B"));
        assert!(!eligible.iter().any(|s| s == "C"));
    }

    #[test]
    fn completing_task_unblocks_dependents() {
        let mut g = TaskGraph::new();
        g.add_dependency("B", "A").unwrap();
        g.add_dependency("C", "B").unwrap();

        let completed = HashSet::from(["A".to_string()]);
        let eligible = g.eligible_tasks(&completed);
        assert!(eligible.iter().any(|s| s == "B"));
        assert!(!eligible.iter().any(|s| s == "C"));

        let completed = HashSet::from(["A".to_string(), "B".to_string()]);
        let eligible = g.eligible_tasks(&completed);
        assert!(eligible.iter().any(|s| s == "C"));
    }

    // ---- blocked reason ----

    #[test]
    fn blocked_reason_lists_unmet_deps() {
        let mut g = TaskGraph::new();
        g.add_dependency("C", "A").unwrap();
        g.add_dependency("C", "B").unwrap();

        let reason = g.blocked_reason("C", &HashSet::new()).unwrap();
        assert!(reason.contains(&"A".to_string()));
        assert!(reason.contains(&"B".to_string()));
    }

    #[test]
    fn no_blocked_reason_when_all_done() {
        let mut g = TaskGraph::new();
        g.add_dependency("B", "A").unwrap();

        let completed = HashSet::from(["A".to_string()]);
        assert!(g.blocked_reason("B", &completed).is_none());
    }

    // ---- transitive deps ----

    #[test]
    fn transitive_deps_full_chain() {
        let mut g = TaskGraph::new();
        g.add_dependency("D", "C").unwrap();
        g.add_dependency("C", "B").unwrap();
        g.add_dependency("B", "A").unwrap();

        let deps = g.transitive_dependencies("D");
        assert!(deps.contains("A"));
        assert!(deps.contains("B"));
        assert!(deps.contains("C"));
    }

    // ---- critical path ----

    #[test]
    fn critical_path_picks_longest() {
        let mut g = TaskGraph::new();
        // A → B → D
        // A → C → D (but C is slower)
        g.add_dependency("B", "A").unwrap();
        g.add_dependency("C", "A").unwrap();
        g.add_dependency("D", "B").unwrap();
        g.add_dependency("D", "C").unwrap();

        let durations: HashMap<String, u64> = [
            ("A".into(), 10),
            ("B".into(), 20),
            ("C".into(), 50),
            ("D".into(), 10),
        ]
        .into_iter()
        .collect();

        let path = g.critical_path(&durations);
        // Critical path should be A → C → D (total 70) not A → B → D (total 40).
        assert!(path.contains(&"C".to_string()));
        assert!(!path.contains(&"B".to_string()) || path.len() <= 2);
    }

    // ---- dispatch ordering ----

    #[test]
    fn dispatch_orders_by_priority_then_age() {
        let mut g = TaskGraph::new();
        g.add_node("t1");
        g.add_node("t2");
        g.add_node("t3");

        let tasks = vec![
            SchedulableTask {
                task_id: "t1".into(),
                priority: Priority::Normal,
                created_at_ms: 200,
            },
            SchedulableTask {
                task_id: "t2".into(),
                priority: Priority::Critical,
                created_at_ms: 100,
            },
            SchedulableTask {
                task_id: "t3".into(),
                priority: Priority::Normal,
                created_at_ms: 150,
            },
        ];

        let order = dispatch_order(&g, &HashSet::new(), &tasks);
        assert_eq!(order[0], "t2"); // Critical first
        assert_eq!(order[1], "t3"); // Normal, older (150 < 200)
        assert_eq!(order[2], "t1");
    }

    #[test]
    fn dispatch_excludes_blocked_tasks() {
        let mut g = TaskGraph::new();
        g.add_dependency("B", "A").unwrap();

        let tasks = vec![
            SchedulableTask {
                task_id: "A".into(),
                priority: Priority::Normal,
                created_at_ms: 100,
            },
            SchedulableTask {
                task_id: "B".into(),
                priority: Priority::Critical,
                created_at_ms: 50,
            },
        ];

        let order = dispatch_order(&g, &HashSet::new(), &tasks);
        assert!(order.contains(&"A".to_string()));
        assert!(!order.contains(&"B".to_string())); // blocked by A
    }

    // ---- plan validation ----

    #[test]
    fn valid_plan_passes() {
        let plan = TaskPlan {
            id: "plan-1".into(),
            version: 1,
            tasks: vec![
                PlannedTask {
                    task_id: "A".into(),
                    title: "Task A".into(),
                    description: "First".into(),
                    estimated_duration_ms: Some(1000),
                    priority: Priority::Normal,
                },
                PlannedTask {
                    task_id: "B".into(),
                    title: "Task B".into(),
                    description: "Second".into(),
                    estimated_duration_ms: Some(2000),
                    priority: Priority::High,
                },
            ],
            dependencies: vec![TaskDependency {
                task_id: "B".into(),
                depends_on: "A".into(),
            }],
            approved: false,
            approval_version: 0,
        };
        assert!(validate_plan(&plan).is_ok());
    }

    #[test]
    fn empty_plan_rejected() {
        let plan = TaskPlan {
            id: "plan-1".into(),
            version: 1,
            tasks: vec![],
            dependencies: vec![],
            approved: false,
            approval_version: 0,
        };
        assert!(matches!(validate_plan(&plan), Err(PlanError::EmptyPlan)));
    }

    #[test]
    fn duplicate_task_rejected() {
        let plan = TaskPlan {
            id: "plan-1".into(),
            version: 1,
            tasks: vec![
                PlannedTask {
                    task_id: "A".into(),
                    title: "A".into(),
                    description: "".into(),
                    estimated_duration_ms: None,
                    priority: Priority::Normal,
                },
                PlannedTask {
                    task_id: "A".into(),
                    title: "A again".into(),
                    description: "".into(),
                    estimated_duration_ms: None,
                    priority: Priority::Normal,
                },
            ],
            dependencies: vec![],
            approved: false,
            approval_version: 0,
        };
        assert!(matches!(
            validate_plan(&plan),
            Err(PlanError::DuplicateTask { .. })
        ));
    }

    #[test]
    fn plan_cycle_rejected() {
        let plan = TaskPlan {
            id: "plan-1".into(),
            version: 1,
            tasks: vec![
                PlannedTask {
                    task_id: "A".into(),
                    title: "A".into(),
                    description: "".into(),
                    estimated_duration_ms: None,
                    priority: Priority::Normal,
                },
                PlannedTask {
                    task_id: "B".into(),
                    title: "B".into(),
                    description: "".into(),
                    estimated_duration_ms: None,
                    priority: Priority::Normal,
                },
            ],
            dependencies: vec![
                TaskDependency {
                    task_id: "B".into(),
                    depends_on: "A".into(),
                },
                TaskDependency {
                    task_id: "A".into(),
                    depends_on: "B".into(),
                },
            ],
            approved: false,
            approval_version: 0,
        };
        assert!(matches!(validate_plan(&plan), Err(PlanError::Cycle { .. })));
    }

    #[test]
    fn unknown_dependency_rejected() {
        let plan = TaskPlan {
            id: "plan-1".into(),
            version: 1,
            tasks: vec![PlannedTask {
                task_id: "A".into(),
                title: "A".into(),
                description: "".into(),
                estimated_duration_ms: None,
                priority: Priority::Normal,
            }],
            dependencies: vec![TaskDependency {
                task_id: "A".into(),
                depends_on: "X".into(), // X doesn't exist
            }],
            approved: false,
            approval_version: 0,
        };
        assert!(matches!(
            validate_plan(&plan),
            Err(PlanError::UnknownTask { .. })
        ));
    }

    // ---- plan approval lifecycle ----

    #[test]
    fn approval_is_version_specific() {
        let mut plan = TaskPlan {
            id: "p".into(),
            version: 1,
            tasks: vec![PlannedTask {
                task_id: "A".into(),
                title: "A".into(),
                description: "".into(),
                estimated_duration_ms: None,
                priority: Priority::Normal,
            }],
            dependencies: vec![],
            approved: false,
            approval_version: 0,
        };
        assert!(!is_approval_valid(&plan));

        approve_plan(&mut plan);
        assert!(is_approval_valid(&plan));

        bump_version(&mut plan); // editing invalidates approval
        assert!(!is_approval_valid(&plan));
        assert_eq!(plan.version, 2);
    }

    // ---- plan diff ----

    #[test]
    fn plan_diff_detects_changes() {
        let old = TaskPlan {
            id: "p".into(),
            version: 1,
            tasks: vec![
                PlannedTask {
                    task_id: "A".into(),
                    title: "A".into(),
                    description: "".into(),
                    estimated_duration_ms: None,
                    priority: Priority::Normal,
                },
                PlannedTask {
                    task_id: "B".into(),
                    title: "B".into(),
                    description: "".into(),
                    estimated_duration_ms: None,
                    priority: Priority::Normal,
                },
            ],
            dependencies: vec![TaskDependency {
                task_id: "B".into(),
                depends_on: "A".into(),
            }],
            approved: true,
            approval_version: 1,
        };

        let new = TaskPlan {
            id: "p".into(),
            version: 2,
            tasks: vec![
                PlannedTask {
                    task_id: "A".into(),
                    title: "A".into(),
                    description: "".into(),
                    estimated_duration_ms: None,
                    priority: Priority::Normal,
                },
                PlannedTask {
                    task_id: "C".into(),
                    title: "C".into(),
                    description: "".into(),
                    estimated_duration_ms: None,
                    priority: Priority::Normal,
                },
            ],
            dependencies: vec![TaskDependency {
                task_id: "C".into(),
                depends_on: "A".into(),
            }],
            approved: false,
            approval_version: 1,
        };

        let diff = plan_diff(&old, &new);
        assert!(diff.version_changed);
        assert_eq!(diff.added_tasks, vec!["C"]);
        assert_eq!(diff.removed_tasks, vec!["B"]);
        assert_eq!(diff.added_deps.len(), 1);
        assert_eq!(diff.removed_deps.len(), 1);
    }

    #[test]
    fn plan_diff_no_changes() {
        let plan = TaskPlan {
            id: "p".into(),
            version: 1,
            tasks: vec![PlannedTask {
                task_id: "A".into(),
                title: "A".into(),
                description: "".into(),
                estimated_duration_ms: None,
                priority: Priority::Normal,
            }],
            dependencies: vec![],
            approved: false,
            approval_version: 0,
        };
        let diff = plan_diff(&plan, &plan);
        assert!(diff.added_tasks.is_empty());
        assert!(diff.removed_tasks.is_empty());
        assert!(!diff.version_changed);
    }

    // ---- priority ordering ----

    #[test]
    fn priority_rank_orders_correctly() {
        assert!(Priority::Critical.rank() < Priority::High.rank());
        assert!(Priority::High.rank() < Priority::Normal.rank());
        assert!(Priority::Normal.rank() < Priority::Low.rank());
    }
}
