//! The launcher's devserver registry seam.
//!
//! The launcher lists and mutates the user's configured devservers, but the
//! devserver set lives in chan-desktop's config (`desktop/src-tauri/src/config.rs`),
//! which sits ABOVE chan-server in the dependency graph and is invisible from
//! here. Mirroring the [`WorkspaceOverlay`](crate::WorkspaceOverlay) inversion,
//! [`WorkspaceHost`](crate::WorkspaceHost) holds an optional
//! `Arc<dyn DevserverRegistry>` the embedder installs; the launcher routes read
//! it at request time. chan-desktop implements the trait over its config vec;
//! the headless devserver and plain `chan open` install none, so the accessor
//! returns `None` and the routes serve an empty list and 404 mutation.
//!
//! The trait lives in chan-library because `WorkspaceHost` — which holds the
//! handle — is a chan-library type, and the crate dependency only flows
//! chan-server -> chan-library, not the reverse. chan-server re-exports these so
//! its routes name them as `chan_server::Devserver*`.
//!
//! Errors are plain `String`s: the only consumer is the route layer, which turns
//! them straight into HTTP bodies, so threading a rich error enum across the
//! chan-library / chan-server / chan-desktop seam buys nothing.

use serde::{Deserialize, Serialize};

/// One configured devserver, as the launcher lists it. The token is WRITE-ONLY:
/// accepted on add/update, never serialized back — [`has_token`](Self::has_token)
/// reports its presence instead.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DevserverEntry {
    /// Stable registry id used for row actions and the connection-state map.
    pub id: String,
    /// The devserver host the desktop dials: hostname or IP, no scheme or port
    /// (`box.example.com`). The desktop forms the dial / tenant URL from `host` +
    /// `port` (`http://{host}:{port}{prefix}...`).
    pub host: String,
    /// The devserver port the desktop dials.
    pub port: u16,
    /// Optional user label for the launcher section header; empty means derive
    /// the label from the URL host.
    pub label: String,
    /// Optional connect script (e.g. an `ssh -L` tunnel) the desktop runs before
    /// the dial.
    pub script: String,
    /// Whether a bearer token is stored for this devserver. The value itself is
    /// never echoed back over the wire.
    pub has_token: bool,
    /// Optional per-library pane-highlight colour as a hex string (`#rrggbb`):
    /// the editor tints a window's active-pane highlight with its library's
    /// colour. `None` falls back to the default accent. Persisted by chan-desktop;
    /// the launcher add/edit dialog sets it. `#[serde(default)]`: a row without
    /// the field reads `None`.
    #[serde(default)]
    pub color: Option<String>,
    /// The library id this devserver is assigned once known, joining its window
    /// rows in the feed to the user's name for it. `None` before the devserver's
    /// first connect, when no library id exists yet.
    pub library_id: Option<String>,
    /// Whether the desktop currently holds a live connection to this devserver.
    /// Volatile runtime state populated by chan-desktop from its connection map;
    /// `false` on a headless/registry-less surface that tracks no connections.
    /// The launcher reads it to show Connect vs Disconnect and gate Edit
    /// read-only while connected. `#[serde(default)]`: a row without the field
    /// reads `false`.
    #[serde(default)]
    pub connected: bool,
}

/// The add/update payload. `host` + `port` are required; the rest are optional.
/// `token` is write-only — `Some` sets it; `None` on an update keeps the stored
/// one. (No `color`: a devserver's pane-highlight colour is set from the
/// focus-border menu and persisted per chan-library, not via this dialog.)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct DevserverInput {
    /// The devserver host (hostname or IP, no scheme or port). Required; the
    /// registry validates it is non-empty.
    pub host: String,
    /// The devserver port. Required.
    pub port: u16,
    /// Optional user label; `None`/empty derives from the host.
    #[serde(default)]
    pub label: Option<String>,
    /// Optional connect script.
    #[serde(default)]
    pub script: Option<String>,
    /// Optional bearer token (write-only). `None` on update keeps the stored one.
    #[serde(default)]
    pub token: Option<String>,
}

/// The launcher's devserver CRUD, inverted so chan-library (and chan-server's
/// routes) reach the desktop config without depending on it. The embedder
/// (chan-desktop) implements it over its persisted `Devserver` vec, persisting on
/// mutate; the headless surfaces install none.
///
/// "Not found" is signalled out-of-band (`Ok(None)` / `Ok(false)`) so the route
/// layer maps it to 404, reserving `Err` for real failures (a bad URL the
/// registry rejects, a persist error).
pub trait DevserverRegistry: Send + Sync {
    /// Every configured devserver, tokens elided. Infallible (mirrors the window
    /// feed): a backing-store read error surfaces as an empty list, not a 500.
    fn list(&self) -> Vec<DevserverEntry>;
    /// Add a devserver, returning the stored row with its assigned id.
    fn add(&self, input: DevserverInput) -> Result<DevserverEntry, String>;
    /// Edit a devserver in place; a blank `token` keeps the stored one. Returns
    /// the updated row, or `Ok(None)` when no devserver has `id`.
    fn update(&self, id: &str, input: DevserverInput) -> Result<Option<DevserverEntry>, String>;
    /// Remove a devserver; `Ok(false)` when no devserver has `id`.
    fn remove(&self, id: &str) -> Result<bool, String>;
}
