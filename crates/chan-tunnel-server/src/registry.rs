//! Live tunnel registry keyed by `(user, workspace)`.
//!
//! A registered tunnel exposes one operation: open a fresh
//! outbound yamux substream against the connected `chan serve`
//! peer. The actual yamux `Connection` is owned by a per-tunnel
//! driver task; the registry only stores a `TunnelHandle` that
//! sends open requests over an mpsc channel.
//!
//! Collision policy: last-writer-wins. When `register` finds an
//! existing entry, it drops the old one. The old driver task sees
//! its shutdown signal and closes its yamux connection, which
//! tears down the underlying h2 stream; the disconnected peer is
//! free to reconnect (with backoff) but will boot whoever is
//! currently registered. This matches a chan-serve restart
//! reclaiming its workspace without waiting for a TCP timeout.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use tokio::sync::{mpsc, oneshot};

#[derive(Debug, thiserror::Error)]
pub enum OpenError {
    #[error("tunnel disconnected")]
    Disconnected,
}

/// Returned by `Registry::register_checked` when registering would
/// take the user over the per-user workspace cap. Carries the username
/// and the cap so the listener can log / report context.
#[derive(Debug, thiserror::Error)]
#[error("user {user} reached max concurrent workspaces ({max})")]
pub struct RegisterCapped {
    pub user: String,
    pub max: usize,
}

/// One row of `Registry::list_workspaces_for`. Pairs the workspace name
/// with the `public` bit captured at handshake time, plus the
/// peer address and connection time so admin tools can render a
/// `ps`-like view without piggybacking on the handshake.
#[derive(Debug, Clone)]
pub struct WorkspaceInfo {
    pub workspace: Arc<str>,
    pub public: bool,
    pub peer_addr: Option<SocketAddr>,
    pub connected_at: DateTime<Utc>,
}

/// One row of `Registry::list_all`. Same as `WorkspaceInfo` but also
/// carries the username so a single call covers the admin
/// `tunnel ps` view.
#[derive(Debug, Clone)]
pub struct TunnelInfo {
    pub user: Arc<str>,
    pub workspace: Arc<str>,
    pub public: bool,
    pub peer_addr: Option<SocketAddr>,
    pub connected_at: DateTime<Utc>,
}

pub(crate) type OpenReply = oneshot::Sender<Result<yamux::Stream, OpenError>>;

/// Sent through `TunnelHandle::open_tx` to ask the per-tunnel
/// driver to allocate a new outbound substream.
pub(crate) type OpenRequest = OpenReply;

/// A handle to a live tunnel. Cheap to clone; the underlying
/// channel is shared. Clones don't keep the tunnel alive on their
/// own: when the registry entry is replaced or removed, the
/// driver's shutdown signal fires, the yamux connection closes,
/// and subsequent `open()` calls return `OpenError::Disconnected`.
#[derive(Clone)]
pub struct TunnelHandle {
    open_tx: mpsc::Sender<OpenRequest>,
    pub user: Arc<str>,
    pub workspace: Arc<str>,
    /// When true the workspace-proxy auth gate skips the OAuth check;
    /// the tunneled `chan serve` is exposed to anonymous public
    /// traffic. Set from the client's Hello frame at handshake.
    pub public: bool,
    /// Peer's TCP address as seen by the listener accept loop.
    /// `None` when the registration didn't go through the listener
    /// path (mainly tests).
    pub peer_addr: Option<SocketAddr>,
    /// Wall-clock time at which the tunnel was registered. Used by
    /// admin tools to render uptime; not load-bearing for routing.
    pub connected_at: DateTime<Utc>,
}

impl TunnelHandle {
    pub async fn open(&self) -> Result<yamux::Stream, OpenError> {
        let (tx, rx) = oneshot::channel();
        if self.open_tx.send(tx).await.is_err() {
            return Err(OpenError::Disconnected);
        }
        rx.await.map_err(|_| OpenError::Disconnected)?
    }
}

