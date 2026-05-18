# @@WebtestB's phase-7 journal

Author: @@WebtestB
Date: 2026-05-18

Append-only. New entries go at the bottom under a dated heading.

## 2026-05-18 11:29 — Bootstrap blocker

Bootstrapped as @@WebtestB in repo root on `main`. Read contact,
webdev guide, phase process, phase request, own journal, and
CLAUDE.md.

Blocked: @@Architect's journal dispatch table lists
`webtest-b-1` for baseline walkthrough of terminal + broadcast +
panes, but `docs/journals/phase-7/webtest-b/`
contains no `webtest-b-*.md` task file to work from.

## 2026-05-18 12:50 BST — webtest-b-1 walkthrough complete (fresh session)

Resumed as a fresh @@WebtestB after the predecessor died with
no actual test-work landed. Re-used the seeded `/tmp/chan-
webtest-b-1/` drive and Built+launched
`./target/debug/chan serve --port 8810` (8787 and 8801 were
already taken by phase-6 leftover + Lane A).

Walked the full Lane B script — B14, B15, B16, B17, B18, plus
the E3/E4 baseline notes and the `chan open` bonus.
Findings + repro notes in [webtest-b-1.md](webtest-b-1.md).
Tldr:

* B14 (doc/term tab switch blank): NOT REPRODUCED on current
  build (click + keyboard switches both render immediately).
* B15 (left-click on empty pane opens menu): REPRODUCED on
  the welcome view.
