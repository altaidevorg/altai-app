//! Environment profile and reproducible setup (plan §E1).
//!
//! Parses an environment specification (install, start, terminal, cache,
//! healthcheck, env-revision), runs setup idempotently inside an isolated
//! workspace, caches safe dependency state keyed by repository + environment
//! revision, tracks long-lived processes, and surfaces environment health and
//! setup logs.
//!
//! Acceptance criteria (plan §E1):
//! - setup is idempotent;
//! - a worktree can boot the application independently;
//! - stale cache keys cannot cross repositories;
//! - background processes cannot outlive cleanup unnoticed.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Environment profile specification
// ---------------------------------------------------------------------------

/// A single shell command to run during setup or as a long-lived process.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ShellCommand {
    /// Executable name (e.g. `npm`, `cargo`, `node`).
    pub program: String,
    /// Arguments passed to the program.
    #[serde(default)]
    pub args: Vec<String>,
    /// Optional environment variable overrides for this command.
    #[serde(default)]
    pub env: HashMap<String, String>,
}

impl ShellCommand {
    /// Render a human-readable representation for logging.
    pub fn display(&self) -> String {
        if self.args.is_empty() {
            self.program.clone()
        } else {
            format!("{} {}", self.program, self.args.join(" "))
        }
    }

    /// Stable signature for cache invalidation.
    fn signature(&self) -> String {
        let mut env_pairs: Vec<String> = self.env.iter().map(|(k, v)| format!("{k}={v}")).collect();
        env_pairs.sort();
        format!(
            "{}|{}|{}",
            self.program,
            self.args.join(" "),
            env_pairs.join(";")
        )
    }
}

/// A healthcheck definition.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HealthcheckSpec {
    /// TCP port to probe.
    pub port: u16,
    /// Path to request (HTTP GET) when the port responds.
    #[serde(default)]
    pub path: Option<String>,
    /// Maximum time in milliseconds to wait for the service to become healthy.
    #[serde(default = "default_healthcheck_timeout_ms")]
    pub timeout_ms: u64,
    /// Polling interval in milliseconds.
    #[serde(default = "default_healthcheck_interval_ms")]
    pub interval_ms: u64,
}

fn default_healthcheck_timeout_ms() -> u64 {
    30_000
}
fn default_healthcheck_interval_ms() -> u64 {
    500
}

/// Cache configuration for safe dependency state.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CacheSpec {
    /// Cache key directories (e.g. `node_modules`, `target`).
    #[serde(default)]
    pub directories: Vec<String>,
    /// Whether the cache is shared across repositories.
    #[serde(default)]
    pub shared: bool,
}

impl Default for CacheSpec {
    fn default() -> Self {
        Self {
            directories: Vec::new(),
            shared: false,
        }
    }
}

/// A terminal environment definition for the workspace.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct TerminalSpec {
    /// Shell program to use.
    #[serde(default)]
    pub shell: Option<String>,
    /// Working directory inside the workspace.
    #[serde(default)]
    pub working_dir: Option<String>,
    /// Environment variables to set in the terminal.
    #[serde(default)]
    pub env: HashMap<String, String>,
}

/// The full environment profile.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentProfile {
    /// Repository identifier (e.g. git remote URL or org/repo).
    pub repo_id: String,
    /// Monotonically increasing revision of the environment spec.
    pub env_revision: String,
    /// Commands to run during setup (install dependencies, build, etc.).
    #[serde(default)]
    pub install: Vec<ShellCommand>,
    /// Long-lived processes to start after setup (dev server, etc.).
    #[serde(default)]
    pub start: Vec<ShellCommand>,
    /// Terminal configuration.
    #[serde(default)]
    pub terminal: TerminalSpec,
    /// Cache configuration.
    #[serde(default = "default_cache_spec")]
    pub cache: CacheSpec,
    /// Healthcheck definition (optional — not all environments have a server).
    #[serde(default)]
    pub healthcheck: Option<HealthcheckSpec>,
}

fn default_cache_spec() -> CacheSpec {
    CacheSpec::default()
}

impl EnvironmentProfile {
    /// Compute a deterministic cache key scoped to repo + env revision.
    ///
    /// Stale cache keys cannot cross repositories because the repo_id and
    /// env_revision are part of the hash.
    pub fn cache_key(&self) -> String {
        let mut parts = vec![self.repo_id.clone(), self.env_revision.clone()];
        for cmd in &self.install {
            parts.push(cmd.signature());
        }
        let joined = parts.join("\n");
        format!("{:x}", seahash(&joined))
    }

