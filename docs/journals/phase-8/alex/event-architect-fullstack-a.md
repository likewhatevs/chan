# event-architect-fullstack-a.md

From: @@Architect
To: @@FullStackA
Date: 2026-05-19

## 2026-05-19 — poke

`fullstack-a-1` approved + cleared to commit. Push waits for
Round-1 close — do NOT push yet. Pick up `fullstack-a-2`
(status-bar click events + flash colour) next.

Also: a new task landed in your queue — `fullstack-a-7`
(switch Hybrid NAV binding from Cmd+K to Cmd+.; introduce
Cmd+, for Settings). Treat it as a queue item after the
existing -2 through -6; coordinate the status-bar label change
with `fullstack-a-3` so the wording only lands once.

See [../fullstack-a/fullstack-a-1.md](../fullstack-a/fullstack-a-1.md)
tail for the review reply.

## 2026-05-19 — poke

`fullstack-a-2` approved + cleared to commit. Push waits for
Round-1 close. Pick up `fullstack-a-3` (Cmd+K cluster) next.

New tasks landed in your queue while you were on -2:
* `fullstack-a-7` (Cmd+K → Cmd+. swap; introduce Cmd+, for
  Settings).
* `fullstack-a-8` (restore CSS wobble on Hybrid + right-click
  menus — regression from phase-7 `fullstack-80` / `-82`).

Treat them as queue items after the existing -3 through -6.
Coordinate `-3`'s status-bar label with `-7`'s Cmd+. wording so
the copy only lands once.

See [../fullstack-a/fullstack-a-2.md](../fullstack-a/fullstack-a-2.md)
tail for the review reply.

## 2026-05-19 — poke

`fullstack-a-3` approved + cleared. Three-part edit landed
cleanly. Pick up `fullstack-a-4` next (rich prompt cluster).

Coordination note for `fullstack-a-7` (Cmd+. swap): the Hybrid
pill copy lands now via `-3` with Cmd+K wording; `-7` updates
the same single line to Cmd+.. Single-line edit, no copy
duplication.

See [../fullstack-a/fullstack-a-3.md](../fullstack-a/fullstack-a-3.md)
tail for the review reply.

## 2026-05-19 — poke (batch clearance: -4 / -5 / -6 / -7 / -8)

Five tasks approved + cleared in one batch. Per-task reviews
appended at the tails of:
* [../fullstack-a/fullstack-a-4.md](../fullstack-a/fullstack-a-4.md) (rich prompt)
* [../fullstack-a/fullstack-a-5.md](../fullstack-a/fullstack-a-5.md) (editor cluster)
* [../fullstack-a/fullstack-a-6.md](../fullstack-a/fullstack-a-6.md) (Cmd+K F focus)
* [../fullstack-a/fullstack-a-7.md](../fullstack-a/fullstack-a-7.md) (Cmd+. swap)
* [../fullstack-a/fullstack-a-8.md](../fullstack-a/fullstack-a-8.md) (CSS wobble)

Each is a standalone commit; suggested subjects in each tail.
Push waits for Round-1 close.

New tasks in your queue:
* `fullstack-a-9` (`[` / `]` resize inversion in Hybrid NAV).
* `fullstack-a-10` (Chrome-style tab-name fade + full-path
  hover on file tabs + FB tree rows).

Pick `-9` next unless `-10` looks lighter. Both have clear
specs.

## 2026-05-20 — poke (batch clearance: -9 / -10 / -11 + wave-2 queue: -12 / -13 / -14)

Three approvals and three new tasks in one go.

**Cleared to commit (push waits for Round-1 close):**

* [../fullstack-a/fullstack-a-9.md](../fullstack-a/fullstack-a-9.md)
  ([ / ] / - / = fixed-direction resize)
* [../fullstack-a/fullstack-a-10.md](../fullstack-a/fullstack-a-10.md)
  (tab strip + FB tree fade + full-path hover)
* [../fullstack-a/fullstack-a-11.md](../fullstack-a/fullstack-a-11.md)
  (regression pin: last back-tab keeps showingBack=true)

Per-task reviews at each tail with suggested commit subjects.
All three are standalone commits.

**Wave-2 queue (from @@WebtestA's Round-1 sweep + side
observations):**

* [../fullstack-a/fullstack-a-12.md](../fullstack-a/fullstack-a-12.md) —
  Graph inspector second-ghost on lazy-tree path (SPA follow-
  up to `systacean-2`).
* [../fullstack-a/fullstack-a-13.md](../fullstack-a/fullstack-a-13.md) —
  Editor image-insert viewport snap + no-roll on subsequent
  typing. **Highest priority** — Alex-visible repro on lane A.
* [../fullstack-a/fullstack-a-14.md](../fullstack-a/fullstack-a-14.md) —
  Rich prompt re-open with bubble present focuses prompt
  input (partial on `-a-4`).

Suggested order: `-13` first (worst user-visible), then `-12`
(needs `systacean-2` committed for verification, so leave it
mid-queue), then `-14`. All three can use @@WebtestA's lane-A
test server (`127.0.0.1:8787`, URL with token in
`event-architect-alex.md` 2026-05-20).

Side observations from the sweep that I'm filing as bug
entries but NOT cutting tasks for in this wave (queue depth
management): `.md.md` double extension on New file, "Stage:"
copy in Hybrid help, Cmd+K p focus race. Cmd+Enter
first-char swallow IS cut as a task — to @@FullStackB
(`fullstack-b-8`) since it's terminal-side. The first three
will get tasks if -12/-13/-14 land fast and you have queue
room; otherwise they roll to Round 2.

Pre-push gate as always before each commit.

## 2026-05-20 — poke (wave-3 small queue: -15 / -16 / -17)

Three small task cuts to keep your queue deep — pick up
after -12/-13/-14 land:

* [../fullstack-a/fullstack-a-15.md](../fullstack-a/fullstack-a-15.md) —
  "New file" dialog double-appends `.md`. Small UX fix.
* [../fullstack-a/fullstack-a-16.md](../fullstack-a/fullstack-a-16.md) —
  Hybrid NAV help overlay copy says "Stage:" but runtime
  is immediate-commit. Pure copy update.
* [../fullstack-a/fullstack-a-17.md](../fullstack-a/fullstack-a-17.md) —
  Cmd+K p (spawn terminal) steals rich-prompt focus.
  Family of the `fullstack-a-4` focus rules.

These are all from the side-observation backlog in
@@WebtestA's Round-1 sweep — Alex asked me to crack on the
bug list while they're away, so I'm queuing depth rather
than gating on commit landings.

Order: whatever's fastest first; -16 is essentially a copy
edit (5 minutes), -15 is small, -17 needs investigation.

@@Alex is stepping away — will return to cut the v0.11.1
build and to handle any permission asks that require their
interactive input. Do NOT push commits without their
return; commit-and-park as today.

## 2026-05-20 — poke (fullstack-a-13 cleared)

`fullstack-a-13` approved + cleared to commit. Strong
root-cause: CM6 caret-tracking is transaction-scoped so any
async layout shift leaves scrollTop stale; image decode is
the worst-felt instance. Fix lands in the right layer
(image widget load handler), with three sane guards
(success-load only, `wrap.isConnected`, line-proximity).
Per-task review at the tail of
[../fullstack-a/fullstack-a-13.md](../fullstack-a/fullstack-a-13.md);
use the suggested commit subject. Push waits for Round-1
close.

Carry on with `fullstack-a-12` next. `systacean-2` is in
HEAD (`4a04917`) so the binary rebuild has the resolver
universe fix; your -12 verification on the lane-A server
will need a server restart to pick up the rebuild.

## 2026-05-20 — poke (fullstack-a-12 cleared)

`fullstack-a-12` approved + cleared to commit. The
two-branch `isFileGhost` was always brittle once the server
became authoritative; collapsing to `selectedNode.missing
=== true` is the right tightening. Regression audit of the
other ghost paths confirms no false negatives. Per-task
review at the tail of
[../fullstack-a/fullstack-a-12.md](../fullstack-a/fullstack-a-12.md);
use the suggested commit subject. Push waits for Round-1
close.

Carry on with `fullstack-a-14` (rich prompt re-open focus)
next. Then -15 / -16 / -17.

## 2026-05-20 — poke (fullstack-a-14 cleared + new task -18)

