//! The desktop window watcher — chan-desktop as a pure view of the library.
//!
//! Every native window is a reconciled reflection of the library's authoritative
//! window set (Seam W). A `LibraryWatcher` per connected library (the embedded
//! local library + each devserver) holds the latest [`WindowRecord`] snapshot and
//! [`reconcile`]s the native surface to it: open a native window for every library
//! window that lacks one, close every native window the library no longer lists.
//!
//! This replaces the old imperative open/close paths (`reopen_devserver_terminal_
//! windows` / `teardown_devserver_windows` / `track_devserver_window` + the in-memory
//! `devserver_windows` map). Because the reconcile is an idempotent diff keyed by the
//! library-minted id, **reconnect = resubscribe + reconcile can never mint a
//! duplicate** — the L0 always-mint growth path is unreachable by construction.
//!
//! Wiring (lands with G1.5/G2): the local library feeds in-process via
//! `host.assemble_window_records()` + the registry's change `Notify`; a devserver
//! feeds over `GET /api/library/windows` + its watch socket. Both drive the same
//! [`reconcile`]. This module is the surface-agnostic core; the Tauri-side
//! [`NativeSurface`] impl and the per-library watcher tasks bind to it.
//!
//! The items below are exercised by the test suite and consumed by the
//! per-library watcher wiring (the `WindowFeed` impls, the Tauri `NativeSurface`
//! impl, and the `watch_loop` spawn) — the `expect(dead_code)` self-clears the
//! moment that wiring lands.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the window-watcher reconcile core + loop; the per-library watcher wiring consumes it"
    )
)]

use std::collections::HashSet;
use std::future::Future;
use std::sync::{Arc, Mutex};

use chan_server::WindowRecord;
use tokio::sync::Notify;

/// The composite native-window key. `window_id` is unique only within its minting
/// library (Amendment W1: libraries mint independently, no global authority), so the
/// globally-unique native key is `{library_id}::{window_id}`. This string IS the
/// Tauri window label; the `?w=` value is the **bare** `window_id` (decoupled — the
/// per-library SPA/session/presence key). `library_id` ∈ {`local`, `lib-<hex>`} and
/// `window_id` = `w-<hex>`, so `::` never appears inside either part.
pub fn native_label(record: &WindowRecord) -> String {
    format!("{}::{}", record.library_id, record.window_id)
}

/// The native window surface a reconcile drives. Abstracted behind a trait so the
/// reconcile is unit-testable without a live Tauri app (the production impl wraps
/// the `AppHandle` + the renamed `remote_*` window builder).
pub trait NativeSurface {
    /// Native window labels (`{library_id}::{window_id}`) currently open for
    /// `library_id` — visible OR buried; the reconcile owns the bury filter.
    fn open_labels(&self, library_id: &str) -> HashSet<String>;
    /// Open (or rebuild-in-place at the same label) a native window for `record`:
    /// native label = [`native_label`]; the loaded tenant URL carries `?w=<window_id>`.
    fn open(&self, record: &WindowRecord);
    /// Close the native window labelled `label`.
    fn close(&self, label: &str);
}

/// Whether the reconcile surfaces `record` as a native window: persisted, not
/// **locally buried**, and backed by a **live tenant** (a non-empty `token`).
/// Bury is desktop-local view state (L5) — the browser has no native windows, so
/// a bury lives only in this process's `buried` set, never in Seam W. The token
/// gate IS the workspace on/off lifecycle: an off workspace's window carries an
/// empty token (no tenant to attach to), so the reconcile CLOSES it while the
/// library KEEPS the record — turning the workspace back on re-tokens it and the
/// reconcile reopens it at the same window_id (the SPA restores its tabs). Discard
/// is the library op (the record leaves the snapshot entirely).
fn should_show(record: &WindowRecord, buried: &HashSet<String>) -> bool {
    record.persisted && !buried.contains(&native_label(record)) && !record.token.is_empty()
}