struct Entry {
    handle: TunnelHandle,
    /// When this sender is dropped, the per-tunnel driver task's
    /// receiver wakes with a `RecvError`, which it treats as
    /// "you've been evicted; close the yamux connection and
    /// exit". The receiver lives in `serve_tunnel`.
    _shutdown_tx: oneshot::Sender<()>,
}

/// Two-level map keyed `user -> workspace -> Entry`. The split lets
/// `get(&str, &str)` resolve the hash via `Arc<str>: Borrow<str>`
/// without allocating fresh `Arc<str>` lookups on every public
/// request, and makes per-user enumeration a direct inner-map
/// iteration instead of a full-table filter.
type UserMap = HashMap<Arc<str>, HashMap<Arc<str>, Entry>>;

#[derive(Default)]
pub struct Registry {
    inner: Mutex<UserMap>,
}

impl Registry {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Register a new tunnel and enforce a per-user concurrent-workspace
    /// cap atomically with the insert. `max_workspaces_per_user == 0`
    /// disables the check. The cap is enforced under the same lock
    /// acquisition that performs the eviction + insert, so two
    /// parallel dials from the same user cannot both observe
    /// `count == max - 1` and both succeed.
    ///
    /// Reconnect of a workspace the user already holds is always
    /// allowed: the same-key entry is evicted and replaced, and the
    /// user's workspace count is unchanged.
    pub(crate) fn register_with_cap(
        self: &Arc<Self>,
        user: Arc<str>,
        workspace: Arc<str>,
        public: bool,
        peer_addr: Option<SocketAddr>,
        max_workspaces_per_user: usize,
    ) -> Result<
        (
            TunnelHandle,
            mpsc::Receiver<OpenRequest>,
            oneshot::Receiver<()>,
        ),
        RegisterCapped,
    > {
        let (open_tx, open_rx) = mpsc::channel::<OpenRequest>(64);
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let handle = TunnelHandle {
            open_tx,
            user: user.clone(),
            workspace: workspace.clone(),
            public,
            peer_addr,
            connected_at: Utc::now(),
        };
        let entry = Entry {
            handle: handle.clone(),
            _shutdown_tx: shutdown_tx,
        };
        let evicted = {
            let mut g = self.inner.lock();
            let workspaces = g.entry(user.clone()).or_default();
            if max_workspaces_per_user > 0
                && !workspaces.contains_key(&workspace)
                && workspaces.len() >= max_workspaces_per_user
            {
                // Clean up the inner map we may have just created
                // via `or_default` so a capped attempt doesn't leave
                // an empty user bucket behind.
                if workspaces.is_empty() {
                    g.remove(&user);
                }
                return Err(RegisterCapped {
                    user: user.to_string(),
                    max: max_workspaces_per_user,
                });
            }
            workspaces.insert(workspace.clone(), entry)
        };
        if let Some(old) = evicted {
            // Log the eviction with the prior registration's age so
            // an operator can spot flap (two chan-serve instances
            // fighting over the same workspace name) without having to
            // diff connection counts. The Drop on `old` fires the
            // shutdown signal that tells the previous driver to
            // close its yamux connection.
            let prior_age = Utc::now()
                .signed_duration_since(old.handle.connected_at)
                .num_milliseconds();
            tracing::info!(
                %user,
                %workspace,
                prior_age_ms = prior_age,
                "tunnel registration evicted predecessor",
            );
            drop(old);
        }
        Ok((handle, open_rx, shutdown_rx))
    }

    /// Look up a live tunnel for a public request. Returns `None`
    /// if no tunnel is currently registered for that pair. Both
    /// hashmap lookups borrow `&str` straight through to the
    /// `Arc<str>` keys, so no per-call allocation.
    pub fn get(&self, user: &str, workspace: &str) -> Option<TunnelHandle> {
        let g = self.inner.lock();
        g.get(user)
            .and_then(|workspaces| workspaces.get(workspace))
            .map(|e| e.handle.clone())
    }

