# event-webtest-b-architect.md

From: @@WebtestB
To: @@Architect
Date: 2026-05-18

## 2026-05-18 11:29 — poke

Bootstrap blocked: dispatch mentions `webtest-b-1`, but no task file exists.
See [../webtest-b/journal.md](../webtest-b/journal.md).

## 2026-05-18 12:55 BST — poke

`webtest-b-1` Lane B walkthrough complete on a fresh session
(predecessor died before any work). Verdicts + repro notes
appended to
[../webtest-b/webtest-b-1.md](../webtest-b/webtest-b-1.md);
short version in
[../webtest-b/journal.md](../webtest-b/journal.md).

Headline:

* B14 NOT REPRODUCED, B15 REPRODUCED, B16 partial (single-
  window Chrome), B17 confirmed (per-tab state not preserved
  across `Cmd+Shift+I`), B18 baseline captured.
* E3 / E4 baseline noted; E3 visual comparison still pending
  (drift blocked clean menu reads on the doc tab).
* Bonus `chan open ./notes.md` works with the dev binary.
* Extra: serious cross-drive nav drift from `127.0.0.1:8810`
  to `127.0.0.1:8801` on stray clicks — deserves its own
  task.

Round-1 hand-off URL (test server stays up):
`http://127.0.0.1:8810/?t=WQjau4Eyyqo3bP337duxscRvq2un3RMn`
on `/private/tmp/chan-webtest-b-1`.

## 2026-05-18 13:22 BST — poke (gap-fill)

Closed the two gaps I left open in the first pass on
[../webtest-b/webtest-b-1.md](../webtest-b/webtest-b-1.md).

* B18 mute: strip-level mute IS clickable on current main
  (toggles `broadcast-mute.muted` cleanly). If the
  "can't click mute" complaint is still live, it's about a
  per-tab mute on the `[BCAST]` pill, which is currently a
  status indicator only.
* E3 doc tab: right-click yields no menu at all (terminal
  tab has a 22-item one). Pane kebab is 10 items on welcome
  view, 3 items once a tab is open. Bigger asymmetry than
  the request implied — "build a doc tab menu" rather than
  "re-order the terminal menu".

## 2026-05-18 13:42 BST — poke (adjacent Lane B)

User opened the session for more work; picked up four
adjacent items. Details in
[../webtest-b/webtest-b-1.md](../webtest-b/webtest-b-1.md).

* **Drift narrowing**: trigger appears to be the welcome-
  state pane menu's `Files` entry (global drives picker
  defaulting to most-recent = Lane A). Once any tab sits in
  the pane the menu collapses to 3 items and the trigger
  goes away. Did not re-fire under interceptor; needs a
  targeted next-round repro before pointing at code.
* **B19**: PTY re-attach works, input enabled, same shell
  survives. Headline B19 doesn't repro on current main.
  Scrollback retention is the remaining gap — narrower item
  worth filing separately.
* **B20**: reproduced. `\e[37m` white is white-on-white in
  light mode (invisible); green/yellow/cyan too pale.
* **`chan open` variants**: dir + non-md file both open
  Files overlay correctly. Small nit on dir case: opens
  parent + highlights, not into the dir's listing.

## 2026-05-18 13:55 BST — poke (drift repro + E2)

* **Drift hypothesis disproved**. Clean repro with the
  interceptor: clicking `Files` from the B15 (left-click on
  empty pane) menu opened Lane B's overlay correctly, no
  hop to `8801`. Also corrected my own earlier writeups:
  the 11-item menu is the **left-click on empty pane**
  menu, not the kebab — the kebab is a separate 3-item
  menu. Drift remains unexplained; likely needs a multi-tab
  Lane B session with Lane A still running side-by-side to
  re-fire.
* **E2 activity indicator: missing**. Two terminals, output
  loop in the backgrounded one, no visual cue on the tab.
  Class stays `tab svelte-at6ci2` — no `has-activity` /
  badge / dot. Confirmed gap for the enhancement.

## 2026-05-18 14:10 BST — poke (drift re-fires + E4 partial)

* **Drift re-fires** with Lane A still running on `8801`.
  Multi-tab Lane B nav on `8810` hops to `8801` during
  page load, before my JS interceptor can install. Server
  returns 200, no `Location:` header. No
  `location.assign / replace / href =` in `web/src`.
  Same hashed JS bundle on both ports (rust-embed). Best
  guess: browser-level shared cache /
  same-host-different-port prediction, or a
  ServiceWorker / OPFS path. Bisect needs an interceptor
  injected before page JS runs — not possible from the
  current chrome-extension surface. Repro recipe + things
  to check in the task file.
* **E4**:
  * Rename indicator is implemented — `stale env` chip +
    inline `Restart now / Later` banner. Better than the
    request implied.
  * Standalone `Restart` menu item still bypasses any
    confirmation. Half of E4 done.

This wraps the round-1 Lane-B sweep from my side. Test
server still up at
`http://127.0.0.1:8810/?t=WQjau4Eyyqo3bP337duxscRvq2un3RMn`.

## 2026-05-18 14:27 BST — poke (fs-move + rich prompt + E1)

Three more items closed.

* **fs-move on open file**: reproduced. `mv` from a chan-
  spawned terminal leaves the open tab on raw `io error:
  No such file or directory (os error 2)`. No remediation
  affordance. Matches the request word-for-word.
* **Rich prompt right-click**: no menu opens. Same shape
  as doc-tab right-click. Toolbar `Aa` toggle currently
  hides the entire style toolbar; request wants source
  toggle to live in a (missing) right-click menu.
* **E1 docked file browser** (@@FullStack): **working**.
  Overlay kebab has `Stick to left` / `Stick to right` /
  `Hide Details`. Both sides can be docked at once,
  overlay still accessible (`Cmd+P`) over the top, dock
  kebab carries `Unstick`. Matches the request precisely.

Final tally table for the whole sweep is in
[../webtest-b/webtest-b-1.md](../webtest-b/webtest-b-1.md).
That's a complete enough Lane-B baseline for round 1; I'm
parked until a fresh task lands.
