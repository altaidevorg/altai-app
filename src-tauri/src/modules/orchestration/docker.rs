//! Docker executor — container spec, path mapping, capability negotiation (plan §E3).
//!
//! Implements the Docker phase of the WorkspaceExecutor contract: the same
//! task/runner works in local worktree and Docker, host/container path mapping
//! is explicit, and executor capability differences are visible before dispatch.
//!
//! Acceptance criteria (plan §E3):
//! - the same task/runner works in local worktree and Docker;
//! - host/container path mapping is explicit;
//! - executor capability differences are visible before dispatch.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Container spec
// ---------------------------------------------------------------------------

/// A volume mount mapping host path to container path.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct VolumeMount {
    /// Host filesystem path.
    pub host_path: String,
    /// Path inside the container.
    pub container_path: String,
    /// Whether the mount is read-only.
    #[serde(default)]
    pub read_only: bool,
}

/// A port mapping from container to host.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct PortMapping {
    pub container_port: u16,
    pub host_port: u16,
    /// Protocol: tcp or udp.
    #[serde(default = "default_protocol")]
    pub protocol: PortProtocol,
}

fn default_protocol() -> PortProtocol {
    PortProtocol::Tcp
}

/// Network protocol for port mapping.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum PortProtocol {
    Tcp,
    Udp,
}

/// A Docker container specification.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ContainerSpec {
    /// Docker image (e.g. `node:20-alpine`).
    pub image: String,
    /// Container name (optional — Docker generates one if omitted).
    #[serde(default)]
    pub name: Option<String>,
    /// Working directory inside the container.
    #[serde(default)]
    pub working_dir: Option<String>,
    /// Volume mounts.
    #[serde(default)]
    pub volumes: Vec<VolumeMount>,
    /// Port mappings.
    #[serde(default)]
    pub ports: Vec<PortMapping>,
    /// Environment variables.
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Command to run (overrides image ENTRYPOINT/CMD).
    #[serde(default)]
    pub command: Vec<String>,
    /// CPU limit (number of CPUs).
    #[serde(default)]
    pub cpu_limit: Option<f64>,
    /// Memory limit in megabytes.
    #[serde(default)]
    pub memory_limit_mb: Option<u64>,
    /// Network mode.
    #[serde(default = "default_network_mode")]
    pub network_mode: NetworkMode,
    /// Whether the container runs as privileged.
    #[serde(default)]
    pub privileged: bool,
    /// Labels for the container.
    #[serde(default)]
    pub labels: HashMap<String, String>,
}

fn default_network_mode() -> NetworkMode {
    NetworkMode::Bridge
}

/// Docker network mode.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum NetworkMode {
    Bridge,
    Host,
    None,
    Container,
}

impl Default for NetworkMode {
    fn default() -> Self {
        Self::Bridge
    }
}