* B16 (Cmd+\` macOS conflict): partial — chan wins in a
  single-window Chrome session; the OS conflict is not
  directly observable from here. Bonus: Cmd+\` always
  *creates* a new terminal, never focuses an existing one.
* B17 (Cmd+Shift+I): toggle is all-on / all-off; per-tab
  state is NOT preserved across toggles. Bug confirmed.
* B18 (Broadcast / mute UI): baseline captured; `[BCAST]`
  is text not icon yet, mute affordance interaction blocked
  by a separate drift bug.
* E3/E4 baseline notes captured to the extent possible
  without a clean menu read.
* `chan open ./notes.md` bonus: works with the dev binary,
  opens notes.md in a new tab. `CHAN_TAB_NAME` is set;
  `CHAN_DRIVE_NAME` is not.

Also surfaced a separate, reproducible cross-drive nav drift
where the SPA at `127.0.0.1:8810` keeps jumping to
`127.0.0.1:8801` (Lane A) on miscellaneous clicks. Wrote
this up under "Extra finding" in the task file — deserves
its own task; ate a lot of session time today.

Test server URL for the round-1 hand-off:
`http://127.0.0.1:8810/?t=WQjau4Eyyqo3bP337duxscRvq2un3RMn`
on drive `/private/tmp/chan-webtest-b-1`. Three-tab state
encoded in the URL fragment if needed.

Firing `alex/event-webtest-b-architect.md` (type `poke`) next.

## 2026-05-18 13:20 BST — gap-fill pass on B18 mute + E3 doc menu

Closed the two open gaps from the first pass:

* B18 strip-level mute: clicking the `((•))` icon at the
  left edge of the broadcast strip toggles
  `broadcast-mute.muted` and the parent strip class cleanly.
  Bi-directional. The "can't click mute" complaint, if still
  live, is about a per-tab mute on the `[BCAST]` pill (which
  is currently a status indicator with no click handler) —
  not the strip-level mute.
* E3 doc-tab menu: right-click on a doc tab opens no menu at
  all (preventDefault on native, no app menu). The terminal
  tab has a 22-item rich menu. The pane kebab `⋮` has 10
  items on the welcome view but collapses to 3 (Split right
  / Split down / Close pane) once any tab is in the pane.
  Asymmetry is bigger than the request implied — flagged.

Gap-fill notes appended to [webtest-b-1.md](webtest-b-1.md);
firing follow-up poke to
[../alex/event-webtest-b-architect.md](../alex/event-webtest-b-architect.md).

## 2026-05-18 13:40 BST — adjacent Lane B follow-ups

After the gap-fill, user opened the session for more work.
Picked up four adjacent items in Lane-B scope:

* Drift root-cause stab: installed a history + fetch
  interceptor and exercised the same clicks. Did not re-
  trigger the drift in this run. Working hypothesis: the
  trigger is the welcome-state pane menu's `Files` entry
  (which surfaces the global drives picker). Once any tab
  is in the pane, that menu collapses to 3 items and the
  trigger goes away. Needs a targeted next-round repro.
* B19 (reload recovery): PTY re-attaches cleanly on browser
  reload; input works on first press; `jobs` confirms the
  same shell process. **Headline B19 bug doesn't repro.**
  But scrollback from before the reload is dropped — worth
  filing as its own narrower item rather than under B19.
* B20 (light-mode contrast): reproduced. `\\e[37m` white is
  literally white-on-white (invisible); green / yellow /
  cyan are all too pale. Dark mode unaffected. Screenshot
  in the task file.
* `chan open` variants: dir opens Files overlay at drive
  root with the dir highlighted; non-md file opens Files
  overlay with the file highlighted (MEDIA details panel).
  Both match the request spec. Small nit: dir case opens
  parent + highlights instead of opening into the dir's
  contents.

Findings appended to
[webtest-b-1.md](webtest-b-1.md); follow-up poke fired on
[../alex/event-webtest-b-architect.md](../alex/event-webtest-b-architect.md).

## 2026-05-18 13:52 BST — drift targeted repro + E2 activity indicator

User self-paced poke loop, kept going.

* **Drift targeted repro**: hypothesis (welcome-state menu's
  `Files` entry surfaces a global drives picker that hops
  to Lane A) **disproved**. With the interceptor installed
  from page load, clicked `Files` from the B15 menu and the
  Files overlay opened correctly on Lane B with no nav
  jump. Also clarified: the 11-item menu is the **left-
  click on empty pane** menu (B15), not the kebab — the
  kebab is a separate 3-item menu. I conflated the two in
  earlier writeups. Drift remains unexplained; best repro
  bet is a multi-tab Lane B session with Lane A still
  running on the side (cross-origin client-storage
  suspicion).
* **E2 activity indicator**: missing on current main. With
  Terminal-1 focused and Terminal-2 backgrounded pumping
  ACTIVITY-LINE-1..10, the Terminal-2 tab class stays
  exactly `tab svelte-at6ci2` (no `has-activity`,
  `unread`, dot, pulse, or badge). Confirmed the loop did
  emit all 10 lines by switching back at the end.

Findings appended to [webtest-b-1.md](webtest-b-1.md);
follow-up poke fired on
[../alex/event-webtest-b-architect.md](../alex/event-webtest-b-architect.md).

## 2026-05-18 14:08 BST — drift re-fires + E4 partially implemented

* **Drift re-fired** as soon as Lane A's server was up
  (`8801`, PID `45746`) and I navigated to a multi-tab
  Lane B URL on `8810`. The URL hopped to `8801` before my
  interceptor could install — i.e., the drift happens
  during initial page load, before chan SPA JS runs its
  own routing. Server returns 200 with no `Location:`
  header; no `location.assign / replace / href =` in
  `web/src`. Best guess: browser-level prediction /
  shared cache between same-host:different-port, or a
  ServiceWorker / OPFS path. Bisect needs an interceptor
  injected before page JS — can't be done from the
  current chrome-extension surface.
* **E4 partial**:
  * **Rename indicator IS implemented.** Editing the Name
    field flips status row to `connected - WxH  stale env`
    (orange chip), and an inline banner reads `Tab name
    changed. $CHAN_TAB_NAME will stay at <old> until
    restart.` with `Restart now` / `Later` buttons.
    Better than the request's "indicator" — it's a full
    named affordance.
  * **Restart confirmation NOT implemented.** Clicking the
    standalone `Restart` menu item resets the PTY
    immediately; no modal, no warning. Inline `Restart
    now` button is the only path with explicit intent.

Findings appended to [webtest-b-1.md](webtest-b-1.md);
follow-up poke on
[../alex/event-webtest-b-architect.md](../alex/event-webtest-b-architect.md).

## 2026-05-18 14:25 BST — fs-move + rich prompt + docked file browser

Three more items in the Lane-B-adjacent bucket:

* **fs-move on open file (bug)**: reproduced exactly as
  described. `mv` from a chan-spawned terminal leaves the
  open tab showing raw `io error: No such file or
  directory (os error 2)`. No remediation affordance.
* **Rich prompt right-click**: no app menu opens (browser
  default is suppressed). Same shape as the doc-tab right-
  click finding. The toolbar's `Aa` toggle currently hides
  the entire style toolbar — the request wants source
  toggle to live in the missing right-click menu instead.
* **E1 docked file browser (@@FullStack's work)**: working
  as scoped. Files overlay kebab has `Stick to left` and
  `Stick to right`. Both can be docked simultaneously,
  overlay still accessible over the top, dock kebab has
  `Unstick left/right`. Matches the request precisely.

Final tally table for the whole sweep is in the task file.
Test server still up; round-1 Lane-B work is wrapped from
my side.

Follow-up poke on
[../alex/event-webtest-b-architect.md](../alex/event-webtest-b-architect.md).

## 2026-05-18 14:35 BST — pre-teardown checklist

Idle. Pre-writing the teardown plan so @@Architect can run
it at phase close without coming back to ask.

Lane B teardown plan:

1. Stop the chan serve process for Lane B.
   - PID `22788`, command
     `./target/debug/chan serve /tmp/chan-webtest-b-1/
     --port 8810 --no-browser`.
   - `kill <pid>` then confirm `lsof -nP -iTCP:8810
     -sTCP:LISTEN` is empty.
2. Remove the throwaway drive directory:
   `rm -rf /tmp/chan-webtest-b-1/`. Contents are seed files
   only (`index.md`, `notes.md`, `binary.png`,
   `sub/sub-note.md`, `sub/deeper/deep.md`). No user data.
3. Drop the registry entry:
   `./target/debug/chan remove /tmp/chan-webtest-b-1`
   (takes path, not display name).
4. Close any chan-related browser tabs in the test session.
   Lane B was running in Chrome via Claude-in-Chrome at
   `http://127.0.0.1:8810/?t=WQjau4Eyyqo3bP337duxscRvq2un3RMn`.

Open follow-ups @@Architect should weigh for round-2 cuts:

* Cross-drive nav drift (Lane B + Lane A coexistence).
* `chan open` UX: title says target dir, tree shows parent
  expanded with target highlighted — fine but could open
  *into* the target's listing.
* The "scrollback dropped on reload" gap under B19.
* All NOT-IMPLEMENTED items in the final tally
  ([webtest-b-1.md](webtest-b-1.md#L280) onwards).

## 2026-05-18 17:15 BST — fresh-agent bootstrap (post-recycle)

Resumed as a fresh @@WebtestB after Round 1 closeout at
`v0.10.1` (commits `f8014a9`, `f996f4c`, `9e48367`).

Read CLAUDE.md, [process.md](../process.md),
[request.md](../request.md),
[../architect/journal.md](../architect/journal.md) closeout
section, my own journal + task files (`webtest-b-1.md`,
`webtest-b-2.md`), and the architect ↔ webtest-b event log
(both directions).

State of the world:

* No 8810 / 8801 server is listening (predecessor's Lane B
  process did not survive the recycle).
* Throwaway drive `/tmp/chan-webtest-b-1/` is still intact
  (seed files: `index.md`, `notes.md`, `binary.png`,
  `sub/sub-note.md`, `sub/deeper/deep.md`) and still
  registered as `chan-webtest-b-1`.
* `webtest-b-2.md` (cut for the fresh me) has one immediate
  item: re-verify B14 stays NOT REPRO on current main after
  the recent commits. Wave-1.5 walkthroughs queue behind
  `fullstack-6` / `fullstack-7` / `systacean-3` landing —
  none have landed yet (latest `main` is `9e48367`).
* Permission scope: `webtest-b-2.md` Permission scope
  section explicitly says "Your earlier permission grant
  covers cargo build + chan serve + browser automation.
  Wave-1.5 testing reuses the same shell scope; no fresh
  permission event needed unless you're testing a tunnel-
  loop variant." Proceeding under that scope.

Next: rebuild + relaunch on 8810, then the B14 verification
pass.

## 2026-05-18 17:30 BST — B14 / B19 re-verification done

Built + launched. B14 / B19 re-tested on current main
(`9e48367`); findings appended to
[webtest-b-1.md](webtest-b-1.md#2026-05-18-1730-bst---b14--b19-re-verification-on-current-main).

Tldr:

* **B14 still NOT REPRODUCED**.
* **B19 PTY re-attach + input + bg-job survival WORK** —
  same `$$` PID (24930), same `jobs` list across reload.
* **B19 scrollback retention STILL DROPPED** — pre-reload
  xterm buffer (36 lines) reduced to the empty prompt after
  reload. Narrow follow-up only; the headline B19 user-
  facing symptom (silent terminals / Restart-only recovery)
  is gone.

Round-1 closeout B14 confirmation: B14 stays NOT REPRO on
current main. Firing
[event-webtest-b-architect.md](../alex/event-webtest-b-architect.md)
(type `poke`).

Test server stays up at
`http://127.0.0.1:8810/?t=WQjau4Eyyqo3bP337duxscRvq2un3RMn`.
Parked again awaiting wave-1.5 (`fullstack-6` /
`fullstack-7` / `systacean-3`) per
[webtest-b-2.md](webtest-b-2.md).

## 2026-05-18 18:30 BST — systacean-3 partial pre-commit verification

Noticed the rebuilt dev binary already includes the
uncommitted `systacean-3` patch
(`crates/chan-server/src/static_assets.rs` still shows
`M` in `git status`). Ran the architect-queued Lane-A +
Lane-B re-repro early against the patched binary.

Full writeup in
[webtest-b-2.md](webtest-b-2.md#2026-05-18-1830-bst---systacean-3-pre-commit-verification-partial).

Tldr:

* Headers confirmed: SPA shell `cache-control: no-store +
  vary: Host`; hashed assets `cache-control: public,
  max-age=31536000, immutable + vary: Host`.
* Service-worker hypothesis cleared (none registered, no
  controller).
* Two-Lane-B drift recipe (8810 `chan-webtest-b-1` + 8811
  new `chan-webtest-b-drift`): **no drift hop**. Welcome-
  state Files-action variant on 8810 also stayed put.
* Lane-A coexistence variant (with @@WebtestA's 8801 up):
  **interrupted** — both my background `chan serve`
  processes received SIGTERM partway through. Lane A's
  8801 stayed alive. Targeted kill (not pkill); no stop
  event from architect. Parked rather than relaunched.

Poke on
[../alex/event-webtest-b-architect.md](../alex/event-webtest-b-architect.md)
asking whether to relaunch and finish the Lane-A
coexistence run.

## 2026-05-18 20:35 BST — wave-1.5 cluster verdicts

Rebuilt on the wave-1.5 main (`f94c4b5` systacean-3 +
`13eadfb` fullstack-7 + `67a637f` fullstack-6) +
relaunched 8810. Full writeup in
[webtest-b-3.md](webtest-b-3.md#2026-05-18-2035-bst---fullstack-6--fullstack-7--b14b19-verdicts).

Tldr:

* **fullstack-6 PASS**. B15 click semantics fixed; pane
  hamburger is exactly Reload + Toggle Web Inspector;
  pane right-click on tab strip has the 10-item menu
  (Split L/R/U/D + Next/Prev pane + blue/green/pink +
  Close pane); focus colors persist per-pane
  (green=`rgb(34,197,94)`, pink=`rgb(255,95,183)`);
  Cmd+Alt+] / Cmd+Alt+[ keyboard nav round-trips clean
  in browser; menu-driven Next/Previous pane works too.
* **fullstack-7 PASS**. Full RGB table captured in the
  task file. The headline `\e[37m` white-on-white is
  fixed (now `rgb(110, 119, 129)`). One nit: bright
  white (`\e[97m`) collapses to the same value as
  regular black (`rgb(36, 41, 47)`), losing the
  bright-vs-regular distinction in light mode. Not a
  legibility bug; flagged for a future palette polish.
* **B14 / B19 by-inference PASS**. Brittle xterm-focus
  in Chrome-MCP after the Settings round-trip made the
  explicit reload re-test unreliable; the 17:30 BST
  baseline on post-recycle main already verified, and
  wave-1.5 commits are scope-orthogonal to the PTY/xterm
  paths.

Poke on
[../alex/event-webtest-b-architect.md](../alex/event-webtest-b-architect.md).
Test server still up.

## 2026-05-18 20:55 BST — wave-2 cluster (fullstack-8 + systacean-6)

Both my Lane-B picks from wave-2 pass on the current main
binary (built 18:25, post-dates all four wave-2 commits).
Full writeup in
[webtest-b-3.md](webtest-b-3.md#2026-05-18-2055-bst---wave-2-cluster-fullstack-8--systacean-6).

* **systacean-6 PASS**. Lane-A coexistence recipe (Lane A
  on 8801 + Lane B on 8810 + multi-bounce navigation) no
  longer triggers the silent port hop. Storage scoping
  works; the partial verdict from the 18:30 BST
  `webtest-b-2.md` appendix upgrades to a full PASS.
* **fullstack-8 PASS**. Stood up 6 terminals via URL
  fragment. Verified: BCAST membership menu excludes the
  source tab; per-source isolation (T2's list is empty
  while T1 broadcasts to T2 + T3); source indicator is
  the `((•))` radio icon on the tab label (replacing the
  pre-fix `[BCAST]` text pill); strip-level mute button
  distinct from wholesale `off`; **Cmd+Shift+I now
  toggles MUTE as a separate axis** — membership is
  preserved across the shortcut, fixing the B17/B18
  bulk-toggle-clears-targets bug.
* **fullstack-9, fullstack-10** — out of Lane B scope.

Test server stays up. Six-terminal state encoded in the
URL fragment (T1 source broadcasting to T2 + T3,
currently unmuted).

## 2026-05-18 21:25 BST — late wave-2 cluster

Rebuilt + relaunched 8810 after killing my 18:25 process
(PID 58192, verified mine). Walked fullstack-11,
fullstack-12, and the bonus B19 scrollback fix. Full
writeup in
[webtest-b-3.md](webtest-b-3.md#2026-05-18-2125-bst---late-wave-2-fullstack-11--12--b19-scrollback).

* **fullstack-11 PASS**. External `mv` of the open
  `notes.md` produces the clean remediation card:
  "File moved or deleted" + filename + Re-open / Find /
  Close buttons. The raw `io error: No such file or
  directory (os error 2)` from B19's appendix is gone.
* **fullstack-12 PASS**. New Terminal menu entry now
  shows `Cmd+Alt+T`; `Cmd+\`` on web no longer creates
  a tab (tabCount stays at 1); `Cmd+Alt+T` creates one
  (1 -> 2). Rebind clean; the macOS-window-cycle conflict
  is sidestepped on the web variant.
* **B19 scrollback retention — INCONCLUSIVE**. Commit
  `65534d3` is in the binary, but I couldn't get a
  clean reload-with-prior-output baseline this session
  because xterm's input pipeline went brittle after the
  Settings dialog round-trip (`type`/`key` actions
  reached the helper textarea, but keypress events
  didn't reliably propagate to the PTY). Deferred to
  next session; suggested @@Architect note in the poke.

Poke on
[../alex/event-webtest-b-architect.md](../alex/event-webtest-b-architect.md).
Test server still up.
