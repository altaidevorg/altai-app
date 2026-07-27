//! SSH and remote worker pool (plan §I3).
//!
//! Worker health/capacity tracking, per-host concurrency, sticky retry
//! placement, explicit failover semantics, and remote cleanup visibility.
//! Local and remote runs use the same orchestration contract.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Worker types
// ---------------------------------------------------------------------------

/// A remote worker host.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerHost {
    pub id: String,
    pub address: String,
    pub max_concurrency: usize,
    pub env_revision: String,
    pub labels: Vec<String>,
}

/// The health status of a worker.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerHealth {
    Healthy,
    Degraded,
    Unhealthy,
    Offline,
}

impl WorkerHealth {
    pub fn can_accept_work(self) -> bool {
        matches!(self, Self::Healthy | Self::Degraded)
    }

    pub fn is_available(self) -> bool {
        !matches!(self, Self::Offline)
    }
}

/// Runtime state tracked per worker.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerState {
    pub host_id: String,
    pub health: WorkerHealth,
    pub active_attempts: usize,
    pub completed_attempts: usize,
    pub failed_attempts: usize,
    pub last_heartbeat_ms: u64,
    pub env_drift_detected: bool,
    /// Sticky placement: which tasks were last assigned here.
    pub sticky_tasks: Vec<String>,
}

// ---------------------------------------------------------------------------
// Worker pool
// ---------------------------------------------------------------------------

/// Manages a pool of remote workers with capacity, health, and failover.
#[derive(Clone, Debug)]
pub struct WorkerPool {
    hosts: HashMap<String, WorkerHost>,
    states: HashMap<String, WorkerState>,
    /// Active task assignment: task_id → host_id.
    assignments: HashMap<String, String>,
    heartbeat_timeout_ms: u64,
    preferred_labels: Vec<String>,
}

/// Result of attempting to assign work to a worker.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum AssignmentResult {
    Assigned { host_id: String },
    Queued { reason: String },
    NoCapacity,
    NoWorkers,
}

/// Configuration for the pool.
#[derive(Clone, Debug)]
pub struct PoolConfig {
    pub heartbeat_timeout_ms: u64,
    pub preferred_labels: Vec<String>,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            heartbeat_timeout_ms: 120_000,
            preferred_labels: vec![],
        }
    }
}

impl WorkerPool {
    pub fn new(config: &PoolConfig) -> Self {
        Self {
            hosts: HashMap::new(),
            states: HashMap::new(),
            assignments: HashMap::new(),
            heartbeat_timeout_ms: config.heartbeat_timeout_ms,
            preferred_labels: config.preferred_labels.clone(),
        }
    }

    /// Register a new worker host.
    pub fn register(&mut self, host: WorkerHost) {
        let host_id = host.id.clone();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        self.states
            .entry(host_id.clone())
            .and_modify(|state| {
                state.last_heartbeat_ms = now;
                state.health = if state.env_drift_detected {
                    WorkerHealth::Degraded
                } else {
                    WorkerHealth::Healthy
                };
            })
            .or_insert_with(|| WorkerState {
                host_id: host_id.clone(),
                health: WorkerHealth::Healthy,
                active_attempts: 0,
                completed_attempts: 0,
                failed_attempts: 0,
                last_heartbeat_ms: now,
                env_drift_detected: false,
                sticky_tasks: vec![],
            });
        self.hosts.insert(host_id, host);
    }

    /// Remove an idle worker host. Active hosts must go through
    /// [`Self::handle_host_loss`] first so assignments cannot disappear.
    pub fn deregister(&mut self, host_id: &str) -> Option<WorkerHost> {
        if self
            .assignments
            .values()
            .any(|assigned_host| assigned_host == host_id)
        {
            return None;
        }
        self.states.remove(host_id);
        self.hosts.remove(host_id)
    }

    /// Update a worker's heartbeat.
    pub fn heartbeat(&mut self, host_id: &str, now_ms: u64) {
        if let Some(state) = self.states.get_mut(host_id) {
            state.last_heartbeat_ms = now_ms;
            if matches!(
                state.health,
                WorkerHealth::Unhealthy | WorkerHealth::Offline
            ) {
                state.health = if state.env_drift_detected {
                    WorkerHealth::Degraded
                } else {
                    WorkerHealth::Healthy
                };
            }
        }
    }