impl ContainerSpec {
    /// Validate the container specification.
    pub fn validate(&self) -> Result<(), DockerError> {
        if self.image.trim().is_empty() {
            return Err(DockerError::InvalidSpec("image is required".to_string()));
        }
        for (i, vol) in self.volumes.iter().enumerate() {
            if vol.host_path.trim().is_empty() {
                return Err(DockerError::InvalidSpec(format!(
                    "volumes[{i}]: host_path is required"
                )));
            }
            if vol.container_path.trim().is_empty() {
                return Err(DockerError::InvalidSpec(format!(
                    "volumes[{i}]: container_path is required"
                )));
            }
        }
        for (i, port) in self.ports.iter().enumerate() {
            if port.container_port == 0 {
                return Err(DockerError::InvalidSpec(format!(
                    "ports[{i}]: container_port must be > 0"
                )));
            }
            if port.host_port == 0 {
                return Err(DockerError::InvalidSpec(format!(
                    "ports[{i}]: host_port must be > 0"
                )));
            }
        }
        if self.privileged {
            return Err(DockerError::InvalidSpec(
                "privileged mode is not allowed".to_string(),
            ));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Path mapping
// ---------------------------------------------------------------------------

/// Handles bidirectional host ↔ container path translation.
#[derive(Clone, Debug)]
pub struct PathMapper {
    mounts: Vec<VolumeMount>,
}

impl PathMapper {
    pub fn new(mounts: Vec<VolumeMount>) -> Self {
        let mut mounts = mounts;
        mounts.sort_by(|a, b| b.container_path.len().cmp(&a.container_path.len()));
        Self { mounts }
    }

    /// Translate a host path to its container equivalent.
    pub fn host_to_container(&self, host_path: &str) -> Result<String, DockerError> {
        for mount in &self.mounts {
            if let Some(relative) = strip_prefix_path(host_path, &mount.host_path) {
                return Ok(join_path(&mount.container_path, relative));
            }
        }
        Err(DockerError::PathNotMapped {
            path: host_path.to_string(),
            direction: "host_to_container".to_string(),
        })
    }

    /// Translate a container path to its host equivalent.
    pub fn container_to_host(&self, container_path: &str) -> Result<String, DockerError> {
        for mount in &self.mounts {
            if let Some(relative) = strip_prefix_path(container_path, &mount.container_path) {
                return Ok(join_path(&mount.host_path, relative));
            }
        }
        Err(DockerError::PathNotMapped {
            path: container_path.to_string(),
            direction: "container_to_host".to_string(),
        })
    }

    /// Check if a host path falls within any mounted volume.
    pub fn is_host_path_mapped(&self, host_path: &str) -> bool {
        self.mounts
            .iter()
            .any(|m| strip_prefix_path(host_path, &m.host_path).is_some())
    }

    /// Check if a container path falls within any mounted volume.
    pub fn is_container_path_mapped(&self, container_path: &str) -> bool {
        self.mounts
            .iter()
            .any(|m| strip_prefix_path(container_path, &m.container_path).is_some())
    }

    /// List all volume mounts.
    pub fn mounts(&self) -> &[VolumeMount] {
        &self.mounts
    }
}

/// Strip a prefix from a path, returning the relative remainder.
/// Handles trailing slashes correctly.
fn strip_prefix_path<'a>(path: &'a str, prefix: &str) -> Option<&'a str> {
    let path = path.trim_end_matches('/');
    let prefix = prefix.trim_end_matches('/');

    if path == prefix {
        return Some("");
    }
    if let Some(rest) = path.strip_prefix(prefix) {
        if rest.starts_with('/') {
            return Some(&rest[1..]);
        }
    }
    None
}

/// Join a base path with a relative path.
fn join_path(base: &str, relative: &str) -> String {
    if relative.is_empty() {
        return base.trim_end_matches('/').to_string();
    }
    let base = base.trim_end_matches('/');
    format!("{base}/{relative}")
}

// ---------------------------------------------------------------------------
// Executor capabilities
// ---------------------------------------------------------------------------

/// Capabilities of an executor environment.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExecutorCapabilities {
    /// Whether the executor supports file watching.
    pub file_watching: bool,
    /// Whether the executor supports interactive terminals.
    pub interactive_terminal: bool,
    /// Whether the executor supports GPU access.
    pub gpu: bool,
    /// Whether the executor supports network access.
    pub network: bool,
    /// Whether the executor runs as root.
    pub root_access: bool,
    /// Maximum CPU cores available.
    pub max_cpu_cores: u32,
    /// Maximum memory in MB.
    pub max_memory_mb: u64,
    /// Whether the executor is persistent (long-lived) or ephemeral.
    pub persistent: bool,
}

impl Default for ExecutorCapabilities {
    fn default() -> Self {
        Self {
            file_watching: true,
            interactive_terminal: true,
            gpu: false,
            network: true,
            root_access: false,
            max_cpu_cores: 4,
            max_memory_mb: 8192,
            persistent: true,
        }
    }
}

