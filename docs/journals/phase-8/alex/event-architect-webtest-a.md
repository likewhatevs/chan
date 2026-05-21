# event-architect-webtest-a.md

From: @@Architect
To: @@WebtestA
Date: 2026-05-20

## 2026-05-20 — poke (Round-1 sweep verdicts received, three new bugs already in flight)

Got your Round-1 sweep summary + the three new-bug
observations at the tail of
[../webtest-a/webtest-a-1.md](../webtest-a/webtest-a-1.md).
Sharp work. The "fix-holds-on-HEAD" verdict batch (bugs 1,
2, 4, 5, 7, 9, 10, 18, 19, 21) is exactly the audit anchor
the wave-1 commits need.

The three new-bug observations are **already dispatched**:

* **Cmd+Enter eats first character** → `fullstack-b-8`
  (terminal-side, in @@FullStackB's queue).
* **Cmd+. p / Cmd+K p focus race** → `fullstack-a-17` (rich-
  prompt cluster, in @@FullStackA's queue).
* **Hybrid NAV help "Stage:" copy** → `fullstack-a-16`
  (copy-only, in @@FullStackA's queue).

Bug entries filed in `phase-8-bugs.md` with `dispatched as
fullstack-{a,b}-N` markers; no need to re-file. Thank you
for surfacing them with task-cuttable detail.

## 2026-05-20 — poke (next-wave verification queue)

As wave-2 fixes land, here is the verification queue for
your lane (file-browser tab, status bar, Cmd+K cluster,
rich-prompt cluster, editor cluster, graph):

* **`systacean-2` re-verify** (bug 8): already committed
  at `4a04917`. Rebuild your lane-A binary
  (`cargo build -p chan` + restart `./target/debug/chan
  serve /tmp/chan-test-phase8-wa/ ...`), then re-pull
  `/api/graph?scope=drive` and check whether the 5 plain
  non-markdown files still flag as missing. Expect them
  to clear. The 3 directory-typed-as-file cases are a
  separate path now scoped under `systacean-4` (option A:
  drop dir dsts from ghost emission) — re-verify those
  after `systacean-4` lands.
* **`fullstack-a-13`** (editor image-insert reflow) —
  committed; needs your verification. Open README.md
  (or any long doc), Cmd+End, type `![](./test-image.png)`,
  confirm the caret stays in view after the image decode.
  The fix lives in `web/src/editor/widgets/image.ts`
  load-handler.
* **`fullstack-a-12`** (graph inspector second-ghost) —
  in @@FullStackA's queue; verify once landed. Pair the
  verification with the bug-8 re-verify above (same drive
  seed, same graph slide).
* **`fullstack-a-14`** (rich prompt re-open focus) — in
  @@FullStackA's queue; verify once landed.
* **`fullstack-a-15`** (`.md.md` double extension) — in
  @@FullStackA's queue; verify once landed.
* **`fullstack-a-16`** (Stage: copy) — your observation;
  verify the help-overlay text matches the immediate-
  commit verb once landed.
* **`fullstack-a-17`** (Cmd+K p focus race) — your
  observation; verify rich-prompt keeps focus on
  Cmd+K p once landed.

Lane-A server URL forwarded to @@Alex via
`event-architect-alex.md` 2026-05-20; @@Alex is stepping
away for a while and will click around on their return.
Keep the server up unless you tear it down for a binary
rebuild — coordinate via this event file if you do.

Round-1 push still parked for @@Alex's return; nothing
goes to GitHub until they cut the build.

## 2026-05-20 — poke (wave-2/-3 has landed — rebuild + verify now)

Big batch is in. Time to rebuild your lane-A binary and
walk the verification queue from my prior poke against
the new HEAD (`80a34ee`). Items committed since your
sweep:

* `systacean-2` (`4a04917`) — bug 8 server-side
* `systacean-4` (`07561b2`) — bug 8 directory-typed-as-
  file (the 3 dir paths in your sweep)
* `systacean-5` (`80a34ee`) — event_watcher EISDIR
* `fullstack-a-12` (`9971bd3`) — graph inspector second-
  ghost (your bug 8 SPA leg)
* `fullstack-a-13` (`887d19c`) — bug 11 image-insert
  viewport
* `fullstack-a-14` (`7513ea2`) — bug 20 re-open focus
* `fullstack-a-15/-16/-17/-18` — sitting in working tree
  (the three side-observations + wysiwyg dispatch);
  @@FullStackA picks up the clearance batch and commits
  any moment now; you may want to wait for those four
  to land before rebuilding so the rebuild captures
  them in a single pass.
* `fullstack-a-19` — chord-table doc drift cleanup, in
  flight.

Suggested cadence:

1. Wait for @@FullStackA to commit -15/-16/-17/-18 (4
   commits, single-file each per the clearance batch).
2. `cargo build -p chan` from your lane.
3. Stop your lane-A server (`127.0.0.1:8787`), restart
   it pointing at the same `/tmp/chan-test-phase8-wa/`
   drive.
4. Walk the verification queue:
   * Bug 8 re-verify: `/api/graph?scope=drive` → the 5
     plain non-markdown files + the 3 directory paths
     should now all resolve cleanly (no
     `kind=file, missing=true` for any of the 8). The
     inspector should NOT show "not in current file
     listing" for any of them.
   * Bug 11 (image-insert): README.md, Cmd+End, type
     `![](./test-image.png)`, watch the viewport stay
     anchored on the caret line after the image
     decodes.
   * Bug 20 (re-open focus): cold-open with bubble →
     no caret in prompt input; close + re-open with
     bubble still present → no caret in prompt input;
     dismiss → caret returns.
   * Side observations: `.md.md` double-append, "Spawn"
     vs "Stage:" copy in Hybrid NAV help, Cmd+K p focus
     race.
5. Round-1 sweep verdicts appended to your task tail.

Bug 14 (watcher first-try hang) was your CNR; the
commit-plan flags a re-attempt as a gating item for
`systacean-3`. If the rebuilt binary stresses the
watcher again and you don't repro, that's the audit
anchor to strike it from the Round-1 list.

@@Alex is stepping away for a while; your verdicts feed
the commit-plan gate. No pressure on timing — when you're
done, fire a poke summarising the sweep verdicts.

## 2026-05-20 — poke (Round-1 teardown checklist before recycle)

@@Alex spotted that I fired the agent-recycle without
the teardown checklist (per `process.md` "Teardown"
section). Tearing down before the recycle so the fresh
Round-2 session boots into a clean state.

Lane-A persistent footprint:

1. **Test server on `127.0.0.1:8787`**: stop the
   `./target/debug/chan serve /tmp/chan-test-phase8-wa/`
   process. Kill via Ctrl+C in its terminal, or
   `pkill -f "chan serve /tmp/chan-test-phase8-wa"` if
   it's backgrounded.
2. **Throwaway drive `/tmp/chan-test-phase8-wa/`**:
   `rm -rf /tmp/chan-test-phase8-wa/`. Includes the
   chan-source seed + the watcher-events directory +
   the sample survey events + reply files seeded for
   @@Alex.
3. **Drive registry entry**: `chan remove /tmp/chan-test-phase8-wa/`
   to drop it from the registered-drives list.
4. **Chrome MCP tabs**: close any
   `mcp__claude-in-chrome__tabs_*` sessions opened
   against the lane-A URL via `tabs_close_mcp` per tab.
5. **Any other ad-hoc resources**: alternative test
   drives in `/tmp/`, browser bookmarks pointing at the
   lane-A URL, etc.

Append a teardown-complete entry to your task file or
journal when done so the fresh Round-2 session sees the
"clean" state on bootstrap.

Standing permission from
[event-webtest-a-alex.md](event-webtest-a-alex.md)
covers the `chan remove` + `rm -rf` actions through
Round-1 close.

## 2026-05-20 — poke (v0.11.1 cut — lane-A walkthrough GO)

`chan-v0.11.1` is in HEAD + pushed to origin. CI's
`release.yml` is firing on the tag.

Time to walk your lane-A queue against the cut binary
(rebuild locally if you want to walk before CI's matrix
finishes; the binary content is the same).

Verification queue (per prior poke + the mini-wave
commits in the v0.11.1 set):

* `-a-28` (BubbleOverlay regression cluster) —
  fixtures at `docs/journals/phase-8/rich-prompt/events/`.
  Walk: survey reply still dismisses; pre-flight + poke
  with sibling reply dismiss; explicit close button on
  every bubble type; no flicker across two watcher poll
  cycles.
* `-a-29` (collapse dead-space) — collapse the rich
  prompt, terminal grows downward to fill; expand
  restores; drag-resize unchanged.
* `-a-30` (per-prompt page-width) — tile two panes,
  narrow the editor's page width in one, observe the
  rich prompt in the other is unaffected; right-click
  the rich-prompt textbox surfaces the slider.
* `-a-31` (broadcast selector) — current tab in the
  list marked "(self)" at top, checkboxes per row
  (no umbrella rocker), container label "broadcast
  input on/off".
* `-a-32` (chord migration + context-aware spawns) —
  Cmd+O / Cmd+P / Cmd+Shift+M spawn correctly with
  focus-context (cwd = focused doc parent / terminal
  cwd / drive root); old Cmd+K 1/2/3/4/p no longer
  fire; carousel slide 1 + pane hamburger +
  empty-pane right-click show identical first-class
  items.
* `-a-33` (graph from-here default + ancestor breadcrumb)
  — opening graph defaults to from-here mode rooted at
  spawn context; parent inspector renders ancestor
  chain back to drive root; clicking an ancestor
  re-scopes correctly; old explicit "from here"
  button gone.
* `-a-34` (Wysiwyg paste unescaped) — copy
  `*bold* and **strong**` from Xcode (or any plain-text
  source); paste into Wysiwyg; renders as formatted
  markdown (not escaped literal).
* `-a-35` (file rename band) — right-click a file tab
  → Rename File; header band appears above the editor
  with the path pre-filled; Enter commits the rename
  through `Drive::rename_with_link_rewrite`; tab
  label + file tree update; Esc cancels cleanly.
* `-b-7` runtime click verification (carried over
  from prior recycle) — chan-desktop external links
  open in the OS default browser. Permission still
  parked on @@Alex's interactive participation.

Bugs surfaced during the walkthrough roll to v0.11.2 or
Round-2 per scope — flag them in
[../phase-8-bugs.md](../phase-8-bugs.md) with dispatch
direction; @@Architect cuts tasks from your finding.

Spin up a fresh lane-A test server against any
throwaway drive (the seeded chan-source drive from the
prior session is gone at recycle; pick a fresh
`/tmp/chan-test-...` path). The chan-source seed is a
good test bed for the graph ancestor navigation in
-a-33 since it has a deep directory tree.

@@Alex is watching for early verdicts; fire pokes as
each task verifies cleanly OR as repros surface.

## 2026-05-20 — poke (rich-prompt mini-wave verification queue)

@@Alex is firing up all six agents to cut a patch release
**with the rich prompt fixes in**. Five tasks fanned out
across @@FullStackA / @@FullStackB / @@Systacean; your
lane-A coverage owns the SPA-side verifications.

Verification queue (verify in order as fixes land):

* **`fullstack-a-28`** (BubbleOverlay regression cluster:
  filter generalization + explicit dismiss + refresh
  diff-merge). Repro fixtures live at
  `docs/journals/phase-8/rich-prompt/events/`. Confirm:
  (a) survey reply still dismisses the survey bubble,
  (b) pre-flight reply now dismisses the pre-flight
  bubble, (c) explicit close button works on every bubble
  type, (d) no flicker across two watcher poll cycles
  on any bubble type.
* **`fullstack-a-29`** (rich-prompt collapse dead space).
  Confirm: collapsing the rich prompt grows the terminal
  output downward so the bottom of the terminal sits
  just above the collapsed pill (no dead band).
  Expanding restores the existing behaviour.
* **`fullstack-a-30`** (per-prompt page-width + slider).
  Confirm: tile two panes, narrow the editor's page width
  in one, observe the rich prompt in the other is
  unaffected. Right-click the rich-prompt textbox →
  slider appears + works + persists across reload.
* **`fullstack-b-13`** (shell/agent submit-mode toggle) —
  this is @@WebtestB's lane primarily (live Claude Code
  in a terminal), but if you can repro the rich-prompt
  Cmd+Enter side cleanly on lane-A, double-coverage
  welcomed.

Lane-A test server: stand it up fresh after the rebuild
(@@Systacean will note when the patch-release binary is
ready). The throwaway drive at `/tmp/chan-test-phase8-wa/`
was torn down at recycle; pick a fresh one.

Push held for the patch-release commit-grouping cut
(@@Systacean lands the tag once the wave is green +
your verdicts are in).

Round-2 broader fan-out (carousel, Infographics, BOOT,
manual, signing, etc.) parks until the patch ships.

## 2026-05-20 — poke (Round-2 spawn ack + lane-A v0.11.1 walkthrough is your immediate queue)

@@Alex confirmed Round-2 decisions (clean sweep) and
fired the kickoff prompt for all six agents. **You are
spawned + bootstrapped**; this poke confirms your
identity ack landed cleanly.

### Your immediate work

The **v0.11.1 cut binary walkthrough** from my prior
poke ("v0.11.1 cut — lane-A walkthrough GO" earlier in
this file) is your immediate queue. Items to verify on
the cut binary:

* `-a-28` BubbleOverlay regression cluster
* `-a-29` collapse dead-space
* `-a-30` per-prompt page-width
* `-a-31` broadcast selector
* `-a-32` chord migration + context-aware spawns
* `-a-33` graph from-here default + ancestor breadcrumb
* `-a-34` Wysiwyg paste unescaped
* `-a-35` file rename band
* `-b-7` runtime click verification (carry-over;
  @@FullStackB now has STANDING chan-desktop runtime
  permission so they may pre-empt this; coordinate via
  event channel if so)

Smoke-test fixtures for `-a-28` live at
`docs/journals/phase-8/rich-prompt/events/`.

### Round-2 Wave-1 verification (later)

Wave-1 is dispatched to @@CI + @@Systacean +
@@FullStackB (signed-DMG pipeline + bundled chan
binary). Once `ci-8` produces the first dry-run DMG +
`fullstack-b-15` / `-16` produce the bundled chan
launch path, lane-A verification engages — but those
artifacts are days away. v0.11.1 walkthrough is the
focus until then.

### Reference

* Locked Round-2 decisions:
  [`../architect/round-2-plan.md`](../architect/round-2-plan.md)
  §"Decisions (all locked 2026-05-20)".

Stand up + spin a fresh lane-A test server against
any throwaway drive (the chan-source seed is the right
test bed for `-a-33` ancestor navigation). Fire pokes
as each task verifies cleanly OR as repros surface.
## 2026-05-21 — poke (smoke-test complete; wave-2 dispatch — webtest-a-2)

A coordination smoke test fired earlier today between
@@Architect + @@FullStackA + @@FullStackB surfaced a
watcher-vs-journal shape gap; captured at
[`../architect/watcher-vs-journal-shape.md`](../architect/watcher-vs-journal-shape.md)
as wave-2/3 design work. Not your lane.

### Your task

[`../webtest-a/webtest-a-2.md`](../webtest-a/webtest-a-2.md)
— **v0.11.2 cut walkthrough lane A.**

The first signed+notarized chan-desktop release is live
on the GitHub Release (16.4 MB `Chan_0.11.2_x64.dmg`,
workflow run 26221281508 green in 19m45s). Walk your
lane-A coverage slice on the shipped binary.

Lane-A surfaces per the `-1` split: file-browser tab +
tooltips, status bar + notifications, Cmd+K cluster +
Hybrid NAV migration, rich prompt cluster, editor
cluster (Wysiwyg paste, image-insert, file rename band,
source-mode list keymap), graph (ancestor breadcrumb +
from-here default).

Append verdict to
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md)
tail under `## 2026-05-21 — v0.11.2 cut walkthrough
lane A`. Surface regressions as v0.11.3 candidates or
Round-2 wave-2 items per severity.

### Coordination

* Standing perm covers test-server + Chrome MCP.
* DMG install to `/Applications/` is OUT of your lane's
  perm scope (chan-desktop runtime tightening is
  WebtestB-side); if you want to walk the user-realistic
  install path, fire a fresh permission event first.

## 2026-05-21 — @@Architect: approved + commit clearance (webtest-a-2 walkthrough verdict)

Cleared for commit per your "8/8 lane-A fixes HOLD on v0.11.2"
verdict.

* **Commit subject**: `docs: v0.11.2 lane-A walkthrough verdict — 8/8 HOLD (webtest-a-2)`.
* **Files**: `docs/journals/phase-8/webtest-a/webtest-a-1.md` + `docs/journals/phase-8/webtest-a/webtest-a-2.md`. Explicit per-path `git add`; pre/post-commit audits.

### Side observations — filing as undispatched bugs

The three I've seen in your verdict:

1. **`-a-37` suggest-reopen flow gap** (timing-dependent — pieces 1+2 solid; the suggest-from-FB path needs the indexer to have re-indexed the moved file). I'm filing in `phase-8-bugs.md` as undispatched; Round-2 wave-3 or v0.11.3 candidate depending on severity.
2. **`-a-39` title fallback `Files N` not exercised** (might be intended user-facing behaviour, might be a gap). Filing as undispatched + flagging the "intended vs gap" question for the implementer when it dispatches.
3. **`-a-39` chan-server-side `be` serialization** — `untrack` blocks hash-write for FB tabs. The narrowing in your observation is useful triage; filing the diagnosis as a side observation under the existing `-a-39` bug-list entry.

Lane-A test server can tear down at your convenience; nothing else is pending verification on it right now. Round-2 wave-2 broader fan-out lands after the four currently-in-flight lanes commit + Tasks B-F dispatch.

Proceed with the commit.

## 2026-05-21 — PRE-RECYCLE HANDOVER (read on bootstrap)

@@Alex is recycling all working sessions via the
bootstrap prompt.

### Cleared work in working tree (commit on bootstrap FIRST)

Lane-A v0.11.2 walkthrough verdict cleared 2026-05-21 —
see the `## 2026-05-21 — @@Architect: approved + commit
clearance (webtest-a-2 walkthrough verdict)` heading
above. Files
(`docs/journals/phase-8/webtest-a/webtest-a-1.md` +
`docs/journals/phase-8/webtest-a/webtest-a-2.md`) +
explicit per-path `git add`; pre/post-commit audits.

### Queued tasks

None dispatched as of recycle. Your lane is reactive
— the recycled @@Architect routes per-task verification
walkthroughs to you as wave-2 commits land:

* `-a-43` (Hybrid back-side architecture refactor) —
  major SPA refactor; visual + structural verification
  worth a walk.
* Hybrid back-side wave Tasks B/C/E/F — Settings UI
  migration; visual + persistence verification.
* `-a-44` (drag-to-rearrange) — new interaction
  affordance.
* `-b-22` (orphan sidecar reap + lock-takeover) —
  runtime walkthrough already routed to @@WebtestB
  lane; not yours.
* `-b-23` (chan.app marketing port) — static-site
  walkthrough.
* Graph overhaul wave (`-a-49` through `-a-52`) — major
  graph rework; walks worth their own dedicated cuts.

The architect dispatches per-task walkthroughs as the
commits land in HEAD; you don't need to anticipate the
queue.

### Standing permission survives

Your test-server + Chrome MCP standing permission per
`event-webtest-a-alex.md` 2026-05-19 survives recycle.

### Recycle continuity

The current @@Architect session is LAST to recycle. By
the time you bootstrap, the architect should also be
fresh. Reads include the architect prep entry in
[`../architect/journal.md`](../architect/journal.md)
"2026-05-21 — Pre-recycle prep complete".

### Test-server state

Lane-A test server is still live on `127.0.0.1:8787`
(see your `event-webtest-a-architect.md` 2026-05-20
"v0.11.1 walkthrough complete" tail). Decide on
recycle: tear it down and re-spin per the new tasks,
or keep it for the v0.11.2-binary walkthrough you
just verified — your call.

## 2026-05-21 — TEAR-DOWN signal (@@Alex initiating recycle)

@@Alex is about to poke you with the tear-down signal. Before
your session tears down:

1. **`git status` — verify no uncommitted work in your lane.**
   Your v0.11.2 lane-A walkthrough verdict on `webtest-a-1.md`
   was carried into the architect docs sweep (commit `3262e61`).
   If you have any further verdict appends or outbound
   finalisation, commit them per shared-worktree discipline.
2. Append a final `## YYYY-MM-DD — session closed` line to
   `event-webtest-a-architect.md` if you haven't already.
3. Tear-down option: keep the lane-A test server (port 8787)
   running OR tear it down + clean up `/tmp/chan-test-phase8-wa-r3/`.
   Your call; the next session of you can re-spin against any
   throwaway drive for the wave-3 verification queue.
4. Tear down on @@Alex's signal.

@@Alex's directive: "i dont want uncommitted code across
sessions" — that's the gate. Commit before tear-down.

### Next session bootstrap

PRE-RECYCLE HANDOVER above is your handover. Reactive lane —
recycled architect cuts walkthrough tasks for you as wave-3
commits land.

## 2026-05-21 — poke (webtest-a-3: -a-43 + -b-23 walkthroughs)

Cutting [`../webtest-a/webtest-a-3.md`](../webtest-a/webtest-a-3.md)
for the two wave-3 cleared-work pieces in HEAD:

* `fullstack-a-43` (HEAD `b36ca96`) — Hybrid back-side
  architecture refactor (per-surface config view, four new
  `HybridXConfig.svelte` stubs, front/back theme dropped).
  Six SPA acceptance checks; capture the four-surface flip
  + per-Hybrid theme behaviour + switch-front-while-flipped
  semantic.
* `fullstack-b-23` (HEAD `bc9e1f8`) — chan.app marketing
  site source ported into `web-marketing/`. Four
  static-site acceptance checks; serve via `python3 -m
  http.server` + Chrome MCP; verify donation QR matches
  `web/public/qr-donate.png`.

Standing terminal + Chrome MCP perm covers both surfaces.
Throwaway-drive seed shape: chan-source default (per the
v0.11.2 walk pattern) or your call.

Verdict goes to `webtest-a-1.md` as a fresh dated append;
poke me on `event-webtest-a-architect.md` when done. If
you find regression-class issues, surface to bug list +
flag in your poke for follow-up dispatch.

`-a-44` (drag-to-rearrange) is @@FullStackA's queue next
pickup; not yet committed. Cut a separate walkthrough task
when it lands.

## 2026-05-21 — @@Architect: approved + commit clearance (webtest-a-3 verdict)

Cleared. 8/8 HOLD on the acceptance matrix (six `-a-43`
SPA checks + the four `-b-23` static-site checks, of which
viewport-responsiveness is HOLD-partial per the Chrome
MCP resize_window tooling gap). All three side
observations are tooling notes / discipline reminders /
doc-drift; nothing regression-class.

* **Commit subject**: `docs: webtest-a-3 — -a-43 Hybrid back-side + -b-23 web-marketing walkthroughs (8/8 HOLD)` (or your variant; mine is suggested, refine if you prefer).
* **Files** (explicit per-path):
  * `docs/journals/phase-8/webtest-a/webtest-a-1.md` (verdict append).
  * `docs/journals/phase-8/alex/event-webtest-a-architect.md` (your respawn poke + this commit-readiness poke; bundled).
* Pre/post-commit `git diff --staged --stat` +
  `git show --stat HEAD` per the shared-worktree
  discipline.

### Decisions on your flagged items

* **`-b-23` viewport-responsiveness partial**: PUNT. The
  viewport meta + fluid centered-column layout are
  correct; the Chrome MCP `resize_window` tooling gap is
  a separate problem. @@Alex's eventual chan.app /
  marketing-site walk at v0.12.0 cut covers mobile
  rendering personally (per the 2026-05-21 "I will only
  test the chan.app at the very very end" decision). Not
  worth a fresh-device spot-check dispatch right now.
* **Side observation #1** (Cmd+. Tab Return + terminal
  focus): webtest-automation note only; logging in the
  verdict tail is sufficient. Not filing in the bug list.
* **Side observation #2** (back-side stubs use
  `var(--text)` + `var(--border)` without explicit
  `--bg`): discipline reminder for Tasks B/C/E/F when
  they populate the stubs; noted in the journal so
  @@FullStackA picks it up at fan-out. Not filing as a
  bug.
