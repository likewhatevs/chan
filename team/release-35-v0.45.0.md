# Phase 35 - the v0.45.0 desktop release: launcher + devserver-in-launcher + lifecycle hardening

Status: released as `v0.45.0`. Four rounds, the last three driven directly by Alex's desktop hand-smoke.
Round 1 finished the launcher on the desktop/WKWebView surface the v0.44.0 headless gate couldn't reach
(on-state, live-refresh, Connect, show/hide, folder picker, auto-update) plus parallelizable carryover.
Round 2 built the full devserver-in-the-launcher experience (windows + served workspaces merged, launcher
row redesign + bulk actions, per-library focus colour, ws-off force-confirm, graph-nav refinements) and gut
the README. Round 3 corrected what Alex's first smoke flagged (devserver model URL→Host+Port, the colour
redesign to a per-host store, control-terminal-in-launcher + auto-hide, window cleanup on off/forget, the
graph chord restore). Round 4 closed the re-smoke bugs (devserver window show/hide hang, control terminal on
reconnect, turn-off preserves layout / only Forget purges, show-from-dot unbury, local-off live-terminal
confirm) and folded in the marketing homepage rework + manual consolidation. Each lane own-gate-green;
full-tree `make pre-push` re-run green at each round close; version pins bump at tag. The WKWebView-native
bits ship gated-green + smoke-confirmed by Alex. Span: 2026-06-22 .. 2026-06-23.
Tags: #web-launcher #chan-library #devserver #desktop-bridge #auto-update #folder-picker
#cs-upload-download #transfer-preflight #graph-indexing #app-icon #6-agent-team

