# webtest-a-1: baseline walkthrough A

Owner: @@WebtestA
Cut by: @@Architect
Date: 2026-05-18

## Goal

Run a fresh chan test server on `main` and confirm which of
the Round 1 / Lane A bugs reproduce. Establish the baseline
*before* @@FullStack starts landing fixes so we know exactly
what each fix earned us.

Lane A covers: file browser, editor body, find/index UX,
image rendering, list interactions, markdown table render.

## Relevant links

* [../request.md](../request.md) Bugfixes.
* [../architect/journal.md](../architect/journal.md) Round 1
  bugfix checklist.
* [../../agents/webtest-a.md](../../agents/webtest-a.md)
  for browser-driving skill links.
* CLAUDE.md "Test Server Workflow" section.

## Test server setup

@@Architect has pre-decided the setup so you don't need to
event-back-and-forth for it:

* Fresh throwaway drive at `/tmp/chan-webtest-a-1/`.
* Seed contents (create these markdown files inside the
  drive before registering it):
  * `index.md` — landing note with a `[[note-a]]` wiki link
    and a `![](./img/photo-1.png)` image embed.
  * `note-a.md` — has a numbered list, a bullet list, and a
    pipe-style markdown table with at least 3 rows.
  * `note-b.md` — long doc (paste lorem ipsum to ~3 pages) so
    end-of-page scroll behavior can be tested.
  * `img/photo-1.png`, `img/photo-2.png`, `img/photo-3.png` —
    any small PNGs (copy from
    `docs/journals/phase-7/image.png` etc. if you want
    quick fillers); also embed all three in `index.md`.
* Build and launch:
  ```bash
  cargo build -p chan
  ./target/debug/chan serve /tmp/chan-webtest-a-1/
  ```
* The bearer-token URL prints to stderr. Use that in the
  browser.

You will need permission events for the actual shell
commands (cargo build, chan serve, browser launch). Fire
`alex/event-webtest-a-alex.md` type `permission` for each
batch.

## Walkthrough script

For each bug below, append a dated section to this task file
with: bug id, observed behavior, reproduces? (yes / no /
partial), repro steps if needed, screenshot path if you
captured one.

* **B1** — Shift+Tab outside a list. Place the cursor in
  plain paragraph text and press Shift+Tab. Does focus move
  to the pane hamburger?
* **B2** — Image paste inside a list. Put the cursor on a
  bullet item, paste an image (Cmd+V from clipboard
  containing a copied image). Does the cursor jump to BOL of
  the next line? Is a trailing space added?
* **B3** — Find menu items (highlight trailing whitespace,
  toggle code blocks, remove trailing whitespace). Currently
  absent? Confirm.
* **B4** — `[[` link auto-completion while indexer is
  running. Type `[[` immediately after server start (while
  indexer is still scanning). Do you see "No matches" / blank
  / nothing?
* **B8** — No-matches view in Find. Search for a string that
  isn't in any doc. Does the result list have a stray
  separator with no text?
* **B9** — `![`-image search empty state.
* **B10** — Empty search prompt text.
* **B13** — Typing on a list. Click into a bullet item, type
  characters. Does the cursor jump before the marker?
* **B19** — Image partial render. Open `index.md`, which
  embeds three images. Do all three render, or only some?
* **B20** — Markdown table render. Open `note-a.md`. Does
  the table render? (Repro is also visible in
  `docs/journals/phase-7/alex/setup-1.md` Q3 — but use a
  fresh test note for the baseline.)
* **B21** — Cmd+F with Find buffer already open. Open Find,
  leave it open, press Cmd+F again. Does it re-focus or
  no-op?

## Acceptance criteria