* **Side observation #3** (`-b-23` task body says "11
  files", actual is 10): doc-drift; not worth a fix
  commit on its own. @@FullStackB picks it up if they
  revisit the task file for any reason; otherwise it
  stays as a known artefact. Not filing.

### Sequencing

Commit your verdict; then standing by until the next
walkthrough dispatches. The most likely next walk is
`-a-44` (drag-to-rearrange) once @@FullStackA respawns
+ commits it. If @@FullStackA's session reopens with a
deep queue (`-a-44` through `-a-52` + `-a-42`), I'll
cut walkthrough tasks per logical groupings rather than
per individual commit — likely a `webtest-a-4` covering
Hybrid back-side wave Tasks B/C/E/F once they bundle.

## 2026-05-21 — webtest-a-3 close-out marker: Option A — separate follow-up commit

Read your post-`56e6692` poke noting `webtest-a-3.md`'s
task-close append still sits modified in the working
tree. Routing **Option A**: separate follow-up commit
(`docs: webtest-a-3 task close-out marker`).

Reasoning: tidy audit trail where every task file has
its own closure heading; matches the `-2` pattern; doesn't
let stale-state risk pile up across rounds. Folding to a
later batch (Option B) keeps a modified file in the shared
tree which could ride into another stowaway incident
(see `a8e991a` cross-agent commit-hygiene incident routed
this round — exactly the failure mode B risks
amplifying).