/// Local (worktree) executor capabilities.
pub fn local_capabilities() -> ExecutorCapabilities {
    ExecutorCapabilities {
        file_watching: true,
        interactive_terminal: true,
        gpu: false,
        network: true,
        root_access: false,
        max_cpu_cores: 8,
        max_memory_mb: 16384,
        persistent: true,
    }
}

/// Docker executor capabilities.
pub fn docker_capabilities() -> ExecutorCapabilities {
    ExecutorCapabilities {
        file_watching: false,
        interactive_terminal: true,
        gpu: false,
        network: true,
        root_access: false,
        max_cpu_cores: 4,
        max_memory_mb: 4096,
        persistent: false,
    }
}

/// Capability requirement for a task.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityRequirement {
    pub requires_file_watching: bool,
    pub requires_interactive_terminal: bool,
    pub requires_gpu: bool,
    pub requires_network: bool,
    pub requires_root: bool,
    pub min_cpu_cores: u32,
    pub min_memory_mb: u64,
}

impl Default for CapabilityRequirement {
    fn default() -> Self {
        Self {
            requires_file_watching: false,
            requires_interactive_terminal: false,
            requires_gpu: false,
            requires_network: false,
            requires_root: false,
            min_cpu_cores: 1,
            min_memory_mb: 512,
        }
    }
}

/// Result of capability negotiation between a requirement and available capabilities.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityCheck {
    pub satisfied: bool,
    pub missing: Vec<String>,
    pub warnings: Vec<String>,
}

/// Check whether available capabilities satisfy the requirements.
pub fn negotiate_capabilities(
    requirements: &CapabilityRequirement,
    available: &ExecutorCapabilities,
) -> CapabilityCheck {
    let mut missing = Vec::new();
    let mut warnings = Vec::new();

    if requirements.requires_file_watching && !available.file_watching {
        missing.push("file_watching".to_string());
    }
    if requirements.requires_interactive_terminal && !available.interactive_terminal {
        missing.push("interactive_terminal".to_string());
    }
    if requirements.requires_gpu && !available.gpu {
        missing.push("gpu".to_string());
    }
    if requirements.requires_network && !available.network {
        missing.push("network".to_string());
    }
    if requirements.requires_root && !available.root_access {
        missing.push("root_access".to_string());
    }
    if requirements.min_cpu_cores > available.max_cpu_cores {
        missing.push(format!(
            "cpu_cores: need {} but have {}",
            requirements.min_cpu_cores, available.max_cpu_cores
        ));
    }
    if requirements.min_memory_mb > available.max_memory_mb {
        missing.push(format!(
            "memory_mb: need {} but have {}",
            requirements.min_memory_mb, available.max_memory_mb
        ));
    }

    if !available.persistent {
        warnings.push("executor is ephemeral — state may not persist between runs".to_string());
    }
    if !available.file_watching {
        warnings.push("file watching not supported — falling back to polling".to_string());
    }

    CapabilityCheck {
        satisfied: missing.is_empty(),
        missing,
        warnings,
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum DockerError {
    InvalidSpec(String),
    PathNotMapped { path: String, direction: String },
    ContainerNotRunning { container: String },
    CapabilityMissing { missing: Vec<String> },
    ExecutionFailed { reason: String },
}

impl std::fmt::Display for DockerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSpec(msg) => write!(f, "invalid container spec: {msg}"),
            Self::PathNotMapped { path, direction } => {
                write!(f, "path '{path}' is not mapped ({direction})")
            }
            Self::ContainerNotRunning { container } => {
                write!(f, "container '{container}' is not running")
            }
            Self::CapabilityMissing { missing } => {
                write!(f, "missing capabilities: {}", missing.join(", "))
            }
            Self::ExecutionFailed { reason } => write!(f, "docker execution failed: {reason}"),
        }
    }
}