    /// Validate the profile for completeness and correctness.
    pub fn validate(&self) -> Result<(), EnvironmentError> {
        if self.repo_id.trim().is_empty() {
            return Err(EnvironmentError::InvalidProfile(
                "repo_id is required".to_string(),
            ));
        }
        if self.env_revision.trim().is_empty() {
            return Err(EnvironmentError::InvalidProfile(
                "env_revision is required".to_string(),
            ));
        }
        for (i, cmd) in self.install.iter().enumerate() {
            if cmd.program.trim().is_empty() {
                return Err(EnvironmentError::InvalidProfile(format!(
                    "install[{i}]: program is required"
                )));
            }
        }
        for (i, cmd) in self.start.iter().enumerate() {
            if cmd.program.trim().is_empty() {
                return Err(EnvironmentError::InvalidProfile(format!(
                    "start[{i}]: program is required"
                )));
            }
        }
        if let Some(hc) = &self.healthcheck {
            if hc.port == 0 {
                return Err(EnvironmentError::InvalidProfile(
                    "healthcheck.port must be > 0".to_string(),
                ));
            }
            if hc.interval_ms == 0 {
                return Err(EnvironmentError::InvalidProfile(
                    "healthcheck.intervalMs must be > 0".to_string(),
                ));
            }
            if hc.timeout_ms < hc.interval_ms {
                return Err(EnvironmentError::InvalidProfile(
                    "healthcheck.timeoutMs must be >= intervalMs".to_string(),
                ));
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum EnvironmentError {
    InvalidProfile(String),
    SetupFailed { step: String, message: String },
    HealthcheckFailed { reason: String },
    ProcessStillRunning { pid: String },
}

impl std::fmt::Display for EnvironmentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidProfile(msg) => write!(f, "invalid environment profile: {msg}"),
            Self::SetupFailed { step, message } => {
                write!(f, "setup failed at '{step}': {message}")
            }
            Self::HealthcheckFailed { reason } => {
                write!(f, "healthcheck failed: {reason}")
            }
            Self::ProcessStillRunning { pid } => {
                write!(f, "process {pid} is still running after cleanup")
            }
        }
    }
}

impl std::error::Error for EnvironmentError {}

// ---------------------------------------------------------------------------
// Process tracking
// ---------------------------------------------------------------------------

/// A tracked long-lived process started from the environment profile.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TrackedProcess {
    /// Synthetic process identifier (for testability — real PID in production).
    pub pid: String,
    /// The command that started this process.
    pub command: ShellCommand,
    /// Whether the process is currently alive.
    pub alive: bool,
    /// Timestamp (ms since epoch) when the process was started.
    pub started_at_ms: u64,
    /// Label identifying which `start` entry this process corresponds to.
    pub label: String,
    /// Cache key of the environment that owns this process.
    pub cache_key: String,
}

// ---------------------------------------------------------------------------
// Setup executor (pure logic, testable without real processes)
// ---------------------------------------------------------------------------

/// A trait abstracting command execution for testability.
/// Production implementations shell out; tests use a mock.
pub trait CommandRunner: std::fmt::Debug {
    /// Run a setup command and return stdout on success or an error message.
    fn run_setup(&mut self, cmd: &ShellCommand) -> Result<String, String>;

    /// Start a long-lived process and return a synthetic PID.
    fn start_process(&mut self, cmd: &ShellCommand) -> Result<String, String>;

    /// Terminate a previously started process by PID.
    fn kill_process(&mut self, pid: &str) -> Result<(), String>;

    /// Check if a process is still alive.
    fn is_process_alive(&self, pid: &str) -> bool;

    /// Probe a healthcheck endpoint.
    fn probe_health(&mut self, hc: &HealthcheckSpec) -> Result<(), String>;
}

/// A setup log entry recording what happened during setup.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SetupLogEntry {
    pub step: String,
    pub command: String,
    pub success: bool,
    pub output: String,
    pub duration_ms: u64,
}

/// The result of running environment setup.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupResult {
    /// Whether setup was a cache hit (no install commands executed).
    pub cache_hit: bool,
    /// The cache key used.
    pub cache_key: String,
    /// Setup log entries.
    pub log: Vec<SetupLogEntry>,
    /// Long-lived processes started.
    pub processes: Vec<TrackedProcess>,
    /// Final health status.
    pub health: EnvironmentHealth,
}

/// The health status of an environment after setup.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvironmentHealth {
    /// Setup succeeded and healthcheck passed (or no healthcheck configured).
    Healthy,
    /// Setup succeeded but healthcheck has not yet passed.
    Starting,
    /// Setup failed or healthcheck failed.
    Unhealthy,
}

