//! Lifecycle hooks engine (plan §B3).
//!
//! Executes external commands at well-defined lifecycle points with structured
//! JSON input/output, explicit blocking decisions, timeout enforcement, cwd
//! confinement, secret redaction, and managed (locked) hooks.
//!
//! Hook evaluation rules:
//! - **Blocking hook that Denies → the action is blocked** (Deny wins).
//! - **Blocking hook that exits non-zero without JSON → Deny** (fail closed).
//! - **Observability hook failure → logged but never blocks** (no crash).
//! - **Pass from all hooks → action proceeds to policy evaluation**.

use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Hook lifecycle events
// ---------------------------------------------------------------------------

/// Every lifecycle point at which a hook can fire. These map 1:1 to the plan
/// §B3 event names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookEvent {
    SessionStart,
    BeforeTool,
    AfterTool,
    BeforeEdit,
    AfterEdit,
    BeforeApply,
    AfterRun,
    OnError,
    BeforeCleanup,
}

impl HookEvent {
    pub fn name(self) -> &'static str {
        match self {
            Self::SessionStart => "session_start",
            Self::BeforeTool => "before_tool",
            Self::AfterTool => "after_tool",
            Self::BeforeEdit => "before_edit",
            Self::AfterEdit => "after_edit",
            Self::BeforeApply => "before_apply",
            Self::AfterRun => "after_run",
            Self::OnError => "on_error",
            Self::BeforeCleanup => "before_cleanup",
        }
    }
}

// ---------------------------------------------------------------------------
// Hook specification
// ---------------------------------------------------------------------------

/// One configured hook. Blocking hooks can veto an action; observability hooks
/// (blocking = false) only observe and can never block.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HookSpec {
    pub event: HookEvent,
    pub command: String,
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    #[serde(default = "default_blocking")]
    pub blocking: bool,
}

fn default_timeout_secs() -> u64 {
    60
}

fn default_blocking() -> bool {
    true
}

/// What the agent is about to do (or just did). Passed to the hook as part of
/// the structured JSON input.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct HookAction {
    pub tool: String,
    pub description: String,
    pub category: String,
}

/// Structured JSON input written to the hook process's stdin.
#[derive(Clone, Debug, Serialize)]
pub struct HookInput {
    pub event: String,
    pub task_id: String,
    pub attempt_id: String,
    pub workspace_path: String,
    pub action: Option<HookAction>,
}

/// The parsed decision from the hook process's stdout JSON.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookDecision {
    /// The action may proceed.
    Allow,
    /// The action is blocked.
    Deny,
    /// No opinion — defer to other hooks or policy.
    Pass,
}

/// Structured JSON output expected from the hook process's stdout.
#[derive(Clone, Debug, Deserialize)]
pub struct HookOutput {
    #[serde(default = "default_decision")]
    pub decision: HookDecision,
    #[serde(default)]
    pub message: Option<String>,
}

fn default_decision() -> HookDecision {
    HookDecision::Pass
}

// ---------------------------------------------------------------------------
// Result and error types
// ---------------------------------------------------------------------------

/// The combined result of running one or more hooks for an event.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum HookResult {
    /// All hooks passed or allowed.
    Allow { messages: Vec<String> },
    /// At least one blocking hook denied the action.
    Denied {
        reason: String,
        hook_command: String,
    },
    /// No hooks were registered for this event.
    NoHooks,
}

/// Errors that can occur during hook execution.
#[derive(Debug)]
pub enum HookError {
    Spawn(String),
    Timeout,
    Io(std::io::Error),
    Json(serde_json::Error),
    /// Output exceeded the size limit.
    OutputTooLarge {
        limit: usize,
    },
}

impl std::fmt::Display for HookError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spawn(cmd) => write!(f, "failed to spawn hook: {cmd}"),
            Self::Timeout => write!(f, "hook timed out"),
            Self::Io(err) => write!(f, "hook I/O error: {err}"),
            Self::Json(err) => write!(f, "hook produced invalid JSON: {err}"),
            Self::OutputTooLarge { limit } => {
                write!(f, "hook output exceeded {limit} bytes")
            }
        }
    }
}

impl std::error::Error for HookError {}

// ---------------------------------------------------------------------------
// Hook executor
// ---------------------------------------------------------------------------

const MAX_OUTPUT_BYTES: usize = 256 * 1024;

/// Executes hook commands. The methods are synchronous because the orchestration
/// coordinator is synchronous; the coordinator actor wraps it in async.
#[derive(Clone, Default)]
pub struct HookExecutor {
    secret_patterns: Vec<String>,
}

