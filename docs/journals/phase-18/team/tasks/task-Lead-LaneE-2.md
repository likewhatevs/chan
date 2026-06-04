# task Lead -> LaneE (2): Terminal+desktop ACCEPTED + 2 flags resolved + 1 add

All 4 items accepted, own-gate green, shortcuts append landed correctly (after C,
distinct trailing "Terminal" group). Pathspec noted (6 single-lane files +
shortcuts.ts shared - I merge + resync). Strong empirical work on the UTF-8
locale (em dash e2 80 94 == image-14). Resolutions:

## FLAG 1 (dead Rust pre-flight backend removal) -> RATIFIED, keep it
Keep the full removal (compute_workspace_preflight IPC + PreflightReport + the 6
helpers + 4 tests + generate_handler! registration + the serve.rs test rewrite).
Reasons: @@Alex's spec = "NOT have any pre-flight in the chan-desktop app
anymore"; pre-release = drop dead code, no back-compat; it's self-contained in
the desktop crate; you rewrote the source-pins into a contract test
(new_workspace_local_choice_has_no_desktop_preflight). Leaving a dead scanning
IPC would contradict the spec. Correct call.
- ONE-LINE CONFIRM back: that compute_workspace_preflight had NO remaining caller
  outside the removed local renderLocal path (i.e. no cloud/remote/tunnel
  workspace flow still invoked it). You characterized it as dead; just confirm
  the grep so the removal is provably caller-free.

## FLAG 2 (desktop `make build` deferred) -> ACCEPTED
Correct - a release tauri DMG build holds the shared cargo target lock ~15 min
and would stall peers. That full build is mine to run in the isolated gate.sh
worktree (own target dir) at Wave-3 / pre-tag. Your scoped dev-mode desktop gate
(fmt + clippy + test/check -p chan-desktop) is sufficient for your own-gate.

## CAVEAT (cross-platform shortcut display) -> ADD the osChord special-case
Add it. @@Alex's spec explicitly requires the chords "ported to linux and macos
and web", and a hint that DISPLAYS "Cmd+..." on Linux/Windows while the handler
uses Ctrl+Shift+... is a known display bug - the round's quality bar is "no known
bug shipped". Apply the existing reload Ctrl+Shift+R `osChord` pattern to
terminal.copy / terminal.paste so the displayed hint is correct per-OS while the
handler stays as-is. In your lane (shortcuts.ts). Re-run make web-check after.
- I am HOLDING the single `node web/scripts/shortcuts-table.mjs` resync to
  crates/chan/src/main.rs until your osChord lands - so I resync ONCE, final,
  with C's + your + the osChord entries all present. Poke me when osChord is in
  + web-check green; that's the last shortcuts step.

## WKWebView hand-smoke (items 1, 2, 4) -> on @@Alex's hand-smoke list
Rich-prompt hide->focus, clipboard copy/paste, and the desktop double-dialog
(image-1/2) can't be driven by Chrome automation; I'm batching them for @@Alex's
hand pass. Item 3 (UTF-8 less/vim) IS Chrome-drivable -> I smoke it on the Wave-2
server.

Nothing else pending; just the osChord add + the one-line caller-free confirm.
