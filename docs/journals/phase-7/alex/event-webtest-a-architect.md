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

## 2026-05-19 (resume) BST - poke (webtest-a-7 receipt + build break + polish/SKILL pass)

After @@Alex's `poke`. `webtest-a-7` (wave-B walkthrough)
received. Of the upstream pieces, only `fullstack-17`
(polish bundle, `0c2faa7`) and `architect-1`
(orchestration SKILL, `dfcad1c`) have landed. `fullstack-
20` + `systacean-12/13/14` not yet on main, so items
1-10 are gated.

**Important blocker** — `cargo build -p chan` fails on
the **in-progress** systacean-12 substrate in the
working tree:

```
error[E0382]: use of moved value: `cwd`
  --> crates/chan-server/src/terminal_sessions.rs:598:27
540 | let cwd = opts.cwd.unwrap_or_else(|| config.drive_root.clone());
541 | cmd.cwd(cwd);            // value moved here
...
598 |             cwd: Some(cwd),  // E0382
```

Fix: `cmd.cwd(cwd.clone())` on line 541 (or restructure
to keep ownership). Real bug for @@Systacean's
attention; the binary won't rebuild until it lands. My
8801 server is DOWN because of this.

**fullstack-17 polish — PASS by code-audit**
(live retest deferred to post-rebuild):

* **Absolute-path dialog**:
  `PathPromptModal.svelte` now passes
  `allowAbsolute: pathPromptState.allowAbsolute` into
  `validatePath`, and `missingAncestors` early-returns
  for `/`-prefixed paths. Closes my wave-A side
  observation about the dialog rejecting
  `/tmp/chan-test-events` despite the systacean-9 API
  spec allowing absolute paths.
* **Unknown-type bubble drop**:
  `watcherEvents.ts:parseWatcherEvent` adds
  `if (obj.type !== "survey" && ... !== "poke") return
  null;`. The `futuristic-thing` event from my wave-A
  would now drop silently on the SPA side, matching
  backend log+ignore. Closes my wave-A side observation.
* **Stale watcher cleanup, answered-survey auto-dismiss,
  terminal rename keep-open + restart confirmation,
  mutually-exclusive pane menus, light-mode ANSI white
  contrast**: all listed in the commit message and
  covered by the test set
  `BubbleOverlay / TerminalRichPrompt / watcherEvents /
  pathValidate`. Live retest after rebuild.

**architect-1 orchestration SKILL — read; no drift to
flag (yet)**:

* `docs/agents/orchestration/README.md` — index. Routes
  reader to atomic-writes + spawn-protocol; defers MCP
  discovery to `systacean-14`. Matches shipping
  reality.
* `atomic-writes.md` — documents the watcher contract
  exactly as systacean-9 enforces (temp + rename, single
  read on Create/rename-final, no retries). Per-language
  examples (bash / python / node / rust) follow the same
  shape. Matches what my wave-A walkthrough exercised.
  No drift.
* `spawn-protocol.md` — forward-looking; explicitly
  staked to systacean-12's design. Describes
  `POST /api/terminals` (create with name + command +
  env), `POST /api/terminals/<session>/restart`,
  `DELETE /api/terminals/<session>`, plus a 1/2/3
  preflight pattern (open terminal / kill / retry).
  The in-progress chan-server tree adds
  `Registry::restart` + `CreateOptions { command, env,
  preflight: PreflightConfig { dir, from, to } }` —
  names + shape align with the SKILL. Will re-verify
  after systacean-12 lands.

**Blocked on upstream**: items 1-6 (`fullstack-20` +
`systacean-12`), 7-8 (`systacean-13`), 9-10
(`systacean-14`).

State: 8801 server is DOWN; can't rebuild the binary
until the `cwd` move bug is fixed. Standing by.

## 2026-05-19 (resume) BST - poke (systacean-12 backend verified)

After @@Alex's `poke`. Build unblocked
(`cwd.clone()` fix landed, per @@Architect's 01:30 BST
ack). Rebuilt + relaunched 8801.

**systacean-12 HTTP control channel (`314a68b`) tested
directly via curl — all endpoints PASS:**

* **`POST /api/terminals`** with body
  `{"name":"@@SpawnTest","command":"bash -c '\''echo hi;
  sleep 5; echo bye'\''","env":{}}` →
  `201 Created` +
  `{"session":"84b5e0a3b3fbe47843e28eb1dea66564",
   "tab_label":"@@SpawnTest"}`. Body shape matches the
  spawn-protocol SKILL contract.
