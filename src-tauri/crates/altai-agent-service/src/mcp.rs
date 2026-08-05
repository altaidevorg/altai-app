//! Host-neutral Model Context Protocol configuration and stdio runtime.
//!
//! Desktop and non-desktop hosts use this module rather than each growing a
//! separate MCP client. Hosts retain authority over workspace admission and
//! expose lifecycle operations through their own protocol adapters.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use isanagent::tools::ToolRegistry;
use isanagent::traits::Tool;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{oneshot, Mutex};

const CONFIG_FILE: &str = "mcp.json";
const PROTOCOL_VERSION: &str = "2024-11-05";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerConfig {
    #[serde(skip_serializing, default)]
    pub id: String,
    #[serde(default, deserialize_with = "deserialize_name_fallback")]
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default = "enabled_by_default")]
    pub enabled: bool,
}

fn deserialize_name_fallback<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_default())
}

fn enabled_by_default() -> bool {
    true
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpProbeTool {
    pub name: String,
    pub description: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpProbeResult {
    pub tools: Vec<McpProbeTool>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerStatus {
    pub server_id: String,
    pub state: McpState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    pub updated_at_ms: u64,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum McpState {
    Starting,
    Connected,
    Error,
}

#[derive(Clone, Default)]
pub struct McpStatusRegistry {
    inner: Arc<Mutex<HashMap<String, McpServerStatus>>>,
}

impl McpStatusRegistry {
    pub fn new() -> Self {
        Self::default()
    }
    fn key(workspace: &Path, server_id: &str) -> String {
        format!("{}::{server_id}", workspace.display())
    }
    pub async fn set(&self, workspace: &Path, mut status: McpServerStatus) {
        status.updated_at_ms = now_epoch_ms();
        self.inner
            .lock()
            .await
            .insert(Self::key(workspace, &status.server_id), status);
    }
    pub async fn snapshot(&self, workspace: &Path) -> Vec<McpServerStatus> {
        let prefix = format!("{}::", workspace.display());
        let mut result: Vec<_> = self
            .inner
            .lock()
            .await
            .iter()
            .filter(|(key, _)| key.starts_with(&prefix))
            .map(|(_, status)| status.clone())
            .collect();
        result.sort_by(|left, right| left.server_id.cmp(&right.server_id));
        result
    }
    pub async fn clear_workspace(&self, workspace: &Path) {
        let prefix = format!("{}::", workspace.display());
        self.inner
            .lock()
            .await
            .retain(|key, _| !key.starts_with(&prefix));
    }
    pub async fn clear_server(&self, workspace: &Path, server_id: &str) {
        self.inner
            .lock()
            .await
            .remove(&Self::key(workspace, server_id));
    }
}

pub fn validate_server(server: &McpServerConfig) -> Result<(), String> {
    if server.id.trim().is_empty()
        || !server
            .id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err("Server id may only contain letters, numbers, '-' and '_'.".into());
    }
    if server.name.trim().is_empty() {
        return Err("Server name is required.".into());
    }
    if server.command.trim().is_empty() {
        return Err("Server command is required.".into());
    }
    Ok(())
}

fn config_path(workspace: &Path) -> PathBuf {
    workspace.join(".isanagent").join(CONFIG_FILE)
}

pub fn load_servers(workspace: &Path) -> Result<Vec<McpServerConfig>, String> {
    let path = config_path(workspace);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(&path)
        .map_err(|error| format!("Could not read MCP config: {error}"))?;
    let parsed: Value =
        serde_json::from_str(&raw).map_err(|error| format!("Invalid MCP config: {error}"))?;
    let mut servers = match parsed {
        Value::Object(object) if object.contains_key("mcpServers") => {
            let map = object
                .get("mcpServers")
                .and_then(Value::as_object)
                .ok_or_else(|| "MCP config 'mcpServers' must be an object.".to_string())?;
            map.iter()
                .map(|(id, body)| {
                    let mut server: McpServerConfig = serde_json::from_value(body.clone())
                        .map_err(|error| format!("Invalid MCP server '{id}': {error}"))?;
                    server.id = id.clone();
                    Ok(server)
                })
                .collect::<Result<Vec<_>, String>>()?
        }
        Value::Array(entries) => entries
            .into_iter()
            .map(serde_json::from_value)
            .collect::<Result<Vec<McpServerConfig>, _>>()
            .map_err(|error| format!("Invalid MCP server entry: {error}"))?,
        _ => return Err("MCP config must be a `mcpServers` object or an array of servers.".into()),
    };
    for server in &mut servers {
        if server.name.trim().is_empty() {
            server.name = server.id.clone();
        }
        validate_server(server)?;
    }
    servers.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(servers)
}

pub fn save_servers(workspace: &Path, servers: &[McpServerConfig]) -> Result<(), String> {
    let mut ids = HashSet::new();
    for server in servers {
        validate_server(server)?;
        if !ids.insert(&server.id) {
            return Err(format!("Duplicate MCP server id: {}", server.id));
        }
    }
    let path = config_path(workspace);
    let parent = path
        .parent()
        .ok_or_else(|| "Invalid MCP config path".to_string())?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("Could not create MCP directory: {error}"))?;
    let mut map = serde_json::Map::new();
    for server in servers {
        map.insert(
            server.id.clone(),
            serde_json::to_value(server)
                .map_err(|error| format!("Could not serialize MCP config: {error}"))?,
        );
    }
    let encoded = serde_json::to_string_pretty(&json!({ "mcpServers": map }))
        .map_err(|error| format!("Could not serialize MCP config: {error}"))?;
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, encoded)
        .map_err(|error| format!("Could not write MCP config: {error}"))?;
    std::fs::rename(&temporary, &path)
        .map_err(|error| format!("Could not save MCP config: {error}"))
}

#[derive(Debug, Deserialize)]
struct ToolsListResult {
    #[serde(default)]
    tools: Vec<RemoteTool>,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteTool {
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default = "empty_schema")]
    input_schema: Value,
}
fn empty_schema() -> Value {
    json!({ "type": "object", "properties": {} })
}

