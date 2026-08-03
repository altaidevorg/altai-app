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
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use shared_child::SharedChild;

use crate::modules::shell::build_oneshot_command;
use crate::modules::workspace::WorkspaceEnv;

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
#[serde(deny_unknown_fields)]
pub struct HookSpec {
    pub event: HookEvent,
    pub command: String,
    #[serde(
        default = "default_timeout_secs",
        rename = "timeout_seconds",
        alias = "timeout_secs"
    )]
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
    pub event: HookEvent,
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
    WorkspaceBoundary(String),
    /// Output exceeded the size limit.
    OutputTooLarge {
        stream: &'static str,
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
            Self::WorkspaceBoundary(message) => write!(f, "hook workspace rejected: {message}"),
            Self::OutputTooLarge { stream, limit } => {
                write!(f, "hook {stream} exceeded {limit} bytes")
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
    workspace_root: Option<PathBuf>,
}

impl HookExecutor {
    pub fn new(secret_patterns: Vec<String>) -> Self {
        Self {
            secret_patterns,
            workspace_root: None,
        }
    }

    /// Bind hook execution to one canonical workspace root. A caller cannot
    /// later substitute a sibling cwd or escape through a symlink.
    pub fn for_workspace(
        workspace_root: &Path,
        secret_patterns: Vec<String>,
    ) -> Result<Self, HookError> {
        let root = canonical_workspace_dir(workspace_root)?;
        Ok(Self {
            secret_patterns,
            workspace_root: Some(root),
        })
    }

    /// Execute a single hook command and return its parsed output.
    pub fn run(
        &self,
        spec: &HookSpec,
        input: &HookInput,
        cwd: &Path,
    ) -> Result<HookOutput, HookError> {
        let input_json = serde_json::to_vec(input).map_err(HookError::Json)?;
        let cwd = canonical_workspace_dir(cwd)?;
        if let Some(root) = &self.workspace_root {
            if !cwd.starts_with(root) {
                return Err(HookError::WorkspaceBoundary(format!(
                    "{} is outside {}",
                    cwd.display(),
                    root.display()
                )));
            }
        }
        let workspace_path = Path::new(&input.workspace_path);
        let declared_workspace = canonical_workspace_dir(workspace_path)?;
        let expected_workspace = self.workspace_root.as_ref().unwrap_or(&cwd);
        if declared_workspace != *expected_workspace {
            return Err(HookError::WorkspaceBoundary(
                "structured input workspace does not match the executor workspace".into(),
            ));
        }

        let cwd_string = cwd.to_string_lossy().into_owned();
        let mut command =
            build_oneshot_command(&spec.command, &WorkspaceEnv::Local, Some(&cwd_string))
                .map_err(HookError::Spawn)?;
        command
            .current_dir(&cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        restrict_hook_environment(&mut command, expected_workspace);
        configure_process_group(&mut command);

        let child = Arc::new(
            SharedChild::spawn(&mut command)
                .map_err(|error| HookError::Spawn(error.to_string()))?,
        );
        let mut process_tree = match ProcessTree::attach(child.id()) {
            Ok(tree) => tree,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
        };

        // Write input to stdin.
        if let Some(mut stdin) = child.take_stdin() {
            use std::io::Write;
            if let Err(error) = stdin.write_all(&input_json) {
                // A hook may intentionally exit without consuming stdin.
                if error.kind() != std::io::ErrorKind::BrokenPipe {
                    process_tree.terminate();
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(HookError::Io(error));
                }
            }
            // stdin dropped here, signaling EOF.
        }

        let stdout = match child.take_stdout() {
            Some(stdout) => stdout,
            None => {
                process_tree.terminate();
                let _ = child.kill();
                let _ = child.wait();
                return Err(HookError::Io(std::io::Error::other("stdout pipe missing")));
            }
        };
        let stderr = match child.take_stderr() {
            Some(stderr) => stderr,
            None => {
                process_tree.terminate();
                let _ = child.kill();
                let _ = child.wait();
                return Err(HookError::Io(std::io::Error::other("stderr pipe missing")));
            }
        };
        let stdout_reader = thread::spawn(move || drain_bounded(stdout));
        let stderr_reader = thread::spawn(move || drain_bounded(stderr));

        // Wait with timeout while both pipes are drained concurrently.
        let timeout = Duration::from_secs(spec.timeout_secs.max(1));
        let (wait_tx, wait_rx) = mpsc::channel();
        let waiter = Arc::clone(&child);
        thread::spawn(move || {
            let _ = wait_tx.send(waiter.wait());
        });
        let (status, timed_out) = match wait_rx.recv_timeout(timeout) {
            Ok(Ok(status)) => (Some(status), false),
            Ok(Err(error)) => {
                process_tree.terminate();
                let _ = child.kill();
                return Err(HookError::Io(error));
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                process_tree.terminate();
                let _ = child.kill();
                let _ = child.wait();
                (None, true)
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                process_tree.terminate();
                let _ = child.kill();
                return Err(HookError::Io(std::io::Error::other(
                    "hook wait thread disconnected",
                )));
            }
        };

        // Hooks may not leave background descendants running after their
        // parent shell exits. Terminate the process group/job before joining
        // readers so inherited pipe handles cannot hang this call.
        process_tree.terminate();
        let stdout = join_reader(stdout_reader)?;
        let stderr = join_reader(stderr_reader)?;
        if timed_out {
            return Err(HookError::Timeout);
        }
        if stdout.truncated {
            return Err(HookError::OutputTooLarge {
                stream: "stdout",
                limit: MAX_OUTPUT_BYTES,
            });
        }
        if stderr.truncated {
            return Err(HookError::OutputTooLarge {
                stream: "stderr",
                limit: MAX_OUTPUT_BYTES,
            });
        }
        let status = status.ok_or_else(|| HookError::Io(std::io::Error::other("process lost")))?;

        let stdout_str = String::from_utf8_lossy(&stdout.bytes);

        // Parse JSON output. If empty stdout, fall back to exit-code semantics.
        if stdout_str.trim().is_empty() {
            return Ok(HookOutput {
                decision: if status.success() {
                    HookDecision::Allow
                } else {
                    HookDecision::Deny
                },
                message: None,
            });
        }

        // Parse the JSON before redacting individual string fields. Replacing a
        // secret in the raw JSON can invalidate escaping when the secret itself
        // contains quotes or backslashes.
        let mut parsed: HookOutput = serde_json::from_str(&stdout_str).map_err(HookError::Json)?;
        if let Some(message) = parsed.message.as_mut() {
            *message = self.redact(message);
        }

        // Any non-zero exit is a denial for a blocking hook. In particular,
        // JSON `pass` or `{}` must not bypass fail-closed exit semantics.
        if !status.success() {
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

fn canonical_workspace_dir(path: &Path) -> Result<PathBuf, HookError> {
    let canonical = std::fs::canonicalize(path)
        .map_err(|error| HookError::WorkspaceBoundary(error.to_string()))?;
    if !canonical.is_dir() {
        return Err(HookError::WorkspaceBoundary(format!(
            "{} is not a directory",
            canonical.display()
        )));
    }
    Ok(canonical)
}

fn restrict_hook_environment(command: &mut std::process::Command, workspace_root: &Path) {
    let path = std::env::var_os("PATH");
    #[cfg(windows)]
    let system_root = std::env::var_os("SystemRoot");
    #[cfg(windows)]
    let path_ext = std::env::var_os("PATHEXT");

    command.env_clear();
    if let Some(path) = path {
        command.env("PATH", path);
    }
    command
        .env("ALTAI_WORKSPACE_ROOT", workspace_root)
        .env("HOME", workspace_root)
        .env("USERPROFILE", workspace_root)
        .env("TMPDIR", workspace_root)
        .env("TMP", workspace_root)
        .env("TEMP", workspace_root)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_ASKPASS", "")
        .env("SSH_ASKPASS", "");
    #[cfg(windows)]
    if let Some(system_root) = system_root {
        command.env("SystemRoot", system_root);
    }
    #[cfg(windows)]
    if let Some(path_ext) = path_ext {
        command.env("PATHEXT", path_ext);
    }
}

#[cfg(unix)]
fn configure_process_group(command: &mut std::process::Command) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
}

#[cfg(windows)]
fn configure_process_group(_command: &mut std::process::Command) {}

struct BoundedOutput {
    bytes: Vec<u8>,
    truncated: bool,
}

fn drain_bounded<R: Read>(mut reader: R) -> Result<BoundedOutput, std::io::Error> {
    let mut bytes = Vec::with_capacity(8192);
    let mut buffer = [0u8; 8192];
    let mut truncated = false;
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        let remaining = MAX_OUTPUT_BYTES.saturating_sub(bytes.len());
        let keep = remaining.min(read);
        bytes.extend_from_slice(&buffer[..keep]);
        truncated |= keep < read;
    }
    Ok(BoundedOutput { bytes, truncated })
}

fn join_reader(
    handle: thread::JoinHandle<Result<BoundedOutput, std::io::Error>>,
) -> Result<BoundedOutput, HookError> {
    handle
        .join()
        .map_err(|_| HookError::Io(std::io::Error::other("hook reader thread panicked")))?
        .map_err(HookError::Io)
}

#[cfg(unix)]
struct ProcessTree {
    process_group: libc::pid_t,
}

#[cfg(unix)]
impl ProcessTree {
    fn attach(pid: u32) -> Result<Self, HookError> {
        let process_group = libc::pid_t::try_from(pid)
            .map_err(|_| HookError::Io(std::io::Error::other("hook pid overflow")))?;
        Ok(Self { process_group })
    }

    fn terminate(&mut self) {
        // Negative pid addresses the process group created before spawn.
        unsafe {
            libc::kill(-self.process_group, libc::SIGKILL);
        }
    }
}

#[cfg(windows)]
struct ProcessTree {
    job: Option<crate::modules::pty::job::PtyJob>,
}

#[cfg(windows)]
impl ProcessTree {
    fn attach(pid: u32) -> Result<Self, HookError> {
        let job = crate::modules::pty::job::PtyJob::create_for(pid)
            .map_err(|error| HookError::Io(std::io::Error::other(error.to_string())))?;
        Ok(Self { job: Some(job) })
    }

    fn terminate(&mut self) {
        // Dropping the job closes KILL_ON_JOB_CLOSE and kills descendants
        // before reader threads join on inherited stdout/stderr handles.
        self.job.take();
    }
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
    pub fn add_project(&mut self, event: HookEvent, mut specs: Vec<HookSpec>) {
        // The map key is authoritative. Normalizing the embedded event avoids
        // a configuration mismatch where a before-tool hook is inspected as
        // one event but executed with another event in its JSON input.
        for spec in &mut specs {
            spec.event = event;
        }
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
        !self.hooks_for(event).is_empty()
    }

    pub fn managed_hooks(&self) -> &[HookSpec] {
        &self.managed
    }

    pub fn project_hooks(&self) -> impl Iterator<Item = &HookSpec> {
        self.project.values().flatten()
    }

    pub fn from_workflow_hooks(config: Option<&super::workflow_v2::HooksConfig>) -> HookRegistry {
        let mut registry = HookRegistry::new();
        let Some(config) = config else {
            return registry;
        };
        for hook in &config.lifecycle {
            registry
                .project
                .entry(hook.event)
                .or_default()
                .push(hook.clone());
        }
        // Preserve the original v2 hook fields. They predate B3's structured
        // lifecycle list but remain valid documents.
        for (event, command) in [
            (HookEvent::SessionStart, config.after_create.as_ref()),
            (HookEvent::SessionStart, config.before_run.as_ref()),
            (HookEvent::AfterRun, config.after_run.as_ref()),
        ] {
            if let Some(command) = command {
                registry.project.entry(event).or_default().push(HookSpec {
                    event,
                    command: command.clone(),
                    timeout_secs: config.timeout_seconds,
                    blocking: true,
                });
            }
        }
        registry
    }
}

/// Fully bound hook runtime used by the coordinator. The workspace path in
/// every JSON payload is derived here rather than trusted from a runner event.
#[derive(Clone)]
pub struct HookRuntime {
    executor: HookExecutor,
    registry: HookRegistry,
    workspace_root: PathBuf,
}

impl HookRuntime {
    pub fn new(
        workspace_root: &Path,
        registry: HookRegistry,
        secret_patterns: Vec<String>,
    ) -> Result<Self, HookError> {
        let workspace_root = canonical_workspace_dir(workspace_root)?;
        let executor = HookExecutor::for_workspace(&workspace_root, secret_patterns)?;
        Ok(Self {
            executor,
            registry,
            workspace_root,
        })
    }

    pub fn registry(&self) -> &HookRegistry {
        &self.registry
    }

    pub fn run(
        &self,
        event: HookEvent,
        task_id: &str,
        attempt_id: &str,
        action: Option<HookAction>,
    ) -> HookResult {
        let input = HookInput {
            event,
            task_id: task_id.to_string(),
            attempt_id: attempt_id.to_string(),
            workspace_path: self.workspace_root.to_string_lossy().into_owned(),
            action,
        };
        evaluate_hooks(
            &self.executor,
            &self.registry,
            event,
            &input,
            &self.workspace_root,
        )
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
                        hook_command: executor.redact(&spec.command),
                    };
                }
            }
            Err(err) => {
                if spec.blocking {
                    // Fail closed: a blocking hook error denies the action.
                    return HookResult::Denied {
                        reason: executor.redact(&format!("hook error: {err}")),
                        hook_command: executor.redact(&spec.command),
                    };
                }
                // Observability hook failure — log and continue.
                messages.push(executor.redact(&format!("observability hook failed: {err}")));
            }
        }
    }

    HookResult::Allow { messages }
}