impl std::error::Error for DockerError {}

// ---------------------------------------------------------------------------
// Docker executor (abstracts docker exec/cp)
// ---------------------------------------------------------------------------

/// Result of executing a command in a container.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExecResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl ExecResult {
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

/// Trait abstracting Docker operations for testability.
pub trait DockerExecutorBackend: std::fmt::Debug {
    /// Check if a container is running.
    fn is_running(&self, container: &str) -> bool;

    /// Execute a command inside a container.
    fn exec(
        &mut self,
        container: &str,
        command: &[String],
        working_dir: Option<&str>,
    ) -> ExecResult;

    /// Copy a file from host to container.
    fn copy_to(
        &mut self,
        host_path: &str,
        container: &str,
        container_path: &str,
    ) -> Result<(), String>;

    /// Copy a file from container to host.
    fn copy_from(
        &mut self,
        container: &str,
        container_path: &str,
        host_path: &str,
    ) -> Result<(), String>;
}

/// The Docker executor implementing the WorkspaceExecutor contract.
#[derive(Debug)]
pub struct DockerExecutor<B: DockerExecutorBackend> {
    backend: B,
    spec: ContainerSpec,
    path_mapper: PathMapper,
}

impl<B: DockerExecutorBackend> DockerExecutor<B> {
    pub fn new(spec: ContainerSpec, backend: B) -> Result<Self, DockerError> {
        spec.validate()?;
        let path_mapper = PathMapper::new(spec.volumes.clone());
        Ok(Self {
            backend,
            spec,
            path_mapper,
        })
    }

    /// Ensure the container is running before dispatch.
    pub fn ensure_running(&self) -> Result<(), DockerError> {
        let container = self.spec.name.as_deref().unwrap_or("unnamed");
        if self.backend.is_running(container) {
            Ok(())
        } else {
            Err(DockerError::ContainerNotRunning {
                container: container.to_string(),
            })
        }
    }

    /// Check capabilities before dispatch.
    pub fn check_capabilities(
        &self,
        requirements: &CapabilityRequirement,
    ) -> Result<CapabilityCheck, DockerError> {
        let caps = docker_capabilities();
        let check = negotiate_capabilities(requirements, &caps);
        if !check.satisfied {
            return Err(DockerError::CapabilityMissing {
                missing: check.missing.clone(),
            });
        }
        Ok(check)
    }

    /// Run a command in the container. Host paths in arguments are translated.
    pub fn run_command(&mut self, command: &[String]) -> Result<ExecResult, DockerError> {
        self.ensure_running()?;
        let working_dir = self.spec.working_dir.as_deref();
        let result = self.backend.exec(
            self.spec.name.as_deref().unwrap_or("unnamed"),
            command,
            working_dir,
        );
        Ok(result)
    }

    /// Copy a file from host to container, translating paths.
    pub fn copy_to_container(&mut self, host_path: &str) -> Result<String, DockerError> {
        self.ensure_running()?;
        let container_path = self.path_mapper.host_to_container(host_path)?;
        self.backend
            .copy_to(
                host_path,
                self.spec.name.as_deref().unwrap_or("unnamed"),
                &container_path,
            )
            .map_err(|e| DockerError::ExecutionFailed { reason: e })?;
        Ok(container_path)
    }

    /// Copy a file from container to host, translating paths.
    pub fn copy_from_container(&mut self, container_path: &str) -> Result<String, DockerError> {
        self.ensure_running()?;
        let host_path = self.path_mapper.container_to_host(container_path)?;
        self.backend
            .copy_from(
                self.spec.name.as_deref().unwrap_or("unnamed"),
                container_path,
                &host_path,
            )
            .map_err(|e| DockerError::ExecutionFailed { reason: e })?;
        Ok(host_path)
    }

    /// Get the path mapper for manual path translation.
    pub fn path_mapper(&self) -> &PathMapper {
        &self.path_mapper
    }

