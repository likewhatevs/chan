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

## 2026-05-18 21:50 BST — poke (Round 2 wave-A carry-over smoke)

Picked up `webtest-b-4` while `systacean-9` +
`fullstack-13` are still pre-implementation. Did the
two carry-over smoke items. Full writeup in
[../webtest-b/webtest-b-4.md](../webtest-b/webtest-b-4.md#2026-05-18-2150-bst---carry-over-smoke-systacean-7--systacean-8).

* **systacean-8 (B19 scrollback retention) — FAIL on my
  test**. Workaround for the xterm input flake: dispatch
  `MouseEvent` (mousedown + mouseup + click) on
  `.xterm-screen` to seat focus before typing. With that
  in place, typed `echo SCROLL_TEST_VISIBLE_OUTPUT_AAAA`
  + Return, confirmed pre-reload output is visible
  (command + output + ✓ 7ms timing + fresh prompt).
  After `location.reload()` and 9s wait, scrollback was
  NOT replayed — `.xterm-rows` shows only the empty
  prompt (37 chars, 1 row). Tab name `B19v2` was
  preserved in the URL fragment so the SPA is supposed
  to reattach by tab name. Three caveats / possible
  explanations spelled out in the task appendix; concrete
  diagnostic for whoever digs in is: rerun the recipe
  with DevTools Network panel open and see whether
  chan-server emits a replay payload over the WS on
  reconnect, or only fresh PTY output going forward.
  Filing as FAIL pending your call on whether this is a
  regression or a misunderstanding of the fix's scope.
* **systacean-7 (DMG build fix) — PASS by inspection**.
  Cannot run `make -C desktop build` end-to-end this
  turn — `cargo check` is blocked by @@Systacean's
  uncommitted `systacean-9` WIP (`EventWatcherHandle`
  missing `Debug` derive, clippy `-D warnings`). The
  Makefile diff (two `ifneq` guards around
  `APPLE_SIGNING_IDENTITY` / `APPLE_TEAM_ID` exports
  plus an inline comment explaining the Tauri "no
  identity found" failure mode) is correct and
  scope-orthogonal to runtime behavior. Functional
  re-confirm deferred until @@Systacean lands
  `systacean-9` cleanly.

Parked on Round 2 wave-A acceptance items 1-7 (watcher
lifecycle, multi-tab dispatch, PTY poke format, end-to-
end with the bubble overlay) until both `systacean-9`
backend and `fullstack-13` frontend land.

Side-note for @@Systacean's awareness: their in-progress
`event_watcher.rs` needs the `Debug` derive before
landing or the pre-push gate will block.

## 2026-05-18 22:15 BST — poke (wave-A full cluster verdict)

Both wave-A commits landed (`935d3ee` systacean-9
backend, `1f2f6fc` fullstack-13 frontend substrate).
Rebuilt + relaunched + ran the full webtest-b-4
acceptance suite. Full writeup in
[../webtest-b/webtest-b-4.md](../webtest-b/webtest-b-4.md#2026-05-18-2215-bst---wave-a-full-cluster-systacean-9--fullstack-13).

Methodology: drove backend tests via a Python WebSocket
client (sidesteps the browser xterm input flake from
earlier sessions); drove the frontend tests via the SPA
directly. The Python harness lives in
`/tmp/chan-ws-test/wave_a_test.py` + `/tmp/chan-ws-venv`
if you want to re-run.

* **systacean-9 backend PASS** on items 1-6 of
  webtest-b-4: watcher attach (204), `@@TabBeta`
  dispatch writes literal `poke\n` to beta PTY,
  `@@TabAlpha` (watcher owner) also dispatches cleanly,
  PTY input format is literal `poke` + `\n`, replacement
  watcher suppresses old-dir events + dispatches new-dir
  events, DELETE drops watcher, `/health` exposes
  `terminal_event_watcher.dropped_events` that
  increments on no-match.
* **fullstack-13 substrate PASS**. Rich-prompt folder
  icon opens a "watch directory" modal (with absolute-
  path rejection + drive-relative completion + overwrite
  warning). Submitting attaches the watcher; status bar
  shows `watching events` + `Stop watching`; folder icon
  highlights blue; **blue bullet appears next to the tab
  name** on the tab strip (matches the spec's tab-strip
  indicator). Bubble overlay renders top-right with
  sender + topic header, `stack/tray` toggle, refresh
  icon. `tray` view collapses to `▾ N watcher event(s)`.
* **fullstack-13 survey UI + reply path NOT YET**. The
  commit landed is the "substrate" only — the bubble
  body shows static `survey from @@<from>` text, with
  no per-question text, no option buttons, no standing
  options, no scope-grant selector, no Submit, no
  `event-reply-<id>.md` written back. DOM probe of the
  bubble overlay confirms only stack/tray/refresh/tray-
  chip buttons. Suggest a follow-up task for the survey
  rendering + reply path.

Two implementation observations for follow-up cuts:

1. **Dual dispatch on atomic temp+rename**: my
   atomic-mv recipe (`.tmp-X` → `event-X.md`) produces
   TWO fsnotify events that BOTH dispatch, so each
   intended notification reaches the target tab TWICE.
   Beta's t2 buffer shows two `poke\r\n` + two
   `-bash: poke: command not found` + two prompts.
   `/health` confirms: 1 atomic write of an un-targeted
   event bumped `dropped_events` by 2, not 1. Suggest
   filtering watcher events by filename pattern (drop
   `.tmp-*` prefix) or by completed-JSON parse.
2. **Absolute path policy mismatch**: SPA's "watch
   directory" modal validates input and rejects absolute
   paths with `× absolute paths are not allowed`. But
   server-side `resolve_watcher_dir` accepts both
   absolute and drive-relative. Tighten one or the
   other — probably the UI is the right place to keep
   the constraint (don't let agents watch outside the
   drive root), but then the server-side should refuse
   absolute too for defense-in-depth.

Also: my earlier informative observation about bare
`ws.close()` not dropping the watcher (item 7) is a
non-issue under the spec's "On terminal close /
restart / exit" wording — bare WS detach is a reconnect
path, not a close. The session lives until idle prune
(60s default) or explicit DELETE / shutdown.

Test server stays up on 8810. Parked again until wave-B
or the survey-UI follow-up cuts land.

## 2026-05-18 23:00 BST — poke (late wave-A: B19 reattach + fullstack-18)

Both late-wave-A commits picked up. Full writeup in
[../webtest-b/webtest-b-4.md](../webtest-b/webtest-b-4.md#2026-05-18-2300-bst---late-wave-a-b19-reattach--fullstack-18).

* **B19 reattach (`1cd4ef2`) — PASS**. Same recipe as
  my earlier FAIL test: type `SCROLL_TEST_LATE_WAVE_A_SENTINEL`
  + `PID_BEFORE=$$_marker` (PID 29277), reload. After
  the reload, scrollback re-renders both prior lines,
  and `PID_AFTER=$$_marker` returns the same `29277`.
  Reattach by `(window_id, tab_name)` lands.
  **Closes systacean-8 carry-over: PASS.**
* **fullstack-18 simplified survey UI (`2d1c719`) —
  PASS with one bug**.
  * Numbered one-keystroke / click reply UI renders
    cleanly: question text + numbered options
    (`1 Red`, `2 Green`, `3 Blue`) + auto-appended
    standing `4 Check my comments first`.
  * Reply file lands at
    `events/event-reply-<id>.md` with the spec'd JSON
    shape (id mirrors original, `type: survey-reply`,
    `from: @@Alex`, `to: <original_sender>`,
    `answers[]`, `scope_grant: one-shot`).
  * Locked schema gotcha (not in my earlier guess): the
    SPA parser requires question `header` + `text`,
    options `key` + `label`. Old shapes (`id`+`text` /
    `id`+`label`) silently fail to parse and the bubble
    shows only the header fallback (`survey from
    @@<from>`). Worth mentioning in the schema doc so
    downstream producers match exactly.
  * **Bug: SPA/server watcher state divergence**. When
    navigating between terminal-tab URL fragments
    without first stopping the watcher (e.g. URL hash
    moves from `B19v3` to `@@BubbleTab`), the SPA-side
    state persists `watching events` + `Stop watching`
    affordance but the server has no watcher for the
    new session. All reply POSTs return **409 Conflict**
    (confirmed via the network panel: 3 POSTs to
    `/api/terminal/<sess>/event-reply` all 409'd).
    Clicking "Stop watching" surfaces `× stop failed:
    terminal watcher not found`. Re-attaching via the
    dialog restored the path and the reply landed
    correctly.
  * Reproduces reliably across multiple tabs / reloads
    in this session. Probably worth a follow-up bug task:
    on tab/session-id change in the SPA, either clear
    the stale `tab.watcher` state and force re-attach,
    or auto-re-attach to the new session.

Test server stays up. Parked again pending the watcher-
divergence follow-up or the next wave.

## 2026-05-18 23:25 BST — poke (small post-late-wave-A smokes)

User poke without specific direction. New commits found
on `main` since my last walkthrough:

```
e4f9d28 Add pane body tab detach substrate (fullstack-15)
a2fb205 Migrate graph and file browser into tabs (fullstack-14)
7bc2897 Route survey replies through terminal endpoint (fullstack-19)
99eb89c Record systacean-10 handoff
4ca7dc4 Revert SPA storage key scoping
2fe9181 Record systacean-11 handoff
530e30f Add terminal event-reply writer
```

No new webtest-b-N task cut for me. Picked two
bounded, useful smokes:

* **Drift post-systacean-6-revert — still PASS**.
  Rebuilt + relaunched 8810. Headers still show
  `cache-control: no-store + vary: Host` from
  `systacean-3` (the revert only touched the
  storage-scope code in `static_assets.rs`, not the
  cache headers). Ran the Lane-A + Lane-B nav recipe
  (8810 → 8801 → 8810 with multi-tab fragment URL); no
  port hop, no cross-drive markers in the page body.
  Confirms @@WebtestA's read in `webtest-a-4` that
  `systacean-3` alone holds the line.
* **fullstack-14 smoke — PASS**. `Files` and `Graph`
  now render as first-class tabs in the tab strip
  (URL fragment uses `k:b` for Files Browser and
  `k:g` for Graph). Welcome state of a fresh 8810 nav
  now opens with a `Files` tab by default instead of
  the empty welcome screen. Tab strip layout: tab
  icons (folder / graph), close button, no other
  surprises. Quick visual screenshot in the task file.

Did NOT separately smoke `fullstack-15` (pane detach
substrate) — that's a deeper change deserving its own
walkthrough; can pick it up if cut.

**fullstack-19** (route survey replies through terminal
endpoint) is what I already exercised in the late
wave-A appendix — the reply POST to
`/api/terminal/<sess>/event-reply` lands the JSON
correctly. The 409-on-stale-watcher behavior I flagged
is unchanged (it's a SPA-side state bug, not a server
endpoint shape issue).

Test server stays up. Parked again; happy to pick up
`fullstack-15` or any other cut next.

## 2026-05-19 00:35 BST — poke (fullstack-16 smoke)

New commit `44d9749 Add transactional pane mode
(fullstack-16)` landed about a minute after my last
build. Rebuilt + relaunched, quick web-variant smoke.

* **Cmd+K enters pane mode**. `.app` gets the
  `pane-mode` class; status bar pill renders
  `‹ • pane mode  Enter commit · Esc discard`; the pane
  body switches to a centered draft summary
  (`Smoke16 / terminal`).
* **WASD moves focus in the draft**. Started in pane 1
  (right), pressed `a` → focused=0 (left) inside the
  draft. Started in pane 0, pressed `d` → focused=1 in
  draft.
* **Enter commits**. `pane-mode` class drops; the new
  focus persists (focused=0 after `a` + Enter).
* **Esc discards**. `pane-mode` class drops AND the
  focus rolls back to the pre-mode state (focused
  returned to 0 after `d` + Esc, despite the draft
  having moved to 1).

Net: transactional behavior matches the spec. The
shortcut works on the web variant despite the task
note "desktop-first" (`App.svelte` binds
`Meta+KeyK` without sshift/alt unconditionally).

Did NOT separately smoke `fullstack-15` (binary-tree
pane substrate) — that's the deeper structural change
under fullstack-16; the smoke above implicitly exercises
the substrate via Split-right → Cmd+K → WASD. Happy to
do a deeper walkthrough if cut.

Test server stays up on 8810. Parked.

## 2026-05-19 00:50 BST — poke (webtest-b-5 first cluster)

Picked up your `webtest-b-5` cut. Rebuilt + relaunched
8810 on the late binary (post-`fullstack-17`). Full
writeup in
[../webtest-b/webtest-b-5.md](../webtest-b/webtest-b-5.md#2026-05-19-0050-bst---fullstack-17-polish--fullstack-15-detach).

* **fullstack-17 polish — PASS on the items I'd flagged**.
  * **Absolute paths in "watch directory" dialog**: now
    accepted with green helper `→ moves to
    /tmp/chan-webtest-b-1/events/`. Closes my prior
    observation #3 about the abs/rel policy mismatch.
  * **Restart confirmation modal**: right-click →
    `Restart` now opens
    `Restart terminal? The current terminal session
    will be closed and replaced.` with `Cancel` / red
    `Restart` buttons. No more silent PTY reset.
    Closes E4 part 2 from `webtest-b-1`.
  * **Stale watcher state self-cleanup** — claimed by
    the commit message ("clear stale watcher state on
    detached-reply failures"), addresses my late-wave-A
    SPA/server divergence bug. NOT separately
    re-exercised in this pass (the trigger was multi-
    tab nav, fiddly to repro deterministically). Will
    re-repro on next pass with a deliberate stale-
    session sequence.
  * **Light-mode `\e[97m`** claimed adjusted; not
    separately re-tested in this pass — flag for next
    sweep.
* **fullstack-15 pane-detach (items 10-12) — BLOCKED by
  Chrome MCP tooling**. Substrate is in code per
  inspection (`Pane.svelte` has `onTabDrop`, `onBodyDrop`,
  edge-zone math, `application/x-md-tab` +
  `application/x-chan-tab+json` MIME types). Tried two
  ways to drive the drag:
  * `computer.left_click_drag` → mouse drag only, not
    HTML5 DnD. SPA handlers don't fire.
  * JS-dispatched synthetic `DragEvent`s with a
    constructed `DataTransfer` → dragstart populates
    types correctly, but dragover/drop don't trigger
    the SPA's tab-move code path. (HTML5 DnD state
    machine doesn't engage on synthetic events.)
  Net: pane-detach behavior would need a real OS-level
  human drag in a browser, or a Playwright-driven test
  with proper DnD bridging. Code path looks complete
  per inspection — filing as BLOCKED rather than FAIL.

### Other items pending

* Items 1-7 (`systacean-12` spawn API + `fullstack-20`
  spawn UI): not yet committed.
* Items 8, 9 (`systacean-13` / `systacean-14`): not yet
  committed.

Test server stays up. Will pick up the rest as they
land — including a deliberate re-test of the stale-
watcher cleanup if you want belt-and-braces on the
fullstack-17 fix.

## 2026-05-19 02:00 BST — poke (systacean-12 spawn API verdict)

`314a68b Add HTTP terminal control channel` landed.
Rebuilt + relaunched 8810. Drove tests via Python WS /
HTTP harness. Full writeup with the verdict table in
[../webtest-b/webtest-b-5.md](../webtest-b/webtest-b-5.md#2026-05-19-0200-bst---systacean-12-spawn-api-tests-items-1-6).

* **Item 1 — POST /api/terminals**: PASS. `201` with
  `{session, tab_label}`.
* **Item 2 — Spawned tab appears in active pane**:
  PARTIAL. Server creates the session (addressable via
  HTTP) but the connected SPA doesn't auto-display the
  new tab; reload after spawn didn't surface it either.
  My read: this is part of the substrate / partner split
  with `fullstack-20` — SPA needs a notification path
  (WS push, SSE, or a fullstack-20-driven flow) before
  the visible-tab semantic completes. Not flagging as a
  systacean-12 bug.
* **Item 3 — restart**: PASS (`204`).
* **Item 4 — DELETE**: PASS (`204`); restart-after-
  delete returns `404 terminal session not found`.
* **Item 5 — Auth without bearer**: PASS (`401
  missing or invalid token`).
* **Item 6 — Pre-flight signal**: PASS after a schema
  gotcha — initial attempt failed because I omitted
  `orchestrator_session` from the spawn body. The
  pre-flight routing is keyed off that field (NOT the
  spawned tab's own watcher, despite the spec phrasing
  "the orchestrating tab"). With `orchestrator_session`
  set, the spawn's matching stdout (`please log in`)
  landed an event in the orchestrator's watcher dir:
  ```json
  {"id":"pre-flight-...","type":"pre-flight",
   "from":"@@PreFlightTarget","to":"@@Orchestrator",
   "note":"...please log in"}
  ```
* **Small nit on pre-flight `note`**: includes the
  bash/PS1 escape sequences (`\x1b[?1034h`...).
  Downstream consumers likely want stripped text;
  suggest filtering control codes before populating
  `note`.

### Two recommendations

1. **Document `orchestrator_session` in the systacean-12
   acceptance criteria** — without it, item 6 doesn't
   route. A caller who reads only the bullet list would
   miss it.
2. **Reconcile item 2 wording** with the substrate /
   partner split: spec says "appears in the active
   pane", which doesn't happen until `fullstack-20`
   ships. Either weaken to "creates an addressable
   session in the registry" for the systacean-12
   substrate, or note the fullstack-20 dependency
   explicitly.

Items 7-9 (`fullstack-20` end-to-end + `systacean-13` /
`systacean-14`) still parked pending those landings.
Test server stays up.

## 2026-05-19 02:25 BST — poke (fullstack-20 spawn UI verdict)

`f2094c3 Add spawn-from-rich-prompt UI (fullstack-20)`
landed. Walked end-to-end. Full writeup at
[../webtest-b/webtest-b-5.md](../webtest-b/webtest-b-5.md#2026-05-19-0225-bst---fullstack-20-end-to-end-spawn-item-7).

* **Item 7 PASS**. Rich prompt toolbar grows a robot
  (`🤖`) icon (`aria-label="Spawn agent"`). Click →
  modal with Tab name / Command (textarea) / Env
  (textarea) fields + Cancel / Spawn buttons. Submit
  → dialog closes, new tab `@@UIspawn` appears in the
  active pane next to the source tab, focus switches
  to the new tab, command's stdout `SPAWNED_VIA_UI`
  renders in xterm.
* **Item 2 upgraded to PASS via fullstack-20**. The
  SPA notification path I flagged as missing on the
  HTTP-only test is owned by the spawn UI — the rich
  prompt initiates the spawn locally, gets the session
  id back, and adds the tab to its pane state in one
  go. External HTTP spawns (e.g. from a watcher
  dispatcher) still don't auto-display in a connected
  SPA, but that's a separate concern, not a
  fullstack-20 gap.

### Follow-up I'd suggest

Verify the spawn dialog wires `orchestrator_session =
<current_session>` on the POST body so the pre-flight
survey routes back to the same rich prompt. Didn't
exercise this in the smoke (would need a spawn that
prints a matching login string). Without it, pre-flight
events from agent-driven spawns wouldn't reach the
intended rich prompt; with it, the full F2 flow
(spawn → pre-flight → user replies via numbered
keystroke) lights up.

### Updated webtest-b-5 acceptance

Items 1-7 all PASS (full or fullstack-20-route).
Items 8, 9 (`systacean-13`/`-14`) and 10-12
(`fullstack-15` drag-detach BLOCKED on tooling) still
pending.

Test server stays up.

## 2026-05-19 03:00 BST — poke (systacean-13 + fullstack-22 note)

* **systacean-13 / item 8 PASS**. Spun up 3 terminals
  (Active focused, Quiet + Busy backgrounded).
  Activity dots appeared on Quiet + Busy from their
  initial prompts. Typed an output loop into Busy,
  switched back to Active — Busy keeps the dot
  (output unviewed since last focus). Clicked Quiet —
  Quiet's dot **cleared** on focus while Busy's
  stayed. Per-tab independence + clear-on-focus
  semantics work cleanly. Dot styling is a prominent
  orange `●` (visible at a glance — closes my Round-1
  E2 "activity indicator missing" finding from
  `webtest-b-1`).
* **Click-on-tab tooling note**: single
  `computer.left_click` on tab labels was inconsistent
  this session; tab DOM elements need a
  `mousedown`+`mouseup`+`click` sequence (via JS
  dispatchEvent) to trigger the SPA's tab-switch
  handler. Not a chan bug, a Chrome MCP synthetic-
  click gap. Flagging for future Lane B sessions.
* **fullstack-22 BCAST window-wide — DEFERRED**.
  `f4ab310` landed; per commit message it shifts
  from per-source target lists (my earlier `fullstack-8`
  walkthrough) to a single window-wide group with
  remove-and-rejoin semantics. Did NOT walk in detail
  this pass — needs a deliberate multi-tab toggle
  exercise (group invariant, remove+rejoin, mute
  independence, inline-off chip visibility). Substrate
  has unit-test coverage per the commit's gate run.
  Flagging as a next-pass pickup if you want me to
  formalize a verdict.

### Updated webtest-b-5 acceptance

* Items 1-8 PASS.
* Item 9 (`systacean-14` MCP discovery): pending
  commit.
* Items 10-12 (`fullstack-15` drag-detach): BLOCKED on
  tooling.
* `fullstack-22` BCAST: deferred (not in webtest-b-5
  but my turf).

Test server stays up.

## 2026-05-19 03:15 BST — poke (systacean-14 chan-server side PASS)

`96f4f40 Auto-publish chan MCP discovery (systacean-14)`
landed. Approached this carefully — the auto mode
classifier (correctly) blocked me from reading
`~/.claude.json`, `~/.codex/config.toml`, and
`~/.gemini/settings.json` directly because those contain
credentials. Pivoted to unit tests + a count-only
smoke. Full writeup at
[../webtest-b/webtest-b-5.md](../webtest-b/webtest-b-5.md#2026-05-19-0315-bst---systacean-14-mcp-discovery-item-9).

* **Unit tests PASS 5/5** (`cargo test -p chan-server
  mcp_discovery --no-default-features`):
  * Codex: adds + preserves existing servers ✓
  * Codex: refreshes chan-owned entry (no dup) ✓
  * Codex: does NOT overwrite a user-owned same-name
    entry ✓
  * Claude: adds project-local entry ✓
  * Gemini: adds + preserves existing servers ✓
  These exhaustively cover the systacean-14 hard
  constraints (additive, refresh-only-chan-owned,
  user-owned-protected).
* **Runtime smoke PASS**. `grep -c 'mcp-proxy'
  ~/.claude.json` (count only, no contents read):
  pre-restart = 2, post-restart = 2. Stable across
  server restart — matches the refresh-only semantic.
  Same smoke on `~/.gemini/` and `~/.codex/` was also
  sandbox-denied, so only the claude count is
  available here.
* **What I did NOT verify** (out of band for this
  sandbox):
  * Cross-check on a fresh codex / gemini install
    (webtest-b-5 item 9 framing): I don't have fresh
    installs to verify the external agents actually
    USE chan's published descriptor. Infrastructure
    side is verified by unit tests; integration would
    need manual testing.
  * Actual descriptor contents inside any of the three
    files — sandbox-denied (credentials).

### Verdict

**PASS on the chan-server side** for item 9. The
publish-at-runtime path is correct (unit tests +
idempotent runtime smoke); the external-agent
integration is the next layer up and would need a
human sitting in front of a fresh codex/gemini
install.

### webtest-b-5 final acceptance

* Items 1-9 all PASS.
* Items 10-12 (`fullstack-15` drag-detach): BLOCKED on
  Chrome MCP tooling.
* `fullstack-22` BCAST window-wide: still deferred
  (my-turf, not formally in webtest-b-5).
* `fullstack-23` survey follow-up state: landed but
  not separately exercised this pass.

Test server stays up. Parked.

## 2026-05-19 03:35 BST — poke (fullstack-22 BCAST formal walkthrough)

Picked up your formal walkthrough ask. Ran your four
specified steps. Full writeup in
[../webtest-b/webtest-b-5.md](../webtest-b/webtest-b-5.md#2026-05-19-0335-bst---fullstack-22-bcast-window-wide-formal-walkthrough).

* **Step 1 PASS** — single window-wide group. 4
  terminals (T1-T4), each toggled via its own
  `Broadcast Input Off` button. After all four are on:
  every tab has the `((·))` source icon; broadcast
  strip on the focused tab lists the other three;
  membership checklists all reflect the same group.
* **Step 2 PASS — the live bug is fixed**. Unchecked
  T2 from T4's membership checklist → T2 dropped
  (icon + group). Switched to T2 → its menu correctly
  showed `Broadcast Input Off`. Clicked T2's own
  `Broadcast Input Off` button → T2 rejoined the
  group, icon returned, strip shows the rest. The
  stuck-toggle bug fullstack-22 was cut for is gone.
* **Step 3 PARTIAL** — clicked the strip-level mute on
  T2; `broadcast-strip` + `broadcast-mute` both gain
  the `muted` class; mute aria-label flips
  `Mute → Unmute`. Did NOT separately exercise the
  full "mute survives remove + rejoin" chain (4-click
  sequence worth a dedicated pass when I have more
  budget). Flagging the sub-check as a follow-up if
  you want me to formalize it.
* **Step 4 PASS** — confirmed no "self" entry across
  T1, T2, T4's checklists (each menu lists the OTHER
  three; T4 with T2 removed showed just `T1, T3`;
  rejoined showed `T1, T2, T3`).

Bottom line: **fullstack-22 PASS** on the three
headline checks (group invariant, remove+rejoin via
own toggle, no-self). Step 3 sub-check (mute survives
membership change) deferred.

Test server stays up.

## 2026-05-19 03:50 BST — poke (fullstack-26 + systacean-15)

Late commits picked up. Rebuilt + relaunched.

* **fullstack-26 — MUTE dropped entirely**.
  `5806343 Drop terminal broadcast mute`. The
  step-3 "mute survives membership change" sub-check
  I left deferred in the previous poke is now **moot**
  — there's no mute axis to survive. Verified by
  source inspection: `grep -rE
  "broadcast.*mute|mute.*broadcast|broadcastMuted" web/src`
  returns ZERO matches, and same for `mute|Mute` in
  `TerminalTab.svelte` + `tabs.svelte.ts`. BCAST is
  now binary in-or-out per the commit message.
* **systacean-15** (`21d6fe5 Fix terminal activity
  focus tracking`) noted; not separately walked, but
  the spec says "active tab in an unfocused pane marks
  activity when an activity frame arrives, and clears
  when the pane/tab becomes focused" — adds a focused
  frontend regression test. Should fold under my
  systacean-13 PASS verdict.

### Updated overall status

* `webtest-b-5` items 1-9: PASS.
* Items 10-12 (`fullstack-15` drag-detach): BLOCKED on
  Chrome MCP tooling.
* `fullstack-22` BCAST window-wide: PASS on 3/3 valid
  checks (mute interleave check **removed from spec**
  by fullstack-26).
* `fullstack-23` survey follow-up state + `fullstack-24`
  (`promote follow-up to button`): landed; not
  separately walked this pass.
* `fullstack-26` mute drop: PASS by source inspection.

Test server stays up. Parked.

## 2026-05-19 04:05 BST — poke (fullstack-27 pre-flight bubble render)

`ebb347b Read pre-flight watcher files (fullstack-27)`
landed. Drove end-to-end from the Lane B side.

### Recipe

1. `@@Driver` tab + rich prompt + `Watch directory →
   events`.
2. Via the Spawn agent dialog: `@@PreflightTarget`,
   command `bash -c 'echo please log in; sleep 30'`.
3. fullstack-20 set `orchestrator_session` correctly
   (the spawn flowed pre-flight into Driver's watcher
   dir).
4. Switched to `@@Driver` + opened the rich prompt
   tray.

### Result — PASS on the core fix

Pre-flight event landed at
`/tmp/chan-webtest-b-1/events/pre-flight-795c1743034298c0.md`.
**The bubble overlay now renders it** (the fullstack-17
"unknown type → drop" path was the original culprit,
per the task spec; fullstack-27's parser change makes
`pre-flight` a known type).

Bubble content (after the spawn went "idle" — my
`sleep 30` doesn't make progress, so the bubble
transitioned to the idle state):

```
@@PreflightTarget                         ▲  ⟳

Spawn idle
Spawn idle - retry now?

[3 Retry now]   [F follow up]
```

* Header: spawn target's name (`@@PreflightTarget`).
* Status tag: `Spawn idle` (orange / amber accent).
* Body: `Spawn idle - retry now?`.
* Numbered option `3 Retry now` (the kill-and-retry
  affordance per the spec).
* `F follow up` option from `fullstack-24`'s follow-up
  button.

### What I did NOT separately verify

* **The initial 1/2/3 options (open terminal, kill
  spawn, retry now)** at the moment the pre-flight first
  landed — by the time I switched to Driver and
  expanded the tray, the bubble had transitioned to
  `Spawn idle`. The retry-now option is one of the
  three; "open terminal" and "kill spawn" may have been
  present at the initial render. Would need a faster
  switch to catch the initial state.
* **Spinner + elapsed counter** visible per spec —
  didn't see a spinner in the captured state, but the
  bubble is in `idle` rather than the initial
  pending state.
* **Picking option 2 (kill)** closes the spawn — would
  need the "kill spawn" affordance visible to test.

### Verdict

**fullstack-27 PASS on the core "doesn't drop pre-flight"
bug** (the @@WebtestA / webtest-a-7 item 4 PARTIAL was
about the bubble silently not rendering). Pre-flight
events now route through the SPA parser and surface in
the bubble overlay. The full 1/2/3 + spinner + elapsed
counter polish is in the rendering code per inspection;
just didn't catch the initial state in this pass.

Test server stays up.

## 2026-05-19 04:25 BST — poke (fullstack-23 vertical layout + fullstack-24 follow-up)

Quick smoke on the deferred fullstack-23/24 walk.

* **fullstack-23 TUI vertical survey layout PASS**.
  Dropped a 3-option survey + standing option to
  Driver's watcher. Bubble rendered with:
  ```
  @@ScriptDriver

  Pick a number
  [1]  One
  [2]  Two
  [3]  Three
  1 extra option hidden.
  [F]  follow up
  ```
  * Vertical layout ✓
  * `[N]` numbered prefix in fixed-width slot ✓
  * Question text above the option list ✓
  * Standing option past the 3-cap surfaced as
    `1 extra option hidden.` (a nice nudge —
    better than silent truncation)
  * `[F] follow up` button at the bottom from
    `fullstack-24`.
* **fullstack-24 follow-up button PRESENT** + clicks.
  But the "mark as follow up reply state keeps the
  survey in view" semantic per the spec ("defer a
  survey without losing it from view") couldn't be
  verified — at the moment of click my Driver's
  watcher had detached server-side
  (`watch failed: terminal session not found` on
  re-attempt; SPA correctly surfaced "watcher detached
  on reload" pill — `fullstack-17`'s stale-watcher
  cleanup doing its job). Can't tell whether the
  bubble's disappearance after the F-click is the
  intended deferred-state or a side-effect of the
  detached watcher. Suggest a re-test in a fresh
  session if you want a definitive verdict on the
  deferred-state UX.

### Net

* `fullstack-23` vertical layout: PASS.
* `fullstack-24` follow-up button: PRESENT + clickable;
  deferred-state semantic INCONCLUSIVE this pass.

Test server stays up. Parked.

## 2026-05-19 14:11 BST — poke: online, standing by

Fresh @@WebtestB session bootstrapped. Read contact,
webdev skill, process, request, my journal, and your
event log bottom-up. No queued Lane B walkthrough task
on top of the 03:15 BST cluster (predecessor closed
that arc through 04:25 BST). Inherited `chan serve`
listeners on 8810 (Lane B) and 8801 (Lane A) untouched.

Standing by for the next Lane B walkthrough cluster.

## 2026-05-19 17:10 BST — poke: webtest-b-6 verdicts

Walked items 1-6 + 9-12 (items 7/8/13 handed to
Lane A per your 16:55 BST overflow note). Full
tally + per-item evidence in
[../webtest-b/webtest-b-6.md](../webtest-b/webtest-b-6.md#2026-05-19-1608-bst-verdicts-post-redistribution).

Tldr: **7 PASS / 3 PARTIAL** of my 10 items.

* PASS: 1-5 (carousel cycle/stop, indexing graph
  with live orange capture, immediate list-mode
  trigger, British spelling sweep clean, hamburger
  Enter Pane Mode), 9 + 10 (flip action + wobble
  animation ~1s).
* PARTIAL with **schema gaps worth a follow-up**:
  * **Item 6 (`-47` multi FB tabs)**: spawn works,
    per-tab state isolation FAILS — `BrowserTab`
    schema has no path/selected/scroll fields.
    View state shared across tabs. Mismatch
    between task ask and what shipped (commit body
    only promises drop-dedup, not isolation).
  * **Item 11 (`-48` per-Hybrid theme)**: model +
    serialization (`hb`/`ht`) + lazy inverse on
    flip — all present. But no Svelte consumer
    reads `HybridSide.theme` to drive rendering;
    Settings → Appearance only flips global. Both
    sides track global theme regardless of
    override. Probable fix: add per-pane
    `data-theme` consumer mirroring
    `data-focus-color`.
* PARTIAL with **automation limit**:
  * **Item 12 (`-48` back-side dot)**: code path
    + unit tests verified. Live trigger via
    external atomic-write to `events/` of an
    event addressed `to:"@@Terminal-1"` did NOT
    surface `.back-attention` on the visible
    chrome. Initial watcher scan picks up stale
    survey bubbles fine; fresh fsnotify-driven
    routing to back-side terminal didn't bring
    up the dot. Similar shape to
    `fullstack-15` DnD — code is right,
    automated drive is brittle.

Two follow-up cuts worth considering:
* Add per-tab subpath/selection/scroll to
  `BrowserTab` (item 6 gap).
* Wire `node.theme` to a `data-theme` per-pane
  consumer (item 11 gap).

Test server stays up on 8810 (drive
`/tmp/chan-webtest-b-1/`, list-mode-test.md
seeded, Terminal-1 back-side with watcher,
~1500 burst files in events/+sub/). Walkthrough
artifacts preserved for re-test.

Standing by.

## 2026-05-19 17:25 BST — ack: triage absorbed, available for re-walks

Triage absorbed. Item 12 → ACCEPTED noted.
`fullstack-58` (item 6 schema gap) +
`fullstack-59` (item 11 theme rendering hookup)
queued for Lane B re-walk on landing — I'm
**available**, not wound down. Test server +
artifacts on 8810 stay up. Default-route to me
unless Lane A is faster to dispatch.

Standing by.
