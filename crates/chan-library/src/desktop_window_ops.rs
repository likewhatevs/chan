//! Bridge from the control socket to chan-desktop's window manager.
//!
//! `cs window list` reads server-side state, but spawning, focusing,
//! closing, hiding, and renaming OS windows are desktop operations: they
//! need the Tauri `AppHandle`, which only the desktop process owns. The
//! embedded server runs in that same process but is handed no
//! `AppHandle` (and chan-server must stay Tauri-free so `chan open`
//! builds standalone). So the desktop installs a [`DesktopBridge`]: an
//! mpsc channel the control socket sends [`DesktopWindowOp`]s down, plus
//! the shared title map (see [`crate::window_titles`]). Each op carries a
//! `oneshot` the desktop completes, so the control handler can `await` a
//! result exactly like the existing window-bus round-trips — but without
//! bouncing through the SPA.
//!
//! In standalone `chan open` the channel is absent ([`DesktopBridge`]'s
//! `window_ops` is `None`); every lifecycle op then refuses with
//! [`NO_DESKTOP`]. The `None` *is* the "no desktop attached" invariant,
//! encoded in the type rather than a runtime flag.

use tokio::sync::{mpsc, oneshot};

use crate::window_titles::SharedWindowTitles;

/// Which kind of window `cs window new` spawns. The control socket fills
/// this in from the calling tenant — a terminal tenant spawns a terminal
/// window, a workspace tenant spawns another window of that workspace —
/// so the CLI never has to name the kind.
#[derive(Debug)]
pub enum NewWindowKind {
    /// A standalone terminal window (the shared `/terminal` tenant).
    Terminal,
    /// Another window of the workspace rooted at `key` (its canonical
    /// path), which the desktop matches against its running workspaces.
    Workspace { key: String },
}

/// A window-management request the control socket hands to the desktop.
/// Each carries a `oneshot` the desktop completes with `Ok(..)`/`Err(msg)`.
#[derive(Debug)]
pub enum DesktopWindowOp {
    /// Spawn a new window; the reply carries its new id (Tauri label).
    New {
        kind: NewWindowKind,
        reply: oneshot::Sender<Result<String, String>>,
    },
    /// Focus a live window or un-hide a buried one; best-effort reopen a
    /// closed-but-saved workspace window when its workspace is running.
    Open {
        id: String,
        reply: oneshot::Sender<Result<(), String>>,
    },
    /// Destroy a window (and let the server drop its saved layout). With
    /// live terminal shells and `force` unset, the desktop raises a
    /// confirmation dialog and this op BLOCKS until the user answers; the
    /// reply is `Ok(true)` (destroyed), `Ok(false)` (no live window
    /// found), or `Err("cancelled")` (user declined).
    Close {
        id: String,
        force: bool,
        reply: oneshot::Sender<Result<bool, String>>,
    },
    /// Bury (hide) a window — the OS close-button behaviour.
    Hide {
        id: String,
        reply: oneshot::Sender<Result<(), String>>,
    },
    /// Connect a registered devserver by id: run its connect command in a
    /// control terminal, scrape the token, dial the URL, and open its
    /// window. The launcher's Connect button drives this over the bridge;
    /// the reply is `Ok(())` once the connect flow is under way (or the
    /// error string when it fails). Inert without a desktop attached — the
    /// route then answers [`NO_DESKTOP`], like the other window ops.
    ConnectDevserver {
        id: String,
        reply: oneshot::Sender<Result<(), String>>,
    },
}

/// Pinned refusal when no desktop is attached (standalone serve / browser).
pub const NO_DESKTOP: &str = "window management requires the chan desktop app";

/// The sender the control socket holds. Bounded so a runaway caller can't
/// grow the queue unbounded; 32 is far above any realistic concurrency of
/// interactive `cs window` calls.
pub type DesktopWindowSender = mpsc::Sender<DesktopWindowOp>;

/// Desktop integration handed to the embedded server. `window_ops` is
/// `None` in standalone mode (lifecycle ops refuse); `window_titles` is
/// always present (empty when no desktop writes to it).
#[derive(Clone, Default)]
pub struct DesktopBridge {
    pub window_ops: Option<DesktopWindowSender>,
    pub window_titles: SharedWindowTitles,
}

impl DesktopBridge {
    /// Send an op and await its reply, mapping the channel-closed / sender
    /// failures to a user-facing string. `None` (standalone) → [`NO_DESKTOP`].
    /// Returns `Err` when no desktop is attached or the manager is gone.
    pub async fn dispatch<T>(
        &self,
        make_op: impl FnOnce(oneshot::Sender<Result<T, String>>) -> DesktopWindowOp,
    ) -> Result<T, String> {
        let Some(sender) = self.window_ops.as_ref() else {
            return Err(NO_DESKTOP.to_string());
        };
        let (reply_tx, reply_rx) = oneshot::channel();
        sender
            .send(make_op(reply_tx))
            .await
            .map_err(|_| "desktop window manager unavailable".to_string())?;
        reply_rx
            .await
            .map_err(|_| "desktop window manager dropped the request".to_string())?
    }
}
