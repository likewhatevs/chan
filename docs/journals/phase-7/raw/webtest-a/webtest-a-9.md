# webtest-a-9: Pre-release walkthrough — Lane B overflow (3 items)

Owner: @@WebtestA
Cut by: @@Architect
Date: 2026-05-19

## Why

@@Alex flagged @@WebtestB is hitting overload
errors slowing their pace. Lane B has 8 items
still pending; redistributing 3 independent items
to Lane A to keep the v0.11.0 wait list moving.
Lane A's 8801 server stays up from `webtest-a-8`;
this is a continuation of that session, not a
fresh boot.

## Scope

The Hybrid-flip cluster (Lane B items 9-12) stays
on Lane B — verifies in sequence, splitting the
cluster costs more than it saves. The three
moving items are self-contained:

| Item | Task            | Commits / scope                                                                 |
|------|-----------------|---------------------------------------------------------------------------------|
|  1   | `fullstack-47`  | Multiple Graph tabs — independent per-tab state, different scopes               |
|  2   | `fullstack-47`  | Tab DnD across panes — Chrome MCP HTML5 DnD limit may bite; INCONCLUSIVE OK     |
|  3   | `fullstack-51`  | xterm row metrics — `claude` / block-character output rendered without row gap  |

Renumbered locally here as items 1-3 for tally
clarity; cross-reference Lane B's `webtest-b-6`
items 7 / 8 / 13 in the verdict.

## Acceptance criteria

PASS / FAIL / PARTIAL / INCONCLUSIVE per item.

### Item 1 — `fullstack-47` multiple Graph tabs

Cross-references `webtest-b-6` item 7.

* Open two Graph tabs in the same pane. Set
  different scopes if possible (e.g. drive root
  on one, a specific dir on the other via
  "Graph from here" inspector button).
* Verify each tab carries its own inspector
  state, hover state, zoom / pan / position
  if applicable.
* Switch between them; confirm state doesn't
  bleed.

### Item 2 — `fullstack-47` tab DnD across panes

Cross-references `webtest-b-6` item 8.

* Drag a tab from one pane to another and drop
  it. Should reparent cleanly.
* **Chrome MCP limit**: `computer.left_click_drag`
  produces pointer events, not HTML5 DnD. The
  SPA's `ondragstart` / `ondrop` chain may not
  fire. If that's what you observe, mark
  **INCONCLUSIVE** with the tool-limit note
  (same pattern as `fullstack-15` on Lane B's
  prior pass). Don't burn cycles trying to
  bridge.

### Item 3 — `fullstack-51` xterm row metrics

Cross-references `webtest-b-6` item 13.

* Open a terminal tab. Run a program that
  renders block-character / ASCII-art output.
  `claude` is the canonical reference per the
  task spec, but anything with vertical block
  glyphs works (`figlet "X"` if available,
  `printf '███\n%.0s' {1..5}`
  if not).
* Verify consecutive block rows have **zero
  pixel gap** between them. Match iTerm's row
  metrics. `0b0c919` set `lineHeight: 1.0` on
  the xterm.js options.
* Eyeball-only is fine; comparison against a
  pre-`0b0c919` build is optional.

## Side observations

Same convention as `webtest-a-8` — append any
"while-I-was-there" findings.

## Gate / setup

* Lane A's 8801 server stays up from
  `webtest-a-8`. No fresh `chan serve` needed
  unless the existing one died (re-boot via
  `target/debug/chan` if so).
* Drive into the SPA via Chrome MCP tab
  `503725239` from the prior session, or open
  a fresh one if that died.
* Permission scope carries.

## Notes

* This is overflow help, not lane reassignment.
  Lane B retains `webtest-b-6` items 1-6 + 9-12.
  Once Lane A verdicts land here AND Lane B
  verdicts close on items 9-12 over there, the
  combined report covers all 13 original items.
* Test server stays up after the walk.

## 2026-05-19 16:09 BST - Verdicts (Lane A overflow)

Continuation of `webtest-a-8` session. 8801 still up
on `cd4ad26`; tab `503725239` reused. No fresh
spawn/build.

### Item 1 — `fullstack-47` multiple Graph tabs — **PASS**

Setup: switched focus to pane-b (Terminal-4) and
spawned 2 Graph tabs via Cmd+K + 3 twice. Layout
after: `b:{t:[Terminal-4, Graph#1, Graph#2]}`, both
graphs initially `gs:drive, gf:ltmaif`.

Differentiated state via filter chips:
* On Graph #2 (active), clicked the `folder` chip
  → toggled off. Persisted hash updated to
  `{gs:drive, gf:ltmai}` for tab index 2 only;
  tab index 1 stayed `{gs:drive, gf:ltmaif}`.
* Switched to Graph #1 via real-coordinate click
  (`computer.left_click` on the tab strip rect from
  the focused pane). Verified: chips dump showed
  folder ON, link/tag/contact/language/media all
  ON, matching `gf:ltmaif`.
* Switched back to Graph #2. Verified: folder
  chip OFF; other chips ON; gf still `ltmai`.

Both tabs render in pane-b alongside Terminal-4
with no state bleed. The `fullstack-47` dedup
drop is observable end-to-end:
* spawn doesn't fold into an existing Graph
  (got 2 distinct tabs from 2 spawn keystrokes);
* per-tab filter state persists across tab
  switches.