/// One idempotent reconcile pass for `library_id`: open every shown record that
/// lacks a native window; close every native window no longer shown (the library
/// discarded it, or it was buried locally). Re-applying the same snapshot is a
/// no-op — which is *why* reconnect (resubscribe → same snapshot) can never spawn a
/// duplicate. A dropped watch frame self-heals on the next full snapshot.
pub fn reconcile(
    library_id: &str,
    snapshot: &[WindowRecord],
    buried: &HashSet<String>,
    surface: &impl NativeSurface,
) {
    debug_assert!(
        snapshot.iter().all(|r| r.library_id == library_id),
        "reconcile got a record from a different library than {library_id}",
    );
    let desired: HashSet<String> = snapshot
        .iter()
        .filter(|r| should_show(r, buried))
        .map(native_label)
        .collect();
    let actual = surface.open_labels(library_id);

    // Open every desired window that has no native surface yet (reattach reuses the
    // existing label — the builder rebuilds in place, never a second window).
    for record in snapshot.iter().filter(|r| should_show(r, buried)) {
        if !actual.contains(&native_label(record)) {
            surface.open(record);
        }
    }
    // Close every native window that is no longer desired (discarded or buried).
    for label in actual.difference(&desired) {
        surface.close(label);
    }
}

/// A library's window-set feed: the current snapshot plus the change signal the
/// loop waits on. The local library implements this in-process
/// (`host.assemble_window_records()` + `host.library_change_notify()`).
///
/// **Why a `Notify`, not an `async fn changed()`:** tokio's `Notified` captures
/// the `notify_waiters()` generation at CREATION (not at first poll), so a
/// `Notified` created BEFORE the snapshot catches a change fired during the
/// snapshot — the next poll sees the advanced generation. The hazard an opaque
/// `async fn changed()` would introduce is creating its `Notified` only when
/// first polled (i.e. AFTER the snapshot), where a same-instant change could be
/// missed. Handing the loop the raw `Notify` lets it guarantee
/// create-before-snapshot, which is the actual correctness property.
pub trait WindowFeed {
    /// The library's current full window set.
    fn snapshot(&self) -> Vec<WindowRecord>;
    /// The change signal, fired (via `notify_waiters`) on every window-set
    /// change. The loop creates this future before snapshotting.
    fn change_notify(&self) -> Arc<Notify>;
}

/// Desktop-local view state the watcher reconciles around. **Bury is
/// desktop-local** (L5 / @@Lead ruling #1): the browser has no native windows,
/// so a buried window lives only in this set, never in Seam W. Mutating it fires
/// `changed` so the loop re-reconciles without waiting on a feed change.
#[derive(Default)]
pub struct WatcherViewState {
    buried: Mutex<HashSet<String>>,
    changed: Notify,
}

impl WatcherViewState {
    /// Bury a native window (the standalone-terminal close button): the next
    /// reconcile closes it, and it surfaces in the Window menu for reopen.
    pub fn bury(&self, native_label: &str) {
        self.buried.lock().unwrap().insert(native_label.to_string());
        self.changed.notify_one();
    }

    /// Un-bury (reopen from the menu): the next reconcile re-opens it.
    pub fn unbury(&self, native_label: &str) {
        self.buried.lock().unwrap().remove(native_label);
        self.changed.notify_one();
    }

    fn buried_snapshot(&self) -> HashSet<String> {
        self.buried.lock().unwrap().clone()
    }
}

