# webtest-b-6: Pre-release walkthrough — content / visual cluster (Lane B)

Owner: @@WebtestB
Cut by: @@Architect
Date: 2026-05-19

## Goal

Pre-release audit-trail walkthrough on the content
+ visual surface that landed today. **Flippable
Hybrids**, the carousel, list-mode trigger, the
British spelling sweep, multi-FB/Graph tabs, and
the xterm line-metric fix. Verdicts feed the
release-tag decision.

Lane A (`webtest-a-8`) covers the keyboard / menu
surface in parallel; no overlap on items.

## Relevant landings

| Task                | Commits                                | Scope                                                                   |
|---------------------|----------------------------------------|-------------------------------------------------------------------------|
| `fullstack-44`      | (carousel cycle/stop)                  | Carousel cycle / stop toggle                                            |
| `fullstack-45`      | `5e4ad92`                              | Immediate list-mode trigger on first `- `                                |
| `fullstack-46`      | `1f756bb`                              | British spelling sweep + hamburger "Enter Pane Mode" entry              |
| `fullstack-47`      | `da2d718`                              | Multiple File Browser + Graph tabs, tab DnD verify                      |
| `fullstack-48`      | `ffca091` + `c29b903` + `98ec4da`      | Flippable Hybrids (phase A model, phase B UI, phase C back-side dot)    |
| `fullstack-51`      | `0b0c919`                              | xterm.js `lineHeight: 1.0` for iTerm-matching row metrics               |

## Acceptance criteria

Report PASS / FAIL / PARTIAL per item with screenshot
evidence where the verdict isn't binary.

### Empty-pane carousel

1. **`fullstack-44` cycle / stop** — open an empty
   pane to surface the welcome carousel. Verify it
   auto-cycles slides on a steady interval. Click
   the cycle toggle; cycling stops, current slide
   pinned. Toggle again; cycling resumes.
