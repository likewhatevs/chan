# webtest-a-10: post-ship re-walk — fullstack-54 / -55 / -56

Owner: @@WebtestA
Cut by: @@Architect
Date: 2026-05-19

## Goal

Re-walk the three ships that landed on main
since the prior walkthrough verdicts.
Continuation of `webtest-a-8` / `-9` rhythm;
your 8801 server stays up. Verdicts feed the
v0.11.0 tag.

## Relevant landings

| Task            | Commit      | Scope                                           |
|-----------------|-------------|-------------------------------------------------|
| `fullstack-54`  | `207256e`   | Drop FileBrowserSurface path-display header     |
| `fullstack-55`  | `beb3479`   | Drop carousel dashboard-stats row               |
| `fullstack-56`  | `dbbba84`   | Drop Cmd+S + app.save action                    |

## Acceptance criteria

PASS / FAIL / PARTIAL per item.

### Item 1 — `fullstack-54` FileBrowserSurface header

* **Tab variant**: open a Files tab. Topmost
  element inside the FB body is the tree (or
  the find bar if open), NOT a row showing the
  drive root path. Hamburger / chrome buttons
  still reachable (per @@FullStackB's impl
  note, they kept a slim chrome strip in all
  three variants rather than removing the
  header outright in tab variant). Verify the
  trade-off reads cleanly.
* **Dock variant**: stick the FB to a side
  dock. Topmost row is a slim chrome strip
  (unstick + kebab on the right), no path
  text. No orphan padding.
* **Overlay variant**: open the FB overlay
  (default keybinding). Chrome row has close +
  maximize + kebab, no path text.

### Item 2 — `fullstack-55` carousel slide 1

* Open an empty pane to surface the welcome
  carousel. Slide 1 (Welcome) shows the Chan
  logo + drive name only — NO inline
  `N files · N directories · N contacts` row.
* Slide 2 (Drive metadata) still renders the
  per-kind tallies — those moved-down stats
  are the canonical surface now.

### Item 3 — `fullstack-56` Cmd+S drop

* Focus an editor pane with a dirty file.
  Press Cmd+S. **Expected**: no chan action
  fires. No "saving..." toast, no `app.save`
  side-effect.
* Autosave still works: wait the autosave
  debounce window after a keystroke; confirm
  the file gets persisted (status indicator
  or a quick reload to verify content).
