//! Configuration precedence shared by desktop and command-line callers.

use std::fmt;
use std::fs;
use std::path::Path;

/// Where an effective configuration value originated.
///
/// The declaration order is intentional: a larger discriminant takes priority
/// during resolution. It mirrors the public CLI contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum ConfigSource {
    Default = 0,
    IsanagentConfig = 1,
    DesktopPreference = 2,
    ProjectConfig = 3,
    Environment = 4,
    CommandLine = 5,
}

impl ConfigSource {
    /// Stable, user-facing origin label for diagnostics such as
    /// `altai config list --resolved --show-origin`.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::IsanagentConfig => "isanagent-config",
            Self::DesktopPreference => "desktop-preference",
            Self::ProjectConfig => "project-config",
            Self::Environment => "environment",
            Self::CommandLine => "command-line",
        }
    }
}

/// An effective value together with the source that supplied it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedConfig<T> {
    pub value: T,
    pub source: ConfigSource,
}

/// Select the highest-precedence value from an iterator.
///
/// For matching sources, the last value wins. This lets an explicit repeated
/// CLI option behave like common command-line tools while retaining a stable
/// source label.
pub fn resolve_config<T>(
    values: impl IntoIterator<Item = (ConfigSource, T)>,
) -> Option<ResolvedConfig<T>> {
    values
        .into_iter()
        .fold(None, |current, (source, value)| match current {
            Some(existing) if existing.source > source => Some(existing),
            _ => Some(ResolvedConfig { value, source }),
        })
}

/// The non-secret agent settings understood by the initial CLI configuration
/// bridge. Credentials deliberately do not enter this type.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AgentConfigLayer {
    pub model: Option<String>,
    pub fallback_model: Option<String>,
    pub provider: Option<String>,
    pub base_url: Option<String>,
}

/// Resolved non-secret settings together with their origins.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResolvedAgentConfig {
    pub model: Option<ResolvedConfig<String>>,
    pub fallback_model: Option<ResolvedConfig<String>>,
    pub provider: Option<ResolvedConfig<String>>,
    pub base_url: Option<ResolvedConfig<String>>,
}