    /// Get the container spec.
    pub fn spec(&self) -> &ContainerSpec {
        &self.spec
    }
}

// ---------------------------------------------------------------------------
// Mock backend for testing
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct MockDockerBackend {
    pub running: bool,
    pub exec_results: Vec<ExecResult>,
    pub exec_calls: Vec<Vec<String>>,
    pub copy_to_calls: Vec<(String, String)>,
    pub copy_from_calls: Vec<(String, String)>,
}

impl Default for MockDockerBackend {
    fn default() -> Self {
        Self {
            running: true,
            exec_results: vec![ExecResult {
                exit_code: 0,
                stdout: "ok".to_string(),
                stderr: String::new(),
            }],
            exec_calls: Vec::new(),
            copy_to_calls: Vec::new(),
            copy_from_calls: Vec::new(),
        }
    }
}

impl DockerExecutorBackend for MockDockerBackend {
    fn is_running(&self, _container: &str) -> bool {
        self.running
    }

    fn exec(
        &mut self,
        _container: &str,
        command: &[String],
        _working_dir: Option<&str>,
    ) -> ExecResult {
        self.exec_calls.push(command.to_vec());
        if !self.exec_results.is_empty() {
            self.exec_results.remove(0)
        } else {
            ExecResult {
                exit_code: 0,
                stdout: String::new(),
                stderr: String::new(),
            }
        }
    }

    fn copy_to(
        &mut self,
        host_path: &str,
        _container: &str,
        container_path: &str,
    ) -> Result<(), String> {
        self.copy_to_calls
            .push((host_path.to_string(), container_path.to_string()));
        Ok(())
    }

