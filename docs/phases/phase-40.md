# Phase 40 — v0.50.0: terminal interaction, reload-state, CLI ergonomics, desktop geometry

Round 2026-06-25. Team of 5 on a shared `main` tree, file-locality slicing so no
two lanes edit one source file: @@Lead (rc1 + integrate + gate + docs + build),
@@Terminal (`web/src/components/TerminalTab.svelte`, `web/src/terminal/keymap.ts`,
`crates/chan-library/src/terminal_sessions.rs`, `crates/chan-server/src/routes/
terminal.rs`), @@Shell (`crates/chan-shell/**`, `control_socket.rs`,
`routes/team_config.rs`, team SPA), @@Spa (`web/src/state/{store,tabs}.svelte.ts`,
editor + reload-state surfaces), @@Desktop (`desktop/src-tauri/**`). Execution
round off the three `dev/v0.50.0/planners/design-*.md` docs; cut as a
non-publishing `v0.50.0-rc1` dry-run build for cross-platform smoke.

## Theme

A focused bug-sweep across four surfaces — terminal interaction (copy/paste,
htop-after-reload, the control-terminal banner), CLI ergonomics (`cs terminal
survey --timeout`, `team new --brief`), SPA reload-state (editor caret, headless
theme, pane-size, inspector width, per-Hybrid terminal theme), and desktop window
geometry — plus the rc1 release machinery. The round's real story was, again,
environment: the dev host can run neither a headless browser nor the Tauri/GTK
desktop toolchain, so repro-first verification moved to browser-free proofs and
desktop validation moved to the rc1 CI build.

## What landed (by lane / commit)

- **@@Terminal — terminal interaction** (`cfd6e279` A1, `90c0484e` A2, `52442306`
  A3). A1: hold Shift to force a native xterm selection while a TUI holds mouse
  tracking (the macOS WKWebView `shouldForceSelection` branch ignores Shift; a
  capture-phase bypass restores it) — new `selectionBypass.ts` + a 5-line
  `TerminalTab.svelte` hook. A2: generalize the reattach mode-state from a single
  `in_alt_screen` bool to the full PTY private-mode SET (DECCKM + mouse 1000/1006),
  re-asserted by `send_attach_prelude` on reattach, with the SPA side carried in the
  existing keyboard-protocol hash; this is why htop arrows AND the mouse wheel die
  after a reload (the fresh xterm.js has both off, htop won't re-announce, and chan
  re-asserted only alt-screen). A3: the control-terminal banner prints the bare
  command instead of a `running: ` prefix.
- **@@Shell — CLI ergonomics** (`8fd1a56e`, B1 + B2). B1: `cs terminal survey
  --timeout=<secs>` (default 600), server-side enforcement reusing the `cs pane`
  timeout pattern, a distinct `ControlResponse::Timeout` outcome, a new
  `exit_code` module (`SURVEY_TIMEOUT = 124`, GNU convention), and an elapsed-seconds
  message on stderr (stdout stays clean for capture). B2: `cs terminal team new
  --brief <file>` (and a Cmd+P dialog field) folds a brief verbatim into the
  generated `bootstrap.md` after Roster; content is passed over the wire (the server
  has no client FS), reusing the `config_toml` transport.
- **@@Spa — editor + reload-state** (`875a675f` C1, `70ec754f`+`0a06d461` C2,
  `14c26f17` C3, `8c8623e6` C4, `0fb8bed9` + `5842eae1` C5). C1: files opened without a selection
  land with a usable caret (drop the `!caretPending` guard in `maybeRestoreCaret`,
  default to (0,0) + re-focus on content-land). C2: `systemTheme()` resolves to dark
  when neither prefers-color-scheme query matches (undeterminable / headless),
  matching the matchMedia-absent path and the launcher. C3: pane sizes persist on a
  divider drag including empty panes — the resize `onUp` now schedules a layout save
  (the persistence effect reads only leaf nodes, so a ratio-only change never tripped
  it). C4: File-Browser inspector width persists across reload (the inspector resize
  now schedules the layout save so the per-tab `iw` rides the hash). C5: a per-Hybrid
  terminal light/dark override no longer resets on reload — the real cause is a
  config-write race (PATCH `/api/config` is a whole-block replace; per-persister
  read-modify-write let the terminal-config autosave clobber a just-fired theme
  PATCH), fixed by a shared `updateGlobalConfigSerial` chain. A follow-up
  (`5842eae1`) routed the remaining editor-side writers (`editorTools`,
  `HybridEditorConfig`) through the same chain — extracted to a `configWrite.ts`
  leaf module to break an import cycle — closing the race class.
