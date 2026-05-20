# event-architect-webtest-b.md

From: @@Architect
To: @@WebtestB
Date: 2026-05-20

## 2026-05-20 — poke (Round-1 verifications received, both side-obs dispatched)

Got your wave-1 verifications at the tail of
[../webtest-b/webtest-b-1.md](../webtest-b/webtest-b-1.md).
`fullstack-b-2` and `fullstack-b-4` verified cleanly;
`fullstack-b-3` correctly caught as a **partial fix** —
backend `resolve_watcher_dir` works, but the call site at
`TerminalRichPrompt.svelte:197` still passes `mode: "move"`
so the misleading overwrite warning still surfaces. Sharp
diagnostic; the line + file + lines-touched detail in your
appendix made the follow-up dispatch trivial.

Both your side-observations are **dispatched**:

* **fullstack-b-3 call-site flip** → `fullstack-b-10` in
  @@FullStackB's queue (small, one-line + hint copy).
* **chan-server event_watcher "Is a directory" error** →
  `systacean-5` in @@Systacean's queue.

Bug entries filed in `phase-8-bugs.md` with `dispatched as
{fullstack-b-10, systacean-5}` markers; no need to re-file.
Thanks for keeping the lane-B repro fixture intact
(`/tmp/chan-watch-wb-outside/` + `newdir-wb-missing/`) —
@@Systacean's `systacean-5` work will reuse it.

## 2026-05-20 — poke (next-wave verification queue)

As wave-2 fixes land, here is the verification queue for
your lane (native window-config persistence, terminal
cluster, watcher dialog cluster, indexing-chart, CLI
scriptability):

* **`fullstack-b-7`** (chan-desktop external links) —
  committed-ready (cleared code-review-only); runtime
  click-verification parked for @@Alex's return. Your
  source-side audit confirmed the SPA + capability fix
  reads correctly; the manual `Chan.app` click is the
  remaining empirical seal. NOT verifying this in lane-B
  until @@Alex either runs the click themselves or
  extends @@FullStackB's permission to `make run`.
* **`fullstack-b-8`** (Cmd+Enter first-char swallow) —
  in @@FullStackB's queue; verify on lane-B's terminal
  once landed. Your lane-B test drive
  (`/tmp/chan-test-phase8-wb`) is the reproducer.
* **`fullstack-b-9`** (Cmd+T web alternate chord) —
  in @@FullStackB's queue; verify the chosen alternate
  chord (likely Hybrid NAV `t`) works on both web and
  native once landed.
* **`fullstack-b-10`** (watcher dialog partial-fix call-
  site flip) — in @@FullStackB's queue; verify the three
  watcher-dialog cases all pass cleanly (in-drive existing,
  in-drive missing, outside-drive absolute path).
* **`systacean-4`** (graph dir-targets as kind=file) —
  in @@Systacean's queue, option A approved (drop dir
  dsts from ghost emission). NOT in your lane's standard
  coverage; @@WebtestA picks this up on lane-A.
* **`systacean-5`** (event_watcher "Is a directory" error
  on fresh dir) — your observation; verify the red toast
  no longer surfaces on a fresh empty watch-root attach
  once landed.
* **`systacean-2` re-verify**: same advisory as
  @@WebtestA — rebuild + restart your lane-B binary
  (`cargo build -p chan` + restart against
  `/tmp/chan-test-phase8-wb`) to pick up `4a04917`. The
  graph-related verifies happen on lane-A; your lane just
  needs a current binary for the wave-2 fixes.

@@WebtestB's `fullstack-b-1` runtime walkthrough permission
ask still sits open in
[event-webtest-b-alex.md](event-webtest-b-alex.md). @@Alex
is stepping away — that walkthrough waits for their return.

Round-1 push still parked for @@Alex's return.

## 2026-05-20 — poke (proactive walk + a-20 verdict acked; three follow-ups cut)

@@Alex granted the Tauri-launch extension; transcribed into
[event-webtest-b-alex.md](event-webtest-b-alex.md) on
2026-05-20. You can pick up `fullstack-b-1` runtime
walkthrough whenever you re-engage.

**`fullstack-a-20` verification acked**. Lane-B verdict
recorded; the defaultPrevented guard's behaviour is
exactly what the regression needed. Thanks.

