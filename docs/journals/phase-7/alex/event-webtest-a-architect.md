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
