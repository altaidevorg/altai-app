//! ALTAI's mapping onto the reusable IsanAgent host boundary.

use altai_core::WorkspacePaths;

/// Build a host configuration without creating state or starting a terminal.
///
/// IsanAgent owns durable data below `.isanagent`, while ALTAI exposes the
/// actual project root as the tool sandbox. Keeping those paths separate is
/// required for desktop and CLI sessions to share state without placing agent
/// databases, logs, or generated skills in the user's source tree.
pub fn host_config_for_workspace(workspace: &WorkspacePaths) -> isanagent::host::HostConfig {
    isanagent::host::HostConfig {
        workspace: Some(workspace.isanagent_state.clone()),
        config: Some(workspace.isanagent_state.join("config.toml")),
        sandbox: Some(workspace.root.clone()),
        theme: isanagent::host::HostThemeMode::Auto,
        ..Default::default()
    }
}

/// Build a one-shot host configuration for `altai-cli --prompt`.
pub fn oneshot_host_config(
    workspace: &WorkspacePaths,
    prompt: String,
    observe_tx: Option<tokio::sync::mpsc::UnboundedSender<isanagent::bus::BusMessage>>,
) -> isanagent::host::HostConfig {
    let mut host = host_config_for_workspace(workspace);
    host.oneshot_prompt = Some(prompt);
    host.observe_tx = observe_tx;
    host
}

/// Build an ACP (Agent Client Protocol) host for `altai acp`.
///
/// Stdin/stdout become the ACP JSON-RPC transport; the interactive terminal is
/// disabled. This is distinct from `altai serve --stdio`, which speaks ALTAI's
/// own agent-host protocol.
pub fn acp_host_config(workspace: &WorkspacePaths) -> isanagent::host::HostConfig {
    let mut host = host_config_for_workspace(workspace);
    host.acp_mode = true;
    host
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn maps_project_sandbox_and_durable_state_separately() {
        let workspace = WorkspacePaths {
            root: PathBuf::from("/project"),
            isanagent_state: PathBuf::from("/project/.isanagent"),
        };
        let config = host_config_for_workspace(&workspace);

        assert_eq!(config.workspace, Some(PathBuf::from("/project/.isanagent")));
        assert_eq!(
            config.config,
            Some(PathBuf::from("/project/.isanagent/config.toml"))
        );
        assert_eq!(config.sandbox, Some(PathBuf::from("/project")));
        assert!(config.oneshot_prompt.is_none());
        assert!(!config.acp_mode);
    }

    #[test]
    fn oneshot_config_injects_prompt_without_tui() {
        let workspace = WorkspacePaths {
            root: PathBuf::from("/project"),
            isanagent_state: PathBuf::from("/project/.isanagent"),
        };
        let config = oneshot_host_config(&workspace, "summarize".into(), None);
        assert_eq!(config.oneshot_prompt.as_deref(), Some("summarize"));
        assert!(!config.line_mode);
        assert!(!config.acp_mode);
    }

    #[test]
    fn acp_config_enables_protocol_mode_without_oneshot() {
        let workspace = WorkspacePaths {
            root: PathBuf::from("/project"),
            isanagent_state: PathBuf::from("/project/.isanagent"),
        };
        let config = acp_host_config(&workspace);
        assert!(config.acp_mode);
        assert!(config.oneshot_prompt.is_none());
        assert_eq!(config.sandbox, Some(PathBuf::from("/project")));
    }
}