Phase 34 shipped launcher-reflects-reality + the transfer bubble + `chan open`/`close`, but Alex's
post-release desktop smoke found the launcher was half-broken on the **desktop / WKWebView** surface that
the headless e2e + static gate couldn't reach: a desktop-served workspace showed "Turn on" (B1), the
devserver Connect button was a disabled stub (B3), the list didn't live-refresh (B1a), and the desktop
never auto-checked for updates (B4 — why v0.43 → v0.44 didn't self-update). Phase 35 finishes the launcher
on the desktop, fixes auto-update, makes the Open-windows show/hide work, and pulls in the parallelizable
carryover (standalone transfers, the upload-surface unification, the devserver close-guard, the
tunnel-publish docs).

## What shipped

**Launcher reflects reality on the desktop (B1):**

- **By-root resolution (the divergence fix).** The launcher computed a workspace's `on` as
  `mounted_prefixes().contains(allocate_workspace_prefix(root))` → `/<slug>`, but the desktop mounts the
  tenant at `workspace-<hash>` (`serve::workspace_window_prefix`), so the slug prefix was never mounted →
  `on:false` on the desktop. New `WorkspaceHost::is_root_mounted(root)` + `mounted_prefix_for_root(root)`;
  `handle_list_workspaces` computes `on` by root, and off/remove target the **actual** mounted prefix.
  The `LauncherWorkspace` wire shape is unchanged.
- **Live-refresh (B1a).** The workspace list re-fetches on the existing `/api/library/windows/watch`
  push — the library change-signal already fired on workspace mount/unmount, so `chan open` / on / off now
  reflect with no manual reload.

**Devserver connect (B3):** The launcher's hardcoded `disabled` Connect stub is now a real button wired to
`POST /api/library/devservers/:id/connect`, gated `!readOnly` (the same gate as window open/hide), inert in
a plain browser (the route answers `NO_DESKTOP`/409 with no bridge). The connect itself reused the existing
fully-implemented desktop flow (control terminal + token scrape + dial + window watcher) — exposed over the
**DesktopBridge** as a new `DesktopWindowOp::ConnectDevserver` variant, the same mechanism window open/hide
already use. The standalone `#[tauri::command] connect_devserver` was dropped (the launcher is pure HTTP;
no other caller).

**Desktop auto-update on launch (B4):** `setup()` spawns a background on-launch `updater.check()` (reusing
the `desktop_handle_upgrade` download+install path), prompts to restart on an available update, and honors
`CHAN_UPDATE_CHECK=0`. unix-only (no Windows updater feed). This is why a directly-booted older desktop
never self-updated.

**Open-windows show/hide (§4):** The whole Open-windows row is now a show/hide toggle (was only the dot).
On the desktop side this surfaced a **real bug, not a confirm**: the launcher status-dot sends a bare
`window_id`, but watched windows carry composite `{library_id}::{window_id}` native labels, so
`get_webview_window` missed them; and a buried *local* window is destroyed on bury (no webview) so it could
never reopen. Fixed with a `resolve_window_label` view + reopening `local::` windows through the watcher.

**New-Workspace folder picker (B2):** A native OS folder dialog (`tauri-plugin-dialog`) behind a **Browse…**
button, exposed over the DesktopBridge as a new `DesktopWindowOp::PickFolder` variant +
`POST /api/library/fs/pick-folder` (returns the chosen path or `null` on cancel). The native picker was
chosen over a path-autocomplete because `web/` and `web-launcher/` are separate bundles with no shared
infra and there is no OS-directory listing surface — the picker reuses the bridge and adds no new infra.

**Carryover (§6):**

- **#1 — standalone-terminal `cs upload`/`cs download`.** Library-level transfers from a standalone
  terminal (no workspace), cwd-anchored / shell-uid scoped (Option A — a transfer wall is theater a shell
  defeats; the boundary is the authenticated owner). New `routes/transfer.rs` mounts cwd-anchored handlers
  on the terminal router at the **same** `/api/files/upload` + `/api/files/*path?download=1` URLs the SPA
  bubble already calls → no web/ change. `<path>` is required (`.` = cwd); a directory streams as a tarball
  built in memory (no staged temp → a cancelled download leaves no traces). **Pre-flight (both paths):**
  download verifies the whole source tree is readable before tarring; upload verifies the destination is
  writable before writing — fail fast, no partial artifact, clear errors.
- **#3cg — transfer close-guard for connected-devserver windows.** A per-window `active_transfer` bit on
  the windows feed (`WindowRecord.active_transfer`, populated from `tenant_has_active_transfer` in
  `assemble_window_records`); the desktop caches it per-devserver off each `WindowSet` push and the
  devserver-window close handler checks the cache (the desktop webview can't see the remote `/ws` traffic).
- **#4 — upload-surface unification.** `replaceFileAt` now drives the transfer bubble; the upload
  status-bar text (`fileTransferStatus`) is retired (v0.44.0 retired the download bar only).
- **#6 — tunnel-publish docs.** README / manual / marketing + the tunnel-crate `design.md`/`README`/
  `Cargo.toml`/source comments corrected to the `chan devserver` reality (publishing is the whole library,
  one registration, `{user}.devserver.chan.app/{workspace}/`); the anonymous "public tunnel" section
  removed (always authenticated, B5); and chan-desktop removed from the tunnel-crate consumer docs (it was
  never a consumer).

**Other folded-in fixes:**

- **Graph "still indexing" state.** The graph empty-state distinguishes indexing from genuinely-empty:
  while the index builds it reads "graph temporarily unavailable while indexing the workspace" (gated on
  the existing `indexBuilding` derived), and the graph auto-repopulates when indexing finishes (a new
  edge-triggered effect, since the load effect didn't depend on the index state).
- **File-browser inspector Open (L6).** Gates on the server-provided content `kind`
  (`isOpenableTextKind`) so an odd-extension plaintext file's inspector reads **Open**, matching the tree's
  content-peek, instead of Download.
- **New desktop app icon (B6).** Black enso on cream paper, margin cropped tight, centered on the fitted
  circle, macOS squircle.

## What shipped - rounds 2-4 (desktop-smoke-driven)

**Round 2 - devserver-in-the-launcher + redesign:**

- **Devserver feed source.** A `DevserverFeedSource` trait installed on `WorkspaceHost` (like the
  devserver registry / overlay) merges each connected devserver's window records + served workspaces into
  `assemble_window_records` / `handle_list_workspaces`, with a per-workspace cache + change-signal so the
  launcher live-updates. Five new `DesktopWindowOp` bridge variants (disconnect, open-terminal,
  open-workspace, set-workspace-on, forget-workspace) + their HTTP routes; `connected` on the launcher
  payload; `LauncherWorkspace` tagged with `library_id`/`prefix` (slash-free invariant) for routing.
- **Launcher redesign.** Icon-button registry rows (New-window / On-Off; New-terminal / Edit /
  Connect-Disconnect) + bulk multi-select Turn-on / Turn-off / Remove; Edit read-only while connected.
- **ws-off force-confirm** (seam #4): an unforced off of a workspace with live terminals answers 409
  `live_terminals` and the launcher confirms + retries forced (devserver path this round; local in round 4).
- **Graph nav** (seam): Graph-from-here + inspector Open each open a new tab; the graph grounds on the FS
  spine and layers index edges as they settle.
- **Per-library colour (seam #5, first cut):** a `WorkspaceHost::pane_color` resolver + `?pane` injection.
- **README** reduced to a minimal pointer; the **queue-drain discipline** baked into the team bootstrap;
  web-check gated to include the launcher's svelte-check + vitest; the NaN%-on-directory-download guard.

**Round 3 - corrections from Alex's first hand-smoke (8 findings):**

- Devserver model **URL -> Host + Port** (S2); the round-2 colour pickers reverted.
- **Window cleanup on off/forget** (S1): off/forget no longer leaves stale window records (later refined in
  round 4 to preserve-on-off / purge-on-forget).
- **Colour redesign** to a per-host model: each host (local + each devserver) owns a file-backed
  `LocalColorStore`; the SPA focus-border menu PUTs its serving host; the desktop caches devserver colours
  for the `?pane` inject; `DevserverEntry.color` dropped.
- **Control terminal in the feed** (S3): a `control` flag + `control_terminal` record rendered first;
  **auto-hide-on-connect** field + form checkbox.
- **Graph chord restored** to `Cmd+Shift+M` (the round-2 retirement was a mistake; `Cmd+Shift+G` stays
  Find-previous; hybrid `Mod+. M`) - decided by Alex survey after a G-vs-M reconciliation.

**Round 4 - re-smoke bugs (all gated green) + marketing:**

- **B1** devserver standalone-terminal dot-hang (the shared `/terminal` tenant never got the remote
  `connected:false` push -> a desktop-side `buried` override); **B2** control terminal now re-emits on
  reconnect; **B3** turn-OFF **preserves** the window records + the SPA layout blob (filtered from the live
  feed while off) and turn-ON restores them, only **FORGET** purges - the existing watcher reconcile already
  reopens-on-reappear (symmetric); **B4** the launcher dot can **show** a buried devserver window (the bare
  `window_id` was resolving to `local::` because `resolve_window_label` scanned only open windows - fixed to
  match the buried list + the `lib-` family); **B8** local workspace-off now confirms live terminals like
  the devserver path (shared `offWorkspaceWithConfirm`). **B6** (Quit-dialog Enter) + **B7** (self-upgrade
  prompt - a non-bug: the dev build's version equalled the published latest) were diagnosed and deferred.
- **Marketing homepage rework + manual consolidation** folded in from the `chan-web-marketing` worktree
  (cherry-pick + the just-made screenshots).

## Team / process

6-agent round (Lead + Server + @@webdev + WebMain + Desktop + CLI), seam-first. Lead ran four
read-only Explore sweeps before pinning the seams — which materially right-sized the work: most of seam #1
(by-root) already existed (`live_workspace` + the library change-signal + the watch-feed snapshot push),
and the entire desktop connect flow (seam #2) was already implemented, so B1/B3 were far smaller than the
plan feared. Three seams were pinned against the **live code** (not the plan's file guesses): #1 by-root,
#2 connect-via-DesktopBridge, #3 the folder picker (decided by Alex as a native picker over the planned
autocomplete). Dispatch + journals under `dev/v0.45.0/team/` (gitignored live bus). Alex folded in two
mid-round directives (transfer pre-flight checks; the `cs` download/upload parameter contract).

## Retrospective

### Done

B1 (by-root + live-refresh), B3 (connect), B4 (auto-update), §4 (show/hide + the real label-resolution
fix), B2 (native folder picker), the §6 carryover (#1 standalone transfers + pre-flight + the param
contract, #3cg devserver close-guard, #4 upload unification, #6 tunnel docs + B5), and the fold-ins (graph
indexing copy + auto-reload, L6 inspector Open, B6 icon). All lanes own-gate-green; Lead verified
clean-scope at every accept.

### Pending (deferred to Alex / next phase)

- **Alex's live desktop smoke** — the WKWebView bits (launcher on-state + live-refresh, Connect dial,
  Open-windows show/hide, native folder picker, standalone-terminal `cs upload`/`download` end-to-end).
  Ships gated-green + live-unverified per the pre-release norm; the on-launch self-update is validated by
  the v0.44.0 → v0.45.0 upgrade.
- See "Next-phase follow-ups" below.

### Highlights

- **Investigation-first seam pinning was the round's best decision.** Four Explore sweeps against HEAD
  before pinning found that seam #1 was mostly already built and seam #2's connect flow already existed —
  so the two "SIGNIFICANT" bugs (B1, B3) collapsed to a narrow on-state fix + a button-wire + a bridge
  variant. Pinning against live code, not the plan's prose, prevented wasted lane work.
- **DesktopBridge reuse.** Connect and the folder picker both landed as new `DesktopWindowOp` variants on
  the existing HTTP→bridge→desktop path that window open/hide already use — minimal, consistent, no new IPC
  surface, and self-protecting (409 with no bridge).
- **Desktop's §4 was a real-bug catch, not a confirm** — composite native-label resolution + the
  buried-local-window reopen, exactly the desktop-only gap class this round existed to close.
- **CLI's "mount at the existing `/api/files` URLs"** kept standalone transfers entirely backend-side
  with zero web/ change, and the pre-flight guards fail fast with no partial artifact.
- **WebMain's graph diligence** — traced that the graph's load effect didn't depend on the index state
  (so the "temporarily" copy would have stuck), flagged it honestly, and proposed the ~6-line auto-reload
  rather than shipping a half-fix.
- **Clean atomic commits across all six lanes**, each verified clean-scope by Lead at accept time, with
  the shared chan-server `routes/` boundary explicitly sequenced (CLI sole editor, Server registered
  the boundary) so nothing clobbered.

### Lowlights

- **Poke-crossing / stale queue, again.** Desktop's fast lane outran Lead's pokes — Lead fired
  "do X" pokes on already-completed work and Desktop re-confirmed several times. Same lowlight as phase
  34; the fix is to read tree/journal state before each poke to a fast lane.
- **A whole-workspace `cargo fmt`** from Server reformatted peer WIP (Desktop + CLI files) between
  its two runs — caught and flagged (no revert, committed only own paths), and Server scoped fmt to owned
  files afterward.
- **The `<path>`-required spec gap.** CLI's first transfers pass shipped `cs upload`/`download` with
  `path` optional (defaulting to cwd), contradicting Alex's explicit "always require a `<path>`"
  directive — caught on Lead's source review at accept time, not by the lane's self-check, because the
  lane's completion report didn't mention the directive at all.
- **Two desktop compile windows instead of one.** Server landed `ConnectDevserver` alone then moved to
  seam #1, deferring `PickFolder`, so the desktop crate took two non-exhaustive-match windows rather than
  the single bundled burst Lead had pinned. Harmless (only `cargo check --workspace` cared), but not the
  plan.

### Honest feedback

- **To the workers:** strong throughput and judgment — Desktop's bug catch, CLI's URL reuse + the
  in-memory-tar no-traces design, WebMain's auto-reload trace, Server's tight seam implementation,
  @@webdev's clean launcher wiring. Two asks: (1) self-verify your deliverable against **every** directive in
  the task before reporting done — the `<path>`-required gap should have been caught lane-side; (2) report
  completion crisply once and trust the ack, to stop the stale-poke loop.
- **To Lead (me):** I poked a fast lane (Desktop) repeatedly on work it had already finished — I should
  read HEAD + the lane's journal before each poke rather than fire from a stale mental queue. And I accepted
  lane reports at face value at first; I only caught the `<path>`-required deviation on a deeper code read —
  I should verify each Alex directive against source at accept time, not spot-check the happy path. The
  one thing I'd keep unchanged: the up-front investigation sweeps before pinning seams.
- **To Alex:** the mid-round directives (the transfer pre-flight checks, the `cs` parameter contract)
  were correct and valuable, but the parameter contract arrived while CLI was mid-build, which is what
  produced the `<path>`-required rework. Specifying the full command parameter contract before the lane
  starts building would avoid the redo — a milder echo of phase 34's late fold-ins.

### Rounds 2-4 retrospective (the smoke-driven arc)

**Highlights.**
- **The smoke -> fix -> re-smoke loop did its job.** Every round-3/4 bug was a real WKWebView/desktop fault
  the headless gate + static checks structurally cannot see (dot-hang, can't-unhide, off/on layout loss,
  reconnect, live-terminal off). The right model for this surface is exactly what we ran: ship gated-green,
  Alex hand-smokes, fixes fold back in, re-smoke.
- **Desktop's diagnoses were the round's backbone.** B4 (the bare `window_id` resolving to `local::`
  because `resolve_window_label` only scanned open windows - deeper than Lead's `:520` trace), B1 (the
  shared `/terminal` tenant never receiving the remote `connected:false` push), and B7 (proving the
  self-upgrade was a non-bug: dev-build version == published latest) each saved a wrong or wasted fix.
- **Work distribution under load.** When the re-smoke piled B4/B6/B7 on Desktop, B8 was split to Server
  (route) + @@webdev (launcher) to keep lanes parallel; the final re-gate ran clean on an idle tree.
- **Per-host colour landed coherent** after the churn: each library owns a file-backed store, the SPA talks
  to its serving host, the desktop caches devserver colours - matching Alex's actual ask.

**Lowlights.**
- **Colour churned hard.** The design flip-flopped (pinned `pane_color` -> "corrected" to two-source ->
  re-added -> fully reshaped to the per-host store; round-2 pickers built then reverted). Root cause: Lead
  re-deciding against a moving target instead of pinning once against live code. Most decision-churn of the
  release.
- **Poke-crossing / stale-queue, worse than phase 34.** In rounds 3-4 the fast lanes reported faster than
  Lead drained the poke queue; Lead re-poked already-decided items (W5 chord, W3/W4/W6 readiness) and
  workers re-reconciled repeatedly - @@webdev eventually went silent to break the loop. The queue-drain
  discipline (now in the bootstrap) helped but "report-before-draining" persisted.
- **Build contention SIGTERMed the first `.app`.** Lead launched two heavy builds (desktop + lima musl)
  at once while workers were still compiling in the shared tree; the desktop build was terminated. Fixed by
  building single + on an idle tree. (The isolated-worktree fallback was readied but not needed.)
- **A route-shape flip-flop** (devserver ops body-vs-path) and a **"hold colour + proceed" contradiction**
  Lead issued and had to own - both from steering without first reading the committed state.

**Honest feedback.**
- **To the workers:** excellent diagnostic depth (Desktop especially) and clean, scope-tight commits
  throughout four rounds. Keep the one habit @@webdev modeled best: report completion once, then go quiet until
  pinged - it's what finally stopped the reconcile loop.
- **To Lead (me):** two repeat faults. (1) Decide once and pin against the **committed code**, not a
  moving plan - the colour churn and the route flip-flop both came from re-deciding mid-flight. (2) Read
  HEAD + the lane's journal **before** every poke to a fast lane; I fired decided-work pokes and crossed my
  own acks all round. And don't parallelize heavy builds into a busy shared tree. What worked and I'd keep:
  isolated full-tree re-gates at each round close, scope-verifying every accept, and distributing re-smoke
  load off the saturated lane.
- **To Alex:** the rapid hand-smoke was the highest-value input of the release - it found bugs nothing
  else could, and the "fix all of these, then I'll test again" cadence kept rounds tight. The only cost is
  that several findings refined earlier ones (colour, off/on lifecycle), so some work was built twice; a
  short up-front "here's the full devserver-in-launcher behavior I want" sketch could have collapsed the
  colour rounds into one. Net, the iterative smoke was right for a surface the gate can't reach.

## Next-phase follow-ups

1. **chan-library metadata** (stretch, not pulled): move workspace-metadata download/upload to become
   chan-library metadata, living in the web-launcher SPA.
2. **Devserver-proxy / OAuth dial** (stretch, not pulled): the proxied `https://{user}.devserver.chan.app`
   connect — the URL scheme + port default are in place; the OAuth branch is marked in `devserver.rs` and
   not built. Raw/loopback connect ships this round.
3. **Streaming directory download.** The directory tarball is currently built fully in memory before
   sending (which satisfies the no-traces-on-cancel requirement by construction); a true on-the-fly tar
   stream would avoid buffering a large directory in RAM.
4. **Upload writability probe temp.** The workspace upload writability check drops a transient
   `.chan-upload-check-*` temp (same class as `atomic_write`'s `.tmp*`); if an upload shows a spurious tree
   refresh, that is the suspect.
5. Any findings from Alex's desktop smoke.

### Deferred to v0.46.0 (from Alex's v0.45.0 hand-smoke; detail in `dev/v0.46.0/carryover.md`)

- **Editor robustness cluster** — F1: editor reports "document not found" for a file in the same directory;
  F2: Graph "Open" on a *file* node should open the Editor (not the File Browser); F4: cursor can't enter /
  is skipped around fenced code blocks. F1 is Alex's top follow-up.
- **F3** — Cmd+Shift+T reopen-closed-tab doesn't restore a File Browser's expanded directories.
- **F5** — native confirm dialogs ignore Enter-to-default (e.g. "Quit Chan?"); a deep rfd/tauri-plugin-dialog
  limit (alert window never key + no default-button setter). Cleanest fix is a `cfg(macos)` NSAlert shim,
  covering all 5 confirm sites.
- **F6** — per-item confirm *during* a bulk workspace-off (single-row off confirms; bulk is fail-safe today).
- **Devserver collaboration model** — a deliberate multi-client design (follow each other's window show/hide,
  per-user pane-colour overlay); resolves the two un-hardened multi-client hazards (Tauri window-label
  collisions, PTY interleaving). Needs a dedicated design pass + a survey to Alex on scope.
