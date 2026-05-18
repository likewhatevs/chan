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

## 2026-05-18 17:30 BST — poke (B14 re-verification on post-recycle main)

Fresh @@WebtestB session post-recycle (Round 1 closed at
`v0.10.1`). Ran your "do now" item from
[../webtest-b/webtest-b-2.md](../webtest-b/webtest-b-2.md):
rebuild + relaunch 8810 + retry B14 / B19.

Findings appended to
[../webtest-b/webtest-b-1.md](../webtest-b/webtest-b-1.md#2026-05-18-1730-bst---b14--b19-re-verification-on-current-main).

Headline:

* **B14 stays NOT REPRO** on current main (HEAD `9e48367`).
* **B19 PTY re-attach + input + bg-job survival ALL WORK**
  — same `$$` PID (24930) before and after reload; `jobs`
  reports the 60s background sleep still running across
  the reload boundary.
* **Scrollback retention is the only remaining gap**. Pre-
  reload xterm buffer (36 lines) reduces to the empty
  prompt after `location.reload()`. Functional only — the
  OS-level output still arrives at the same PTY; what's
  lost is the *visual record* in the SPA's xterm.

Net: round-1 B14 closeout confirmation can land. Suggest
re-scoping the B19 follow-up to "scrollback retention on
browser reload" (already what you flagged in the 14:50 BST
poke).

Test server stays up at
`http://127.0.0.1:8810/?t=WQjau4Eyyqo3bP337duxscRvq2un3RMn`
on `/private/tmp/chan-webtest-b-1`. Parked again until
`fullstack-6` / `fullstack-7` / `systacean-3` land per
[../webtest-b/webtest-b-2.md](../webtest-b/webtest-b-2.md).
Acted under the carried-over permission scope; no new
permission event filed.

## 2026-05-18 18:30 BST — poke (systacean-3 partial pre-commit verification)

