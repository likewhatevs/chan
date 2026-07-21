//! Live tunnel registry: thin facade over `chan_tunnel_server::Registry`.
//!
//! The tunnel-server crate already maintains the authoritative
//! `(user, workspace) -> TunnelHandle` map (collision policy, eviction
//! on disconnect, substream open). devserver-proxy adds two things on
//! top:
//!
//! Lookup helpers return the immutable owner UUID carried by the signed
//! registration and stored on `TunnelHandle`. Username remains a routing
//! label only; it is never resolved through mutable cached identity state.

use std::sync::Arc;

use chan_tunnel_server::{Registry as TunnelRegistry, TunnelHandle};
use uuid::Uuid;

#[derive(Clone)]
pub struct Registry {
    tunnels: Arc<TunnelRegistry>,
}

/// Bundle returned by `Registry::get` for the proxy auth gate plus
/// substream open.
#[derive(Clone)]
pub struct Entry {
    pub handle: TunnelHandle,
    pub owner_id: Uuid,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            tunnels: TunnelRegistry::new(),
        }
    }

    /// The shared tunnel-server registry. Handed to
    /// `serve_tunnel_listener` so registrations observed on the
    /// tunnel listener are visible to the public listener.
    pub fn tunnels(&self) -> Arc<TunnelRegistry> {
        self.tunnels.clone()
    }

    /// Resolve a registered tunnel by its `(user, devserver_id)` key.
    /// Returns `None` when the tunnel disconnected, or when the username
    /// The second key is the devserver id (the registry's
    /// registration name), not a workspace slug.
    pub fn get(&self, username: &str, devserver_id: &str) -> Option<Entry> {
        let handle = self.tunnels.get(username, devserver_id)?;
        let owner_id = handle.owner_user_id;
        if owner_id.is_nil() {
            return None;
        }
        Some(Entry { handle, owner_id })
    }

    /// Resolve a devserver by host disc: the unique live devserver id
    /// of `username` that starts with `disc` (the first 12 hex chars
    /// of the id, carried in the `{user}--{disc}` host form). Returns
    /// the full devserver id paired with its entry. Zero matches and
    /// ambiguous prefixes both return `None`; the proxy maps either
    /// to 404 so a probe cannot tell them apart.
    pub fn get_user_devserver_by_prefix(
        &self,
        username: &str,
        disc: &str,
    ) -> Option<(String, Entry)> {
        let mut matches = self
            .tunnels
            .list_workspaces_for(username)
            .into_iter()
            .map(|info| info.workspace.as_ref().to_string())
            .filter(|id| id.starts_with(disc));
        let devserver_id = matches.next()?;
        if matches.next().is_some() {
            return None;
        }
        let entry = self.get(username, &devserver_id)?;
        Some((devserver_id, entry))
    }

    /// Live devserver ids registered for `username`, sorted by id.
    /// The proxy's bare-host path iterates these to find the
    /// registration a request's gate credential (`drv` claim) was
    /// minted for; the sort keeps that iteration deterministic.
    pub fn live_devserver_ids(&self, username: &str) -> Vec<String> {
        self.tunnels
            .list_workspaces_for(username)
            .into_iter()
            .map(|info| info.workspace.as_ref().to_string())
            .collect()
    }

    /// Number of immutable owners represented by live signed tunnels.
    #[cfg(test)]
    pub(crate) fn cached_user_count(&self) -> usize {
        self.tunnels
            .list_all()
            .into_iter()
            .map(|row| row.owner_user_id)
            .filter(|owner| !owner.is_nil())
            .collect::<std::collections::HashSet<_>>()
            .len()
    }

    /// Fail closed after the controller reconnect grace expires.
    /// Clears every local tunnel in one
    /// proxy-local operation; new registrations remain blocked by control
    /// readiness until a fresh snapshot reaches `FleetReady`.
    pub fn evict_all_for_control_loss(&self) -> usize {
        self.tunnels.evict_all()
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_loss_on_an_empty_registry_is_idempotent() {
        let r = Registry::new();
        assert_eq!(r.evict_all_for_control_loss(), 0);
        assert_eq!(r.cached_user_count(), 0);
    }
}