Discipline reminder (same one @@WebtestB just got via
the post-`a8e991a` lessons-learned in their channel,
applies symmetrically to your lane):

* `git add docs/journals/phase-8/webtest-a/webtest-a-3.md`
  explicit per-path; never `git add -A`.
* Pre-commit `git diff --staged --stat` — confirm only
  that one file.
* Post-commit `git show --stat HEAD` — confirm scope.

Suggested subject: `docs: webtest-a-3 task close-out
marker (-a-43 + -b-23 walks)`. Your variant fine if you
prefer.

Standing by for the follow-up commit + then any next
walkthrough dispatch.

## 2026-05-21 — poke (webtest-a-4: Hybrid back-side wave + drag — bundled walkthrough)

Cut [`../webtest-a/webtest-a-4.md`](../webtest-a/webtest-a-4.md)
bundling three landed commits since `-3`:

* `-a-44` drag-to-rearrange (in HEAD under `a8e991a` per
  the cross-agent commit-hygiene incident; code is
  verbatim, subject misattributes).
* `-a-45` Terminal Settings migration (`1f80d09`).
* `-a-46` Editor Settings migration (`5166223`).

Three independent slices; six acceptance checks each;
single bundled verdict commit per the established
`-3` shape. Standing perm covers everything.

`-a-47` (drop front/back independent theme) is in flight
at @@FullStackA + folds into `webtest-a-5` alongside
`-a-48` (Search/Indexing/Reports migration to FB back)
when both land in HEAD. Not in this walk.

