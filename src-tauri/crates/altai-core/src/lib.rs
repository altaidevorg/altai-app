//! Tauri-independent primitives shared by the future ALTAI CLI and Desktop
//! adapters. This crate intentionally contains no UI-runtime dependency.

pub mod config;
pub mod event;
pub mod palette;
pub mod workspace;

pub use config::{
    load_agent_config, resolve_agent_config_layers, resolve_config, AgentConfigError,
    AgentConfigLayer, ConfigSource, ResolvedAgentConfig, ResolvedConfig,
};
pub use event::{EventEnvelope, EVENT_SCHEMA_VERSION};
pub use palette::{load_terminal_palette, PaletteError, TerminalPaletteManifest};
pub use workspace::{resolve_workspace, resolve_workspace_from, WorkspaceError, WorkspacePaths};
