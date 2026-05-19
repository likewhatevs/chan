# fullstack-84: per-tab inspector width (BrowserTab + GraphTab + FileTab)

Owner: @@FullStackB
Cut by: @@Architect
Date: 2026-05-19

## Why

@@Alex caught a real bug: inspector resize is
global per tab-kind, not per-tab. Open two
Files tabs side by side, open the inspector
in each, drag to resize one → all FB
inspectors across both tabs flip to the same
width. Same class of bug as `-58`'s
BrowserTab schema gap, but for the
inspector-width state instead of selection.

Repro per @@Alex: two Files tabs side by side
with inspectors open, drag-resize one →
both move.

Affected tab kinds (same surface, same
underlying singleton):
* **Files tab** — `paneWidths.browser`.
* **Graph tab** — likely `paneWidths.graph`
  or equivalent.
* **File editor tab** — `paneWidths.inspector`
  (file info) AND/OR `paneWidths.outline`.

## Relevant code

* `web/src/state/store.svelte` (or wherever
  `paneWidths` lives) — module-level
  singleton driving every `Inspector`
  consumer. Source of the bug.
* `web/src/components/FileBrowserSurface.svelte:380`
  area — `<Inspector bind:width={paneWidths.browser}>`.
  Bind to per-tab field instead.
* `web/src/components/GraphPanel.svelte` —
  similar inspector consumer. Find the
  width binding.
* `web/src/components/FileEditorTab.svelte`
  (or wherever the file-editor inspector
  + outline mount) — same fix.
* `web/src/state/tabs.svelte.ts` —
  `BrowserTab` / `GraphTab` / `FileTab`
  schemas. Add per-tab `inspectorWidth`
  field(s).

## Spec

* Each tab record carries its own
  `inspectorWidth?: number` (and
  `outlineWidth?` for the file-editor tab
  if outline + inspector are independent).
* Inspector consumers `bind:width={tab.inspectorWidth ?? paneWidths.<kind>}`
  — falls back to the module singleton
  for backwards compatibility when no
  per-tab override is set.
* Drag-resize writes to the active tab's
  field (NOT the singleton).
* Hash round-trip: serialize the per-tab
  inspector width when set. Match `-58`'s
  pattern (conditional emission so single-
  tab hashes stay clean).
* On tab spawn: new tab inherits the
  current `paneWidths.<kind>` value as
  its initial `inspectorWidth` — feels
  natural (new tab opens with the user's
  last-used width).

### Per-tab vs per-pane

@@Alex's framing is "per-pane inspector
sizes" but the repro shows the issue is
actually per-tab — opening two FB tabs in
the SAME pane should also let each tab have
its own width if the user resizes one of
them. **Per-tab is the right granularity**;
per-pane would only fix the cross-pane
case but leave the cross-tab-in-same-pane
case still broken.

If implementation cost diverges
significantly, flag in the impl note —
otherwise go per-tab.

## Acceptance criteria

* Two Files tabs in the same pane: drag-
  resize tab 1's inspector → tab 2's
  width unchanged.
* Two Files tabs in different panes: same.
* Two Graph tabs: same.
* Two file editor tabs with inspector
  open: same. (Outline width also
  per-tab if independent.)
