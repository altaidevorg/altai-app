//! Desktop command wrappers around the shared, host-neutral MCP runtime.

use std::path::PathBuf;

use altai_agent_service::mcp::{self, McpServerConfig};
use tauri::State;

use super::workspace::WorkspaceRegistry;

pub use altai_agent_service::mcp::{McpServerStatus, McpStatusRegistry};

fn authorized_workspace(
    workspace_path: &str,
    registry: &WorkspaceRegistry,
) -> Result<PathBuf, String> {
    let canonical = registry
        .canonicalize_cached(workspace_path)
        .map_err(|error| format!("Workspace is not accessible: {error}"))?;
    if !canonical.is_dir() || !registry.is_authorized(&canonical) {
        return Err("Workspace is not authorized.".into());
    }
    Ok(canonical)
}

#[tauri::command]
pub fn mcp_get_servers(
    workspace_path: String,
    registry: State<'_, WorkspaceRegistry>,
) -> Result<Vec<McpServerConfig>, String> {
    mcp::load_servers(&authorized_workspace(&workspace_path, &registry)?)
}

#[tauri::command]
pub async fn mcp_server_status(
    workspace_path: String,
    registry: State<'_, WorkspaceRegistry>,
    statuses: State<'_, McpStatusRegistry>,
) -> Result<Vec<McpServerStatus>, String> {
    Ok(statuses
        .snapshot(&authorized_workspace(&workspace_path, &registry)?)
        .await)
}

#[tauri::command]
pub fn mcp_save_servers(
    workspace_path: String,
    servers: Vec<McpServerConfig>,
    registry: State<'_, WorkspaceRegistry>,
) -> Result<(), String> {
    mcp::save_servers(&authorized_workspace(&workspace_path, &registry)?, &servers)
}

#[tauri::command]
pub async fn mcp_probe_server(
    workspace_path: String,
    server: McpServerConfig,
    registry: State<'_, WorkspaceRegistry>,
) -> Result<mcp::McpProbeResult, String> {
    mcp::validate_server(&server)?;
    mcp::probe_server(&server, &authorized_workspace(&workspace_path, &registry)?).await
}
