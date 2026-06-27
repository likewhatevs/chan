# Phase 31 - one window registry: a watcher drives the local + devserver window lifecycle

Status: code complete, gated full-tree green; pre-tag close (version pins still `0.40.0`, CHANGELOG
`[Unreleased]` filled, `v0.41.0` not yet tagged — the release cut is a separate step). The
window-lifecycle core landed against both a local `chan serve` path and a connected `chan devserver`,
with headless reload/persistence smokes; the WKWebView-native bits (Cmd+W / red-dot bury vs discard,
the launcher visuals) are desktop-only and lean on manual hand-smoke.
Span: 2026-06-19 → 2026-06-21.
Tags: #window-registry #window-watcher #reload-survival #live-sync #dashboard-config #rich-prompt
#launcher #cs-terminal-close #login-shell-path #async-perf #chan-library

The v0.40.0 round made the `chan devserver` window lifecycle work end to end, but the local desktop
windows and the devserver windows still went through two different imperative open/close layers, and
several window kinds did not survive a reload. This round unifies that: a single per-library **window
registry** is the authoritative window set, and a **window watcher** reconciles native windows against
its live feed — for local windows and a connected devserver alike. Standalone terminals become
first-class library windows under the same lifecycle, so they mint, persist, reconnect, and restore
their layout like any workspace window. On top of the lifecycle core: **live cross-window settings
sync**, **dashboard config moved out of the search index** (so a reindex can't reset it), broader
**reload-survival** (terminal-only / empty-split layouts and the Hybrid flip), **rich-prompt queuing**
fixes, a **Gmail-style multi-select launcher**, `cs terminal close`, **macOS GUI login-shell PATH**
resolution, and an **async/perf** pass. To get there, `WorkspaceHost` and the terminal-session
registry moved into a new `chan-library` crate so the library is the single source of truth and the
`chan-server → chan-library` dependency stays acyclic.

## What shipped

**The window registry and watcher — the core:**

- A new `chan-library` crate holds the authoritative window set: the `WindowRegistry` mints an opaque
  per-library `w-<hex>` window id, assigns the lowest-free "Window N" ordinal per (kind, workspace),
  composes and persists the display title, and persists the set atomically. A change notify drives a
  live window-set watch feed.
- The desktop spawns a surface-agnostic **window watcher** that opens, closes, and restores native
  windows purely by reconciling against that feed — `GET /api/library/windows[/watch]`. Window
  creation paths (Launch, Cmd+Shift+N, `cs window new`, CLI handoff) route through the registry mint,
  so the watcher is the sole driver of local windows; closing a window routes through the watcher's
  view state (discard removes the record; bury hides but keeps it).
- The devserver cutover makes the same watcher the live driver of a connected devserver's windows
  (connect / disconnect / reconnect reconcile its feed), replacing the imperative re-open layer.
- Standalone terminals are now minted + persisted library windows resolved against the shared
  terminal tenant, so they survive Cmd+Q / reopen and a desktop relaunch instead of restarting blank.
- `cs window list` renders the same `WindowRecord` set the desktop watcher and launcher reconcile to,
  so `cs`, HTTP, and the desktop never disagree; the launcher serves the window feed live over the
  library watch socket.

**Live cross-window settings sync:**

- A Settings save (`PATCH /api/config`) broadcasts a synthetic `config_changed` frame on the
  workspace's event bus, so every open window of the workspace re-reads and reflects the change —
  theme, fonts, pane widths, page-width, overlay-maximize — without a reload. It rides the existing
  filesystem-event channel but bypasses the self-write dedupe so siblings reliably see it.

**Dashboard config out of the search index:**

- The screensaver overlay, the report opt-in, and the semantic-search opt-in moved out of the search
  `IndexConfig` (where a reindex or a vector wipe could reset them) into a per-workspace
  `dashboard.toml`. Existing workspaces migrate their toggles in place on first open. The accessors,
  endpoints, and wire forms are unchanged — the move is purely behind the storage.

**Reload-survival and rich-prompt:**

- Terminal-only and empty-split window layouts now persist on disk (the on-open prune no longer GCs a
  layout that has a split or any tab as a phantom), and a restored-empty window's record survives a
  close until it has held content — so neither resets on off/on or restart.
- A Hybrid pane flip (and its per-Hybrid theme) persists across reload: the layout-persist effect now
  observes the flip + theme fields it was missing.
- Rich-prompt queuing is a fire-and-forget composer: submit clears it and keeps it editable for
  back-to-back queuing, ArrowUp recalls the last queued message to edit, and Esc dequeues it (or
  abandons the current draft); a failed send restores the text for retry.
- `cs terminal restart` re-attaches the tab to the relaunched session in place (a new `Restarted`
  event consumed server-side) instead of dropping it as a ghost; a killed session is reaped from the
  registry so it frees its tab name and a re-spawn under that name no longer comes up renamed.

**Launcher, `cs`, and the cs-card:**

- The web launcher gains Gmail-style multi-select with a Turn On / Turn Off / Delete bulk bar (each
  loops the singular library op, no new endpoints) and an Open-terminal button that mints a local
  terminal window.
- `cs terminal close --tab-name | --tab-group` tears down sessions by name or group — the explicit
  teardown partner to restart / new — freeing the tab name.
- The `cs-link-dismissed`, page-width, and overlay-maximize UI settings migrated from browser-local
  storage into per-library server preferences, so they travel with the library and sync live across
  windows; the cs-offer card now gates on real `cs` presence surfaced on the preflight snapshot, not
  the host type.

**macOS login-shell PATH and async/perf:**

- A macOS GUI launch resolves the user's real interactive shell PATH before the embedded server
  starts (so `~/.local/bin`, Homebrew, and custom dirs are visible, fixing the false "create the `cs`
  alias" card under the restricted launchd PATH), bounded with a ~3s timeout so a hanging rc can't
  stall app launch.
- The desktop key-bridge only swallows a keystroke when its IPC is present, so Cmd+R / devtools / zoom
  chords are no longer dead on a devserver window whose bridge didn't survive the cross-origin
  navigation.
- Async/perf: PTY spawn and the `lsof` cwd probes run off the terminal-registry lock and off the
  async runtime; preference writes serialize through one in-flight chain; a workspace-off runs off the
  desktop runtime instead of busy-waiting on the lock release under it.
- The editor hang-recovery buffer is namespaced per workspace root, and the onboarding nudge shows
  only on a workspace's first boot (gated on a "has data" signal, not a per-WebView localStorage
  dismiss).

## Leftovers / deferred

- **Local workspace-off is unguarded.** The devserver off path got the confirm-before-off /
  live-terminal-count guard this round, but the local in-process path still calls `serve::stop`
  directly with no confirm. Pre-existing, not a regression; the one user-facing rough edge, a
  one-commit fast-follow mirroring the devserver guard onto the local path.
- **The team-setup terminal can't reload or preserve state.** Cmd+P in a workspace opens the
  spawn-team dialog, which provisions a terminal for the team work; that window does not preserve its
  exact state across a reload like every other window (on both local and devserver workspaces). Same
  reload / state-coverage class as this round's work, deferred to a subsequent release.
- **New-Terminal → registry + mount-intent unification.** The devserver-desktop New-Terminal create
  side does not yet wire `window_id` through the registry mint + mount intent, so the link-on-create
  cascade is a no-op until it does (a self-healing forget-leftover guard covers the gap meanwhile).
- **The cross-origin `__TAURI__` root cause.** The Cmd+R fix removed the dead-key symptom regardless
  of cause; whether the Tauri bridge truly drops across the connecting-screen → external navigation is
  a runtime probe still owed.
- **The gateway proxy retarget / rename.** A shallow rename (workspace-proxy → devserver-proxy, tunnel
  flags, domains) plus a deeper layer (per-library gate granularity, profile Postgres reshape),
  deferred to a focused pass.
- **The imperative-window-layer deletion pass.** The devserver cutover left the old imperative
  re-open / fetch-terminals orchestrators `#[allow(dead_code)]`'d transitionally; deleting that
  ~15-function set and retiring the legacy menu path is a deferred cleanup once the watcher is the
  sole driver everywhere.
- **Smaller deferrals.** Full preference-write serialization across the remaining low-frequency
  writers; running `Session::spawn` via `spawn_blocking` from its cold async callers; an
  orchestrator restart-in-place leveraging the transparent re-attach; graph-layout + dashboard-config
  persistence as a separate feature; and a devserver-resilience test port-bind TOCTOU flake.
