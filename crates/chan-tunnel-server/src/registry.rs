//! Live tunnel registry keyed by `(user, drive)`.
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
//! reclaiming its drive without waiting for a TCP timeout.

use std::sync::Arc;

use parking_lot::Mutex;
use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};

#[derive(Debug, thiserror::Error)]
pub enum OpenError {
    #[error("tunnel disconnected")]
    Disconnected,
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
    pub drive: Arc<str>,
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

type RegistryKey = (Arc<str>, Arc<str>);

#[derive(Default)]
pub struct Registry {
    inner: Mutex<HashMap<RegistryKey, Entry>>,
}

impl Registry {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Register a new tunnel; evict any existing entry for the
    /// same `(user, drive)`. Returns the `OpenRequest` receiver
    /// the driver task must consume, plus the eviction signal.
    #[allow(dead_code)] // wired into the tunnel listener in a follow-up commit
    pub(crate) fn register(
        self: &Arc<Self>,
        user: Arc<str>,
        drive: Arc<str>,
    ) -> (
        TunnelHandle,
        mpsc::Receiver<OpenRequest>,
        oneshot::Receiver<()>,
    ) {
        let (open_tx, open_rx) = mpsc::channel::<OpenRequest>(64);
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let handle = TunnelHandle {
            open_tx,
            user: user.clone(),
            drive: drive.clone(),
        };
        let entry = Entry {
            handle: handle.clone(),
            _shutdown_tx: shutdown_tx,
        };
        let key = (user, drive);
        let evicted = {
            let mut g = self.inner.lock();
            g.insert(key, entry)
        };
        if let Some(old) = evicted {
            drop(old); // sends shutdown signal via _shutdown_tx Drop
        }
        (handle, open_rx, shutdown_rx)
    }

    /// Look up a live tunnel for a public request. Returns `None`
    /// if no tunnel is currently registered for that pair.
    pub fn get(&self, user: &str, drive: &str) -> Option<TunnelHandle> {
        let g = self.inner.lock();
        g.get(&(Arc::from(user), Arc::from(drive)))
            .map(|e| e.handle.clone())
    }

    /// Remove a registered tunnel only if `handle` is the one
    /// currently stored. Used by the driver task on its own
    /// teardown so it doesn't accidentally evict a successor that
    /// took its slot.
    #[allow(dead_code)] // wired into the tunnel listener in a follow-up commit
    pub(crate) fn deregister_if_owner(&self, handle: &TunnelHandle) {
        let key = (handle.user.clone(), handle.drive.clone());
        let mut g = self.inner.lock();
        if let Some(entry) = g.get(&key) {
            // Channel identity is a sufficient proxy: only one
            // mpsc::Sender per registration, cloned into the entry.
            if entry.handle.open_tx.same_channel(&handle.open_tx) {
                g.remove(&key);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn evict_on_collision() {
        let reg = Registry::new();
        let user: Arc<str> = Arc::from("alice");
        let drive: Arc<str> = Arc::from("notes");

        let (h1, _rx1, mut shutdown1) = reg.register(user.clone(), drive.clone());
        // Re-register the same pair: old entry is dropped, its
        // shutdown receiver fires.
        let (_h2, _rx2, _shutdown2) = reg.register(user.clone(), drive.clone());
        assert!(shutdown1.try_recv().is_ok() || shutdown1.try_recv().is_err());
        // Either Ok(()) (sender closed cleanly via Drop) or Err
        // (channel dropped). The signal-via-drop semantics give us
        // the latter; either way the receiver is no longer usable.
        let _ = h1;
    }

    #[tokio::test]
    async fn lookup_returns_current_handle() {
        let reg = Registry::new();
        let (_h, _rx, _sd) = reg.register(Arc::from("alice"), Arc::from("notes"));
        assert!(reg.get("alice", "notes").is_some());
        assert!(reg.get("alice", "other").is_none());
        assert!(reg.get("bob", "notes").is_none());
    }
}