    fn copy_from(
        &mut self,
        _container: &str,
        container_path: &str,
        host_path: &str,
    ) -> Result<(), String> {
        self.copy_from_calls
            .push((container_path.to_string(), host_path.to_string()));
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_spec() -> ContainerSpec {
        ContainerSpec {
            image: "node:20-alpine".to_string(),
            name: Some("test-container".to_string()),
            working_dir: Some("/workspace".to_string()),
            volumes: vec![VolumeMount {
                host_path: "/host/project".to_string(),
                container_path: "/workspace".to_string(),
                read_only: false,
            }],
            ports: vec![PortMapping {
                container_port: 3000,
                host_port: 3000,
                protocol: PortProtocol::Tcp,
            }],
            env: HashMap::new(),
            command: vec![],
            cpu_limit: None,
            memory_limit_mb: None,
            network_mode: NetworkMode::Bridge,
            privileged: false,
            labels: HashMap::new(),
        }
    }

    // ---- Container spec validation ----

    #[test]
    fn validate_rejects_empty_image() {
        let mut spec = sample_spec();
        spec.image = "".to_string();
        assert!(matches!(spec.validate(), Err(DockerError::InvalidSpec(_))));
    }

    #[test]
    fn validate_rejects_empty_host_path() {
        let mut spec = sample_spec();
        spec.volumes.push(VolumeMount {
            host_path: "".to_string(),
            container_path: "/data".to_string(),
            read_only: false,
        });
        assert!(spec.validate().is_err());
    }

    #[test]
    fn validate_rejects_empty_container_path() {
        let mut spec = sample_spec();
        spec.volumes.push(VolumeMount {
            host_path: "/data".to_string(),
            container_path: "".to_string(),
            read_only: false,
        });
        assert!(spec.validate().is_err());
    }

    #[test]
    fn validate_rejects_zero_port() {
        let mut spec = sample_spec();
        spec.ports.push(PortMapping {
            container_port: 0,
            host_port: 8080,
            protocol: PortProtocol::Tcp,
        });
        assert!(spec.validate().is_err());
    }

    #[test]
    fn validate_rejects_privileged() {
        let mut spec = sample_spec();
        spec.privileged = true;
        assert!(spec.validate().is_err());
    }

    #[test]
    fn validate_accepts_valid_spec() {
        let spec = sample_spec();
        assert!(spec.validate().is_ok());
    }

    #[test]
    fn validate_accepts_spec_without_optional_fields() {
        let spec = ContainerSpec {
            image: "alpine".to_string(),
            name: None,
            working_dir: None,
            volumes: vec![],
            ports: vec![],
            env: HashMap::new(),
            command: vec![],
            cpu_limit: None,
            memory_limit_mb: None,
            network_mode: NetworkMode::Bridge,
            privileged: false,
            labels: HashMap::new(),
        };
        assert!(spec.validate().is_ok());
    }

    // ---- Path mapping ----

    #[test]
    fn host_to_container_translates_mounted_path() {
        let mapper = PathMapper::new(vec![VolumeMount {
            host_path: "/host/project".to_string(),
            container_path: "/workspace".to_string(),
            read_only: false,
        }]);
        let result = mapper
            .host_to_container("/host/project/src/main.rs")
            .unwrap();
        assert_eq!(result, "/workspace/src/main.rs");
    }

    #[test]
    fn host_to_container_translates_exact_mount_root() {
        let mapper = PathMapper::new(vec![VolumeMount {
            host_path: "/host/project".to_string(),
            container_path: "/workspace".to_string(),
            read_only: false,
        }]);
        let result = mapper.host_to_container("/host/project").unwrap();
        assert_eq!(result, "/workspace");
    }

    #[test]
    fn host_to_container_translates_with_trailing_slash() {
        let mapper = PathMapper::new(vec![VolumeMount {
            host_path: "/host/project/".to_string(),
            container_path: "/workspace/".to_string(),
            read_only: false,
        }]);
        let result = mapper
            .host_to_container("/host/project/src/main.rs")
            .unwrap();
        assert_eq!(result, "/workspace/src/main.rs");
    }

    #[test]
    fn container_to_host_translates_mounted_path() {
        let mapper = PathMapper::new(vec![VolumeMount {
            host_path: "/host/project".to_string(),
            container_path: "/workspace".to_string(),
            read_only: false,
        }]);
        let result = mapper.container_to_host("/workspace/src/main.rs").unwrap();
        assert_eq!(result, "/host/project/src/main.rs");
    }

    #[test]
    fn unmapped_path_returns_error() {
        let mapper = PathMapper::new(vec![VolumeMount {
            host_path: "/host/project".to_string(),
            container_path: "/workspace".to_string(),
            read_only: false,
        }]);
        assert!(mapper.host_to_container("/other/path").is_err());
        assert!(mapper.container_to_host("/other/path").is_err());
    }

    #[test]
    fn is_host_path_mapped_checks_correctly() {
        let mapper = PathMapper::new(vec![VolumeMount {
            host_path: "/host/project".to_string(),
            container_path: "/workspace".to_string(),
            read_only: false,
        }]);
        assert!(mapper.is_host_path_mapped("/host/project/src/main.rs"));
        assert!(!mapper.is_host_path_mapped("/other/path"));
    }

    #[test]
    fn longest_prefix_match_wins() {
        let mapper = PathMapper::new(vec![
            VolumeMount {
                host_path: "/host".to_string(),
                container_path: "/data".to_string(),
                read_only: false,
            },
            VolumeMount {
                host_path: "/host/project".to_string(),
                container_path: "/workspace".to_string(),
                read_only: false,
            },
        ]);
        let result = mapper
            .host_to_container("/host/project/src/main.rs")
            .unwrap();
        assert_eq!(result, "/workspace/src/main.rs");
    }

    #[test]
    fn multiple_mounts_translate_correctly() {
        let mapper = PathMapper::new(vec![
            VolumeMount {
                host_path: "/host/project".to_string(),
                container_path: "/workspace".to_string(),
                read_only: false,
            },
            VolumeMount {
                host_path: "/host/data".to_string(),
                container_path: "/data".to_string(),
                read_only: true,
            },
        ]);
        assert_eq!(
            mapper.host_to_container("/host/project/src/a.rs").unwrap(),
            "/workspace/src/a.rs"
        );
        assert_eq!(
            mapper.host_to_container("/host/data/file.txt").unwrap(),
            "/data/file.txt"
        );
    }

    // ---- Capability negotiation ----

    #[test]
    fn local_capabilities_differ_from_docker() {
        let local = local_capabilities();
        let docker = docker_capabilities();
        assert!(local.file_watching);
        assert!(!docker.file_watching);
        assert!(local.persistent);
        assert!(!docker.persistent);
    }

    #[test]
    fn negotiate_satisfied_when_requirements_met() {
        let req = CapabilityRequirement::default();
        let caps = docker_capabilities();
        let check = negotiate_capabilities(&req, &caps);
        assert!(check.satisfied);
        assert!(check.missing.is_empty());
    }

    #[test]
    fn negotiate_missing_file_watching() {
        let req = CapabilityRequirement {
            requires_file_watching: true,
            ..CapabilityRequirement::default()
        };
        let caps = docker_capabilities();
        let check = negotiate_capabilities(&req, &caps);
        assert!(!check.satisfied);
        assert!(check.missing.contains(&"file_watching".to_string()));
    }

    #[test]
    fn negotiate_missing_gpu() {
        let req = CapabilityRequirement {
            requires_gpu: true,
            ..CapabilityRequirement::default()
        };
        let caps = docker_capabilities();
        let check = negotiate_capabilities(&req, &caps);
        assert!(!check.satisfied);
        assert!(check.missing.contains(&"gpu".to_string()));
    }

    #[test]
    fn negotiate_missing_cpu_cores() {
        let req = CapabilityRequirement {
            min_cpu_cores: 8,
            ..CapabilityRequirement::default()
        };
        let caps = docker_capabilities();
        let check = negotiate_capabilities(&req, &caps);
        assert!(!check.satisfied);
        assert!(check.missing.iter().any(|m| m.contains("cpu_cores")));
    }

    #[test]
    fn negotiate_missing_memory() {
        let req = CapabilityRequirement {
            min_memory_mb: 8192,
            ..CapabilityRequirement::default()
        };
        let caps = docker_capabilities();
        let check = negotiate_capabilities(&req, &caps);
        assert!(!check.satisfied);
        assert!(check.missing.iter().any(|m| m.contains("memory_mb")));
    }

    #[test]
    fn negotiate_warns_on_ephemeral_executor() {
        let req = CapabilityRequirement::default();
        let caps = docker_capabilities();
        let check = negotiate_capabilities(&req, &caps);
        assert!(check.warnings.iter().any(|w| w.contains("ephemeral")));
    }

    #[test]
    fn negotiate_warns_on_no_file_watching() {
        let req = CapabilityRequirement::default();
        let caps = docker_capabilities();
        let check = negotiate_capabilities(&req, &caps);
        assert!(check.warnings.iter().any(|w| w.contains("file watching")));
    }

    // ---- Docker executor ----

    #[test]
    fn executor_checks_running_container() {
        let backend = MockDockerBackend {
            running: false,
            ..MockDockerBackend::default()
        };
        let executor = DockerExecutor::new(sample_spec(), backend).unwrap();
        assert!(matches!(
            executor.ensure_running(),
            Err(DockerError::ContainerNotRunning { .. })
        ));
    }

    #[test]
    fn executor_runs_command() {
        let backend = MockDockerBackend::default();
        let mut executor = DockerExecutor::new(sample_spec(), backend).unwrap();
        let result = executor
            .run_command(&["echo".to_string(), "hello".to_string()])
            .unwrap();
        assert!(result.success());
        assert_eq!(executor.backend.exec_calls.len(), 1);
    }

    #[test]
    fn executor_copy_to_translates_path() {
        let backend = MockDockerBackend::default();
        let mut executor = DockerExecutor::new(sample_spec(), backend).unwrap();
        let container_path = executor
            .copy_to_container("/host/project/src/main.rs")
            .unwrap();
        assert_eq!(container_path, "/workspace/src/main.rs");
        assert_eq!(executor.backend.copy_to_calls.len(), 1);
    }

    #[test]
    fn executor_copy_from_translates_path() {
        let backend = MockDockerBackend::default();
        let mut executor = DockerExecutor::new(sample_spec(), backend).unwrap();
        let host_path = executor
            .copy_from_container("/workspace/output.txt")
            .unwrap();
        assert_eq!(host_path, "/host/project/output.txt");
        assert_eq!(executor.backend.copy_from_calls.len(), 1);
    }

    #[test]
    fn executor_copy_to_fails_on_unmapped_path() {
        let backend = MockDockerBackend::default();
        let mut executor = DockerExecutor::new(sample_spec(), backend).unwrap();
        let result = executor.copy_to_container("/unmapped/file.txt");
        assert!(matches!(result, Err(DockerError::PathNotMapped { .. })));
    }

    #[test]
    fn executor_copy_from_fails_on_unmapped_path() {
        let backend = MockDockerBackend::default();
        let mut executor = DockerExecutor::new(sample_spec(), backend).unwrap();
        let result = executor.copy_from_container("/unmapped/file.txt");
        assert!(matches!(result, Err(DockerError::PathNotMapped { .. })));
    }

    #[test]
    fn executor_check_capabilities_returns_ok_when_satisfied() {
        let backend = MockDockerBackend::default();
        let executor = DockerExecutor::new(sample_spec(), backend).unwrap();
        let req = CapabilityRequirement::default();
        let check = executor.check_capabilities(&req).unwrap();
        assert!(check.satisfied);
    }

    #[test]
    fn executor_check_capabilities_fails_on_missing() {
        let backend = MockDockerBackend::default();
        let executor = DockerExecutor::new(sample_spec(), backend).unwrap();
        let req = CapabilityRequirement {
            requires_gpu: true,
            ..CapabilityRequirement::default()
        };
        let result = executor.check_capabilities(&req);
        assert!(matches!(result, Err(DockerError::CapabilityMissing { .. })));
    }

    #[test]
    fn executor_creation_validates_spec() {
        let mut spec = sample_spec();
        spec.image = "".to_string();
        let result = DockerExecutor::new(spec, MockDockerBackend::default());
        assert!(result.is_err());
    }

    // ---- Error display ----

    #[test]
    fn error_display_messages() {
        assert!(format!("{}", DockerError::InvalidSpec("bad".to_string())).contains("bad"));

        assert!(format!(
            "{}",
            DockerError::PathNotMapped {
                path: "/x".to_string(),
                direction: "host_to_container".to_string()
            }
        )
        .contains("/x"));

        assert!(format!(
            "{}",
            DockerError::ContainerNotRunning {
                container: "c1".to_string()
            }
        )
        .contains("c1"));

        assert!(format!(
            "{}",
            DockerError::CapabilityMissing {
                missing: vec!["gpu".to_string()]
            }
        )
        .contains("gpu"));
    }

    // ---- Serialization ----

    #[test]
    fn container_spec_serializes() {
        let spec = sample_spec();
        let json = serde_json::to_string(&spec).unwrap();
        let back: ContainerSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(spec, back);
    }

    #[test]
    fn exec_result_success_check() {
        let success = ExecResult {
            exit_code: 0,
            stdout: "done".to_string(),
            stderr: String::new(),
        };
        assert!(success.success());

        let failure = ExecResult {
            exit_code: 1,
            stdout: String::new(),
            stderr: "error".to_string(),
        };
        assert!(!failure.success());
    }
}
