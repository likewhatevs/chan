# Phase 32 - devserver = chan-library: per-devserver gateway proxy + library-owned open

Status: code complete, gated full-tree green (workspace + gateway workspace + web), version pins at
`0.42.0`, CHANGELOG `[v0.42.0]` filled; tagged `v0.42.0`. The library/store/HTTP behaviors are covered
by unit + HTTP-layer tests and the full gate; the WKWebView-native bits (the desktop New Terminal /
Cmd+Shift+N, intra-window d&d, the rich-prompt composer, Cmd+R reattach) are desktop-only and were
confirmed by Alex's end-to-end re-smoke against a fresh Mac app + a fresh `chan devserver` on lima.
Span: 2026-06-21 → 2026-06-22.
Tags: #chan-library #devserver #gateway #devserver-proxy #per-devserver-tunnel #first-open
#workspace-overlay #terminal-persistence #drag-and-drop #rich-prompt #dead-code

Phase 31 made one window registry the authoritative window set for local and devserver windows. This
round closes the gap the audit called out: the *open rules themselves* still lived in the desktop
client, implemented two different ways, and absent from a headless library. Phase 32 moves them into
the library — so "opening a chan-library spawns exactly one terminal the first time, and a closed-then-
reopened library comes back with none" is a property of `chan-library`, identical whether the desktop
opens its local library or connects to a `chan devserver`. In parallel, the chan.app gateway migrated
from a workspace-proxy to a **per-devserver** model: a user's devserver is a first-class entity reached
through an always-authenticated, segment-preserving reverse-proxy, so the gateway renders nothing and
forwards to whatever the devserver's own router serves.

## What shipped

**Library-owned open (local == remote):**

- A persisted per-library first-open marker (`LibraryState{first_open_done}` in a sibling
  `windows-state.json`) + `WorkspaceHost::ensure_first_open_terminal()` as the single mint path. The
  desktop boot and headless `run_devserver` both route through it, so the per-boot "always a shell"
  floor and the per-connection `Devserver.bootstrapped` flag both collapse into one definition.
- A library-owned **workspace on/off overlay** (`WorkspaceOverlay`, persisted beside each
  `windows.json`): one shape (`{path, on}`) for both the desktop config and the headless devserver,
  replacing the divergent `Config.enabled_workspaces: Vec<String>` and the devserver's
  `PersistedConfig.workspaces`. The library registry stays the existence source; the overlay is the
  on/off layer over it. The mount prefix is no longer persisted — it is re-derived per library at
  restore (the devserver's gateway slug and the desktop's hashed window-label route are different
  schemes).
- **Uniform terminal-window persistence:** `persist_terminal_window` fires for every Terminal window
  (the `library_id != "local"` branch is gone), so local and devserver terminals persist the same way.
- The SPA window URL carries `?lib=<library_id>` so drag-scope and window identity are library-aware.

**The gateway per-devserver migration:**

- `workspace-proxy → devserver-proxy`, `workspace-gate → devserver-gate`; tunnel registration is
  re-keyed on the token-resolved `devserver_id` (SHA-256 of the PAT); the tunnel is always
  authenticated (`public` wire field and the per-workspace public-router path removed). The proxy
  forwards the full inbound path unchanged over a yamux substream to the devserver's own router.
- Identity and profile services consume the per-devserver model: a devserver is a first-class entity,
  with email-based sharing grants and a sharing-only **Devservers** dashboard (opening the whole
  devserver as a launcher is deferred — see "Still open"). Glossary + ADR-0001 record the model.

**Desktop smoke fixes (confirmed by Alex's re-smoke):**

- New Terminal and Cmd+Shift+N on a devserver window mint through the focused window's library instead
  of a local/legacy isolated terminal.
- Library-aware drag-and-drop scope `(library_id, container, workspace)`; the scope token is hex-encoded
  at the DataTransfer MIME boundary so WebKit can't mangle it (the regression that broke even
  intra-window pane drops).
- The rich-prompt composer re-enables + refocuses in the same transaction after a queued message
  drains (no more dead composer until a hide/show).
- The Cmd+R reattach replay window ends on ring-drain, not on `ready`, so historical CPR/DA query
  replies are dropped instead of echoing as `…R`/`…c` at the prompt.

**Removed (dead code):**

- The per-label devserver terminal subsystem (`POST /api/devserver/terminals` + handlers,
  `PersistedTerminal`, the Window-menu terminal-reopen path) — superseded by library terminals on the
  shared tenant; the live workspace menu-reopen path was kept.

## Still open / next

These were scoped OUT of v0.42.0 (acknowledged, not regressions). They form one coherent next theme —
"the devserver serves its own launcher SPA" — captured in `dev/components.md` (last section) and
`dev/devserver-chan-library/followups-next-session.md`:

- **Mount `web-launcher` at the devserver/library root.** The launcher SPA (a pure `/api/library/*`
  HTTP client) is built but not yet served at `/` (the devserver root is still 404). Mounting it would
  give one launcher across three surfaces: chan-desktop (replacing `main.js`), a headless browser, and
  the gateway's deferred "Open devserver."
- **Gateway "Open whole devserver."** Deferred this round (sharing-only dashboard shipped); becomes a
  trivial navigate-to-proxied-root once the launcher is served at `/`.
- **Prod rollout** of the gateway devserver-proxy (gated on v0.42.0 — `gateway/docs/prod-rollout-handoff.md`).
- **Windows Authenticode signing** (procedure in `docs/release/windows-signing.md`; purchase pending).
- Simplify the window-"close" notification to say the window was buried, not closed, and can be
  reopened from the Window menu (next-session follow-up).
