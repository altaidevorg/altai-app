//! Notifications and collaboration (plan §I4).
//!
//! Desktop notifications for approval, input, failure, and handoff.
//! Notifications link to the exact task/attempt. Replies are authenticated
//! and origin-bound — one reply cannot resume multiple runtimes.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Notification types
// ---------------------------------------------------------------------------

/// What triggered a notification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationTrigger {
    ApprovalRequired,
    InputRequired,
    TaskFailed,
    TaskCompleted,
    HandoffReady,
    BudgetWarning,
    NeedsAttention,
    SteeringRequired,
}

impl NotificationTrigger {
    pub fn label(self) -> &'static str {
        match self {
            Self::ApprovalRequired => "Approval required",
            Self::InputRequired => "Input required",
            Self::TaskFailed => "Task failed",
            Self::TaskCompleted => "Task completed",
            Self::HandoffReady => "Ready for handoff",
            Self::BudgetWarning => "Budget warning",
            Self::NeedsAttention => "Needs attention",
            Self::SteeringRequired => "Steering required",
        }
    }

    pub fn priority(self) -> NotificationPriority {
        match self {
            Self::ApprovalRequired | Self::InputRequired => NotificationPriority::Urgent,
            Self::TaskFailed | Self::NeedsAttention | Self::SteeringRequired => {
                NotificationPriority::High
            }
            Self::HandoffReady | Self::BudgetWarning => NotificationPriority::Normal,
            Self::TaskCompleted => NotificationPriority::Low,
        }
    }
}

/// How urgent a notification is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationPriority {
    Low,
    Normal,
    High,
    Urgent,
}

/// Where a notification is delivered.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationChannel {
    Desktop,
    Webhook,
    Email,
}

/// A notification to be delivered.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Notification {
    pub id: String,
    pub trigger: NotificationTrigger,
    pub task_id: String,
    pub attempt_id: Option<String>,
    pub title: String,
    pub body: String,
    pub priority: NotificationPriority,
    pub channel: NotificationChannel,
    pub created_at_ms: u64,
    pub action_url: Option<String>,
}

// ---------------------------------------------------------------------------
// Notification routing
// ---------------------------------------------------------------------------

/// Configuration for notification routing.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationConfig {
    pub enabled_channels: Vec<NotificationChannel>,
    pub suppressed_triggers: HashSet<NotificationTrigger>,
    /// Minimum priority to show (lower priorities are filtered).
    pub min_priority: Option<NotificationPriority>,
    /// Dedup window in ms — same trigger+task within this window is suppressed.
    pub dedup_window_ms: u64,
    /// Quiet hours: don't send notifications during these hours.
    pub quiet_hours: Option<QuietHours>,
}

/// Quiet hours configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuietHours {
    pub start_hour: u8,
    pub end_hour: u8,
}

impl QuietHours {
    pub fn is_quiet(&self, hour: u8) -> bool {
        if self.start_hour <= self.end_hour {
            hour >= self.start_hour && hour < self.end_hour
        } else {
            hour >= self.start_hour || hour < self.end_hour
        }
    }
}

/// Determine if a trigger should produce a notification given the config.
pub fn should_notify(
    trigger: NotificationTrigger,
    config: &NotificationConfig,
    now_hour: u8,
) -> bool {
    if config.suppressed_triggers.contains(&trigger) {
        return false;
    }
    if let Some(min) = config.min_priority {
        if trigger.priority() < min {
            return false;
        }
    }
    // Urgent notifications bypass quiet hours.
    if trigger.priority() < NotificationPriority::Urgent {
        if let Some(ref quiet) = config.quiet_hours {
            if quiet.is_quiet(now_hour) {
                return false;
            }
        }
    }
    true
}

// ---------------------------------------------------------------------------
// Notification dispatcher (dedup + queue)
// ---------------------------------------------------------------------------

/// Dispatches notifications with deduplication.
#[derive(Clone, Debug)]
pub struct NotificationDispatcher {
    config: NotificationConfig,
    /// (trigger, task_id, attempt_id) → last notification time, for dedup.
    last_sent: HashMap<(NotificationTrigger, String, Option<String>), u64>,
    queue: Vec<Notification>,
}