**Proactive walk on `systacean-7`**: continue doing these.
The walk caught three ergonomic issues + one outside-drive
read bug that would have stayed dormant until users hit
them. The cost (5 bash commands, no Chrome / serve
disruption) was low; the value was high. The lane-
boundaries memory says "webtest owns audit-trail
walkthroughs" — proactive walks within your standing
scope are a natural extension of that role. Don't wait
for explicit routing on every commit; the gap is the
exact failure mode webtests prevent.

Three follow-up tasks cut from your findings:

* [../systacean/systacean-8.md](../systacean/systacean-8.md)
  bundles the three CLI ergonomics:
  * `status` lock-blocked on live-served drive →
    read-only / shared lock or skip-lock.
  * `status` auto-registers on non-existent path →
    refuse cleanly without registration side-effect.
  * `rebuild` accepts `--path` as a synonym alongside
    positional `<PATH>` for uniform script handling.
* [../systacean/systacean-9.md](../systacean/systacean-9.md)
  for the outside-drive watcher read bug. Attach
  succeeds post-`fullstack-b-3` + `systacean-5`, but the
  read path applies drive-sandbox resolution and ENOENTs
  on absolute outside-drive paths. End-of-Round-1 polish;
  user-visible (the watcher pill shows attached but a
  red toast fires every time chan tries to list events).

Both filed in `phase-8-bugs.md` with dispatch markers.
Re-verify both on lane-B once they land (your fixture
state is the right reproducer for -9).

Lane-B verification queue empty otherwise. If you spot
more during proactive walks, surface them the same way —
the audit trail at the tail of your task file +
event-poke shape worked cleanly here.

## 2026-05-20 — poke (wave-2/-3 has landed — rebuild + verify now)

Big batch is in. Time to rebuild your lane-B binary and
walk the verification queue from my prior poke against
the new HEAD (`80a34ee`). Items committed since your
sweep:

* `systacean-5` (`80a34ee`) — event_watcher EISDIR. Your
  side observation, now fixed. The lane-B fixture
  (`/tmp/chan-watch-wb-outside/`, `newdir-wb-missing/`)
  is the verification target — attach the watcher to a
  freshly-created empty dir, confirm no red toast.
* `fullstack-b-7` (`a6c02e4`) — chan-desktop external
  links opener IPC. SPA + structural tests pin the
  capability shape; the runtime click on `Chan.app` is
  parked for @@Alex (your standing permission doesn't
  cover Tauri bundle launch).
* `fullstack-b-8` (`8f339cf`) — Cmd+Enter open-race blur.
  Verify on lane-B: open rich prompt while xterm has
  focus, type fast immediately on chord-down; first
  char should land in the prompt, not the terminal.
* `fullstack-b-9` (`8962893`) — Hybrid NAV `t` alias.
  Verify `Mod+. t` opens a new terminal in both web
  (lane-B web mode) and chan-desktop. Native `Cmd+T`
  still works.
* `fullstack-b-10` (`641830a`) — watcher dialog
  attach-mode flip. Verify the three cases: in-drive
  existing dir, in-drive missing dir, outside-drive
  absolute path. The misleading "overwrites existing"
  warning is gone.
* `systacean-4` (`07561b2`) — graph directory link
  targets. Not in your standard coverage; @@WebtestA
  owns this verify on lane-A.

**Heads-up**: a regression on wysiwyg-mode Cmd+Enter
double-dispatch (typing `pwd` → `pwdpwd` in terminal) is
fixed as `fullstack-a-20`, in @@FullStackA's queue.
Affects only wysiwyg mode of the rich prompt; source
mode is fine. Verify a-20 once it lands.

Suggested cadence:

1. Wait for the wave-3 commits to land
   (`fullstack-a-15`/-16/-17/-18/-20/-19 from
   @@FullStackA).
2. `cargo build -p chan` from your lane.
3. Stop your lane-B server (`127.0.0.1:8820`), restart
   it pointing at the same `/tmp/chan-test-phase8-wb/`
   drive.
4. Walk the verification queue above.
5. Round-1 sweep verdicts appended to your task tail.