type Pending = Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value, String>>>>>;
struct McpClient {
    writer: Mutex<ChildStdin>,
    child: Mutex<Child>,
    pending: Pending,
    next_id: AtomicU64,
}

impl Drop for McpClient {
    fn drop(&mut self) {
        if let Ok(mut child) = self.child.try_lock() {
            let _ = child.start_kill();
        }
    }
}

impl McpClient {
    async fn start(server: &McpServerConfig, cwd: &Path) -> Result<Arc<Self>, String> {
        let mut command = Command::new(server.command.trim());
        command
            .args(&server.args)
            .current_dir(cwd)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_ASKPASS", "")
            .env("SSH_ASKPASS", "")
            .envs(&server.env);
        let mut child = command
            .spawn()
            .map_err(|error| format!("Could not start '{}': {error}", server.name))?;
        let writer = child
            .stdin
            .take()
            .ok_or_else(|| "MCP stdin unavailable".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "MCP stdout unavailable".to_string())?;
        let pending: Pending = Arc::new(Mutex::new(HashMap::new()));
        let pending_for_reader = pending.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let Ok(message) = serde_json::from_str::<Value>(&line) else {
                    continue;
                };
                let Some(id) = message.get("id").and_then(Value::as_u64) else {
                    continue;
                };
                let result = if let Some(error) = message.get("error") {
                    Err(error
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or("MCP server returned an error")
                        .to_string())
                } else {
                    Ok(message.get("result").cloned().unwrap_or(Value::Null))
                };
                if let Some(sender) = pending_for_reader.lock().await.remove(&id) {
                    let _ = sender.send(result);
                }
            }
            for (_, sender) in pending_for_reader.lock().await.drain() {
                let _ = sender.send(Err("MCP server closed its connection.".into()));
            }
        });
        Ok(Arc::new(Self {
            writer: Mutex::new(writer),
            child: Mutex::new(child),
            pending,
            next_id: AtomicU64::new(1),
        }))
    }
    async fn request(&self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (sender, receiver) = oneshot::channel();
        self.pending.lock().await.insert(id, sender);
        let message = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        let written = async {
            let mut writer = self.writer.lock().await;
            writer
                .write_all(message.to_string().as_bytes())
                .await
                .map_err(|error| format!("MCP write failed: {error}"))?;
            writer
                .write_all(b"\n")
                .await
                .map_err(|error| format!("MCP write failed: {error}"))?;
            writer
                .flush()
                .await
                .map_err(|error| format!("MCP flush failed: {error}"))
        }
        .await;
        if let Err(error) = written {
            self.pending.lock().await.remove(&id);
            return Err(error);
        }
        match tokio::time::timeout(Duration::from_secs(60), receiver).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err("MCP response channel closed.".into()),
            Err(_) => {
                self.pending.lock().await.remove(&id);
                Err(format!("MCP request '{method}' timed out."))
            }
        }
    }
    async fn notify(&self, method: &str, params: Value) -> Result<(), String> {
        let message = json!({ "jsonrpc": "2.0", "method": method, "params": params });
        let mut writer = self.writer.lock().await;
        writer
            .write_all(message.to_string().as_bytes())
            .await
            .map_err(|error| format!("MCP write failed: {error}"))?;
        writer
            .write_all(b"\n")
            .await
            .map_err(|error| format!("MCP write failed: {error}"))?;
        writer
            .flush()
            .await
            .map_err(|error| format!("MCP flush failed: {error}"))
    }
}