    /// Check for expired heartbeats and mark workers as offline/unhealthy.
    pub fn check_health(&mut self, now_ms: u64) -> Vec<String> {
        let mut changed = Vec::new();
        for (id, state) in &mut self.states {
            if now_ms > state.last_heartbeat_ms {
                let elapsed = now_ms - state.last_heartbeat_ms;
                if elapsed > self.heartbeat_timeout_ms.saturating_mul(2) {
                    if state.health != WorkerHealth::Offline {
                        state.health = WorkerHealth::Offline;
                        changed.push(id.clone());
                    }
                } else if elapsed > self.heartbeat_timeout_ms
                    && state.health != WorkerHealth::Unhealthy
                {
                    state.health = WorkerHealth::Unhealthy;
                    changed.push(id.clone());
                }
            }
        }
        changed
    }

    /// Mark environment drift detected on a worker.
    pub fn mark_env_drift(&mut self, host_id: &str) {
        if let Some(state) = self.states.get_mut(host_id) {
            state.env_drift_detected = true;
            if state.health == WorkerHealth::Healthy {
                state.health = WorkerHealth::Degraded;
            }
        }
    }

    /// Clear environment drift (after worker is updated).
    pub fn clear_env_drift(&mut self, host_id: &str) {
        if let Some(state) = self.states.get_mut(host_id) {
            state.env_drift_detected = false;
            if state.health == WorkerHealth::Degraded {
                state.health = WorkerHealth::Healthy;
            }
        }
    }

    /// Attempt to assign a task to a worker. Prefers sticky placement, then
    /// least-loaded healthy worker, then label preferences.
    pub fn assign(&mut self, task_id: &str, _now_ms: u64) -> AssignmentResult {
        // Assignment is idempotent per active task. A retry cannot increment
        // capacity or launch the same task twice.
        if let Some(host_id) = self.assignments.get(task_id) {
            return AssignmentResult::Assigned {
                host_id: host_id.clone(),
            };
        }

        let available: Vec<String> = self
            .states
            .iter()
            .filter(|(_, s)| s.health.can_accept_work())
            .filter(|(_, s)| {
                let host = self.hosts.get(&s.host_id);
                host.is_some_and(|h| s.active_attempts < h.max_concurrency)
            })
            .map(|(id, _)| id.clone())
            .collect();

        if available.is_empty() {
            // Check if there are workers at all.
            if self.hosts.is_empty() {
                return AssignmentResult::NoWorkers;
            }
            return AssignmentResult::NoCapacity;
        }

        // Sticky placement: prefer the worker that previously ran this task.
        let sticky = available.iter().find(|id| {
            self.states
                .get(*id)
                .is_some_and(|s| s.sticky_tasks.iter().any(|task| task == task_id))
        });

        let chosen = sticky.cloned().or_else(|| {
            // Least-loaded worker; configured labels and worker ID provide
            // deterministic tie-breakers.
            available.into_iter().min_by_key(|id| {
                let active = self
                    .states
                    .get(id)
                    .map(|s| s.active_attempts)
                    .unwrap_or(usize::MAX);
                let label_rank = self
                    .hosts
                    .get(id)
                    .and_then(|host| {
                        self.preferred_labels
                            .iter()
                            .position(|preferred| host.labels.contains(preferred))
                    })
                    .unwrap_or(usize::MAX);
                (active, label_rank, id.clone())
            })
        });

        match chosen {
            Some(host_id) => {
                if let Some(state) = self.states.get_mut(&host_id) {
                    state.active_attempts = state.active_attempts.saturating_add(1);
                    if !state.sticky_tasks.iter().any(|task| task == task_id) {
                        state.sticky_tasks.push(task_id.to_string());
                        if state.sticky_tasks.len() > 10 {
                            state.sticky_tasks.remove(0);
                        }
                    }
                }
                self.assignments
                    .insert(task_id.to_string(), host_id.clone());
                AssignmentResult::Assigned { host_id }
            }
            None => AssignmentResult::NoCapacity,
        }
    }