// ---------------------------------------------------------------------------
// Read-only Settings inspector
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HookInspectionEntry {
    pub source: &'static str,
    pub event: HookEvent,
    pub command: String,
    pub timeout_seconds: u64,
    pub blocking: bool,
    pub locked: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HookInspection {
    pub workspace_path: String,
    pub workflow_path: String,
    pub validation_error: Option<String>,
    pub hooks: Vec<HookInspectionEntry>,
}

fn inspect_registry(
    workspace_path: String,
    workflow_path: String,
    validation_error: Option<String>,
    registry: &HookRegistry,
) -> HookInspection {
    let mut hooks = Vec::new();
    hooks.extend(
        registry
            .managed_hooks()
            .iter()
            .map(|hook| HookInspectionEntry {
                source: "managed",
                event: hook.event,
                command: hook.command.clone(),
                timeout_seconds: hook.timeout_secs,
                blocking: hook.blocking,
                locked: true,
            }),
    );
    hooks.extend(registry.project_hooks().map(|hook| HookInspectionEntry {
        source: "project",
        event: hook.event,
        command: hook.command.clone(),
        timeout_seconds: hook.timeout_secs,
        blocking: hook.blocking,
        locked: false,
    }));
    hooks.sort_by(|left, right| {
        left.event
            .name()
            .cmp(right.event.name())
            .then_with(|| left.source.cmp(right.source))
            .then_with(|| left.command.cmp(&right.command))
    });
    HookInspection {
        workspace_path,
        workflow_path,
        validation_error,
        hooks,
    }
}

#[tauri::command]
pub fn orchestration_hooks_inspect(
    workspace_key: String,
    workspace: Option<WorkspaceEnv>,
    workspace_registry: tauri::State<'_, crate::modules::workspace::WorkspaceRegistry>,
    managed_hooks: tauri::State<'_, HookRegistry>,
    app: tauri::AppHandle,
) -> Result<HookInspection, String> {
    let workspace = WorkspaceEnv::from_option(workspace);
    let path =
        super::workflow::workflow_path(&app, &workspace_registry, &workspace_key, &workspace)?;
    let workspace_path = path
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .to_string_lossy()
        .replace('\\', "/");
    let document = super::workflow::load_at(path);
    let mut registry = HookRegistry::from_workflow_hooks(
        document
            .config_v2
            .as_ref()
            .and_then(|config| config.hooks.as_ref()),
    );
    for hook in managed_hooks.managed_hooks() {
        registry.add_managed(hook.clone());
    }
    Ok(inspect_registry(
        workspace_path,
        document.path,
        document.validation_error,
        &registry,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn input(workspace: &Path) -> HookInput {
        HookInput {
            event: HookEvent::BeforeTool,
            task_id: "t-1".into(),
            attempt_id: "t-1-att-1".into(),
            workspace_path: workspace.to_string_lossy().into_owned(),
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

    fn file_exists_command(path: &str) -> String {
        if cfg!(windows) {
            format!(
                "if (Test-Path -LiteralPath '{}') {{ exit 0 }} else {{ exit 1 }}",
                path.replace('\'', "''")
            )
        } else {
            format!("test -f '{}'", path.replace('\'', "'\\''"))
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
        let output = executor
            .run(&spec("exit 0"), &input(&tmp), &tmp)
            .expect("run");
        assert_eq!(output.decision, HookDecision::Allow);
    }

    // ---- HookExecutor: deny via exit non-zero ----

    #[test]
    fn hook_exit_nonzero_denies() {
        let tmp = tempdir();
        let executor = HookExecutor::default();
        let output = executor
            .run(&spec("exit 1"), &input(&tmp), &tmp)
            .expect("run");
        assert_eq!(output.decision, HookDecision::Deny);
    }

    // ---- HookExecutor: JSON output parsed ----

    #[test]
    fn hook_json_allow_output() {
        let tmp = tempdir();
        let executor = HookExecutor::default();
        let cmd = r#"echo '{"decision":"allow","message":"ok"}'"#;
        let output = executor.run(&spec(cmd), &input(&tmp), &tmp).expect("run");
        assert_eq!(output.decision, HookDecision::Allow);
        assert_eq!(output.message.as_deref(), Some("ok"));
    }

    #[test]
    fn hook_json_deny_output() {
        let tmp = tempdir();
        let executor = HookExecutor::default();
        let cmd = r#"echo '{"decision":"deny","message":"blocked"}'"#;
        let output = executor.run(&spec(cmd), &input(&tmp), &tmp).expect("run");
        assert_eq!(output.decision, HookDecision::Deny);
        assert_eq!(output.message.as_deref(), Some("blocked"));
    }

    #[test]
    fn hook_json_pass_output() {
        let tmp = tempdir();
        let executor = HookExecutor::default();
        let cmd = r#"echo '{"decision":"pass"}'"#;
        let output = executor.run(&spec(cmd), &input(&tmp), &tmp).expect("run");
        assert_eq!(output.decision, HookDecision::Pass);
    }

    // ---- HookExecutor: exit non-zero overrides JSON allow ----

    #[test]
    fn non_zero_exit_overrides_json_allow() {
        let tmp = tempdir();
        let executor = HookExecutor::default();
        // JSON says allow but exit code is 1.
        let cmd = r#"echo '{"decision":"allow"}'; exit 1"#;
        let output = executor.run(&spec(cmd), &input(&tmp), &tmp).expect("run");
        assert_eq!(output.decision, HookDecision::Deny);
    }

    // ---- HookExecutor: invalid JSON on blocking hook ----

    #[test]
    fn invalid_json_is_error() {
        let tmp = tempdir();
        let executor = HookExecutor::default();
        let cmd = r#"echo 'not json'"#;
        let result = executor.run(&spec(cmd), &input(&tmp), &tmp);
        assert!(result.is_err());
    }

    // ---- HookExecutor: secret redaction ----

    #[test]
    fn secrets_redacted_from_output() {
        let tmp = tempdir();
        let executor = HookExecutor::new(vec!["super-secret-key".into()]);
        let cmd = r#"echo '{"decision":"allow","message":"key=super-secret-key"}'"#;
        let output = executor.run(&spec(cmd), &input(&tmp), &tmp).expect("run");
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
        let result = executor.run(&s, &input(&tmp), &tmp);
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
        let cmd = {
            #[cfg(windows)]
            {
                // A .cmd script is reliable under both pwsh -Command and cmd /C,
                // and inherits the HookExecutor cwd (unlike nested quoting tricks).
                fs::write(
                    tmp.join("check-marker.cmd"),
                    "@echo off\r\nif exist \"marker.txt\" (exit /b 0) else (exit /b 1)\r\n",
                )
                .unwrap();
                "cmd.exe /d /c check-marker.cmd".to_string()
            }
            #[cfg(not(windows))]
            {
                file_exists_command("marker.txt")
            }
        };
        let mut hook = spec(&cmd);
        // Windows oneshot shells (pwsh) can be slower under CI load.
        hook.timeout_secs = 30;
        let output = executor.run(&hook, &input(&tmp), &tmp).expect("run");
        assert_eq!(output.decision, HookDecision::Allow);
    }

    #[test]
    fn executor_rejects_cwd_outside_bound_workspace() {
        let workspace = tempdir();
        let outside = tempdir();
        let executor = HookExecutor::for_workspace(&workspace, Vec::new()).expect("executor");
        let result = executor.run(&spec("exit 0"), &input(&workspace), &outside);
        assert!(matches!(result, Err(HookError::WorkspaceBoundary(_))));
    }

    #[test]
    fn non_zero_exit_overrides_json_pass() {
        let tmp = tempdir();
        let executor = HookExecutor::default();
        let cmd = r#"echo '{"decision":"pass"}'; exit 1"#;
        let output = executor.run(&spec(cmd), &input(&tmp), &tmp).expect("run");
        assert_eq!(output.decision, HookDecision::Deny);
    }

    #[cfg(unix)]
    #[test]
    fn oversized_stdout_is_bounded_without_deadlock() {
        let tmp = tempdir();
        let executor = HookExecutor::default();
        let mut hook = spec("yes x | head -c 300000");
        hook.timeout_secs = 3;
        let result = executor.run(&hook, &input(&tmp), &tmp);
        assert!(matches!(
            result,
            Err(HookError::OutputTooLarge {
                stream: "stdout",
                ..
            })
        ));
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

        let result = evaluate_hooks(&executor, &reg, HookEvent::BeforeTool, &input(&tmp), &tmp);
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

        let result = evaluate_hooks(&executor, &reg, HookEvent::BeforeTool, &input(&tmp), &tmp);
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

        let result = evaluate_hooks(&executor, &reg, HookEvent::OnError, &input(&tmp), &tmp);
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

        let result = evaluate_hooks(&executor, &reg, HookEvent::BeforeTool, &input(&tmp), &tmp);
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

        let result = evaluate_hooks(&executor, &reg, HookEvent::BeforeTool, &input(&tmp), &tmp);
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

        let result = evaluate_hooks(&executor, &reg, HookEvent::BeforeTool, &input(&tmp), &tmp);
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

        let result = evaluate_hooks(&executor, &reg, HookEvent::BeforeTool, &input(&tmp), &tmp);
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
