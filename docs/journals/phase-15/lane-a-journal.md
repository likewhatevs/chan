# Lane A journal — phase-15

## Setup
- Throwaway drive: `/tmp/chan-test-flip` (welcome/ideas + projects/chan,notes;
  cross-links for the graph). Server: renamed binary `/tmp/lane-a-chan`,
  `serve --port 8810 --no-token --standalone --no-browser`, log at
  `/tmp/lane-a-server.log`. Pkills scoped to that path/port only.
- Decisions locked by @@Alex (see lane-a-tasks.md): metadata->Workspace back;
  slide-3 relabel Indexing->Search; slot picker = keep dots + add labels.

## A1 — true two-face card flip

### Current mechanism (to remove)
- `paneFlip` version bus (`tabs.svelte.ts:758-771`) bumped by `flipHybrid`
  (`:3007`), subscribed in `Pane.svelte:405-420` -> `flipActive` rAF
  double-tap -> `class:flipping` (`:935`) -> `@keyframes pane-flip-once`
  half-flip on the WHOLE `.pane` (`:1444-1457`). Content is swapped, not
  co-rendered, via `{#if pane.showingBack}` (`:1247`). The rAF races the
  content-swap teardown + focus blur -> keyframe only fires once focus
  leaves (BUG-1).

### New mechanism
- Co-render front + back in a `.flip-card` (`transform-style: preserve-3d`)
  INSIDE `.editor-wrap` (tab strip chrome stays put, does NOT rotate).
  `.editor-wrap` gets `perspective`. Card `transform: rotateY(0|180deg)` via
  a CSS transition driven by a derived `flipped = showingBack &&
  !paneMode.active`. No keyframe, no rAF, no bus.
- `.face.front` (`position:absolute; inset:0; flex column`) replicates the
  old `.editor-wrap` layout: holds the front `{#if}` content chain
  (pane-mode-preview as its first arm) AND the always-mounted terminals
  (`.terminal-tab` is `absolute inset:0`, so they fill the face).
- `.face.back` (`rotateY(180deg)`, `backface-visibility:hidden`) holds the
  `.back-side` config bodies, still gated `{#if !paneMode.active}`.
- Pointer/aria: front `pointer-events:none` + `aria-hidden` when flipped;
  back the inverse. `backface-visibility:hidden` hides the rotated-away face
  visually; the pointer/aria gates stop the stacked faces from capturing
  input.
- Focus-follow (f6684aba): `FileEditorTab focused` gains `&& !showingBack`
  (mirrors terminals at `:1362-1363`) so the editor only pulls DOM focus
  while front-facing. The front no longer unmounts on flip, so f6684aba's
  "indexing graph survives flip" is now structural; the `indexingCache`
  workaround stays (harmless) and gets revisited in A2.
- `flipHybrid` reduces to lazy-init back + toggle `showingBack`; remove
  `requestPaneFlip` + the `paneFlip` export.

### Smoke plan (before A2)
Cmd+, on file / terminal / graph / browser / dashboard panes, both
directions, focused and unfocused. The original repro: flip fires only after
focus leaves the pane -> must now flip immediately, in-place, focus retained.

### Smoke results (browser, /tmp/chan-test-flip @ :8810) — A1 PASS
Real Cmd+, via CDP (synthetic KeyboardEvent did NOT reach the app's
keybinding dispatcher; not a code issue, just the harness path). DOM probed
per step. All green:
- BUG-1 core: flipped a FOCUSED editor (cm-content active) and a FOCUSED
  terminal (xterm-helper-textarea active); the card rotated immediately
  (`showing-back` class + computed `matrix3d` == rotateY(180deg)), no need to
  click away first. This is the exact regression, fixed.
- Two-face card: `transition: transform 0.4s`, parent `perspective: 1600px`,
  `transform-style: preserve-3d`; both faces mounted; rotated-away face
  `inert` (front when flipped, back when front). Front released DOM focus on
  flip (activeElement -> BODY, no focus trap in the inert subtree).
- Focus-follow: editor regained cm-content focus and terminal regained
  xterm focus on flip-back.
- All 5 surfaces: file / terminal / graph / browser / dashboard each show the
  correct Hybrid*Config back; front content SURVIVES the flip (terminal stays
  mounted + `visibility:hidden` not unmounted -> scrollback safe; GraphCanvas
  survives -> subsumes f6684aba bug-2; browser + dashboard carousel survive).