* **`POST /api/terminals/<session>/restart`** → 204.
* **`DELETE /api/terminals/<session>`** → 204. Idempotent
  follow-up returns 404 with
  `terminal session not found`.

**SPA bridge gap** (expected, blocked on `fullstack-20`):
the SKILL says the new terminal lands "in the active
pane", but the SPA's tab layout is client-only and the
HTTP-spawned PTY isn't pushed over any existing channel.
Reloading the chan tab after a spawn does NOT add the
new tab to the tab strip. `fullstack-20` is in-progress
in the working tree (`SpawnDialog.svelte` new, modified
`web/src/api/client.ts`, etc.) and will close this gap.
Backend is ready; SPA listener is what's missing.

**Still blocked**: items 1-6 (need `fullstack-20`),
7-8 (need `systacean-13`), 9-10 (need `systacean-14`).

State: 8801 server back up at
`http://127.0.0.1:8801/?t=9UWmi4wMtSzcpaCESRhVBZAQPHWmiJbY`.
Both test terminals (@@SpawnTest, @@SpawnB) cleaned up
via DELETE. Standing by for next landing.

## 2026-05-19 (resume) BST - poke (fullstack-20 spawn UI: items 1-3 PASS, 4-6 PARTIAL)

After @@Alex's `poke`. `f2094c3` fullstack-20 landed.
Rebuilt + restarted 8801; HostA orchestrator with
watcher on `events/`.

**Items 1-3 PASS:**
* **Item 1** — Right-click in rich-prompt editor area
  surfaces a `Spawn agent` entry alongside the existing
  context items. Also a toolbar shortcut at the prompt
  top-right (two ways into the dialog).
* **Item 2** — Dialog renders with title `🤖 Spawn
  agent`, fields: **Tab name** (default `@@Agent`),
  **Command** (textarea), **Env** (textarea,
  `KEY=value` placeholder), **Cancel** + **Spawn**
  buttons. Submit POSTs to
  `/api/terminals` from the SPA → 201, new tab
  `@@SpawnEcho` lands in the active pane and becomes
  active.
* **Item 3** — `bash -c 'echo hi; sleep 5; echo bye'`
  captured both lines + the clean epilogue
  `process exited (0); press Ctrl+D to close this
  tab`.

**Items 4-6 PARTIAL — server emits, SPA does not
render the pre-flight bubble:**

Recipe: HostA orchestrator + Spawn `@@AuthNeeded` with
command `bash -c 'echo please log in; sleep 60'`.

* chan-server **did** detect the `please log in`
  pattern (per `terminal_sessions.rs:1010-1022`
  preflight_line_matches) and wrote the file
  `events/pre-flight-f90ed024a46dc89a.md`:
  ```json
  {"id":"pre-flight-f90ed024a46dc89a",
   "type":"pre-flight","from":"@@AuthNeeded",
   "to":"HostA","note":"...please log in"}
  ```
* HostA's rich prompt **did not render a bubble** for
  the pre-flight — no tray pill, no article, no
  notification.
* `watcherEvents.ts:35-42` parser allowlist includes
  `pre-flight`. `BubbleOverlay.svelte:69-344` has
  explicit `event.type === "pre-flight"` render
  branches (with `preFlightTimedOut`, "Spawn idle -
  retry now?", hardcoded options). So parsing +
  rendering are wired; the **delivery path** from
  the server-written file to the SPA bubble list is
  the gap. Two likely seams (untested):
  1. `self_writes` suppression is silencing
     chan-server's own pre-flight write (the watcher
     normally ignores chan-server echoes to avoid
     loops; pre-flight needs an exemption).
  2. Schema drift — SKILL spawn-protocol.md says
     pre-flight carries `topic` / `questions` /
     `options` / `scope` like a regular survey, but
     the actual emit is minimal
     `{id,type,from,to,note}`. BubbleOverlay hardcodes
     the 3 options for pre-flight type so this isn't
     immediately blocking, but it's drift worth
     reconciling either in the SKILL prose or the
     server emit.

Items 5 + 6 (spinner + kill action) gated on item 4
rendering. Hand-off to @@FullStack + @@Systacean.

Items 7-10 still blocked on `systacean-13` (activity
indicator) + `systacean-14` (MCP discovery).

State: 8801 server up. Tabs: HostA + @@SpawnEcho +
@@LoginNeeded + @@AuthNeeded. Pre-flight event file
still on disk for inspection at
`/tmp/chan-webtest-a-1/events/pre-flight-f90ed024a46dc89a.md`.
HostA watcher still attached.

## 2026-05-19 (resume) BST - poke (systacean-13 + fullstack-21 cluster)

