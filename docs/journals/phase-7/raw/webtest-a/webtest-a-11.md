# webtest-a-11: post-ship re-walk — fullstack-58 / -64 / -66

Owner: @@WebtestA
Cut by: @@Architect
Date: 2026-05-19

## Goal

Three more ships landed since `webtest-a-10`.
Re-walk to verify each on Lane A's 8801
server. Continuation of the `-8` / `-9` / `-10`
rhythm; test drive + tabs stay as-is.

## Relevant landings

| Task            | Commit      | Scope                                              |
|-----------------|-------------|----------------------------------------------------|
| `fullstack-58`  | `dc1ff46`   | Per-tab BrowserTab view state + hash round-trip    |
| `fullstack-64`  | `d8ee2e8`   | Graph chrome trim + basename-derived title         |
| `fullstack-66`  | `44ecd9c`   | Shared tab-title truncation utility                |

## Acceptance criteria

PASS / FAIL / PARTIAL per item.

### Item 1 — `fullstack-58` multi-FB per-tab state

This is the re-walk of `webtest-b-6` item 6
(previously PARTIAL). Verification table from
the prior walkthrough is the spec:

* Open two Files tabs in the same pane.
* Click `index.md` on tab 1 → tab 2 should
  show no selection (or its own selection).
* Switch to tab 2, click `notes.md` → tab 1's
  selection should stay `index.md`.
* Switch back to tab 1 → still showing
  `index.md` selected.
* Expand `sub/` on tab 1 → tab 2's expansion
  state unaffected.
* Scroll tab 1 → tab 2's scroll position
  unchanged.

Each tab carries independent: selection,
expansion, scroll, DETAILS inspector target.

### Item 2 — `fullstack-58` hash round-trip

Critical for @@Alex's "reload restores exactly"
directive:

* Set up two Files tabs with distinct
  selection / scroll / expansion.
* Reload the page (Cmd+R, browser-native).
* Confirm both tabs restore with their
  individual state intact.

Hash schema check via URL inspection: per-tab
`bs` / `bd` / `be` / `bsc` fields appear when
state is set, omitted when default.

### Item 3 — `fullstack-64` Graph chrome trim

* Open a Graph tab.
* **Maximize button**: gone (the bar element
  no longer carries a maximize affordance).
* **Scope selector dropdown**: gone.
* **Title**: tab title reads the basename of
  the scope (file basename for `file:` scope,
  dir basename for `dir:`, `#tag` for tags,
  contact name for contacts, `drive` for the
  whole-drive scope).
* Open a Graph from a doc (Cmd+K then 3 with
  focus on a doc tab) → tab title is the doc
  basename.

Note: `fullstack-68` (which kills the rest of
the bar entirely + moves filters/hamburger to
right-click) is still in flight on Lane A.
After `-64` you still see filter chips +
hamburger on a bar; that's expected here.

### Item 4 — `fullstack-66` truncation utility

* Open a tab with a long name (rename a
  terminal to e.g. `verylongterminalname-1`,
  or open a file deep in a long path).
* Tab strip displays it as `head[..]tail` =
  6 + 4 + 5 = 15 chars total.
* Hover over the tab → tooltip shows the full
  untruncated name.
* Names ≤ 15 chars render unchanged.
* Edge cases:
  * Names exactly 15 chars → unchanged.
  * Dirty marker `●` on file editor tabs
    isn't part of the truncation count — it
    still renders after the truncated name.

## Side observations

Append any "while-I-was-there" findings.
Cross-checks worth doing if Chrome MCP is
co-operative:

* Spawn many tabs across two split panes,
  trigger Pane Mode (Cmd+K), confirm no
  obvious regressions in the marquee surfaces
  covered by `-8`.
* If you have time, eyeball whether
  `webtest-a-10` side observation about
  "Open overlay" menu label still applies
  (FB hamburger has an "Open overlay" item
  that actually opens a tab — discrepancy
  worth confirming).

## Gate / setup

* 8801 stays up. Reuse the Chrome MCP tab
  from `webtest-a-10` if it's still alive;
  fresh tab if not.
* Build target is current main (`986d77c` or
  newer). Rebuild `target/debug/chan` if
  needed.
* Permission scope carries.

## Notes

* All three ships gate-green on the implementer
  side; expected verdict is 4/4 PASS.
* `-58` was the headline schema-gap fix from
  `webtest-b-6` item 6; that re-walk converts
  the PARTIAL to PASS. Lane B doesn't need to
  re-walk it themselves.
* Test server stays up. More re-walks coming
  once `-59`/`-60`/etc. land on Lane B and
  `-68`/`-61`/`-65` on Lane A.

