# journal - @@LaneE (Terminal + chan-desktop)

Append-only running log. Round-1, Wave-1.

## 2026-06-04 - round-1 wave-1: Terminal + chan-desktop (4 items)

Spawned, self-identified `@@LaneE` from `$CHAN_TAB_NAME`, read
bootstrap.md + round-1-plan.md + draft.md + the cited images. Task:
task-Lead-LaneE-1.md.

### Item 3 - UTF-8 garble in less AND vim (terminal_sessions.rs)

Root cause confirmed: image-14/15 show `<E2><80><94>` (an em dash,
UTF-8 bytes `e2 80 94`) rendered as raw bytes. GUI-launched servers
(notably chan-desktop on macOS) inherit an EMPTY locale, so `less`/`vim`
fall back to the POSIX/C codeset and byte-render multibyte UTF-8.

Fix: in `Session::spawn` PTY env, when the inherited/requested env does
not already select a UTF-8 codeset (LC_ALL > LC_CTYPE > LANG precedence),
set `LANG=C.UTF-8` and drop any non-UTF-8 `LC_ALL`/`LC_CTYPE` so LANG
actually controls the codeset (and the user's shell profile can still
re-export LANG). Added `locale_selects_utf8()` helper + a deterministic
unit test (`locale_selects_utf8_honors_lc_all_codeset`).

`C.UTF-8` chosen: present on macOS (verified `locale -a`), every musl
Linux build, glibc >= 2.35 / Debian / Ubuntu / RHEL 8+.

Empirical mechanism check (zero-contention, no server):
- `env -u LANG -u LC_ALL -u LC_CTYPE locale charmap` -> `US-ASCII` (the
  garble case)
- `env LANG=C.UTF-8 ... locale charmap` -> `UTF-8` (the fix)
- em dash confirmed `e2 80 94`, matching image-14's `<E2><80><94>`.
Live less/vim-in-terminal smoke is Wave-2 (backend env fix, so it is
Chrome-drivable; NOT WKWebView-specific).

### Item 1 - hide rich prompt returns focus to terminal (TerminalTab.svelte)