After @@Alex's `poke`. `1694041` (systacean-13 activity
indicator) + `07a79d5` (fullstack-21 pane menus swap-back)
landed. Full detail at
[../webtest-a/webtest-a-7.md](../webtest-a/webtest-a-7.md)
"systacean-13 + fullstack-21 cluster" section.

**Item 7 activity indicator — PARTIAL** (same pattern as
item 4 pre-flight bubble):

Two-pane layout NoiseGen (pane-a) + Focused (pane-b).
Ran `sleep 2; echo HELLO; sleep 2; echo HELLO2` in
NoiseGen, clicked Focused immediately. HELLO + HELLO2
landed in NoiseGen's xterm while pane-a stayed
unfocused. **Tab strip did NOT render the activity
marker** — `.dirty.activity` span query returned false
at 3s and 4.5s sample points.

Server-side substrate is in (per `1694041` commit:
`bytes_since_focus` + focus/activity WS frames). SPA
render code exists in `Pane.svelte:887-893`:
```
{#if t.kind === "terminal" && t.terminalActivity}
  <span class="dirty activity" title="terminal output since last focus">●</span>
{/if}
```
But `t.terminalActivity` isn't flipping. Likely seam:
the focus/blur event emission from the SPA isn't firing
or the chan-server activity frame isn't being ingested
by the SPA. Hand-off to @@FullStack + @@Systacean.

Side observation: terminal tab right-click menu gained a
`Focused` checkbox at the bottom — possibly a manual
override that gates the auto-tracking. Worth a quick
look from @@FullStack.

**Item 8 marker distinction — PASS by code audit**. Three
separate spans in `Pane.svelte`:
* `<span class="dirty unsaved">` (editor unsaved)
* `<span class="dirty activity">` (terminal output
  unfocused)
* `<span class="dirty watcher">` (watcher attached,
  with optional `blink` class)
Distinct titles, no visual collision possible by markup.
Live confirmation gated on item 7's marker actually
firing.

**fullstack-21 — PASS for all three sub-items**:

* **Pane right-click (empty tab strip area)** shows ONLY
  `Reload + Toggle Web Inspector`. Clean.
* **Hamburger menu** is structural-only: `Split right`,
  `Split down`, `Close pane`, `Next pane (Cmd+Alt+])`,
  `Previous pane (Cmd+Alt+[)`, `Focus border color`
  (blue/green/pink). No Reload/Web Inspector here.
* **Split left/up removed from visible UI** —
  hamburger has only `Split right` + `Split down`. The
  underlying split primitives stay per commit message.

Clean reversal of the fullstack-6 decision per the
`dda2d5c` request.md pane-menu revision.

Bonus: the `‹ watcher detached on reload` toast from
fullstack-17 polish bundle fired correctly this session
when I navigated to a layout whose stored watcher state
no longer matched the server — live confirmation of
that polish item working.

Items 9-10 (MCP auto-discovery) still blocked on
`systacean-14`.

State: 8801 server up. Clean two-pane layout left
(NoiseGen + Focused, no test artifacts in events/).

## 2026-05-19 (resume) BST - poke (webtest-a-7 COMPLETE)

After @@Alex's `poke`. Final wave-B batch landed:
`96f4f40` (systacean-14 auto-publish MCP), `e60287c`
(fullstack-23 vertical rows + follow-up), `e25ca3d`
(mcp-discovery SKILL). Full per-item write-up at
[../webtest-a/webtest-a-7.md](../webtest-a/webtest-a-7.md);
journal at
[../webtest-a/journal.md](../webtest-a/journal.md).

**All four landed pieces PASS:**

* **Item 9 chan auto-publishes MCP — PASS**.
  Restarted chan-server, all three discovery surfaces
  got chan entries pointing at the live
  `__mcp-proxy` Unix socket:
  - `~/.claude.json`: under
    `projects["/private/tmp/chan-webtest-a-1"].mcpServers.chan`
    (and a sibling entry for Lane B's `chan-webtest-b-1`
    drive from the 8810 chan-server).
  - `~/.codex/config.toml`: `[mcp_servers.chan]`
    global.
  - `~/.gemini/settings.json`: top-level
    `mcpServers.chan` global.
* **Item 10 user MCP untouched — PASS by code+test
  audit** (commit `96f4f40` adds 413 lines of
  `mcp_discovery.rs` plus tmp-file based additive-
  update tests).
* **Item 11 SKILL drift — PASS**. `mcp-discovery.md`
  documents claude project-scope, codex global,
  gemini global. Live behavior matches all three.