impl Default for EnvironmentHealth {
    fn default() -> Self {
        Self::Healthy
    }
}

/// The environment setup executor.
///
/// Runs install commands idempotently (skipping if cache is valid), starts
/// long-lived processes, runs healthcheck, and tracks all processes for
/// cleanup.
#[derive(Debug)]
pub struct EnvironmentSetup<R: CommandRunner> {
    runner: R,
    /// Cache keys that are known to be valid (setup already completed).
    valid_cache: HashMap<String, Vec<SetupLogEntry>>,
    /// Currently tracked processes across all environments.
    processes: Vec<TrackedProcess>,
    /// Health status per cache key.
    health: HashMap<String, EnvironmentHealth>,
}

impl<R: CommandRunner> EnvironmentSetup<R> {
    pub fn new(runner: R) -> Self {
        Self {
            runner,
            valid_cache: HashMap::new(),
            processes: Vec::new(),
            health: HashMap::new(),
        }
    }

    /// Run environment setup.
    ///
    /// If the cache key is already valid, install commands are skipped
    /// (idempotent). Start commands are always executed to bring up
    /// long-lived processes.
    pub fn setup(
        &mut self,
        profile: &EnvironmentProfile,
        now_ms: u64,
    ) -> Result<SetupResult, EnvironmentError> {
        profile.validate()?;
        let cache_key = profile.cache_key();

        let cache_hit = self.valid_cache.contains_key(&cache_key);
        let mut log = Vec::new();

        if cache_hit {
            log = self.valid_cache[&cache_key].clone();
        } else {
            for cmd in &profile.install {
                let display = cmd.display();
                let start = now_ms;
                match self.runner.run_setup(cmd) {
                    Ok(output) => {
                        let duration = now_ms.saturating_sub(start);
                        log.push(SetupLogEntry {
                            step: "install".to_string(),
                            command: display.clone(),
                            success: true,
                            output,
                            duration_ms: duration,
                        });
                    }
                    Err(err) => {
                        let duration = now_ms.saturating_sub(start);
                        log.push(SetupLogEntry {
                            step: "install".to_string(),
                            command: display.clone(),
                            success: false,
                            output: err.clone(),
                            duration_ms: duration,
                        });
                        self.health
                            .insert(cache_key.clone(), EnvironmentHealth::Unhealthy);
                        return Err(EnvironmentError::SetupFailed {
                            step: display,
                            message: err,
                        });
                    }
                }
            }
            self.valid_cache.insert(cache_key.clone(), log.clone());
        }

        let mut started_processes = Vec::new();
        for cmd in &profile.start {
            let display = cmd.display();
            match self.runner.start_process(cmd) {
                Ok(pid) => {
                    let proc = TrackedProcess {
                        pid: pid.clone(),
                        command: cmd.clone(),
                        alive: true,
                        started_at_ms: now_ms,
                        label: display,
                        cache_key: cache_key.clone(),
                    };
                    started_processes.push(proc.clone());
                    self.processes.push(proc);
                }
                Err(err) => {
                    self.health
                        .insert(cache_key.clone(), EnvironmentHealth::Unhealthy);
                    return Err(EnvironmentError::SetupFailed {
                        step: display,
                        message: err,
                    });
                }
            }
        }

        let health = if let Some(hc) = &profile.healthcheck {
            match self.runner.probe_health(hc) {
                Ok(()) => EnvironmentHealth::Healthy,
                Err(_) => EnvironmentHealth::Starting,
            }
        } else {
            EnvironmentHealth::Healthy
        };
        self.health.insert(cache_key.clone(), health);

        Ok(SetupResult {
            cache_hit,
            cache_key,
            log,
            processes: started_processes,
            health,
        })
    }

    /// Cleanup all tracked processes for a given cache key.
    ///
    /// Returns an error if any process is still alive after kill attempts.
    pub fn cleanup(&mut self, cache_key: &str) -> Result<(), EnvironmentError> {
        let to_kill: Vec<String> = self
            .processes
            .iter()
            .filter(|p| p.cache_key == cache_key && p.alive)
            .map(|p| p.pid.clone())
            .collect();

        for pid in &to_kill {
            let _ = self.runner.kill_process(pid);
        }

        let mut still_running = Vec::new();
        for proc in &mut self.processes {
            if to_kill.contains(&proc.pid) {
                let alive = self.runner.is_process_alive(&proc.pid);
                proc.alive = alive;
                if alive {
                    still_running.push(proc.pid.clone());
                }
            }
        }

        self.health.remove(cache_key);

        if let Some(pid) = still_running.first() {
            return Err(EnvironmentError::ProcessStillRunning { pid: pid.clone() });
        }

        self.processes.retain(|p| !to_kill.contains(&p.pid));

        Ok(())
    }