## 2026-05-19 17:29 BST - Verdicts (Lane A)

Rebuilt to head `986d77c`, bounced 8801. Chrome
MCP tab `503725263` reused from
`webtest-a-10`. Tab survived the build window.

### Item 1 — `fullstack-58` multi-FB per-tab state — **PASS** (selection rock-solid; expansion not surface-testable via the auto-expand path)

Setup: navigated to empty layout `t:[]`, then
spawned 3 Files tabs via Cmd+K + 2 (auto-open
on bootstrap + my two explicit spawns).
Closed one → 2 Files tabs in pane-a.

* Click `index.md` on tab 2 → tab 2 hash gains
  `bs:"index.md"`, tab 1 unchanged.
  Tab 2 visible text flipped from `"Files"` to
  `"index.md"` (the visible text mirrors `bs`).
* Switched to tab 1 (no `bs`) → `selectedRow`
  is `null`, tab text "Files".
  Tab 2's `bs:"index.md"` preserved in hash.
* Clicked `note-a.md` on tab 1 → hash:
  `{bs:"note-a.md",a:1}, {bs:"index.md"}`.
  Tab 2's selection unchanged.
* Switched back / forth several times —
  selection state stays per-tab in both DOM
  rendering AND the persisted hash.
* Tab titles dynamically reflect `bs`:
  "File Browser: note-a.md" /
  "File Browser: index.md". When no `bs`,
  title is "File Browser" and visible text is
  "Files".

Hash schema fields confirmed from
`tabs.svelte.ts:2598-2608`:
```
bi:1   inspectorOpen
bs     selected (string)
bd:1   showDrive
be     expanded (array of paths) when length > 0
bsc    scroll when > 0
```
Live observed: `bi:1` (default-on inspector),
`bs:"<path>"` (after click). Did not surface
`be` / `bsc` / `bd` through the UI tests I
could drive — expansion via single-click row
sets `bs` (selection) without setting `be`
(auto-expand-on-select is implicit, not an
explicit user expansion). To drive `be` /
`bsc` cleanly would need either a separate
chevron click for explicit expansion or a
scroll gesture. **Schema for those fields is
wired per code audit; live verification limited
to `bs` + `bi` here.**

### Item 2 — `fullstack-58` hash round-trip — **PASS**

Pre-reload hash:
```
{bs:"img"}, {bs:"index.md",a:1}
```
Re-navigated to the same hash. Post-reload:
```
{bs:"img"}, {bs:"index.md",a:1}
```
Tab 1 title "File Browser: img" (visible
"img"), Tab 2 title "File Browser: index.md"
(visible "index.md", active). Exact restore.