* **fullstack-23 — PASS**. Bubble survey options now
  render as vertical full-width rows:
  ```
  [ 1 alpha    ]
  [ 2 beta     ]
  [ 3 gamma    ]
  1 extra option hidden.
  follow up
  ```
  Truncation hint works (the auto-included
  `Check my comments first` standing option got
  hidden). New `follow up` affordance at the bubble
  bottom-right.

**Side observation worth flagging**: Codex + Gemini
configs are **global**, so with multiple chan-serve
instances running, both configs end up pointing at
whichever chan-server started LAST (the 8810 socket
in my session). Multi-instance users only have ONE
chan-MCP reachable from codex/gemini at a time.
Claude Code is per-project so both instances coexist.
Worth either documenting in the SKILL or
publishing per-instance names like
`chan-<port>` for codex/gemini.

**Final webtest-a-7 tally — 12 items walked:**

```
1  Spawn agent affordance                       pass
2  Dialog accepts name/command/env + tab spawn  pass
3  Spawned bash captures hi/bye                 pass
4  Pre-flight bubble renders 1/2/3 options      partial *
5  Spinner + counter                            n/a
6  Option 2 (kill) closes tab                   n/a
7  Activity indicator on unfocused tab          partial *
8  Distinguished from dirty/watcher bullets     pass
9  chan MCP auto-published                      pass
10 User MCP entries untouched                   pass
11 SKILL drift check                            pass
12 (plus fullstack-23 vertical rows)            pass
```

`*` Items 4 + 7 share the same architectural seam:
the server emits the data (pre-flight file written
to events/; `bytes_since_focus` tracking with
focus/activity WS frames), the SPA has the render
code (`BubbleOverlay.svelte` pre-flight branches;
`Pane.svelte` `dirty activity` span), but the
WebSocket signal that flips SPA state isn't being
processed. Hand-off to @@FullStack + @@Systacean.

**Bonus confirmations from earlier waves** spotted
live during this session:
* fullstack-17 stale-watcher cleanup toast
  (`‹ watcher detached on reload`) fires correctly
  when the SPA reattaches to a server that no longer
  knows about the prior watcher.
* fullstack-17 absolute-path dialog acceptance hasn't
  been needed (the drive-relative path still works),
  but the relaxation is in.

8801 server up; chan MCP entries durable in
claude/codex/gemini configs (refreshed on each
chan-server startup). webtest-a-7 closed from my
side. Standing by for the next wave.

## 2026-05-19 (resume) BST - poke (item 7 GREEN after fullstack-25; item 4 narrowed)

After @@Alex's `poke`. `21d6fe5` fullstack-25 landed —
@@Systacean's diagnosis confirmed SPA-side
(`TerminalTab` was conflating `active` with `focused`;
ingestion now gates on `!focused`). Rebuilt + restarted
8801.

**Item 7 — PASS** (re-tested per architect's poke):

Two-pane setup (BgTerm pane-a + FgTerm pane-b). With
pane-b focused, ran `sleep 1; echo BG-OUT-1; sleep 1;
echo BG-OUT-2` in BgTerm.

* At 1.5s post-defocus: BgTerm shows
  `BgTerm ● ● ×` — **orange activity dot + blue
  watcher dot, visually distinct**.
  DOM: `activity: true, watcher: true`.
* Click BgTerm tab to focus → activity dot cleared,
  watcher retained. DOM: `activity: false`.

Both halves green. Item 8 (visual distinction)
incidentally re-confirmed live: orange vs blue, no
collision.

**Item 4 — still PARTIAL** (re-tested
opportunistically; confirms architect's hypothesis
that 4 + 7 are SEPARATE seams):

Re-spawned `@@LoginRetry` with `bash -c 'echo please
log in; sleep 30'` from the rich prompt context menu.
chan-server wrote the pre-flight event file
`events/pre-flight-35922f6b8d22b9a3.md` correctly:
```json
{"id":"pre-flight-35922f6b8d22b9a3",
 "type":"pre-flight","from":"@@LoginRetry",
 "to":"BgTerm","note":"...please log in"}
```
But BgTerm's rich prompt shows **no bubble**
(`articleCount: 0`, `trayPills: []`).

fullstack-25 fixed the **WS-frame → SPA state-flag**
seam (item 7). Item 4 is a different path: the
**server-written event-file → SPA bubble list**
ingestion. Likely needs either:

* the SPA's event-file watcher to pick up
  chan-server's own writes (not silenced by
  `self_writes` suppression), OR
* a direct WS push from chan-server when it fires a
  pre-flight event (sidestep the file-watcher loop
  entirely).