    /// Release an attempt from a worker (completed or failed).
    pub fn release(&mut self, host_id: &str, task_id: &str, success: bool) {
        if self.assignments.get(task_id).map(String::as_str) != Some(host_id) {
            return;
        }
        self.assignments.remove(task_id);
        if let Some(state) = self.states.get_mut(host_id) {
            state.active_attempts = state.active_attempts.saturating_sub(1);
            if success {
                state.completed_attempts = state.completed_attempts.saturating_add(1);
            } else {
                state.failed_attempts = state.failed_attempts.saturating_add(1);
            }
        }
    }

    /// Handle worker loss — fails over active attempts. Returns tasks that
    /// need reassignment. Loss of a host cannot duplicate an active attempt
    /// because the original assignment is atomically revoked.
    pub fn handle_host_loss(&mut self, host_id: &str) -> Vec<String> {
        let mut orphaned: Vec<String> = self
            .assignments
            .iter()
            .filter(|(_, assigned_host)| assigned_host.as_str() == host_id)
            .map(|(task_id, _)| task_id.clone())
            .collect();
        orphaned.sort();
        for task_id in &orphaned {
            self.assignments.remove(task_id);
        }
        if let Some(state) = self.states.get_mut(host_id) {
            state.health = WorkerHealth::Offline;
            state.active_attempts = 0;
            state.sticky_tasks.clear();
        }
        orphaned
    }

    /// Total capacity across all healthy workers.
    pub fn total_capacity(&self) -> usize {
        self.states
            .values()
            .filter(|s| s.health.can_accept_work())
            .map(|s| {
                self.hosts
                    .get(&s.host_id)
                    .map(|h| h.max_concurrency.saturating_sub(s.active_attempts))
                    .unwrap_or(0)
            })
            .fold(0usize, usize::saturating_add)
    }

    /// Total active assignments across all workers.
    pub fn total_active(&self) -> usize {
        self.states
            .values()
            .map(|s| s.active_attempts)
            .fold(0usize, usize::saturating_add)
    }

    /// Get the state of a specific worker.
    pub fn state(&self, host_id: &str) -> Option<&WorkerState> {
        self.states.get(host_id)
    }

    /// List all worker IDs.
    pub fn worker_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.hosts.keys().cloned().collect();
        ids.sort();
        ids
    }

    /// Count workers by health status.
    pub fn health_counts(&self) -> HealthCounts {
        let mut healthy = 0;
        let mut degraded = 0;
        let mut unhealthy = 0;
        let mut offline = 0;
        for s in self.states.values() {
            match s.health {
                WorkerHealth::Healthy => healthy += 1,
                WorkerHealth::Degraded => degraded += 1,
                WorkerHealth::Unhealthy => unhealthy += 1,
                WorkerHealth::Offline => offline += 1,
            }
        }
        HealthCounts {
            healthy,
            degraded,
            unhealthy,
            offline,
        }
    }
}

/// Worker health counts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthCounts {
    pub healthy: usize,
    pub degraded: usize,
    pub unhealthy: usize,
    pub offline: usize,
}

impl HealthCounts {
    pub fn total(self) -> usize {
        self.healthy
            .saturating_add(self.degraded)
            .saturating_add(self.unhealthy)
            .saturating_add(self.offline)
    }
}

// ---------------------------------------------------------------------------
// Failover semantics
// ---------------------------------------------------------------------------

/// How to handle failover when a worker is lost.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailoverPolicy {
    /// Immediately reassign orphaned tasks to other workers.
    ImmediateReassign,
    /// Queue orphaned tasks — they wait for capacity.
    QueueAndWait,
    /// Fail orphaned tasks to NeedsAttention.
    FailToNeedsAttention,
}

/// Execute failover for orphaned tasks based on policy.
pub fn execute_failover(orphaned_tasks: Vec<String>, policy: FailoverPolicy) -> FailoverResult {
    match policy {
        FailoverPolicy::ImmediateReassign => FailoverResult {
            reassigned: orphaned_tasks,
            queued: vec![],
            failed: vec![],
        },
        FailoverPolicy::QueueAndWait => FailoverResult {
            reassigned: vec![],
            queued: orphaned_tasks,
            failed: vec![],
        },
        FailoverPolicy::FailToNeedsAttention => FailoverResult {
            reassigned: vec![],
            queued: vec![],
            failed: orphaned_tasks,
        },
    }
}