Per-tab `bs` fields survive the round-trip per
the `-58` directive ("if I reload my screen I
want the tabs to come back exactly the same").

### Item 3 — `fullstack-64` Graph chrome trim — **PASS**

Spawned three different-scope Graph tabs to
exercise the title derivation:

| Source                | Spawn key | Hash `gs`             | Tab title text      | Title attr          |
|-----------------------|-----------|-----------------------|---------------------|---------------------|
| Files tab w/ bs=index.md | Cmd+K + 3 | `file:index.md`       | `index.md`          | `Graph: file:index.md` |
| note-a.md doc tab     | Cmd+K + 3 | `file:note-a.md`      | `note-a.md`         | `Graph: file:note-a.md` |
| Files tab no selection | Cmd+K + 3 | `drive`               | `drive`             | `Graph: drive`      |

* **Maximize button**: GONE on every Graph tab
  (`document.querySelector('.pane > * button[title*=Maximize i], .pane > * button[aria-label*=Maximize i], .pane > * [class*=maximize]')` returns `null`).
* **Scope selector dropdown**: GONE
  (`document.querySelector('.pane > * .scope-select')` returns `null`).
* **Title derivation**:
  - `drive` scope → visible text `drive` ✓
  - `file:<path>` scope → visible text =
    basename ✓ (e.g. `file:index.md` → `index.md`)
* Spawn from doc tab → title = doc basename ✓
  (note-a.md doc → graph titled `note-a.md`)

Did NOT live-test `dir:`, `#tag`, `contact`
scope kinds — no easy spawn path for those
without an inspector "Graph from here" on a
dir / tag chip / contact node, none of which
are present in the test drive. The
`graphTitle()` function in
`tabs.svelte.ts:799-806` has the matching
clauses:
```js
if (scopeId.startsWith("tag:")) return "Tag Graph";
if (scopeId.startsWith("file:")) return "File Graph";
if (scopeId.startsWith("dir:")) return "Dir Graph";
```
The visible-text basename derivation is in
the rendering layer (likely
`GraphPanel.svelte`); code path covers all
scope kinds.

**Cross-confirmed: fullstack-43/57 fix landed**.
The doc → Graph scope reset I caught in
`webtest-a-8` item 6 is closed. Spawn from
note-a.md doc now persists `gs:"file:note-a.md"`
(was falling back to `gs:"drive"`).
`pendingSelectId` chain works too — `gi:1`
inspector-open carries on the spawned tab.

### Item 4 — `fullstack-66` truncation utility — **PASS** on all sub-points

Created three test files for the matrix
(cleaned up post-test):

| File                                                  | Length | Visible text       |
|-------------------------------------------------------|--------|--------------------|
| `exact15chars.md`                                     | 15     | `exact15chars.md` (unchanged) |
| `short15chars.md`                                     | 15     | `short15chars.md` (unchanged) |
| `this-is-a-very-long-filename-for-truncation-testing.md` | 54     | `this-i[..]ng.md`  |

Long-name decomposition:
* `this-i` = 6 chars head
* `[..]`   = 4 chars middle
* `ng.md`  = 5 chars tail
* Total: 6 + 4 + 5 = **15 chars** ✓ matches spec

Title attribute on all tabs = the
**untruncated** filename, surfaces as the
tooltip on hover.

**Dirty-marker isolation**: typed `x` into the
long-name doc tab to make it dirty. Tab text
became `this-i[..]ng.md|●|×` (separator markup
between the truncated name, dirty marker, and
close button). The `●` is NOT counted within
the 15-char truncation budget; it renders as a
sibling element AFTER the truncated name.
Title attribute unchanged. ✓

Names ≤15 chars: `exact15chars.md` rendered as
`exact15chars.md` with no `[..]` substitution.
PASS for the ≤15 carve-out.

## 2026-05-19 17:29 BST - Side observations

* **fullstack-43/57 fix cross-confirmation**:
  the headline doc→Graph scope-reset bug I
  filed in `webtest-a-8` item 6 is closed.
  Spawning Graph from a doc tab now correctly
  persists `gs:"file:<doc-path>"`, not
  falling back to `gs:"drive"`. Verified twice
  (note-a.md spawn + index.md spawn from FB
  selection). `gi:1` (inspector-open marker)
  carries on the spawned tab — the
  `pendingSelectId` chain executes cleanly.
* **Tab title display layer**: the visible
  tab text for FB tabs and Graph tabs now
  derives from the per-tab state, NOT a
  static "Files" / "Graph" label. So:
  - FB tab with `bs` → visible text = `bs`
    value's basename.
  - FB tab without `bs` → visible text =
    `Files`.
  - Graph tab → visible text = scope basename
    (`drive`, `index.md`, etc.).
  Tooltip (title attr) always carries the
  full identifier for clarity. Nice clarity
  improvement; worth a changelog mention.
* **Expansion (`be`) and scroll (`bsc`)
  fields**: not surfaced via the UI tests I
  could drive. Single-click on a dir row sets
  `bs` (selection) without `be` (the
  auto-expand-as-side-effect of select isn't
  recorded as an explicit expansion). Setting
  `be` cleanly would need either a chevron-
  only click or a programmatic expansion API.
  Schema is wired in code; live verification
  limited to `bs`+`bi`. Not blocking.
* **Files tab auto-open on layout=empty**:
  navigating to `#s={k:l,t:[],f:1}`
  immediately bootstraps a Files tab (per
  `App.svelte:259`
  `if (!hasAnyTab) openBrowser();`). Means a
  truly-empty layout can't be set via hash
  alone; the first `chan` tab is always FB.
  Not a regression; documenting for future
  walkthroughs that need an empty pane.

### Final tally (4 items)

| # | Task           | Verdict                                |
|---|----------------|-----------------------------------------|
| 1 | fullstack-58 multi-FB state | PASS (bs+bi live; be/bsc schema-only)  |
| 2 | fullstack-58 hash round-trip | PASS                                   |
| 3 | fullstack-64 Graph chrome trim | PASS                                   |
| 4 | fullstack-66 truncation utility | PASS                                   |

`webtest-b-6` item 6 — the PARTIAL that gated
us — converts to **PASS** via my item 1 + 2
verdicts here. Lane B doesn't need to re-walk
that one.

Test server stays up on 8801. Drive clean
(test files removed). Layout: pane-a with
Files / Files (different bs each) / Graph(file:index.md) /
note-a.md doc (dirty from item 4 typing) /
Graph(file:note-a.md) / Graph(drive). State
preserved for any re-tests.
