//! Live tunnel registry: thin facade over `chan_tunnel_server::Registry`.
//!
//! The tunnel-server crate already maintains the authoritative
//! `(user, workspace) -> TunnelHandle` map (collision policy, eviction
//! on disconnect, substream open). devserver-proxy adds two things on
//! top:
//!
//!   * a `username -> user_id` cache populated by the validator
//!     wrapper on every successful tunnel handshake, so the
//!     reverse-proxy auth gate can resolve `owner_id` without an
//!     extra round trip to profile-service;
//!
//!   * lookup helpers that bundle the tunnel handle with the cached
//!     `owner_id` required by the reverse-proxy authorization gate.
//!
//! Cache invalidation is fleet fail-closed: normal handshakes refresh the
//! entry, and controller grace expiry clears the complete cache alongside
//! every local tunnel.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use chan_tunnel_server::{Registry as TunnelRegistry, TunnelHandle};
use uuid::Uuid;

/// `RwLock` (not `Mutex`) because `Registry::get` is on the hot path:
/// one read per inbound wildcard request, against an entry that is
/// already cached by `record_user` on tunnel handshake. Writes are rare.
/// `parking_lot` would buy a
/// little extra throughput on contended reads but `std::sync::RwLock`
/// keeps dependencies flat and is sufficient for the request rates we
/// target.
#[derive(Clone)]
pub struct Registry {
    tunnels: Arc<TunnelRegistry>,
    user_ids: Arc<RwLock<HashMap<String, Uuid>>>,
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
            user_ids: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// The shared tunnel-server registry. Handed to
    /// `serve_tunnel_listener` so registrations observed on the
    /// tunnel listener are visible to the public listener.
    pub fn tunnels(&self) -> Arc<TunnelRegistry> {
        self.tunnels.clone()
    }

    /// Record the user_id seen for a username on the latest token
    /// validate. Idempotent.
    pub fn record_user(&self, username: &str, user_id: Uuid) {
        // The cache holds no integrity invariant; a poisoned lock
        // (some past handler panicked while holding it) is fine to
        // reuse, so we transparently recover instead of propagating
        // the poison. Without this, a single panic kills the proxy
        // auth gate for the lifetime of the process.
        self.user_ids
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert(username.to_string(), user_id);
    }

    /// Resolve a registered tunnel by its `(user, devserver_id)` key.
    /// Returns `None` when the tunnel disconnected, or when the username
    /// hasn't been seen in any tunnel handshake yet (no cached
    /// owner_id). The second key is the devserver id (the registry's
    /// registration name), not a workspace slug.
    pub fn get(&self, username: &str, devserver_id: &str) -> Option<Entry> {
        let handle = self.tunnels.get(username, devserver_id)?;
        let owner_id = self
            .user_ids
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(username)
            .copied()?;
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

    /// Size of the `username -> user_id` cache. Tests assert the
    /// fail-closed path empties it without re-registering a tunnel,
    /// which would repopulate the cache through the validator wrapper.
    #[cfg(test)]
    pub(crate) fn cached_user_count(&self) -> usize {
        self.user_ids
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .len()
    }

    /// Fail closed after the controller reconnect grace expires.
    /// Clears every local tunnel and all cached user identities in one
    /// proxy-local operation; new registrations remain blocked by control
    /// readiness until a fresh snapshot reaches `FleetReady`.
    pub fn evict_all_for_control_loss(&self) -> usize {
        let killed = self.tunnels.evict_all();
        self.user_ids
            .write()
            .unwrap_or_else(|error| error.into_inner())
            .clear();
        killed
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
    fn control_loss_clears_every_cached_user() {
        let r = Registry::new();
        r.record_user("alice", Uuid::new_v4());
        r.record_user("bob", Uuid::new_v4());

        assert_eq!(r.evict_all_for_control_loss(), 0);
        assert!(r.user_ids.read().unwrap().is_empty());
    }

    #[test]
    fn record_user_overwrites_existing() {
        let r = Registry::new();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        r.record_user("alice", a);
        r.record_user("alice", b);
        assert_eq!(r.user_ids.read().unwrap().get("alice").copied(), Some(b));
    }
}