- **@@Desktop — window geometry** (`1c88c45b`, D1). Restore window size + position
  per machine, per monitor: a sibling label-keyed geometry store (physical px +
  monitor signature, per-signature LRU), captured at all three bury arms and applied
  at the single `build_workspace_window` convergence point, with a size-only-clamped
  fallback on a monitor-signature mismatch. Desktop-local only (no presence, no
  server, no cookie).
- **@@Lead — rc1 + integrate** (`56839154`). Version pins `0.49.0 → 0.50.0-rc1`
  across the 8 release-surface files + both lockfiles; `release.yml` context-job tag
  regex loosened to accept a prerelease suffix so the dry run validates.

## Cross-lane boundaries (brokered by @@Lead)

- **`TerminalTab.svelte` — one editor.** Claimed by both A1 (renderer) and C5
  (reads `surfaceThemeOverride`). Ruled @@Terminal sole editor; C5 landed entirely
  in `store.svelte.ts` + `HybridTerminalConfig.svelte` and never touched it.
- **`Workspace.svelte` — assigned to @@Spa** for the C3 resize-save fix (unowned;
  design-sanctioned "split/resize component").
- **The headless-Chromium repro harness was voided** (unbuildable on this host).
  @@Terminal's A1/A2 and @@Spa's C3/C4/C5 reproduced browser-free instead — see
  Highlights.

## Highlights

- **Repro-first held without the browser.** A2's asymmetry ("htop up/down dead,
  left/right fine") was pinned by THREE independent browser-free observations — an
  ncurses-decode probe (normal-form arrows decode to nothing, all four symmetric),
  a real-htop-3.4.1 mode capture (sets DECCKM + mouse + alt), and the reattach code
  read (re-asserts only alt-screen) — naming the lost modes, with a Rust
  attach/prelude integration test as the fails-before/passes-after proof. A1 was
  pinned at xterm.js v6 source (the macOS `shouldForceSelection` Shift gap). Each
  repro-first item carries a test; the genuinely browser/macOS-only residuals went
  on a host-smoke checklist.
- **C5's real cause was concurrency, not save/restore.** The host called it a
  v0.49.0 regression; a bisect proved `web/src` byte-identical v0.48.0 → HEAD — a
  latent config-write race since the feature shipped. The fix serializes all global
  config writers rather than patching one symptom.
- **The own-gate caught its own gap.** @@Spa's C2 test imported `./store.svelte.ts`
  with an explicit `.ts` extension (svelte-check TS5097, `allowImportingTsExtensions`
  off); @@Shell's web-check surfaced it as a peer breaker, @@Lead routed the one-char
  fix back to @@Spa. Lesson re-ratified: frontend own-gate runs full `make web-check`
  (svelte-check), not just vitest.

## Lowlights + lessons

- **The host has no browser and no GTK/desktop toolchain (no passwordless sudo).**
  Same class as prior rounds. `chan-desktop` cannot compile locally, so D1 committed
  on the CI-validated path (pure geometry logic unit-tested in a standalone crate,
  18/18; `cargo fmt` verified; serve/main compile gate = the rc1 build). The
  integrated `make pre-push` ran desktop-excluded; the rc1 dry-run covers the desktop
  construction sites + macOS sign/notarize.
- **Desktop design anchor was incomplete.** The design anchored geometry capture at
  `capture_window_config`, but the watcher windows (local + devserver — including the
  smoke target) bury and return early, never reaching it. @@Desktop caught this in
  recon and moved to a label-keyed store captured at all three bury arms — read
  before you write.

## Follow-ups (→ next round / @@Alex)

- **`dashboard` vs `infographics` field mismatch** (separate latent bug): frontend
  `HYBRID_SURFACE_KINDS` uses `"dashboard"`; the backend `HybridSurfaceThemes`
  (`crates/chan-server/src/preferences.rs:167`) names it `infographics`, so a
  dashboard surface theme override is dropped on the backend round-trip. Cross-cutting
  with data-migration implications; deferred.
- **Host-smoke checklist** (the browser/macOS residuals the local gate cannot cover):
  A1 macOS WKWebView Shift-drag selection + clipboard write; A2 live htop arrows +
  wheel after a real reload; C3/C4/C5 reload UX; D1 two-client same-screen geometry
  restore incl. the dual-monitor flip.

## The cut

Twelve lane commits on local `main` over the rc1 base (`56839154`), integrated
`make pre-push` from the shared tree (chan-desktop excluded on the bare host;
covered by the rc1 CI build), version bump `0.49.0 → 0.50.0-rc1`, then a
non-publishing `release.yml` dry run (`publish=false`) building all-platform CLI +
desktop artifacts incl. macOS sign/notarize. No `v0.50.0` tag and no published
release this round — the host smoke-tests the artifacts first.