    /// Cleanup ALL tracked processes regardless of cache key.
    pub fn cleanup_all(&mut self) -> Result<(), EnvironmentError> {
        let pids: Vec<String> = self
            .processes
            .iter()
            .filter(|p| p.alive)
            .map(|p| p.pid.clone())
            .collect();

        for pid in &pids {
            let _ = self.runner.kill_process(pid);
        }

        let mut still_running = Vec::new();
        for proc in &mut self.processes {
            if pids.contains(&proc.pid) {
                let alive = self.runner.is_process_alive(&proc.pid);
                proc.alive = alive;
                if alive {
                    still_running.push(proc.pid.clone());
                }
            }
        }

        self.processes.clear();
        self.health.clear();
        self.valid_cache.clear();

        if let Some(pid) = still_running.first() {
            return Err(EnvironmentError::ProcessStillRunning { pid: pid.clone() });
        }
        Ok(())
    }

    /// Get current health status for a cache key.
    pub fn health_status(&self, cache_key: &str) -> EnvironmentHealth {
        self.health
            .get(cache_key)
            .copied()
            .unwrap_or(EnvironmentHealth::Unhealthy)
    }

    /// Get all currently tracked processes.
    pub fn tracked_processes(&self) -> &[TrackedProcess] {
        &self.processes
    }

    /// Check if a cache key is valid (setup completed successfully).
    pub fn is_cache_valid(&self, cache_key: &str) -> bool {
        self.valid_cache.contains_key(cache_key)
    }

    /// Re-check health for an environment (e.g. after waiting for startup).
    pub fn recheck_health(&mut self, profile: &EnvironmentProfile) -> EnvironmentHealth {
        let cache_key = profile.cache_key();
        if let Some(hc) = &profile.healthcheck {
            let health = match self.runner.probe_health(hc) {
                Ok(()) => EnvironmentHealth::Healthy,
                Err(_) => EnvironmentHealth::Starting,
            };
            self.health.insert(cache_key.clone(), health);
            health
        } else {
            EnvironmentHealth::Healthy
        }
    }
}

// ---------------------------------------------------------------------------
// Seahash implementation (deterministic, no external dependency)
// ---------------------------------------------------------------------------

fn seahash(data: &str) -> u64 {
    let bytes = data.as_bytes();
    if bytes.is_empty() {
        return 0x16f11fe89b0d677c;
    }
    let mut hash: u64 = 0x16f11fe89b0d677c;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x9e3779b97f4a7c15);
        hash = (hash << 13) | (hash >> 51);
    }
    hash
}

// ---------------------------------------------------------------------------
// Mock command runner for testing
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct MockCommandRunner {
    /// Whether setup commands succeed.
    pub setup_succeeds: bool,
    /// Whether process starts succeed.
    pub start_succeeds: bool,
    /// Whether kill succeeds.
    pub kill_succeeds: bool,
    /// Whether health probes succeed.
    pub health_succeeds: bool,
    /// PIDs that are "alive" (for testing process tracking).
    pub alive_pids: Vec<String>,
    /// Counter for generating PIDs.
    pid_counter: u64,
    /// Captured setup commands.
    pub captured_setup: Vec<ShellCommand>,
    /// Captured start commands.
    pub captured_starts: Vec<ShellCommand>,
}

impl Default for MockCommandRunner {
    fn default() -> Self {
        Self {
            setup_succeeds: true,
            start_succeeds: true,
            kill_succeeds: true,
            health_succeeds: true,
            alive_pids: Vec::new(),
            pid_counter: 0,
            captured_setup: Vec::new(),
            captured_starts: Vec::new(),
        }
    }
}

impl CommandRunner for MockCommandRunner {
    fn run_setup(&mut self, cmd: &ShellCommand) -> Result<String, String> {
        self.captured_setup.push(cmd.clone());
        if self.setup_succeeds {
            Ok(format!("ok: {}", cmd.display()))
        } else {
            Err(format!("setup failed: {}", cmd.display()))
        }
    }

    fn start_process(&mut self, cmd: &ShellCommand) -> Result<String, String> {
        self.captured_starts.push(cmd.clone());
        if self.start_succeeds {
            self.pid_counter += 1;
            let pid = format!("mock-pid-{}", self.pid_counter);
            self.alive_pids.push(pid.clone());
            Ok(pid)
        } else {
            Err(format!("start failed: {}", cmd.display()))
        }
    }

