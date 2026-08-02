//! Fingerprint + instance types for the long-lived agent registry.

use std::sync::Arc;
use std::time::Duration;

use isanagent::bus::BusMessage;
use isanagent::channels::Channel;
use sha2::{Digest, Sha256};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::compaction::CompactionArg;

/// Non-reversible stable identity; raw provider credentials never enter Hash/Debug state.
pub fn secret_identity(secret: &str) -> String {
    if secret.is_empty() {
        return "none".to_string();
    }
    let digest = Sha256::digest(secret.as_bytes());
    format!("sha256:{}", &hex::encode(digest)[..16])
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FallbackFingerprint {
    pub provider_name: String,
    pub model_name: String,
    pub base_url: String,
    pub secret_identity: String,
}

impl From<&isanagent::agent::FallbackProviderSpec> for FallbackFingerprint {
    fn from(spec: &isanagent::agent::FallbackProviderSpec) -> Self {
        Self {
            provider_name: spec.provider_name.clone(),
            model_name: spec.model_name.clone(),
            base_url: spec.base_url.trim_end_matches('/').to_string(),
            secret_identity: secret_identity(&spec.api_key),
        }
    }
}

/// Identifies a particular agent configuration. Changing any field rebuilds
/// the instance so permission/model/MCP switches cannot silently reuse a stale loop.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RuntimeFingerprint {
    pub provider_name: String,
    pub model_name: String,
    pub secret_identity: String,
    pub base_url: String,
    pub fallback: Option<FallbackFingerprint>,
    pub persona: String,
    pub workspace_root: String,
    pub permission_mode: String,
    pub compaction: Option<(bool, usize, usize)>,
    pub mcp_config: String,
}

impl RuntimeFingerprint {
    #[allow(clippy::too_many_arguments)]
    pub fn make(
        provider_name: &str,
        api_key: &str,
        model_name: &str,
        persona_instructions: Option<&str>,
        base_url_override: Option<&str>,
        workspace_path: Option<&str>,
        permission_mode: Option<&str>,
        compaction: Option<&CompactionArg>,
        fallback: Option<&isanagent::agent::FallbackProviderSpec>,
    ) -> Self {
        let workspace_root = workspace_path
            .map(|p| format!("{}/.isanagent", p.trim_end_matches('/')))
            .unwrap_or_default();
        let mcp_config = {
            let root = if workspace_root.is_empty() {
                isanagent::workspace::resolve_workspace_root(None)
            } else {
                std::path::PathBuf::from(&workspace_root)
            };
            std::fs::read_to_string(root.join("mcp.json")).unwrap_or_default()
        };
        Self {
            provider_name: provider_name.to_string(),
            model_name: model_name.to_string(),
            secret_identity: secret_identity(api_key),
            base_url: base_url_override.unwrap_or("").to_string(),
            fallback: fallback.map(FallbackFingerprint::from),
            persona: persona_instructions.unwrap_or("").to_string(),
            workspace_root,
            permission_mode: permission_mode.unwrap_or("").to_string(),
            compaction: compaction.map(|c| c.fingerprint_tuple()),
            mcp_config,
        }
    }
}

/// One running IsanAgent instance — channel + bus routers.
pub struct Instance<C> {
    pub channel: Arc<C>,
    pub bus_tx: mpsc::Sender<BusMessage>,
    pub shutdown: tokio::sync::oneshot::Sender<()>,
    pub bus_router: JoinHandle<()>,
    pub outbound_router: JoinHandle<()>,
}

const INSTANCE_TASK_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);

pub async fn stop_instance<C>(instance: Instance<C>)
where
    C: Channel + Send + Sync + 'static,
{
    let Instance {
        channel,
        shutdown,
        mut bus_router,
        mut outbound_router,
        ..
    } = instance;

    let _ = shutdown.send(());

    if tokio::time::timeout(INSTANCE_TASK_SHUTDOWN_TIMEOUT, &mut bus_router)
        .await
        .is_err()
    {
        bus_router.abort();
        let _ = bus_router.await;
    }
    if tokio::time::timeout(INSTANCE_TASK_SHUTDOWN_TIMEOUT, &mut outbound_router)
        .await
        .is_err()
    {
        outbound_router.abort();
        let _ = outbound_router.await;
    }
    let _ = channel.stop().await;
}

#[cfg(test)]
mod fingerprint_tests {
    use super::*;

    #[test]
    fn secret_identity_is_stable_and_non_reversible() {
        let a = secret_identity("sk-test-secret");
        let b = secret_identity("sk-test-secret");
        assert_eq!(a, b);
        assert!(a.starts_with("sha256:"));
        assert!(!a.contains("sk-test-secret"));
        assert_eq!(secret_identity(""), "none");
    }

    #[test]
    fn permission_mode_participates_in_fingerprint() {
        let ask = RuntimeFingerprint::make(
            "openai",
            "k",
            "m",
            None,
            None,
            Some("/tmp/proj"),
            Some("ask"),
            None,
            None,
        );
        let bypass = RuntimeFingerprint::make(
            "openai",
            "k",
            "m",
            None,
            None,
            Some("/tmp/proj"),
            Some("bypass"),
            None,
            None,
        );
        assert_ne!(ask, bypass);
        assert_eq!(ask.permission_mode, "ask");
        assert_eq!(bypass.permission_mode, "bypass");
    }
}