/// Result of a dispatch attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchResult {
    Queued,
    Suppressed,
    Deduplicated,
}

impl NotificationDispatcher {
    pub fn new(config: NotificationConfig) -> Self {
        Self {
            config,
            last_sent: HashMap::new(),
            queue: Vec::new(),
        }
    }

    /// Try to dispatch a notification. May be suppressed by config or deduped.
    #[allow(clippy::too_many_arguments)]
    pub fn dispatch(
        &mut self,
        trigger: NotificationTrigger,
        task_id: &str,
        attempt_id: Option<&str>,
        title: &str,
        body: &str,
        now_ms: u64,
        now_hour: u8,
    ) -> DispatchResult {
        // Config suppression.
        if !should_notify(trigger, &self.config, now_hour) {
            return DispatchResult::Suppressed;
        }

        // Keep only entries that can still participate in deduplication so
        // long-running dispatchers do not retain every task forever.
        self.last_sent
            .retain(|_, last| now_ms.saturating_sub(*last) < self.config.dedup_window_ms);

        // Dedup check. Attempts are isolated so a new attempt on the same task
        // cannot lose an actionable notification.
        let dedup_key = (trigger, task_id.to_string(), attempt_id.map(str::to_string));
        if let Some(&last) = self.last_sent.get(&dedup_key) {
            if now_ms.saturating_sub(last) < self.config.dedup_window_ms {
                return DispatchResult::Deduplicated;
            }
        }

        // Queue the notification.
        let notif = Notification {
            id: format!("notif-{now_ms}-{}", uuid::Uuid::new_v4()),
            trigger,
            task_id: task_id.to_string(),
            attempt_id: attempt_id.map(|s| s.to_string()),
            title: title.to_string(),
            body: body.to_string(),
            priority: trigger.priority(),
            channel: self
                .config
                .enabled_channels
                .first()
                .copied()
                .unwrap_or(NotificationChannel::Desktop),
            created_at_ms: now_ms,
            action_url: Some(format!("/task/{task_id}")),
        };

        self.queue.push(notif);
        self.last_sent.insert(dedup_key, now_ms);
        DispatchResult::Queued
    }

    /// Drain the queue (deliver all pending notifications).
    pub fn drain(&mut self) -> Vec<Notification> {
        std::mem::take(&mut self.queue)
    }

    /// Pending count.
    pub fn pending(&self) -> usize {
        self.queue.len()
    }
}

// ---------------------------------------------------------------------------
// Reply binding (authentication + origin isolation)
// ---------------------------------------------------------------------------

/// A bound reply token — ensures one reply resumes exactly one runtime.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplyToken {
    pub token: String,
    pub task_id: String,
    pub attempt_id: Option<String>,
    pub runtime_id: String,
    pub created_at_ms: u64,
    pub expires_at_ms: u64,
    pub used: bool,
}

/// Manages reply tokens — each can be used exactly once.
#[derive(Clone, Debug, Default)]
pub struct ReplyTokenStore {
    tokens: HashMap<String, ReplyToken>,
}

/// Error for reply token operations.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ReplyError {
    NotFound,
    AlreadyUsed,
    Expired,
    WrongRuntime { expected: String, got: String },
}

impl ReplyTokenStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Issue a new reply token bound to a specific task/attempt/runtime.
    pub fn issue(
        &mut self,
        task_id: &str,
        attempt_id: Option<&str>,
        runtime_id: &str,
        now_ms: u64,
        ttl_ms: u64,
    ) -> ReplyToken {
        let token = format!("rt-{now_ms}-{}", uuid::Uuid::new_v4());
        let reply = ReplyToken {
            token: token.clone(),
            task_id: task_id.to_string(),
            attempt_id: attempt_id.map(|s| s.to_string()),
            runtime_id: runtime_id.to_string(),
            created_at_ms: now_ms,
            expires_at_ms: now_ms.saturating_add(ttl_ms),
            used: false,
        };
        self.tokens.insert(token, reply.clone());
        reply
    }

    /// Consume a reply token. Validates it belongs to the right runtime,
    /// hasn't expired, and hasn't been used. Marks it as used on success.
    pub fn consume(
        &mut self,
        token: &str,
        runtime_id: &str,
        now_ms: u64,
    ) -> Result<ReplyToken, ReplyError> {
        let reply = self.tokens.get_mut(token).ok_or(ReplyError::NotFound)?;

        if reply.used {
            return Err(ReplyError::AlreadyUsed);
        }
        if now_ms >= reply.expires_at_ms {
            return Err(ReplyError::Expired);
        }
        if reply.runtime_id != runtime_id {
            return Err(ReplyError::WrongRuntime {
                expected: reply.runtime_id.clone(),
                got: runtime_id.to_string(),
            });
        }

        reply.used = true;
        Ok(reply.clone())
    }

    /// Check if a token is valid (not used, not expired, correct runtime).
    pub fn is_valid(&self, token: &str, runtime_id: &str, now_ms: u64) -> bool {
        match self.tokens.get(token) {
            Some(r) => !r.used && now_ms < r.expires_at_ms && r.runtime_id == runtime_id,
            None => false,
        }
    }
}