    fn kill_process(&mut self, pid: &str) -> Result<(), String> {
        if self.kill_succeeds {
            self.alive_pids.retain(|p| p != pid);
            Ok(())
        } else {
            Err(format!("kill failed: {pid}"))
        }
    }

    fn is_process_alive(&self, pid: &str) -> bool {
        self.alive_pids.contains(&pid.to_string())
    }

    fn probe_health(&mut self, _hc: &HealthcheckSpec) -> Result<(), String> {
        if self.health_succeeds {
            Ok(())
        } else {
            Err("healthcheck failed: connection refused".to_string())
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_profile() -> EnvironmentProfile {
        EnvironmentProfile {
            repo_id: "org/repo".to_string(),
            env_revision: "v1".to_string(),
            install: vec![ShellCommand {
                program: "npm".to_string(),
                args: vec!["install".to_string()],
                env: HashMap::new(),
            }],
            start: vec![ShellCommand {
                program: "npm".to_string(),
                args: vec!["run".to_string(), "dev".to_string()],
                env: HashMap::new(),
            }],
            terminal: TerminalSpec::default(),
            cache: CacheSpec {
                directories: vec!["node_modules".to_string()],
                shared: false,
            },
            healthcheck: Some(HealthcheckSpec {
                port: 3000,
                path: Some("/health".to_string()),
                timeout_ms: 10_000,
                interval_ms: 500,
            }),
        }
    }

    // ---- Profile parsing & validation ----

    #[test]
    fn profile_cache_key_is_deterministic() {
        let p1 = sample_profile();
        let p2 = sample_profile();
        assert_eq!(p1.cache_key(), p2.cache_key());
    }

    #[test]
    fn cache_key_changes_with_repo_id() {
        let p1 = sample_profile();
        let mut p2 = sample_profile();
        p2.repo_id = "other/repo".to_string();
        assert_ne!(p1.cache_key(), p2.cache_key());
    }

    #[test]
    fn cache_key_changes_with_env_revision() {
        let p1 = sample_profile();
        let mut p2 = sample_profile();
        p2.env_revision = "v2".to_string();
        assert_ne!(p1.cache_key(), p2.cache_key());
    }

    #[test]
    fn cache_key_changes_with_install_commands() {
        let p1 = sample_profile();
        let mut p2 = sample_profile();
        p2.install[0].args = vec!["ci".to_string()];
        assert_ne!(p1.cache_key(), p2.cache_key());
    }

    #[test]
    fn cache_key_changes_with_env_vars() {
        let p1 = sample_profile();
        let mut p2 = sample_profile();
        p2.install[0]
            .env
            .insert("NODE_ENV".to_string(), "production".to_string());
        assert_ne!(p1.cache_key(), p2.cache_key());
    }

    #[test]
    fn validate_rejects_empty_repo_id() {
        let mut p = sample_profile();
        p.repo_id = "".to_string();
        assert!(matches!(
            p.validate(),
            Err(EnvironmentError::InvalidProfile(_))
        ));
    }

    #[test]
    fn validate_rejects_empty_env_revision() {
        let mut p = sample_profile();
        p.env_revision = "  ".to_string();
        assert!(p.validate().is_err());
    }

    #[test]
    fn validate_rejects_empty_install_program() {
        let mut p = sample_profile();
        p.install.push(ShellCommand {
            program: "".to_string(),
            args: vec![],
            env: HashMap::new(),
        });
        assert!(p.validate().is_err());
    }

    #[test]
    fn validate_rejects_healthcheck_port_zero() {
        let mut p = sample_profile();
        if let Some(ref mut hc) = p.healthcheck {
            hc.port = 0;
        }
        assert!(p.validate().is_err());
    }

    #[test]
    fn validate_rejects_healthcheck_timeout_less_than_interval() {
        let mut p = sample_profile();
        if let Some(ref mut hc) = p.healthcheck {
            hc.timeout_ms = 100;
            hc.interval_ms = 500;
        }
        assert!(p.validate().is_err());
    }

    #[test]
    fn validate_accepts_profile_without_healthcheck() {
        let mut p = sample_profile();
        p.healthcheck = None;
        assert!(p.validate().is_ok());
    }

    #[test]
    fn validate_accepts_profile_without_install() {
        let mut p = sample_profile();
        p.install.clear();
        assert!(p.validate().is_ok());
    }

    #[test]
    fn shell_command_display_no_args() {
        let cmd = ShellCommand {
            program: "ls".to_string(),
            args: vec![],
            env: HashMap::new(),
        };
        assert_eq!(cmd.display(), "ls");
    }

    #[test]
    fn shell_command_display_with_args() {
        let cmd = ShellCommand {
            program: "npm".to_string(),
            args: vec!["run".to_string(), "build".to_string()],
            env: HashMap::new(),
        };
        assert_eq!(cmd.display(), "npm run build");
    }

    // ---- Setup execution ----

    #[test]
    fn setup_runs_install_and_starts_processes() {
        let runner = MockCommandRunner::default();
        let mut setup = EnvironmentSetup::new(runner);
        let profile = sample_profile();
        let result = setup.setup(&profile, 1000).unwrap();

        assert!(!result.cache_hit);
        assert_eq!(result.log.len(), 1);
        assert!(result.log[0].success);
        assert_eq!(result.processes.len(), 1);
        assert_eq!(result.health, EnvironmentHealth::Healthy);
    }

    #[test]
    fn setup_is_idempotent_on_cache_hit() {
        let runner = MockCommandRunner::default();
        let mut setup = EnvironmentSetup::new(runner);
        let profile = sample_profile();

        let r1 = setup.setup(&profile, 1000).unwrap();
        assert!(!r1.cache_hit);

        let r2 = setup.setup(&profile, 2000).unwrap();
        assert!(r2.cache_hit);
        // Install log should be the same (from cache)
        assert_eq!(r1.log, r2.log);
        // But processes should still be started
        assert_eq!(r2.processes.len(), 1);
    }

    #[test]
    fn setup_does_not_execute_install_on_cache_hit() {
        let runner = MockCommandRunner::default();
        let mut setup = EnvironmentSetup::new(runner);
        let profile = sample_profile();

        setup.setup(&profile, 1000).unwrap();
        let setup_captured_count = setup.runner.captured_setup.len();
        assert_eq!(setup_captured_count, 1);

        setup.setup(&profile, 2000).unwrap();
        // Should not have run install again
        assert_eq!(setup.runner.captured_setup.len(), 1);
    }

    #[test]
    fn setup_fails_on_install_error() {
        let runner = MockCommandRunner {
            setup_succeeds: false,
            ..MockCommandRunner::default()
        };
        let mut setup = EnvironmentSetup::new(runner);
        let profile = sample_profile();

        let result = setup.setup(&profile, 1000);
        assert!(matches!(result, Err(EnvironmentError::SetupFailed { .. })));
    }

    #[test]
    fn setup_fails_on_start_error() {
        let runner = MockCommandRunner {
            start_succeeds: false,
            ..MockCommandRunner::default()
        };
        let mut setup = EnvironmentSetup::new(runner);
        let profile = sample_profile();

        let result = setup.setup(&profile, 1000);
        assert!(matches!(result, Err(EnvironmentError::SetupFailed { .. })));
    }

    #[test]
    fn setup_health_starting_when_healthcheck_fails() {
        let runner = MockCommandRunner {
            health_succeeds: false,
            ..MockCommandRunner::default()
        };
        let mut setup = EnvironmentSetup::new(runner);
        let profile = sample_profile();

        let result = setup.setup(&profile, 1000).unwrap();
        assert_eq!(result.health, EnvironmentHealth::Starting);
    }

    #[test]
    fn setup_healthy_when_no_healthcheck() {
        let runner = MockCommandRunner::default();
        let mut setup = EnvironmentSetup::new(runner);
        let mut profile = sample_profile();
        profile.healthcheck = None;

        let result = setup.setup(&profile, 1000).unwrap();
        assert_eq!(result.health, EnvironmentHealth::Healthy);
    }

    #[test]
    fn different_revisions_invalidate_cache() {
        let runner = MockCommandRunner::default();
        let mut setup = EnvironmentSetup::new(runner);

        let p1 = sample_profile();
        let mut p2 = sample_profile();
        p2.env_revision = "v2".to_string();

        setup.setup(&p1, 1000).unwrap();
        assert!(setup.is_cache_valid(&p1.cache_key()));
        assert!(!setup.is_cache_valid(&p2.cache_key()));

        let r2 = setup.setup(&p2, 2000).unwrap();
        assert!(!r2.cache_hit);
        assert!(setup.is_cache_valid(&p2.cache_key()));
    }

    #[test]
    fn different_repos_do_not_share_cache() {
        let runner = MockCommandRunner::default();
        let mut setup = EnvironmentSetup::new(runner);

        let p1 = sample_profile();
        let mut p2 = sample_profile();
        p2.repo_id = "other/repo".to_string();

        setup.setup(&p1, 1000).unwrap();
        assert!(!setup.is_cache_valid(&p2.cache_key()));
    }

    // ---- Process tracking & cleanup ----

    #[test]
    fn cleanup_terminates_all_processes() {
        let runner = MockCommandRunner::default();
        let mut setup = EnvironmentSetup::new(runner);
        let profile = sample_profile();

        setup.setup(&profile, 1000).unwrap();
        assert!(!setup.tracked_processes().is_empty());

        let cache_key = profile.cache_key();
        setup.cleanup(&cache_key).unwrap();
        assert!(setup.tracked_processes().is_empty());
    }

    #[test]
    fn cleanup_returns_error_when_process_still_alive() {
        let runner = MockCommandRunner {
            kill_succeeds: false,
            ..MockCommandRunner::default()
        };
        let mut setup = EnvironmentSetup::new(runner);
        let profile = sample_profile();

        setup.setup(&profile, 1000).unwrap();
        let cache_key = profile.cache_key();

        let result = setup.cleanup(&cache_key);
        assert!(matches!(
            result,
            Err(EnvironmentError::ProcessStillRunning { .. })
        ));
    }

    #[test]
    fn cleanup_all_terminates_everything() {
        let runner = MockCommandRunner::default();
        let mut setup = EnvironmentSetup::new(runner);

        let p1 = sample_profile();
        let mut p2 = sample_profile();
        p2.env_revision = "v2".to_string();

        setup.setup(&p1, 1000).unwrap();
        setup.setup(&p2, 2000).unwrap();
        assert_eq!(setup.tracked_processes().len(), 2);

        setup.cleanup_all().unwrap();
        assert!(setup.tracked_processes().is_empty());
    }

    #[test]
    fn cleanup_all_returns_error_when_process_survives() {
        let runner = MockCommandRunner {
            kill_succeeds: false,
            ..MockCommandRunner::default()
        };
        let mut setup = EnvironmentSetup::new(runner);
        let profile = sample_profile();

        setup.setup(&profile, 1000).unwrap();

        let result = setup.cleanup_all();
        assert!(matches!(
            result,
            Err(EnvironmentError::ProcessStillRunning { .. })
        ));
    }

    // ---- Health re-checking ----

    #[test]
    fn recheck_health_transitions_starting_to_healthy() {
        let runner = MockCommandRunner {
            health_succeeds: false,
            ..MockCommandRunner::default()
        };
        let mut setup = EnvironmentSetup::new(runner);
        let profile = sample_profile();

        let result = setup.setup(&profile, 1000).unwrap();
        assert_eq!(result.health, EnvironmentHealth::Starting);

        // Now health succeeds
        setup.runner.health_succeeds = true;
        let health = setup.recheck_health(&profile);
        assert_eq!(health, EnvironmentHealth::Healthy);
    }

    #[test]
    fn recheck_health_stays_starting_when_still_failing() {
        let runner = MockCommandRunner {
            health_succeeds: false,
            ..MockCommandRunner::default()
        };
        let mut setup = EnvironmentSetup::new(runner);
        let profile = sample_profile();

        setup.setup(&profile, 1000).unwrap();
        let health = setup.recheck_health(&profile);
        assert_eq!(health, EnvironmentHealth::Starting);
    }

    #[test]
    fn health_status_returns_unhealthy_for_unknown_key() {
        let runner = MockCommandRunner::default();
        let setup = EnvironmentSetup::new(runner);
        assert_eq!(
            setup.health_status("nonexistent"),
            EnvironmentHealth::Unhealthy
        );
    }

    #[test]
    fn health_status_returns_current_for_known_key() {
        let runner = MockCommandRunner {
            health_succeeds: false,
            ..MockCommandRunner::default()
        };
        let mut setup = EnvironmentSetup::new(runner);
        let profile = sample_profile();

        setup.setup(&profile, 1000).unwrap();
        let key = profile.cache_key();
        assert_eq!(setup.health_status(&key), EnvironmentHealth::Starting);
    }

    // ---- Setup logs ----

    #[test]
    fn setup_log_records_success() {
        let runner = MockCommandRunner::default();
        let mut setup = EnvironmentSetup::new(runner);
        let profile = sample_profile();

        let result = setup.setup(&profile, 5000).unwrap();
        let entry = &result.log[0];
        assert_eq!(entry.step, "install");
        assert!(entry.success);
        assert!(entry.output.contains("ok"));
        assert_eq!(entry.command, "npm install");
    }

    #[test]
    fn setup_log_records_failure() {
        let runner = MockCommandRunner {
            setup_succeeds: false,
            ..MockCommandRunner::default()
        };
        let mut setup = EnvironmentSetup::new(runner);
        let profile = sample_profile();

        let result = setup.setup(&profile, 5000);
        assert!(result.is_err());
    }

    // ---- Profile without start commands ----

    #[test]
    fn setup_works_without_start_commands() {
        let runner = MockCommandRunner::default();
        let mut setup = EnvironmentSetup::new(runner);
        let mut profile = sample_profile();
        profile.start.clear();

        let result = setup.setup(&profile, 1000).unwrap();
        assert!(result.processes.is_empty());
        assert_eq!(result.health, EnvironmentHealth::Healthy);
    }

    #[test]
    fn setup_works_without_install_commands() {
        let runner = MockCommandRunner::default();
        let mut setup = EnvironmentSetup::new(runner);
        let mut profile = sample_profile();
        profile.install.clear();

        let result = setup.setup(&profile, 1000).unwrap();
        assert!(result.log.is_empty());
        assert_eq!(result.processes.len(), 1);
    }

    // ---- Multiple start commands ----

    #[test]
    fn setup_starts_multiple_processes() {
        let runner = MockCommandRunner::default();
        let mut setup = EnvironmentSetup::new(runner);
        let mut profile = sample_profile();
        profile.start.push(ShellCommand {
            program: "redis-server".to_string(),
            args: vec![],
            env: HashMap::new(),
        });

        let result = setup.setup(&profile, 1000).unwrap();
        assert_eq!(result.processes.len(), 2);
        assert_eq!(setup.tracked_processes().len(), 2);
    }

    // ---- Error display ----

    #[test]
    fn error_display_messages() {
        assert!(format!("{}", EnvironmentError::InvalidProfile("bad".to_string())).contains("bad"));

        assert!(format!(
            "{}",
            EnvironmentError::SetupFailed {
                step: "npm install".to_string(),
                message: "ENOENT".to_string()
            }
        )
        .contains("npm install"));

        assert!(format!(
            "{}",
            EnvironmentError::HealthcheckFailed {
                reason: "timeout".to_string()
            }
        )
        .contains("timeout"));

        assert!(format!(
            "{}",
            EnvironmentError::ProcessStillRunning {
                pid: "123".to_string()
            }
        )
        .contains("123"));
    }

    // ---- Serialization round-trip ----

    #[test]
    fn profile_serializes_and_deserializes() {
        let profile = sample_profile();
        let json = serde_json::to_string(&profile).unwrap();
        let back: EnvironmentProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(profile, back);
    }

    #[test]
    fn profile_with_defaults_deserializes_from_minimal_json() {
        let json = r#"{
            "repoId": "org/repo",
            "envRevision": "v1"
        }"#;
        let profile: EnvironmentProfile = serde_json::from_str(json).unwrap();
        assert_eq!(profile.repo_id, "org/repo");
        assert_eq!(profile.env_revision, "v1");
        assert!(profile.install.is_empty());
        assert!(profile.start.is_empty());
        assert!(profile.healthcheck.is_none());
        assert!(profile.cache.directories.is_empty());
        assert!(!profile.cache.shared);
    }

    #[test]
    fn setup_result_serializes() {
        let runner = MockCommandRunner::default();
        let mut setup = EnvironmentSetup::new(runner);
        let profile = sample_profile();

        let result = setup.setup(&profile, 1000).unwrap();
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("cacheKey"));
        assert!(json.contains("processes"));
    }

    // ---- Cache key uniqueness across different env vars ordering ----

    #[test]
    fn cache_key_is_order_invariant_for_env_vars() {
        let mut env1 = HashMap::new();
        env1.insert("A".to_string(), "1".to_string());
        env1.insert("B".to_string(), "2".to_string());

        let mut env2 = HashMap::new();
        env2.insert("B".to_string(), "2".to_string());
        env2.insert("A".to_string(), "1".to_string());

        let p1 = EnvironmentProfile {
            repo_id: "org/repo".to_string(),
            env_revision: "v1".to_string(),
            install: vec![ShellCommand {
                program: "make".to_string(),
                args: vec![],
                env: env1,
            }],
            start: vec![],
            terminal: TerminalSpec::default(),
            cache: CacheSpec::default(),
            healthcheck: None,
        };

        let mut p2 = p1.clone();
        p2.install[0].env = env2;

        assert_eq!(p1.cache_key(), p2.cache_key());
    }
}
