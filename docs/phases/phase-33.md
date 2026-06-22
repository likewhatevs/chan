# Phase 33 - web-launcher unification across all surfaces + embeddings honesty + carryover

Status: shipped — tagged `v0.43.0` (@ `38612355`), full-tree gate green (workspace + gateway workspace +
web + web-launcher + xwin), version pins at `0.43.0`, CHANGELOG `[v0.43.0]` filled. The library/server/HTTP
behaviors
are covered by unit + HTTP-layer tests and the full gate; the WKWebView-native bits (the desktop launcher
visually rendering at the loopback, click-through, drag-drop folder-add) are desktop-only and deferred to
@@Alex's end-to-end re-smoke against a fresh Mac app. Span: 2026-06-22.
Tags: #web-launcher #chan-library #devserver #devserver-proxy #embeddings #candle #accelerate #editor
#cs-upload-download #team-reload #window-close #root-fallback

Phase 32 made the *devserver = chan-library* and shipped the per-devserver gateway proxy, but the launcher
itself still lived only in the desktop client as a native `main.js`, the devserver/library root `/` still
404'd, and the gateway's "open the whole devserver" was deferred. Phase 33 closes that: one launcher SPA
(`web-launcher`) is served at `/` by the `chan-library` `WorkspaceHost` root fallback and reached on all
three surfaces - the desktop loopback, a `chan devserver`, and the gateway-proxied root - through the
existing transparent proxy. In parallel it resolves the v0.42.0-reported "indexing stalls" report (it was
never a regression - a slow, single-tail-flush cold embed that looked frozen), folds in the editor / team /
window-close / empty-workspace carryover, and adds `cs upload`/`cs download`.

## What shipped

**Web-launcher unification (one launcher, three surfaces):**

- chan-server `static_assets.rs` embeds `web-launcher/dist` (`LauncherAssets` + `serve_launcher`);
  `routes/library.rs` assembles the launcher bundle (the `/` SPA + `/api/library/{workspaces,windows}`);
  an install-once `root_fallback` hook on `chan-library` `WorkspaceHost` (`install_launcher_root_fallback`)
  serves it where no tenant prefix matches `/`. The hook respects the chan-server -> chan-library dependency
  direction (chan-library exposes the slot; chan-server fills it - no frontend bundle in the low-level crate).
- A Makefile `web-launcher` target builds `web-launcher/dist` into the pipeline (it is gitignored), so clean
  CI/release builds embed a real launcher rather than failing the rust-embed derive on a missing dir.
- The desktop loads the SPA from its embedded loopback `http://{addr}/?t={token}` and the native `main.js`
  launcher (+ `index.html`, its serve.rs content tests, the IPC helper) was retired.
- `/api/library/workspaces` list + add/on/off/rm over the `WorkspaceHost` pub API. Auth is per-surface: the
  loopback installs bearer-`Some` (a minted token) with full mutation; the devserver/tunnel installs
  bearer-`None` (tunnel-trust - the gateway proxy is the sole gate) with workspace mutation **read-only**
  (a grantee with a per-workspace-share cookie must not mutate the owner's library; full gateway mutation
  awaits a signed proxy role header, deferred).

**Gateway "Open whole devserver":**

- An owner-only `GET /s/:owner` (identity) mints a `devserver_gate` entry token and 303s the browser to
  `{owner}.devserver.chan.app/?t=`; devserver-proxy's root short-circuit was narrowed so a credential-bearing
  `/` falls through to `proxy::handle` (forwarding to the launcher) while the bare/unauth root still bounces
  to the dashboard. `drv` auto-resolves from the user's one live tunnel. The gateway renders nothing.

**Embeddings - honest cold reindex (the v0.42.0 "stall" report):**

- Root-caused as **slow, not hung, and not a v0.42.0 regression** (v0.41.0 is byte-identical and reproduces
  identically): a prose-dense repo cold-embeds ~1200 chunks in one tail flush of ~14 s/batch CPU gemm, so
  `vectors` stayed 0 for ~8 min and the pill looked frozen. Fixed by committing embeddings incrementally
  (`EMBED_BATCH_CHUNKS` 2048->128 + budget arms) - vectors climb live, the Dashboard settles, search upgrades
  bm25->hybrid mid-run. Apple Accelerate CPU BLAS on macOS adds ~1.5-2x (target-gated like `metal`, proven
  absent from the static-musl Linux binary).

**Carryover + CLI:**

- `cs upload` / `cs download` raise the same Inspector upload/download UI from a workspace terminal (a new
  control-socket `WindowCommand` reusing `fileOps`, standalone + in-terminal).
- Editor source<->rendered toggle gated to renderable files (`.md`/`.json`/`.csv`), Ctrl+E on Linux/Windows;
  `web/EDITOR.md` refreshed to the shipped `@today`/`@date` macros.
- Team-setup dialog survives a window reload (config carried on the lead tab, session-blob only).
- Window-close notice simplified; empty-workspace copy reframed as a project dir + inline Open-terminal.

## Deferred
- `/api/library/devservers*` desktop-registry bridge; a proxy-injected signed role header for grantee
  mutation over the gateway (+ gating the headless loopback bind); the launcher drag-drop folder-add gesture;
  Windows Authenticode signing (certs pending).