// ---------------------------------------------------------------------------
// Task comments / follow-ups
// ---------------------------------------------------------------------------

/// A task comment or user-to-agent follow-up.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskComment {
    pub id: String,
    pub task_id: String,
    pub author: CommentAuthor,
    pub body: String,
    pub created_at_ms: u64,
    pub reply_to: Option<String>,
}

/// Who wrote a comment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommentAuthor {
    User,
    Agent,
    System,
}

/// A thread of comments on a task.
#[derive(Clone, Debug, Default)]
pub struct CommentThread {
    comments: Vec<TaskComment>,
}

impl CommentThread {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, comment: TaskComment) {
        self.comments.push(comment);
    }

    pub fn for_task(&self, task_id: &str) -> Vec<&TaskComment> {
        self.comments
            .iter()
            .filter(|c| c.task_id == task_id)
            .collect()
    }

    pub fn replies_to(&self, comment_id: &str) -> Vec<&TaskComment> {
        self.comments
            .iter()
            .filter(|c| c.reply_to.as_deref() == Some(comment_id))
            .collect()
    }

    pub fn len(&self) -> usize {
        self.comments.len()
    }

    pub fn is_empty(&self) -> bool {
        self.comments.is_empty()
    }

    /// Get all comments in chronological order.
    pub fn all(&self) -> &[TaskComment] {
        &self.comments
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- trigger → priority mapping ----

    #[test]
    fn approval_and_input_are_urgent() {
        assert_eq!(
            NotificationTrigger::ApprovalRequired.priority(),
            NotificationPriority::Urgent
        );
        assert_eq!(
            NotificationTrigger::InputRequired.priority(),
            NotificationPriority::Urgent
        );
    }

    #[test]
    fn task_completed_is_low() {
        assert_eq!(
            NotificationTrigger::TaskCompleted.priority(),
            NotificationPriority::Low
        );
    }

    // ---- should_notify ----

    #[test]
    fn suppressed_trigger_filtered() {
        let config = NotificationConfig {
            suppressed_triggers: HashSet::from([NotificationTrigger::TaskCompleted]),
            ..Default::default()
        };
        assert!(!should_notify(
            NotificationTrigger::TaskCompleted,
            &config,
            12
        ));
        assert!(should_notify(NotificationTrigger::TaskFailed, &config, 12));
    }

    #[test]
    fn min_priority_filters_lower() {
        let config = NotificationConfig {
            min_priority: Some(NotificationPriority::High),
            ..Default::default()
        };
        assert!(!should_notify(
            NotificationTrigger::TaskCompleted,
            &config,
            12
        ));
        assert!(should_notify(NotificationTrigger::TaskFailed, &config, 12));
        assert!(should_notify(
            NotificationTrigger::ApprovalRequired,
            &config,
            12
        ));
    }

    #[test]
    fn quiet_hours_block_non_urgent() {
        let config = NotificationConfig {
            quiet_hours: Some(QuietHours {
                start_hour: 22,
                end_hour: 6,
            }),
            ..Default::default()
        };
        assert!(!should_notify(
            NotificationTrigger::TaskCompleted,
            &config,
            23
        ));
        assert!(should_notify(
            NotificationTrigger::ApprovalRequired,
            &config,
            23
        ));
    }

    #[test]
    fn no_quiet_hours_allows_all() {
        let config = NotificationConfig::default();
        assert!(should_notify(
            NotificationTrigger::TaskCompleted,
            &config,
            3
        ));
    }

    // ---- dispatcher dedup ----

    #[test]
    fn dispatch_queues_notification() {
        let mut dispatcher = NotificationDispatcher::new(NotificationConfig {
            enabled_channels: vec![NotificationChannel::Desktop],
            ..Default::default()
        });
        let result = dispatcher.dispatch(
            NotificationTrigger::TaskFailed,
            "t1",
            Some("a1"),
            "Task failed",
            "Error: OOM",
            1000,
            12,
        );
        assert_eq!(result, DispatchResult::Queued);
        assert_eq!(dispatcher.pending(), 1);
    }

    #[test]
    fn dispatch_deduplicates_within_window() {
        let mut dispatcher = NotificationDispatcher::new(NotificationConfig {
            dedup_window_ms: 5000,
            ..Default::default()
        });
        dispatcher.dispatch(
            NotificationTrigger::TaskFailed,
            "t1",
            None,
            "A",
            "",
            1000,
            12,
        );
        let result = dispatcher.dispatch(
            NotificationTrigger::TaskFailed,
            "t1",
            None,
            "B",
            "",
            3000,
            12,
        );
        assert_eq!(result, DispatchResult::Deduplicated);
        assert_eq!(
            dispatcher.pending(),
            1,
            "dedup should not add a second notification"
        );
    }

    #[test]
    fn dispatch_allows_after_dedup_window() {
        let mut dispatcher = NotificationDispatcher::new(NotificationConfig {
            dedup_window_ms: 5000,
            ..Default::default()
        });
        dispatcher.dispatch(
            NotificationTrigger::TaskFailed,
            "t1",
            None,
            "A",
            "",
            1000,
            12,
        );
        let result = dispatcher.dispatch(
            NotificationTrigger::TaskFailed,
            "t1",
            None,
            "B",
            "",
            6001,
            12,
        );
        assert_eq!(result, DispatchResult::Queued);
        assert_eq!(dispatcher.pending(), 2);
    }

    #[test]
    fn dispatch_different_tasks_not_deduplicated() {
        let mut dispatcher = NotificationDispatcher::new(NotificationConfig {
            dedup_window_ms: 5000,
            ..Default::default()
        });
        dispatcher.dispatch(
            NotificationTrigger::TaskFailed,
            "t1",
            None,
            "A",
            "",
            1000,
            12,
        );
        let result = dispatcher.dispatch(
            NotificationTrigger::TaskFailed,
            "t2",
            None,
            "B",
            "",
            1000,
            12,
        );
        assert_eq!(result, DispatchResult::Queued);
    }

    #[test]
    fn dispatch_different_attempts_not_deduplicated() {
        let mut dispatcher = NotificationDispatcher::new(NotificationConfig {
            dedup_window_ms: 5000,
            ..Default::default()
        });
        dispatcher.dispatch(
            NotificationTrigger::ApprovalRequired,
            "t1",
            Some("attempt-1"),
            "First",
            "",
            1000,
            12,
        );
        let result = dispatcher.dispatch(
            NotificationTrigger::ApprovalRequired,
            "t1",
            Some("attempt-2"),
            "Second",
            "",
            2000,
            12,
        );

        assert_eq!(result, DispatchResult::Queued);
        assert_eq!(dispatcher.pending(), 2);
    }

    #[test]
    fn dispatch_suppressed_by_config() {
        let mut dispatcher = NotificationDispatcher::new(NotificationConfig {
            suppressed_triggers: HashSet::from([NotificationTrigger::TaskCompleted]),
            ..Default::default()
        });
        let result = dispatcher.dispatch(
            NotificationTrigger::TaskCompleted,
            "t1",
            None,
            "Done",
            "",
            1000,
            12,
        );
        assert_eq!(result, DispatchResult::Suppressed);
        assert_eq!(dispatcher.pending(), 0);
    }

    #[test]
    fn drain_returns_all_pending() {
        let mut dispatcher = NotificationDispatcher::new(NotificationConfig::default());
        dispatcher.dispatch(
            NotificationTrigger::TaskFailed,
            "t1",
            None,
            "A",
            "",
            1000,
            12,
        );
        dispatcher.dispatch(
            NotificationTrigger::HandoffReady,
            "t2",
            None,
            "B",
            "",
            2000,
            12,
        );
        let notifs = dispatcher.drain();
        assert_eq!(notifs.len(), 2);
        assert_eq!(dispatcher.pending(), 0);
    }

    #[test]
    fn notification_ids_remain_unique_after_drain() {
        let mut dispatcher = NotificationDispatcher::new(NotificationConfig::default());
        dispatcher.dispatch(
            NotificationTrigger::TaskFailed,
            "t1",
            None,
            "First",
            "",
            1000,
            12,
        );
        let first_id = dispatcher.drain()[0].id.clone();
        dispatcher.dispatch(
            NotificationTrigger::TaskFailed,
            "t2",
            None,
            "Second",
            "",
            1000,
            12,
        );
        let second_id = dispatcher.drain()[0].id.clone();

        assert_ne!(first_id, second_id);
    }

    #[test]
    fn dedup_window_math_does_not_overflow() {
        let mut dispatcher = NotificationDispatcher::new(NotificationConfig {
            dedup_window_ms: 10,
            ..Default::default()
        });
        dispatcher.dispatch(
            NotificationTrigger::TaskFailed,
            "t1",
            None,
            "First",
            "",
            u64::MAX - 5,
            12,
        );
        let result = dispatcher.dispatch(
            NotificationTrigger::TaskFailed,
            "t1",
            None,
            "Second",
            "",
            u64::MAX,
            12,
        );

        assert_eq!(result, DispatchResult::Deduplicated);
    }

    #[test]
    fn notification_links_to_task_and_attempt() {
        let mut dispatcher = NotificationDispatcher::new(NotificationConfig::default());
        dispatcher.dispatch(
            NotificationTrigger::ApprovalRequired,
            "task-42",
            Some("att-7"),
            "Need approval",
            "",
            1000,
            12,
        );
        let notifs = dispatcher.drain();
        assert_eq!(notifs[0].task_id, "task-42");
        assert_eq!(notifs[0].attempt_id.as_deref(), Some("att-7"));
        assert!(notifs[0].action_url.as_ref().unwrap().contains("task-42"));
    }

    // ---- reply tokens ----

    #[test]
    fn issue_and_consume_token() {
        let mut store = ReplyTokenStore::new();
        let token = store.issue("t1", Some("a1"), "runtime-1", 1000, 60_000);

        let result = store.consume(&token.token, "runtime-1", 2000);
        assert!(result.is_ok());
    }

    #[test]
    fn token_single_use() {
        let mut store = ReplyTokenStore::new();
        let token = store.issue("t1", None, "rt-1", 1000, 60_000);

        store.consume(&token.token, "rt-1", 2000).unwrap();
        let result = store.consume(&token.token, "rt-1", 3000);
        assert_eq!(result.unwrap_err(), ReplyError::AlreadyUsed);
    }

    #[test]
    fn token_expired() {
        let mut store = ReplyTokenStore::new();
        let token = store.issue("t1", None, "rt-1", 1000, 60_000);

        let result = store.consume(&token.token, "rt-1", 70_000);
        assert_eq!(result.unwrap_err(), ReplyError::Expired);
    }

    #[test]
    fn token_expires_at_the_exact_deadline() {
        let mut store = ReplyTokenStore::new();
        let token = store.issue("t1", None, "rt-1", 1000, 60_000);

        assert!(!store.is_valid(&token.token, "rt-1", 61_000));
        assert_eq!(
            store.consume(&token.token, "rt-1", 61_000).unwrap_err(),
            ReplyError::Expired
        );
    }

    #[test]
    fn token_expiry_saturates_and_tokens_are_unique() {
        let mut store = ReplyTokenStore::new();
        let first = store.issue("t1", None, "rt-1", u64::MAX - 5, 10);
        let second = store.issue("t1", None, "rt-1", u64::MAX - 5, 10);

        assert_eq!(first.expires_at_ms, u64::MAX);
        assert_ne!(first.token, second.token);
        assert!(store.is_valid(&first.token, "rt-1", u64::MAX - 1));
    }

    #[test]
    fn token_wrong_runtime_rejected() {
        let mut store = ReplyTokenStore::new();
        let token = store.issue("t1", None, "rt-1", 1000, 60_000);

        let result = store.consume(&token.token, "rt-2", 2000);
        assert!(matches!(
            result.unwrap_err(),
            ReplyError::WrongRuntime { .. }
        ));
    }

    #[test]
    fn token_not_found() {
        let mut store = ReplyTokenStore::new();
        let result = store.consume("nonexistent", "rt-1", 1000);
        assert_eq!(result.unwrap_err(), ReplyError::NotFound);
    }

    #[test]
    fn is_valid_checks_all_conditions() {
        let mut store = ReplyTokenStore::new();
        let token = store.issue("t1", None, "rt-1", 1000, 60_000);

        assert!(store.is_valid(&token.token, "rt-1", 2000));
        assert!(!store.is_valid(&token.token, "rt-2", 2000));
        assert!(!store.is_valid(&token.token, "rt-1", 70_000));

        store.consume(&token.token, "rt-1", 2000).unwrap();
        assert!(
            !store.is_valid(&token.token, "rt-1", 2000),
            "used token is invalid"
        );
    }

    // ---- comments ----

    #[test]
    fn comment_thread_add_and_query() {
        let mut thread = CommentThread::new();
        thread.add(TaskComment {
            id: "c1".into(),
            task_id: "t1".into(),
            author: CommentAuthor::User,
            body: "Please fix the auth".into(),
            created_at_ms: 1000,
            reply_to: None,
        });
        thread.add(TaskComment {
            id: "c2".into(),
            task_id: "t1".into(),
            author: CommentAuthor::Agent,
            body: "Fixed in commit abc".into(),
            created_at_ms: 2000,
            reply_to: Some("c1".into()),
        });
        thread.add(TaskComment {
            id: "c3".into(),
            task_id: "t2".into(),
            author: CommentAuthor::System,
            body: "Task created".into(),
            created_at_ms: 3000,
            reply_to: None,
        });

        assert_eq!(thread.len(), 3);
        assert_eq!(thread.for_task("t1").len(), 2);
        assert_eq!(thread.for_task("t2").len(), 1);
        assert_eq!(thread.replies_to("c1").len(), 1);
    }

    #[test]
    fn empty_thread() {
        let thread = CommentThread::new();
        assert!(thread.is_empty());
        assert_eq!(thread.len(), 0);
    }

    // ---- I4 acceptance ----

    #[test]
    fn notification_links_to_exact_task_attempt() {
        let mut dispatcher = NotificationDispatcher::new(NotificationConfig::default());
        dispatcher.dispatch(
            NotificationTrigger::ApprovalRequired,
            "task-99",
            Some("attempt-3"),
            "Approve merge",
            "",
            1000,
            12,
        );
        let notif = &dispatcher.drain()[0];
        assert_eq!(notif.task_id, "task-99");
        assert_eq!(notif.attempt_id.as_deref(), Some("attempt-3"));
    }

    #[test]
    fn one_reply_cannot_resume_multiple_runtimes() {
        let mut store = ReplyTokenStore::new();
        let token = store.issue("t1", None, "rt-1", 1000, 60_000);

        // First runtime consumes successfully.
        assert!(store.consume(&token.token, "rt-1", 2000).is_ok());
        // Second runtime cannot use the same token.
        assert!(store.consume(&token.token, "rt-2", 3000).is_err());
    }

    #[test]
    fn urgent_notifications_bypass_quiet_hours() {
        let config = NotificationConfig {
            quiet_hours: Some(QuietHours {
                start_hour: 22,
                end_hour: 8,
            }),
            ..Default::default()
        };
        // ApprovalRequired is Urgent → bypasses quiet hours.
        assert!(should_notify(
            NotificationTrigger::ApprovalRequired,
            &config,
            23
        ));
        // TaskCompleted is Low → blocked by quiet hours.
        assert!(!should_notify(
            NotificationTrigger::TaskCompleted,
            &config,
            23
        ));
    }
}
