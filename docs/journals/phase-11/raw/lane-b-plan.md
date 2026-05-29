# Phase 11 Lane B: Editor, desktop, and release weight

```
================================ BOOTSTRAP PROMPT ================================
You are @@LaneB, an architect agent on the `chan` notes app. You are a
peer to @@LaneA and you report to @@Architect (the orchestrator). @@Alex
is the human owner; Alex watches the channels but does not relay messages
between agents, so do not expect copy/paste. You hold the `architect`
skill and you spawn your own `webdev` and `rustacean` subagents to do the
implementation work.

Your lane: discrete editor bugs, the image-drag feature, the desktop
shell (drag removal + download indicator, auto-reload), the binary-size
audit, and the macOS CLI-to-desktop handoff. Scope and order are below.
Two tracks run in parallel: a webdev track and a rustacean track.

DEFERRED, do not start: the Linux desktop launch item. @@Alex will run it
later on a Linux machine; it cannot be validated in this environment.

First actions, in order:
1. Read this whole file, then `CLAUDE.md`, then
   `docs/journals/phase-11/phase-11-round-1.md` and
   `phase-11-round-2.md`, then
   `docs/journals/phase-11/coordination/README.md`.
2. Create your worktree off the current `main`:
   `git worktree add ../chan-lane-b -b phase-11-lane-b`
   Your worktree is for SOURCE CODE only. Read this plan and read/write
   all coordination docs (journals + channels) at the MAIN checkout by
   absolute path: `/Users/fiorix/dev/github.com/fiorix/chan/docs/
   journals/phase-11/`. Do not append to channel files inside your
   worktree copy.
3. Create and open
   `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-11/lane-b/journal.md`;
   log kickoff (baseline commit, plan link, first tasks dispatched).
4. Execute the scope in order, spawning webdev/rustacean subagents.

Coordination (append-only, never rewrite a peer's entries):
- Report progress to @@Architect: append to
  `docs/journals/phase-11/coordination/event-lane-b-architect.md`.
- Read direction from me in `event-architect-lane-b.md`.
- Read cross-lane messages from @@LaneA in `event-lane-a-lane-b.md`;
  send cross-lane to `event-lane-b-lane-a.md`. @@LaneA owns the
  structural shape of the shared files (store.svelte.ts, tabs.svelte.ts,
  lib.rs::router(), state.rs); rebase onto `main` frequently and keep
  your edits to those files minimal.
- ONE @@Alex gate in this lane: the CLI-to-desktop handoff DESIGN NOTE.
  Post it to `event-lane-b-alex.md` and wait for ratification before
  implementing. Everything else is architect-approved (including the
  binary-size decision; record findings in your journal).

Discipline:
- Pre-push gate on every push: `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test`,
  `cargo build --no-default-features`, and in `web/`: `npm run build` +
  svelte-check. CI breaks otherwise.
- Merge to `main` in small frequent slices, each passing the full gate.
- Empirical bug re-walks use a fresh binary: `pkill chan serve`,
  rebuild, verify provenance, restart. Desktop bugs need a fresh
  `chan-desktop` build, not a stale bundle.
- The Linux desktop launch item is DEFERRED (later Linux run); do not
  start it. Everything else in this lane is macOS/cross-platform here.
- Status to @@Architect is curated highlights/lowlights/contention.
=================================================================================
```

## Context

Phase 11 round 1 clears a backlog of editor and desktop bugs, removes
File Browser native drag in/out in favor of Upload/Download buttons, and
shrinks the release binaries. Round 2 carries the macOS CLI-to-desktop
handoff (design-note-first). The Linux desktop launch blocker is deferred
to a later run on a Linux machine and is NOT in this lane's active scope.
Run the quick editor fixes and the desktop track in parallel.

## Scope and execution order

### webdev track (quick wins, then the feature)
1. **Bug 1: list input regression.** Typing `- `, `* `, or `1. ` at the
   start of an empty line must create the matching list type (bullet for
   `-`/`*`, numbered for `N. `), not always a bullet. Regular editor
   behavior; this is a regression, not a new design. File:
   `web/src/editor/commands/list.ts` (`parseListPrefix` and friends).
2. **Bug 4: New File/Dir trailing slash.** A path ending in `/` should be
   accepted as a directory (the dialog caption invites it), not rejected
   with "path ends with /, type a name". Files:
   `web/src/state/pathValidate.ts` (the reject branch),
   `web/src/components/PathPromptModal.svelte`.
3. **Bug 5: image paste lands at row 1.** A pasted image must land at the
   cursor, not the first row. The drop handler already computes the
   position via `posFromEvent`; the paste handler should match. File:
   `web/src/editor/bubbles/image_drop.ts`.
4. **Bug 10: Cmd+N cursor.** After Cmd+N opens a new draft, place the
   cursor in the document so the user can type immediately. File:
   `web/src/App.svelte` (the Cmd+N handler). Keep this edit minimal;
   App.svelte is a shared merge point with @@LaneA.
5. **Feature: image drag across rows.** Let the user drag an image to a
   different row to move where its markdown lands; left/center/right then
   come from the existing image dropdown on that row. Same file as bug 5
   (`image_drop.ts`); do it after bug 5 so they share one mental model.
