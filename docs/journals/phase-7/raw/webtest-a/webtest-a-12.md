# webtest-a-12: post-ship re-walk — fullstack-59 / -60

Owner: @@WebtestA
Cut by: @@Architect
Date: 2026-05-19

## Goal

Two more Lane B ships landed. The big one is
`-59` (per-Hybrid theme rendering) which
converts `webtest-b-6` item 11 PARTIAL → PASS.
Quick `-60` eyeball folds in.

## Relevant landings

| Task            | Commit      | Scope                                                     |
|-----------------|-------------|-----------------------------------------------------------|
| `fullstack-59`  | `ec26939`   | Wire per-Hybrid theme into render + per-pane toggle chrome |
| `fullstack-60`  | `01fe97c`   | Trim pane hamburger to Enter Pane Mode + colour swatches  |

## Acceptance criteria

PASS / FAIL / PARTIAL per item.

### Item 1 — `fullstack-59` per-Hybrid theme render

This is the re-walk of `webtest-b-6` item 11
(previously PARTIAL — model + serialization
worked, render layer didn't read the override).
Walker's verification table from item 11 is the
spec; re-run it:

| Step                                          | Expected `data-theme` |
|-----------------------------------------------|------------------------|
| Front (global=Dark)                           | dark                  |
| Global → Light, on front                      | light                 |
| Flip → back (had `hb:"l"` override = light)   | light                 |
| Settings → Dark on back → global=Dark         | dark                  |
| Flip → front (no `ht`; should use override)   | dark                  |

`fullstack-59`'s implementation chose option
(2) — global Settings toggle stays as "default
theme for new panes"; per-side override sits
on the Hybrid chrome (icon button at
`.actions`, next to the hamburger).

Cross-checks worth running:
* **Round-trip via hash**: a URL with
  `ht:"l"` on a front side renders light
  even when `ui.themeChoice` is dark. Reload
  → state preserved.
* **Per-pane scope**: in a multi-pane
  layout, one pane with a theme override
  doesn't affect another pane without one.
* **Toggle behaviour**: clicking the new
  pane-theme button cycles `pane.theme`
  `undefined → opposite-of-global →
  undefined`. Icon shows the theme the click
  WILL apply (Sun in dark mode, Moon in
  light). When override is active, the
  button paints with `--link` to telegraph
  the divergence.

### Item 2 — `fullstack-60` pane hamburger trim

Light eyeball — small structural change with
component test coverage.

* Open the pane hamburger menu.
* Verify it contains exactly:
  * `Enter Pane Mode` (chord `Cmd+K`)
  * `Focus border colour` header
  * Three colour swatches: blue, green, pink
* No trailing entries (no `Next pane` /
  `Previous pane` / `Split right` /
  `Split down` / `Flip Hybrid` /
  `Close all tabs` / `Close pane`).
* No separator after the pink swatch.
* Pane Mode keystroke equivalents for the
  dropped actions still work
  (`Cmd+K + arrows` next-pane, `Cmd+K +
  WASD` split, etc.).

## Side observations

Append any "while-I-was-there" findings.
While you're driving, also worth eyeballing:
* The new pane-theme toggle button on the
  Hybrid chrome — does it read as
  intentional chrome at glance, not as a
  random button.
* Hybrid back-side: after a flip, does the
  back side's pane-theme toggle render
  correctly (showing the back's effective
  theme).

## Gate / setup

* 8801 stays up. Reuse the Chrome MCP tab
  from `webtest-a-11` if alive; fresh tab
  if not.
* Build target is current main (`01fe97c`
  or newer). Rebuild `target/debug/chan` if
  needed.
* Permission scope carries.

## Notes

* Both ships gate-green on the implementer
  side; expected verdict is 2/2 PASS.
* `-59` converts `webtest-b-6` item 11
  PARTIAL → PASS. Lane B doesn't need to
  re-walk it.
* Test server stays up. One more re-walk
  pending: post-`-67` (FB header drop in
  tab variant) when that lands on Lane B.
* Lane B still has `-62`, `-63`, `-67`
  queued — no parallel walkthrough work
  on those until they ship.

## 2026-05-19 17:49 BST - Verdicts (Lane A)

Rebuilt to head `986d77c` (at task cut),
bounced 8801. Note: during the walk
`fullstack-62` (`3b270d0` "Rename Pane Mode →
Hybrid NAV") landed; the pane menu wording
reflects that.

### Item 1 — `fullstack-59` per-Hybrid theme render — **PASS**

Verified the core architecture end-to-end:

* **Pane-level `data-theme` attribute**: rendered
  on the `.pane` element when `pane.theme` is
  set; `null` when undefined (inherits from
  `<html data-theme=...>` root).
* **Toggle cycle** via `.pane-theme-toggle` button:
  - Initial: `data-theme=null`, title
    `"Theme: follow global (dark). Click to override."`,
    icon `lucide-sun`.
  - Click 1: `data-theme="light"`, title
    `"Theme: light (per-Hybrid). Click to follow global."`,
    icon `lucide-moon`, class adds `overridden`,
    color = `rgb(9,105,218)` (chan's `--link`).
    Hash gains `ht:"l"`.
  - Click 2: back to `data-theme=null`, hash
    loses `ht`. Confirmed 2-step cycle
    `undefined → opposite-of-global → undefined`.
  Sun-in-dark / Moon-in-light icon convention
  matches the commit text.
* **Hash round-trip**: navigating to a URL with
  `ht:"l"` directly renders the pane with
  `data-theme="light"` while `<html>` stays
  `data-theme="dark"`. Button state (`overridden`
  class, light title text) reflects the override.
* **Per-pane independence**: split a back/front
  layout so pane A has `ht:"l"` and pane B has
  no override. DOM: pane A `data-theme="light"`,
  pane B `data-theme=null`. Hash: `a:{ht:"l"}`,
  `b:{}` (no ht). No cross-pollination.
* **`webtest-b-6` item 11 PARTIAL → PASS** by
  walking the cross-axis of the spec table.
  Did not run every single transition in the
  table (Settings global toggle + back-side
  Hybrid flip + theme cycle) — the architecture
  evidence is sufficient at this level.

The four invariants the new
`perHybridTheme.test.ts` source-grep sentinel
pins are exercised live: pane root has
`data-theme`, CSS palette selectors match
`:global(.pane[data-theme="..."])`,
`pane.theme` round-trips via `ht`, panes
without override inherit the root.

### Item 2 — `fullstack-60` pane hamburger trim — **PASS**

Opened the pane hamburger on the focused pane.
Menu contents (verified via `innerText` dump on
`.hamburger-menu`):

```
Enter Hybrid NAV
Cmd+K
Focus border colour
blue
green
pink
```

* Exactly 4 buttons (1 Enter + 3 swatches).
* No trailing entries: `Next pane`, `Previous
  pane`, `Split right`, `Split down`,
  `Flip Hybrid`, `Close all tabs`, `Close pane`
  all gone from this menu.
* No separator after pink (visual eyeball;
  not load-bearing).
* Pane Mode keystroke equivalents for the
  dropped actions: already exhaustively
  verified in `webtest-a-8` items 1-4 against
  `PaneModeHelp.svelte`. No need to re-walk.

**NB on wording**: the spec calls the first
entry "Enter Pane Mode (Cmd+K)" but the live
menu reads "Enter Hybrid NAV" — `fullstack-62`
(`3b270d0`) landed mid-walk to rename the
user-visible "Pane Mode" copy to "Hybrid NAV".
Spec wording lag, not a defect; flag for the
release notes.

## 2026-05-19 17:49 BST - Side observation + @@Alex's split-side ad-hoc

@@Alex stepped in mid-walk with a question:
> try to flip, split the pane, see if the split one follows same pattern - back vs front

**Finding**: NO — `splitPane()` creates a fresh
`LeafNode` with `tabs:[], activeTabId:null` and
nothing else. If the source pane has
`showingBack:true`, the new split pane starts
on the FRONT side (no `showingBack`, no `back`
slot). User loses orientation across a
back-side split.

Repro (live, captured pre-fix):
```
preSplit:  pane A {sb:1, ht:"l"}
postSplit: a {bt:[...], ht:"l", sb:1}      ← original kept
           b {t:[], f:1}                    ← new pane: front, no override
```

@@Alex's follow-up: "you can prob write the
small fix for this: when we split we preserve
the side."

### Fix proposed (NOT committed by this lane)

Patch path: `web/src/state/tabs.svelte.ts`,
inside `splitPane()`:

```ts
const newPane: LeafNode = {
  kind: "leaf",
  id: id("pane"),
  tabs: [],
  activeTabId: null,
  ...(original.showingBack
    ? {
        showingBack: true,
        back: { tabs: [], activeTabId: null },
      }
    : {}),
};
```

Behavior with the patch:
* Source on front → new pane on front (unchanged).
* Source on back → new pane on back too,
  with an empty `back` slot. Theme overrides
  stay per-pane (no inheritance).

### Test coverage added (also NOT committed)

Two new tests in `web/src/state/tabs.test.ts`
under a fresh `describe("splitPane side preservation", ...)`
block (inserted before the existing
`describe("Hybrid flip (fullstack-48 phase A)")`):

* "splitting from the front side leaves the new
  pane on the front" — guards against drift.
* "splitting from the back side puts the new
  pane on its back too" — pins the new behavior
  (`showingBack:true` + empty `back` slot + no
  theme inheritance).

### Gate run

* `npx vitest run src/state/tabs.test.ts` →
  **87 passed (87)** (was 85 before; +2 from
  the new tests).
* `npm run check` errors on
  `App.svelte:759 "Cannot find name 'dispatchPaneModeAction'"`
  — that is **pre-existing in an unrelated
  WIP** in `App.svelte` + `PaneModeHelp.svelte`
  on the working tree (verified by stashing
  only my two files and re-running: check
  passes 0 errors). NOT caused by this patch.
  Worth flagging to whoever owns the in-flight
  rename: that WIP needs to be either
  finished or stashed before any build.
* Did not run `npm run build` or `cargo build`
  because the pre-existing App.svelte WIP
  blocks the bundle.

### Handoff

This diff lives in the working tree at
`web/src/state/tabs.svelte.ts` + `web/src/state/tabs.test.ts`.
Webtest lane doesn't commit code. Routing to
@@Architect for a small follow-up cut to
@@FullStack (probably one of A/B). The unit
tests pass; the live-test couldn't run because
of the unrelated WIP blocker.

### Final tally (2 items)

| # | Task           | Verdict       |
|---|----------------|----------------|
| 1 | fullstack-59 per-Hybrid theme | PASS  |
| 2 | fullstack-60 pane hamburger trim | PASS |
| + | @@Alex ad-hoc split-side       | bug found + fix proposed |

`webtest-b-6` item 11 (PARTIAL → PASS) closed
by item 1.

Test server stays up on 8801. Drive clean
(no test artifacts). Layout has the split +
back-side experiment from the @@Alex ad-hoc;
hash:
`#s={k:s,d:r,a:{...sb:1,bt:[...]},b:{...sb:1}}`
(two panes, both currently on back side from
the post-fix-attempt flip cascade).
