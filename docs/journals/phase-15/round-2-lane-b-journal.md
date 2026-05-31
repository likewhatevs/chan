# Round-2 @@LaneB journal — Dashboard part-1 + frontend bugs

Domain: the Dashboard / carousel / flip frontend (round-1's Lane-A domain).
Tasks: wave-1 = A6, A7, A4, A3, BUG-GRAPH; wave-2 = BUG-EDITOR.
Coordinate through @@Architect (@@LaneA).

## Status

- **Wave-1 code complete + static-gated green** (A6, A7, A4, A3, BUG-GRAPH).
  svelte-check 0/0, `vite build` clean, vitest 1576/1577 (the one failure is
  @@LaneD's uncommitted Cmd+R WIP, see "Shared worktree" below — not mine).
- **Browser-smoke pending**: A3 menu / skip-rotation / persistence, A7 live
  preview swap, A4 inspector actions, BUG-GRAPH mode switch all need a running
  test server. Requested a seeded drive (nested dirs) from @@Architect.
- **Not yet committed/merged** — holding for the browser smoke before merging
  the reactive pieces.

## A6 — About-front license placement (done, code)

`EmptyPaneCarousel.svelte` slide 0:
- Moved chan's own `Apache 2.0` LICENSE anchor onto the version row
  (`<span class="v mono">{version} <a class="version-license" ...>Apache
  2.0</a></span>`). New `.about-grid .version-license` CSS: left margin +
  link color + underline (not mono).
- Removed the `chan` / `Apache 2.0` pair from `.about-licenses`; left the
  `terminal font` (SIL OFL) + `matrix screen lock` (MIT) attributions.
- Rewrote the stale block comment (was "Chan's own Apache 2 license joins the
  section so the three runtime licenses sit together").
- Test `dashboardTabAndCarousel.test.ts`: rewrote "About widget licenses block
  sits after the QR" -> asserts the version row carries the LICENSE anchor, the
  `chan` row is gone from `.about-licenses`, and the LICENSE anchor appears
  exactly once (no duplicate). Font + screensaver rows still asserted single.

## A7 — About-back screensaver preview reacts to theme (done, code)

- New `web/src/components/screensaver/PlainScreensaverPreview.svelte`: static
  enso mark on a dark backdrop, mirroring `ScreensaverOverlay.svelte`'s
  `.screensaver-mark` (masked `/chan-mark.png` over `--text-secondary`, ~0.38
  opacity, `var(--bg)` backdrop). Same prop interface as `MatrixRainPreview`
  (`width`/`height`), no timers (static = rAF-free, safe for the always-mounted
  back face).
- `dashboard/AboutSlotConfig.svelte`: imported it; the preview now switches on
  `screensaverTheme` (`matrix` -> `MatrixRainPreview`, else
  `PlainScreensaverPreview`); the hint string tracks the theme
  ("... Matrix/Default lock theme"). Updated the punt comment.
- Test: asserts the `{#if screensaverTheme === "matrix"} ... {:else}` switch,
  the theme-tracking hint, and the new import; `not.toMatch` the old
  Matrix-only hint. Plus a component test (chan-mark mask + --bg backdrop).

## A4 — Search-slot directory inspector actions (done, code)

- `FileInfoBody.svelte`: added `onNewTerminal?: () => void` + `allowUpload?:
  boolean` (default true). Upload button gated behind `{#if allowUpload}`
  (Download stays unconditional). New "New Terminal" button in the directory
  action row, `{#if onNewTerminal && isDir}`, after "Graph from here".
- `InspectorBody.svelte`: added both props; forwards `onNewTerminal` +
  `allowUpload` in the directory arm, `allowUpload` in the file arm.
- `EmptyPaneCarousel.svelte` slide-2 index-graph inspector mount: bound
  `onReveal` (revealPathInBrowser, enter+inspectorOpen), `onSetAsScope`
  (openFsGraphForDirectory), `onNewTerminal` (openTerminalInPane(
  layout.activePaneId, terminalFromHereTarget(path, true))), `allowUpload=
  {false}`. Each handler guards `selectedIndexPath !== null` (the mount is
  already gated on it; the guard also satisfies the type narrowing). Imported
  `revealPathInBrowser` + `openFsGraphForDirectory` from store.svelte
  (call-only, not edited), `layout` + `openTerminalInPane` from tabs.svelte,
  `terminalFromHereTarget` from terminal/fromHere — the same helpers the File
  Browser tree menu uses (FileTree.svelte graphThis / terminalFromHere).
- Default-true `allowUpload` keeps File Browser / editor / full Graph-tab
  inspectors showing Upload (no regression). New Terminal is directory-only.
- Tests verified A4 does NOT break inspectorActionsLayout.test.ts (it only
  pins onReveal/onSetAsScope, not Upload). Added A4 assertions to
  dashboardTabAndCarousel.test.ts.

## A3 — Dashboard tab slot on/off menu + Settings (done, code)

`tabs.svelte.ts` (my DashboardTab region):
- `DashboardTab` gains `disabledSlots?: number[]`.
- New `export const DASHBOARD_SLOT_COUNT = 3` + helpers `dashboardSlotEnabled`,
  `toggleDashboardSlot` (refuses to disable the last enabled slot; clears the
  field to undefined when all-on), `firstEnabledSlot`, `nextEnabledSlot`.
- Serialize arm: emits `ds: number[]` only when non-empty (next to `cs`).
  `SerTab` gains `ds?: number[]`.
- Restore arm: sanitizes `ds` to in-range unique indices (ignores it if it
  would disable every slot), and clamps the restored `carouselSlide` to the
  first enabled slot when it points at a disabled one (incl. the default-slot-0
  case).
- `cloneTab` dashboard arm now carries `carouselSlide` + `disabledSlots` when
  set (was dropping both — latent omission); default tab still clones to the
  same minimal `{kind,id,title}` shape (no test breakage).

`EmptyPaneCarousel.svelte`:
- New `disabledSlots = []` prop. `slotEnabled` / `enabledSlots` (derived) /
  `firstEnabled` / `nextEnabled` / `prevEnabled` over the prop.
- `slideIndex` now `$derived.by`, clamps off a disabled slot to firstEnabled.
- Auto-rotate advances via `nextEnabled(slideIndex)`; prev/next/goTo skip
  disabled; pagination dots iterate `enabledSlots` (no dot for a hidden slot).

`DashboardTab.svelte`:
- Menu (HamburgerMenu) now: one `role="menuitemcheckbox"` row per slot
  (About/Workspace/Search), separator, "Settings" (chord app.settings.toggle =
  Cmd+,) calling `doSettings -> flipHybrid(layout.activePaneId)` (mirrors the
  global Cmd+, in App.svelte; in-scope, no Pane.svelte edit), then Reload.
- `onSlotToggle` toggles + advances the cursor to firstEnabled if the active
  slide was switched off, else persists via scheduleSessionSave.
- Reversed the round-1 lock-out test "carries only Reload (no Settings entry)".
- Bumped the HamburgerMenu height estimate 58 -> 200 (real size is measured
  post-mount; estimate only drives the first-paint above/below flip).

### A3 menu-surface decision (coordination, raised to @@Architect)

The roadmap wanted the slot menu reachable from the **tab title**. The
tab-strip context menu lives in `Pane.svelte`, which I do **not** own. I
implemented the rich menu on the DashboardTab **body** right-click (in scope,
fully functional: checkboxes + Settings + Reload). The tab-TITLE reachability
needs a `Pane.svelte` edit (dashboard arm in the tab-strip menu, or routing the
tab-title right-click to DashboardTab's menu). Cutting that as a cross-lane task
via @@Architect rather than editing Pane.svelte myself. The part doc flagged
this as an implementation decision.

## BUG-GRAPH — in-graph "Graph from here" on a directory (done, code)

`GraphPanel.svelte` `graphFromHere`: added `graphState.mode = "filesystem"` in
the **directory** branch only (one line). Fixes both symptoms (wrong plot +
dead double-click expand, both rooted in the missing mode switch). Did NOT
touch the file case or breadcrumb `rescopeFromHere` — that widening is
@@Architect's call per the lane task.

## Shared worktree (tabs.svelte.ts) + the cmdR test failure

- `tabs.svelte.ts` is co-edited with @@LaneD (their TerminalTab / TeamWorkState
  region). My edits are the disjoint DashboardTab slot region. On commit I will
  chain `git add <my paths>` + `git diff --staged --stat` + commit + `git show
  --stat HEAD` and stage ONLY my files.
- Full vitest shows 1 failure: `cmdRWindowReload.test.ts` (App.svelte's
  onWindowKey Cmd+R matcher). App.svelte / shortcuts.ts / keymap.ts /
  keymap.test.ts / serve.rs / Pane.svelte are all dirty in the worktree from
  @@LaneD's Ctrl+R-remap + shift+Enter work — none are files I touched. This is
  the same concurrent-WIP pattern as round-1; not my regression.

## Gate (web; frontend-only, no Rust)

- svelte-check: 0 errors / 0 warnings.
- vite build: success (only pre-existing chunk-size / dynamic-import warnings).
- vitest: 1576/1577 (1 unrelated @@LaneD WIP failure as above). My own files'
  suites (dashboardTabAndCarousel, inspectorActionsLayout) all pass.

## Wave-2 prep landed early: CK-CAROUSEL autoRotate

@@Architect ruled `cs dashboard --carousel-off` is a per-tab **autoRotate**
flag, NOT `disabledSlots` (I had flagged the same). Added `autoRotate?:
boolean` on DashboardTab (default true via `?? true`), serialize (`ar:false`
only when off) + restore + clone, and the carousel `paused` derived now folds
in `!autoRotate`. @@LaneD sets `tab.autoRotate = false` in
`store.svelte.ts` `open_dashboard` handler (~733-745, mirroring the existing
`carousel_index` path) when `--carousel-off` is passed. Field name communicated
at CK-CAROUSEL.

## A3 tab-title hook (Pane.svelte NOT needed)

@@Architect expected a Pane.svelte tab-menu-region edit. Source said
otherwise: the tab menu is rendered per-tab-component (TerminalTab /
FileEditorTab / FileBrowserSurface each read `tabMenu.openForTabId === tab.id`),
and Pane.svelte's `openTabMenu` trigger already fires for every tab kind
(no dashboard gate). So the hook lives entirely in DashboardTab: a `$effect`
watches the shared `tabMenu` state and, when it targets this tab, consumes it
(`closeTabMenu()`) and opens the existing HamburgerMenu at the click point -
reusing the same rows as the body menu. Zero Pane.svelte change; @@LaneD's 2
Pane.svelte lines stay untouched.

## Wave-1 browser smoke (server: my binary on /tmp/chan-test-lb2 @ 8822)

Heavy repo-clone seed (/tmp/chan-test-r2, 4096 files) WEDGES preflight: the
embedding pass pegs CPU (measured 80.9%) and the status counter pins at
4099/4096, never reaching Idle in a debug build, so PreflightOverlay never
unlocks. This is the CK-INDEX-IDLE bug (confirmed independently by @@LaneD;
@@Architect concurred). Data point for @@LaneC: the **initial bulk index** is
just impractically slow in debug (not a logic deadlock) - a small 11-file drive
reached Idle in ~6s. The part-2 wedge is the post-draft *reindex* path.
-> Switched to a small seeded drive (11 notes + nested dirs:
notes/{,deep}, project/{,src}, gateway/{,identity}).

All five wave-1 items VERIFIED empirically:

- **A6**: About front version row reads `chan version 0.20.0 Apache 2.0`
  (Apache 2.0 a working link next to the version); attributions block below
  Fund-the-work has only terminal font + matrix screen lock; no duplicate chan
  row. (version is 0.20.0 - the v0.21.0 pin is @@Host's release task.)
- **A7**: About back, Screensaver preview. Default theme -> enso mark on dark
  backdrop + "Static preview of the Default lock theme."; switching the Theme
  dropdown to Matrix swapped the preview LIVE to the matrix-rain frame + hint
  "... Matrix lock theme." Both branches + theme-tracking hint confirmed.
- **A4**: Dashboard Search-slot directory inspector (clicked `gateway/identity`
  node): Show Directory + Graph from here + New Terminal + Download, NO Upload.
  New Terminal opened a terminal with cwd = `.../chan-test-lb2/gateway/identity`
  (binding wiring proven). No regression: the full Graph-tab dir inspector for
  `project/` still shows Upload (default-true flag). Show Directory + the
  Dashboard-inspector Graph-from-here clicks were NOT individually fired
  (parity-verified: same mount + guard pattern as the proven New Terminal,
  calling the same FileTree store helpers revealPathInBrowser /
  openFsGraphForDirectory).
- **A3**: tab-title right-click opened the menu (3 slot checkboxes + Settings
  Cmd+, + Reload). Unchecking About (the active slide) -> carousel clamped to
  Workspace, hash `cs:1,ds:[0]`, dots 3->2. Unchecking Workspace -> Search,
  `cs:2,ds:[0,1]`, 1 dot. Unchecking Search (last enabled) -> REFUSED (min-one
  guard), hash unchanged. Reload -> restored to Search with ds:[0,1] (serialize
  /restore round-trip + clamp confirmed). Re-checking all -> `ds` cleared from
  the hash (toggle clears the field when all-on).
- **BUG-GRAPH**: semantic Graph tab, selected `project/` dir, in-graph "Graph
  from here" -> status bar flipped to "filesystem graph", hash `gm:"f",
  gs:"dir:project"`, plotting project/'s files (src/ + readme.md), NOT the
  semantic markdown neighbourhood (symptom 1 fixed). Double-clicking `src/`
  expanded it to main.md + util.md, hash gained `ge:["project/src"]` (symptom 2
  fixed).

Teardown: my tab closed, 8822 server killed (scoped pid), /tmp/chan-test-lb2 +
/tmp/chan-test-lane-b removed; shared baseline /tmp/r2srv:8820 and @@LaneC/D
servers untouched. /tmp/laneb-srv binary kept for any post-CK-INDEX-IDLE
re-smoke.

## Merge (local, on main, NOT pushed)

- `fc1730e5` fix(graph): BUG-GRAPH (GraphPanel.svelte, +6).
- `37d68bef` feat(dashboard): part-1 (A6/A7/A4/A3) + autoRotate + tests (8
  files, +607/-66).
- Chained `git add <explicit paths>` + `git diff --staged --stat` audit +
  commit + `git show --stat HEAD` per commit. tabs.svelte.ts had ONLY my
  DashboardTab-region hunks at commit time (HEAD unmoved at 403547c4; @@LaneD's
  round-2 work is in keymap.ts/App.svelte/Pane.svelte, not tabs.svelte.ts yet).
  Verified post-commit: every remaining dirty file is @@LaneD's; none of mine.
- **@@LaneD: rebase** - tabs.svelte.ts now carries the DashboardTab
  disabledSlots + autoRotate region on main.

## Wave-1 status: A6, A7, A4, A3, BUG-GRAPH all DONE, smoked, merged to main
locally. Carryover: A4 Show-Directory / Graph-from-here button clicks are
parity-verified not individually fired (negligible risk). Next: wave-2
BUG-EDITOR.

## Wave-2: BUG-EDITOR (done, committed d861b61b) - EMPIRICALLY UNVERIFIED IN CHROME

Fix: candidate (a) from the part doc - add `geometryChanged` to the conceal
walker's recompute condition (`walker.ts` update()). Root cause: editor tabs
unmount/remount on tab switch, the reconstructed EditorView's constructor walks
the INITIAL pre-layout viewport (top only), and the post-layout measure does
not reliably fire `viewportChanged`, so lower blocks stay raw until a caret
move/scroll. Recomputing on `geometryChanged` re-decorates over the settled
viewport. Walk is viewport-bounded -> cheap. Added a source-pattern regression
test (`walker.test.ts`).

**The bug does NOT reproduce in Chrome/Blink.** I built + served the PRE-FIX
binary and ran the exact repro (tall raw-marker doc, switch editor->editor->
back, no click): the lower blocks rendered concealed correctly anyway. Then the
post-fix binary: also clean. So Chrome cannot distinguish fixed from unfixed -
the layout-measure race manifests under chan-desktop's **WKWebView**, not Blink
(same class as the terminal 'garbled until click' WKWebView bug; matches the
`reference_terminal_webgl_wkwebview` rule + the part doc's "same class"
framing). I CANNOT automate a chan-desktop (WKWebView) repro - the Chrome MCP
tools only drive Blink.

What I DID verify: svelte-check 0/0, vitest 1584/1584, vite build clean; the
fix causes **no Chrome regression** (conceal still works perfectly with it).
What's PENDING: a chan-desktop manual repro to confirm the fix resolves the
desktop bug (tall doc full of `**bold**`/`` `code` ``, switch tab away+back,
confirm no raw markers in the lower viewport without clicking). Flagged to
@@Architect; recommend routing the desktop verify to @@Host.

Decision rationale for committing despite the Chrome gap: gated-green + root-
cause-sound + Chrome-regression-free + low-risk (extra viewport-bounded
recompute), consistent with the pre-release "merge gated-green, record
empirically-unverified, re-report if it breaks" norm. Local, revertible.

## Round-2 Lane-B status: ALL SIX items (A6, A7, A4, A3, BUG-GRAPH, BUG-EDITOR)
implemented, gated green, merged to main locally. Commits: fc1730e5 (BUG-GRAPH),
37d68bef (part-1 + autoRotate), d861b61b (BUG-EDITOR). Empirically-unverified:
BUG-EDITOR (WKWebView-only, needs chan-desktop) + A4 two button clicks (parity).
Nothing pushed. No round-3 carryover from Lane B.