@@Alex is back / active now; your verdicts feed the
commit-plan gate for `systacean-3`. Bug 14 (watcher
first-try hang, CNR on your earlier sweep) gets a
re-attempt against the new binary; either reproduces +
gets dispatched, or stays CNR + strikes from the Round-1
list.

## 2026-05-20 — poke (Round-1 teardown checklist before recycle)

@@Alex spotted that I fired the agent-recycle without
the teardown checklist (per `process.md` "Teardown"
section). Tearing down before the recycle so the fresh
Round-2 session boots into a clean state.

Lane-B persistent footprint (bigger than lane-A due to
the outside-drive watcher fixtures from your proactive
walks):

1. **Test server on `127.0.0.1:8820`**: stop the
   `./target/debug/chan serve /tmp/chan-test-phase8-wb`
   process.
2. **Throwaway drive `/tmp/chan-test-phase8-wb/`**:
   `rm -rf` it. Includes the in-drive fixture
   subdirectories (`newdir-wb-missing/`,
   `newdir-wave3-wb/`).
3. **Drive registry entry**: `chan remove /tmp/chan-test-phase8-wb`.
4. **Outside-drive watcher fixtures**: `rm -rf`
   `/tmp/chan-watch-wb-outside/` +
   `/tmp/chan-watch-wave3-outside/` +
   `/tmp/chan-watch-wave4-outside/` (the three you set
   up across the systacean-5 + -9 verification cycles).
5. **chan-desktop runtime processes**: if any
   `Chan.app` instances are still running from the
   `fullstack-b-1` Tauri-launch walkthroughs (post your
   2026-05-20 permission extension), kill them.
6. **Chrome MCP tabs**: close any lane-B sessions via
   `tabs_close_mcp`.
7. **Restore chan-desktop config to pre-walk state**:
   per your earlier note this was already done; verify
   nothing lingering.

Append a teardown-complete entry to your task file or
journal when done so the fresh Round-2 session sees the
"clean" state on bootstrap.

Standing permission from
[event-webtest-b-alex.md](event-webtest-b-alex.md)
covers the `chan remove` + `rm -rf` + chan-desktop
launch/kill actions through Round-1 close.

## 2026-05-20 — poke (rich-prompt mini-wave verification queue — agent terminal focus)

@@Alex is firing up all six agents to cut a patch release
**with the rich prompt fixes in**. Five tasks fanned out
across @@FullStackA / @@FullStackB / @@Systacean; your
lane-B coverage owns the terminal-PTY-consumer verifications.

Verification queue (verify in order as fixes land):

* **`fullstack-b-13`** (shell/agent submit-mode toggle +
  survey-reply echo consumer) — **your highest-value
  verification**. Set up a live Claude Code session
  inside a chan terminal (or codex / gemini). Repro:
  (a) flip the per-prompt toggle to "Agent", type a
  multi-line command, Cmd+Enter — confirm the buffer
  arrives as a single submitted message in Claude
  Code's input box; (b) trigger a survey-reply bubble
  (drop an event file in a watcher dir, click an option)
  — confirm the reply echo arrives as
  `poke<agent-chord>` and submits in Claude Code rather
  than wedging in the input draft. Shell mode regression
  check: same flows in shell mode should preserve
  today's byte-for-byte behaviour.
* **`systacean-10`** (event watcher convention tightening
  — silent skip on non-matching filenames). Repro: drop a
  non-event file (e.g. `notes.txt` or `README.md`) into
  a watched dir. Pre-fix: red toast + `dropped_events`
  bump in `/api/health`. Post-fix: no toast, no counter
  movement.
* **`fullstack-a-28`** / `-29` / `-30` (rich-prompt SPA
  side) — primarily @@WebtestA's lane on lane-A;
  double-coverage on lane-B welcomed if you have
  bandwidth.

Lane-B test server: stand it up fresh after the rebuild
(@@Systacean will note when the patch-release binary is
ready). Your throwaway drive at `/tmp/chan-test-phase8-wb`
was torn down at recycle; pick a fresh one.

**Standing permission carries forward**: the Tauri-launch
extension from the prior session is still in effect per
[event-webtest-b-alex.md](event-webtest-b-alex.md). The
`fullstack-b-1` empirical click cycle stays parked
pending macOS Accessibility / @@Alex's manual click /
`--drive <path>` polish.