/// Drive a library's native surface to its window set: reconcile on every feed
/// change AND every local view change (bury/unbury), until `cancel` resolves
/// (disconnect). The reconcile is idempotent (snapshot-not-delta), so reconnect
/// = resubscribe + reconcile can never spawn a duplicate. On exit the surface is
/// left as-is; disconnect reconciles to empty separately (detach, not reap).
///
/// Correctness: both change `Notified`s are created BEFORE the snapshot. tokio
/// captures the `notify_waiters()` generation at creation, so a change firing in
/// the snapshot↔await window advances the generation and the first poll catches
/// it — not missed. (Each is also `enable()`d to register the waiter eagerly;
/// belt-and-suspenders — the generation capture is what's load-bearing here.)
pub async fn watch_loop<F, S, C>(
    library_id: &str,
    feed: F,
    surface: S,
    view: Arc<WatcherViewState>,
    cancel: C,
) where
    F: WindowFeed,
    S: NativeSurface,
    C: Future<Output = ()>,
{
    let feed_notify = feed.change_notify();
    tokio::pin!(cancel);
    loop {
        // Create both change futures BEFORE the snapshot so each captures the
        // current notify_waiters generation; a change during the snapshot then
        // advances it and the next poll catches it. (enable() arms the waiter
        // eagerly too — harmless belt-and-suspenders.)
        let feed_changed = feed_notify.notified();
        tokio::pin!(feed_changed);
        feed_changed.as_mut().enable();
        let view_changed = view.changed.notified();
        tokio::pin!(view_changed);
        view_changed.as_mut().enable();

        reconcile(
            library_id,
            &feed.snapshot(),
            &view.buried_snapshot(),
            &surface,
        );

        tokio::select! {
            _ = feed_changed => {}
            _ = view_changed => {}
            _ = &mut cancel => break,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chan_server::WindowKind;
    use std::cell::RefCell;

    /// A test surface: a settable "currently open" set + a recording of the
    /// `open`/`close` calls a reconcile makes.
    struct FakeSurface {
        open_now: RefCell<HashSet<String>>,
        opened: RefCell<Vec<String>>,
        closed: RefCell<Vec<String>>,
    }
    impl FakeSurface {
        fn with(open_now: &[&str]) -> Self {
            Self {
                open_now: RefCell::new(open_now.iter().map(|s| s.to_string()).collect()),
                opened: RefCell::new(Vec::new()),
                closed: RefCell::new(Vec::new()),
            }
        }
    }
    impl NativeSurface for FakeSurface {
        fn open_labels(&self, _library_id: &str) -> HashSet<String> {
            self.open_now.borrow().clone()
        }
        fn open(&self, record: &WindowRecord) {
            self.opened.borrow_mut().push(native_label(record));
        }
        fn close(&self, label: &str) {
            self.closed.borrow_mut().push(label.to_string());
        }
    }

    fn rec(library_id: &str, window_id: &str, kind: WindowKind) -> WindowRecord {
        WindowRecord {
            window_id: window_id.into(),
            library_id: library_id.into(),
            kind,
            title: "🏠 Terminal Window 1".into(),
            ordinal: 1,
            workspace_path: None,
            prefix: "/terminal".into(),
            token: "tok".into(),
            persisted: true,
            connected: false,
        }
    }

    fn none() -> HashSet<String> {
        HashSet::new()
    }

    #[test]
    fn opens_a_new_window() {
        let s = FakeSurface::with(&[]);
        let snap = vec![rec("local", "w-1", WindowKind::Terminal)];
        reconcile("local", &snap, &none(), &s);
        assert_eq!(*s.opened.borrow(), vec!["local::w-1"]);
        assert!(s.closed.borrow().is_empty());
    }

    #[test]
    fn reattach_is_idempotent_no_duplicate() {
        // The L0 bug-can't-happen test: a native window already exists for the id,
        // and the same snapshot re-applies (reconnect = resubscribe + reconcile).
        let s = FakeSurface::with(&["local::w-1"]);
        let snap = vec![rec("local", "w-1", WindowKind::Terminal)];
        reconcile("local", &snap, &none(), &s);
        assert!(s.opened.borrow().is_empty(), "must NOT open a duplicate");
        assert!(s.closed.borrow().is_empty(), "must NOT close the live one");
    }

    #[test]
    fn closes_a_discarded_window() {
        // Library no longer lists the window (discarded server-side) -> close it.
        let s = FakeSurface::with(&["local::w-1"]);
        reconcile("local", &[], &none(), &s);
        assert_eq!(*s.closed.borrow(), vec!["local::w-1"]);
        assert!(s.opened.borrow().is_empty());
    }

    #[test]
    fn off_workspace_window_is_closed_and_not_opened() {
        // A workspace turned OFF: its window record persists but carries an empty
        // token (no live tenant). The reconcile must NOT open it, and must CLOSE
        // it if already open — the library keeps the record so a re-on reopens it
        // at the same window_id.
        let mut off = rec("local", "w-1", WindowKind::Workspace);
        off.token = String::new();
        // Not yet open -> stays closed.
        let s = FakeSurface::with(&[]);
        reconcile("local", std::slice::from_ref(&off), &none(), &s);
        assert!(
            s.opened.borrow().is_empty(),
            "off-tenant window must not open"
        );
        // Already open -> reconcile closes it (record kept; tenant gone).
        let s2 = FakeSurface::with(&["local::w-1"]);
        reconcile("local", std::slice::from_ref(&off), &none(), &s2);
        assert_eq!(*s2.closed.borrow(), vec!["local::w-1"]);
    }

    #[test]
    fn buried_window_is_not_opened_and_is_closed() {
        let mut buried = HashSet::new();
        buried.insert("local::w-1".to_string());
        // Buried + not yet open -> stays closed.
        let s = FakeSurface::with(&[]);
        let snap = vec![rec("local", "w-1", WindowKind::Terminal)];
        reconcile("local", &snap, &buried, &s);
        assert!(s.opened.borrow().is_empty(), "buried window must not open");
        // Buried + currently open -> reconcile closes it (bury hides the surface).
        let s2 = FakeSurface::with(&["local::w-1"]);
        reconcile("local", &snap, &buried, &s2);
        assert_eq!(*s2.closed.borrow(), vec!["local::w-1"]);
    }

    #[test]
    fn opens_missing_keeps_existing_closes_extra() {
        // w-1 already open (keep), w-2 new (open), w-9 open but gone from snap (close).
        let s = FakeSurface::with(&["local::w-1", "local::w-9"]);
        let snap = vec![
            rec("local", "w-1", WindowKind::Terminal),
            rec("local", "w-2", WindowKind::Workspace),
        ];
        reconcile("local", &snap, &none(), &s);
        assert_eq!(*s.opened.borrow(), vec!["local::w-2"]);
        assert_eq!(*s.closed.borrow(), vec!["local::w-9"]);
    }

    #[test]
    fn library_id_scopes_the_native_key() {
        // The same window_id in two libraries are distinct native windows.
        let a = rec("local", "w-1", WindowKind::Terminal);
        let b = rec("lib-abc", "w-1", WindowKind::Terminal);
        assert_ne!(native_label(&a), native_label(&b));
        assert_eq!(native_label(&b), "lib-abc::w-1");
    }

    /// Guards the create-before-snapshot property against a REAL `notify_waiters()`
    /// (not the fake feed). The feed fires its change signal SYNCHRONOUSLY during
    /// the first `snapshot()` — inside the snapshot↔await window — then returns the
    /// window on the next snapshot. Because the change future is created before the
    /// snapshot, it captures the pre-fire generation; the fire advances it and the
    /// next poll catches it, so the loop re-reconciles and opens `w-1`. A loop that
    /// created its change future only AFTER the snapshot (e.g. an opaque
    /// `async fn changed()`) would capture the post-fire generation, block forever,
    /// and never open `w-1` — so this assertion fails if that regression slips in.
    #[tokio::test]
    async fn watch_loop_catches_a_change_fired_in_the_snapshot_gap() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        #[derive(Default)]
        struct ShareSurface {
            opened: std::sync::Mutex<Vec<String>>,
        }
        impl NativeSurface for Arc<ShareSurface> {
            fn open_labels(&self, _library_id: &str) -> HashSet<String> {
                self.opened.lock().unwrap().iter().cloned().collect()
            }
            fn open(&self, record: &WindowRecord) {
                self.opened.lock().unwrap().push(native_label(record));
            }
            fn close(&self, label: &str) {
                self.opened.lock().unwrap().retain(|l| l != label);
            }
        }

        struct GapFeed {
            notify: Arc<Notify>,
            calls: AtomicUsize,
        }
        impl WindowFeed for GapFeed {
            fn snapshot(&self) -> Vec<WindowRecord> {
                if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
                    // Fire a change DURING the snapshot — the snapshot↔await gap.
                    self.notify.notify_waiters();
                    Vec::new()
                } else {
                    vec![rec("local", "w-1", WindowKind::Terminal)]
                }
            }
            fn change_notify(&self) -> Arc<Notify> {
                self.notify.clone()
            }
        }

        let notify = Arc::new(Notify::new());
        let feed = GapFeed {
            notify: notify.clone(),
            calls: AtomicUsize::new(0),
        };
        let surface = Arc::new(ShareSurface::default());
        let view = Arc::new(WatcherViewState::default());
        let cancel = Arc::new(Notify::new());

        let surface_in = Arc::clone(&surface);
        let cancel_in = Arc::clone(&cancel);
        let task = tokio::spawn(async move {
            watch_loop("local", feed, surface_in, view, cancel_in.notified()).await;
        });

        // Give the loop time to run the gap iteration + the re-reconcile, then stop it.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        cancel.notify_waiters();
        let _ = task.await;

        assert_eq!(
            *surface.opened.lock().unwrap(),
            vec!["local::w-1".to_string()],
            "create-before-snapshot must catch the gap-fired notify_waiters and reconcile",
        );
    }

    #[test]
    fn view_state_bury_unbury_tracks_local_set() {
        let view = WatcherViewState::default();
        assert!(view.buried_snapshot().is_empty());
        view.bury("local::w-1");
        view.bury("local::w-2");
        assert_eq!(view.buried_snapshot().len(), 2);
        assert!(view.buried_snapshot().contains("local::w-1"));
        view.unbury("local::w-1");
        assert_eq!(
            view.buried_snapshot().into_iter().collect::<Vec<_>>(),
            vec!["local::w-2".to_string()]
        );
    }
}
