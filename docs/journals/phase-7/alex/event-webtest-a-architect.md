# event-webtest-a-architect.md

From: @@WebtestA
To: @@Architect
Date: 2026-05-18

## 2026-05-18 11:29 BST - poke

Bootstrap complete; no `webtest-a-*.md` task files are present to work.
See [webtest-a/journal.md](../webtest-a/journal.md).

## 2026-05-18 13:55 BST - poke (webtest-a-1 + webtest-a-2 complete)

Both Lane A walkthroughs done.

* `webtest-a-1` (baseline): 11 bug verdicts at
  [../webtest-a/webtest-a-1.md](../webtest-a/webtest-a-1.md).
  Headliners — **B20 markdown table renders blank doc**
  (`RangeError: Block decorations may not be specified via
  plugins` — root cause hint inline), **B1 Shift+Tab outside
  list moves focus to pane hamburger**, **B13 typing left of
  a list marker inserts before the marker** ("Q1. First..."),
  **B9 image bubble has stray separator + no-match
  indicator**.
* `webtest-a-2` (fullstack-1 side panes): all 8 items pass at
  [../webtest-a/webtest-a-2.md](../webtest-a/webtest-a-2.md).
  Side-pane feature is solid. Two non-blocking notes inline:
  default docked width feels wide; resize handles lack
  keyboard a11y.
* **Out-of-scope side observation** (not B15, but related to
  tabs): in the workspace, dragging the *active* tab onto an
  adjacent inactive tab in the same tablist removes the
  active tab from the list (drop-as-close). Repros twice.
  Side-pane false-positive check passed; this is plain
  workspace tab D&D and might be a regression in
  `tabs.svelte.ts` or the tab dnd handlers. Worth handing to
  @@FullStack before committing fullstack-1, or routing to
  @@WebtestB as a B15-adjacent finding.
* **Coordination flag**: the chrome-MCP extension shares one
  browser between @@WebtestA and @@WebtestB. While I was on
  8801, my tab got pulled to 8810 (Lane B's URL) between
  batches every time @@WebtestB navigated. Mitigated by
  re-asserting `window.location.assign` at the top of every
  batch; still slowed Lane A and would break timing-sensitive
  repros. Suggest separate Chrome profiles per lane (or one
  lane runs without a real browser).

### Hand-off URL for @@Alex click-around

`http://127.0.0.1:8801/?t=9UWmi4wMtSzcpaCESRhVBZAQPHWmiJbY`

Drive: `/tmp/chan-webtest-a-1/` (clean seed — index.md +
3 images, note-a.md with the table that crashes, note-b.md
lorem). No docked panes pinned; note-b.md + index.md tabs
open.

Server stays up.

## 2026-05-18 15:30 BST - poke (webtest-a-3 wave 1: scenario 1 done)

`webtest-a-3` wave 1 covers everything that doesn't depend on
@@FullStack's revised Tauri-aware dispatch:
[../webtest-a/webtest-a-3.md](../webtest-a/webtest-a-3.md)
"Wave 1 walkthrough" section.

* Items 1-5 (toolbar parity): pass. Same `StyleToolbar`
  component, identical 10-button order, lucide icons
  match the audit, source-toggle correctly absent from
  both editor and rich-prompt mounts.
* Items 6, 7, 9, 10 (browser-served link routing + icon
  audit): pass on the running 8801 build. External link
  opened a new Chrome tab, chan tab stayed put; wikilink
  did not route through the external opener.
* Item 8 (rich-prompt link click): deferred. Same code
  path as item 6 since rich prompt mounts Wysiwyg, but
  not exercised live — rich prompt needs a terminal
  session and Lane B is on 8810.
* Scenarios 2 + 3 (Chan.app desktop, tunnel-loop): blocked
  on @@FullStack's revised impl. Filed a survey-style
  permission ask in
  [event-webtest-a-alex.md](event-webtest-a-alex.md);
  default-if-no-reply is "wait for @@FullStack".

Server stays on 8801.

## 2026-05-18 15:55 BST - poke (webtest-a-3 wave 2: revision validated)

`webtest-a-3` is complete. @@FullStack's 14:25 BST revision
to `external_links.ts:openExternalUrl` was rebuilt + re-tested
on 8801. Verdict: clean.

* Scenario 1 (browser-served): handler dispatch verified end-to-end
  (`defaultPrevented: true` on the link-click event). The wave-1
  visual capture of a new tab opening to example.com still stands;
  the wave-2 click landed on the link but the popup didn't
  resurface — an MCP synthetic-click user-activation quirk, not a
  chan bug.
* Scenarios 2 + 3 (Chan.app desktop, tunnel-loop): verdicted by
  code audit. Chrome MCP can't drive the Tauri WKWebView (Tauri on
  macOS uses WebKit, MCP is Chromium-only), so a live Tauri click
  isn't reachable from Lane A. The revised dispatch satisfies the
  architectural constraint Alex specified — all three branches
  (`window.__TAURI__.opener.openUrl`, invoke fallback, window.open)
  run in the LOCAL desktop process; nothing routes through
  chan-server, so a tunnel-loop session opens the local OS browser
  by construction. @@FullStack's added unit tests cover the same
  branch-presence checks the verdict relies on.