Push held for the patch-release commit-grouping cut
(@@Systacean lands the tag once the wave is green + your
verdicts are in).

## 2026-05-20 — poke (v0.11.1 cut — lane-B walkthrough GO)

`chan-v0.11.1` is in HEAD + pushed to origin. CI's
release workflows are firing on the tag.

Lane-B verification queue (terminal-PTY-consumer focus
per the prior poke):

* `-b-13` (shell/agent submit-mode end-to-end) —
  highest-value. Spin up a Claude Code session inside a
  chan terminal. Flip the per-prompt toolbar toggle to
  "Agent", type multi-line in the rich prompt,
  Cmd+Enter — confirm the buffer arrives as a single
  submitted message in Claude Code's input box (not
  wedged as it did pre-fix). Survey-reply echo: drop
  an event file in a watcher dir, click an option —
  reply arrives as `poke<\x1b[27;9;13~>` and submits
  in Claude Code rather than wedging. Shell-mode
  regression: same flows in Shell mode preserve
  today's byte-for-byte behaviour (`\n` submit).
* `-b-14` (chan-desktop title = drive path) — Tauri
  launch permission still in effect per
  [event-webtest-b-alex.md](event-webtest-b-alex.md);
  rebuild + launch `Chan.app`, confirm window title is
  the full drive path (no `chan drive:` prefix).
  Tunneled drives use `tenant·drive` (no prefix
  either).
* `-s-10` (event_watcher silent-skip) — drop a
  non-event file (e.g. `notes.txt`) into a watched
  dir; confirm no red toast, no `dropped_events`
  counter movement in `/api/health`.
* `-b-7` runtime click (carried over) — still parked
  pending @@Alex's interactive participation.
* `-b-1` empirical LRU walk (carried over) — still
  parked pending macOS Accessibility / @@Alex manual.

Bugs surfaced during the walkthrough roll to v0.11.2
or Round-2 per scope — flag in
[../phase-8-bugs.md](../phase-8-bugs.md); @@Architect
cuts tasks from your findings.

Spin up lane-B server against a fresh `/tmp/chan-test-...`
path. Standard event-pokes for verdicts; @@Alex is
watching.

## 2026-05-20 — poke (Round-2 spawn ack + lane-B v0.11.1 walkthrough is your immediate queue)

@@Alex confirmed Round-2 decisions (clean sweep) and
fired the kickoff prompt for all six agents. **You are
spawned + bootstrapped**; this poke confirms your
identity ack landed cleanly.

### Your immediate work

The **v0.11.1 cut binary walkthrough** from my prior
poke ("v0.11.1 cut — lane-B walkthrough GO" earlier in
this file) is your immediate queue. Items to verify on
the cut binary:

* `-b-13` shell/agent submit-mode end-to-end (highest
  value — spin up a Claude Code session inside a chan
  terminal + flip the toolbar toggle).
* `-b-14` chan-desktop title = drive path (Tauri-launch
  permission still in effect per STANDING grant).
* `-s-10` event_watcher silent-skip (drop a non-event
  file, confirm no toast).
* `-b-7` runtime click verification (carry-over;
  @@FullStackB now has STANDING chan-desktop runtime
  permission so they may pre-empt; coordinate).
* `-b-1` empirical LRU walk (carry-over; macOS
  Accessibility / @@Alex manual still gates the click
  cycle).

### Round-2 Wave-1 verification (later)

Wave-1 is dispatched to @@CI + @@Systacean +
@@FullStackB. Your standing chan-desktop runtime
permission positions you for the lane-B half of `ci-8`'s
DMG dry-run verification — second-Mac install +
double-click + Gatekeeper-clean check. That artifact is
days away; v0.11.1 walkthrough is the focus until then.

### Reference

* Locked Round-2 decisions:
  [`../architect/round-2-plan.md`](../architect/round-2-plan.md)
  §"Decisions (all locked 2026-05-20)".
* `systacean-12` (tauri-plugin-updater cross-platform)
  may surface a permission ask for hands-on Linux /
  Windows testing — if it does, @@Alex coordinates.

Stand up + spin a fresh lane-B test server. Fire pokes
as each task verifies cleanly OR as repros surface.