//! Host-neutral ownership for concrete agent instances.
//!
//! An agent host still owns the channel implementation and the expensive
//! construction/teardown work. This registry owns the small, but safety
//! critical, portion shared by every host: selecting the concrete instance by
//! configuration fingerprint, binding a chat only after a successful send,
//! and atomically taking stale instances out of service before teardown.
//!
//! No async work may be performed while a registry closure is running. In
//! particular, callers must take instances first and await their shutdown only
//! after the closure returns. That prevents a slow provider teardown from
//! blocking an unrelated fingerprint's send or cancel.

use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;
use std::sync::Mutex;

/// Lock failure returned instead of panicking at a host boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentInstanceRegistryError;

impl fmt::Display for AgentInstanceRegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("agent instance registry is unavailable")
    }
}

impl std::error::Error for AgentInstanceRegistryError {}

/// Concurrent-instance and successful-send ownership registry.
///
/// `F` is an opaque host configuration fingerprint and `I` is a concrete
/// instance owned by that host (for example, an IsanAgent node plus a Tauri or
/// stdio channel). The service intentionally does not inspect either type.
pub struct AgentInstanceRegistry<F, I> {
    instances: Mutex<HashMap<F, I>>,
    chat_owners: Mutex<HashMap<(String, String), F>>,
}

impl<F, I> Default for AgentInstanceRegistry<F, I>
where
    F: Clone + Eq + Hash,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<F, I> AgentInstanceRegistry<F, I>