`fullstack-a-14` approved + cleared to commit. Outstanding
Svelte-5 lifecycle audit — child `onMount` fires
synchronously before parent `$effect`, so the
`bubbleCount > 0` gate in the parent runs too late. The
autoFocus prop at the child level keeps focus state
correct from first paint; effect-level blur would have
flickered. Per-task review at the tail of
[../fullstack-a/fullstack-a-14.md](../fullstack-a/fullstack-a-14.md);
use the suggested commit subject. Push waits for Round-1
close.

**New task: -18.** @@FullStackB caught a separate wysiwyg-
mode dispatch bug during their `fullstack-b-8` work:
`TerminalRichPrompt` doesn't thread `onSubmit` into the
`<Wysiwyg>` child, so Cmd+Enter is silently consumed by
the Wysiwyg keymap (which calls `onSubmit?.()` against
undefined). Source-mode works only because Source has no
Mod-Enter binding and the event bubbles. Cut as
[../fullstack-a/fullstack-a-18.md](../fullstack-a/fullstack-a-18.md).
Small task — coordinate with -14 if its commit hasn't
landed yet (same Wysiwyg child instantiation site).

Updated queue: `-15` (md.md double ext) → `-16` (Stage:
copy) → `-17` (Cmd+K p focus race) → `-18` (wysiwyg
dispatch). Or reorder if -18 feels small enough to slot
in front of -15/-16/-17.

## 2026-05-20 — poke (batch clearance: -15 / -16 / -17 / -18 + new task -19)

Four-way batch clearance. All approved + suggested commit
subjects in each task tail.

* [../fullstack-a/fullstack-a-15.md](../fullstack-a/fullstack-a-15.md) —
  new file dialog selection range. Right layer, four
  cases enumerated.
* [../fullstack-a/fullstack-a-16.md](../fullstack-a/fullstack-a-16.md) —
  "Stage:" → "Spawn" copy fix + test regex update.
* [../fullstack-a/fullstack-a-17.md](../fullstack-a/fullstack-a-17.md) —
  TerminalTab focus gate. Bonus catch on the pane-switch-
  return path; clean queueMicrotask boundary keeps
  reactive tracking honest.
* [../fullstack-a/fullstack-a-18.md](../fullstack-a/fullstack-a-18.md) —
  Wysiwyg `onSubmit` threading. One-line; composes with
  -14's autoFocus prop without interaction.

Each is a standalone commit. Push waits for Round-1
close.

**New task: -19.** Cut the last side-observation backlog
item — Hybrid NAV chord-table doc drift across
PaneModeHelp + SERVE_LONG_ABOUT.
[../fullstack-a/fullstack-a-19.md](../fullstack-a/fullstack-a-19.md).
Three known stale entries (Pane Mode header, `s` → `f`,
`k` → Cmd+K Backspace); audit the rest of the table for
additional drift while you're in the file. Final task in
your Round-1 queue.

After -19 lands, you're queue-empty for Round 1. The
commit-grouping plan is published at
[../architect/commit-plan-v0.11.1.md](../architect/commit-plan-v0.11.1.md);
@@Systacean's `systacean-3` (version bump + tag + push)
unblocks when @@Alex returns + the gating verifications
in that plan land.

## 2026-05-20 — poke (HOTFIX: fullstack-a-20 — your -a-18 regression)

@@Alex caught a regression from your `fullstack-a-18`
fix. The thread now produces a double-dispatch on
wysiwyg-mode Cmd+Enter: typing `pwd` arrives in the
terminal as `pwdpwd`. Diagnosed: the wrapper's `onKeydown`
at `TerminalRichPrompt.svelte:118-122` doesn't check
`e.defaultPrevented`, so when Wysiwyg's keymap calls
`submit()` + returns true (which preventDefaults the
event), the wrapper STILL calls `submit()` on the
bubbled event.

Cut as [../fullstack-a/fullstack-a-20.md](../fullstack-a/fullstack-a-20.md).
One-line fix: `if (e.defaultPrevented) return;` at the top
of `onKeydown`. Plus a test pin that asserts a
defaultPrevented Cmd+Enter doesn't call `submit()`.

**Hard gate**: must land before v0.11.1 tag fires. Slot
ahead of -19. Stack: -15/-16/-17/-18 (the 4 cleared from
my prior batch) + -20 (this hotfix on top of -18) + -19
(chord-table doc drift) → 6 commits to land before
queue-empty.

If the wave-3 commits haven't picked up yet from the
prior clearance batch: please pick up that batch + slot
-20 in front of -19 (i.e., commit order is -15, -16,
-17, -18, -20, -19).

@@Alex is back / active now; turnaround on this one is
the path to unblocking @@Systacean's `systacean-3`. The
commit-plan has been updated to mark -a-20 as a hard
gate; see
[../architect/commit-plan-v0.11.1.md](../architect/commit-plan-v0.11.1.md).

## 2026-05-20 — poke (detour fan-out: -21 + -22)

@@Alex landed a structural change while @@Architect was
drafting the wave-3 follow-ups. **Short version**: Round 1
closes WITHOUT a binary cut; the detour brings two SPA-side
tasks forward of Round 2 so they ship in the first proper
release at end of Round 2.

Two new tasks for your queue:

* [../fullstack-a/fullstack-a-21.md](../fullstack-a/fullstack-a-21.md)
  — Settings page UI for semantic-search opt-in. Toggle
  + download progress bar + storage info row. Depends on
  `systacean-7`'s API contract; you can start layout
  against mock responses, finalize wiring once -7's
  endpoints are live.
* [../fullstack-a/fullstack-a-22.md](../fullstack-a/fullstack-a-22.md)
  — animate the Hybrid pane flip with a 3D card-flip
  transition (style of `nnattawat.github.io/flip`). Pure
  CSS, ~400ms duration, `prefers-reduced-motion` honoured.
  Independent of the other detour tasks.

Updated queue (in priority order after the wave-3 commits
+ -a-20 hotfix land):

1. -20 (HOTFIX: regression from -18)
2. -19 (Hybrid NAV chord-table doc drift)
3. -21 (Settings UI for semantic-search) — wait for
   systacean-7 API contract before final wiring; layout
   can start sooner against mocks.
4. -22 (pane-flip animation) — independent, can land
   anytime.

## 2026-05-20 — poke (fullstack-a-19 cleared)

`-19` approved + cleared to commit. Comprehensive audit on
both PaneModeHelp + SERVE_LONG_ABOUT against the dispatch
source-of-truth — exactly the right shape. Six fixes in
the CLI block + two test updates land cleanly. Per-task
review at the tail of
[../fullstack-a/fullstack-a-19.md](../fullstack-a/fullstack-a-19.md);
use the suggested commit subject. Push waits until end
of Round 2 (no Round-1 binary cut per the restructure).

Wave-3 commits in your working tree (commit in any order;
each is single-file or close to it):
* -15 (md.md double-append)
* -16 (Stage: → Spawn)
* -17 (TerminalTab focus gate)
* -18 (Wysiwyg onSubmit threading)
* -19 (Hybrid NAV chord-table drift)
* -20 (defaultPrevented guard hotfix)

Then -21 / -22 from the detour queue when you're ready.
-21 waits on systacean-7 API contract; -22 is
independent. No urgency on either; queue items, not
hotfixes.

## 2026-05-20 — poke (fullstack-a-20 cleared)

`-20` approved + cleared to commit. Single-line
`defaultPrevented` guard in exactly the right place; the
post-fix Wysiwyg / Source / Escape table covers all three
paths cleanly. Bonus correctness for Escape (no current
consumer cancels Escape via preventDefault, but the guard
respects the discipline). Test pin exercises the
`defaultPrevented=true` branch; the existing line-133 test
exercises the `defaultPrevented=false` branch.

Per-task review at the tail of
[../fullstack-a/fullstack-a-20.md](../fullstack-a/fullstack-a-20.md);
use the suggested commit subject. Push waits until end of
Round 2.

**Commit ordering recommendation**: land -20 BEFORE -19 in
the wave-3 set. -20 fixes the regression -18 introduced;
ordering keeps the git-log story linear (regression
introduced + fixed before any other wave-3 commit lands).
The order I'd suggest:

1. -15 (md.md double-append)
2. -16 (Stage: → Spawn)
3. -17 (TerminalTab focus gate)
4. -18 (Wysiwyg onSubmit threading)
5. -20 (defaultPrevented guard hotfix — comes right after
   -18 to close the regression in one bisect window)
6. -19 (Hybrid NAV chord-table drift)

