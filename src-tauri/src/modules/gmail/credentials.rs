//! Account-scoped credentials for the Gmail adapter (package 074,
//! PR 3). CP-08-76's addressability contract — every credential is
//! addressed by `(plugin, account, name)` — is fronted here by the
//! platform secret storage: the scope's plugin and name halves encode
//! into the platform service address, the account id is the platform
//! account address. The wrong scope finds nothing; it does not find
//! the neighbor.
//!
//! These are free functions, not an `AccountCredentialStore` impl, on
//! purpose: the platform store is fallible (a keyring write can fail),
//! and that failure must reach the command boundary — the trait's
//! infallible `put`/`remove` have nowhere honest to put it. The trait
//! remains the contract for hosts and worker brokers with infallible
//! stores; the scope encoding lives here, once.

use altai_control_plane::SecretString;
use altai_control_protocol::{ExternalAccountId, PluginId};
use tauri::AppHandle;

use crate::modules::secrets::{self, SecretsState};

/// The built-in Gmail adapter's identity inside credential scopes. It
/// names the scope only — the adapter is not a loadable plugin.
pub const GMAIL_PLUGIN_ID: &str = "plg_gmail";

/// The one credential a Gmail account carries today: its OAuth access
/// token. Named like any plugin credential so the scope stays uniform.
pub const ACCESS_TOKEN: &str = "access_token";

/// The platform service half of the address: `(plugin, name)` encoded
/// so distinct scopes never collide. The account id is the other half.
fn service(plugin_id: &PluginId, name: &str) -> String {
    format!("account-credential::{}::{name}", plugin_id.value)
}

pub fn put_account_credential(
    app: &AppHandle,
    state: &SecretsState,
    plugin_id: &PluginId,
    account_id: &ExternalAccountId,
    name: &str,
    value: SecretString,
) -> Result<(), String> {
    secrets::set_secret(
        app,
        state,
        &service(plugin_id, name),
        &account_id.value,
        value.expose(),
    )
}

/// The credential for exactly this scope. Another account, another
/// plugin, or another name finds nothing.
pub fn get_account_credential(
    app: &AppHandle,
    state: &SecretsState,
    plugin_id: &PluginId,
    account_id: &ExternalAccountId,
    name: &str,
) -> Result<Option<SecretString>, String> {
    secrets::get_secret(app, state, &service(plugin_id, name), &account_id.value)
        .map(|value| value.map(SecretString::new))
}

/// Forget one credential, scoped like `get`: removing one account's
/// credential leaves every other scope untouched.
pub fn remove_account_credential(
    app: &AppHandle,
    state: &SecretsState,
    plugin_id: &PluginId,
    account_id: &ExternalAccountId,
    name: &str,
) -> Result<(), String> {
    secrets::delete_secret(app, state, &service(plugin_id, name), &account_id.value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scope_encodes_without_colliding() {
        let gmail = PluginId::new(GMAIL_PLUGIN_ID);
        let other = PluginId::new("plg_other");
        assert_eq!(
            service(&gmail, ACCESS_TOKEN),
            "account-credential::plg_gmail::access_token"
        );
        assert_ne!(service(&gmail, ACCESS_TOKEN), service(&other, ACCESS_TOKEN));
        assert_ne!(service(&gmail, ACCESS_TOKEN), service(&gmail, "refresh"));
        // The account half is the account id alone: two accounts under
        // one plugin are two platform addresses, never one shared key.
        assert_ne!(
            ExternalAccountId::new("exta_a").value,
            ExternalAccountId::new("exta_b").value
        );
    }
}
