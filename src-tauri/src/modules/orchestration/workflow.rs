use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::State;

use crate::modules::{
    fs::file::write_atomic,
    workspace::{resolve_path, WorkspaceEnv, WorkspaceRegistry},
};

const WORKFLOW_FILE: &str = "WORKFLOW.md";
const MAX_WORKFLOW_BYTES: u64 = 128 * 1024;
const MAX_PROMPT_CHARS: usize = 32_000;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct WorkflowConfig {
    pub orchestration: SchedulerConfig,
    pub agent: AgentConfig,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct SchedulerConfig {
    pub max_concurrent: usize,
    pub max_attempts: u32,
    pub retry_base_seconds: u64,
    pub retry_max_seconds: u64,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 2,
            max_attempts: 4,
            retry_base_seconds: 5,
            retry_max_seconds: 300,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct AgentConfig {
    pub model_id: Option<String>,
    pub permission_mode: Option<PermissionMode>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PermissionMode {
    Ask,
    AutoEdit,
    Plan,
    Bypass,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowDocument {
    pub exists: bool,
    pub path: String,
    pub content: String,
    pub config: Option<WorkflowConfig>,
    pub prompt: Option<String>,
    pub validation_error: Option<String>,
    pub modified_at_ms: Option<u64>,
}

pub fn validate_config(config: &WorkflowConfig, prompt: &str) -> Result<(), String> {
    let scheduler = &config.orchestration;
    if !(1..=8).contains(&scheduler.max_concurrent) {
        return Err("orchestration.max_concurrent must be between 1 and 8.".to_string());
    }
    if !(1..=10).contains(&scheduler.max_attempts) {
        return Err("orchestration.max_attempts must be between 1 and 10.".to_string());
    }
    if !(1..=3_600).contains(&scheduler.retry_base_seconds) {
        return Err("orchestration.retry_base_seconds must be between 1 and 3600.".to_string());
    }
    if scheduler.retry_max_seconds < scheduler.retry_base_seconds
        || scheduler.retry_max_seconds > 86_400
    {
        return Err(
            "orchestration.retry_max_seconds must be at least retry_base_seconds and no more than 86400."
                .to_string(),
        );
    }
    if let Some(model_id) = config.agent.model_id.as_deref() {
        if model_id.trim().is_empty() || model_id.len() > 128 {
            return Err(
                "agent.model_id must be a non-empty model id under 128 characters.".to_string(),
            );
        }
    }
    if prompt.trim().is_empty() {
        return Err("The workflow prompt cannot be empty.".to_string());
    }
    if prompt.chars().count() > MAX_PROMPT_CHARS {
        return Err(format!(
            "The workflow prompt cannot exceed {MAX_PROMPT_CHARS} characters."
        ));
    }
    Ok(())
}

pub fn parse_workflow(content: &str) -> Result<(WorkflowConfig, String), String> {
    let normalized = content.replace("\r\n", "\n");
    let (config, prompt) = if let Some(rest) = normalized.strip_prefix("---\n") {
        let end = rest
            .find("\n---\n")
            .ok_or_else(|| "WORKFLOW.md front matter is missing its closing `---`.".to_string())?;
        let yaml = &rest[..end];
        let config = serde_yaml::from_str::<WorkflowConfig>(yaml)
            .map_err(|error| format!("Invalid WORKFLOW.md front matter: {error}"))?;
        (config, rest[end + 5..].to_string())
    } else {
        (WorkflowConfig::default(), normalized)
    };
    validate_config(&config, &prompt)?;
    Ok((config, prompt.trim().to_string()))
}

pub fn default_content() -> String {
    [
        "---",
        "orchestration:",
        "  max_concurrent: 2",
        "  max_attempts: 4",
        "  retry_base_seconds: 5",
        "  retry_max_seconds: 300",
        "agent:",
        "  model_id: null",
        "  permission_mode: ask",
        "---",
        "Complete the assigned local project task end-to-end.",
        "",
        "Inspect the repository before editing. Keep all changes inside the assigned worktree.",
        "Run relevant tests and summarize the implementation, verification, and remaining risks.",
    ]
    .join("\n")
}

fn workflow_path(
    registry: &WorkspaceRegistry,
    workspace_key: &str,
    workspace: &WorkspaceEnv,
) -> Result<PathBuf, String> {
    let resolved = resolve_path(workspace_key.trim(), workspace);
    let root = std::fs::canonicalize(&resolved)
        .map_err(|error| format!("Could not access the workspace: {error}"))?;
    if !root.is_dir() || !registry.is_authorized(&root) {
        return Err("The workflow path is outside the authorized workspace.".to_string());
    }
    Ok(root.join(WORKFLOW_FILE))
}

fn modified_at_ms(path: &Path) -> Option<u64> {
    path.metadata()
        .ok()?
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis() as u64)
}

fn load_at(path: PathBuf) -> WorkflowDocument {
    let display_path = path.to_string_lossy().replace('\\', "/");
    if !path.exists() {
        let content = default_content();
        let (config, prompt) = parse_workflow(&content).expect("default workflow must be valid");
        return WorkflowDocument {
            exists: false,
            path: display_path,
            content,
            config: Some(config),
            prompt: Some(prompt),
            validation_error: None,
            modified_at_ms: None,
        };
    }
    let metadata = match std::fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) => {
            return WorkflowDocument {
                exists: true,
                path: display_path,
                content: String::new(),
                config: None,
                prompt: None,
                validation_error: Some(format!("Could not inspect WORKFLOW.md: {error}")),
                modified_at_ms: None,
            }
        }
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return WorkflowDocument {
            exists: true,
            path: display_path,
            content: String::new(),
            config: None,
            prompt: None,
            validation_error: Some(
                "WORKFLOW.md must be a regular file, not a symlink.".to_string(),
            ),
            modified_at_ms: None,
        };
    }
    if metadata.len() > MAX_WORKFLOW_BYTES {
        return WorkflowDocument {
            exists: true,
            path: display_path,
            content: String::new(),
            config: None,
            prompt: None,
            validation_error: Some(format!(
                "WORKFLOW.md cannot exceed {} KiB.",
                MAX_WORKFLOW_BYTES / 1024
            )),
            modified_at_ms: modified_at_ms(&path),
        };
    }
    let content = match std::fs::read_to_string(&path) {
        Ok(content) => content,
        Err(error) => {
            return WorkflowDocument {
                exists: true,
                path: display_path,
                content: String::new(),
                config: None,
                prompt: None,
                validation_error: Some(format!("Could not read WORKFLOW.md: {error}")),
                modified_at_ms: modified_at_ms(&path),
            }
        }
    };
    match parse_workflow(&content) {
        Ok((config, prompt)) => WorkflowDocument {
            exists: true,
            path: display_path,
            content,
            config: Some(config),
            prompt: Some(prompt),
            validation_error: None,
            modified_at_ms: modified_at_ms(&path),
        },
        Err(error) => WorkflowDocument {
            exists: true,
            path: display_path,
            content,
            config: None,
            prompt: None,
            validation_error: Some(error),
            modified_at_ms: modified_at_ms(&path),
        },
    }
}

#[tauri::command]
pub fn orchestration_workflow_load(
    workspace_key: String,
    workspace: Option<WorkspaceEnv>,
    registry: State<'_, WorkspaceRegistry>,
) -> Result<WorkflowDocument, String> {
    let workspace = WorkspaceEnv::from_option(workspace);
    Ok(load_at(workflow_path(
        &registry,
        &workspace_key,
        &workspace,
    )?))
}

#[tauri::command]
pub fn orchestration_workflow_save(
    workspace_key: String,
    content: String,
    workspace: Option<WorkspaceEnv>,
    registry: State<'_, WorkspaceRegistry>,
) -> Result<WorkflowDocument, String> {
    if content.len() as u64 > MAX_WORKFLOW_BYTES {
        return Err(format!(
            "WORKFLOW.md cannot exceed {} KiB.",
            MAX_WORKFLOW_BYTES / 1024
        ));
    }
    parse_workflow(&content)?;
    let workspace = WorkspaceEnv::from_option(workspace);
    let path = workflow_path(&registry, &workspace_key, &workspace)?;
    if path.exists()
        && std::fs::symlink_metadata(&path)
            .map_err(|error| error.to_string())?
            .file_type()
            .is_symlink()
    {
        return Err("Refusing to replace a symlinked WORKFLOW.md.".to_string());
    }
    write_atomic(&path, content.as_bytes())
        .map_err(|error| format!("Could not save WORKFLOW.md: {error}"))?;
    Ok(load_at(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_default_workflow() {
        let (config, prompt) = parse_workflow(&default_content()).expect("parse default");
        assert_eq!(config.orchestration.max_concurrent, 2);
        assert_eq!(config.orchestration.max_attempts, 4);
        assert!(prompt.contains("Complete the assigned"));
    }

    #[test]
    fn rejects_unknown_and_unsafe_values() {
        let unknown = "---\nunknown: true\n---\nDo it.";
        assert!(parse_workflow(unknown).is_err());
        let excessive = "---\norchestration:\n  max_concurrent: 99\n---\nDo it.";
        assert!(parse_workflow(excessive).is_err());
    }

    #[test]
    fn markdown_only_uses_defaults() {
        let (config, prompt) = parse_workflow("Inspect and fix the task.").expect("parse markdown");
        assert_eq!(config.orchestration.retry_base_seconds, 5);
        assert_eq!(prompt, "Inspect and fix the task.");
    }
}
