# fullstack-54: drop the path-display header from FileBrowserSurface

Owner: @@FullStackB
Cut by: @@Architect
Date: 2026-05-19

## Goal

Remove the path-display header that sits at the top
of every File Browser surface (the bar showing
`/private/tmp/chan-webtest-a-1` or the equivalent
drive root). It duplicates context the tab strip
already provides and consumes vertical space the
tree could use.

@@Alex confirmed: "this entire top bar with the path
is not useful anymore". The screenshot shows the
**tab variant** where the redundancy is most stark
(Files tab strip immediately above the path bar),
but the path display should go in all variants.

## Relevant code

* `web/src/components/FileBrowserSurface.svelte:75-77`
  — `browserTitle` derived value:
  `fileBrowserTitlePath(browserSelection.path,
   drive.info?.root ?? drive.info?.name ?? "drive")`.
  If nothing else consumes `browserTitle`, the
  derived can go too.
* `web/src/components/FileBrowserSurface.svelte:312`
  — `<span class="name" title={browserTitle}>...</span>`
  inside the surface `<header>`. The span is what
  the user is pointing at.
* Same file's `<header>` block (starts around the
  tab/overlay/dock chrome rows roughly line 280-321)
  — the chrome buttons that share the header
  (close, maximize, unstick, hamburger) need a home;
  see the variant matrix below.
* `web/src/state/` — `fileBrowserTitlePath` helper.
  If unreferenced after this change, it can also go;
  audit usages first (other surfaces might consume
  it).

## Variant matrix

Three variants render through this surface. The
right answer for each:

1. **Tab variant (`isTab`)** — the parent tab strip
   already carries the "Files" label, the × close,
   and (per other tabs in this app) a per-tab
   kebab. The surface header is fully redundant.
   **Recommendation: drop the entire `<header>` in
   this variant.** Tab-strip kebab inherits any
   menu items the surface header was hosting (verify
   the inheritance path; if the surface menu isn't
   already on the tab kebab, wire it through).
2. **Dock variant (`variant === "dock"`)** — the
   surface is the side panel; there's no tab strip
   above it. The header currently carries the
   unstick button + the kebab. **Recommendation:
   keep the header chrome row but drop the
   `<span class="name">` from it.** The remaining
   row should be a slim chrome strip (unstick +
   kebab on the right), not a thick path bar.
3. **Overlay variant (`isOverlay`)** — the surface
   opens over the workspace. Header carries the
   close + maximize + kebab.
   **Recommendation: same as dock — keep the chrome
   row, drop the path span.**

If the cleanest implementation is to drop the
header span in all three variants and let the
chrome row slim down naturally (rather than
conditionally hiding the whole header in tab
variant), that's fine too. Whichever lands
cleanly without leaving an empty box of padding
behind.

## Acceptance criteria

* `<span class="name" title={browserTitle}>{browserTitle}</span>`
  no longer renders in any variant.
* `browserTitle` derived (line 75-77) removed if
  unreferenced; otherwise left alone.
* In tab variant, the file-browser surface's
  topmost element is the tree (or the find bar
  if open), with no path-display row above it.
  Kebab menu items still reachable (via tab-strip
  kebab or equivalent path).
* In dock + overlay variants, the chrome row
  collapses to a slim strip (chrome buttons only,
  no path text); no orphan padding / empty title
  area.
* `fileBrowserTitlePath` helper: if no remaining
  consumers, delete it. If still used somewhere
  (e.g. document title), leave alone and note
  where.
* No regression on the existing chrome buttons
  (unstick / maximize / close / kebab) — they
  continue to fire their handlers in their
  respective variants.

### Tests

* Update or add a small test asserting the rendered
  surface DOM does NOT include the path-display
  span. Light-touch is fine — a `getByText` /
  similar against `drive.info.root` or the
  computed `browserTitle` should fail.
* If a test currently asserts the title is shown,
  flip the assertion.

### Gate

