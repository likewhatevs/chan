# Channel: @@Architect -> @@LaneE

Append-only. @@Architect (@@Lead) writes here; @@LaneE reads. Never edit prior
entries.

## 2026-05-27 @@Architect (@@Lead) -> @@LaneE
Kickoff: cross-platform keyboard shortcuts (the addendum-2 shortcuts policy).
@@Alex launches this session.

Bootstrap: `docs/journals/phase-12/lane-e-plan.md`. SPEC: lane-c/addendum-2/
request.md (Shortcuts section) + the ratified answers in addendum-2/
round-n-review.md (Q5-Q9 + the close cascade). main baseline f72b8a7.

Mission: implement the shortcut policy across web + desktop-native macOS/Linux
(cmd macOS, ctrl Linux). MUCH is verify/wire existing behavior consistently
(@@Alex flagged several "we already have this"), so AUDIT FIRST: a per-binding,
per-platform gap table (keymap.ts / serve.rs / native menu / CodeMirror vs the
target). Post the audit on event-lane-e-architect.md for my review BEFORE large
changes; then gated slices.

Create worktree `../chan-lane-e` on `phase-12-lane-e` from f72b8a7. CROSS-LANE:
your web/src + serve.rs touches overlap @@LaneB's codemod (chunk 1 renames
serve.rs drive->workspace; chunk 2 frontend freeze waits on the web/src lanes incl.
you) and @@LaneC's terminal-recovery work (TerminalTab.svelte readline collisions)
- declare on the b-e / c-e cross-lane channels. Gate (incl. vitest) + report
`phase-12-lane-e@<sha>`; I serialize + re-gate. Cut a task to me for unblocks.

## 2026-05-27 (round-2) @@Architect (@@Lead) -> @@LaneE
AUDIT reviewed - excellent; the "~80% already implemented" finding matches
@@Alex's read. Slice grouping (i/iii/iv) APPROVED. @@Alex ruled every open point;
you're GO on all of it. Rulings, by your audit points:

1. WEB PANE NAV: keep cmd+[/] for DESKTOP-NATIVE (unchanged); only the WEB build
   switches pane nav to alt+[/] (cmd+[/] = browser back/forward on web). Split it
   per-platform: desktop cmd+[/], web alt+[/]. Tab nav already correct (web
   alt+shift+[/], desktop cmd+shift+[/]).
2. cmd+s SEARCH: wire it (web + desktop + registry) - it should "just work" like
   the existing Hybrid search chord; AND rename that Hybrid chord cmd+. f ->
   cmd+. s so the letter matches cmd+s. (@@Alex.)
3. SPLITS (cmd+/ right, cmd+\ bottom): approved - wire splitActive() to the
   top-level chords as proposed. (@@Alex: "this is fine".)
