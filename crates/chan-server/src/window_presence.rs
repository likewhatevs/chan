//! Live-window presence: which window ids currently hold a `/ws`
//! socket against this tenant.
//!
//! Every SPA window opens one event socket and tags it with its window
//! id (`/ws?w=<id>` — the same id that keys the per-window session
//! blob). The refcounted map below turns those sockets into a presence
//! set consumed by `GET /api/windows` and `cs window list`, so a
//! desktop attached to a remote serve can tell which saved windows are
//! currently open somewhere and which are reopenable.
//!
//! Refcounted, not boolean: a reload briefly overlaps the old and new
//! socket of the same window, and a plain set would flicker the window
//! "disconnected" when the old socket drops. Connections register via
//! the RAII [`PresenceGuard`] so every pump exit path (clean close,
//! network drop, server shutdown) deregisters without bookkeeping at
//! the call sites.
//!
//! Semantics note for consumers: "connected" means a live socket
//! exists SOMEWHERE — a hidden (buried) chan-desktop window keeps its
//! webview and therefore its socket, so hidden-vs-visible is not
//! distinguishable here. The honest vocabulary is connected / saved.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct WindowPresence {
    /// window id -> live socket count.
    inner: Mutex<HashMap<String, usize>>,
}

impl WindowPresence {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register one live socket for `id`; presence holds until the
    /// returned guard drops.
    pub fn connect(self: &Arc<Self>, id: &str) -> PresenceGuard {
        let mut inner = self.lock();
        *inner.entry(id.to_string()).or_insert(0) += 1;
        PresenceGuard {
            presence: Arc::clone(self),
            id: id.to_string(),
        }
    }

    /// Window ids with at least one live socket, in arbitrary order.
    pub fn connected_ids(&self) -> Vec<String> {
        self.lock().keys().cloned().collect()
    }

    /// Recover from a poisoned lock: the critical sections are simple
    /// counter ops that can't leave the map inconsistent, and presence
    /// must never panic a ws teardown path.
    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, usize>> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn disconnect(&self, id: &str) {
        let mut inner = self.lock();
        if let Some(count) = inner.get_mut(id) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                inner.remove(id);
            }
        }
    }
}

/// RAII handle for one window socket; dropping it releases the
/// presence ref. Held by the `/ws` pump for the socket's lifetime.
pub struct PresenceGuard {
    presence: Arc<WindowPresence>,
    id: String,
}

impl Drop for PresenceGuard {
    fn drop(&mut self) {
        self.presence.disconnect(&self.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn connected(presence: &WindowPresence, id: &str) -> bool {
        presence.connected_ids().iter().any(|c| c == id)
    }

    #[test]
    fn presence_follows_guard_lifetimes() {
        let presence = Arc::new(WindowPresence::new());
        assert!(!connected(&presence, "w1"));

        let g1 = presence.connect("w1");
        assert_eq!(presence.connected_ids(), vec!["w1".to_string()]);

        // Reload overlap: a second socket for the same window keeps the
        // window connected after the FIRST guard drops.
        let g2 = presence.connect("w1");
        drop(g1);
        assert!(connected(&presence, "w1"));

        drop(g2);
        assert!(presence.connected_ids().is_empty());
    }

    #[test]
    fn windows_track_independently() {
        let presence = Arc::new(WindowPresence::new());
        let _g1 = presence.connect("w1");
        let g2 = presence.connect("w2");
        let mut ids = presence.connected_ids();
        ids.sort();
        assert_eq!(ids, ["w1", "w2"]);
        drop(g2);
        assert!(connected(&presence, "w1"));
        assert!(!connected(&presence, "w2"));
    }
}