/// Result of a failover operation.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FailoverResult {
    pub reassigned: Vec<String>,
    pub queued: Vec<String>,
    pub failed: Vec<String>,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    trait AssignedExt {
        fn unwrap_assigned(self) -> String;
    }

    impl AssignedExt for AssignmentResult {
        fn unwrap_assigned(self) -> String {
            match self {
                AssignmentResult::Assigned { host_id } => host_id,
                _ => panic!("expected Assigned, got {self:?}"),
            }
        }
    }

    fn make_host(id: &str, max_conc: usize) -> WorkerHost {
        WorkerHost {
            id: id.into(),
            address: format!("worker-{id}.local"),
            max_concurrency: max_conc,
            env_revision: "v1".into(),
            labels: vec![],
        }
    }

    fn make_pool(hosts: &[(&str, usize)]) -> WorkerPool {
        let mut pool = WorkerPool::new(&PoolConfig::default());
        for (id, conc) in hosts {
            pool.register(make_host(id, *conc));
        }
        pool
    }

    // ---- registration ----

    #[test]
    fn register_and_deregister() {
        let mut pool = make_pool(&[("w1", 2)]);
        assert_eq!(pool.worker_ids(), vec!["w1"]);
        pool.deregister("w1");
        assert!(pool.worker_ids().is_empty());
    }

    #[test]
    fn reregister_preserves_active_assignments() {
        let mut pool = make_pool(&[("w1", 2)]);
        pool.assign("t1", 0).unwrap_assigned();
        pool.register(make_host("w1", 4));

        assert_eq!(pool.state("w1").unwrap().active_attempts, 1);
        assert_eq!(pool.total_capacity(), 3);
    }

    #[test]
    fn reregister_revives_an_offline_worker() {
        let mut pool = make_pool(&[("w1", 2)]);
        pool.handle_host_loss("w1");
        pool.register(make_host("w1", 2));

        assert_eq!(pool.state("w1").unwrap().health, WorkerHealth::Healthy);
    }

    #[test]
    fn active_worker_cannot_be_deregistered_silently() {
        let mut pool = make_pool(&[("w1", 1)]);
        pool.assign("t1", 0).unwrap_assigned();

        assert!(pool.deregister("w1").is_none());
        assert!(pool.state("w1").is_some());
    }

    // ---- assignment ----

    #[test]
    fn assign_to_healthy_worker() {
        let mut pool = make_pool(&[("w1", 2)]);
        let result = pool.assign("t1", 0);
        assert!(matches!(result, AssignmentResult::Assigned { host_id } if host_id == "w1"));
        assert_eq!(pool.state("w1").unwrap().active_attempts, 1);
    }

    #[test]
    fn assign_respects_concurrency_limit() {
        let mut pool = make_pool(&[("w1", 1)]);
        pool.assign("t1", 0).unwrap_assigned();
        let result = pool.assign("t2", 0);
        assert!(matches!(result, AssignmentResult::NoCapacity));
    }

    #[test]
    fn assigning_the_same_active_task_is_idempotent() {
        let mut pool = make_pool(&[("w1", 2)]);
        let first = pool.assign("t1", 0).unwrap_assigned();
        let second = pool.assign("t1", 0).unwrap_assigned();

        assert_eq!(first, second);
        assert_eq!(pool.state("w1").unwrap().active_attempts, 1);
        assert_eq!(pool.total_active(), 1);
    }

    #[test]
    fn assign_spreads_across_workers() {
        let mut pool = make_pool(&[("w1", 3), ("w2", 3)]);
        let r1 = pool.assign("t1", 0);
        let host1 = r1.unwrap_assigned();
        // Second task should go to the other (less-loaded) worker.
        let r2 = pool.assign("t2", 0);
        let host2 = r2.unwrap_assigned();
        assert_ne!(
            host2, host1,
            "second task should spread to different worker"
        );
    }

    #[test]
    fn assign_prefers_sticky_placement() {
        let mut pool = make_pool(&[("w1", 3), ("w2", 3)]);
        // Assign t1 to w1.
        let r1 = pool.assign("t1", 0);
        let host1 = r1.unwrap_assigned();
        // Release t1.
        pool.release(&host1, "t1", true);
        // Assign some other work to balance.
        pool.assign("t2", 0);
        // Re-assign t1 — should prefer sticky host.
        let r3 = pool.assign("t1", 0);
        let host3 = r3.unwrap_assigned();
        assert_eq!(host3, host1, "sticky placement should prefer original host");
    }

    #[test]
    fn preferred_label_breaks_equal_load_ties() {
        let config = PoolConfig {
            preferred_labels: vec!["gpu".into()],
            ..Default::default()
        };
        let mut pool = WorkerPool::new(&config);
        pool.register(make_host("cpu", 2));
        let mut gpu = make_host("gpu", 2);
        gpu.labels.push("gpu".into());
        pool.register(gpu);

        assert_eq!(pool.assign("t1", 0).unwrap_assigned(), "gpu");
    }

    #[test]
    fn no_workers_returns_no_workers() {
        let mut pool = WorkerPool::new(&PoolConfig::default());
        let result = pool.assign("t1", 0);
        assert!(matches!(result, AssignmentResult::NoWorkers));
    }

    #[test]
    fn all_offline_returns_no_capacity() {
        let mut pool = make_pool(&[("w1", 1)]);
        pool.handle_host_loss("w1");
        let result = pool.assign("t1", 0);
        assert!(matches!(result, AssignmentResult::NoCapacity));
    }

    // ---- release ----

    #[test]
    fn release_decrements_active() {
        let mut pool = make_pool(&[("w1", 2)]);
        pool.assign("t1", 0).unwrap_assigned();
        assert_eq!(pool.state("w1").unwrap().active_attempts, 1);
        pool.release("w1", "t1", true);
        assert_eq!(pool.state("w1").unwrap().active_attempts, 0);
        assert_eq!(pool.state("w1").unwrap().completed_attempts, 1);
    }

    #[test]
    fn release_failed_tracks_failures() {
        let mut pool = make_pool(&[("w1", 2)]);
        pool.assign("t1", 0).unwrap_assigned();
        pool.release("w1", "t1", false);
        assert_eq!(pool.state("w1").unwrap().failed_attempts, 1);
    }

    #[test]
    fn wrong_or_duplicate_release_is_a_noop() {
        let mut pool = make_pool(&[("w1", 2), ("w2", 2)]);
        let host = pool.assign("t1", 0).unwrap_assigned();
        let wrong_host = if host == "w1" { "w2" } else { "w1" };

        pool.release(wrong_host, "t1", true);
        assert_eq!(pool.total_active(), 1);
        pool.release(&host, "t1", true);
        pool.release(&host, "t1", true);
        assert_eq!(pool.total_active(), 0);
        assert_eq!(pool.state(&host).unwrap().completed_attempts, 1);
    }

    // ---- health checking ----

    #[test]
    fn expired_heartbeat_marks_unhealthy() {
        let mut pool = make_pool(&[("w1", 2)]);
        pool.heartbeat("w1", 1000);
        // After timeout (>120s) → unhealthy.
        let changed = pool.check_health(200_000);
        assert!(changed.contains(&"w1".to_string()));
        assert_eq!(pool.state("w1").unwrap().health, WorkerHealth::Unhealthy);
    }

    #[test]
    fn double_timeout_marks_offline() {
        let mut pool = make_pool(&[("w1", 2)]);
        pool.heartbeat("w1", 1000);
        let _ = pool.check_health(300_000);
        assert_eq!(pool.state("w1").unwrap().health, WorkerHealth::Offline);
    }

    #[test]
    fn heartbeat_revives_unhealthy() {
        let mut pool = make_pool(&[("w1", 2)]);
        pool.heartbeat("w1", 1000);
        let _ = pool.check_health(200_000);
        assert_eq!(pool.state("w1").unwrap().health, WorkerHealth::Unhealthy);
        pool.heartbeat("w1", 201_000);
        assert_eq!(pool.state("w1").unwrap().health, WorkerHealth::Healthy);
    }

    #[test]
    fn env_drift_cannot_revive_an_offline_worker() {
        let mut pool = make_pool(&[("w1", 2)]);
        pool.handle_host_loss("w1");
        pool.mark_env_drift("w1");

        assert_eq!(pool.state("w1").unwrap().health, WorkerHealth::Offline);
    }

    // ---- env drift ----

    #[test]
    fn env_drift_degrades_worker() {
        let mut pool = make_pool(&[("w1", 2)]);
        pool.mark_env_drift("w1");
        assert_eq!(pool.state("w1").unwrap().health, WorkerHealth::Degraded);
        assert!(pool.state("w1").unwrap().env_drift_detected);
    }

    #[test]
    fn clear_drift_restores_health() {
        let mut pool = make_pool(&[("w1", 2)]);
        pool.mark_env_drift("w1");
        pool.clear_env_drift("w1");
        assert_eq!(pool.state("w1").unwrap().health, WorkerHealth::Healthy);
        assert!(!pool.state("w1").unwrap().env_drift_detected);
    }

    // ---- host loss + failover ----

    #[test]
    fn host_loss_orphans_sticky_tasks() {
        let mut pool = make_pool(&[("w1", 2), ("w2", 2)]);
        let r = pool.assign("t1", 0);
        let host = r.unwrap_assigned();
        let orphaned = pool.handle_host_loss(&host);
        assert!(orphaned.contains(&"t1".to_string()));
        assert_eq!(pool.state(&host).unwrap().health, WorkerHealth::Offline);
        assert_eq!(pool.state(&host).unwrap().active_attempts, 0);
    }

    #[test]
    fn host_loss_orphans_only_active_tasks() {
        let mut pool = make_pool(&[("w1", 2)]);
        let host = pool.assign("completed", 0).unwrap_assigned();
        pool.release(&host, "completed", true);
        pool.assign("active", 0).unwrap_assigned();

        assert_eq!(pool.handle_host_loss(&host), vec!["active"]);
    }

    #[test]
    fn failover_immediate_reassign() {
        let result = execute_failover(
            vec!["t1".into(), "t2".into()],
            FailoverPolicy::ImmediateReassign,
        );
        assert_eq!(result.reassigned.len(), 2);
        assert!(result.queued.is_empty());
        assert!(result.failed.is_empty());
    }

    #[test]
    fn failover_queue_and_wait() {
        let result = execute_failover(vec!["t1".into()], FailoverPolicy::QueueAndWait);
        assert_eq!(result.queued.len(), 1);
    }

    #[test]
    fn failover_to_needs_attention() {
        let result = execute_failover(vec!["t1".into()], FailoverPolicy::FailToNeedsAttention);
        assert_eq!(result.failed.len(), 1);
    }

    // ---- capacity ----

    #[test]
    fn total_capacity_excludes_offline() {
        let mut pool = make_pool(&[("w1", 2), ("w2", 2)]);
        pool.handle_host_loss("w2");
        assert_eq!(pool.total_capacity(), 2);
    }

    #[test]
    fn total_capacity_accounts_for_active() {
        let mut pool = make_pool(&[("w1", 3)]);
        pool.assign("t1", 0).unwrap_assigned();
        assert_eq!(pool.total_capacity(), 2);
    }

    // ---- health counts ----

    #[test]
    fn health_counts_aggregate() {
        let mut pool = make_pool(&[("w1", 1), ("w2", 1), ("w3", 1)]);
        pool.mark_env_drift("w1"); // degraded
        pool.handle_host_loss("w2"); // offline
        let counts = pool.health_counts();
        assert_eq!(counts.healthy, 1);
        assert_eq!(counts.degraded, 1);
        assert_eq!(counts.offline, 1);
        assert_eq!(counts.total(), 3);
    }

    // ---- I3 acceptance ----

    #[test]
    fn loss_of_host_cannot_duplicate_active_attempt() {
        let mut pool = make_pool(&[("w1", 1), ("w2", 1)]);
        let r1 = pool.assign("t1", 0);
        let host = r1.unwrap_assigned();
        // The assigned host had t1.
        let _orphaned = pool.handle_host_loss(&host);
        // The assignment was revoked — host has 0 active.
        assert_eq!(pool.state(&host).unwrap().active_attempts, 0);
        // Now we can reassign t1 to the other host.
        let result = pool.assign("t1", 0);
        assert!(matches!(result, AssignmentResult::Assigned { .. }));
    }

    #[test]
    fn capacity_exhaustion_waits() {
        let mut pool = make_pool(&[("w1", 1)]);
        pool.assign("t1", 0).unwrap_assigned();
        // No capacity → returns NoCapacity (waits), not a silent mode change.
        let result = pool.assign("t2", 0);
        assert!(matches!(result, AssignmentResult::NoCapacity));
    }
}
