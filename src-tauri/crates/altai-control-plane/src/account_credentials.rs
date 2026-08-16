//! CP-08 account-scoped credentials (package 074, PR 1). The store that
//! decides which credential belongs to which account of which plugin:
//! every read and write is addressed by `(plugin, account, name)`, so
//! one account's credential is not reachable under another account's
//! scope — the wrong key finds nothing, it does not find the neighbor.
//!
//! Values are [`SecretString`]s: redacting by construction. The host
//! owns the store and brokers values out (072's worker hand-off is the
//! one channel they travel on); account metadata itself lives in the
//! [`ExternalAccountRepository`](crate::ExternalAccountRepository),
//! never here.

use crate::SecretString;
use altai_control_protocol::{ExternalAccountId, PluginId};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Scope {
    plugin_id: PluginId,
    account_id: ExternalAccountId,
    name: String,
}

pub trait AccountCredentialStore: Send + Sync {
    /// Store one credential under an account's scope. Storing under the
    /// same scope again replaces the value.
    fn put(
        &self,
        plugin_id: &PluginId,
        account_id: &ExternalAccountId,
        name: &str,
        value: SecretString,
    );
    /// The credential for exactly this scope. Another account, another
    /// plugin, or another name finds nothing.
    fn get(
        &self,
        plugin_id: &PluginId,
        account_id: &ExternalAccountId,
        name: &str,
    ) -> Option<SecretString>;
    /// Forget one credential. Scoped like `get`: removing one account's
    /// credential leaves every other scope untouched.
    fn remove(&self, plugin_id: &PluginId, account_id: &ExternalAccountId, name: &str);
}

/// In-memory store: the desktop host fronts this with its platform
/// secret storage; tests and tooling use it directly.
#[derive(Default)]
pub struct InMemoryAccountCredentialStore {
    credentials: std::sync::Mutex<HashMap<Scope, SecretString>>,
}

impl InMemoryAccountCredentialStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl AccountCredentialStore for InMemoryAccountCredentialStore {
    fn put(
        &self,
        plugin_id: &PluginId,
        account_id: &ExternalAccountId,
        name: &str,
        value: SecretString,
    ) {
        self.credentials
            .lock()
            .expect("account credential store lock poisoned")
            .insert(
                Scope {
                    plugin_id: plugin_id.clone(),
                    account_id: account_id.clone(),
                    name: name.to_string(),
                },
                value,
            );
    }

    fn get(
        &self,
        plugin_id: &PluginId,
        account_id: &ExternalAccountId,
        name: &str,
    ) -> Option<SecretString> {
        self.credentials
            .lock()
            .expect("account credential store lock poisoned")
            .get(&Scope {
                plugin_id: plugin_id.clone(),
                account_id: account_id.clone(),
                name: name.to_string(),
            })
            .cloned()
    }

    fn remove(&self, plugin_id: &PluginId, account_id: &ExternalAccountId, name: &str) {
        self.credentials
            .lock()
            .expect("account credential store lock poisoned")
            .remove(&Scope {
                plugin_id: plugin_id.clone(),
                account_id: account_id.clone(),
                name: name.to_string(),
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store_with_work_token() -> (InMemoryAccountCredentialStore, PluginId, ExternalAccountId) {
        let store = InMemoryAccountCredentialStore::new();
        let plugin = PluginId::new("plg_gmail");
        let work = ExternalAccountId::new("exta_work");
        store.put(&plugin, &work, "oauth_refresh_token", SecretString::new("work-secret"));
        (store, plugin, work)
    }

    #[test]
    fn a_credential_round_trips_within_its_own_scope() {
        let (store, plugin, work) = store_with_work_token();
        assert_eq!(
            store
                .get(&plugin, &work, "oauth_refresh_token")
                .unwrap()
                .expose(),
            "work-secret"
        );
    }

    #[test]
    fn another_account_never_sees_a_credential() {
        let (store, plugin, _work) = store_with_work_token();
        let personal = ExternalAccountId::new("exta_personal");
        assert!(
            store.get(&plugin, &personal, "oauth_refresh_token").is_none(),
            "the personal account's scope finds nothing of work's"
        );
    }

    #[test]
    fn another_plugin_never_sees_a_credential() {
        let (store, _plugin, work) = store_with_work_token();
        let other_plugin = PluginId::new("plg_other");
        assert!(store
            .get(&other_plugin, &work, "oauth_refresh_token")
            .is_none());
    }

    #[test]
    fn removal_is_scoped_to_one_account() {
        let (store, plugin, work) = store_with_work_token();
        let personal = ExternalAccountId::new("exta_personal");
        store.put(&plugin, &personal, "oauth_refresh_token", SecretString::new("personal-secret"));

        store.remove(&plugin, &work, "oauth_refresh_token");

        assert!(store.get(&plugin, &work, "oauth_refresh_token").is_none());
        assert_eq!(
            store
                .get(&plugin, &personal, "oauth_refresh_token")
                .unwrap()
                .expose(),
            "personal-secret",
            "removing work's credential never touched personal's"
        );
    }

    #[test]
    fn storing_again_replaces_the_value_in_place() {
        let (store, plugin, work) = store_with_work_token();
        store.put(&plugin, &work, "oauth_refresh_token", SecretString::new("rotated"));
        assert_eq!(
            store
                .get(&plugin, &work, "oauth_refresh_token")
                .unwrap()
                .expose(),
            "rotated"
        );
    }
}