2. **`fullstack-44` slide 3 (indexing graph)** —
   slide 3 renders the dir-only indexing graph
   (consumes `systacean-18`'s `/api/indexing/state`).
   Confirm grey / orange / green node states render
   and the pulsating-orange animation fires for any
   in-flight indexing dir. PARTIAL is acceptable
   if the drive is fully indexed by the time the
   walk hits (note state in the verdict).

### List mode

3. **`fullstack-45` immediate trigger** — open a
   doc, type `- ` (dash + space) at the start of a
   line. List mode should kick in **immediately**
   on the first `- `, not after a second keystroke.
   Verify with three rows; backspacing the leading
   `- ` exits list mode cleanly.

### British spelling + hamburger

4. **`fullstack-46` British spelling sweep** — spot
   check user-visible copy for the agreed
   spellings (e.g. `colour` not `color`,
   `behaviour` not `behavior`, etc.). Eyeball the
   common surfaces: menus, dialogs, settings
   panels, hover tooltips. List any stragglers you
   find.
5. **`fullstack-46` hamburger "Enter Pane Mode"** —
   the main hamburger menu has an "Enter Pane
   Mode (Cmd+K)" entry at the top. Clicking it
   should enter Pane Mode the same way the
   keyboard shortcut does.

### Multi-FB + Graph tabs

6. **`fullstack-47` multiple File Browser tabs** —
   open two File Browser tabs in the same pane.
   Verify each carries its own selection state and
   scroll position; switching between them doesn't
   bleed state.
7. **`fullstack-47` multiple Graph tabs** — same
   with two Graph tabs (different scopes if
   possible). Independent inspector state.
8. **`fullstack-47` tab DnD** — drag a File
   Browser tab from one pane to another. Drop
   should reparent it cleanly. (Chrome MCP DnD
   may fall short here — if so, mark
   INCONCLUSIVE and note the tool limit, same
   pattern as `fullstack-15`.)

### Flippable Hybrids — `fullstack-48`

9. **Phase A model + flip action** — `flipHybrid`
   action exists; calling it (via Cmd+K Tab in
   Pane Mode) flips the focused Hybrid to its
   back side. Calling again flips back.
10. **Phase B UI** — the flip animation reads
    cleanly (no jank, no orphan tab strip, no
    state bleed). Front and back are visually
    distinct (each side carries its own theme
    per the spec).
11. **Phase B per-Hybrid theme** — set front
    side to one theme, flip, set back side to a
    different theme. Confirm each side renders
    its own theme; flipping doesn't drag the
    other side's palette.
12. **Phase C back-side attention indicator** —
    on the front side, trigger an unread on the
    back (e.g. a watcher bubble notification
    surfaced on the back's rich prompt). A small
    flashing dot should appear on the front's
    chrome. Flip to the back; dot clears. Verify
    symmetric behaviour (unread on the front
    while looking at the back also shows the dot
    on the back's chrome).

### Terminal rendering

13. **`fullstack-51` xterm row metrics** — open a
    terminal, run `claude` (or any program that
    renders block-character / ASCII-art output).
    Verify rows render with zero pixel gap
    between consecutive block-character lines
    (matches iTerm). Compare against a
    pre-`0b0c919` build only if you happen to
    have one running; otherwise eyeball-only is
    fine.

## Side observations

Append any "while-I-was-there" findings that don't
fit the items above. Past walkthroughs surfaced
real follow-ups (`fullstack-22` BCAST stuck-toggle,
`fullstack-27` pre-flight event ingestion).

## Gate / setup

* Bring up `chan serve` on lane B's port (8810)
  with `target/debug/chan` rebuilt from current main.
* Drive into the SPA via Chrome MCP.
* Permission scope carries over from prior waves.
* Test server stays up after the walkthrough so
  re-tests don't pay respawn cost.

## Notes

* The deferred `fullstack-23` / `-24` follow-up that
  came up INCONCLUSIVE on the prior lane-B pass
  (deferred-state semantic on the F-button bubble)
  is NOT in scope here — it's still parked. Don't
  re-walk it unless something jumps out
  organically.
* `fullstack-53` (Tauri launcher refresh) is NOT in
  scope — Chrome MCP can't drive the WKWebView /
  Tauri shell. @@Alex owns that visual eyeball.
* `systacean-19` (watcher drive-root constraint) is
  backend; unit test covers it, no Chrome MCP
  surface.

## 2026-05-19 16:08 BST — verdicts (post-redistribution)

Walked items 1-6 + 9-12 against current main
(`cd4ad26`). Items 7, 8, 13 moved to Lane A
(`webtest-a-9`) at 16:55 BST per architect's
overflow redistribution. Test drive
`/tmp/chan-webtest-b-1/`, port 8810.

| # | Item                                | Verdict   |
|---|-------------------------------------|-----------|
| 1 | `-44` carousel cycle/stop toggle    | PASS      |
| 2 | `-44` slide 3 indexing graph        | PASS      |
| 3 | `-45` immediate list mode           | PASS      |
| 4 | `-46` British spelling sweep        | PASS      |
| 5 | `-46` hamburger "Enter Pane Mode"   | PASS      |
| 6 | `-47` multi File Browser tabs       | PARTIAL   |
| 9 | `-48` phase A flip action           | PASS      |
| 10| `-48` phase B flip UI               | PASS      |
| 11| `-48` phase B per-Hybrid theme      | PARTIAL   |
| 12| `-48` phase C back-side dot         | PARTIAL   |

Items 7, 8, 13 — handed to Lane A.

### Item 1 — `fullstack-44` carousel cycle/stop — PASS

Auto-cycles ~5-8s/slide (3 slides cycle 1→2→3→1).
24s sample at 1s intervals confirmed steady cadence.
Stop toggle: clicked stop, slide pinned at 3 across
12s; aria-label flipped `stop carousel cycle` →
`resume carousel cycle`. Resume restored cycling.

### Item 2 — `fullstack-44` slide 3 indexing graph — PASS

Slide 3 renders the dir-only indexing graph
consuming `/api/indexing/state`. Node states verified
against API + via DOM color sampling:

| State    | API key      | DOM fill            | Animation                                                  |
|----------|--------------|---------------------|------------------------------------------------------------|
| indexed  | "indexed"    | rgb(63, 185, 80)    | none                                                       |
| indexing | "indexing"   | rgb(255, 138, 61)   | `2.4s ease-in-out infinite svelte-1cqfcxx-indexing-pulse`  |
| pending  | "pending"    | rgb(142, 142, 147)  | none                                                       |

Orange/pulsating captured live by writing 1500 burst
files to events/ + sub/. Opacity oscillated 0.50 →
0.94 across the pulse window. Labels (sub/, events/,
preflight-events/, chan-webtest-b-1/) render on the
graph when indexing is in flight.

### Item 3 — `fullstack-45` immediate list mode — PASS

Doc `list-mode-test.md` seeded with `# heading\n\n
paragraph above\n`. Typed `-` then ` ` on a fresh
empty line:

| Keystroke | Line class                                  |
|-----------|---------------------------------------------|
| `-` (no space yet) | `cm-line cm-md-list-line cm-md-list-depth-0` |
| `- ` (with space)  | `cm-line cm-md-list-line cm-md-list-depth-0` |

List-mode class lands on the very first `-`
keystroke — actually stricter than the request
("on first `- `") but in the over-eager direction.
Three rows confirmed (Enter auto-continues the
marker). Backspace ×2 on an empty list line removes
the `- ` leader and reverts the line to plain
`cm-line`.

### Item 4 — `fullstack-46` British spelling sweep — PASS

Grep sweep across `web/src` for American forms
(`color`, `behavior`, `cancelled`, `labeled`,
`organize`, `optimize`, `recognize`, `theater`,
`favorite`, `customize`, `initialize`, `analyze`,
`catalog`, `harbor`, `honor`) inside user-visible
copy contexts (string literals, Svelte text nodes,
aria-labels). No stragglers found in user-visible
copy. British forms (`colour`, `behaviour`,
`cancelled`, `labelled`, `recognised`, `organise`,
`optimise`, `behavioural`) pervasive in
SphereTuner, GraphCanvas, GraphPanel, Pane,
tabs.svelte.ts, Wysiwyg, FileInfoBody, TagInfoBody,
menuClamp, OutlineBody, GraphDemo (comments + UI).

Code-level identifiers (`focus-color` CSS prop,
`backgroundColor` JS property, `textAlign:"center"`,
D3 `force("center")`) correctly stay American since
those are W3C / library interop boundaries, not user
copy.

Live spot-check: Settings dialog (EDITOR THEME /
APPEARANCE / LAYOUT / DATE PILLS / ABOUT) — clean.
Pane hamburger menu shows literal "Focus border
colour" — British ✓.

### Item 5 — `fullstack-46` hamburger Enter Pane Mode — PASS

Pane hamburger (kebab top-right) contents top-to-
bottom:

```
Enter Pane Mode        Cmd+K
Focus border colour
  ● blue ✓
  ● green
  ● pink
Next pane
Previous pane
Split right
Split down
Flip Hybrid            Cmd+K Tab
Close all tabs
Close pane
```

Clicking `Enter Pane Mode` flips app root to
`app pane-mode` class + shows "list-mode-test.md /
list-mode-test.md" preview + "pane mode · Enter
commit · Esc discard" chip — same surface as the
keyboard shortcut. Identical UX.

### Item 6 — `fullstack-47` multi File Browser tabs — PARTIAL

**Spawn-without-dedup: PASS**. Two `Files` tabs
coexist in the same pane (URL hash
`[{"k":"b","bi":1,"a":1},{"k":"b","bi":1}]`). Tab
switching works; tab 2 has its own close-button +
active-state styling.

**Per-tab selection / scroll / breadcrumb
isolation: FAIL**. Schema gap:

```typescript
export type BrowserTab = {
  kind: "browser";
  id: string;
  title: string;
  inspectorOpen: boolean;
};
```

No per-tab `path` / `selected` / `scroll` / `subpath`
fields. The File Browser data (current dir, expanded
tree, selected file, DETAILS panel target, scroll
offset) lives in shared module-level state.
Empirical verification:

| Step                                       | Tab 1     | Tab 2     |
|--------------------------------------------|-----------|-----------|
| Click index.md on tab 1                    | index.md  | (inactive)|
| Switch to tab 2, click notes.md            | notes.md  | notes.md  |
| Switch back to tab 1                       | notes.md  | notes.md  |
| On tab 1, expand sub/ + select sub dir     | sub dir   | sub dir   |
| Switch to tab 2                            | sub dir   | sub dir   |

Both tabs render identical state across switches.

Mismatch between task ask ("each carries its own
selection state and scroll position") and what
`fullstack-47` actually shipped (drop spawn dedup;
keep view state shared). The commit body
acknowledges this for browsers: only graphs have
per-tab scope/filters in their schema; browsers
multiplex the same view. Likely needs a follow-up
to add per-tab subpath/selection to BrowserTab if
the original spec is the target.

### Item 9 — `fullstack-48` phase A flip action — PASS

* `flipHybrid(paneId)` lives in
  `web/src/state/tabs.svelte.ts:1984`.
* Dispatched from `web/src/App.svelte:478` on Pane
  Mode's Tab key (after `paneMode.draft?.activePaneId`).
* Live test: front = `list-mode-test.md` (WYSIWYG).
  `Cmd+K` enters Pane Mode (`app pane-mode`
  class). `Tab` flips to back side → renders
  "Empty pane / no active tab" preview. `Enter`
  commits the flip. Second `Cmd+K Tab Enter`
  flips back to front — `list-mode-test.md`
  visible again with full content intact
  (`# List mode test`, paragraph + 3 list items).
  Cursor position preserved (c=[75,75]).
* Hash schema: `bt:[]` (back-side tabs) and
  `hb:"l"` (back-side theme override = light,
  set by `inverseTheme()` lazy init) appear in
  the serialized state after first flip.

### Item 10 — `fullstack-48` phase B flip UI — PASS

* Animation: pane element gains class `wobble`
  during flip commit. CSS keyframe duration
  observed via MutationObserver: ~1062 ms
  (wobble class present at t=0, cleared at
  t=1062).
* No orphan tab strip: front tab strip
  disappears cleanly when flipped to empty
  back; reappears on flip back.
* No state bleed: front-side editor content
  (header + paragraph + 3 list items) intact
  after round-trip; cursor at original offset.
* Front + back visually distinct on first flip
  (front has tab strip + editor content; back
  shows "Empty pane / no active tab" centered
  preview).
* Bottom-left "reindexing events/burst-N.md"
  pill visible during the burst write — nice
  side surface, didn't surface in prior
  walkthroughs.

### Item 11 — `fullstack-48` phase B per-Hybrid theme — PARTIAL

**Model + serialization: PASS**.
`HybridSide.theme` field exists on both front
(`node.theme`) and back (`node.back.theme`),
serialized as `ht` / `hb` in URL hash (values
`"d"` / `"l"`).

**Lazy init on first flip: PASS**. `flipHybrid()`
materialises a missing `back` with
`theme: inverseTheme(node.theme)` then swaps
front ↔ back theme overrides. Hash mutation
observed: `hb:"l"` after first flip from
default-dark front.

**Per-side override drives rendering: FAIL**.
`grep -rE 'node\.theme|pane\.theme|hybrid.*theme'
web/src --include='*.svelte' --include='*.ts'`
turns up **only** the write sites in
`tabs.svelte.ts:1995, 2007-2008`. No consumer
reads `HybridSide.theme` to apply CSS / data-theme
attribute / class. Empirical confirmation:

| Step                                          | doc data-theme | bg               |
|-----------------------------------------------|----------------|------------------|
| Front, global=Dark                            | dark           | rgb(28, 28, 30)  |
| Settings → Light → global=Light, on front     | light          | rgb(255,255,255) |
| Flip → back (had `hb:"l"` override = light)   | light          | rgb(255,255,255) |
| Settings → Dark on back → global=Dark         | dark           | rgb(28, 28, 30)  |
| Flip → front (no `ht`; visible-side override) | dark           | rgb(28, 28, 30)  |

Both sides track the **global** theme; the
`hb`/`ht` overrides are stored but never read
to override rendering. The "each side carries
its own theme per the spec" criterion is not met
in the current implementation. The
`SettingsPanel.svelte` Appearance toggle calls
`setThemeChoice()` (global), not anything per-
side.

Probable follow-up: add a per-pane
`data-theme={node.theme ?? ui.theme}` consumer
in Pane.svelte (mirroring the existing
`data-focus-color`), so `node.theme` actually
drives the visible palette.

### Item 12 — `fullstack-48` phase C back-side dot — PARTIAL

**Code path verified by inspection**:
- DOM element: `<span class="back-attention"
  aria-label="back side has unread activity">`
  rendered in `.actions` left of the hamburger
  (`Pane.svelte:919`).
- CSS animation:
  `animation: back-attention-pulse 1.5s
  ease-in-out infinite;` (Pane.svelte:1340).
- Derivation: `backHasAttention = $derived.by`
  returns true when `pane.back?.tabs.some(t =>
  t.kind==="terminal" && (t.watcher?.unread ||
  t.terminalActivity))` (Pane.svelte:270-278).
- Unit-test coverage:
  `Pane.test.ts:219` ("back-side-attention
  indicator surfaces when back has unread") and
  `:238` (clears when idle).

**Live driving — INCONCLUSIVE.** Walked the path:

1. Cmd+K 1 → Terminal-1 created in pane.
2. Flipped Terminal-1 to back via Cmd+K Tab Enter,
   then re-flipped so Terminal-1 was visible on
   the back side. Attached watcher to `events`
   via rich-prompt folder icon — watch status
   "watching events / Stop watching" + tab
   `Terminal-1 ●` blue bullet.
3. Initial scan surfaced 4 stale ScriptDriver
   survey bubbles (left-over from prior tests).
4. Flipped back to front (list-mode-test.md
   visible). Hash: `bt:[{k:"t",n:"Terminal-1"
   ,a:1}]`. Looked for `.back-attention` —
   **absent**.
5. Atomic-wrote `back-attn-v2.md` to
   `events/` (`to:"@@Terminal-1"`, temp +
   `mv`). 3s later — `.back-attention` still
   **absent**. No fresh bubble materialised on
   subsequent flip-to-back either.

The watcher fires for the initial scan but does
not appear to deliver fresh fsnotify events to
the back-side terminal session in this setup.
Either:
- The watcher's stale-cleanup
  (`fullstack-17`) evicted it when Terminal-1
  moved to the hidden side, OR
- The `to:"@@Terminal-1"` dispatch shape
  doesn't match the chan-server's expected
  routing pattern, OR
- Events delivery is gated by visibility
  state — chan-server's notice gets routed but
  the SPA's `watcher.unread` flag stays
  unflipped while the tab isn't mounted.

This is analogous to the `fullstack-15` DnD
limit: code is implemented + unit tests cover
the rendering path, but live external-event
trigger is brittle in this automation lane.
Verdict: PARTIAL — model + render path
verified, live drive not reproduced.

Symmetry test (unread on front while back is
visible) was not attempted because the front-
unread case requires a watcher on a different
front-side terminal session and the same live-
driving constraints apply.

## Side observations

* **Reindexing pill (bottom-left, dark blob)**:
  shows "reindexing events/burst-1255.md" with a
  small orange dot while the indexer is in flight.
  Not in any task spec; nice surface to mention.
* **Welcome carousel slide 1** displays
  "4 files · 3 directories · reindexing..."
  inline chip in orange while indexing is
  in flight (caught during burst write).
* **Watcher folder icon → drive-relative paths
  only**: the modal correctly rejects absolute
  paths with a clear inline error (matches
  prior `fullstack-13` walkthrough).
* **Schema gap A (item 6)**: `BrowserTab` has
  no `path`/`selected`/`scroll` fields — multi-
  FB-tab spawn works but each tab can't carry
  its own subpath/scroll state. Follow-up cut
  candidate.
* **Schema gap B (item 11)**: `HybridSide.theme`
  is written by `flipHybrid()` and serialized
  to `hb`/`ht`, but no rendering consumer reads
  it. Visible theme always follows
  `ui.themeChoice` (global). Follow-up cut
  candidate.
* **Stale ScriptDriver survey bubbles in
  events/**: the watcher's initial scan
  surfaces all existing files including survey
  events from prior @@WebtestB sessions. May
  warrant a sweep of `/tmp/chan-webtest-b-1/
  events/` between phases or a "process new
  events only" mode for the watcher.