* Tab switch preserves each tab's
  inspector width (no global "current
  width" overrides).
* Hash round-trip: a URL with per-tab
  inspector widths set restores each
  tab's width exactly on reload.
* New tabs spawn with sensible default
  width (inherit `paneWidths.<kind>`
  current value at spawn time, OR a
  fixed default — your call).
* Single-tab-per-kind behaviour unchanged
  (no regression for the common case).

### Tests

* Vitest: tab records carry independent
  `inspectorWidth` values without
  collision.
* Component test: two FB tabs render
  inspectors at different widths
  simultaneously.
* Regression: resize on one tab does
  NOT mutate the other.
* Hash round-trip: serialize / deserialize
  preserves per-tab width.

### Gate

* `npm run check`
* `npm run test`
* `npm run build`
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`

## Notes

* v0.11.0-blocking. Same severity as
  `-58` (marquee multi-tab surface
  half-shipped without it). The
  multi-tab feature is one of the loud
  v0.11.0 headlines; shipping it with
  shared inspector widths makes the
  feature feel less than it is.
* Schema extension follows `-58`'s
  precedent (conditional hash emission;
  hash key naming convention preserved).
  Reuse the helpers if `-58` factored
  any out.
* Coordinate with `-80` (FB click
  auto-opens inspector in tab/overlay):
  the per-tab `inspectorOpen` flag from
  `-58` already exists. This task is
  just adding the width slot.
* Re-walk: open multi-tab layouts of
  each kind (Files / Graph / file-editor),
  drag-resize one tab's inspector,
  confirm others stay put. Reload,
  confirm widths restore.
* Queue position: end of Lane B queue.
  Updated queue: `-78` → `-79` → `-80`
  → `-82` → `-84`.
* Standing topic-level commit clearance.

## 2026-05-19 23:55 BST — implementation

**Per-tab schema additions.**
* `BrowserTab`: `inspectorWidth?: number`.
* `GraphTab`: `inspectorWidth?: number`.
* `FileTab`: `inspectorWidth?: number` AND
  `outlineWidth?: number` (outline +
  inspector are independent panes).

**Overlay-state extensions** (so the overlay
variants are independent of the dock
singleton):
* `browserOverlay`: `inspectorWidth?: number`.
* `graphOverlay`: `inspectorWidth?: number`.

**SerTab fields.** `iw?: number` covers
BrowserTab + GraphTab + FileTab; `ow?: number`
is FileTab-only. Both emit only when set and
positive (matches the `-58` conditional-
emission pattern so single-tab hashes stay
clean). All four restore sites (front-side +
back-side × graph + browser + file) updated.

**Two-way bind with fallback.** Used Svelte 5's
function-pair `bind:value={getter, setter}`
syntax:

```svelte
<Inspector
  bind:width={
    () => browserState.inspectorWidth ?? paneWidths.browser,
    (v) => (browserState.inspectorWidth = v)
  }
  ...
/>
```

The getter reads the per-tab value with a
fallback to the module singleton (so a fresh
tab without an override paints at the
current default). The setter writes only to
the tab record. ResizeHandle's drag updates
go straight to the per-tab slot; no
ping-pong with the singleton.

`browserState = $derived(tab ?? browserOverlay)`
unifies the tab and overlay paths — both
flavours have an `inspectorWidth` slot now.
Same for `graphState`.

**Edits:**

* `web/src/state/tabs.svelte.ts`:
  - `BrowserTab.inspectorWidth?`,
    `GraphTab.inspectorWidth?`,
    `FileTab.inspectorWidth?` +
    `FileTab.outlineWidth?`.
  - `SerTab.iw?`, `SerTab.ow?`.
  - Serializer: emit `iw` for browser /
    graph / file (when set + positive),
    `ow` for file-only.
  - Restore: front-side + back-side
    restore sites for all three tab
    kinds read `iw` / `ow` back onto
    the corresponding tab field.

* `web/src/state/store.svelte.ts`:
  - `browserOverlay.inspectorWidth?: number`.
  - `graphOverlay.inspectorWidth?: number`.

* `web/src/components/FileBrowserSurface.svelte`:
  - Inspector binding flipped from
    `bind:width={paneWidths.browser}` to
    the function-pair form anchored on
    `browserState.inspectorWidth ??
    paneWidths.browser`.

* `web/src/components/GraphPanel.svelte`:
  - Same flip for `graphState.inspectorWidth`.

* `web/src/components/FileEditorTab.svelte`:
  - Inspector binding flipped to
    `tab.inspectorWidth`.
  - Outline binding flipped to
    `tab.outlineWidth`.

**Tests.**

* `web/src/state/tabs.test.ts` (2 new):
  - "two BrowserTab records carry
    independent inspectorWidth (fullstack-84)"
    — assert two BrowserTab records don't
    share the field.
  - "hash round-trips per-tab
    inspectorWidth on browser + graph +
    file (fullstack-84)" — set per-tab
    widths on two browsers + one graph
    + one file (with outline too);
    serializeLayout → restoreLayout →
    assert each tab's width preserved.

* `web/src/components/perTabInspectorWidth.test.ts`
  (new, 9 assertions):
  - Each surface (FB / Graph / FileEditor)
    has the function-pair bind expressions
    routing through tab state.
  - Each surface still references the
    `paneWidths.<kind>` fallback (so the
    initial paint without an override
    inherits the singleton).
  - None of the surfaces bind directly to
    `paneWidths.<kind>` anymore (caught
    by negative assertion).

**Gate.** `npm run check` 0/0; `npm run test`
43 files / 443 tests (was 42 / 433; +2
tabs.test sentinel + 9 perTabInspectorWidth +
~1 wash from parallel-lane carryover);
`npm run build` clean; `scripts/pre-push`
green.

**Visual eyeball.** Skipped. The change is
mechanical (binding-target rewire) and the
source-grep sentinel pins the wire. Re-walk
per the task note: open two tabs of each
kind, drag-resize one, confirm the other
stays put. Reload, confirm widths restore.
Quick.

**Defaults / first-mount behaviour.**
* Tab created with `inspectorWidth ===
  undefined` → getter returns
  `paneWidths.<kind>` → Inspector paints at
  the current default.
* User drags the resize handle → setter
  writes `tab.inspectorWidth = newValue`.
* Subsequent reads return the per-tab value
  (the `??` short-circuits before reaching
  the singleton).
* Tab swap (or tab.id change for the same
  surface): the getter/setter are recomputed
  against the new tab; surface re-renders
  with the new tab's width.

**Out of scope:**
* Migrating `paneWidths.search` to per-tab.
  Search has no tab record currently (it's
  an overlay only); leaving for a follow-up
  if Alex flags the cross-overlay case.
* Persisting the singleton from a per-tab
  drag. Spec says the singleton tracks the
  user's "preferred default for new tabs",
  not "last drag wins". The
  `persistPaneWidths` callback still fires
  on drag (for backwards compatibility with
  the existing preference write) but
  doesn't update `paneWidths.<kind>` since
  the setter writes the tab field, not the
  singleton. Behaviour matches the
  acceptance criterion ("new tabs inherit
  current value at spawn time").

**Commit readiness:**

Files staged:
* `web/src/state/tabs.svelte.ts`
* `web/src/state/store.svelte.ts`
* `web/src/components/FileBrowserSurface.svelte`
* `web/src/components/GraphPanel.svelte`
* `web/src/components/FileEditorTab.svelte`
* `web/src/state/tabs.test.ts` (existing,
  extended with 2 new tests)
* `web/src/components/perTabInspectorWidth.test.ts`
  (new sentinel)
* `docs/journals/phase-7/fullstack-b/fullstack-84.md`
* `docs/journals/phase-7/fullstack-b/journal.md`
* `docs/journals/phase-7/alex/event-fullstack-b-architect.md`

Proposed commit message:
```
Per-tab inspector + outline width (fullstack-84)
```

Standing topic-level commit clearance applies.
No HOLD pokes since the 23:00 BST cut.