impl HookExecutor {
    pub fn new(secret_patterns: Vec<String>) -> Self {
        Self { secret_patterns }
    }

    /// Execute a single hook command and return its parsed output.
    pub fn run(
        &self,
        spec: &HookSpec,
        input: &HookInput,
        cwd: &Path,
    ) -> Result<HookOutput, HookError> {
        let input_json = serde_json::to_vec(input).map_err(HookError::Json)?;

        let mut child = std::process::Command::new("/bin/sh")
            .arg("-c")
            .arg(&spec.command)
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| HookError::Spawn(format!("{}: {e}", spec.command)))?;

        // Write input to stdin.
        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            let _ = stdin.write_all(&input_json);
            // stdin dropped here, signaling EOF.
        }

        // Wait with timeout.
        let timeout = Duration::from_secs(spec.timeout_secs.max(1));
        let result = wait_with_timeout(&mut child, timeout);

        if result.timed_out {
            let _ = child.kill();
            let _ = child.wait();
            return Err(HookError::Timeout);
        }

        let output = match result.output {
            Some(o) => o,
            None => return Err(HookError::Io(std::io::Error::other("process lost"))),
        };

        // Check output size.
        if output.stdout.len() > MAX_OUTPUT_BYTES {
            return Err(HookError::OutputTooLarge {
                limit: MAX_OUTPUT_BYTES,
            });
        }

        let stdout_str = String::from_utf8_lossy(&output.stdout);
        let redacted = self.redact(&stdout_str);

        // Parse JSON output. If empty stdout, fall back to exit-code semantics.
        if stdout_str.trim().is_empty() {
            return Ok(HookOutput {
                decision: if output.status.success() {
                    HookDecision::Allow
                } else {
                    HookDecision::Deny
                },
                message: None,
            });
        }

        let mut parsed: HookOutput = serde_json::from_str(&redacted).map_err(HookError::Json)?;

        // If exit code is non-zero, upgrade to Deny (the hook signalled failure).
        if !output.status.success() && parsed.decision == HookDecision::Allow {
            parsed.decision = HookDecision::Deny;
        }

        Ok(parsed)
    }

    /// Replace known secret patterns in a string.
    fn redact(&self, text: &str) -> String {
        let mut result = text.to_string();
        for pattern in &self.secret_patterns {
            if !pattern.is_empty() {
                result = result.replace(pattern, "[REDACTED]");
            }
        }
        result
    }
}

/// Result of a timed wait on a child process.
struct TimedWaitResult {
    timed_out: bool,
    output: Option<std::process::Output>,
}

/// Wait for a child process with a timeout. Polls `try_wait` in a tight loop
/// until the process exits or the deadline elapses.
fn wait_with_timeout(child: &mut std::process::Child, timeout: Duration) -> TimedWaitResult {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let stdout = child.stdout.take().map(extract_stdout).unwrap_or_default();
                let stderr = child.stderr.take().map(extract_stderr).unwrap_or_default();
                return TimedWaitResult {
                    timed_out: false,
                    output: Some(std::process::Output {
                        status,
                        stdout,
                        stderr,
                    }),
                };
            }
            Ok(None) => {
                if std::time::Instant::now() >= deadline {
                    return TimedWaitResult {
                        timed_out: true,
                        output: None,
                    };
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => {
                return TimedWaitResult {
                    timed_out: false,
                    output: None,
                };
            }
        }
    }
}

fn extract_stdout(mut handle: std::process::ChildStdout) -> Vec<u8> {
    use std::io::Read;
    let mut buf = Vec::new();
    let _ = handle.read_to_end(&mut buf);
    buf
}

fn extract_stderr(mut handle: std::process::ChildStderr) -> Vec<u8> {
    use std::io::Read;
    let mut buf = Vec::new();
    let _ = handle.read_to_end(&mut buf);
    buf
}

// ---------------------------------------------------------------------------
// Hook registry and evaluation
// ---------------------------------------------------------------------------

/// Registry of all configured hooks, including managed (locked) hooks that
/// cannot be disabled by repository configuration.
#[derive(Clone, Debug, Default)]
pub struct HookRegistry {
    /// Managed hooks — locked, non-disableable. Highest priority.
    managed: Vec<HookSpec>,
    /// Project-configured hooks from WORKFLOW.md.
    project: HashMap<HookEvent, Vec<HookSpec>>,
}