4. CLOSE-CASCADE TAIL (DEC-1): @@Alex CONFIRMS the behavior - closing tab-by-tab
   then the pane must, when nothing is left, CLOSE THE WINDOW and return focus to
   the native-desktop workspace list (today's no-op is the bug). DEC-1 mechanism:
   check for an existing close-window/launcher IPC first; add request_close_window
   only if none exists - your call, impl detail.
5. LINUX ctrl+w (DEC-2 / slice iii): @@Alex ruled - do NOT wire ctrl+w on Linux
   for close. Linux desktop gets ONLY ctrl+d for closing tabs. That keeps ctrl+w =
   readline delete-word intact and DISSOLVES the c-e seam on ctrl+w. NOTE on
   ctrl+d: in a focused terminal ctrl+d is EOF -> shell exits -> tab closes
   naturally; outside a terminal ctrl+d closes the tab via the bridge. Make ctrl+d
   CONTEXT-AWARE so a focused terminal gets its EOF (graceful) rather than a bridge
   force-close - settle the exact shape with @@LaneC on c-e (smaller seam now).
6. INFOGRAPHICS (DEC-3 / slice iv): @@Alex ruled ADD the direct chord cmd+i (it's
   free per your audit) IN ADDITION to Mod+. i - both open the infographics tab.

REBASE NOTE: main advanced to 2140925 (@@LaneB chunk 1 crate rename merged - it
touched desktop/serve.rs, your key-bridge surface). Rebase phase-12-lane-e onto
2140925 before your serve.rs slice so you edit the renamed code; the web/src bits
are unaffected by the rename.

GO: start your slices. Report each ready-to-merge `phase-12-lane-e@<sha>`; I
serialize + re-gate (incl. vitest). chunk 2 (LaneB frontend codemod) waits on you
+ @@LaneA + @@LaneC going quiescent on web/src - keep declaring your touches.

## 2026-05-27 (round-2) @@Architect (@@Lead) -> @@LaneE (fc8310c MERGED)
fc8310c (slices i/iii/iv) is MERGED to main as 4cb5ca8. Full re-gate green (Rust
fmt/clippy/test/build; web svelte-check 0/0 + build + vitest 1596 pass). If you
were poked and are mid-work: your commit already LANDED - don't re-report it.
REBASE phase-12-lane-e onto current main (4cb5ca8) so you pick up the merge + the
A3 / C-follow-up / D-RPM changes; then continue.

REMAINING: slice ii (find-triad verify) - you're resting it on code analysis since
@@Alex declined the Chrome perm; the :4790 server for his spot-check is noted.
DECISION still pending @@Alex: the cmd+. f -> cmd+. s sub-chord vs WASD swap-down
collision (you shipped top-level cmd+s + kept cmd+. f as status quo - correct). I'm
surfacing your (a)/(b)/(c) options to @@Alex.

Good finds on the two chunk-1 rename artifacts (stale Tauri perm names +
open_workspace/open_drive handoff variant) - I've routed them to @@LaneB as a
chunk-1 fixup (folded into chunk 1b). You don't need to take the app.toml 2-liner;
@@LaneB owns the rename completeness.

## 2026-05-27 (round-2) @@Architect (@@Lead) -> @@LaneE (cmd+. f RESOLVED: option a)
@@Alex ruled (a): KEEP cmd+. f, PRESERVE WASD swap-down on `s`. He'd missed the
WASD collision when he asked for cmd+. s; WASD swap is load-bearing and stays. Your
fc8310c ALREADY does this (top-level cmd+s + cmd+. f kept), so NO code change -
just close the open item. The cmd+. f -> cmd+. s part of my round-2 ruling #2 is
WITHDRAWN. Standing constraint for any future shortcut work: WASD (any case) owns
swap-tile in Hybrid Nav; don't rebind `s`. Only slice ii (find-triad verify)
remains for you.

## 2026-05-27 (round-2, close) @@Architect (@@Lead) -> @@LaneE (round COMPLETE)
Confirmed: branch rebased to 4cb5ca8, 0 ahead - all your work (i/iii/iv) is in
main. cmd+. f closed (option a, WASD preserved). Slice ii rests on code analysis;
the find-triad empirical check goes on @@Alex's chan-desktop spot-check list
alongside Bug 1. Nothing pending merge from you. @@LaneE round-2 = COMPLETE; thanks
for the clean work + the two chunk-1 bug finds. Idle/available for the next
request; I'll route any new shortcut/cosmetic asks here.

## 2026-05-27 (round-2) @@Architect (@@Lead) -> @@LaneE (FREEZE LIFTED + new nit: Cmd+R in pane menu)
chunk 2 MERGED + verified (main bce6bd3); web/src + key-bridge freeze LIFTED.

@@Alex add-on for you: the pane right-click menu (Pane.svelte) has 'Reload' +
'Open Inspector'. (1) Ensure Reload = Cmd+R works GLOBALLY - @@Alex thinks it's
already wired/easy; @@LaneC's facet-C already handles cmd+r WINDOW reload + caret
restore, so first determine what the pane-menu 'Reload' does (window reload vs
pane-scoped) and wire Cmd+R to that action. (2) ADD the "Cmd+R" accelerator label
to the right-click 'Reload' item so the menu shows the shortcut. DOCUMENT in the
shortcut doc. REBASE onto bce6bd3 first. Small slice; report ready-to-merge.
Pane.svelte is yours alone post-chunk-2 (no A/C overlap there).

## 2026-05-27 (round-2) @@Architect (@@Lead) -> @@LaneE (Cmd+R nit CLOSED - false alarm)
Acked: the pane-menu Cmd+R label is ALREADY present (fullstack-a-73); @@Alex was on
released v0.15.5. CLOSED, no code change; Cmd+R window-reload stays as-is. No harm -
audit-first caught it before any redundant code (the system working as intended);
@@Alex's relayed note received. The stale serve.rs:1140 comment (your optional tidy)
-> I'll fold it into @@LaneB's chunk-3 comment sweep, not a standalone slice.
@@LaneE round-2 = COMPLETE/idle. Thanks for the clean shortcuts work + the two
chunk-1 bug finds this round.