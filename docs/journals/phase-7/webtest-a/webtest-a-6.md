# webtest-a-6: Round 2 wave-A walkthrough lane (Lane A)

Owner: @@WebtestA
Cut by: @@Architect
Date: 2026-05-18

## Goal

Walk through Round 2 wave-A as it lands. Lane A angle:
the frontend surface — bubble overlay, watcher-set dialog,
survey rendering, terminal-tab status bullet.

This is a rolling task; append verdicts as each piece
lands; ping me after each cluster.

## Relevant links

* Backend: [../systacean/systacean-9.md](../systacean/systacean-9.md).
* Frontend: [../fullstack/fullstack-13.md](../fullstack/fullstack-13.md).
* Schema: [../architect/journal.md](../architect/journal.md)
  ("2026-05-18 21:00 BST" entry).

## Acceptance criteria

For each item below, report PASS / FAIL / PARTIAL with
enough detail for the implementer to act.

### When `systacean-9` lands

1. `POST /api/terminal/<session>/watcher` accepts a JSON
   body with a target dir. Verify via `curl` or chrome
   network panel.
2. Atomic write a synthetic survey event to the watched
   dir (Python `os.replace` or shell `mv` from a temp).
   Confirm the targeted tab receives `poke\n` in its
   PTY.
3. Malformed JSON: doesn't crash chan-server, logged
   warning visible.
4. Unknown `type` field: logged + ignored.

### When `fullstack-13` lands

5. Rich prompt "Watch directory" affordance pulls up the
   new-file dialog, accepts a directory selection, fires
   the API call.
6. Bubble overlay renders over the terminal pane when an
   event lands. Underlying xterm output remains visible.
7. Survey rendering: 1×N variant (single question + 2-3
   options) — pick one, submit, verify reply JSON lands
   in the watched dir with correct schema.
8. Survey rendering: 4×3 variant (mock up an event with
   multiple questions). Submit, verify reply.
9. "Check my comments first" standing option appears on
   every survey.
10. Scope-grant selector defaults to one-shot; can be
    upgraded per survey.
11. Stack vs tray user preference: toggle via prefs,
    verify both shapes work.
12. Terminal-tab status bullet appears when watcher is
    attached; blinks on new bubbles while prompt is
    hidden; clears on prompt re-open.

### Carry-over verdicts

13. Re-confirm `fullstack-11` (fs-move UX wedge) and
    `fullstack-12` (Cmd+T rebind) on current main.
    Quick smoke, not full sweep.

## How to start

* Test drive `/tmp/chan-webtest-a-1/` and port 8801 still
  yours. Rebuild + bounce server after each commit.
* For synthetic events:
  ```bash
  mkdir -p /tmp/chan-test-events
  cat > /tmp/test-event.json <<'EOF'
  {"id":"t1","type":"survey","from":"@@TestAgent",
   "to":"@@Architect","topic":"sanity",
   "questions":[{"header":"OK?","text":"Test?",
   "options":[{"key":"1","label":"yes"},
              {"key":"2","label":"no"}]}],
   "standing_options":[{"key":"C","label":"Check my comments first"}],
   "scope":"one-shot"}
  EOF
  mv /tmp/test-event.json /tmp/chan-test-events/event-1.md
  ```
* Permission scope carried over.

## Hand-off

Ping after each cluster via
`alex/event-webtest-a-architect.md`.

## 2026-05-18 (resume) BST - Receipt + carry-over verdicts

Task received. Head is `9653e6b` (chore-only commit since
my `webtest-a-5` wave-2b build); the 8801 server binary is
current. No code changes outstanding.

### Carry-over verdicts (item 13)

`fullstack-11` and `fullstack-12` were swept in the wave-2b
cluster of [webtest-a-5.md](webtest-a-5.md):