The bubble hides via THREE paths (tab-menu Hide, Cmd+Shift+P, the
bubble's own Escape). The existing focus-pulse `$effect` only re-runs on
a tab switch, so it never observes a same-tab show->hide flip. Added a
dedicated transition-watcher `$effect` keyed on `isRichPromptVisible`
with a plain (non-reactive) `richPromptWasVisible` tracker: on the
show->hide edge, for the active+focused terminal, refocus xterm in a
microtask. One reactive watcher covers all three hide paths.

### Item 2 - terminal copy/paste chords (TerminalTab.svelte + shortcuts.ts)

Copy/Paste/Copy-Scrollback handlers already existed + were wired to the
context-menu buttons; the gap was (a) no keyboard chords, (b) empty chord
hint spans.
- Added `handleTerminalClipboardChord` resolved at the TOP of
  `handleTerminalKeyEvent` (xterm custom handler, runs before xterm
  processes the key): returns false so xterm skips the key -> no stray
  bytes, no SIGINT on Ctrl+Shift+C, no double-paste from xterm's native
  paste. `preventDefault` also suppresses the browser default.
- OS-divergent chords: macOS Cmd+C/Cmd+V (Cmd never collides with a
  control code); every other platform Ctrl+Shift+C/V (bare Ctrl+C/V stays
  the shell's SIGINT). `Mod+C` is unusable here (Mod -> Ctrl on Linux).
- Keyboard copy copies the SELECTION ONLY (no scrollback dump); the menu
  "Copy" keeps selection-or-scrollback.
- Menu hint spans now read `chordFor("terminal.copy")` / `("terminal.paste")`
  (empty until the registry entries land - same as the current state).
- shortcuts.ts: NOT TOUCHED. Per task, staged the entries + WAIT for
  @@Lead's go (C-then-E sequencing). Entries listed in the completion
  task. @@LaneC's FB chords are already in the working-tree shortcuts.ts
  (+15, not mine).

### Item 4 - remove desktop pre-flight dialog (main.js + scope expansion)

@@Alex's spec: "we should NOT have any pre-flight in the chan-desktop app
anymore since this have moved over to chan's SPA" (PreflightOverlay.svelte,
phase-17). The SPA already owns BOTH the locked boot surface AND the
post-boot onboarding card that enables Semantic/Reports.
- `desktop/src/main.js renderLocal`: dropped the scan + the bge/reports
  toggles + the baseline/layered/footer explanatory copy. The folder-chosen
  step is now just "confirm path + Back/Open"; Open calls
  `add_workspace` WITHOUT a features arg (defaults both layers off; SPA's
  onboarding card enables them post-boot).
- Removed the now-dead JS helpers `renderPreflightReport` /
  `appendPreflightRow` / `formatPreflightBytes`.
- styles.css: removed the orphaned `.preflight-baseline/-layered/-footer`,
  `.preflight-toggle*`, `.preflight-report*`, `.preflight-warn` rules +
  the stale `compute_workspace_preflight` comment; updated the
  `.preflight-overlay` comment. Kept `.preflight-overlay/-dialog/-intro/
  -path/-buttons` (still used by the default-workspace chooser).
- SCOPE EXPANSION (flagged to @@Lead): the desktop source-pin tests in
  serve.rs forced a test rewrite, and @@Alex's spec says "no pre-flight in
  chan-desktop at all", so I also removed the dead Rust backend:
  `compute_workspace_preflight` IPC + `PreflightReport` struct + 6 walk/
  classify/scm helpers + their 4 unit tests + the generate_handler!
  registration. Rewrote the 4 serve.rs source-pin tests (+ the add-time-
  toggle test) into ONE `new_workspace_local_choice_has_no_desktop_preflight`
  test that pins the new no-desktop-preflight contract.

### Own-gate (ran on the live shared tree = my 6 files + peers' WIP)
- cargo fmt --check: clean
- cargo clippy -p chan-server --all-targets -D warnings: clean
- cargo clippy -p chan-desktop --all-targets -D warnings: clean
- cargo test -p chan-server (locale test): pass
- cargo test -p chan-desktop: 74 + 7 pass (incl. the rewritten pin test)
- make web-check (svelte-check + vitest + vite build): green
- desktop `make build` (release DMG bundle): DEFERRED to @@Lead's isolated
  full-tree gate (a release tauri build holds the shared cargo target lock
  ~15 min and would stall peer lanes' cargo; desktop crate already
  validated via dev test/clippy/check). Dialog removal is WKWebView-visual
  -> @@Alex hand-smoke.

Pathspec (my 6 files): crates/chan-server/src/terminal_sessions.rs,
desktop/src-tauri/src/main.rs, desktop/src-tauri/src/serve.rs,
desktop/src/main.js, desktop/src/styles.css,
web/src/components/TerminalTab.svelte. Base HEAD d5f7dd38. NOT committed
(Wave-3 @@Lead commits). NOT touched: shortcuts.ts (gated), App.svelte.

Cut completion task task-LaneE-Lead-1.md, poked @@Lead.

## 2026-06-04 - shortcuts.ts append (after @@Lead's C-then-E release)

@@Lead poked "shortcuts.ts RELEASE" (C landed app.files.delete / "File"
group). Appended my Terminal block: added `"Terminal"` to the
`ShortcutGroup` union (after `"Find"`) + `terminal.copy` / `terminal.paste`
at the END of `SHORTCUTS` (distinct trailing group, after C's mid-array
entry). Cmd+C/Cmd+V (web+native), note documents the Ctrl+Shift+ Linux
divergence. Did NOT run shortcuts-table.mjs (@@Lead's single resync).
Re-ran full web-check after the append: svelte-check 0 errors (1
pre-existing a11y warning in RichPrompt.svelte, not mine), 1685 vitest
pass, build green. The menu chord hints (chordFor terminal.copy/paste)
now resolve to Cmd+C / Cmd+V. Updated task-LaneE-Lead-1.md, re-poked
@@Lead.

## 2026-06-04 - round-2 (task-Lead-LaneE-2): caller-free + osChord

@@Lead ratified the Rust removal, accepted the make-build deferral, asked
for (a) a one-line caller-free confirm and (b) the osChord cross-platform
fix.

Caller-free: grepped the whole repo. compute_workspace_preflight is
caller-free EXCEPT two stale TRACKED references the cargo gate didn't flag
(Tauri perms are build/runtime-validated): `permissions/app.toml` (the
main-window set listed + DEFINED `allow-compute-workspace-preflight`) -
removed both; and `gen/schemas/acl-manifests.json` (gitignored artifact) -
rebuilt to regenerate it clean. This is the "gate-blind wire rename" class
- glad @@Lead asked for the grep. cargo test -p chan-desktop still 74+7.

osChord: added TERMINAL_COPY_ID/TERMINAL_PASTE_ID + two `os !== "mac"`
lines (Mod+Shift+C/V) following the reload pattern; KEPT the reload line
verbatim so cmdRWindowReload's source-pin stays green. mac shows Cmd+C/V,
Linux/Windows show Ctrl+Shift+C/V (display now matches the handler). Added
terminalCopyPasteChords.test.ts mirroring the reload test. make web-check
green (1692 vitest pass). Did NOT run shortcuts-table.mjs (@@Lead's single
resync - I told them they're clear). Cut task-LaneE-Lead-2.md, poked @@Lead.

## 2026-06-04 - round-1 CLOSED for @@LaneE (task-Lead-LaneE-3)

@@Lead accepted round-2 + ran the resync themselves: crates/chan/src/main.rs
SERVE_LONG_ABOUT now carries the File + Terminal groups (and incidentally
corrected a pre-existing Dashboard `Cmd+. i` -> `Alt+Shift+D` drift). main.rs
is @@Lead's commit artifact - I do NOT touch it or re-run the script.

My Wave-3 commit pathspec (per @@Lead): the task-1 six files +
desktop/src-tauri/permissions/app.toml + web/src/components/
terminalCopyPasteChords.test.ts. shortcuts.ts is shared (@@Lead merges);
gen/ not committed. Status: DONE for round-1, no action pending.

STANDING BY for Wave-2:
- @@Lead drives the UTF-8 less/vim smoke on the Chrome convergence server
  (item 3, backend env fix, Chrome-drivable).
- @@Alex hand-smokes the WKWebView-only items: rich-prompt hide->focus,
  terminal clipboard copy/paste, desktop double-dialog removal (image-1/2).
Will pick back up only if a Wave-2 smoke surfaces a bug in my lane.

## 2026-06-04 - RECYCLED as the RELEASE lane (own the release with @@Alex)

@@Lead handed off (RELEASE-HANDOFF.md): @@LaneE is now the single release
lane; A/B/C/D/F + @@Lead cleared out. Accepted ("release handoff accepted").
Two jobs + own the release end-to-end.

Progress this session:
1. Job 1 - desktop OFF/ON toggle race (committed 20526d0c, gated green,
   pending @@Alex WKWebView hand-smoke). Two-part fix:
   - main.js: disable the toggle for the whole start/stop transition (no
     mid-flight re-click) + force-refresh to reconcile the toggle + Open to
     the true serve state on every outcome (bypassing the list-JSON dedupe).
   - embedded.rs: retry open_workspace on WorkspaceAlreadyOpen/Locked
     (8x150ms) so a background-held just-closed flock no longer fails a
     legitimate OFF->ON; mirrors unregister_with_retry on the close side.
   Root cause + analysis captured in desktop-off-toggle-bug.md.
2. @@Alex ask: moved the bug-report details into phase-18 docs
   (desktop-off-toggle-bug.md) + deleted the untracked desktop-bug-report/
   draft dir ("don't leave drafts behind").
3. Version bump 0.25.0 -> 0.26.0 (committed ea1fca51): workspace.package +
   the 8 internal workspace.dependencies pins, tauri.conf.json,
   web/package.json, root Cargo.lock, AND gateway/Cargo.toml +
   gateway/Cargo.lock (gateway versions in lockstep per its manifest note).
   Both lockfiles regenerated via cargo check (root + gateway green).
4. Full release gate launched (scripts/release-gate-0.26.0.sh, background):
   make pre-push (fmt/clippy/test --all-targets / --no-default-features /
   gateway-build / web-check / web-marketing-check) + desktop DMG. Gating
   the committed clean main tree (no WIP).

Remaining (held for @@Alex coordination): gate result review -> @@Alex
hand-smoke of the DMG (Job 1 + YET-TO-CHECK WKWebView items) -> fold
phase-18 + ordered deletions (final close-out) -> dry-run
(workflow_dispatch publish=false) -> tag ONLY on @@Alex's explicit go.

GATE GREEN (scripts/release-gate-0.26.0.sh, on the committed clean main):
- make pre-push exit 0: fmt + clippy --all-targets -D warnings + test
  --all-targets + build --no-default-features + gateway-build + web-check
  + web-marketing-check ALL pass.
- desktop DMG exit 0: target/release/bundle/dmg/Chan_0.26.0_aarch64.dmg
  (14M), codesigned (Developer ID Alexandre Fiori; codesign --verify
  "valid on disk" + "satisfies its Designated Requirement"); notarization
  correctly skipped (no APPLE_* env; that runs on Actions). Version 0.26.0
  propagated to the artifact name. (chan-desktop is a root workspace member
  -> bundles land in the shared root target/, not desktop/src-tauri/target/.)
Handed the DMG to @@Alex for the 5 YET-TO-CHECK hand-smoke items.

## 2026-06-04 - @@Alex hand-smoke round (release-lane fixes, all gated green)

@@Alex hand-smoked the rebuilt binary + reported a stream of finds; fixed
each (committed to main, gated via web-check + cargo check, re-baked into
target/release/chan-desktop each batch). Used 3 parallel research subagents
(@@Alex OK'd subagents) to root-cause the deeper ones; I implemented +
built + committed.
- fix(editor) 7a114943: list alignment guides pinned to a fixed x (the
  per-depth margin-left was dragging the guide ::before; compensated `left`
  so every line's leftmost bar anchors at content-left + 10px). Verified
  in-browser (guideX constant 237.13 across depths 0-3).
- fix(inspector) 9fb3ec4c: Drafts NODE -> single Terminal-from-here button.
- fix(inspector) 625debf5: draft FILE inspector was blank (draft files live
  outside the workspace tree + /api/inspector's classify_path errored).
  Server resolves drafts via resolve_physical_path + classify_abs + a new
  optional InspectorPayload.abs_path; FileInfoBody synthesizes the draft
  entry from the payload + a single Terminal-from-here seeded with
  {cursor}{space}{abs-path}.
- feat(dashboard) ca8d1ea1: search-index root anchored near the bottom
  (above the carousel scroller) via a prop-gated focalAnchor="bottom" in
  GraphCanvas.computeFit (Graph tab + FS graph unchanged).
- Bug 2 (graph file->parent-dir edges): subagent verified NOT reproducible
  on current code (server payload + client filter + live render all show the
  parent-dir edge; the pink {} is the language node, not a scope-hub). Like
  the dashboard #14, the flat/old screenshots were @@Alex's older team
  environment. Pending @@Alex's re-check on the new binary; no code change
  unless it reproduces there.
@@Alex confirmed the fixes ("yes!!!"). Round-1 + hand-smoke fixes all on
main, NOT pushed. Remaining: phase-18 fold + ordered deletions + dry-run +
tag, all gated on @@Alex's explicit go.

## 2026-06-04 - v0.26.0 SHIPPED

Close-out + release cut:
- Folded the round into docs/phases/phase-18.md (subagent-distilled from the
  journals; I finalized + added the release-wave + retrospective) + README
  index entry (a67b21f1).
- @@Alex chose to KEEP the phase-18 bus around for now -> all docs-cleanup
  deletions (.claude/.codex/docs/archive/cut cards/bootstrap/docs/journals)
  are HELD, not done. They do not affect the release build.
- Final full gate (scripts/release-gate-0.26.0.sh) initially RED: the Bug 1
  inspector commit (625debf5) had an un-rustfmt'd block (cargo-checked but
  not fmt'd); the full pre-push fmt --check caught it. Fixed (64025a01).
  Re-gate GREEN: fmt + clippy --all-targets + test --all-targets (all 0
  failed) + --no-default-features + gateway-build + web-check +
  web-marketing-check + signed DMG.
- @@Alex gave the explicit go. Pushed main (foreground gated, ls-remote
  verified, no SIGPIPE). Dry-ran release.yml publish=false (run 26969540074)
  GREEN incl the macOS sign/notarize. Cut v0.26.0 (annotated tag on
  64025a01), pushed it (foreground gated, verified). The tag fired the real
  release.yml (run 26972170443) -> completed/success: GitHub Release
  published (not draft) with the full asset set (signed Chan_0.26.0.dmg,
  .app.tar.gz + updater .sig, AppImages, debs, rpms, static musl Linux
  binaries, gateway debs, manual tarball, CLI tarballs), /dl metadata built
  + Pages deployed.
- Known external follow-up (NOT a release failure): chan.app -> Pages
  routing for /dl is unwired, so desktop self-upgrade 404s until that's
  fixed (chan-prod-setup owns it); manual DMG install works.
v0.26.0 is live.