Side note on tool friction: stale element refs
from `find` didn't switch active tabs reliably
after layout changes. Worked around by computing
`getBoundingClientRect()` on the live DOM and
clicking real coordinates via
`computer.left_click`. Worth flagging for future
Chrome MCP walkthroughs.

### Item 2 — `fullstack-47` tab DnD across panes — **INCONCLUSIVE (live) / PASS (code+tests)**

Live: dragged Terminal-1 tab (pane-a, x=128) →
pane-b tab strip (x=848) via
`computer.left_click_drag`. Result: pane-a still
holds 8 tabs, pane-b still holds 3 (Terminal-4 +
2 Graphs). Hash unchanged. As the task spec
anticipated, Chrome MCP's left_click_drag fires
pointer events, not the HTML5 DnD chain
(`dragstart` / `dragover` / `drop`), so the SPA's
DnD handlers never fire and the move doesn't
commit.

Code audit: `da2d718` (`fullstack-47`) ships a
unit test
`"detachTabToPaneEdge moves a browser or graph tab end-to-end"`
that locks the cross-pane DnD path for non-file
tab kinds (the commit message calls out that the
dedup branches previously made it untested for
browser/graph tabs). Combined with item 1 PASS
(stacking works) the DnD machinery is verified
at unit-test level. Same pattern as
`fullstack-15`'s prior Lane A verdict (live MCP
limit, code+tests cover it).

### Item 3 — `fullstack-51` xterm row metrics — **PASS**

Unstuck both docks first (left + right FB) to
give pane-b full width; Terminal-4 reached normal
columns. Wrote
`.test-blocks.sh` (8 rows of `██████████████████████`)
into the drive root, ran `bash .test-blocks.sh`,
cleaned up post-test.

Measurement on the 8 rendered block rows in
`.xterm-screen`:
| Row | y (px) | height | bottom |
|-----|--------|--------|--------|
|  1  |  46    | 15     |  61    |
|  2  |  61    | 15     |  76    |
|  3  |  76    | 15     |  91    |
|  4  |  91    | 15     | 106    |
|  5  | 106    | 15     | 121    |
|  6  | 121    | 15     | 136    |
|  7  | 136    | 15     | 151    |
|  8  | 151    | 15     | 166    |

Gap = `row[n].y - row[n-1].bottom` = **0px** for
every consecutive pair (7/7).

Code audit confirms `0b0c919` flips xterm.js
`lineHeight: 1.15` → `1.0`. Empirical 15px row
height for 13px SFMono matches the intrinsic
font cell (no 1.15x leading added by xterm.js).
Row containers stack flush — matches iTerm's
zero-leading metric.

Visual note: the JPEG screenshot shows faint
horizontal seams between block rows. The DOM
measurements rule out cell-leading (the divs are
flush), so any residual seam is from SFMono's
own U+2588 glyph not painting absolutely
edge-to-edge in the font, not from xterm.js. The
fullstack-51 fix is doing what it claims at the
xterm.js layer.

## 2026-05-19 16:09 BST - Side observations

* **Chrome MCP `find` ref staleness on tab
  switches**: refs returned by `find` for tab
  bar items go stale after a layout-touching
  click (e.g. switching to another tab in the
  same pane invalidates refs to other tabs). The
  click registers visually but the target tab
  doesn't activate. Workaround used here:
  re-query the DOM via JS, compute live
  `getBoundingClientRect()`, click those raw
  coordinates with `computer.left_click`. Worth
  noting in the orchestration SKILL for future
  walkthroughs.
* **`computer.type` into xterm interleaves
  characters on narrow terminals**: typing the
  block-print shell command directly into a
  ~25-col terminal produced reordered output
  (semicolons + quotes scrambling the shell
  parse). Worked around by dropping the command
  into a shell script on disk and running
  `bash <script>`. Trigger seems to be a
  combination of typing speed + narrow xterm
  reflow. Same root surface in Lane A as
  Lane B's typing limitations.
* **No-shell-session terminals are typing
  no-ops**: same observation from `webtest-a-8`:
  Terminal-4 spawned via Cmd+K + p but no
  session id yet → typing into it produces no
  shell output (PTY isn't attached). Once you
  click into the xterm body and start typing,
  the WS handshake initialises the session.
  The first few characters typed before
  handshake completion can be lost. Not strictly
  a bug — just an edge worth surfacing.
* **Lane B coordination memory note**: per the
  redistribution-tail rule (`feedback_redistribution_queue_head.md`),
  redistributed items skip Lane B's next-up
  item. My item 1 (Graph stacking) IS Lane B
  item 7; the architect's table mapped 7/8/13 →
  1/2/3. Trusted the cut, didn't double-walk;
  if @@WebtestB was mid-walk on 7 in parallel,
  the duplicate-verdict is moot since my PASS
  matches the spec.

### Final tally (3 items)

| # | Task           | Lane B # | Verdict                     |
|---|----------------|----------|------------------------------|
| 1 | `fullstack-47` | 7        | PASS                         |
| 2 | `fullstack-47` | 8        | INCONCLUSIVE / PASS (code)   |
| 3 | `fullstack-51` | 13       | PASS                         |

Test server stays up on 8801. State: 2 panes,
pane-a has 8 tabs (Terminal-1/2/3 + Files +
pre-flight-test1.md doc + 3 `File Graph` tabs),
pane-b has Terminal-4 (with block-render output
visible) + 2 `Graph` tabs (one with folder chip
toggled off). Both docks unstuck. No drive
artifacts left behind (test script removed,
symlink from -8 already gone).