After the wave-3 set lands, your queue is the detour
tasks -21 (waits for systacean-7's API contract) and
-22 (independent). Plus the new Round-2 chord migration
task drafted in round-2-plan.md (slotted for post-recycle
fan-out).

## 2026-05-20 — poke (fullstack-a-21 unblocked: systacean-7's API contract is locked)

`@@Systacean`'s `systacean-7` landed at `6bf44cd`. The
API contract for `fullstack-a-21` is **locked**:

* `GET /api/index/semantic/state` (open, read-only).
* `POST /api/index/semantic/download` (settings-gated,
  **synchronous** in v1 — request blocks until download
  completes / fails).
* `POST /api/index/semantic/enable` (settings-gated;
  409 if model not present, with download_endpoint
  pointer in the body).
* `POST /api/index/semantic/disable` (settings-gated,
  always 200, idempotent).

Full SemanticState shape at the tail of
[../systacean/systacean-7.md](../systacean/systacean-7.md):

```json
{
  "mode": "bm25" | "hybrid",
  "model_present": true | false,
  "model_name": "BAAI/bge-small-en-v1.5",
  "model_path": "<global_models_dir>/models--BAAI--bge-small-en-v1.5",
  "model_size_bytes": 132456789,
  "semantic_enabled": false
}
```

`mode` is derived: `"hybrid"` iff
`semantic_enabled AND model_present`. The
flag-on-but-model-deleted shape falls back to `"bm25"`
defensively.

**UX adjustment for fullstack-a-21**: the download path
is synchronous in v1 (no per-byte progress events).
Recommended UX: when the user flips the toggle on, the
Settings UI polls `/api/index/semantic/state` every few
seconds during the download. The `model_present`
transition (false → true) is the state-change signal —
when true, flip the Settings UI to "enabled" + auto-fire
the enable endpoint. No progress bar in v1; a spinner +
"Downloading model… this may take a few minutes" string
covers it.

The original task spec asked for a progress bar; @@Systacean
flagged the hf-hub no-progress-callback constraint + the
spawn_blocking responsiveness as the reason for the v1
shape. Async-with-progress is parked for Round 3 polish.
This deviation is approved on the architect side; carry
on with polling-for-state-transition.

Queue update (priority order):

1. Wave-3 commits in working tree (-15/-16/-17/-18/-20/-19
   — recommended commit order in the prior poke).
2. -21 (Settings UI for semantic search) — **now
   unblocked**. Polling pattern + spinner UX per above.
3. -22 (pane-flip animation) — independent, can land
   anytime.

## 2026-05-20 — poke (fullstack-a-21 cleared + new task -23 (FB dock separator))

`-21` approved + cleared to commit. Polling + spinner UX
landed cleanly; the build-not-built guardrail
(`buildInfo.features.embeddings === false` → render
rebuild hint instead of broken toggle) is the right
defensive coverage. Per-task review at the tail of
[../fullstack-a/fullstack-a-21.md](../fullstack-a/fullstack-a-21.md);
use the suggested commit subject. Push waits until end
of Round 2.

**New task: `-23`** —
[../fullstack-a/fullstack-a-23.md](../fullstack-a/fullstack-a-23.md).
@@Alex flagged 2026-05-20 with a screenshot: drop the
visible idle paint on the docked-FB resize handle. The
4 px hit area + drag-resize stay; only the
`background: var(--separator)` idle paint goes. Hover
state keeps the 6 px + `--separator-hover` cue as the
discovery affordance, plus the existing `cursor:
col-resize` as the fallback cue.

Two implementation options in the task body — Option A
(per-instance opt-out via new `idleVisible` prop on
`ResizeHandle`, default true) recommended for surgical
blast radius; Option B (global flip) acceptable if the
audit shows the "invisible idle, visible hover" pattern
fits every consumer. Pick after a quick grep for other
`ResizeHandle` consumers.

**Update 2026-05-20**: @@Alex locked Option A. Task body
updated to remove the choice; just implement the
per-instance opt-out with the new `idleVisible?: boolean`
prop. No need to audit-then-pick — go straight to the
prop + the two FB dock side-pane call sites.

Updated queue:

1. Wave-3 commits in working tree (-15/-16/-17/-18/-20/-19).
2. -22 (pane-flip animation) — independent.
3. -23 (FB dock separator paint) — independent, very
   small.

-21 just landed so the queue contains the remaining two
detour tasks + this small visual cleanup. No urgency on
ordering between -22 and -23; pick whichever fits.

## 2026-05-20 — poke (two new Round-1 tasks: -24 + -25; @@Alex stepping away ~40 min)

@@Alex pulled two more items into Round 1 (was Round 2)
before stepping away briefly. Cracking on now so the work
is queued before they return.

**`-24` rich prompt + bubbles visual redesign + collapse**:
[../fullstack-a/fullstack-a-24.md](../fullstack-a/fullstack-a-24.md).
The rich prompt currently renders as a rectangle attached
to the bottom of the screen; @@Alex wants a softly-rounded
floating pill (reference image in the conversation).
Three visual deltas + a new collapse/expand affordance:

* Rounded corners + breathing-room float on the prompt
  + every chat/survey bubble.
* Default placeholder: "Write a multi-line command and
  Cmd+Enter".
* Style toolbar moves INSIDE the bubble (was outside);
  default OFF; when ON sits at the top with margin so
  the first-line cursor has clearance.
* New collapse/expand control next to close. Collapsed
  state shrinks the prompt to minimal height; chat /
  survey area gains the freed vertical space. Persists
  across close → re-open within the same session.

Composes with every prior rich-prompt fix
(`fullstack-a-4`/`-14`/`-17`/`-18`/`-20`) cleanly — task
body enumerates the compositions.

**`-25` editor trailing-whitespace toggle → Settings**:
[../fullstack-a/fullstack-a-25.md](../fullstack-a/fullstack-a-25.md).
Remove the checkbox from the editor menu; add it to the
Settings page under an Editor section (create if not
present). Preserves the current behaviour + default
value; UI-only relocation.

Updated queue priority order:

1. Wave-3 commits in working tree (commit pass — should
   be straightforward; you've already declared queue-
   clear on the detour set).
2. `-23` (FB dock separator, Option A locked) — small,
   independent.
3. `-24` (rich prompt redesign) — medium; pairs with
   the other Settings-side work in -21 / -25.
4. `-25` (editor toggle relocation) — small.
5. (After all of the above) `-22` is already committed-
   ready in working tree — make sure that committed.

@@Alex stepping away for ~40 min. No need to wait; crack
on the queue. They'll review on return.

## 2026-05-20 — poke (fullstack-a-23 cleared)

`-23` approved + cleared. Clean Option A landing: per-
instance `idleVisible?: boolean` prop on `ResizeHandle`
(default `true`); FB side-pane passes `false` to both
instances; audit of Inspector + GraphPanel consumers
confirmed they inherit the default. Hover state +
cursor untouched as the discovery affordances.

Per-task review at the tail of
[../fullstack-a/fullstack-a-23.md](../fullstack-a/fullstack-a-23.md);
use the suggested commit subject. Push waits until end
of Round 2.

Commit -23, then pick up `-24` (rich prompt redesign) +
`-25` (editor toggle → Settings) — both cut while you
were detour-clearing, both in your queue per the
2026-05-20 dispatch poke above. -22 is already
committed at `6ed7ebb`.

Round-1 detour set fully cleared on the architect side;
remaining work is the -23 commit + the -24 / -25 commit
sequence.

## 2026-05-20 — poke (fullstack-a-24 cleared + new task -26)

`-24` approved + cleared. Five-area landing in one
cohesive commit: floating-pill visual + default-off
style toolbar + collapse/expand chevron + placeholder
overlay (clean editor/prompt boundary) + bubble corner
12 vs 14 asymmetry as a design language. Composition
with every prior rich-prompt fix (`-a-4`/`-14`/`-17`/
`-18`/`-20`) verified + reasoned through. Per-task
review at the tail of
[../fullstack-a/fullstack-a-24.md](../fullstack-a/fullstack-a-24.md);
use the suggested commit subject. Push waits until end
of Round 2.

**New task: `-26`** — hybrid markdown-editor style
toolbar parity with rich-prompt.
[../fullstack-a/fullstack-a-26.md](../fullstack-a/fullstack-a-26.md).
@@Alex flagged: the rich prompt's toolbar has a
separator + rendered/source mode toggle (you just
touched the mode-toggle tests in -a-24); the hybrid
markdown editor's toolbar is missing that pair. Add to
the hybrid editor toolbar so they match.

The Wysiwyg ↔ Source mode swap already works at the
component level (both Hybrid editor + rich prompt
mount the same `Wysiwyg.svelte` + `Source.svelte`); the
toggle just exposes it in the hybrid toolbar surface.
Optional extract-to-shared-component if the separator
+ toggle pair is worth deduping.

Queue order (priority):

1. Commit -22 / -23 / -24 (whichever still uncommitted).
2. -25 (editor trailing-whitespace toggle → Settings).
3. -26 (markdown-editor toolbar parity).

All three are small; pick whichever ordering fits.

## 2026-05-20 — poke (fullstack-a-25 cleared + new task -27 (Hybrid hamburger polish))

`-25` approved + cleared. The "storage was already in the
right place" finding made this a clean UI-only relocation;
the `$effect`-driven sync to keep `editorToolsPrefs`
in-memory in lockstep with the autosaved `editing.*` field
sidesteps the persist-helper-vs-autosave race cleanly.
Per-task review at the tail of
[../fullstack-a/fullstack-a-25.md](../fullstack-a/fullstack-a-25.md);
use the suggested commit subject. Push waits until end of
Round 2.

**New task: `-27`** — Hybrid pane hamburger polish.
[../fullstack-a/fullstack-a-27.md](../fullstack-a/fullstack-a-27.md).
@@Alex 2026-05-20:

> One more polish in the hybrid's hamburger: move the
> dark/light mode in there, and add flip button as well.

Two small entries in the Hybrid pane's hamburger menu:
1. **Dark/light mode toggle** — flips the per-Hybrid
   `data-theme` via the `fullstack-b-5` override. Today
   the toggle lives elsewhere; @@Alex wants it
   accessible directly from the hamburger.
2. **Flip button** — triggers `flipHybrid()` /
   `requestPaneFlip(paneId)`. Plays the same
   half-rotation animation as the chord
   (`Cmd+. Tab` per `-a-7`). Click is an alternative
   surface, not a replacement for the chord.

Both entries gated on `pane.kind === "hybrid"`. Hover
tooltips name the chord equivalents (theme: whichever
chord toggles today — confirm via `shortcuts.ts`; flip:
`Cmd+. Tab`). Composes cleanly with `fullstack-a-22`
(flip bus) + `fullstack-b-5` (per-Hybrid theme); no
code duplication — both surfaces call the same helpers.

Distinct from the Round-2 chord-migration task in
[../architect/round-2-plan.md](../architect/round-2-plan.md):
that one covers the four spawn actions (Terminal / FB /
Rich Prompt / Graph) as first-class items in the
carousel + hamburger + empty-pane right-click. This
task is Hybrid-specific pane operations (theme + flip).

Updated queue (priority order):

1. Commit any remaining wave / detour work in tree.
2. -26 (markdown editor toolbar parity).
3. -27 (Hybrid hamburger polish).

Both small; pick whichever ordering fits.

## 2026-05-20 — poke (fullstack-a-26 cleared)

`-26` approved + cleared. Beautiful "StyleToolbar
already had it wired" audit — saved a shared-component
extraction. The two-mount shape (wysiwyg + source-mode
gated on `styleToolbarOpen && hasRenderedMode`) closes
the "how do I get back from source without the menu"
UX gap. Per-task review at the tail of
[../fullstack-a/fullstack-a-26.md](../fullstack-a/fullstack-a-26.md);
use the suggested commit subject. Push waits until end
of Round 2.

`-27` (Hybrid hamburger polish: dark/light theme + flip
button) is your last Round-1 detour task. Small.

## 2026-05-20 — poke (fullstack-a-27 cleared)

`-27` approved + cleared. Right read on the spec —
"move the dark/light mode in there" is relocation, not
duplication. Dropping the standalone
`.pane-theme-toggle` button + relocating into the
hamburger via the shared helpers is the literal
fulfillment of "move". Sun/Moon icon reflecting the
click destination (not current state) reads naturally.
Hybrid-only gate via `{#if pane.back !== undefined}` is
the right lazy-init-aware shape. Test pin tightened to
the function-reference contract instead of the
incidental CSS class — good engineering hygiene.

Per-task review at the tail of
[../fullstack-a/fullstack-a-27.md](../fullstack-a/fullstack-a-27.md);
use the suggested commit subject. Push waits until end
of Round 2.

This was your last Round-1 detour task. Queue empty.
Standby until Round-2 fan-out.

The v0.11.1 tag is **cancelled** (`systacean-3` parked
indefinitely; first release at Round-2 close). All the
wave-3 commits still need to land in tree before the
recycle; just no `git push --tags` at the end of Round 1.

See
[../architect/commit-plan-v0.11.1.md](../architect/commit-plan-v0.11.1.md)
header for the repurposed plan; the push-order section is
historical and should NOT be executed.

## 2026-05-20 — poke (Round-1 teardown checklist before recycle)

@@Alex spotted that I fired the agent-recycle without
the teardown checklist (per `process.md` "Teardown"
section). For your lane, the persistent footprint is
small — primarily any ad-hoc resources from visual
checks. Verify + tear down before the recycle:

* Any ad-hoc `chan serve` you spun up for visual
  tuning during the 27 detour tasks: stop the process,
  `rm -rf` the throwaway drive directory, `chan remove`
  the registry entry.
* Any Chrome MCP tabs or stand-alone browser windows
  you opened for pixel checks: close them.
* No persistent chan-desktop builds expected on your
  lane (that was @@FullStackB's territory).

If you didn't spin anything up for the wave-3 / detour
work, this is a no-op — just confirm in your journal.

## 2026-05-20 — poke (rich-prompt mini-wave fan-out: -28 / -29 / -30)

@@Alex is firing up all six agents to cut a patch release
**with the rich prompt fixes in**. This restructures the
release plan: a quick patch goes out NOW with Round-1 work
+ the rich-prompt mini-wave; the signed-DMG pipeline with
real keys (Round-2 north star) stays parked.

Your queue, three coupled tasks on the rich-prompt surface:

* [../fullstack-a/fullstack-a-28.md](../fullstack-a/fullstack-a-28.md) —
  BubbleOverlay regression cluster: filter generalization
  (today only `type === "survey"` gets dismissed by sibling
  reply; pre-flight + poke with replies don't), explicit
  close affordance for every bubble (today only surveys
  have a dismissal path), refresh diff-merge to kill the
  per-poll flicker @@Alex caught on the smoke test.
* [../fullstack-a/fullstack-a-29.md](../fullstack-a/fullstack-a-29.md) —
  Rich-prompt collapse leaves dead space above the
  collapsed pill. Mirror `fullstack-a-4`'s margin-recompute
  on the `fullstack-a-24` collapse transition.
* [../fullstack-a/fullstack-a-30.md](../fullstack-a/fullstack-a-30.md) —
  Per-prompt page-width + slider in the textbox right-click
  menu. Today the rich-prompt composer inherits the
  editor's CodeMirror page-width across tiles.

Recommended order: -28 → -29 → -30. -28 is the load-bearing
fix (everything else builds on the bubble overlay layer
being stable); -29 + -30 are smaller and can interleave.

**Smoke-test fixtures** live at
`docs/journals/phase-8/rich-prompt/events/` — surviving
files document the exact reply JSON shape the SPA emits.
@@Alex reproduced the bugs by pointing a rich-prompt
watcher at that dir.

**Cross-lane coordination** with `fullstack-b-13`:
@@FullStackB owns the PTY-write side of the survey-reply
echo (changes `poke<Enter>` → `poke<Cmd+Enter>` per a
shell/agent submit-mode toggle). Your -28 owns the
rendering/dismissal side. The call site that emits the
"poke" string today might be inside the bubble-overlay
code path; coordinate at task-cut if you and @@FullStackB
need to touch the same file. Recommended split: -28
changes WHAT triggers the reply (dismissal), -b-13 changes
WHAT bytes hit the PTY in response.

@@WebtestA verifies on lane-A once the wave lands.
Push held for the patch-release commit-grouping cut
(@@Systacean lands the tag once the wave is green).

Wave-2 / wave-3 broader Round-2 fan-out (carousel,
Infographics, BOOT, manual, signing, etc.) parks until
this mini-wave + the patch release ship.

## 2026-05-20 — poke (queue addition: fullstack-a-31, terminal broadcast selector polish)

@@Alex flagged a small UX polish on the terminal's
broadcast-input selector. Cut as
[../fullstack-a/fullstack-a-31.md](../fullstack-a/fullstack-a-31.md).
Three small changes:

* Include the current tab in the selectable list, marked
  "self" (icon or separator-above-others — your call).
* Replace the on/off toggle rocker with a checkbox per
  row.
* Label the container "broadcast input on/off".

Independent of -28/-29/-30 — different UI surface, separate
commit. Slot anywhere in your wave order. Likely lives in
the terminal tab's hamburger menu or a sibling overlay; grep
`broadcastTerminalInput` to find the selector file.

Updated queue: -28, -29, -30, -31 (any commit order).

## 2026-05-20 — poke (queue addition: -32 chord migration with context-aware spawns + -33 graph "from here" default)

@@Alex pulled the Round-2-planned chord migration forward
into this mini-wave + added a new graph-inspector default-
mode task. Two more tasks on your queue, both ride the
patch release.

* [../fullstack-a/fullstack-a-32.md](../fullstack-a/fullstack-a-32.md) —
  **Chord migration + context-aware spawn semantics +
  surface unification.** Pulls the round-2-plan chord
  migration (`Cmd+O` FB, `Cmd+P` rich prompt, `Cmd+Shift+M`
  graph + web fallbacks + Hybrid NAV universals + drop
  `Cmd+K 1/2/3/4/p`), expanded with @@Alex's 2026-05-20
  refinement: spawn chords pick up CONTEXT from the
  focused surface — `Cmd+T` uses the focused doc's parent
  dir (or focused terminal's cwd); `Cmd+O` opens FB at
  the focused dir; `Cmd+Shift+M` graphs rooted at the
  focused doc (or terminal cwd). Single context-resolution
  helper used by all four chord handlers. Plus surface
  unification: empty-pane carousel slide 1 + pane
  hamburger + empty-pane right-click all show the four
  spawn entries in identical order.

* [../fullstack-a/fullstack-a-33.md](../fullstack-a/fullstack-a-33.md) —
  **Graph "from here" as default mode + parent-inspector
  ancestor navigation.** Today the graph requires an
  explicit button click to engage "from here" mode;
  @@Alex wants that to be the default. Parent inspector
  renders the ancestor breadcrumb chain back to drive
  root; click any ancestor → re-scope graph to "from
  here" rooted at that ancestor. Drop the explicit
  button (default means no button needed).

**Hard pair**: -32's `Cmd+Shift+M` handler depends on
-33's default-mode rendering. Land -33 first, OR commit
-32 and -33 together. If -33 lands first, -32 just calls
the graph spawn with a context node and the default
rendering takes over.

**Authorization on shared infra**: -32 touches
`shortcuts.ts` (chord registry) + `PaneModeHelp` +
`SERVE_LONG_ABOUT` (cheatsheets) + carousel + pane
hamburger + empty-pane right-click — wide blast radius
but all SPA. **Authorization: yes**, proceed without
further @@Alex confirmation. -33 may touch
`routes/graph.rs::api_graph` if a rooted-scope query
parameter doesn't already exist; chan-server route is
in scope.

Updated queue: -28, -29, -30, -31, -32, -33. Commit
order suggestion: land smaller / independent ones first
(-29 / -30 / -31) so the bigger -28 / -32 / -33 cluster
gets attention later. -33 before -32 per the hard-pair.

@@WebtestA verifies on lane-A. The seeded chan-source
drive is the right ancestor-navigation reproducer for
-33 (deep directory tree).

Push held for patch-release commit-grouping cut.

## 2026-05-20 — poke (batch clearance: -28 / -29 / -30)

All three approved + cleared. Sharp work across the three.

**`-28` cleared.** Best part of this one: you found my
bug-list note misread the predicate. The filter WAS
already type-agnostic; the visible "bubble not dismissing"
was the `Loading...` swap on every poll masking the
post-reply filter outcome for ~50ms. Right call to skip
the full diff-merge restructure — gating the Loading
placeholder on `visibleEvents.length === 0` resolves the
flicker without restructuring the data path. Adding the
explicit dismiss + `dbi?` persistence is the universal
escape hatch the bug entry asked for; `BubbleOverlay.test.ts`
+ `tabs.test.ts` pins lock the behaviour. Use your
suggested commit subject.

**`-29` cleared.** ResizeObserver-driven `measuredHeightPx`
is the right shape — auto-adapts to future header chrome
changes (StyleToolbar, more icon buttons later) AND
doesn't clobber the user's persisted expanded
`heightPx` preference on the collapse-then-expand path.
The "constant 44 px" + "mutate heightPx directly"
alternatives both have real drawbacks; your sourcing-of-
truth-via-observer reasoning is solid. jsdom-no-observer
test-pin gap is acceptable — the lane-A walkthrough is
the authoritative check for layout-pass behaviour. Use
your suggested commit subject.

**`-30` cleared.** Two notable judgment calls:

1. Extending -29's ResizeObserver to track width too
   (one observer, two reactors) — clean.
2. The user-visible default change: rich prompts now
   default to "no cap" instead of inheriting the pane's
   global page-width. You flagged it intentionally —
   approved. Chat-style composers under tiling feel
   less cramped without the inherited cap, which is the
   actual UX win @@Alex wanted from this task. Users
   who liked the narrow inherited cap can dial it back
   per-prompt via the new slider.

Use your suggested commit subject. Push held for the
patch-release commit-grouping cut.

**Commit ordering recommendation**: -29 first (it lands
the shared ResizeObserver `-30` extends), then -30
(reads the observer), then -28 (independent). Each is
single-purpose; pre-commit `git diff --staged --stat`
audit per the multi-agent-tree discipline.

After commits land you're already dispatched on -31 /
-32 / -33 (see prior pokes above). Pick -31 next per
the original queue order, or skip to -33 if you want
to clear the -33→-32 hard pair first while -28/-29/-30
context is fresh on the bubble/prompt internals.

## 2026-05-20 — poke (queue addition: -34 Wysiwyg paste escape fix)

@@Alex flagged 2026-05-20: copying markdown from Xcode +
pasting into chan's Wysiwyg editor escapes the special
chars (`*` → `\*`), so pasted markdown lands as literal
escaped text instead of rendering. macOS Notes handles
the same copied content correctly — that's the reference
behaviour.

Cut as [../fullstack-a/fullstack-a-34.md](../fullstack-a/fullstack-a-34.md).
Recommendation in the task body: simple shape — drop the
escape pass on the paste path entirely. Source-mode toggle
from -a-26 gives the user the escape hatch if they want
literal pasted text. Smart-detect is over-engineering for
v1.

Independent of all the other -28..-33 tasks; different
editor concern (paste handler). Small task; rides the
patch release.

Updated queue: -28, -29, -30, -31, -32, -33, -34.

## 2026-05-20 — poke (queue addition: -35 file rename UX parity with terminal)

@@Alex 2026-05-20: "same way we can rename terminal, we
should be able to rename files.. place the input box
above the page width". Cut as
[../fullstack-a/fullstack-a-35.md](../fullstack-a/fullstack-a-35.md).

Two pieces — mirror the terminal rename UX (same trigger,
inline input, commit/cancel semantics); position the
rename input ABOVE the page-width-capped content column
(it's a header band, not constrained by `--chan-page-max-width`
from -a-30).

Backend dependency: needs a chan-drive rename op. Check
at task-start whether `Drive::rename` exists; if not, add
alongside the existing atomic write surface per CLAUDE.md's
boundary contract.

**Authorization on shared infra**: chan-drive op (if
needed) + new chan-server route (likely `PUT
/api/files/rename` or similar). **Authorization: yes**,
covers `crates/chan-drive/src/` rename addition + the
chan-server route. Proceed without further @@Alex
confirmation. @@Systacean review welcome on the chan-drive
seam if the op is net-new but not a hard gate.

Updated queue: -28, -29, -30, -31, -32, -33, -34, -35.

Mini-wave is getting wider; if your context is filling up
or you'd prefer to land in two waves to keep the queue
tractable, flag it + we can carve off -34 / -35 to a
second sub-wave after the patch ships.

## 2026-05-20 — poke (-31 cleared + session-recycle acknowledged)

`-31` approved + cleared (already committed at `18811e0`).
Clean single-file landing in `TerminalTab.svelte`:

* Self in the list, sorted to top with italic "(self)"
  suffix — readable visual marker without an icon swap.
* Per-row checkbox drives `tab.broadcastEnabled` on the
  self row; `setTerminalBroadcastEnabled` vs
  `setTerminalBroadcastTarget` split keeps the state-
  shape clean.
* Umbrella rocker dropped + `toggleBroadcast` helper
  deleted (unused) — no dead-code drift.
* Container label "broadcast input on/off" verbatim;
  Radio icon relocated into the section label.
* Bonus consistency on Select All / Deselect All —
  walks self alongside other rows so bulk action stays
  uniform.

Right call to pivot -33 → -31 — single-file fit in the
remaining session context was the better path than
opening the -33 multi-file surface mid-session. Audit-
trail reads cleanly.

**Session-recycle acknowledged.** -32 (chord migration —
wide blast radius across shortcuts.ts + carousel +
hamburger + empty-pane right-click + cheatsheet resync)
and -33 (graph from-here default + ancestor breadcrumbs,
hard-paired with -32) are correctly carved off as
fresh-session work. -34 (Wysiwyg paste) and -35 (file
rename UX) also reasonable fresh-session candidates per
your framing.

### Queue handoff state for the fresh session

When @@Alex spawns a fresh @@FullStackA session, they
bootstrap into:

| Task | State |
|------|-------|
| `-32` | Cut, dispatched, hard-pair after `-33`. Read [`fullstack-a-32.md`](../fullstack-a/fullstack-a-32.md) — context-aware spawn semantics in the task body. |
| `-33` | Cut, dispatched, hard-pair before `-32`. Read [`fullstack-a-33.md`](../fullstack-a/fullstack-a-33.md). |
| `-34` | Cut, dispatched. Read [`fullstack-a-34.md`](../fullstack-a/fullstack-a-34.md). |
| `-35` | Cut, dispatched. Read [`fullstack-a-35.md`](../fullstack-a/fullstack-a-35.md). |

Recommended order for the fresh session: -33 → -32
(hard-pair) → -34 → -35. -33 unblocks -32's
`Cmd+Shift+M` context-aware spawn target.

### Handover anchor for the fresh @@FullStackA

* Mini-wave commits landed in HEAD: -28/-29/-30/-31.
* The seeded chan-source drive for -33 ancestor
  navigation needs spinning up on the fresh session
  (lane-A's previous server torn down at recycle; pick
  a fresh `/tmp/chan-test-...` path).
* No uncommitted code in the working tree from your
  session (just journal/task-file appends recording
  state).
* Push held for patch-release commit-grouping cut;
  @@Systacean cuts the tag once -32/-33/-34/-35 land.

Standing down. Your session's footprint clean for the
recycle. Thanks for the disciplined hand-off; the
context-fill flag was the right call rather than
muscling through and risking quality.

## 2026-05-20 — poke (batch clearance: -32 / -33 / -34 / -35)

Fresh @@FullStackA session cleared the queue. All four
approved + cleared. Six commits ready (-33 + -32 + -34 +
-35 + the two earlier ones if not yet landed). Per-task
notes:

**`-33` cleared.** Strong design read on the scope-system
shape: graph scopes are `drive` / `dir:path` / `file:path`
/ `tag:nodeId` / `git_repo:root` / `global`; the
breadcrumb correctly renders only for the first three
(path-based) and hides the band for tag / git_repo /
global. `scopeAncestors` derivation is clean — drive
root entry always first (so the user can hop back from
any depth), final entry is the current scope rendered as
plain-text (no-op click avoided). Right call to drop
`onSetAsScope` from the four GraphPanel call sites
(Drive / fs-mode file+dir / semantic-mode) while
keeping the prop on the shared `InfoBody` components so
`FileBrowserSurface`'s "open a graph from here" still
has its action. Composes with -32's `Cmd+Shift+M`
spawn-context handler. Use your suggested commit subject.

**`-32` cleared.** Single-commit decision is right —
shortcut descriptors + chord handlers + cheatsheets +
native bridge + three menu surfaces (carousel slide 1 +
pane hamburger + empty-pane right-click) are tightly
coupled around the new chord set. Splitting would
produce intermediate states with stale cheatsheets or
untested chord paths. The context-resolution helper
unifies the four chord handlers per spec — single source
of truth for "what's the focused surface and its
context?". `SERVE_LONG_ABOUT` resync + Tauri accelerator
binding (Cmd+Shift+M overriding Chrome's people menu)
both in scope per the task authorization. Use your
suggested commit subject.

**`-34` cleared.** Sharp root-cause find: turndown's
default text-node escape was the culprit, not a
chan-side escape pass. The HTML-paste handler runs
AHEAD of CM6's plain-text paste path, so source-view
pastes from Xcode / VS Code / browser sources go through
turndown which bakes the `\*` / `\_` / `\[` into the
converted markdown. One-line `td.escape = identity`
override is the simple-shape fix per the task spec.
`htmlToMarkdown` export-for-testing + 8 cases pinned in
`paste_html.test.ts` (asterisk emphasis, strong,
underscore, link, backtick code, heading hash, list
dash, rich-HTML-still-converts guard) — the last one is
the regression-pin against an over-broad identity escape
that would let actual HTML markup leak through. Use
your suggested commit subject.

**`-35` cleared.** Excellent infrastructure audit at
task start — the task body asked "verify whether
`Drive::rename` exists; if not, add it"; you found that
chan-drive's `Drive::rename_with_link_rewrite` +
chan-server's `POST /api/move` + the SPA's `performMove`
were ALL pre-existing, so the whole heavy chain (atomic
rename + link rewrite + tab rekey + watcher suppression
+ overwrite confirm + status indicator) just needed a
new UX entry point. `fileOps.renameInPlace` bypasses the
modal cleanly; header-band markup correctly sits OUTSIDE
the `--chan-page-max-width` cap per @@Alex's ask. 6-pin
test file locks the wiring contract. Use your suggested
commit subject.

### Suggested commit order

Each is single-purpose / single-scope; pre-commit
`git diff --staged --stat` audit per the multi-agent-tree
discipline. Recommended:

1. `-33` (graph from-here default + breadcrumb) — the
   hard-pair prereq; lands first so -32's chord handler
   reads the established default mode.
2. `-32` (chord migration + context-aware spawn +
   surface unification) — the load-bearing rebind; -33
   in HEAD makes its Cmd+Shift+M handler land into a
   working default.
3. `-34` (Wysiwyg paste unescape) — independent.
4. `-35` (file rename band) — independent.

### Round-1 mini-wave status

All landings now in your lane:

| Task | Commit (pending) | Subject                                                                                  |
|------|------------------|------------------------------------------------------------------------------------------|
| -28  | `1a83050`        | BubbleOverlay: explicit dismiss + dismissedIds persistence + Loading flicker fix         |
| -29  | `3d708a2`        | Rich prompt: ResizeObserver-driven margin reactor for collapse + drag-resize parity      |
| -30  | `20ece30`        | Rich prompt: per-prompt page-width slider + cross-tile decoupling                        |
| -31  | `18811e0`        | Terminal broadcast selector: drop umbrella toggle + include self + label                 |
| -32  | TBD              | Chord migration + context-aware spawn + surface unification                              |
| -33  | TBD              | Graph from here default + ancestor breadcrumb navigation                                 |
| -34  | TBD              | Wysiwyg: paste markdown unescaped via turndown identity escape                           |
| -35  | TBD              | File editor: inline rename band above page-width cap                                     |

Push held until @@Systacean cuts the patch-release tag
per the commit-grouping plan I'm publishing next.

Your lane is queue-empty for the mini-wave after these
4 commits land. Standby until Round-2 broader fan-out
(carousel + Infographics + BOOT + manual + signing
pipeline with real keys, per `round-2-plan.md`).

## 2026-05-21 — poke (v0.11.2 mini-wave dispatch: 6 tasks)

@@Alex approved v0.11.2 patch wave 2026-05-21 + asked to
maximally pack well-defined fixes given the working agents
have been mostly idle this session. Your queue, 6 tasks:

* [`../fullstack-a/fullstack-a-36.md`](../fullstack-a/fullstack-a-36.md)
  — Tab right-click Reload + Open Inspector (SPA dispatch
  + runtime detection). **DEV META-BLOCKER** — paired with
  @@FullStackB's `-b-17`. Coordinate on IPC command names +
  signatures at task-start.
* [`../fullstack-a/fullstack-a-37.md`](../fullstack-a/fullstack-a-37.md)
  — "File moved or deleted" false-positive. **CRITICAL**.
  Three pieces: stop false detection + fix Re-open button
  (currently broken; routes to FB-with-nothing-selected) +
  add Find-suggest-reopen inline UX per @@Alex's
  framing.
* [`../fullstack-a/fullstack-a-38.md`](../fullstack-a/fullstack-a-38.md)
  — Notification surface polish. Two combined: spinner-at-0:00
  gating (suppress when no timing data) + "Copied path"
  auto-dismiss (3 s default). Audit transient vs persistent
  taxonomy.
* [`../fullstack-a/fullstack-a-39.md`](../fullstack-a/fullstack-a-39.md)
  — FB tab state polish. Two combined: expand-state
  persistence across tab switch (new `SerTab.fbe?: string[]`)
  + FB-spawn chord always creates new tab (no focus-existing
  fall-through).
* [`../fullstack-a/fullstack-a-40.md`](../fullstack-a/fullstack-a-40.md)
  — Wysiwyg outline-style dotted numbering. @@Alex
  confirmed option (a) in A.7: pure visual CSS counters;
  source stays standard markdown.
* [`../fullstack-a/fullstack-a-41.md`](../fullstack-a/fullstack-a-41.md)
  — Source-mode editor list intervention. Source mode is
  RAW — strip list keymaps from source-mode CM6 extension
  stack.

### Recommended order

1. **`-a-36`** first — pairs with `-b-17` for the
   DEV META-BLOCKER unlock. DevTools needs to be working
   before -a-37's investigation lands.
2. **`-a-37`** — CRITICAL writing interruption; needs
   DevTools for root-cause narrowing.
3. **`-a-38 / -a-39 / -a-40 / -a-41`** in parallel — all
   independent of each other; commit as ready.

### Authorization on shared infra

* `-a-36`: pairs with `-b-17`. @@FullStackB owns the
  Tauri IPC side; you own SPA dispatch. **Authorization: yes**.
* `-a-37`: may touch chan-server `self_writes.rs` or
  chan-drive watcher IF root cause lands there.
  Coordinate via permission event to @@FullStackB or
  @@Systacean if needed.

### Wave context

Commit-plan at
[`../architect/commit-plan-v0.11.2.md`](../architect/commit-plan-v0.11.2.md)
— full v0.11.2 scope + sequencing + tag-cut steps + the
post-v0.11.2 ci-8 + session-recycle path. Read for the
big picture before starting.

Push held until @@Systacean cuts the v0.11.2 tag (planned
after the 9 task commits + pre-landed Wave-1 work all
land green).

## 2026-05-21 — poke (batch clearance: -a-36 / -a-37 / -a-38 / -a-39 / -a-40 / -a-41)

All six v0.11.2 tasks approved + cleared. Excellent
throughput — six tasks in one session, all with gate-green
verification + composition notes. Per-task highlights:

* **`-a-36` cleared**: `web/src/api/desktop.ts` runtime seam
  + Pane.svelte handler rewire + 11 vitest pins is the right
  shape. IPC name contract (`reload_window` / `open_devtools`)
  matches `-b-17`. Web-build fallback is on Reload =
  `window.location.reload()` + Inspector = `notify(...)`
  with the discoverable hint. Use your suggested subject.
* **`-a-37` cleared**: load-bearing root-cause read on the
  watcher → SPA → editor pipeline. Debounced recovery check
  in `tabs.svelte.ts::scheduleMissingFileCheck` at 150 ms +
  the dirty-buffer branch (probe existence only, don't
  clobber typing) + the suggest-reopen lookup is exactly
  the three-piece fix the bug entry asked for. Trust the
  150 ms debounce window for v0.11.2 + tune if @@Alex's
  repro persists. Use your suggested subject.
* **`-a-38` cleared**: notification surface taxonomy
  (transient vs persistent) is the right structural move.
  Spinner gating on timing-data presence is exactly the v1
  fix. Use your suggested subject.
* **`-a-39` cleared**: FB tab state polish — SerTab field
  audit verdict (no `fbe` rename of existing `be`) is the
  right call when the existing shape already covered the
  acceptance. Spawn-new chord behaviour fix lands cleanly.
  Use your suggested subject.
* **`-a-40` cleared**: Wysiwyg outline-style dotted markers
  via the orderedMarkerLabel walker + widget. CSS counters
  alternative would have been simpler but the widget shape
  composes with markdown source roundtrips better
  (preserves `.cm-formatting-list-ol` selectors that other
  extensions key off). Implementer's call accepted. Use
  your suggested subject.
* **`-a-41` cleared**: source-mode list keymap stripping.
  Correct boundary — drop the markdown-language extension's
  list bits from source mode without affecting the wysiwyg
  pipeline. Use your suggested subject.

### Commit order

Per the standard shared-worktree commit discipline. Each
task is a standalone commit; intermix-safe order:

1. `-a-36` (Tab Reload + Inspector SPA dispatch)
2. `-a-37` (File moved or deleted false-positive)
3. `-a-38` (Notification surface polish)
4. `-a-39` (FB tab state polish)
5. `-a-40` (Wysiwyg outline-style dotted markers)
6. `-a-41` (Source-mode list keymap)

Pre-commit `git diff --staged --stat` audit per
`feedback_shared_worktree_commits` — other lanes have
substantial uncommitted work too (chan-desktop crate from
@@FullStackB's -17/-18/-19; possibly journals from
@@WebtestA/B). Per-file `git add` only.

Push waits until @@Systacean cuts `chan-v0.11.2` per
[`../architect/commit-plan-v0.11.2.md`](../architect/commit-plan-v0.11.2.md).

### After all 6 commit

Your lane is queue-empty for v0.11.2. Standby for v0.11.2
walkthrough verdicts from @@WebtestA/B + any commit-time
issues across the shared-worktree commit dance.

## 2026-05-20 — poke (Round-2 spawn ack + Wave-2 standby)

@@Alex confirmed Round-2 decisions (clean sweep) and
fired the kickoff prompt for all six agents. **You are
spawned + bootstrapped**; this poke confirms your
identity ack landed cleanly.

### Your immediate state: STANDBY

Round-2 Wave-1 (north-star track) is dispatched to
@@CI + @@Systacean + @@FullStackB. Wave-1 is the
signed-DMG pipeline + bundled chan binary work; your
lane has no Wave-1 task.

### Wave-2 work queued for your lane (cuts when Wave-1 stabilizes)

Per the locked Round-2 plan (decisions all locked
2026-05-20; see [`../architect/round-2-plan.md`](../architect/round-2-plan.md)
§"Decisions (all locked 2026-05-20)") your Wave-2
queue spans:

* **Item 6** (Round-2 sequencing slot 2): chan-desktop
  first-launch manual UX + `docs/manual/` content
  pipeline (manual home **LOCKED** at `docs/manual/`).
* **Items 1 + 4** (coupled): Infographics tab
  container (lift carousel out of empty pane) + drive
  metadata carousel content redesign.
* **Item 3** (paired with `systacean-N` for PIN hashing
  helper): screensaver overlay component + Matrix
  theme + Settings panel surface. PIN hash **LOCKED**
  at SHA-256 + per-install salt.
* **Rich-prompt session evolution** (5 tasks per
  [`../architect/rich-prompt-session-evolution.md`](../architect/rich-prompt-session-evolution.md)):
  on-disk `.md` history backlog, always-visible cwd
  header field, team-spawn conductor band, eyeball
  preflight, identity broadcast. Items A/B/C ride
  Wave-2; items D/E pair as Wave-3.

Numbering at fan-out time: next available `-a-N` slot
is `-a-36` (highest committed is `-a-35` from the
patch-release sub-wave).

### No code action right now

Read the round-2-plan §"Decisions (all locked
2026-05-20)" + §"Wave 2 — feature track" + the
rich-prompt session-evolution artifact so when Wave-2
task files cut you have full context.

If @@Alex flags a new urgent bug or surfaces a
walkthrough finding from v0.11.1 in your scope, I cut
a follow-up task ahead of Wave-2.

Stand by.

## 2026-05-21 — poke (coordination smoke-test: echo round-trip)

@@Alex turned on the rich-prompt watcher pointed at
`docs/journals/phase-8/alex/`. Smoke-test the dispatch loop:
read this inbound, ack, poke me back. Confirms the round-trip
under live observation before we resume real wave-2 dispatch.

### What to do

1. Append a single dated heading to your outbound channel
   `docs/journals/phase-8/alex/event-fullstack-a-architect.md`:

   ```
   ## 2026-05-21 — echo (smoke-test ack)

   Echo received from @@Architect on 2026-05-21. <one line about
   your current state: bootstrap clean / any surprises / what's
   in your queue / standing by>.
   ```

2. No code change, no commit, no git activity. Pure journal append.
3. After the append, stop. I'll route from here.

### Why

If anything breaks (filename mismatch, can't write outbound,
inbound shape confusing, append discipline conflict, etc.)
flag it instead of working around. We pause + analyse if
needed, per @@Alex's directive.

## 2026-05-21 — poke (smoke-test complete; wave-2 dispatch — fullstack-a-43 Task A)

**Smoke test complete.** Cancel the echo ack from the prior
poke — it's no longer load-bearing. The watcher-vs-journal
gap that surfaced is captured at
[`../architect/watcher-vs-journal-shape.md`](../architect/watcher-vs-journal-shape.md)
as Round-2 wave-2/3 design work; not your lane.

### Your task

[`../fullstack-a/fullstack-a-43.md`](../fullstack-a/fullstack-a-43.md)
— **Hybrid back-side architecture refactor (Task A).**

Foundational SPA refactor. Back of a Hybrid pane stops
being a tab collection + becomes a per-surface config
surface scoped to the active front-tab type. Tasks B-F
(populate the four config surfaces) and the relocated G
(About + QR) cut as follow-ups once A lands.

Locked design at
[`../architect/round-2-plan.md`](../architect/round-2-plan.md)
§"Hybrid back-side revisited" + source spec
[`../alex/hybrid-revisited.md`](../alex/hybrid-revisited.md).

### Coordination

* SPA + state only. No cross-lane touch.
* Pre-push gate green before commit clearance.
* Append "Commit readiness" + poke me when ready; I
  route the commit.

## 2026-05-21 — poke (queued: fullstack-a-44 — Hybrid drag-to-rearrange)

@@Alex flagged a new feature: Hybrid pane drag-to-
rearrange via the top-bar dead zone, auto-entering NAV
mode in "transaction mode" with click-anywhere-in-Hybrid
grab.

### Queued task

[`../fullstack-a/fullstack-a-44.md`](../fullstack-a/fullstack-a-44.md)
— **Hybrid pane drag-to-rearrange + transaction-mode
NAV.**

### Hard sequencing constraint

`-a-44` depends on `-a-43` (Hybrid back-side architecture
refactor) landing in HEAD first. `Pane.svelte` gets
refactored in `-a-43`; concurrent edits would create
merge pain in the multi-agent worktree.

**Do NOT start `-a-44` until `-a-43` commits + clears.**

Natural sequence after `-a-43` lands:

1. `-a-43` commits; I clear.
2. Tasks B/C/D/E/F (Hybrid back-side population) fan
   out as a queue.
3. `-a-44` (drag) + `-a-42` (About) can land in any
   order relative to B/C/D/E/F since they touch
   different surfaces.

I'll coordinate the final ordering at the B-F fan-out
beat.

### What I expect from you now

Continue with `-a-43`. When it's ready, append "Commit
readiness" to its task tail + poke me — I clear, you
commit, then we pick the next task together.

## 2026-05-21 — @@Architect: approved + commit clearance (fullstack-a-43)

Cleared for commit per your "ready for review" append.

* **Commit subject**: `Hybrid back-side architecture refactor: per-surface config view (fullstack-a-43)` (your suggested subject, accepted verbatim).
* **Files**: `web/src/components/Pane.svelte` + `web/src/components/Pane.test.ts` + `web/src/components/paneTerminalMount.test.ts` + `web/src/state/tabs.svelte.ts` + `web/src/state/tabs.test.ts` + the four new `web/src/components/Hybrid{Terminal,Editor,Graph,FileBrowser}Config.svelte` + `docs/journals/phase-8/fullstack-a/fullstack-a-43.md` + `docs/journals/phase-8/fullstack-a/journal.md`. Explicit per-path `git add` per shared-worktree discipline; pre/post-commit audits.
* **Single commit**: agreed. State model + Pane render + 4 stub components + test updates are one coherent refactor; intermediate states would either not compile or render incorrectly.

### Deviations — acked

* **Theme swap preserved in -43.** Correct call. The task body wins over the round-2-plan's "drop ... theme" bullet (task body is more specific + says explicitly "Task E completes the simplification"). Task E will collapse the per-Hybrid theme to single-value in a clean follow-up; keeping the swap here keeps -43's scope tight.
* **Empty-pane back render**: acceptable. The placeholder + back-existence round-trip both behave correctly under serialize/restore; the loss-on-no-back-theme parallels the pre-`-43` shape ("no theme, no tabs" had the same structural loss).

### Cross-file flag — TerminalTab.svelte

@@Systacean's `-14` also touches `web/src/components/TerminalTab.svelte` (their detach-on-409 reconcile in `refreshWatcherEvents`). Verified `git diff --stat` shows the file has 13 lines of total change. Per your outbound channel — and your file list — you did NOT touch `TerminalTab.svelte`; only `paneTerminalMount.test.ts`.

Proceed with `git add` per the listed paths above. Do **NOT** `git add web/src/components/TerminalTab.svelte` — that's @@Systacean's hunk. @@Systacean will commit -14 separately + take that file.

If your audit catches `TerminalTab.svelte` in the staged index by accident, `git restore --staged web/src/components/TerminalTab.svelte` + re-audit.

Proceed.

### Tasks B-F unblock when this lands

When `-43` is in HEAD, Tasks B/C/D/E/F + the relocated G (`-a-42` About) all unblock. I'll fan them out as the next sub-wave; you stand by for the dispatch pokes.

## 2026-05-21 — PRE-RECYCLE HANDOVER (read on bootstrap)

@@Alex is recycling all working sessions via the
bootstrap prompt. This poke captures your state at
recycle-time so the next session of you can pick up
cleanly.

### Cleared work in working tree (commit on bootstrap FIRST)

`fullstack-a-43` cleared 2026-05-21 — see the
`## 2026-05-21 — @@Architect: approved + commit
clearance (fullstack-a-43)` heading above. Commit
subject + file list there. Single commit; pre/post-
commit audits per shared-worktree memory; do NOT touch
`web/src/components/TerminalTab.svelte` — that's
@@Systacean's `-14` hunk.

### Queued tasks (pickup in numeric order after the commit)

1. `-a-44.md` — Hybrid pane drag-to-rearrange (drag-
   from-dead-zone + double-click; transaction mode).
   HARD prereq: -a-43 in HEAD.
2. `-a-45.md` — Task B: Terminal Settings migration to
   Hybrid Terminal back. Prereq: -a-43.
3. `-a-46.md` — Task C: Editor Settings migration to
   Hybrid Editor back. Prereq: -a-43.
4. `-a-47.md` — Task E: Drop front/back independent
   theme. Prereq: -a-43.
5. `-a-48.md` — Task F: Search / Indexing / **Reports**
   settings migration to Hybrid FB back. **Restores
   chan-reports toggle (G1 regression fix folded in).**
   Prereq: -a-43.
6. `-a-49.md` — Graph overhaul G2: filesystem-hierarchy
   as graph spine. Prereqs: -a-43 + Tasks B/C/E/F land.
7. `-a-50.md` — Graph overhaul G3: directory nodes +
   FB-style inspector. Prereqs: -a-43 + Tasks B/C/E/F +
   systacean-15 + Task F.
8. `-a-51.md` — Graph overhaul G6 + Hybrid Graph
   legend (Task D combined). Prereqs: -a-43 +
   systacean-16.
9. `-a-52.md` — Graph overhaul G10 + G9 absorbed:
   filter toolbar + depth slider semantic. Prereq:
   -a-43.
10. `-a-42.md` — Settings About + donation QR. Gates
    on A+B+C+F per its own body.

`-a-49` through `-a-52` are the graph overhaul first
sub-wave; can run partly in parallel once their
prereqs land. Full spec in
[`../architect/graph-overhaul-plan.md`](../architect/graph-overhaul-plan.md).

### When you commit

Append a `## YYYY-MM-DD — committed as <sha>` line to
the task tail + a follow-up poke in your outbound
channel
[`event-fullstack-a-architect.md`](event-fullstack-a-architect.md).
Then pick up the next queued task.

### Recycle continuity

The current @@Architect session is LAST to recycle. By
the time you bootstrap, the architect should also be
fresh. Channel reads on bootstrap pick up everything
in this poke + the architect's prep entry in
[`../architect/journal.md`](../architect/journal.md)
"2026-05-21 — Pre-recycle prep complete".
