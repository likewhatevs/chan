//! Live tunnel registry: thin facade over `chan_tunnel_server::Registry`.
//!
//! The tunnel-server crate already maintains the authoritative
//! `(user, drive) -> TunnelHandle` map (collision policy, eviction
//! on disconnect, substream open). drive-proxy adds two things on
//! top:
//!
//!   * a `username -> user_id` cache populated by the validator
//!     wrapper on every successful tunnel handshake, so the
//!     reverse-proxy auth gate can resolve `owner_id` without an
//!     extra round trip to profile-service;
//!
//!   * lookup helpers that bundle the tunnel handle with the cached
//!     `owner_id` and the SPA-facing metadata (`public`, `label`).
//!     `public` and `label` aren't carried by the wire today;
//!     defaults are applied here. Per-tunnel customisation is a
//!     follow-up that extends the Hello frame in chan-tunnel-proto.
//!
//! Cache invalidation: defensive. In normal flow the cache
//! self-converges because `CapturingValidator::validate` runs
//! `record_user` before the tunnel is inserted into the underlying
//! registry, so a `Registry::get` that returns Some always sees a
//! fresh `owner_id`. We still drop the entry explicitly on account
//! delete (`evict_all_for_user` calls `forget_user`) so a future
//! refactor that decouples validate-time recording from
//! registration-time can't leak a stale uuid to the proxy auth gate
//! during the window between account deletion and a same-username
//! signup's first reconnect.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use chan_tunnel_server::{Registry as TunnelRegistry, TunnelHandle, TunnelInfo};
use uuid::Uuid;

/// `RwLock` (not `Mutex`) because `Registry::get` is on the hot path:
/// one read per inbound wildcard request, against an entry that is
/// already cached by `record_user` on tunnel handshake. Writes
/// (`record_user`, `forget_user`) are rare. `parking_lot` would buy a
/// little extra throughput on contended reads but `std::sync::RwLock`
/// keeps dependencies flat and is sufficient for the request rates we
/// target.
#[derive(Clone)]
pub struct Registry {
    tunnels: Arc<TunnelRegistry>,
    user_ids: Arc<RwLock<HashMap<String, Uuid>>>,
}

/// Bundle returned by `Registry::get` for the proxy auth gate plus
/// substream open. Keeps `public` here even though it's a constant
/// today, so the auth-gate code path stays parameterised for the
/// day per-tunnel `public` lands on the wire.
#[derive(Clone)]
pub struct Entry {
    pub handle: TunnelHandle,
    pub owner_id: Uuid,
    pub public: bool,
}

/// Row shape for `/api/me`. `label` defaults to the drive slug
/// until the Hello frame carries a separate display label.
#[derive(Debug, Clone)]
pub struct DriveView {
    pub username: String,
    pub drive: String,
    pub label: String,
    pub public: bool,
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
    ///
    /// Race notes: `record_user` and `forget_user` race on the same
    /// map entry. The intended ordering inside `evict_all_for_user`
    /// is list live tunnels, evict each, then call `forget_user`. A
    /// reconnect from `chan serve` between the list step and the
    /// final `forget_user` step calls `record_user` and reseeds the
    /// cache before the remove fires. That window leaks a stale
    /// uuid only if the new validate observed a uuid different from
    /// the one we forgot, which is impossible under the current
    /// validate path (the underlying account is gone or blocked, so
    /// validate fails outright before `record_user` runs). The
    /// cache is therefore self-converging in practice; this comment
    /// is here so a future validate-path refactor that admits the
    /// post-delete reconnect case is forced to re-examine the
    /// invariant.
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

    /// Resolve a registered tunnel for the proxy auth gate. Returns
    /// `None` when the tunnel disconnected, or when the username
    /// hasn't been seen in any tunnel handshake yet (no cached
    /// owner_id).
    pub fn get(&self, username: &str, drive: &str) -> Option<Entry> {
        let handle = self.tunnels.get(username, drive)?;
        let owner_id = self
            .user_ids
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(username)
            .copied()?;
        let public = handle.public;
        Some(Entry {
            handle,
            owner_id,
            public,
        })
    }

    /// Snapshot every registered tunnel for the admin `tunnel ps`
    /// view. Sorted by `(user, drive)` so output is stable.
    pub fn list_all_tunnels(&self) -> Vec<TunnelInfo> {
        self.tunnels.list_all()
    }

    /// Force a tunnel offline. Returns `true` if a registration
    /// was actually removed; `false` is the "nothing to kill"
    /// case that the CLI surfaces as a 404.
    pub fn evict(&self, user: &str, drive: &str) -> bool {
        self.tunnels.evict(user, drive)
    }

    /// Evict every tunnel a user has live. Used on account-delete
    /// to drop sessions whose backing PAT was just cascade-deleted;
    /// without this the chan serve substreams stay alive until the
    /// remote process exits or the underlying TCP closes. Returns
    /// the count actually evicted (0 is fine).
    ///
    /// Also clears the username -> user_id cache so a brand-new
    /// signup that reuses the username doesn't get rejected by the
    /// proxy auth gate against the old uuid until its first
    /// reconnect. The cache will repopulate on the next successful
    /// validate.
    pub fn evict_all_for_user(&self, username: &str) -> usize {
        let drives = self.tunnels.list_workspaces_for(username);
        let mut killed = 0;
        for d in drives {
            if self.tunnels.evict(username, d.workspace.as_ref()) {
                killed += 1;
            }
        }
        self.forget_user(username);
        killed
    }

    /// Drop the cached username -> user_id mapping. Idempotent;
    /// missing entries are a no-op. Exposed for explicit
    /// invalidation paths (account delete, future block-active
    /// flow); routine tunnel reconnects should let the cache
    /// converge instead of calling this.
    pub fn forget_user(&self, username: &str) {
        self.user_ids
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .remove(username);
    }

    /// Active drives for one user, sorted by drive name for stable
    /// SPA ordering. Empty when nothing is registered.
    pub fn list_for(&self, username: &str) -> Vec<DriveView> {
        self.tunnels
            .list_workspaces_for(username)
            .into_iter()
            .map(|info| {
                let drive = info.workspace.as_ref().to_string();
                DriveView {
                    username: username.to_string(),
                    label: drive.clone(),
                    drive,
                    public: info.public,
                }
            })
            .collect()
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
    fn forget_user_drops_cache() {
        let r = Registry::new();
        let a = Uuid::new_v4();
        r.record_user("alice", a);
        // Probe via the private map: forget_user is what we're
        // testing, so reach in to confirm the entry actually went
        // away rather than relying on an indirect side effect.
        assert!(r.user_ids.read().unwrap().contains_key("alice"));
        r.forget_user("alice");
        assert!(!r.user_ids.read().unwrap().contains_key("alice"));
        // Idempotent on missing entries.
        r.forget_user("alice");
        r.forget_user("ghost");
    }

    #[test]
    fn evict_all_for_user_clears_cache() {
        // Account-delete path: we expose evict_all_for_user without
        // requiring a live tunnel; calling it on a username that was
        // only ever cached (no tunnel) still clears the entry.
        let r = Registry::new();
        let a = Uuid::new_v4();
        r.record_user("alice", a);
        let killed = r.evict_all_for_user("alice");
        assert_eq!(killed, 0, "no tunnels were registered");
        assert!(!r.user_ids.read().unwrap().contains_key("alice"));
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