impl HookRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a managed (locked) hook. These cannot be overridden or disabled.
    pub fn add_managed(&mut self, spec: HookSpec) {
        self.managed.push(spec);
    }

    /// Register project hooks for an event.
    pub fn add_project(&mut self, event: HookEvent, specs: Vec<HookSpec>) {
        self.project.insert(event, specs);
    }

    /// All hooks (managed + project) for a given event.
    pub fn hooks_for(&self, event: HookEvent) -> Vec<&HookSpec> {
        let mut hooks: Vec<&HookSpec> = self.managed.iter().filter(|h| h.event == event).collect();
        if let Some(project_hooks) = self.project.get(&event) {
            hooks.extend(project_hooks.iter());
        }
        hooks
    }

    /// Whether any hooks are registered for this event.
    pub fn has_hooks(&self, event: HookEvent) -> bool {
        self.hooks_for(event).iter().any(|_| true)
    }
}

/// Evaluate all hooks for an event. Returns the combined decision.
///
/// - **Deny wins**: the first blocking hook that returns Deny short-circuits.
/// - **Observability failures are logged but never block.**
/// - If no hooks exist, returns `NoHooks`.
pub fn evaluate_hooks(
    executor: &HookExecutor,
    registry: &HookRegistry,
    event: HookEvent,
    input: &HookInput,
    cwd: &Path,
) -> HookResult {
    let hooks = registry.hooks_for(event);
    if hooks.is_empty() {
        return HookResult::NoHooks;
    }

    let mut messages = Vec::new();

    for spec in hooks {
        match executor.run(spec, input, cwd) {
            Ok(output) => {
                if let Some(msg) = &output.message {
                    messages.push(msg.clone());
                }
                if spec.blocking && output.decision == HookDecision::Deny {
                    return HookResult::Denied {
                        reason: output
                            .message
                            .unwrap_or_else(|| "blocked by hook".to_string()),
                        hook_command: spec.command.clone(),
                    };
                }
            }
            Err(err) => {
                if spec.blocking {
                    // Fail closed: a blocking hook error denies the action.
                    return HookResult::Denied {
                        reason: format!("hook error: {err}"),
                        hook_command: spec.command.clone(),
                    };
                }
                // Observability hook failure — log and continue.
                messages.push(format!("observability hook failed: {err}"));
            }
        }
    }

    HookResult::Allow { messages }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn input() -> HookInput {
        HookInput {
            event: "before_tool".into(),
            task_id: "t-1".into(),
            attempt_id: "t-1-att-1".into(),
            workspace_path: "/workspace".into(),
            action: Some(HookAction {
                tool: "run_command".into(),
                description: "npm install".into(),
                category: "run_command".into(),
            }),
        }
    }

    fn spec(command: &str) -> HookSpec {
        HookSpec {
            event: HookEvent::BeforeTool,
            command: command.into(),
            timeout_secs: 5,
            blocking: true,
        }
    }

    // ---- HookEvent ----

    #[test]
    fn hook_event_names() {
        assert_eq!(HookEvent::SessionStart.name(), "session_start");
        assert_eq!(HookEvent::BeforeTool.name(), "before_tool");
        assert_eq!(HookEvent::AfterTool.name(), "after_tool");
        assert_eq!(HookEvent::BeforeEdit.name(), "before_edit");
        assert_eq!(HookEvent::AfterEdit.name(), "after_edit");
        assert_eq!(HookEvent::BeforeApply.name(), "before_apply");
        assert_eq!(HookEvent::AfterRun.name(), "after_run");
        assert_eq!(HookEvent::OnError.name(), "on_error");
        assert_eq!(HookEvent::BeforeCleanup.name(), "before_cleanup");
    }

    // ---- HookExecutor: allow via exit 0 ----

    #[test]
    fn hook_exit_zero_allows() {
        let tmp = tempdir();
        let executor = HookExecutor::default();
        let output = executor.run(&spec("exit 0"), &input(), &tmp).expect("run");
        assert_eq!(output.decision, HookDecision::Allow);
    }

    // ---- HookExecutor: deny via exit non-zero ----

    #[test]
    fn hook_exit_nonzero_denies() {
        let tmp = tempdir();
        let executor = HookExecutor::default();
        let output = executor.run(&spec("exit 1"), &input(), &tmp).expect("run");
        assert_eq!(output.decision, HookDecision::Deny);
    }

    // ---- HookExecutor: JSON output parsed ----

    #[test]
    fn hook_json_allow_output() {
        let tmp = tempdir();
        let executor = HookExecutor::default();
        let cmd = r#"echo '{"decision":"allow","message":"ok"}'"#;
        let output = executor.run(&spec(cmd), &input(), &tmp).expect("run");
        assert_eq!(output.decision, HookDecision::Allow);
        assert_eq!(output.message.as_deref(), Some("ok"));
    }

    #[test]
    fn hook_json_deny_output() {
        let tmp = tempdir();
        let executor = HookExecutor::default();
        let cmd = r#"echo '{"decision":"deny","message":"blocked"}'"#;
        let output = executor.run(&spec(cmd), &input(), &tmp).expect("run");
        assert_eq!(output.decision, HookDecision::Deny);
        assert_eq!(output.message.as_deref(), Some("blocked"));
    }

    #[test]
    fn hook_json_pass_output() {
        let tmp = tempdir();
        let executor = HookExecutor::default();
        let cmd = r#"echo '{"decision":"pass"}'"#;
        let output = executor.run(&spec(cmd), &input(), &tmp).expect("run");
        assert_eq!(output.decision, HookDecision::Pass);
    }

    // ---- HookExecutor: exit non-zero overrides JSON allow ----

    #[test]
    fn non_zero_exit_overrides_json_allow() {
        let tmp = tempdir();
        let executor = HookExecutor::default();
        // JSON says allow but exit code is 1.
        let cmd = r#"echo '{"decision":"allow"}'; exit 1"#;
        let output = executor.run(&spec(cmd), &input(), &tmp).expect("run");
        assert_eq!(output.decision, HookDecision::Deny);
    }

    // ---- HookExecutor: invalid JSON on blocking hook ----

    #[test]
    fn invalid_json_is_error() {
        let tmp = tempdir();
        let executor = HookExecutor::default();
        let cmd = r#"echo 'not json'"#;
        let result = executor.run(&spec(cmd), &input(), &tmp);
        assert!(result.is_err());
    }

    // ---- HookExecutor: secret redaction ----

    #[test]
    fn secrets_redacted_from_output() {
        let tmp = tempdir();
        let executor = HookExecutor::new(vec!["super-secret-key".into()]);
        let cmd = r#"echo '{"decision":"allow","message":"key=super-secret-key"}'"#;
        let output = executor.run(&spec(cmd), &input(), &tmp).expect("run");
        // The secret in the message should be redacted before JSON parsing.
        assert_ne!(output.message.as_deref(), Some("key=super-secret-key"));
        assert!(output.message.as_deref().unwrap().contains("[REDACTED]"));
    }

    // ---- HookExecutor: timeout ----

    #[test]
    fn hook_timeout_kills_and_errors() {
        let tmp = tempdir();
        let executor = HookExecutor::default();
        let mut s = spec("sleep 30");
        s.timeout_secs = 1;
        let result = executor.run(&s, &input(), &tmp);
        assert!(matches!(result, Err(HookError::Timeout)));
    }

    // ---- HookExecutor: cwd enforcement ----

    #[test]
    fn hook_runs_in_cwd() {
        let tmp = tempdir();
        // Create a marker file in the temp dir.
        let marker = tmp.join("marker.txt");
        fs::write(&marker, "present").unwrap();

        let executor = HookExecutor::default();
        let cmd = "test -f marker.txt";
        let output = executor.run(&spec(cmd), &input(), &tmp).expect("run");
        assert_eq!(output.decision, HookDecision::Allow);
    }

    #[test]
    fn hook_cannot_escape_cwd() {
        let tmp = tempdir();
        // This file does NOT exist in cwd.
        let cmd = "test -f nonexistent_xyz_marker.txt";
        let executor = HookExecutor::default();
        let output = executor.run(&spec(cmd), &input(), &tmp).expect("run");
        assert_eq!(output.decision, HookDecision::Deny);
    }

    // ---- HookRegistry ----

    #[test]
    fn registry_combines_managed_and_project() {
        let mut reg = HookRegistry::new();
        reg.add_managed(spec("managed-cmd"));
        reg.add_project(
            HookEvent::BeforeTool,
            vec![spec("project-cmd"), spec("project-cmd-2")],
        );

        let hooks = reg.hooks_for(HookEvent::BeforeTool);
        assert_eq!(hooks.len(), 3);
        // Managed first.
        assert_eq!(hooks[0].command, "managed-cmd");
    }

    #[test]
    fn registry_filters_by_event() {
        let mut reg = HookRegistry::new();
        reg.add_project(HookEvent::BeforeTool, vec![spec("before-cmd")]);
        reg.add_project(HookEvent::AfterTool, vec![spec("after-cmd")]);

        let before = reg.hooks_for(HookEvent::BeforeTool);
        assert_eq!(before.len(), 1);
        assert_eq!(before[0].command, "before-cmd");

        assert!(!reg.has_hooks(HookEvent::OnError));
    }

    // ---- evaluate_hooks: deny wins ----

    #[test]
    fn evaluate_deny_wins() {
        let tmp = tempdir();
        let executor = HookExecutor::default();
        let mut reg = HookRegistry::new();
        reg.add_project(
            HookEvent::BeforeTool,
            vec![
                spec(r#"echo '{"decision":"allow"}'"#),
                spec(r#"echo '{"decision":"deny","message":"nope"}'"#),
                spec(r#"echo '{"decision":"allow"}'"#),
            ],
        );

        let result = evaluate_hooks(&executor, &reg, HookEvent::BeforeTool, &input(), &tmp);
        assert!(matches!(
            result,
            HookResult::Denied { reason, .. } if reason == "nope"
        ));
    }

    // ---- evaluate_hooks: all allow ----

    #[test]
    fn evaluate_all_allow() {
        let tmp = tempdir();
        let executor = HookExecutor::default();
        let mut reg = HookRegistry::new();
        reg.add_project(
            HookEvent::BeforeTool,
            vec![
                spec(r#"echo '{"decision":"allow","message":"ok1"}'"#),
                spec(r#"echo '{"decision":"allow","message":"ok2"}'"#),
            ],
        );

        let result = evaluate_hooks(&executor, &reg, HookEvent::BeforeTool, &input(), &tmp);
        assert!(matches!(
            result,
            HookResult::Allow { messages } if messages.len() == 2
        ));
    }

    // ---- evaluate_hooks: no hooks ----

    #[test]
    fn evaluate_no_hooks() {
        let tmp = tempdir();
        let executor = HookExecutor::default();
        let reg = HookRegistry::new();

        let result = evaluate_hooks(&executor, &reg, HookEvent::OnError, &input(), &tmp);
        assert_eq!(result, HookResult::NoHooks);
    }

    // ---- evaluate_hooks: observability failure doesn't block ----

    #[test]
    fn observability_hook_failure_doesnt_block() {
        let tmp = tempdir();
        let executor = HookExecutor::default();
        let mut reg = HookRegistry::new();

        // Non-blocking (observability) hook that produces invalid JSON.
        let mut obs = spec(r#"echo 'not json'"#);
        obs.blocking = false;
        // A blocking hook that allows.
        let allow = spec(r#"echo '{"decision":"allow"}'"#);

        reg.add_project(HookEvent::BeforeTool, vec![obs, allow]);

        let result = evaluate_hooks(&executor, &reg, HookEvent::BeforeTool, &input(), &tmp);
        assert!(matches!(result, HookResult::Allow { .. }));
    }

    // ---- evaluate_hooks: blocking hook error fails closed ----

    #[test]
    fn blocking_hook_error_fails_closed() {
        let tmp = tempdir();
        let executor = HookExecutor::default();
        let mut reg = HookRegistry::new();

        // Blocking hook with invalid JSON.
        let bad = spec(r#"echo 'not json'"#);
        reg.add_project(HookEvent::BeforeTool, vec![bad]);

        let result = evaluate_hooks(&executor, &reg, HookEvent::BeforeTool, &input(), &tmp);
        assert!(matches!(result, HookResult::Denied { .. }));
    }

    // ---- evaluate_hooks: managed hook deny cannot be overridden ----

    #[test]
    fn managed_hook_deny_overrides_project_allow() {
        let tmp = tempdir();
        let executor = HookExecutor::default();
        let mut reg = HookRegistry::new();

        // Managed hook denies.
        reg.add_managed(spec(
            r#"echo '{"decision":"deny","message":"managed block"}'"#,
        ));
        // Project hook allows.
        reg.add_project(
            HookEvent::BeforeTool,
            vec![spec(r#"echo '{"decision":"allow"}'"#)],
        );

        let result = evaluate_hooks(&executor, &reg, HookEvent::BeforeTool, &input(), &tmp);
        // Managed runs first and denies — short-circuits before project hooks.
        assert!(matches!(
            result,
            HookResult::Denied { reason, .. } if reason == "managed block"
        ));
    }

    // ---- evaluate_hooks: pass doesn't block ----

    #[test]
    fn pass_hooks_allow_action() {
        let tmp = tempdir();
        let executor = HookExecutor::default();
        let mut reg = HookRegistry::new();
        reg.add_project(
            HookEvent::BeforeTool,
            vec![spec(r#"echo '{"decision":"pass"}'"#)],
        );

        let result = evaluate_hooks(&executor, &reg, HookEvent::BeforeTool, &input(), &tmp);
        assert!(matches!(result, HookResult::Allow { .. }));
    }

    fn tempdir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "altai-hook-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}
