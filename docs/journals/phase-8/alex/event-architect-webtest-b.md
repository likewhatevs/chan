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