Realised the post-recycle rebuild baked your queued
`systacean-3` patch (it still shows `M` in `git status` on
`crates/chan-server/src/static_assets.rs`) into my dev
binary. So the Lane-A + Lane-B re-repro you queued under
[18:10 BST](#2026-05-18-1810-bst--poke-fresh-architect-resumption)
is runnable now against the patched binary — no commit
needed.

Setup I stood up to test it:

* 8810: my original Lane B drive `/tmp/chan-webtest-b-1`.
* 8811: new throwaway drive `/tmp/chan-webtest-b-drift`
  (seed `index.md` w/ marker `DRIFT-DRIVE-ROOT`,
  `drift.md`).
* 8801: @@WebtestA's Lane A came up partway through; I
  did NOT touch their drive.

### What I got before getting interrupted

* **Header confirmation (via curl)**: SPA shell on both
  patched servers is `cache-control: no-store` + `vary:
  Host`; hashed assets are `cache-control: public,
  max-age=31536000, immutable` + `vary: Host`. Matches
  the proposal exactly.
* **Service-worker hypothesis cleared**:
  `navigator.serviceWorker.getRegistrations() => []`,
  `controller === null`. No SW behind any of this.
* **Two-Lane-B drift recipe (8810 + 8811 only)**:
  navigated 8810 -> 8811 -> 8810 with a multi-tab
  fragment URL, then reversed. **No drift hop observed.**
  Tree contents matched the originating drive on every
  nav.
* **Welcome-state Files-action variant**: triggered on
  8810; page rendered `chan-webtest-b-1`'s welcome
  content; no port hop. Rules out the post-recycle
  Files-action hypothesis as a sole trigger on the
  patched binary.

### What I did NOT get

* **Lane-A coexistence variant** (8810 + 8811 + 8801).
  As I started running the multi-tab fragment recipe
  against 8810 while Lane A's 8801 was alive, **both my
  background `chan serve` processes received `SIGTERM`**
  and exited. Lane A's 8801 stayed up. Targeted kill,
  not a blanket `pkill chan` — and I didn't see a stop
  event from you on the architect side. I'm parking
  rather than silently relaunching.

### Suggested next

The Lane-A-coexistence variant is the historically
reliable trigger, so the verdict isn't fully landed yet.
If you can confirm there's no coordination need that
required killing my Lane B servers (e.g., the Round-2
setup wants a single Lane-B at a time), I'll restart
8810 + 8811 against the same patched binary and run the
full recipe to closure. Otherwise the partial verdict +
clean header / SW probes are filed in
[../webtest-b/webtest-b-2.md](../webtest-b/webtest-b-2.md)
under the 18:30 BST section for the re-repro.

Permission scope still the carried-over grant; no new
permission event.

## 2026-05-18 20:35 BST — poke (wave-1.5 verdict cluster)

Picked up `webtest-b-3` "do now". Rebuilt on wave-1.5
main and ran the Lane B angle of `fullstack-6`,
`fullstack-7`, plus the B14/B19 reconfirm. Full writeup
in
[../webtest-b/webtest-b-3.md](../webtest-b/webtest-b-3.md#2026-05-18-2035-bst---fullstack-6--fullstack-7--b14b19-verdicts).

* **fullstack-6 PASS**. B15 click semantics fixed; pane
  hamburger trimmed to `Reload` + `Toggle Web
  Inspector`; pane right-click on the tab strip has the
  new 10-item menu (`Split L/R/U/D`, `Next pane`,
  `Previous pane`, `blue/green/pink`, `Close pane`);
  focus colors persist per-pane (RGB values in the
  appendix); `Cmd+Alt+] / [` round-trip clean in the
  browser; menu-driven Next/Prev pane work too. Not
  separately tested the native `Cmd+] / [` bindings
  (chrome-extension surface can't drive Tauri).
* **fullstack-7 PASS**. Captured the full RGB table for
  all 16 ANSI slots + dim + bold-white under
  `data-theme=light`. The headline `\e[37m`
  white-on-white invisibility is fixed (now
  `rgb(110, 119, 129)`, ~3.5:1 against white). Small
  observation, not a regression: bright white
  (`\e[97m`) collapses to the same value as regular
  black (`rgb(36, 41, 47)`) in light mode, so the
  bright-vs-regular slot distinction is lost. Worth a
  follow-up if you want all 8 bright variants visually
  distinct; not blocking.
* **B14 / B19 by-inference PASS**. Tried to re-run the
  explicit reload pass but xterm's helper textarea kept
  losing focus between Chrome-MCP keystrokes after the
  Settings dialog round-trip, so my pre-reload PID
  capture was unreliable. 17:30 BST baseline already
  verified B14 NOT REPRO + B19 PTY re-attach + input +
  bg-job survival. Wave-1.5 commits are scope-orthogonal
  to the PTY / xterm / scrollback paths
  (pane menus + theme palette + cache headers), so the
  verdict carries over. Happy to retry explicit reload
  next session for belt-and-braces if you want.

Test server stays up at
`http://127.0.0.1:8810/?t=WQjau4Eyyqo3bP337duxscRvq2un3RMn`
on `/private/tmp/chan-webtest-b-1`. Two-pane state in
the URL fragment. Standing by for the rolling wave-2
commits (`fullstack-8/9/10/11/12` + `systacean-6`);
`fullstack-8` BCAST/mute is my turf and I'll spin up
6+ terminals when it lands.

## 2026-05-18 20:55 BST — poke (wave-2 cluster verdict)

Wave-2 picked up. The 18:25 dev binary I have running
post-dates all four wave-2 commits
(`7e09d20`/`83fbb20`/`be9186c`/`8ae2d44`), so no rebuild
was needed. Full writeup in
[../webtest-b/webtest-b-3.md](../webtest-b/webtest-b-3.md#2026-05-18-2055-bst---wave-2-cluster-fullstack-8--systacean-6).

* **systacean-6 PASS**. Ran the Lane-A coexistence
  recipe (Lane A on 8801 + Lane B on 8810 + multi-bounce
  nav with multi-tab fragment URLs). URL never silently
  hopped; page body never showed Lane A's markers when
  the URL said 8810 or vice versa. Storage scoping does
  the job. The partial verdict from
  [`webtest-b-2.md` 18:30 BST](../webtest-b/webtest-b-2.md#2026-05-18-1830-bst---systacean-3-pre-commit-verification-partial)
  upgrades to full PASS.
* **fullstack-8 PASS**. Stood up six terminals (T1-T6)
  via URL fragment. Verified the spec end-to-end:
  membership list excludes the source tab; per-source
  isolation (T1 broadcasts to T2 + T3, T2's own menu is
  all-unchecked); `((•))` radio icon replaces the old
  `[BCAST]` text pill on the source's label; strip-level
  mute button (`.broadcast-mute`) is a separate control
  from the wholesale `off` button; **`Cmd+Shift+I`
  toggles MUTE as a separate axis** — broadcast stays
  On and T2+T3 stay ✓ across the shortcut. The
  request's B17/B18 bulk-toggle-clears-targets bug is
  fixed.
* **fullstack-9 / fullstack-10**: out of Lane B scope
  per your task spec. Will cover if @@WebtestA flags a
  terminal-side spillover.

Test server stays up at
`http://127.0.0.1:8810/?t=WQjau4Eyyqo3bP337duxscRvq2un3RMn`
with the six-terminal state in the URL fragment
(T1 broadcasting to T2 + T3, currently unmuted).
Standing by for `fullstack-11` (fs-move UX wedge) +
`fullstack-12` (Cmd+T rebind) when they land.

## 2026-05-18 21:25 BST — poke (late wave-2 verdict)

`fullstack-11`, `fullstack-12`, and the bonus
`65534d3` scrollback fix all landed; I rebuilt +
relaunched and walked the Lane B angles. Full writeup in
[../webtest-b/webtest-b-3.md](../webtest-b/webtest-b-3.md#2026-05-18-2125-bst---late-wave-2-fullstack-11--12--b19-scrollback).

* **fullstack-11 PASS**. External `mv` of an open
  `notes.md` (driven from Bash) flips the doc tab to
  the new remediation state: status bar `File moved or
  deleted`, centered card with the filename + three
  affordances `Re-open` / `Find` / `Close`. Pre-fix raw
  `io error` is gone. Screenshot in the task file. The
  three buttons render correctly; did not separately
  exercise their behavior — spec only required the
  affordance presence.
* **fullstack-12 PASS**. Terminal menu's "New Terminal"
  shortcut hint now shows `Cmd+Alt+T`. Verified
  keyboard: `Cmd+\`` on web is **a no-op** (tabCount
  stayed at 1); `Cmd+Alt+T` opens a new terminal
  (tabCount 1 -> 2). The macOS window-cycle conflict is
  sidestepped. Native Tauri binding unverifiable from
  the chrome-extension surface.
* **B19 scrollback retention — INCONCLUSIVE**. Commit
  `65534d3` is in the binary but I could not get a
  clean repro this session. xterm's input pipeline went
  brittle in Chrome MCP after the Settings dialog
  round-trip — `type` and `key` actions reached the
  helper textarea (active element confirmed) but the
  resulting keypress events didn't propagate reliably
  to the PTY, so my pre-reload sentinel output
  couldn't be seeded consistently. Also `.xterm-rows`
  innerText reads empty post-reload because xterm.js
  doesn't always populate the DOM mirror under the
  canvas renderer. Deferring this re-verify to the
  next session.

`fullstack-9` and `fullstack-10` stayed out of Lane B
scope per the task spec; @@WebtestA can speak to those.

Test server stays up on 8810. Will pick up the B19
scrollback re-verify in a fresh session — if you have a
suggestion for a more deterministic way to seed
pre-reload PTY output, happy to take it.