6. **Bug 6: idle terminal garbled until clicked/resized.** Fix the
   FitAddon timing so an idle pane renders correctly while another pane
   is active. File: `web/src/components/TerminalTab.svelte` (the
   resize-observer / trailing-fit scheduler).

### rustacean track (desktop, release, handoff)
7. **Bug 2: remove drag in/out + native download indicator.** Remove File
   Browser native drag in and out entirely on macOS and Linux (the macOS
   drag-out crashes; Linux/Windows are already no-ops). Operate via the
   Upload/Download buttons. Add a download progress indicator in the
   native desktop to mimic what browser users get. Files:
   `desktop/src-tauri/src/drag_out.rs` (delete the macOS path), the JS
   drag wiring in `web/src/components/FileTree.svelte`, and the inspector
   download UI in `web/src/components/FileInfoBody.svelte` /
   `web/src/api/client.ts` (`uploadAttachment`, `downloadUrl`).
8. **Bug 8: native desktop auto-reload + hang.** The desktop app
   auto-reloads during editing and hangs on "loading..." (cmd+ resolves
   it). Investigate the reload/watch path: `desktop/src-tauri/src/
   watcher.rs` (config-dir watch), `main.rs` (`reload_window`),
   `embedded.rs` (embedded server start).
9. **Linux desktop launch (round-2 blocker). DEFERRED, do not start.**
   @@Alex runs this later on a Linux machine; it cannot be validated in
   this environment. Recorded here for continuity only: shell launch,
   first-window routing (white window), missing File menu, blank
   duplicate window from Window->Drives. Files for the future run:
   `desktop/src-tauri/src/main.rs` (`install_app_menu`, window setup),
   `serve.rs`, `embedded.rs`; repro on lima-vm + sdme (aarch64).
10. **Bug 3: binary-size audit.** Confirm no BGE embedding model is baked
    into any release binary; ship the smallest single binaries (SPA only
    for chan / chan-desktop). Note: `embed-model` and `embed-font` are
    OFF by default and chan-desktop builds with `embeddings` (runtime
    resolver, nothing baked in); the one lever to verify/decide is the
    chan CLI release path (`make build-release` historically used
    `--features embed-model`). Audit the actual sizes across releases,
    record findings + a chan-CLI recommendation in your journal, then
    apply (architect-approved, no @@Alex gate). Touching
    `.github/workflows/` is authorized for this task; state that inline
    before the first workflow edit. Files: `crates/chan-server/Cargo.toml`,
    `crates/chan/Cargo.toml`, `crates/fetch-models`,
    `.github/workflows/release-desktop.yml`, the `Makefile` build targets.
11. **macOS CLI-to-desktop handoff (design note -> @@Alex gate ->
    implement).** Scoped to macOS (Linux blocked behind item 9, Windows
    deferred). Write a short design note: should `chan serve <path>`
    attach to a running desktop-owned server, ask desktop to open the
    drive, or keep owning its own server; how same-user desktop discovery
    works (Unix-domain socket); how ownership, bearer-token discovery,
    lifecycle, version, and capability mismatch are represented; the
    no-desktop-running fallback; which flags force standalone behavior.
    Give one recommendation. `crates/chan-server/src/mcp_bridge.rs`
    already uses a per-pid UDS and is the reuse pattern. Post the note to
    `event-lane-b-alex.md` and WAIT for ratification before implementing.
    Files for context: `crates/chan/src/main.rs` (`cmd_serve`),
    `mcp_bridge.rs`, `desktop/src-tauri/src/main.rs`
    (`run_hidden_mcp_proxy_if_requested`).

Sequence: run the webdev quick wins and the rustacean desktop items in
parallel. Do the CLI-handoff design note after the desktop bugs land,
since the handoff design depends on the launch/ownership model. (Linux
launch is deferred and not a prerequisite here.)

## Conflict surface (what you own, what you share)

You OWN: `web/src/editor/*`, `web/src/state/pathValidate.ts`,
`web/src/components/{PathPromptModal,TerminalTab,FileInfoBody}.svelte`,
the terminal modules, and the whole desktop shell
(`desktop/src-tauri/*`), the release/build config, and the CLI-handoff
surface.

You SHARE (@@LaneA owns the structural shape; rebase onto `main` often,
keep your edits minimal): `web/src/state/store.svelte.ts`,
`web/src/state/tabs.svelte.ts`, `web/src/App.svelte` (your Cmd+N fix),
`web/src/api/client.ts`. Announce any edit to these on
`event-lane-b-lane-a.md`.

Integration seam: when @@LaneA's bootstrap/init slice merges to `main`,
rebase and re-validate the macOS desktop launch against the new
embedded-server init path before calling it done. (Linux re-validation
folds into the deferred Linux run.)

## Verification
- Rust gate: `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test`,
  `cargo build --no-default-features`.
- Web gate: `npm run build` + svelte-check in `web/`.
- Editor bugs: re-walk each on a fresh binary in the browser.
- Desktop bugs: fresh `chan-desktop` build on macOS for drag/reload.
  (Linux launch validation is deferred to the later Linux run.)
- Binary size: compare release artifact sizes before/after; assert no
  models.tar.zst in the shipped binary.

Line numbers in the round-1 journal and the @@Architect findings are
approximate; verify against HEAD before editing.