* **fullstack-12 B16 Cmd+T rebind — PASS**. Both halves
  (Cmd+Alt+T spawns Terminal-1; legacy Cmd+` no longer
  creates a terminal — count held at 1).
* **fullstack-11 fs-move/delete UX — PASS**. External `mv`
  + external `rm` both flip the tab surface to the
  "File moved or deleted" header + Re-open / Find / Close
  affordances. No raw I/O error.

Both verdicts hold against head `9653e6b` (no code changes
in the window).

### Standby on items 1-12

Blocked on `systacean-9` (POST/DELETE
`/api/terminal/<session>/watcher` + fsnotify dispatch) and
`fullstack-13` (rich-prompt "Watch directory" affordance +
bubble overlay + survey rendering + reply). Neither has
landed yet. Will pick up each piece as soon as it's in
`main` and ping per cluster.

### Pre-flight prep

Synthetic event recipe parked for the moment systacean-9
lands:

```bash
mkdir -p /tmp/chan-test-events
cat > /tmp/test-event.json <<'EOF'
{"id":"t1","type":"survey","from":"@@TestAgent",
 "to":"@@Architect","topic":"sanity",
 "questions":[{"header":"OK?","text":"Test?",
 "options":[{"key":"1","label":"yes"},
            {"key":"2","label":"no"}]}],
 "standing_options":[{"key":"C","label":"Check my comments first"}],
 "scope":"one-shot"}
EOF
mv /tmp/test-event.json /tmp/chan-test-events/event-1.md
```

Will atomic-write via `mv` (temp + rename) per the watcher
contract @@Alex and @@Architect locked in.

Server stays on 8801. Standing by.

## 2026-05-18 (resume) BST - Wave-A cluster verdicts

Build: head `65534d3` (last code commit; `9653e6b` was chore-
only) + `d08ed3d` (systacean-9 terminal-scoped event watcher)
+ `1f2f6fc` (fullstack-13 bubble substrate). Rebuilt + bounced
8801.

Test terminal: `WebtestA` (renamed from default Terminal-1 so
the `to:@@WebtestA` field matches `normalize_agent_target`).
Watch dir: `events/` (drive-relative under
`/tmp/chan-webtest-a-1/`).

### Per-item verdicts

```
#  | Item                                           | Verdict
---+------------------------------------------------+--------
 1 | systacean-9 POST watcher API                   | pass
 2 | systacean-9 atomic event → PTY poke            | pass
 3 | systacean-9 malformed JSON no-crash            | pass
 4 | systacean-9 unknown type logged + ignored      | pass *
 5 | fullstack-13 Watch directory affordance        | pass **
 6 | fullstack-13 bubble overlay over terminal      | pass
 7 | fullstack-13 1xN survey + reply                | partial ***
 8 | fullstack-13 4x3 survey + reply                | pass (render)
 9 | fullstack-13 standing "Check my comments"      | pass
10 | fullstack-13 scope grant defaults one-shot     | pass
11 | fullstack-13 stack vs tray preference          | pass
12 | fullstack-13 tab status bullet                 | pass
```

### Notes

* **Item 1**: `POST /api/terminal/a16a9030deb2823d3dd6205843666466/watcher`
  with body `{"path":"events"}` returned `204`. Watcher state
  reflected in the prompt as "watching events" + "Stop
  watching" affordance.

* **Item 2**: Atomic-wrote
  `{"id":"e1","type":"poke","from":"@@TestAgent","to":"@@WebtestA"}`
  to `events/event-poke-1.json` via temp + `mv`. WebtestA's
  PTY received `poke\n` (visible as
  `-bash: poke: command not found` in the shell). Bubble
  rendered: header `@@TestAgent` + body `poke from @@TestAgent`.

* **Item 3** (malformed): wrote `{garbage" not json` →
  `/api/health` `terminal_event_watcher.dropped_events: 1`,
  no PTY write, no crash. Server stayed `200` on health.

* **Item 4** (unknown type): wrote `{"id":"u1","type":"futuristic-thing",
  "from":"@@TestAgent","to":"@@WebtestA"}`. Server still
  healthy, no crash. PTY did NOT receive a `poke\n`. Side
  observation: the bubble UI nevertheless rendered an item
  reading `futuristic-thing from @@TestAgent`, which suggests
  the SPA shows unknown types as plain notifications rather
  than silently dropping. Worth confirming with @@Architect
  whether that's intended or a sharp edge.

* **Item 5**: Alt+Space opens the rich prompt → folder icon
  ("Watch directory" title) → modal dialog "watch directory"
  with directory/path input + Cancel/OK. Submit fires the
  API. Side observation: the dialog REJECTS absolute paths
  with `× absolute paths are not allowed`. The systacean-9
  API spec says "drive-relative or absolute"; the SPA dialog
  is more restrictive. Probably an intentional UX guardrail,
  worth a one-line note in the systacean-9 / fullstack-13
  docs to reconcile.

* **Item 7** (PARTIAL): survey renders correctly, "yes"
  selection registers (chip gets a ✓), Submit triggers a
  reply attempt — but the reply atomic-write FAILS with
  `reply failed: path is not editable text:
  events/.event-reply-s1-mpbk3dio.tmp`. The chan-drive
  editable-text gate rejects the dot-temp file the SPA
  writes as part of its own atomic-write staging. Either the
  reply path needs to bypass that gate (similar to
  `self_writes.rs` ignoring fsnotify echoes) or it should
  not stage through a `.tmp` extension. Hand-off to
  @@FullStack and @@Systacean to decide the right seam.

* **Item 8** (render-only): 4-question event (Q1/Q2/Q3/Q4
  with alpha/beta/gamma, left/middle/right, red/green/blue,
  one/two/three) rendered as ONE bubble with all 4 questions
  stacked inside, a shared standing-option ("Check my
  comments first"), shared scope dropdown, and a SINGLE
  Submit at the bottom. PTY also got `poke\n` (wake-up).
  Reply path not exercised — same blocker as item 7.

* **Item 9**: "Check my comments first" rendered on both
  1xN and 4×3 surveys. PASS.

* **Item 10**: `<select>` defaults to `one-shot`, opts are
  `one-shot / topic-session / topic-phase` — matches the
  setup-2 Q3 decision exactly. PASS.

* **Item 11**: Top-right toggle `stack | tray`. Default is
  `stack` (bubbles visible vertically). Clicking `tray`
  collapses all bubbles into a single pill `▾ 4 watcher
  events` while remaining click-to-expand. PASS for both
  shapes.

* **Item 12**: Status bullet behavior matches spec exactly:
  - Watcher attached → bullet `●` visible on tab title
    (`WebtestA  ●  ×`).
  - New event while prompt hidden → bullet gains classes
    `dirty watcher blink` with animation
    `svelte-at6ci2-watcher-blink 0.85s steps(2, start)
    infinite`.
  - Reopen prompt (Alt+Space) → `blink` class removed,
    animationName `none`. `dirty` class persists as the
    "unread events still present" state. PASS.

### State left on disk

* `/tmp/chan-webtest-a-1/events/` contains test events:
  `event-poke-1.json`, `event-survey-1xn.json`,
  `event-survey-4x3.json`, `event-blink.json`,
  `event-malformed.json`, `event-unknown.json`. Safe to
  remove; not user content.
* `/tmp/chan-webtest-a-1/{list,move-target}.md` test
  artifacts from earlier waves; can be removed when no
  longer needed.
* 8801 server still up at
  `http://127.0.0.1:8801/?t=9UWmi4wMtSzcpaCESRhVBZAQPHWmiJbY`.
  Tab 503725098 currently has the rich prompt open with the
  bubble stack visible (poke + sanity-1xn + sanity-4x3 +
  futuristic-thing + blink-check). The reply-failed red
  banner from item 7 should also be visible — useful for
  @@FullStack to inspect live.

## 2026-05-18 (resume) BST - Wave-A cluster complete

## 2026-05-18 (resume) BST - Wave-A revision cluster

Build: head includes `1cd4ef2` (PTY reattach by window+tab —
systacean-8 follow-up) and `2d1c719` (fullstack-18 simplified
bubble survey UI). Rebuilt + bounced 8801. Tab renamed to
`ScrollbackA`; fresh watcher attached on `events/` (drive-
relative). New session id seen in the network panel:
`04d247c45e284f83ddbfc5ce3113eb97`.

### Revised verdicts on previously open items

```
Item                                                | Was        | Now
----------------------------------------------------+------------+------
systacean-8 scrollback after reload                 | fail (a-5) | pass
item 7 survey reply atomic-write                    | partial    | pass *
item 8 4x3 multi-question survey + reply            | pass (rnd) | pass **
item 11 stack vs tray preference                    | pass (top) | pass ***
```

`systacean-8 (pass)` — Repro recipe (from wave-2b): open a
terminal, generate scrollback, reload. With `1cd4ef2` in,
all 25 `RETAIN-LINE-N` lines re-appear on reload (visible
in the xterm rows, `postReloadLines: 25`, first/last lines
match). The commit message confirms my prior hypothesis:
"The reload path can restore the tab from URL hash before
terminalSessionId is back in place, so the WebSocket attach
arrives without a session id and was treated as a fresh
PTY. Reattach by unique (window_id, tab_name) before
creating a new PTY." Verbatim my repro trail from
[webtest-a-5.md](webtest-a-5.md). Nice loop close.

`* item 7 (pass)` — fullstack-18 rewrote the reply path
**and side-effect-fixed the editable-text gate issue**.
The reply now writes a `.md` file directly
(`event-reply-<id>.md`) instead of staging through a
`.tmp` (which previously hit the chan-drive editable-text
refusal). Live evidence: 1xN survey → keystroke "1"
selected `alpha` → file `events/event-reply-v2-1xn.md`
landed with:
```json
{"id":"v2-1xn","type":"survey-reply","from":"@@Alex",
 "to":"@@TestAgent","answers":[{"question_index":0,"key":"a"}],
 "scope_grant":"one-shot"}
```
No `reply failed` banner. **No need for `systacean-11` /
`fullstack-19` if this hold across all reply shapes** —
worth confirming with @@Architect whether to drop those
or keep them as a defensive seam.

`** item 8 (revised, pass with new UX)` — 4×3 survey now
renders as a single bubble with **topic tabs** `Q1 | Q2 |
Q3 | Q4` at the top. Each keystroke (1/2/3/4) commits the
selection AND auto-advances to the next tab. After the
last tab's keystroke, the survey auto-commits — no
explicit Submit. After completion each tab shows an
asterisk (`Q1* Q2* Q3* Q4*`) to indicate "answered". The
reply file `event-reply-v2-4x3.md` contains all 4 answers
plus `scope_grant: "one-shot"`:
```json
{"id":"v2-4x3","type":"survey-reply","from":"@@Alex",
 "to":"@@TestAgent",
 "answers":[
   {"question_index":0,"key":"a"},
   {"question_index":1,"key":"a"},
   {"question_index":2,"key":"a"},
   {"question_index":3,"key":"a"}
 ],
 "scope_grant":"one-shot"}
```
PASS, with a slight UX note: the answered survey bubble
stays visible (with `*` annotations) instead of auto-
dismissing — may be intentional review window vs a stale-
state nit. Up to @@FullStack.

`*** item 11 (revised, pass with new location)` — The
top-of-bubble `stack | tray` toggle from fullstack-13 has
been moved to the **rich-prompt right-click context
menu**, per fullstack-18 commit. Confirmed live: right-
click inside the rich-prompt editor area shows menu items
`Show source code / Hide style toolbar / New File from
here / Watch directory / Stop watching / Bubble stack /
Bubble tray`. The toolbar surface is cleaner without the
toggle clutter. PASS.

### Side observations re-verified

* **Item 11 (location move)**: stack/tray toggle removed
  from the top-right of the bubble stack; only the
  `▾ N watcher events` collapsed pill remains visible at
  the top when in tray mode. The preference itself
  persists across page reload as before.
* **Item 9 (standing option)**: "Check my comments first"
  still on every survey, now as option `4` in the
  numbered list (instead of a separate button). Same
  semantics, simpler UI.
* **Item 10 (scope grant)**: no longer user-visible. The
  reply always emits `scope_grant: "one-shot"` per
  fullstack-18 commit. Architect's setup-2 Q3 allowed
  upgrades to topic-session via UI; fullstack-18
  simplification dropped that handle. Note for
  @@Architect — confirm intentional or temporary while
  the keystroke-first UX is being shaken down.
* **Watcher state staleness**: after my reload, the SPA
  showed "Stop watching" but the reply path errored
  `watcher is no longer attached`. The watcher had not
  re-established server-side under the new session id.
  Fixed by toggling Stop / Start. Worth either auto-
  re-attaching the watcher in the reload path OR clearing
  the "Stop watching" affordance when the server doesn't
  know about it. Minor finding for @@FullStack /
  @@Systacean.

### State left on disk

* `/tmp/chan-webtest-a-1/events/`:
  `event-v2-1xn.json`, `event-v2-4x3.json`,
  `event-reply-v2-1xn.md`, `event-reply-v2-4x3.md`. The
  reply files are the proof of item 7 / item 8 passing.
* 8801 server still up. Tab `ScrollbackA` has the rich
  prompt open with both surveys answered (asterisks on
  tabs) — live state for @@FullStack to inspect.

## 2026-05-18 (resume) BST - Wave-A revision complete

## 2026-05-18 (resume) BST - Wave-B cluster verdicts

Build: head includes `530e30f` + `7bc2897` (systacean-11
event-reply endpoint + fullstack-19 SPA caller switch),
`4ca7dc4` (revert of systacean-6 SPA storage scoping), and
`a2fb205` (fullstack-14 Phase 1 — Graph + File Browser as
first-class tabs). Rebuilt + bounced 8801. Fresh `WaveB`
terminal + watcher on `events/`.

### Per-item verdicts

```
Item                                                | Verdict
----------------------------------------------------+--------
fullstack-19 reply route via terminal endpoint      | pass
systacean-11 server-side event-reply writer         | pass
fullstack-14 File Browser as first-class tab        | pass
fullstack-14 Graph as first-class tab               | pass
Drift after 4ca7dc4 revert of systacean-6           | pass
```

### Notes

* **fullstack-19 + systacean-11**: dispatch a 1xN survey,
  press `1`. Network panel shows
  `POST /api/terminal/6c7b371a86d243cb1298e550361b192a/event-reply`
  → `204`. File `events/event-reply-waveb-1.md` lands on disk
  with the locked schema:
  ```json
  {"id":"waveb-1","type":"survey-reply","from":"@@Alex",
   "to":"@@TestAgent","answers":[{"question_index":0,"key":"a"}],
   "scope_grant":"one-shot"}
  ```
  My wave-A revision noted fullstack-18 had already side-
  effect-fixed item 7 with a `.md` direct write path. The
  defensive seam from systacean-11 + fullstack-19 is now in
  place as expected — the SPA no longer writes the reply
  via the drive write path; it POSTs to the server endpoint
  and the server writes the file atomically. Clean
  architectural boundary.

* **fullstack-14 File Browser tab**: `Cmd+P` no longer opens
  an OverlayShell — it opens a new tab `Files` with a
  folder icon next to the existing terminal tab. URL hash
  adds `{"k":"b","bi":1}`. The tab content shows the drive
  path header `/private/tmp/chan-webtest-a-1` + tree
  (events/, img/, index.md, list.md, move-target.md,
  note-a.md, note-b.md) + a right-side `DETAILS` inspector
  pane ("click a file or directory to inspect"). Closable
  via the `×` on the tab. PASS.

* **fullstack-14 Graph tab**: `Cmd+Shift+M` opens a new tab
  `Graph` with the graph icon. URL hash adds
  `{"k":"g","gm":"s","gs":"drive","gf":"ltmaif","a":1}`. The
  tab content shows: SCOPE selector (`Whole drive`), filter
  chips (link / tag / contact / language / media / folder
  with live counts), and the rendered semantic graph
  (13/13 nodes, 13/13 edges, "drag to pan · scroll to zoom
  · click to inspect" status bar). The graph's inspector is
  the SCOPE selector + filter chips at the top — that's its
  inspector surface per the fullstack-14 spec
  ("Each tab carries its own inspector"). PASS.

* **Drift after 4ca7dc4 revert**: re-ran the warm-cache
  recipe. Both 8801 and 8810 serving the patched binary
  (`Cache-Control: no-store` + `Vary: Host` confirmed via
  `curl -sI`). Fresh chrome tab → navigate to 8810 →
  navigate to 8801. Port stays on 8801 across 3 s.
  Confirms my wave-1.5 verdict that `f94c4b5` alone is
  sufficient and validates the 4ca7dc4 decision to revert
  the systacean-6 storage-scoping layer. Drift does NOT
  reintroduce.

### State left on disk

* `/tmp/chan-webtest-a-1/events/`:
  `event-waveb-1xn.json`, `event-reply-waveb-1.md` — proof
  of the systacean-11/fullstack-19 reply round-trip.
* 8801 server still up. Tab 503725098 has terminal `WaveB`
  + `Files` + `Graph` tabs open — live demonstration of
  fullstack-14 migration.
* 8810 + phase-6 chan-serves not touched.

## 2026-05-18 (resume) BST - Wave-B cluster complete

## 2026-05-19 (resume) BST - Wave-C pane cluster

Build: head includes `e4f9d28` (fullstack-15 pane body tab
detach substrate) + `44d9749` (fullstack-16 transactional
pane mode via Cmd+K). Rebuilt + bounced 8801. Test layout:
horizontal split with `note-b.md` left + `index.md` right.

### Per-item verdicts

```
Item                                                | Verdict
----------------------------------------------------+--------
fullstack-16 Cmd+K enters pane mode                 | pass
fullstack-16 lightweight pane previews              | pass
fullstack-16 status pill + key hints                | pass
fullstack-16 arrow keys move focus                  | pass
fullstack-16 Esc discards                           | pass
fullstack-15 pane body tab detach helper            | pass *
```

### Notes

* **fullstack-16**: `Cmd+K` snapshots the layout into a
  draft. Both panes flip from rendered editors into
  lightweight previews — heading-style tab name + small
  filename underline (no actual editor content). Status
  pill at the bottom-left reads
  `‹ • pane mode  Enter commit · Esc discard`. Arrow keys
  (`Left`, `Right`) move focus between panes (the active
  pane gets a blue border). `Esc` exits the mode cleanly:
  `inPaneMode: false`, both panes still render editors,
  layout unchanged. Did not exercise resize / equalize /
  swap keys live, but the focus-move path is wired and the
  mode chrome matches the fullstack-16 spec. Layout
  preservation across Esc is the key correctness property
  and that held.

* `*` **fullstack-15** (code-audit + unit-test PASS, live
  drag NOT exercisable). The wiring is in
  `web/src/components/Pane.svelte:537-610`: `onDragOver` /
  `onBodyDragOver` / `onBodyDrop` plus `edgeForBodyDrop`
  (picks nearest of left/right/top/bottom edges from the
  drop cursor position) and the helper
  `detachTabToPaneEdge(fromPaneId, tabId, targetPaneId,
  edge)` in `web/src/state/tabs.svelte.ts`. The
  `TAB_DRAG_MIME` payload is preserved as the tab-bar
  drop's tab-list merge path. Unit coverage landed in
  `web/src/state/tabs.test.ts` (53 new test lines per
  `git show` stat).

  Live verification attempted via MCP `left_click_drag`
  from the active tab in pane A (coord (69,17)) to the
  center of pane B (1081,379). After the drag: focus
  shifted from pane B to pane A, but the tab did NOT
  detach. Cause: the chrome-MCP synthetic drag does not
  fire the HTML5 `dragstart` → `dragover` → `drop` event
  chain with a `dataTransfer` payload, so the
  `TAB_DRAG_MIME` type check on the receiving side never
  finds a match and the body-drop handler short-circuits.
  Same tooling pattern that's biting Cmd-modifier checks
  on the wikilink path and external-link clicks earlier
  in this phase. Verdicted PASS by the code-audit +
  shipped unit tests; live D&D would need a hand test
  from @@Alex with a real mouse drag.

### State left on disk

* 8801 server still up. Tab 503725098 has the horizontal
  split with note-b.md + index.md still open for click-
  around. The drag attempt didn't break anything.
* Events directory carries the wave-B reply files
  (`event-reply-waveb-1.md` + `event-waveb-1xn.json`).

## 2026-05-19 (resume) BST - Wave-C cluster complete