    /// Workspaces currently registered for `user`, sorted by name.
    /// Used by the public dashboard to enumerate "workspaces I have
    /// online" without needing a separate metadata service. Each
    /// entry carries the `public` bit so callers can render the
    /// public/private badge without a second lookup.
    pub fn list_workspaces_for(&self, user: &str) -> Vec<WorkspaceInfo> {
        let g = self.inner.lock();
        let Some(workspaces) = g.get(user) else {
            return Vec::new();
        };
        let mut out: Vec<WorkspaceInfo> = workspaces
            .iter()
            .map(|(d, e)| WorkspaceInfo {
                workspace: d.clone(),
                public: e.handle.public,
                peer_addr: e.handle.peer_addr,
                connected_at: e.handle.connected_at,
            })
            .collect();
        out.sort_by(|a, b| a.workspace.cmp(&b.workspace));
        out
    }

    /// Snapshot every registered tunnel. Sorted by `(user, workspace)`
    /// so the admin `tunnel ps` view is stable across calls.
    pub fn list_all(&self) -> Vec<TunnelInfo> {
        let g = self.inner.lock();
        let mut out: Vec<TunnelInfo> = g
            .iter()
            .flat_map(|(u, workspaces)| {
                workspaces.iter().map(move |(d, e)| TunnelInfo {
                    user: u.clone(),
                    workspace: d.clone(),
                    public: e.handle.public,
                    peer_addr: e.handle.peer_addr,
                    connected_at: e.handle.connected_at,
                })
            })
            .collect();
        out.sort_by(|a, b| {
            a.user
                .cmp(&b.user)
                .then_with(|| a.workspace.cmp(&b.workspace))
        });
        out
    }

    /// Force a tunnel offline. Drops the registry entry, which
    /// fires the per-tunnel driver's shutdown signal and tears
    /// down the yamux connection. Returns `true` if a row was
    /// removed, `false` if nothing was registered for the pair.
    pub fn evict(&self, user: &str, workspace: &str) -> bool {
        let mut g = self.inner.lock();
        let Some(workspaces) = g.get_mut(user) else {
            return false;
        };
        let removed = workspaces.remove(workspace).is_some();
        if workspaces.is_empty() {
            g.remove(user);
        }
        removed
    }