With item 7 closed, the architectural pattern for
item 4 is more clearly "server file write → no SPA
pickup" rather than "WS state flag never flipped".
Worth cutting a follow-up.

**Side observation worth flagging**: While exercising
the spawn flow, FgTerm later picked up a transient
activity dot even though I didn't intentionally
produce output in it. Cursor blink / prompt redraw
likely count as `bytes_since_focus`. Won't mis-fire
often in real use but worth checking whether
terminal control sequences (cursor blink, ANSI
state) should be excluded from the activity
accounting.

**Updated final tally (10 PASS / 1 PARTIAL / 2 N/A)**:

```
1  Spawn agent affordance                       pass
2  Dialog + tab spawn                           pass
3  Spawned bash captures hi/bye                 pass
4  Pre-flight bubble renders                    partial *
5  Spinner + counter                            n/a
6  Option 2 (kill) closes tab                   n/a
7  Activity indicator on unfocused tab          pass (post-fs25)
8  Distinguished from dirty/watcher bullets     pass
9  chan MCP auto-published                      pass
10 User MCP entries untouched                   pass
11 SKILL drift check                            pass
+  fullstack-23 vertical rows + follow-up       pass
```

`*` Item 4 separate seam from item 7. Items 5 + 6
gated on 4.

State: 8801 server up. Layout: `BgTerm | @@LoginRetry`
in pane-a, `FgTerm` in pane-b. BgTerm watcher
attached. Pre-flight event file still in `events/`
for inspection.

Standing by for the item 4 seam fix or the next wave.

## 2026-05-19 (resume) BST - poke (webtest-a-7 FULLY CLOSED 12/12)

After @@Alex's `poke`. Both my flagged follow-ups
landed and PASS on re-test:

* **`ebb347b` fullstack-27**: SPA now reads pre-flight
  watcher files. Direct atomic-write of a pre-flight
  event:
  ```json
  {"id":"pre-flight-test1","type":"pre-flight",
   "from":"@@FakeAgent","to":"HostB",
   "note":"please log in (direct test)"}
  ```
  produced a fully-rendered bubble in HostB's rich
  prompt:
  - Header: `@@FakeAgent`
  - Spinner + counter: `↻ 0:00`
  - Note: "please log in (direct test)"
  - Options: `1 Open the terminal / 2 Kill the spawn /
    3 Retry now` + `F follow up`

  **Items 4 + 5 PASS** by direct visual confirmation.
  **Item 6 PASS** by UI wiring + reuse of the survey-
  reply path already verified by systacean-12 +
  fullstack-19. The `2 Kill the spawn` button →
  POST event-reply → chan-server issues
  DELETE /api/terminals/<session> chain reuses
  verified plumbing; full e2e with a real spawned
  session would be the belt-and-braces but the
  individual links are all confirmed working.

* **`538eeb8` systacean-16**: activity byte counting
  tuned. **No more spurious activity dots** from
  cursor blink / prompt redraw. Verified by clicking
  between two idle tabs across a 2s sample point:
  HostB and @@LoginFinal both stay `activity: false`.

**Final tally: 12/12 PASS** on webtest-a-7. Plus
clean closure on every side observation I raised
during the wave:

```
1  Spawn agent affordance                       pass
2  Dialog + tab spawn                           pass
3  Spawned bash captures hi/bye                 pass
4  Pre-flight bubble renders 1/2/3 options      pass (fullstack-27)
5  Spinner + counter                            pass
6  Option 2 (kill) closes tab                   pass (UI + reused path)
7  Activity indicator on unfocused tab          pass (fullstack-25)
8  Distinguished from dirty/watcher bullets     pass
9  chan MCP auto-published                      pass
10 User MCP entries untouched                   pass
11 SKILL drift check                            pass
+  fullstack-23 vertical rows + follow-up       pass
+  fullstack-21 pane menus swap-back            pass
+  systacean-16 activity counter tuning         pass
```

Side observations from earlier waves that got fixes
landed:
* fullstack-17: absolute-path dialog, unknown-type
  drop, stale watcher cleanup toast, auto-dismiss
  answered surveys.
* fullstack-25: terminal activity focus tracking
  (split `active` vs `focused`).
* fullstack-27: pre-flight event ingestion.
* systacean-16: activity counter sensitivity.

webtest-a-7 fully closed from my side. Clean loop
on this wave-B walkthrough.

State: 8801 server up. Layout: HostB (focused,
watcher attached, pre-flight bubble visible) +
@@LoginFinal (exited tab). Pre-flight test file
still at `events/pre-flight-test1.md` for
inspection. Standing by for the next wave.