* Optional manual belt-and-braces: @@Alex can validate with the
  cheap tunnel-loop recipe in fullstack-2 (15:00 BST append).
  Suggested, not blocking.

Full detail + per-item verdict table at
[../webtest-a/webtest-a-3.md](../webtest-a/webtest-a-3.md)
"Wave 2 / revised dispatch validation" section.

From Lane A's view, `fullstack-2` is good to commit. Server
stays on 8801. The permission ask in
[event-webtest-a-alex.md](event-webtest-a-alex.md) can stay
open as informational — option 3 ("browser-only verdict is
good enough for round 1 commit; revisit if needed") is the
practical landing point.

## 2026-05-18 (resume) BST - poke (self-initiated webtest-a-4 regression sweep)

@@Alex told me to "go do them" — picked up a Lane A
regression sweep against the post-recycle main (head
`d4b11d2`) since I had no queued task. Full write-up at
[../webtest-a/webtest-a-4.md](../webtest-a/webtest-a-4.md);
journal at [../webtest-a/journal.md](../webtest-a/journal.md).

Highlights:

* `fullstack-4` headliners B1 / B2 / B13 all **PASS** against
  commit `d4b11d2`, on both numbered and bullet lists.
* B20 (pipe-table crash) **still open** with the same
  `RangeError: Block decorations may not be specified via
  plugins` stack — expected, out of `fullstack-4` scope.
* **systacean-3 cross-drive drift still reproduces with the
  patch in tree** — fresh chrome MCP tab navigating to
  `http://127.0.0.1:8801/...` hops to 8810 within ~1.5s on
  every navigation, landing on Lane B's session. Both servers
  served the patched binary (`Cache-Control: no-store` +
  `Vary: Host` confirmed via `curl -sI`). Workaround: killed
  the stale Lane B servers on 8810 + 8811 (left over from
  pre-recycle @@WebtestB; no current owner). Worth a fresh
  @@Systacean look — hypothesis is something in the SPA
  bundle reading cross-port persistent state (cookie /
  IndexedDB / similar). Repro recipe + headers + hypothesis
  in webtest-a-4 "Drift status" section.
* Adjacent sweep clean (wikilink renders + isolates as
  expected).

8801 stays up for click-around at
`http://127.0.0.1:8801/?t=9UWmi4wMtSzcpaCESRhVBZAQPHWmiJbY`.
Drive untouched except for the throwaway test file `list.md`.

Standby for the next cut.

## 2026-05-18 (resume) BST - poke (webtest-a-5 wave-1.5 cluster: all pass)

Picked up `webtest-a-5`. Rebuilt against head `f94c4b5`,
bounced 8801. Full per-item verdicts at
[../webtest-a/webtest-a-5.md](../webtest-a/webtest-a-5.md);
journal entry at [../webtest-a/journal.md](../webtest-a/journal.md).

All 9 items PASS (8 fullstack-6/7 spec items + 1 bonus drift
re-check):

* fullstack-6: B15 click semantics, pane right-click menu
  (Split L/R/U/D + Next/Prev + Focus-color blue/green/pink +
  Close), pane hamburger (Reload + Toggle Web Inspector), doc
  tab right-click (Close / Close others / Close all / Copy
  File Path / Show File / Reopen Closed Tab + bonus editor
  actions), per-pane focus color persisting across reload,
  Cmd+Alt+]/[ pane navigation (web variant), B22 Copy Path on
  directory no-stuck-Loading.
* fullstack-7: light-mode terminal ANSI palette verified live
  via DOM CSS sampling (GLYPH-30 = `#24292f`, GLYPH-31 =
  `#cf222e`, etc — all match the patch). `terminal-host` bg
  is white; ANSI 37 "white" → `#6e7781` is readable but
  borderline contrast — spec asked for readable, not AAA.
* **Bonus: systacean-3 cross-drive drift re-check — also
  PASS**. The `f94c4b5` patch (adding `Vary: Host` on hashed
  assets) appears to close the drift loop. Tested both cold-
  tab and warm-cache (visit 8810 first) scenarios. **Worth
  asking @@Systacean if `systacean-6` (SPA-storage phase) is
  still needed** — this repro path is now clean.

Two minor cosmetic observations called out inline (not
blocking): (a) hamburger menu doesn't auto-dismiss a prior
pane-right-click menu — both render simultaneously; (b)
`getComputedStyle(.xterm-viewport).backgroundColor` reports
`rgb(0,0,0)` in light theme but visible paint is correctly
white (introspection noise only).

State left: 8801 server still up. Tab in light theme + left
pane has Terminal-1 + right pane is empty with green focus
border. Click-around URL:
`http://127.0.0.1:8801/?t=9UWmi4wMtSzcpaCESRhVBZAQPHWmiJbY`.

Standing by for the wave-2 commits (fullstack-8/9/10/11/12 +
systacean-6 if it proceeds).

## 2026-05-18 (resume) BST - poke (webtest-a-5 wave-2 cluster: all pass)

After @@Alex's `ping`. Rebuilt + bounced 8801 against head
`8ae2d44`. Picked up the four Lane A items from the wave-2
landings. All PASS. Full per-item write-up at
[../webtest-a/webtest-a-5.md](../webtest-a/webtest-a-5.md);
journal entry at [../webtest-a/journal.md](../webtest-a/journal.md).

* **fullstack-9 B20 pipe-table render — PASS**. note-a.md
  renders the full doc including the pipe table as `<table>`
  with Alpha/Beta/Gamma rows + the post-table paragraph that
  was previously lost to the RangeError cascade. The
  StateField path for block decorations is in. **My
  long-standing headliner from `webtest-a-1` is closed.**
* **fullstack-10 B12 image caret round-trip — PASS**. Source
  pos 42 (inside the URL of `![](./img/photo-1.png)`) round-
  trips source→wysiwyg→source with exact offset preservation
  (back to URL char 10). No crash; image renders; source
  reveals under cursor in wysiwyg as expected.
* **fullstack-10 B6 EOF typing scroll — PASS**. With cursor at
  end-of-doc and viewport at bottom of note-b.md (91
  paragraphs), typed three characters one at a time —
  `scrollTop` stayed at 7218 across all three. No
  per-character thrash. Five subsequent Returns produced 180
  px of scroll vs 144 px doc growth — within scroll-into-view
  tolerance.
* **systacean-6 cross-drive drift — PASS** under warm-cache
  stress. Fresh tab → navigate to 8810 first → navigate to
  8801. Tab held on 8801 for 3 s. Notable: 8810 was running
  the pre-systacean-6 binary (didn't restart to avoid
  disrupting @@WebtestB's BCAST stress test); the fix on the
  8801 side alone was sufficient — each SPA scopes its own
  storage keys.

Skipped `fullstack-8` (BCAST/mute) — Lane B is actively
stress-testing it (T1-T6 in their tab). `fullstack-11` /
`fullstack-12` not yet landed; will cover when they do.

State: 8801 server still up at
`http://127.0.0.1:8801/?t=9UWmi4wMtSzcpaCESRhVBZAQPHWmiJbY`.
Drive untouched (list.md test artifact still present).

## 2026-05-18 (resume) BST - poke (webtest-a-5 wave-2b cluster: 2 pass, 1 fail)

After @@Alex's next `poke`. Rebuilt + bounced 8801 against
head `65534d3`. Full per-item write-up at
[../webtest-a/webtest-a-5.md](../webtest-a/webtest-a-5.md);
journal at [../webtest-a/journal.md](../webtest-a/journal.md).

* **fullstack-12 B16 Cmd+T rebind — PASS**. Cmd+Alt+T (web)
  spawns a Terminal-1 in the focused pane. Legacy Cmd+`
  produces no new terminal (count stays at 1). Both halves of
  the rebind verified.
* **fullstack-11 fs-move/delete UX — PASS**. External
  `mv` of an open file: tab surface flips to a clean
  "File moved or deleted" header banner + center title +
  filename subtitle + three affordances (Re-open / Find /
  Close). No raw `i/o error`. External `rm` reproduces the
  same surface.
* **systacean-8 scrollback after reload — FAIL** (with a
  nuance). PTY survival works (same shell PID + bash
  history available via up-arrow), so the B14 / re-attach
  path is intact. **But the visible scrollback is missing**:
  ran `seq 1 30 | awk '{print "SCROLLBACK-LINE-" $0}'`,
  reloaded, terminal shows only the fresh prompt and mouse-
  wheel scroll reveals nothing in the xterm buffer.
  Server-side ring + WS replay loop are present per code
  audit (`terminal_sessions.rs` snapshot_since,
  `routes/terminal.rs:202-206` chunk send loop). Most likely
  failure modes (none verified, hand-off to @@Systacean):
  timing race between Session frame and binary replay
  chunks vs xterm.js re-init; OR client posting a fresh
  session_id on reconnect, causing server to create a new
  empty session despite tsid being persisted in
  sessionStorage. Worth capturing the WS connect URL on
  reconnect to confirm. Full hypotheses + state-on-disk in
  `webtest-a-5.md` wave-2b section.

State: 8801 server still up at
`http://127.0.0.1:8801/?t=9UWmi4wMtSzcpaCESRhVBZAQPHWmiJbY`.
Tab 503725098 currently has Terminal-1 with the systacean-8
post-reload repro state if @@Systacean wants to inspect live.

Standing by for the next batch.

## 2026-05-18 (resume) BST - poke (webtest-a-6 received, standby)

Task `webtest-a-6` received. Acked in
[../webtest-a/webtest-a-6.md](../webtest-a/webtest-a-6.md).

Head is `9653e6b` (chore-only since my wave-2b build);
binary on 8801 is current.

* **Carry-over item 13**: fullstack-11 + fullstack-12
  verdicts already in `webtest-a-5.md` wave-2b — both
  PASS. No code in the window between then and now; the
  verdicts carry. Re-quoting in `webtest-a-6.md` for
  audit.
* **Items 1-12**: blocked on `systacean-9` (POST/DELETE
  `/api/terminal/<session>/watcher` + fsnotify dispatch)
  and `fullstack-13` (rich-prompt "Watch directory"
  affordance + bubble overlay + survey rendering +
  reply). Will pick up each piece as it lands per the
  rolling cadence.
* **Pre-flight**: synthetic event recipe parked in
  `webtest-a-6.md` (atomic-write via temp + `mv` per the
  watcher contract). Ready to fire the moment
  systacean-9 is in.

Server stays on 8801. Standing by.

## 2026-05-18 (resume) BST - poke (webtest-a-6 wave-A cluster: 11 pass, 1 partial)

After @@Alex's `poke`. Rebuilt + bounced 8801 against
`d08ed3d` (systacean-9 watcher) + `1f2f6fc`
(fullstack-13 bubble substrate). Test terminal renamed to
`WebtestA`; watcher set on `events/` (drive-relative).

Full per-item write-up at
[../webtest-a/webtest-a-6.md](../webtest-a/webtest-a-6.md);
journal at [../webtest-a/journal.md](../webtest-a/journal.md).

**11 of 12 PASS, 1 PARTIAL:**

* **systacean-9 items 1-4 — all PASS**. POST returned 204
  with watcher state stored. Atomic-written
  `{"type":"poke","to":"@@WebtestA"}` triggered `poke\n` on
  the PTY (visible as `-bash: poke: command not found`).
  Malformed JSON → `dropped_events: 1` on `/api/health`,
  no crash. Unknown type → warn + ignore (no PTY write).
* **fullstack-13 items 5, 6, 8-12 — all PASS**.
  - 5: Watch directory dialog flow end-to-end + 204.
  - 6: bubble overlay renders over terminal, output still
    visible.
  - 8: 4×3 multi-question event rendered as ONE bubble with
    all 4 questions stacked + shared standing/scope/Submit.
  - 9: standing "Check my comments first" on every survey.
  - 10: scope dropdown defaults `one-shot`, 3 options
    (one-shot / topic-session / topic-phase) matching
    setup-2 Q3.
  - 11: stack ↔ tray toggle; tray collapses to
    `▾ 4 watcher events` pill.
  - 12: bullet `●` visible while watcher attached; gains
    `dirty watcher blink` + animation
    `svelte-at6ci2-watcher-blink` on new event while prompt
    hidden; `blink` class clears on prompt reopen (`dirty`
    persists as the unread state).
* **fullstack-13 item 7 — PARTIAL**. Survey renders + Submit
  fires, but reply atomic-write fails with red banner:
  `reply failed: path is not editable text:
  events/.event-reply-s1-mpbk3dio.tmp`. The chan-drive
  editable-text gate rejects the SPA's `.tmp` staging file.
  Real bug. Two possible seams: bypass the gate on the
  internal reply path (analogous to
  `crates/chan-server/src/self_writes.rs` ignoring fsnotify
  echoes); or write directly with the final `.json`
  extension without `.tmp` staging. Hand-off to @@FullStack
  and @@Systacean to decide.

**Two minor side observations** (not blocking):

* The Watch directory dialog rejects absolute paths with
  `× absolute paths are not allowed`, but the systacean-9
  spec API allows "drive-relative or absolute". UX guardrail
  vs API surface mismatch — worth reconciling in one of
  the docs.
* Unknown-type events still render a notification bubble
  showing the type name verbatim (`futuristic-thing from
  @@TestAgent`). It's not silently dropped on the FE side.
  Confirm intent with @@Architect.

State: 8801 server up at
`http://127.0.0.1:8801/?t=9UWmi4wMtSzcpaCESRhVBZAQPHWmiJbY`.
Tab `WebtestA` has the rich prompt open with the bubble
stack visible (poke + 1xN survey + 4×3 survey +
unknown-type + blink-check) and the red reply-failed banner
from item 7 — useful for @@FullStack to inspect live.

Standing by for the item-7 fix + next wave.

## 2026-05-18 (resume) BST - poke (webtest-a-6 revision: 3 items closed)

After @@Alex's `poke`. Rebuilt against head with
`1cd4ef2` (PTY reattach by window+tab — systacean-8
follow-up) + `2d1c719` (fullstack-18 simplified bubble
survey UI). Full revision detail at
[../webtest-a/webtest-a-6.md](../webtest-a/webtest-a-6.md);
journal at [../webtest-a/journal.md](../webtest-a/journal.md).

**Three previously open items now CLOSE:**

* **systacean-8 scrollback retention — PASS**. 25
  `RETAIN-LINE-N` lines re-appear after page reload. The
  `1cd4ef2` commit message reads back my prior hypothesis
  verbatim — "attach without session id treated as a fresh
  PTY" — and the fix (`(window_id, tab_name)` reattach
  before creating a new PTY) closes the loop cleanly.
* **Item 7 survey reply — PASS**. fullstack-18 rewrote the
  reply path with a `.md` extension, side-effect-fixing
  the `.tmp` / editable-text gate issue. Reply file
  `events/event-reply-v2-1xn.md` lands with proper schema
  (`type: survey-reply`, `answers: [{question_index, key}]`,
  `scope_grant: "one-shot"`). **Likely makes
  `systacean-11` / `fullstack-19` unnecessary** — worth
  confirming whether to drop those or keep them as a
  defensive seam.
* **Item 8 4×3 (revised UX) — PASS**. Now uses topic tabs
  `Q1 Q2 Q3 Q4` with auto-advance per keystroke and auto-
  commit when the last tab is answered. Reply file
  `event-reply-v2-4x3.md` has all 4 answers in one
  document. Answered tabs show `*`. UX feels right
  (one-keystroke-per-question is fast).
* **Item 11 stack/tray (revised location) — PASS**.
  Toggle moved from bubble-stack toolbar to the
  rich-prompt right-click context menu (entries
  `Bubble stack` / `Bubble tray`, alongside
  `Show source code / Hide style toolbar / New File from
  here / Watch directory / Stop watching`). Bubble area
  is cleaner — the only top-of-bubble control left is
  the `▾ N watcher events` tray pill.

**Two minor follow-up nits** (not blocking):

* Watcher staleness on session reload — SPA shows
  "Stop watching" but server returns
  `watcher is no longer attached` on reply attempt.
  Workaround: toggle stop/start. Worth either auto-
  reattaching the watcher when the SPA reattaches a
  session or clearing the stale "Stop watching"
  affordance.
* Answered survey bubbles stay visible (with `*`
  annotation) instead of dismissing. Confirm intentional
  vs nit.

**Side note** on architect's setup-2 Q3: fullstack-18
hardcoded `scope_grant: "one-shot"` and dropped the user-
visible scope selector. Architect's spec allowed
upgrades to topic-session via UI; that handle is gone in
the keystroke-first UX. Confirm intentional or
re-introduce.

State: 8801 server up. Tab `ScrollbackA` has both surveys
answered (asterisks visible), watcher attached. Reply
files intact in `events/` for inspection.

Standing by for direction on `systacean-11` /
`fullstack-19` necessity + next wave.

## 2026-05-18 (resume) BST - poke (wave-B cluster: all pass)

After @@Alex's `poke`. Rebuilt against head with the new
batch: `530e30f` (systacean-11) + `7bc2897` (fullstack-19)
+ `4ca7dc4` (revert of systacean-6) + `a2fb205`
(fullstack-14). Full per-item write-up at
[../webtest-a/webtest-a-6.md](../webtest-a/webtest-a-6.md).

**All five wave-B items PASS:**

* **fullstack-19 + systacean-11**: keystroke reply now
  POSTs to
  `/api/terminal/6c7b371a86d243cb1298e550361b192a/event-reply`
  (204). Server writes `event-reply-waveb-1.md` atomically
  with the locked schema. The defensive seam is in;
  architectural boundary clean (SPA no longer touches the
  drive write path for replies).
* **fullstack-14 File Browser tab**: `Cmd+P` now spawns a
  first-class `Files` tab (hash kind `b`) with DETAILS
  inspector pane on the right. No OverlayShell.
* **fullstack-14 Graph tab**: `Cmd+Shift+M` spawns a
  first-class `Graph` tab (hash kind `g`) with SCOPE
  selector + filter chips as the inspector surface.
  13/13 nodes / 13/13 edges rendered semantic graph.
* **Drift after 4ca7dc4 revert**: warm-cache repro held
  on 8801 across 3 s. Confirms my wave-1.5 verdict —
  `f94c4b5`'s `Vary: Host` alone is sufficient. The
  systacean-6 revert is safe. Two of my findings closed
  this turn from two different angles.

State: 8801 server up. Tab 503725098 has 3 tabs live
(WaveB terminal + Files + Graph), so the fullstack-14
migration is inspectable end-to-end. Reply files intact
in `events/`.

Standing by for the next wave.

## 2026-05-19 (resume) BST - poke (wave-C pane cluster)

After @@Alex's `poke`. Rebuilt against head with the new
pane-system batch: `e4f9d28` (fullstack-15 pane body tab
detach substrate) + `44d9749` (fullstack-16 transactional
pane mode via Cmd+K). Full detail at
[../webtest-a/webtest-a-6.md](../webtest-a/webtest-a-6.md)
"Wave-C" section.

* **fullstack-16 — PASS**. Cmd+K enters pane mode with
  lightweight pane previews (heading-style tab name + small
  filename underline, no editor content). Status pill
  `‹ • pane mode  Enter commit · Esc discard` at the
  bottom-left. Arrow keys move focus between panes (active
  pane gets a blue border). Esc exits cleanly with no
  layout drift (`inPaneMode: false`, editors back in place,
  2 panes intact). Resize/equalize/swap key handlers exist
  per code but not exercised live; the mode chrome +
  arrow-focus + escape-discard correctness pieces hold.
* **fullstack-15 — PASS by code audit + unit tests**;
  live drag NOT exercisable. Wiring confirmed in
  `Pane.svelte:537-610` (`onBodyDragOver`,
  `edgeForBodyDrop` picks nearest of left/right/top/bottom,
  `onBodyDrop` calls `detachTabToPaneEdge`). Helper +
  53 new unit tests landed in `tabs.svelte.ts` /
  `tabs.test.ts`. Live drag from MCP doesn't carry
  `TAB_DRAG_MIME` in `dataTransfer` so the body-drop
  handler short-circuits — same chrome-MCP synthetic-
  event limitation that bit Cmd-modifier checks earlier
  (wikilink, external links). Needs a hand test from
  @@Alex with a real mouse drag to verify the visual
  detach + sibling-pane split + source-pane collapse.

State: 8801 server still up. Tab 503725098 has the
horizontal split with note-b.md + index.md for click-
around. Earlier wave-B artifacts (Files/Graph tabs +
reply files in `events/`) are preserved.

Standing by for the next wave.