    /// Remove a registered tunnel only if `handle` is the one
    /// currently stored. Used by the driver task on its own
    /// teardown so it doesn't accidentally evict a successor that
    /// took its slot.
    pub(crate) fn deregister_if_owner(&self, handle: &TunnelHandle) {
        let mut g = self.inner.lock();
        let Some(workspaces) = g.get_mut(handle.user.as_ref()) else {
            return;
        };
        let should_remove = workspaces
            .get(handle.workspace.as_ref())
            // Channel identity is a sufficient proxy: only one
            // mpsc::Sender per registration, cloned into the entry.
            .map(|entry| entry.handle.open_tx.same_channel(&handle.open_tx))
            .unwrap_or(false);
        if should_remove {
            workspaces.remove(handle.workspace.as_ref());
            if workspaces.is_empty() {
                g.remove(handle.user.as_ref());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn evict_on_collision() {
        use tokio::sync::oneshot::error::TryRecvError;
        let reg = Registry::new();
        let user: Arc<str> = Arc::from("alice");
        let workspace: Arc<str> = Arc::from("notes");

        let (_h1, _rx1, mut shutdown1) = reg
            .register_with_cap(user.clone(), workspace.clone(), false, None, 0)
            .unwrap();
        // Before the collision, the receiver has no value and the
        // sender is still alive: try_recv must report Empty.
        assert!(matches!(shutdown1.try_recv(), Err(TryRecvError::Empty)));

        // Re-register the same pair: old entry is dropped, its
        // shutdown sender is dropped with it, so the receiver wakes
        // with Closed.
        let (_h2, _rx2, _shutdown2) = reg
            .register_with_cap(user.clone(), workspace.clone(), false, None, 0)
            .unwrap();
        match shutdown1.try_recv() {
            Err(TryRecvError::Closed) => {}
            other => panic!("expected Closed after eviction, got {other:?}"),
        }

        // The new handle is what the registry returns from now on.
        assert!(reg.get("alice", "notes").is_some());
    }

    #[tokio::test]
    async fn evict_drops_empty_user_bucket() {
        let reg = Registry::new();
        let _h = reg
            .register_with_cap(Arc::from("alice"), Arc::from("notes"), false, None, 0)
            .unwrap();
        assert!(reg.evict("alice", "notes"));
        // After the last workspace is removed, the user bucket is
        // cleaned up so a stale `list_workspaces_for("alice")` doesn't
        // hold a reference to an empty inner map indefinitely.
        assert!(reg.list_workspaces_for("alice").is_empty());
        // Sanity: evict again returns false.
        assert!(!reg.evict("alice", "notes"));
    }

    #[tokio::test]
    async fn lookup_returns_current_handle() {
        let reg = Registry::new();
        let (_h, _rx, _sd) = reg
            .register_with_cap(Arc::from("alice"), Arc::from("notes"), false, None, 0)
            .unwrap();
        assert!(reg.get("alice", "notes").is_some());
        assert!(reg.get("alice", "other").is_none());
        assert!(reg.get("bob", "notes").is_none());
    }

    #[tokio::test]
    async fn register_with_cap_enforces_per_user_limit() {
        let reg = Registry::new();
        let _a = reg
            .register_with_cap(Arc::from("alice"), Arc::from("d1"), false, None, 2)
            .unwrap();
        let _b = reg
            .register_with_cap(Arc::from("alice"), Arc::from("d2"), false, None, 2)
            .unwrap();
        let err = reg
            .register_with_cap(Arc::from("alice"), Arc::from("d3"), false, None, 2)
            .err()
            .expect("third workspace should be capped");
        assert_eq!(err.user, "alice");
        assert_eq!(err.max, 2);
        // Reconnect of an existing workspace bypasses the cap.
        let _a2 = reg
            .register_with_cap(Arc::from("alice"), Arc::from("d1"), false, None, 2)
            .unwrap();
        // Other user is unaffected.
        let _bob = reg
            .register_with_cap(Arc::from("bob"), Arc::from("d1"), false, None, 2)
            .unwrap();
    }

    #[tokio::test]
    async fn register_with_cap_zero_disables_check() {
        let reg = Registry::new();
        for i in 0..5 {
            let _ = reg
                .register_with_cap(
                    Arc::from("alice"),
                    Arc::from(format!("d{i}")),
                    false,
                    None,
                    0,
                )
                .unwrap();
        }
    }

    #[tokio::test]
    async fn list_workspaces_for_returns_sorted_names_per_user() {
        let reg = Registry::new();
        let (_h1, _rx1, _sd1) = reg
            .register_with_cap(Arc::from("alice"), Arc::from("notes"), false, None, 0)
            .unwrap();
        let (_h2, _rx2, _sd2) = reg
            .register_with_cap(Arc::from("alice"), Arc::from("ideas"), true, None, 0)
            .unwrap();
        let (_h3, _rx3, _sd3) = reg
            .register_with_cap(Arc::from("bob"), Arc::from("notes"), false, None, 0)
            .unwrap();

        let alice: Vec<(String, bool)> = reg
            .list_workspaces_for("alice")
            .into_iter()
            .map(|d| (d.workspace.as_ref().to_string(), d.public))
            .collect();
        assert_eq!(
            alice,
            vec![("ideas".to_string(), true), ("notes".to_string(), false),]
        );

        let bob: Vec<String> = reg
            .list_workspaces_for("bob")
            .into_iter()
            .map(|d| d.workspace.as_ref().to_string())
            .collect();
        assert_eq!(bob, vec!["notes".to_string()]);

        assert!(reg.list_workspaces_for("nobody").is_empty());
    }
}