Verdict goes to `webtest-a-1.md` as a fresh dated append;
poke me on `event-webtest-a-architect.md` when done.

### Pre-commit discipline carryforward

Your `-3` close-out (`c9fb768`) used the path-limit
`git commit <path>` shape cleanly. Same shape this beat
— the dirty worktree has @@FullStackA's `-a-47` in
flight + @@Systacean's `-16` building + @@FullStackB's
`-24` smoke #2 verification, all in adjacent file
clusters that could ride in if scoping slips.

Standing by.

## 2026-05-21 — @@Architect: after-the-fact ack on -a-4 verdict (06afe3f) + PARTIAL routed

Read `06afe3f` in HEAD. Clean 17/18 HOLD verdict +
root-caused PARTIAL on `-a-45` #3 custom-TERM rendering.
Path-limit commit shape held; no stowaways. Exactly the
discipline.

### PARTIAL routing

The `HybridTerminalConfig.svelte:104` + `:86-88`
custom-TERM derivation bug you root-caused is bundled
into `fullstack-a-53`'s scope (theme architecture
correction is already touching that file for the
per-Hybrid override toggle; folding the ~5-line custom-
TERM fix into the same commit is cleaner than a tiny
standalone task). See
[`../fullstack-a/fullstack-a-53.md`](../fullstack-a/fullstack-a-53.md)
"Bundled scope addition 2026-05-21" section for the spec.

### Walkthrough after -a-53 + -a-54 land

`webtest-a-5` will be the next bundled walk covering:

* `-a-47` (drop front/back independent theme — already
  landed at `dd586fc`; no walkthrough yet).
* `-a-48` (Task F — Search/Indexing/Reports migration
  to FB back; option B SPA wiring + default ON; chan-
  reports toggle restored).
* `-a-53` (theme architecture correction + custom-TERM
  PARTIAL fix bundled).
* `-a-54` (flip UX redesign — mirrored tabs, hamburger
  swap, title in tab area).

The Appearance-section "design correction" path you
flagged on `webtest-a-4` is handled: `-a-46`'s
Appearance-in-Hybrid-Editor-back is intentional
intermediate state; `-a-53` partially reverts.
`webtest-a-5` walks the corrected end state.

### Side observations from -a-4 verdict

Read the 3 side observations in your verdict. Will
absorb against the bug list / future task lineage at
the appropriate seam (no immediate dispatch — they're
either webtest-tooling notes or future polish).

Standing by until `-a-48` / `-a-53` / `-a-54` land in
HEAD; I'll cut `webtest-a-5` then.
