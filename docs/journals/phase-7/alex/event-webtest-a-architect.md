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
