# Phase 12 backlog (carryover from phase 11 + phase 10)

Opened 2026-05-27 from the phase-11 close. main baseline at open: `5f25cc1`
(phase-11 continuation closed, all local, NOT yet pushed - the first push fires
the new `make ci-linux`/`ci-macos` CI over the whole phase-11 round).

Lane assignment up front (detail in `bootstrap.md`):
- @@LaneA = graph + File Browser carryover (may spawn 2-3 subagents).
- @@LaneB = drive -> workspace terminology/docs/codemod (SCOPE FIRST).
- @@LaneC = @@Alex ad-hoc frontend / cosmetics / keyboard shortcuts.
- Future lanes (@@Alex will add): a release/build lane is the obvious home for
  the release carryover below.

## Graph + File Browser  (-> @@LaneA)

- OVERLAY / SCOPE-CONCEPT WIPE (the big one): `../phase-11/overlay-scope-wipe-spec.md`
  (W1-W7, design-resolved with @@Alex). Kill `availableGraphScopes` / "panes
  form scope"; scope == filesystem directory (tag rootable depth-1, language
  rootable, file -> parent dir, drop group/global/git_repo); GraphPanel
  graphState=tab; dock owns browserState; retire legacy `?graph=`/`?files=`
  hash (degrade old bookmarks gracefully); delete the LOAD-BEARING
  graphOverlay/browserOverlay state (coupling documented in the spec). C3
  (FileBrowserSurface/FileTree browserOverlay leftovers) is the FB slice of it.
- GI-10: drive node pinned to the BOTTOM, spine grows upward (GraphCanvas
  layout). Not started.
- Graph loading-state UX: parent-dir loading/pulsing while a scope indexes vs
  rendering an incomplete graph as fact; broken links shown distinctly once a
  scope is complete. May need a per-scope index-completeness signal. See
  `../phase-11/graph-loading-state-spec.md`. Not started.
- GI-11: confirmed a STALE-INDEX non-bug (both resolvers already normalize
  `../`). Optional: add `../` / `./` / multi-`../` link-target regression-lock
  tests, else drop.
- (Backlog/idea) media browser: non-editable files (images) stay in the tree;
  a future media browser is the first-class surface for them.

## Drive -> Workspace rename  (-> @@LaneB, SCOPE FIRST)

Rename the `chan-drive` crate to `chan-workspace` and all "drive" terminology
to "workspace" across code, comments, and documentation. @@LaneB is a scoping
architect: produce a scope doc + surface the big decisions to @@Alex BEFORE any
codemod. Known surfaces / decisions to scope (non-exhaustive):
- Crate: `chan-drive` -> `chan-workspace`; `chan_drive::` paths; `Cargo.toml`
  workspace member + all dependents (chan-server, chan, native shells).
- Types/API: `Drive` -> `Workspace`, `Drive::write_text/write_bytes`, the
  registry, `design.md` + `crates/chan-drive/design.md`, CLAUDE.md.
- COLLISION: a "team workspace" concept ALREADY exists (Drafts/ team workspace
  metadata) - disambiguate so drive->workspace does not clash.
- User-facing: CLI subcommands/help, registry on disk, config dir layout,
  error messages ("drive not registered"), the embedded editor copy.
- INFRA (big call): the tunnel domain `drive.chan.app` /
  `{user}.drive.chan.app` is user-facing - does it rename? Coordinate with the
  release/build carryover + a future release lane.
- Native bindings: iOS/Android link `chan-drive` via uniffi - crate rename
  affects them.
- Back-compat: pre-release, so likely a clean break (no migration), but the
  scope MUST state it.
- SEQUENCING: the codemod touches the same frontend/back-end files @@LaneA +
  @@LaneC edit - @@LaneB proposes when the codemod lands (likely a quiescent
  window or last + mechanical); @@Architect serializes. This is the dominant
  phase-12 coordination problem.

## Frontend / cosmetics / keyboard shortcuts  (-> @@LaneC, @@Alex ad-hoc)

@@Alex-driven ad-hoc requests. Standing theme:
- Frontend cosmetics + polish.
- Keyboard shortcuts, and the DIFFERENCES between web, Linux desktop, and macOS
  native desktop client shortcuts (Cmd vs Ctrl, native menu accelerators vs DOM
  handlers, the desktop key-bridge in `desktop/src-tauri/src/serve.rs`, the
  shortcut/chord registry + `web/src/terminal/keymap.ts`).

## Release / build carryover  (no lane yet - awaiting a future @@Alex lane)

From phase-11 LaneC (release contract slices 1-4 landed; 5-6 deferred):
- Slice 5: Tauri desktop updater UX (Check-for-Updates menu, prompt, signed
  payloads) -> `/dl/desktop/latest.json` + the Tauri dep bump (Cargo.lock).
- Slice 6: graph manual/site copy (waits on @@LaneA GI-10 + loading-state).
- First push to origin -> CI shakedown of `make ci-linux`/`ci-macos` +
  release.yml (unrun so far).

## Deferred carryover (unchanged, low priority)

- Linux desktop launch (phase-10 + phase-11 carryover) - @@Alex POSTPONES again
  but explicitly carries to phase 12. Run on a Linux machine (lima + sdme).
- macOS handoff WINDOW-PAINT visual check in a real desktop build (the
  socket->open path is verified via logs; the native window paint is unproven).
- GPU embedding proper fix (`../phase-11/gpu-embed-followup.md`): timeout + CPU
  fallback or correct Metal command-buffer usage (defaulted-off for now).
- Linux inotify watch-count follow-up (with Linux desktop).
- Manual/site streaming-copy update (now mostly unblocked; write against final
  graph behavior once @@LaneA settles).

## Verification gaps to confirm in a real build (from phase-11)

- Editor "Reveal in browser" + search "Show File" -> open a File Browser tab
  (could not be clicked live; thin wrappers).
- Terminal WebGL self-heal: background/foreground, display sleep, monitor
  switch; watch console for `[chan]` context-loss lines.
