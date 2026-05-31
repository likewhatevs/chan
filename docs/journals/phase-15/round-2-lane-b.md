# Round-2 @@LaneB — Dashboard part-1 + frontend bugs

## You are @@LaneB

Domain this round: the Dashboard / carousel / flip frontend (round-1's Lane-A
domain). The four part-1 items below are round-1 Lane-A drops coming back to
their own files. Read `bootstrap.md`, then this file, then `round-2-part-1.md`
(the technical source of truth: root cause + file:line + reuse anchors for
A4/A3/A6/A7) and the two frontend bugs in `round-2-part-2.md`, then
`coordination.md`. Coordinate through **@@Architect (@@LaneA)**, not @@Host.
Confirm understanding, then start wave-1 (no go-gate; wave-1 has no cross-lane
dependency).

You may spawn subagents within your scope.

## You own (do not edit another lane's files)

`EmptyPaneCarousel.svelte`, `DashboardTab.svelte`, `FileInfoBody.svelte`,
`InspectorBody.svelte`, `dashboard/AboutSlotConfig.svelte` +
`dashboard/*SlotConfig.svelte`, a new `PlainScreensaverPreview.svelte`,
`GraphPanel.svelte`, `web/src/editor/decorations/walker.ts` +
`FileEditorTab` / `Wysiwyg.svelte` / `Source.svelte`, and the
**`DashboardTab` slot region** of `web/src/state/tabs.svelte.ts`.

You *call but do not edit* `revealPathInBrowser` / `openFsGraphForDirectory`
in `store.svelte.ts` (they already exist; A4 only binds them).

Shared-file note: `tabs.svelte.ts` is co-edited with @@LaneD (they own the
`TerminalTab` / `TeamWorkState` region). Your `DashboardTab` edits are a
disjoint region. Use the chained `git add <paths>` + `git diff --staged --stat`
+ commit + `git show --stat HEAD` discipline on every commit that touches it.

## Tasks

### Wave 1 — part-1 (start now)

- **A6** (`round-2-part-1.md` §A6). About-front: move *only* chan's Apache 2.0
  link onto the version row (`chan version {version} Apache 2.0`); drop the
  `chan` row from `about-licenses`; fix the stale block comment. Trivial; do
  first. Browser-smoke the About front.
- **A7** (§A7). About-back preview switches on `screensaverTheme`: Matrix ->
  `MatrixRainPreview`, else a new `PlainScreensaverPreview.svelte` (enso mark on
  dark backdrop, mirroring `ScreensaverOverlay.svelte:155-160` + CSS `:221`),
  hint tracks the theme. Back face stays timer-free. Browser-smoke the dropdown
  live-swap.
- **A4** (§A4 + BUG-2). Search-slot directory inspector: in `FileInfoBody`
  add `onNewTerminal?` + `allowUpload?` (default true), gate Upload on
  `allowUpload`, add New-Terminal in the directory actions; forward both through
  `InspectorBody`; in `EmptyPaneCarousel` (the slide-2 mount ~592-599) bind
  `onReveal` / `onSetAsScope` / `onNewTerminal` + `allowUpload={false}`. Reuse
  anchors are listed in the part doc. Browser-smoke: Show Directory + Graph from
  here + New Terminal present, Upload gone, Download kept; confirm File Browser /
  editor / full Graph-tab inspectors **still** show Upload (no regression).
- **A3** (§A3). Dashboard tab right-click menu: per-slot on/off checkboxes
  (>=1 enabled, default all-on, persisted as `ds?: number[]` beside
  `carouselSlide`), separator, "Settings (Cmd+,)" calling `flipHybrid`.
  Auto-rotation skips disabled slots; `carouselSlide` clamps off a disabled
  slot. **You must reverse the existing lock-out test** at
  `dashboardTabAndCarousel.test.ts:282` ("carries only Reload") and the
  no-Settings design comment - see the §A3 caveat. Browser-smoke the menu,
  the skip-rotation, the min-one guard, and reload persistence.
- **BUG-GRAPH** (`round-2-part-2.md`, the "Graph from here" bug). One-line root
  cause: `graphFromHere` (`GraphPanel.svelte:390`) never sets
  `graphState.mode = "filesystem"`. Set it for the **directory** case only
  (matches the bug report); leave the file case and the breadcrumb
  `rescopeFromHere` unchanged - that is @@Architect's call, do not widen it.
  Fixes both symptoms (wrong plot + dead double-click expand). Browser-smoke:
  semantic graph -> Graph from here on a dir shows the dir's files + double-click
  expands; file-browser Graph-from-here still works.

### Wave 2

- **BUG-EDITOR** (`round-2-part-2.md`, "bold/inline marks revert to raw after a
  tab switch"). Root cause + three candidate fixes are in the part doc; decide
  empirically (candidate (a), adding `geometryChanged` to the walker recompute
  condition, is the lowest-touch start). Browser-smoke required - static gates
  miss CodeMirror measure/decoration timing. This is the same class as the
  already-fixed terminal "garbled until click" bug; mirror that pattern.

## Cross-lane coordination

- You have **no inbound checkpoint dependency** for wave-1; start immediately.
- **CK-CAROUSEL (you -> @@LaneD, wave-2):** @@LaneD's `cs dashboard
  --carousel-off` flag sets a field on a newly created `DashboardTab`. Decide
  with @@LaneD what that field is (reuse `ds` / `disabledSlots`, or a separate
  `autoRotate:false`) and expose it cleanly so they can set it. Tell
  @@Architect when the field shape is fixed.
- If A4's New-Terminal / Graph-from-here behavior needs a store helper that
  doesn't exist, cut a task to the owning lane via @@Architect - do not edit
  `store.svelte.ts` regions you don't own.

## Gate + smoke (every increment)

`web/`: `npx svelte-check` (0/0), `npm run test` (vitest), `npm run build`.
Frontend-only - no Rust expected; if you touch Rust, run the full repo pre-push
gate. Browser-smoke everything reactive (A3 menu, A7 live preview swap,
BUG-EDITOR conceal, BUG-GRAPH mode switch) on a running test server - ask
@@Architect for the seeded drive (the BUG-GRAPH + A4 smokes want nested dirs;
the editor bug wants a doc with raw markers like this very part doc).

## Merge

Merge gated-green increments to `main` locally as they land; tell @@Architect
on each merge that touches `tabs.svelte.ts` so @@LaneD rebases. Append progress
to a `round-2-lane-b-journal.md` + `event-lane-b.md`; poke @@Architect on
completion/checkpoint.
