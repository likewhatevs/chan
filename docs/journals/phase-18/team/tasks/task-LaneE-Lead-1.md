# task LaneE -> Lead (1): Terminal + chan-desktop round-1 wave-1 DONE

All 4 wave-1 items landed. Scoped own-gate green. Awaiting your go on the
shortcuts.ts append (C-then-E). Details below.

## Per-item status

1. **Hide rich prompt -> focus terminal** DONE. One reactive
   transition-watcher `$effect` in TerminalTab.svelte (keyed on
   `isRichPromptVisible`, plain `richPromptWasVisible` tracker) refocuses
   xterm on the show->hide edge for the active+focused terminal. Covers
   ALL THREE hide paths uniformly (tab-menu Hide, Cmd+Shift+P, the
   bubble's own Escape) instead of patching each call site.
   Empirical: WKWebView-visual focus -> needs @@Alex hand-smoke.

2. **Terminal copy/paste chords** DONE (code) / shortcuts.ts WAITING ON
   YOU. Wired `handleTerminalClipboardChord` at the top of
   `handleTerminalKeyEvent` so xterm skips the key (no stray bytes, no
   SIGINT, no double-paste). OS-divergent: macOS Cmd+C/Cmd+V, else
   Ctrl+Shift+C/V (bare Ctrl+C/V stays SIGINT; `Mod+C` is unusable here).
   Keyboard copy = selection only (menu Copy keeps selection-or-scrollback).
   Menu hint spans now read `chordFor("terminal.copy")`/`("terminal.paste")`
   (empty until the registry entries land - matches current state).
   Empirical: WKWebView clipboard -> needs @@Alex hand-smoke.

3. **UTF-8 garble in less AND vim** DONE. terminal_sessions.rs
   `Session::spawn`: when the inherited/requested env does not already
   select a UTF-8 codeset (LC_ALL > LC_CTYPE > LANG precedence), set
   `LANG=C.UTF-8` + drop non-UTF-8 LC_ALL/LC_CTYPE. New `locale_selects_utf8`
   helper + unit test. Empirically validated the mechanism (zero
   contention): empty locale -> `US-ASCII` codeset (the garble), `C.UTF-8`
   -> `UTF-8` codeset (the fix); em dash = `e2 80 94` = image-14's
   `<E2><80><94>`. Live less/vim smoke is Chrome-drivable (backend fix, NOT
   WKWebView) -> Wave-2.

4. **chan-desktop pre-flight removal** DONE (incl. a flagged scope
   expansion). main.js renderLocal no longer scans / shows the bge+reports
   toggles / explanatory copy; the Local choice now just confirms the path
   and calls `add_workspace` with no features arg (SPA's onboarding card
   enables layers post-boot). Removed dead JS helpers + orphaned CSS.
   Empirical: WKWebView double-dialog -> needs @@Alex hand-smoke (this is
   the image-1/image-2 fix).

## FLAG: scope expansion on item 4 (your call to keep or revert)

@@Alex's spec says "we should NOT have any pre-flight in the chan-desktop
app anymore". The desktop SOURCE-PIN TESTS in serve.rs forced a test
rewrite regardless, and keeping a dead filesystem-scanning IPC would
contradict the spec, so I also removed the **dead Rust backend**:
- `compute_workspace_preflight` IPC + `PreflightReport` struct + 6
  walk/classify/scm helpers + their 4 unit tests + the generate_handler!
  registration (main.rs).
- Rewrote the 4 old serve.rs source-pin tests + the add-time-toggle test
  into ONE `new_workspace_local_choice_has_no_desktop_preflight` test
  pinning the new contract (modal registers via add_workspace w/o features;
  no scan IPC / renderer / toggles / explanatory copy survive in main.js;
  the Rust IPC is gone).
Self-contained in the desktop crate. If you'd rather I'd left the dead
backend, say so and I'll restore just the Rust side + revert the test to a
keep-the-command shape.

## FLAG: desktop `make build` deferred

Scoped desktop gate is green (cargo test/clippy/check + fmt -p
chan-desktop). I did NOT run `cd desktop && make build`: a release tauri
DMG build holds the shared cargo target lock ~15 min and would stall the
peer lanes' cargo (the desktop crate is already validated in dev mode).
Recommend it runs in your isolated full-tree gate.sh worktree (own target
dir, immune to peers).

## SHORTCUTS append - LANDED (your release poke received)

Per your "shortcuts.ts RELEASE" poke (C landed app.files.delete / "File"
group), I appended my Terminal block. Done:
- Added `"Terminal"` to the `ShortcutGroup` union (after `"Find"`).
- Appended `terminal.copy` + `terminal.paste` at the END of `SHORTCUTS`
  (after the Editor block, a distinct trailing Terminal group;
  unambiguously after C's mid-array entry).
  Chords: web/native `Cmd+C` / `Cmd+V`, group `"Terminal"`, note
  "Ctrl+Shift+C/V on Linux / Windows", no `escapeTerminal`.
- Did NOT run `web/scripts/shortcuts-table.mjs` - the main.rs resync is
  yours, ONCE, after both C and E land (this is the only step left for the
  Terminal/File chords).
- Re-ran the FULL frontend gate after the append: svelte-check 0 errors
  (1 pre-existing a11y WARNING in RichPrompt.svelte, not mine), vitest
  1685 passed (incl. shortcuts.test.ts + chordEscapeRegistry.test.ts),
  vite build green.

Caveat (your call): stored as literal `Cmd+` (correct on macOS / the
image / @@Alex's machine). On Linux/Windows the DISPLAYED hint reads
"Cmd+..." while the handler correctly uses Ctrl+Shift+...; the `note`
documents it. Fully-correct cross-platform display needs an `osChord`
special-case (like reload's Ctrl+Shift+R) - say the word and I'll add it.

## Own-gate (ran on the live shared tree: my files + peers' WIP)

- cargo fmt --check: clean
- cargo clippy -p chan-server --all-targets -D warnings: clean
- cargo clippy -p chan-desktop --all-targets -D warnings: clean
- cargo test -p chan-server (locale): pass
- cargo test -p chan-desktop: 74 + 7 pass
- make web-check (svelte-check + vitest + vite build): green, RE-RUN after
  the shortcuts.ts append (1685 vitest pass).

## Pathspec (base HEAD d5f7dd38; NOT committed - Wave-3 yours)

My 6 single-lane files (commit race-safe with the explicit pathspec):
```
crates/chan-server/src/terminal_sessions.rs
desktop/src-tauri/src/main.rs
desktop/src-tauri/src/serve.rs
desktop/src/main.js
desktop/src/styles.css
web/src/components/TerminalTab.svelte
```

SHARED file you merge: `web/src/state/shortcuts.ts` now carries BOTH
@@LaneC's `app.files.delete` (mid-array) AND my `terminal.copy` /
`terminal.paste` (end) + the `"Terminal"` union member. You commit the
merged file + run the single `shortcuts-table.mjs` resync to
crates/chan/src/main.rs.

NOT mine: web/src/App.svelte (+ everything else in `git diff --stat` is
peers' WIP).