- Latch: never-flipped pane has empty back (`backChildCount 0`); first flip
  mounts it; persists across flip-back for the animation.
- Multi-pane: split right, two independent `.flip-card`s. Flipping the right
  editor pane left the left dashboard pane untouched (not flipped, not inert,
  carousel still showing). pane-mode (Hybrid Nav) preview overlay intact.

### Gate status
svelte-check 0/0. vitest: my flip suites green (tabs.test.ts 149,
paneFocusFollowFlip 7). Two reds remain — altSpaceXtermHandlerRemoved +
terminalGeneratedReplyFanout — both pure terminal/xterm source-pattern locks
from @@LaneC's in-flight BUG-3 work (I touch neither file). Flagged to @@Alex.
Rust untouched (frontend-only change); `cargo build -p chan` green.

### A1 committed: f5c773c5 (2026-05-30)
Committed solo per @@Alex while @@LaneC's tabs.svelte.ts work is still
in-flight. 4 files: Pane.svelte + paneFocusFollowFlip.test.ts (clean) + my
two hunks of tabs.svelte.ts (paneFlip-bus removal @755, flipHybrid @3001) +
three hunks of tabs.test.ts. Staged the shared-file hunks via
`git apply --cached` of a filtered patch (`/tmp/hunkfilter.py`, by old-start
line) so @@LaneC's live working tree was never touched -- `git add -p` is
unavailable in this harness. Verified post-commit: Pane.svelte clean, C's 13
remaining hunks + 38 marker-lines still in the tabs.svelte.ts working tree.
A5 (screensaver/*) intentionally NOT in this commit (separate task, ready).

### Shared-file note (tabs.svelte.ts)
@@LaneC is co-editing tabs.svelte.ts (added `keyboardProtocol` on
TerminalTab, +32 lines). My edits (removed `paneFlip` bus + `requestPaneFlip`,
simplified `flipHybrid`) are in disjoint A-owned regions. Working tree now
carries BOTH lanes' uncommitted changes to this file -> do NOT commit it
solo; coordinate the staged split with @@Alex/@@LaneC at merge time.
(Resolved: A1 committed via filtered `git apply --cached`; LaneC's hunks
later committed wholesale once my work was in.)

## A2 — controlled carousel + per-slot Dashboard back (commits)
- `fa5eff36` controlled carousel: `slide` prop (single source =
  tab.carouselSlide), `active` pause when flipped, "Indexing"->"Search"
  relabel, conditional legend. Browser-smoked: manual + auto-rotate
  round-trip, active-pause freezes while flipped, relabel + legend.
- `29433566` A5: screensaver/matrixRain.ts helper + MatrixRainPreview.
- `dbf59875` per-slot Dashboard back = **CK-INDEX**. DashboardSlotBack
  (shell + force-paused slot picker) dispatches AboutSlotConfig /
  WorkspaceSlotConfig / SearchSlotConfig off tab.carouselSlide.
  HybridDashboardConfig deleted. A-helper built the 3 leaf bodies; I
  restored the global Appearance radio it had dropped (it conflated my
  "Plain preview is A-core's" note). Browser-smoked all 3 slot backs:
  About (Appearance radio + Screen lock + MatrixRainPreview static
  frame), Workspace (chan-reports + Metadata archive), Search (Index
  widget live state idle/7/7/model + Semantic + Embedding). Slot picker
  switches slots while flipped. **CK-INDEX confirmed -> @@LaneB cleared
  to delete SearchStatusOverlay.svelte (B3).**
- `dd81b9c4` File Browser back reduced to placeholder (its semantic/
  embedding/reports moved to the Dashboard backs; no duplication).

## Merge (per @@Alex)
- All my work committed (f5c773c5 A1, 29433566 A5, fa5eff36 + dbf59875 +
  dd81b9c4 A2). LaneB's B1/B2 (SCOPE+STATUS removal, `fda36d53`) already
  on main.
- LaneC's completed terminal work committed as `41b28e7a` (one commit:
  Shift+Enter fix + tab groups + chan shell/cs; entangled shared files
  made a clean per-feature split risky on a peer's code). Whole tree
  gate-green: fmt + clippy + test + --no-default-features + svelte-check
  + vitest 1572 + npm build. Working tree clean (only journals untracked).
- A2.4b FB placeholder is gated-green + a static component; not
  separately browser-smoked (no reactivity; the running test binary at
  :8810 predates it).