where
    F: Clone + Eq + Hash,
{
    pub fn new() -> Self {
        Self {
            instances: Mutex::new(HashMap::new()),
            chat_owners: Mutex::new(HashMap::new()),
        }
    }

    /// Read a value derived from one concrete instance without exposing the
    /// backing map. The closure must be non-blocking.
    pub fn with_instance<R>(
        &self,
        fingerprint: &F,
        read: impl FnOnce(&I) -> R,
    ) -> Result<Option<R>, AgentInstanceRegistryError> {
        let instances = self
            .instances
            .lock()
            .map_err(|_| AgentInstanceRegistryError)?;
        Ok(instances.get(fingerprint).map(read))
    }

    /// Find an instance by a host-owned predicate, such as its immutable
    /// owner id. The returned value must not borrow the instance.
    pub fn find_instance<R>(
        &self,
        find: impl FnMut(&I) -> Option<R>,
    ) -> Result<Option<R>, AgentInstanceRegistryError> {
        let instances = self
            .instances
            .lock()
            .map_err(|_| AgentInstanceRegistryError)?;
        Ok(instances.values().find_map(find))
    }

    /// Collect small derived values from matching instances. This is intended
    /// for admission checks before the caller atomically takes those instances
    /// for asynchronous teardown.
    pub fn collect_matching<R>(
        &self,
        mut matches: impl FnMut(&F, &I) -> bool,
        mut map: impl FnMut(&F, &I) -> R,
    ) -> Result<Vec<R>, AgentInstanceRegistryError> {
        let instances = self
            .instances
            .lock()
            .map_err(|_| AgentInstanceRegistryError)?;
        Ok(instances
            .iter()
            .filter(|(fingerprint, instance)| matches(fingerprint, instance))
            .map(|(fingerprint, instance)| map(fingerprint, instance))
            .collect())
    }

    /// Insert a newly built instance unless another concurrent sender already
    /// installed that fingerprint. Returns the losing instance to its caller,
    /// which must tear it down outside this registry.
    pub fn insert_if_absent(
        &self,
        fingerprint: F,
        instance: I,
    ) -> Result<Result<(), I>, AgentInstanceRegistryError> {
        let mut instances = self
            .instances
            .lock()
            .map_err(|_| AgentInstanceRegistryError)?;
        if instances.contains_key(&fingerprint) {
            return Ok(Err(instance));
        }
        instances.insert(fingerprint, instance);
        Ok(Ok(()))
    }

    /// Remove a group of instances synchronously. Awaiting their shutdown is
    /// deliberately the caller's responsibility.
    pub fn take_matching(
        &self,
        mut matches: impl FnMut(&F, &I) -> bool,
    ) -> Result<Vec<(F, I)>, AgentInstanceRegistryError> {
        let mut instances = self
            .instances
            .lock()
            .map_err(|_| AgentInstanceRegistryError)?;
        let keys: Vec<F> = instances
            .iter()
            .filter(|(fingerprint, instance)| matches(fingerprint, instance))
            .map(|(fingerprint, _)| fingerprint.clone())
            .collect();
        Ok(keys
            .into_iter()
            .filter_map(|fingerprint| {
                instances
                    .remove(&fingerprint)
                    .map(|instance| (fingerprint, instance))
            })
            .collect())
    }

    /// Bind a chat to an already accepted send. The old binding is returned so
    /// restart recovery can remain a once-per-chat side effect.
    pub fn bind_chat(
        &self,
        workspace_root: impl Into<String>,
        chat_id: impl Into<String>,
        fingerprint: F,
    ) -> Result<Option<F>, AgentInstanceRegistryError> {
        let previous = self
            .chat_owners
            .lock()
            .map_err(|_| AgentInstanceRegistryError)?
            .insert((workspace_root.into(), chat_id.into()), fingerprint);
        Ok(previous)
    }

    pub fn chat_owner(
        &self,
        workspace_root: &str,
        chat_id: &str,
    ) -> Result<Option<F>, AgentInstanceRegistryError> {
        Ok(self
            .chat_owners
            .lock()
            .map_err(|_| AgentInstanceRegistryError)?
            .get(&(workspace_root.to_string(), chat_id.to_string()))
            .cloned())
    }

    /// Discard stale chat bindings after their workspace's instances have been
    /// removed. This order ensures no synthetic send can route to a torn-down
    /// instance.
    pub fn retain_chat_owners_for_workspace(
        &self,
        workspace_root: &str,
    ) -> Result<(), AgentInstanceRegistryError> {
        self.chat_owners
            .lock()
            .map_err(|_| AgentInstanceRegistryError)?
            .retain(|(root, _), _| root == workspace_root);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::AgentInstanceRegistry;

    #[test]
    fn concurrent_fingerprints_remain_isolated_and_duplicate_build_loses() {
        let registry = AgentInstanceRegistry::<String, String>::new();
        assert_eq!(
            registry
                .insert_if_absent("model-a".to_string(), "instance-a".to_string())
                .unwrap(),
            Ok(())
        );
        assert_eq!(
            registry
                .insert_if_absent("model-b".to_string(), "instance-b".to_string())
                .unwrap(),
            Ok(())
        );
        assert_eq!(
            registry
                .insert_if_absent("model-a".to_string(), "duplicate".to_string())
                .unwrap(),
            Err("duplicate".to_string())
        );
        assert_eq!(
            registry
                .with_instance(&"model-a".to_string(), Clone::clone)
                .unwrap(),
            Some("instance-a".to_string())
        );
        assert_eq!(
            registry
                .with_instance(&"model-b".to_string(), Clone::clone)
                .unwrap(),
            Some("instance-b".to_string())
        );
    }

    #[test]
    fn chat_binding_happens_after_send_and_stale_workspace_is_removed() {
        let registry = AgentInstanceRegistry::<String, String>::new();
        assert!(registry
            .bind_chat("workspace-a", "chat-a", "model-a".to_string())
            .unwrap()
            .is_none());
        assert_eq!(
            registry
                .bind_chat("workspace-a", "chat-a", "model-b".to_string())
                .unwrap(),
            Some("model-a".to_string())
        );
        registry
            .bind_chat("workspace-b", "chat-b", "model-c".to_string())
            .unwrap();
        registry
            .retain_chat_owners_for_workspace("workspace-a")
            .unwrap();
        assert_eq!(
            registry.chat_owner("workspace-a", "chat-a").unwrap(),
            Some("model-b".to_string())
        );
        assert_eq!(registry.chat_owner("workspace-b", "chat-b").unwrap(), None);
    }
}