* `npm run check`
* `npm run test`
* `npm run build`
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`

## Notes

* This is a small, low-risk visual change in
  active release-prep. The cost: `webtest-b-6`
  item 6 (multi-FB tabs walkthrough) needs a small
  re-walk on the FB chrome once this lands — fine
  if you ship promptly; flag if you hit anything
  unexpected.
* Visual eyeball worth doing here — ad-hoc
  `chan serve` + Chrome MCP for two minutes to
  confirm all three variants read cleanly without
  the path bar. Teardown after.
* Standing topic-level commit clearance.

## 2026-05-19 18:30 BST — implementation

**Approach:** chose the "drop span in all variants,
slim the chrome row" path (the matrix's permitted
alternative). Reason: the file-browser hamburger
carries FB-specific menu items (toggle inspector,
new file here, new dir here, search this, reload,
etc.) that are NOT on the Pane tab-strip's kebab —
dropping the whole header in tab variant would
require wiring those items onto the tab-strip
kebab, which is bigger scope than this task and
risks regressing the dock + overlay paths that
already work. A slim chrome strip with just the
hamburger preserves access at ~38px tall, which is
clearly chrome rather than a path-display row.

**Edits:**

* `web/src/components/FileBrowserSurface.svelte`
  * Removed import:
    `import { fileBrowserTitlePath } from "../terminal/fromHere";`
  * Removed derived: `browserTitle` (lines 75-77).
  * Removed:
    `<span class="name" title={browserTitle}>{browserTitle}</span>`
    at line 312.
  * Replaced with: `<span class="header-spacer" aria-hidden="true"></span>`
    so the hamburger stays right-anchored.
  * Renamed `.name { flex: 1; ... }` CSS rule to
    `.header-spacer { flex: 1; }`; dropped the
    overflow / ellipsis declarations that only
    mattered for the text node.

* `web/src/terminal/fromHere.ts`
  * Removed the `fileBrowserTitlePath` export. The
    function had only one caller and that caller
    is now gone.

* `web/src/terminal/fromHere.test.ts`
  * Removed `describe("fileBrowserTitlePath", …)`
    block + the import of the helper.
  * `terminalFromHereTarget` + `shellQuotePath`
    describe blocks unchanged.

* `web/src/components/revealBrowserActions.test.ts`
  * Added `describe("fullstack-54: no path-display
    header on FileBrowserSurface", …)` with two
    assertions on the imported `?raw` source:
    `class="name"` absent and the
    `fileBrowserTitlePath` / `browserTitle`
    identifiers absent. Same pattern as the
    surrounding `fullstack-38` and "no inline close
    affordance" sentinel tests.

**Audit (`fileBrowserTitlePath` / `browserTitle`
references after the change):**

* `web/src/components/FileBrowserSurface.svelte` —
  none (just verified via `grep`).
* `web/src/terminal/fromHere.ts` — none.
* `web/src/terminal/fromHere.test.ts` — none.
* `web/src/components/revealBrowserActions.test.ts`
  — the absence is what the new sentinel test
  asserts (string-literal arguments to `not.toContain`).

**Visual eyeball.** Ad-hoc chan serve on a
throwaway drive at `/tmp/chan-test-fullstack-54`,
new Chrome MCP tab (no impact on the persistent
webtest tabs at 8801 / 8810). Verified:

* **Tab variant** (URL hash `{k:l, t:[{k:b, a:1}],
  f:1}`): `.browser > header` outerHTML reads
  `<header><!----> <span class="header-spacer">
  </span> <!----><button …>⋮</button> …</header>`.
  `headerText.trim() === "⋮"` — only the kebab
  glyph, no path text. Body topmost child is
  `.tree-wrap`.
* **Dock variant** (clicked "Stick to left" from
  the hamburger menu): `.browser.dock header` height
  = 37.8px. Content = unstick button + spacer +
  hamburger. `headerText.trim() === "⋮"`. Slim
  chrome strip, no orphan padding.
* **Overlay variant** not separately repro'd; it
  uses the same `<header>` block as dock (with the
  `Maximize2` button in place of the unstick
  arrow), so the result is symmetric by
  construction.
* Hamburger menu items still reachable from the
  slim chrome strip in tab variant — confirmed 12
  visible items including "Stick to left", "Show
  Details", "New file", "New directory", etc.

**Teardown done.** chan serve killed
(PID 268), `/tmp/chan-test-fullstack-54` drive
unregistered + rm'd, Chrome MCP tab closed. The
webtest persistent tabs (`127.0.0.1:8801`,
`127.0.0.1:8810`) untouched.

**Gate.** `npm run check` 0/0; `npm run test`
35 files / 343 tests (net 0 — dropped two
`fileBrowserTitlePath` tests, added two source-grep
sentinel tests); `npm run build` clean;
`scripts/pre-push` green.

**Re-walk cost flag.** `webtest-b-6` item 6 (the
multi-FB walkthrough) needs to refresh the FB
chrome screenshots — the path bar is gone from all
three variants and the chrome row is slimmer.
Architect to forward the re-walk note to
@@WebtestB when they pick this up.

**Commit readiness.**

* Files staged:
  * `web/src/components/FileBrowserSurface.svelte`
    (the change).
  * `web/src/terminal/fromHere.ts` (helper drop).
  * `web/src/terminal/fromHere.test.ts` (helper
    test drop).
  * `web/src/components/revealBrowserActions.test.ts`
    (sentinel test added).
  * This task file.
  * `docs/journals/phase-7/fullstack-b/journal.md`
    (append).
  * `docs/journals/phase-7/alex/event-fullstack-b-architect.md`
    (event append).
* Proposed commit message:
  ```
  Drop FileBrowserSurface path-display header (fullstack-54)
  ```
* Standing topic-level commit clearance applies.
  No HOLD pokes since the 15:35 BST cut.