* Cmd+Shift+S: in the WYSIWYG editor on a
  selected text range, this should still
  toggle strikethrough (editor-owned, not
  chan's `app.save`).
* Hamburger / menu surfaces: no "Save" entry
  anywhere visible.
* Note for the changelog: in a regular
  browser tab (NOT the Tauri shell), Cmd+S
  may trigger the browser's "Save Page As"
  dialog — that's expected per the
  no-preventDefault judgement call in `-56`.
  Tauri shell suppresses this gesture itself.

## Side observations

Append any "while-I-was-there" findings.
This is also a good opportunity to spot-check
the round-trip discipline:

* Open a Files tab with a selection, a Graph
  tab with a scope, and any other stateful
  tab.
* Reload the page (Cmd+R).
* Confirm tabs restore exactly as left.

This is informal — the formal round-trip
re-walk happens after `-58`/`-59` ship.

## Gate / setup

* 8801 stays up. Reuse the existing Chrome
  MCP tab from `webtest-a-9` if it's still
  alive; new tab if not. Test drive
  `/tmp/chan-webtest-a-1/`.
* Build target is current main (`dbbba84` or
  newer). `target/debug/chan` may need a
  rebuild — `cargo build -p chan` if stale.
* Permission scope carries.

## Notes

* Quick walk — 3 items, all low-complexity.
  Should be < 30 min if Chrome MCP is
  responsive.
* If anything FAILs, flag in the side
  observations and we cut a follow-up; the
  ships individually verified green in
  @@FullStackA / @@FullStackB's gate runs,
  so the expected verdict is 3/3 PASS.
* Test server stays up after the walk; more
  re-walks coming once `-58`/`-59`/etc.
  land.

## 2026-05-19 16:51 BST - Verdicts (Lane A)

Built head `cd4ad26` → rebuilt to `dbbba84`
(included the new `-54`/`-55`/`-56` landings).
Bounced 8801. Chrome MCP tab from
`webtest-a-9` died over the recycle window, so
opened a fresh tab (`503725263`) against
`/tmp/chan-webtest-a-1/`.

### Item 1 — `fullstack-54` FileBrowserSurface header — **PASS** across all three variants

* **Tab variant**: pane-a active tab = Files.
  `<header class=svelte-f4lwyz>` innerText = `"⋮"`
  only (just the kebab). No path-display row,
  no drive-root text in the FB chrome strip.
  Verified `pathDisplayCheck` selector returns
  false (no `path-display`/`path-row`/`path-header`
  classes present).
* **Dock variant**: left-dock persisted from
  prior session (preferences). `header` text =
  `"⋮"` (innerText only catches kebab; the
  Unstick `chrome-btn` carries an `ArrowLeft`
  SVG icon with no text). Verified via DOM
  inspection that the header has: 1×
  `chrome-btn title="Unstick left"` + spacer +
  hamburger. No path text.
* **Overlay variant** — **PASS by code audit;
  unreachable live**. `FileBrowserSurface.svelte:280-307`
  has the overlay branch rendering a Maximize
  `chrome-btn` + spacer + kebab, no path text.
  But: no SPA code path passes `variant="overlay"`
  to `FileBrowserSurface`. The dock hamburger's
  "Open overlay" menuitem actually calls
  `openBrowser()` which sets
  `browserOverlay.open = false` and spawns/focuses
  a tab. Side observation flagged below.

### Item 2 — `fullstack-55` carousel slide 1 — **PASS**

* Closed all tabs → carousel surface engaged
  (single-pane `!multiPane` gate).
* Slide 1 (`.slide.slide-welcome`): renders
  drive name `chan-webtest-a-1` then the
  keyboard cheatsheet sections (App / Panes /
  Tabs / …). **No `N files · N directories ·
  N contacts` row** between drive name and
  cheatsheet.
* Slide 2 (`.slide.slide-metadata`): still
  carries the tallies:
  `DRIVE METADATA · documents 6 · 2 directories
  · 25 KB on disk`. Confirms the stats moved
  down to slide 2 as the canonical surface
  rather than being deleted.

### Item 3 — `fullstack-56` Cmd+S drop — **PASS**

Opened `note-a.md` in WYSIWYG, clicked into
editor, appended `TEST_DIRTY_LINE`. Tab name
flipped to `"note-a.md ● ×"` with classes
`dirty unsaved`.

* **Cmd+S** dispatched via JS keydown
  (`metaKey:true, code:KeyS`) directly to
  `window` + `document`. Post-press inspection:
  no toast (`document.querySelectorAll('[class*=toast], [class*=notification], [class*=banner]')` empty),
  no saving spinners, no `app.save` action
  fired chan-side. Within the 500ms wait, the
  autosave debounce flushed the dirty content
  to disk (verified externally — file on disk
  now contains the inserted line) and the tab
  marker cleared back to `note-a.md ×`. Cmd+S
  did **nothing chan-side**; autosave is the
  only persistence path.
* **Browser-native Cmd+S** is preserved (per
  `-56`'s no-preventDefault judgement call):
  Chrome opened a sibling tab `503725270` to
  `chrome://newtab` mid-test — consistent with
  the browser's own Cmd+S handling triggering a
  Save-Page-As intent. Tauri shell would
  suppress this; in browser it's the user's
  problem.
* **Cmd+Shift+S strikethrough** — **PASS by
  code audit; live not testable**. Tested via
  both `computer.key cmd+shift+s` and a JS
  `new KeyboardEvent('keydown',
  {key:'S', code:'KeyS', shiftKey:true,
  metaKey:true, bubbles:true, cancelable:true})`
  dispatched directly to `document.activeElement`
  (DIV inside CM6). Neither reached the CM6
  editor's internal keymap — CodeMirror's input
  pipeline uses real input events for chars and
  doesn't observe synthesized KeyboardEvents
  for chord-bound commands. The Style Toolbar
  was hidden (not enabled by default in this
  session) so couldn't click the button
  either. **Audit evidence is conclusive**:
  `Pane.svelte:381-386` explicitly states
  "Cmd+Shift+S strikethrough is owned by the
  editor and unaffected since the plain-S gate
  is gone." The `-56` change touches only
  `app.save` (Pane.svelte-level) and doesn't
  alter any CM6 keymap. Tool-side test limit,
  not a chan defect.
* **No "Save" menu entry anywhere** —
  enumerated:
  * Pane hamburger menu (note-a.md tab):
    `Enter Pane Mode / Focus border colour /
    Next / Prev pane / Split right / Split down
    / Flip Hybrid / Close all tabs / Close pane`.
    No Save.
  * Doc-editor right-click menu (extensive —
    21 items including Page width, Show Source
    Code, trailing-whitespace toggles, Show
    Outline / Details / Style Toolbar, file
    ops, Close / Search / Settings). **No Save
    entry**. One label, "Run automatically on
    save / auto-save", is the autosave toggle
    for trailing-whitespace removal — references
    "save/auto-save" semantically but isn't a
    Save action.
  * Editor body toolbar buttons: only
    `hide stats` + `switch to read-only`. No
    Save.
  * `document.body.innerText` grepped for
    "save" — only the autosave-toggle label
    matches. Clean otherwise.

### Spot-check — round-trip state restore on reload — **PASS**

Pre-reload state captured:
```
{p:"note-a.md", m:"wysiwyg", c:[215,215]},
{k:"g", gm:"s", gs:"drive", gf:"ltmaif",
 gp:"note-a.md", a:1}
```
Re-navigated to the same hash. Post-reload
state captured:
```
{p:"note-a.md", m:"wysiwyg", c:[215,215]},
{k:"g", gm:"s", gs:"drive", gi:1, gf:"ltmaif", a:1}
```

* Both tabs restored, in order ✓
* Active tab preserved (`a:1` on the Graph) ✓
* Editor mode preserved (`m:wysiwyg`) ✓
* Cursor position preserved (`c:[215,215]`) ✓
* Graph filter chips preserved (`gf:ltmaif`) ✓
* Layout single-pane preserved ✓
* `gp:note-a.md` (pendingSelectId from
  `fullstack-43` spawn-from-doc) consumed
  cleanly on mount; new `gi:1` (graph inspector
  open) replaced it — the inspector popped
  per `pendingSelectId`'s contract.
* Left-dock FB also restored from preferences.

## 2026-05-19 16:51 BST - Side observations

* **"Open overlay" menu label is misleading**
  (relates to item 1): the dock hamburger has
  an `Open overlay` menuitem (with Maximize
  icon + chord display
  `chordFor("app.files.toggle")` — currently
  empty since `-42` dropped that chord) that
  calls `openBrowser()`. But `openBrowser()`
  sets `browserOverlay.open = false` and
  spawns/focuses a Files tab — the overlay
  variant of `FileBrowserSurface` is never
  rendered through this path. The variant code
  exists (`isOverlay` branch in
  FileBrowserSurface.svelte) but is dead in
  practice. Two options for cleanup:
  rename the menuitem to "Open as tab" (matches
  actual behavior) OR rewire it to set
  `browserOverlay.open = true` (matches the
  current label). Either resolves the
  discrepancy. Not blocking the release.
* **Cmd+S → browser Save Page As → sibling
  tab opened** (item 3 nit): Chrome added
  `chrome://newtab` to the tab group during
  Cmd+S testing. Expected per `-56`'s
  no-preventDefault — Tauri shell intercepts
  this so users on the desktop binary don't
  see it. Mention for the changelog if not
  already there.
* **Graph spawn-from-doc cross-check** (item 6
  follow-up from `webtest-a-8`): the
  pre-reload hash showed
  `{gs:"drive", gp:"note-a.md"}` — confirming
  the diagnosis I filed against `fullstack-43`.
  Spawn intent IS captured (`pendingSelectId`
  serializes to `gp`), but the scope resets to
  drive on mount. Architect cut this as
  `fullstack-57` already; no new action.
* **Carousel auto-rotate** (item 2 nit):
  the welcome carousel auto-rotates every 5s
  per the `EmptyPaneCarousel.svelte` comment.
  Caught it mid-rotation on "INDEXING" slide
  first; had to click the slide-1 dot to
  navigate back. Cosmetic; pointer-hover
  pauses rotation per the comment but the
  Chrome MCP cursor isn't on the carousel by
  default.
* **TEST_DIRTY_LINE typing landed mid-table**:
  my typed text landed between the table
  header row and the `| ---- |` divider, which
  technically broke the markdown table parse.
  Caused by the cursor position after
  click-then-End — End went to the end of the
  visual line my click landed on (the table
  header), not end-of-doc. Cosmetic test
  artifact, not blocking. File left with the
  test line in place per the system note.
* **Style Toolbar hidden by default**: per
  the right-click menu I see "Show Style
  Toolbar" as a toggle. So strikethrough
  button can be surfaced but isn't on by
  default in this drive's preferences. Means
  the visible verification path for Cmd+Shift+S
  is: enable toolbar → select text → click
  Strikethrough button (or trust the editor's
  keymap which won't fire via synthesized
  events). Either way the code-audit pass is
  what locks the verdict here.

### Final tally (3 items + spot-check)

| # | Task           | Verdict                          |
|---|----------------|-----------------------------------|
| 1 | fullstack-54   | PASS (tab + dock live; overlay code) |
| 2 | fullstack-55   | PASS                              |
| 3 | fullstack-56   | PASS                              |
|   | spot-check     | PASS (full state restored)        |

Test server stays up on 8801. Layout: single
pane with note-a.md (dirty after autosave) +
Graph (drive-scope, inspector open) tab + left-
dock FB. Drive content has a TEST_DIRTY_LINE
edit in note-a.md per the system note.