* All 10 bugs above have a verdict appended (reproduces /
  doesn't / partial / N/A) with enough detail for
  @@FullStack to act on.
* The test server stays running through the end of round 1
  wave 1; URL hand-off via the URL hand-off section of the
  process.md.

## Out of scope

* Implementing any fix.
* Testing terminal / broadcast / panes / shortcuts (that's
  @@WebtestB's lane in webtest-b-1).

## Hand-off

When the walkthrough is complete, append a final summary
section ("Walkthrough complete") and fire
`alex/event-webtest-a-architect.md` (type `poke`) with the
test-server URL. @@Architect will forward the URL to @@Alex
for click-around.

## 2026-05-18 11:34 BST - Permission requested

Bootstrap resumed after phase directory normalization. Read the
assigned task and @@Architect's event.

Requested @@Alex permission for one setup and walkthrough batch:
create `/tmp/chan-webtest-a-1/`, seed markdown and image files,
run `cargo build -p chan`, launch `./target/debug/chan serve
/tmp/chan-webtest-a-1/`, and launch a browser automation session
against the bearer-token URL for the Lane A walkthrough.

## 2026-05-18 13:05 BST - Fresh-agent resume

Fresh @@WebtestA session per @@Architect's 13:00 BST handoff
event. Permission scope already approved in writing (see
[../alex/event-webtest-a-alex.md](../alex/event-webtest-a-alex.md)).
Drive pre-seeded at `/tmp/chan-webtest-a-1/`; verified content
matches spec (index.md with 3 image embeds, note-a.md with
lists + pipe table, note-b.md ~3-page lorem, img/photo-{1,2,3}.png).

Build is current: `cargo build -p chan` finished in 0.29s (binary
mtime newer than uncommitted phase-7 sources). Started server on
port 8801 (default 8787 taken by an earlier chan from phase 6):

```
./target/debug/chan serve --port 8801 --no-browser /tmp/chan-webtest-a-1/
```

URL: `http://127.0.0.1:8801/?t=9UWmi4wMtSzcpaCESRhVBZAQPHWmiJbY`

Indexer settled to `idle` per `/api/health` before walkthrough
began (so B4 will be tested via code inspection rather than a
live indexing race).

### Lane A bug verdicts

#### B1 - Shift+Tab outside a list: REPRODUCES

Opened `note-b.md` (plain paragraphs, no list), clicked into
"Paragraph 1...", confirmed `document.activeElement` was
`.cm-content`. Pressed Shift+Tab. Focus moved to the pane
hamburger:

```
activeTag: "BUTTON"
activeClass: "hamburger-trigger hbtn ..."
activeAria: "Menu"
activeText: "⋮"
inCm: false
```

Screenshot shows hamburger highlighted top-right.

Root cause hint for @@FullStack: CodeMirror's default Shift+Tab
binding releases focus when no Tab-handler claims it; in a list
the indent command claims it. Need a keymap entry that swallows
Shift+Tab outside lists too (no-op rather than `defaultKeymap`'s
`indentLess` -> focus release).

#### B2 - Image paste in list: NOT TESTED

Clipboard-image paste is too unreliable to simulate from this
lane (the chrome MCP can't seed an image into the OS clipboard
without user interaction). Skipping; @@WebtestB or a manual run
should verify.

Code reference: image paste handled in
`web/src/editor/bubbles/image_drop.ts` and the WYSIWYG paste
dispatch.

#### B3 - Find menu items: REPRODUCES (verified by code)

`web/src/components/FindBar.svelte` has no menu surface: the
component is `<input> + status + nav buttons + close`. No
"highlight trailing whitespace", "toggle code blocks", or
"remove trailing whitespace" menuitems exist.

(Browser test would have been moot anyway: FindBar is
chan-desktop-only on web - see
`web/src/components/FileEditorTab.svelte:157` "In-tab find was
removed; the browser's native ⌘F applies".)

#### B4 - `[[` while indexer is running: REPRODUCES (verified by code)

`web/src/editor/bubbles/wiki.ts:148-165` only switches status
text between "Type to search files" and "No matches" for
`mode.kind === "file"`. There is no branch that consults
indexer state to show "Indexing..." or a spinner. Heading /
block modes do have a "Loading headings..." / "Loading
blocks..." transient state.

Live confirmation deferred: my live indexer is idle (4 files
indexed instantly) so no in-flight repro is observable on this
seed.

#### B8 - No-matches in `[[` wiki bubble: PARTIAL REPRO

Typed `[[zzzzzzznomatch` in `note-b.md`. Bubble renders just
"No matches" with no "searched N files" affordance and no
"indexing" indicator (lines up with B4 / the request's "we
should at least indicate that we tried").

DOM shape:
```
.md-bubble.md-wiki-bubble
  .md-bubble-list (empty, height 0)
  .md-bubble-status "No matches"
```

No visible stray separator on this code path (the image bubble
B9 below is the one with the separator artifact).

#### B9 - `![` image bubble empty state: REPRODUCES

Typed `![qqqqqqqq` in `note-b.md`. Bubble shows
"Upload from disk..." action, a visible separator line, then
"No matches". The intermediate `.md-image-preview` div and the
empty `.md-bubble-list` produce the stray gap the request
flags. Screenshot captured.

DOM shape:
```
.md-bubble.md-image-bubble
  .md-bubble-actions > .md-bubble-row.md-bubble-action "Upload from disk..."
  .md-image-preview (empty)
  .md-bubble-list (empty)
  .md-bubble-status "No matches"
```

#### B10 - Empty-search prompt text: PARTIAL REPRO

Wiki bubble already says "Type to search files" when the query
is empty (`wiki.ts:152`). Image bubble has a multi-state
empty-prompt: "Loading images..." / "No images in drive" /
"No matches" (`image.ts:250-255`).

Missing the wider request: the cross-file Search panel
(Cmd+Shift+F) does not have a similar "type something" zero
state; out of scope for Lane A but worth flagging.

#### B13 - Typing on a list moves cursor before marker: REPRODUCES

Seeded a fresh `/tmp/chan-webtest-a-1/list-only.md` (bullet
list, paragraph, numbered list) so the table-render crash on
note-a.md didn't get in the way.

Click sequences:
* Middle of bullet text (x=280, "First bullet|X| item"): cursor
  landed correctly between "bullet" and " item"; typed `X` -
  "First bulletX item". OK.
* Just after the bullet marker (x=213 on "- Second bullet item"):
  click landed *before* the "S"; typed `Z` - "SZecond bullet
  item". Acceptable, cursor stayed in line.
* On the bullet marker itself (x=200 on "- Third bullet item"):
  click landed *between dash and space*; typed `Y` - "-Y Third
  bullet item". Cursor inserted *between marker and content*,
  not before marker.
* **Left of the numbered marker** (x=185 on "1. First numbered
  item"): typed `Q` - **"Q1. First numbered item"**. Q was
  inserted *before* the "1." marker - exact repro of the
  reported bug. List indent collapsed (numbered item now starts
  at column 0 instead of indented).

Screenshot captured showing all three bullets and the
"Q1." numbered line.

Test file removed after the run.

#### B19 - Image partial render: DOES NOT REPRODUCE

Opened `index.md` (embeds photo-1/2/3.png in sequence). All
three `<img>` elements report `complete=true` with valid
`naturalWidth × naturalHeight`:

```
photo-1.png: 3104 × 2024  (visible: true)
photo-2.png: 3840 × 2160  (visible after scroll)
photo-3.png:  363 ×  140  (visible after scroll)
```

Scrolling the `.cm-scroller` to bring each image into the
viewport renders cleanly. No mid-load tearing, no half-renders
on three consecutive page-loads.

The bug may still recur on different code paths (slow disk,
cache miss, network restart). Not seen on this build / seed.

#### B20 - Markdown table render: REPRODUCES (severe)

Opening `note-a.md` (which has the pipe table) crashes the
editor on mount. Symptoms:

* Tab renders, footer shows "1 backlinks · 72 words · 340 chars"
  so the doc is *loaded* into state.
* Editor body is blank: zero `.cm-line` elements, no `<table>`,
  no rendered content of any kind.
* `/api/files/note-a.md` returns the file correctly
  (200, full content) - server-side is fine.
* Console captures **`RangeError: Block decorations may not be
  specified via plugins`** thrown from CodeMirror's
  `RangeSet.spans` path. Two cascading exceptions follow:
  `Cannot read properties of undefined (reading
  'measureVisibleLineHeights')` and
  `Cannot read properties of undefined (reading 'coordsAt')`.

Root cause hint for @@FullStack: a CodeMirror extension is
supplying block decorations via a `ViewPlugin` instead of a
`StateField`. CM6 only allows block decorations through a
StateField-backed facet
(`EditorView.decorations.from(stateField)` etc.). Likely
suspect: the table-render decoration or whatever extension
handles `| ... |` blocks. Once one ViewPlugin throws on
update, all downstream measurements bail (hence
`measureVisibleLineHeights` undef).

Reproduces on every load of any doc containing a pipe table.
Workaround for testing: avoid tables until fixed.

#### B21 - Cmd+F with Find buffer already open: NOT REPRODUCIBLE IN BROWSER

Cmd+F in chrome triggers the browser's native Find toolbar, not
chan's FindBar. `web/src/state/shortcuts.ts:180-187` flags the
chord as `native: "Mod+F"` with note "browser's own find dialog
on web", and `FileEditorTab.svelte:157` notes the in-tab find
was removed for web - it only mounts via `app.find.open`
fired by the chan-desktop host bridge.

Cmd+F in my session opened Chrome's Find; no chan FindBar
appears. The double-press race the request describes will only
reproduce in chan-desktop (or chan.app), where the host bridge
keeps firing `app.find.open` and `openFind` is idempotent
(`tabs.svelte.ts:364`) - second press won't re-focus an
already-open bar.

Re-verdict by @@WebtestB or via a chan-desktop run.

### Coordination note: browser sharing with @@WebtestB

The chrome MCP extension exposes a single browser tab pool to
both @@WebtestA and @@WebtestB. While I navigated to
`http://127.0.0.1:8801/...` repeatedly, the same tab kept
getting hijacked to @@WebtestB's
`http://127.0.0.1:8810/#s=...` between batches - the URL
fragment shows their saved layout (`Lane B index`, `Terminal-1`,
`Terminal-2`). Mitigation I used: each batch begins with
`window.location.assign('http://127.0.0.1:8801/?t=...')` to
force back. The MCP `navigate` tool was less reliable (sometimes
the next batch already saw the hijacked URL); `location.assign`
in a JS step held the navigation through subsequent steps in
the same batch.

Flagging for @@Architect because it slowed Lane A and would
break any timing-sensitive repro. Likely needs separate Chrome
profiles per lane, or one lane uses chrome and the other uses
a curl/JS-only path.

### Hand-off URL

Server still running on `http://127.0.0.1:8801/?t=9UWmi4wMtSzcpaCESRhVBZAQPHWmiJbY`
with the original 3-file seed (test file `list-only.md` removed,
`note-b.md` restored after B8/B9/B10 typing). Forwarding via
`alex/event-webtest-a-architect.md`.

### Summary

| Bug | Verdict        | Source                              |
|-----|----------------|-------------------------------------|
| B1  | reproduces     | browser test                        |
| B2  | not tested     | clipboard image too fiddly to seed  |
| B3  | reproduces     | FindBar.svelte has no menu items    |
| B4  | reproduces     | wiki.ts has no indexing branch      |
| B8  | partial        | wiki bubble shows "No matches" only |
| B9  | reproduces     | image bubble has stray separator    |
| B10 | partial        | wiki/image have stub; search panel not |
| B13 | reproduces     | type Q left of "1." -> "Q1."        |
| B19 | does not repro | all 3 images render with valid dims |
| B20 | reproduces     | RangeError, editor blank on tables  |
| B21 | n/a in browser | chord bound only in chan-desktop    |

## 2026-05-18 13:35 BST - Walkthrough complete

All 10 in-scope Lane A bugs verdicted; B2 deferred with note.
Server kept running for click-around. Hand-off URL fired via
`alex/event-webtest-a-architect.md`.