/// A malformed or unreadable local configuration file.
#[derive(Debug)]
pub enum AgentConfigError {
    Read {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    Parse {
        path: std::path::PathBuf,
        source: toml::de::Error,
    },
}

impl fmt::Display for AgentConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(
                    f,
                    "could not read configuration {}: {source}",
                    path.display()
                )
            }
            Self::Parse { path, source } => {
                write!(
                    f,
                    "could not parse configuration {}: {source}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for AgentConfigError {}

/// Resolve agent settings from the supplied precedence layers.
pub fn resolve_agent_config_layers(
    layers: impl IntoIterator<Item = (ConfigSource, AgentConfigLayer)>,
) -> ResolvedAgentConfig {
    let layers = layers.into_iter().collect::<Vec<_>>();
    let resolve = |field: fn(&AgentConfigLayer) -> Option<String>| {
        resolve_config(
            layers
                .iter()
                .filter_map(|(source, layer)| field(layer).map(|value| (*source, value))),
        )
    };

    ResolvedAgentConfig {
        model: resolve(|layer| layer.model.clone()),
        fallback_model: resolve(|layer| layer.fallback_model.clone()),
        provider: resolve(|layer| layer.provider.clone()),
        base_url: resolve(|layer| layer.base_url.clone()),
    }
}

/// Load and resolve non-secret agent settings from the documented file and
/// environment layers. Missing files are valid and simply contribute no value.
///
/// Project configuration accepts `[agent]` fields. IsanAgent configuration
/// follows its native `[provider]` names. Environment variables take priority.
pub fn load_agent_config(
    project_config: &Path,
    isanagent_config: &Path,
) -> Result<ResolvedAgentConfig, AgentConfigError> {
    let isanagent = read_layer(isanagent_config, ConfigFormat::Isanagent)?;
    let project = read_layer(project_config, ConfigFormat::AltaiProject)?;
    let environment = AgentConfigLayer {
        model: std::env::var("ALTAI_MODEL").ok(),
        fallback_model: std::env::var("ALTAI_FALLBACK_MODEL").ok(),
        provider: std::env::var("ALTAI_PROVIDER").ok(),
        base_url: std::env::var("ALTAI_BASE_URL").ok(),
    };

    Ok(resolve_agent_config_layers([
        (ConfigSource::IsanagentConfig, isanagent),
        (ConfigSource::ProjectConfig, project),
        (ConfigSource::Environment, environment),
    ]))
}

#[derive(Clone, Copy)]
enum ConfigFormat {
    AltaiProject,
    Isanagent,
}

fn read_layer(path: &Path, format: ConfigFormat) -> Result<AgentConfigLayer, AgentConfigError> {
    if !path.exists() {
        return Ok(AgentConfigLayer::default());
    }
    let contents = fs::read_to_string(path).map_err(|source| AgentConfigError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let document =
        toml::from_str::<toml::Value>(&contents).map_err(|source| AgentConfigError::Parse {
            path: path.to_path_buf(),
            source,
        })?;

    let (section, model_key, provider_key) = match format {
        ConfigFormat::AltaiProject => ("agent", "model", "provider"),
        ConfigFormat::Isanagent => ("provider", "model_name", "provider_name"),
    };
    let table = document.get(section).and_then(toml::Value::as_table);
    Ok(AgentConfigLayer {
        model: table
            .and_then(|table| table.get(model_key))
            .and_then(toml::Value::as_str)
            .map(str::to_string),
        fallback_model: table
            .and_then(|table| table.get("fallback_model"))
            .and_then(toml::Value::as_str)
            .map(str::to_string),
        provider: table
            .and_then(|table| table.get(provider_key))
            .and_then(toml::Value::as_str)
            .map(str::to_string),
        base_url: table
            .and_then(|table| table.get("base_url"))
            .and_then(toml::Value::as_str)
            .map(str::to_string),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_line_overrides_every_other_source() {
        let resolved = resolve_config([
            (ConfigSource::Default, "default"),
            (ConfigSource::IsanagentConfig, "isanagent"),
            (ConfigSource::DesktopPreference, "desktop"),
            (ConfigSource::ProjectConfig, "project"),
            (ConfigSource::Environment, "environment"),
            (ConfigSource::CommandLine, "flag"),
        ])
        .expect("one value should resolve");

        assert_eq!(resolved.value, "flag");
        assert_eq!(resolved.source, ConfigSource::CommandLine);
    }

    #[test]
    fn later_values_from_the_same_source_win() {
        let resolved = resolve_config([
            (ConfigSource::Environment, "first"),
            (ConfigSource::Environment, "second"),
        ])
        .expect("one value should resolve");

        assert_eq!(resolved.value, "second");
    }

    #[test]
    fn source_labels_are_stable() {
        assert_eq!(ConfigSource::ProjectConfig.label(), "project-config");
    }

    #[test]
    fn resolves_each_agent_setting_with_its_own_origin() {
        let resolved = resolve_agent_config_layers([
            (
                ConfigSource::IsanagentConfig,
                AgentConfigLayer {
                    model: Some("isan/model".into()),
                    provider: Some("isan".into()),
                    ..AgentConfigLayer::default()
                },
            ),
            (
                ConfigSource::ProjectConfig,
                AgentConfigLayer {
                    model: Some("project/model".into()),
                    fallback_model: Some("project/fallback".into()),
                    ..AgentConfigLayer::default()
                },
            ),
            (
                ConfigSource::Environment,
                AgentConfigLayer {
                    provider: Some("environment".into()),
                    ..AgentConfigLayer::default()
                },
            ),
        ]);

        assert_eq!(resolved.model.unwrap().value, "project/model");
        assert_eq!(resolved.fallback_model.unwrap().value, "project/fallback");
        let provider = resolved.provider.expect("provider resolves");
        assert_eq!(provider.value, "environment");
        assert_eq!(provider.source, ConfigSource::Environment);
    }

    #[test]
    fn reads_the_native_isanagent_provider_shape() {
        let temp = tempfile::tempdir().expect("temporary directory");
        let config = temp.path().join("config.toml");
        fs::write(
            &config,
            "[provider]\nprovider_name = \"openai\"\nmodel_name = \"gpt-test\"\nbase_url = \"https://relay.test/v1\"\n",
        )
        .expect("fixture config");

        let layer = read_layer(&config, ConfigFormat::Isanagent).expect("config parses");
        assert_eq!(layer.provider.as_deref(), Some("openai"));
        assert_eq!(layer.model.as_deref(), Some("gpt-test"));
        assert_eq!(layer.base_url.as_deref(), Some("https://relay.test/v1"));
    }

    #[test]
    fn reads_the_altai_project_agent_shape() {
        let temp = tempfile::tempdir().expect("temporary directory");
        let config = temp.path().join("config.toml");
        fs::write(
            &config,
            "[agent]\nmodel = \"anthropic/claude-test\"\nfallback_model = \"openai/gpt-test\"\nprovider = \"anthropic\"\n",
        )
        .expect("fixture config");

        let layer = read_layer(&config, ConfigFormat::AltaiProject).expect("config parses");
        assert_eq!(layer.model.as_deref(), Some("anthropic/claude-test"));
        assert_eq!(layer.fallback_model.as_deref(), Some("openai/gpt-test"));
        assert_eq!(layer.provider.as_deref(), Some("anthropic"));
    }
}