pub struct McpTool {
    name: String,
    description: String,
    parameters: Value,
    remote_name: String,
    client: Arc<McpClient>,
}
#[async_trait::async_trait]
impl Tool for McpTool {
    fn name(&self) -> &str {
        &self.name
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn parameters(&self) -> Value {
        self.parameters.clone()
    }
    async fn execute(&self, args: Value) -> Result<String, String> {
        serde_json::to_string(
            &self
                .client
                .request(
                    "tools/call",
                    json!({ "name": self.remote_name, "arguments": args }),
                )
                .await?,
        )
        .map_err(|error| format!("Could not read MCP tool result: {error}"))
    }
}

pub async fn connect_server(server: &McpServerConfig, cwd: &Path) -> Result<Vec<McpTool>, String> {
    let client = McpClient::start(server, cwd).await?;
    client.request("initialize", json!({ "protocolVersion": PROTOCOL_VERSION, "capabilities": {}, "clientInfo": { "name": "ALTAI", "version": env!("CARGO_PKG_VERSION") } })).await?;
    client
        .notify("notifications/initialized", json!({}))
        .await?;
    let listed: ToolsListResult = serde_json::from_value(
        client.request("tools/list", json!({})).await?,
    )
    .map_err(|error| {
        format!(
            "Invalid tools/list response from '{}': {error}",
            server.name
        )
    })?;
    Ok(listed
        .tools
        .into_iter()
        .map(|remote| McpTool {
            name: tool_name(&server.id, &remote.name),
            description: if remote.description.trim().is_empty() {
                format!("MCP tool '{}' from {}", remote.name, server.name)
            } else {
                format!("[MCP: {}] {}", server.name, remote.description)
            },
            parameters: remote.input_schema,
            remote_name: remote.name,
            client: client.clone(),
        })
        .collect())
}

pub async fn probe_server(server: &McpServerConfig, cwd: &Path) -> Result<McpProbeResult, String> {
    let client = McpClient::start(server, cwd).await?;
    client.request("initialize", json!({ "protocolVersion": PROTOCOL_VERSION, "capabilities": {}, "clientInfo": { "name": "ALTAI", "version": env!("CARGO_PKG_VERSION") } })).await?;
    client
        .notify("notifications/initialized", json!({}))
        .await?;
    let listed: ToolsListResult = serde_json::from_value(
        client.request("tools/list", json!({})).await?,
    )
    .map_err(|error| {
        format!(
            "Invalid tools/list response from '{}': {error}",
            server.name
        )
    })?;
    Ok(McpProbeResult {
        tools: listed
            .tools
            .into_iter()
            .map(|tool| McpProbeTool {
                name: tool.name,
                description: tool.description,
            })
            .collect(),
    })
}

/// Connect every enabled server for one newly-created agent instance.
/// Connections belong to the registered tools and are killed when that
/// instance drops, which makes a subsequent run a real, clean restart.
pub async fn register_enabled_tools(
    workspace: &Path,
    tools: &mut ToolRegistry,
    statuses: &McpStatusRegistry,
) -> Result<(), String> {
    let servers = load_servers(workspace)?;
    let mut connections = tokio::task::JoinSet::new();
    for server in servers.into_iter().filter(|server| server.enabled) {
        statuses
            .set(
                workspace,
                McpServerStatus {
                    server_id: server.id.clone(),
                    state: McpState::Starting,
                    tool_count: None,
                    last_error: None,
                    updated_at_ms: 0,
                },
            )
            .await;
        let cwd = workspace.to_path_buf();
        connections.spawn(async move {
            let outcome = connect_server(&server, &cwd).await;
            (server, outcome)
        });
    }
    while let Some(joined) = connections.join_next().await {
        let Ok((server, outcome)) = joined else {
            continue;
        };
        match outcome {
            Ok(registered) => {
                let count = registered.len();
                for tool in registered {
                    tools.register(Box::new(tool));
                }
                statuses
                    .set(
                        workspace,
                        McpServerStatus {
                            server_id: server.id,
                            state: McpState::Connected,
                            tool_count: Some(count),
                            last_error: None,
                            updated_at_ms: 0,
                        },
                    )
                    .await;
            }
            Err(error) => {
                log::warn!("MCP '{}' unavailable: {error}", server.name);
                statuses
                    .set(
                        workspace,
                        McpServerStatus {
                            server_id: server.id,
                            state: McpState::Error,
                            tool_count: None,
                            last_error: Some(error),
                            updated_at_ms: 0,
                        },
                    )
                    .await;
            }
        }
    }
    Ok(())
}

fn now_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
fn tool_name(server_id: &str, remote_name: &str) -> String {
    format!(
        "mcp__{}__{}",
        normalize_segment(server_id),
        normalize_segment(remote_name)
    )
}
fn normalize_segment(input: &str) -> String {
    let value = input
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    let normalized = value
        .split('-')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if normalized.is_empty() {
        "x".into()
    } else {
        normalized
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn config_round_trips_claude_desktop_shape() {
        let directory = tempfile::tempdir().unwrap();
        let servers = vec![McpServerConfig {
            id: "files".into(),
            name: "Files".into(),
            command: "node".into(),
            args: vec!["server.js".into()],
            env: HashMap::new(),
            enabled: true,
        }];
        save_servers(directory.path(), &servers).unwrap();
        assert_eq!(load_servers(directory.path()).unwrap()[0].id, "files");
    }
}
