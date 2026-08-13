//! Trusted execution-profile resolution for control-plane attempts.
//!
//! Unlike `agent_send`, this module never accepts provider credentials or an
//! endpoint from the webview.  The immutable agent-profile revision supplies a
//! canonical `provider/model` selector and the native host reads its matching
//! credential from the existing OS-backed secret store.

// CP-08-07 wires this host-only resolver into the trusted start command. Keep
// the seam independently testable before that command exists.
#![allow(dead_code)]

use tauri::AppHandle;

use crate::modules::secrets::{self, SecretsState};

const AI_SECRET_SERVICE: &str = "altai-ai";

/// Host-owned runtime settings. `api_key` is intentionally not serializable or
/// printable, so it cannot accidentally cross the Tauri boundary or logs.
pub struct TrustedExecutionProfile {
    pub provider_name: String,
    pub model_name: String,
    pub base_url: String,
    pub permission_mode: String,
    pub api_key: String,
}

impl std::fmt::Debug for TrustedExecutionProfile {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TrustedExecutionProfile")
            .field("provider_name", &self.provider_name)
            .field("model_name", &self.model_name)
            .field("base_url", &self.base_url)
            .field("permission_mode", &self.permission_mode)
            .field("api_key", &"[REDACTED]")
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProviderRoute {
    provider_name: &'static str,
    secret_account: &'static str,
    base_url: &'static str,
}

/// Resolve a profile revision's model selection and its host credential.
///
/// The selection is intentionally narrow until profile revisions gain a
/// canonical endpoint/credential-reference schema: `provider/model` is the
/// only accepted representation. Local and custom-compatible providers cannot
/// be activated by model text alone because their endpoint is user mutable.
pub fn resolve_trusted_execution_profile(
    app: &AppHandle,
    secrets_state: &SecretsState,
    model: Option<&str>,
    permission_policy: &str,
) -> Result<TrustedExecutionProfile, String> {
    let (provider, model_name) = parse_model_selector(model)?;
    let route = provider_route(provider)?;
    let api_key = secrets::get_secret(app, secrets_state, AI_SECRET_SERVICE, route.secret_account)?
        .filter(|secret| !secret.trim().is_empty())
        .ok_or_else(|| format!("No host credential is configured for provider '{provider}'"))?;
    Ok(TrustedExecutionProfile {
        provider_name: route.provider_name.to_string(),
        model_name: model_name.to_string(),
        base_url: route.base_url.to_string(),
        permission_mode: normalize_permission_policy(permission_policy)?,
        api_key,
    })
}

fn parse_model_selector(model: Option<&str>) -> Result<(&str, &str), String> {
    let selector = model
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "An authorized attempt requires an immutable provider/model profile".to_string())?;
    let (provider, model_name) = selector
        .split_once('/')
        .ok_or_else(|| "Authorized profile model must use provider/model form".to_string())?;
    if provider.is_empty() || model_name.trim().is_empty() || model_name.contains('/') {
        return Err("Authorized profile model must use one non-empty provider/model pair".to_string());
    }
    Ok((provider, model_name.trim()))
}

fn provider_route(provider: &str) -> Result<ProviderRoute, String> {
    let route = match provider {
        "openai" => ProviderRoute { provider_name: "openai", secret_account: "openai-api-key", base_url: "https://api.openai.com/v1/chat/completions" },
        "anthropic" => ProviderRoute { provider_name: "anthropic", secret_account: "anthropic-api-key", base_url: "https://api.anthropic.com/v1/messages" },
        "google" => ProviderRoute { provider_name: "gemini", secret_account: "google-api-key", base_url: "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions" },
        "xai" => ProviderRoute { provider_name: "openai", secret_account: "xai-api-key", base_url: "https://api.x.ai/v1/chat/completions" },
        "groq" => ProviderRoute { provider_name: "openai", secret_account: "groq-api-key", base_url: "https://api.groq.com/openai/v1/chat/completions" },
        "deepseek" => ProviderRoute { provider_name: "openai", secret_account: "deepseek-api-key", base_url: "https://api.deepseek.com/chat/completions" },
        "mistral" => ProviderRoute { provider_name: "openai", secret_account: "mistral-api-key", base_url: "https://api.mistral.ai/v1/chat/completions" },
        "openrouter" => ProviderRoute { provider_name: "openai", secret_account: "openrouter-api-key", base_url: "https://openrouter.ai/api/v1/chat/completions" },
        _ => return Err(format!("Provider '{provider}' is not an authorized managed execution route")),
    };
    Ok(route)
}

fn normalize_permission_policy(policy: &str) -> Result<String, String> {
    match policy.trim() {
        "ask" | "auto-edit" | "plan" | "bypass" => Ok(policy.trim().to_string()),
        _ => Err("Authorized attempt permission policy is invalid".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_selector_requires_one_known_provider_model_pair() {
        assert_eq!(parse_model_selector(Some("openai/gpt-5")).unwrap(), ("openai", "gpt-5"));
        assert!(parse_model_selector(Some("gpt-5")).is_err());
        assert!(parse_model_selector(Some("openai/a/b")).is_err());
        assert!(provider_route("lmstudio").is_err());
    }

    #[test]
    fn route_and_permission_are_canonical_and_bounded() {
        let route = provider_route("google").unwrap();
        assert_eq!(route.provider_name, "gemini");
        assert_eq!(route.secret_account, "google-api-key");
        assert_eq!(normalize_permission_policy("auto-edit").unwrap(), "auto-edit");
        assert!(normalize_permission_policy("renderer-supplied").is_err());
    }
}
