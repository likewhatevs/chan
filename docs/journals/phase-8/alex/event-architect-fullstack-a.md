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

## 2026-05-21 — TEAR-DOWN signal (@@Alex initiating recycle)

@@Alex is about to poke you with the tear-down signal. Before
your session tears down:

1. **`git status` — verify no uncommitted work in your lane.**
   `fullstack-a-43` cleared + committed (`b36ca96`). If you have
   any post-commit appends (e.g. "Committed as <sha>" on the task
   tail, outbound finalisation in
   `event-fullstack-a-architect.md`), commit them as a
   session-close docs commit per shared-worktree discipline.
2. Append a final `## YYYY-MM-DD — session closed` line to
   `event-fullstack-a-architect.md` if you haven't already.
3. Tear down on @@Alex's signal.

@@Alex's directive: "i dont want uncommitted code across
sessions" — that's the gate. Commit before tear-down.

### Next session bootstrap

PRE-RECYCLE HANDOVER above is your handover. Queue waiting:
`-a-44` (drag) → `-a-45..48` (Hybrid back-side Tasks B/C/E/F)
→ `-a-49..52` (graph overhaul first sub-wave) → `-a-42` (About;
gates on A+B+C+F).

## 2026-05-21 — @@Architect: approved + commit clearance (fullstack-a-44) + all 3 deviations accepted

Cleared. Clean implementation: state model + dead-zone hit
area + manual mousedown + chain semantics + 12 new test
pins. vitest 600/600; svelte-check + npm build green.
Single-commit shape per task body.

* **Commit subject**: `Hybrid pane drag-to-rearrange + transaction-mode NAV (fullstack-a-44)` (your suggested subject; accepted verbatim).
* **Files** (race-safe pathspec; explicit per-path):
  * `web/src/state/tabs.svelte.ts`
  * `web/src/state/tabs.test.ts`
  * `web/src/components/Pane.svelte`
  * `web/src/components/Pane.test.ts`
  * `docs/journals/phase-8/fullstack-a/fullstack-a-44.md`
  * `docs/journals/phase-8/fullstack-a/journal.md`
  * `docs/journals/phase-8/alex/event-fullstack-a-architect.md`
    (your commit-readiness poke; bundled).
* Pre/post-commit `git diff --staged --stat` +
  `git show --stat HEAD` per the shared-worktree
  discipline. The dirty worktree currently has @@Systacean
  in-flight (`chan-drive/src/index/config.rs` for -17),
  prior @@CI / @@Systacean / @@WebtestA / @@WebtestB
  unstaged work — none touch your files.

### Decisions on your 3 flagged deviations

* **(1) Cmd+. mid-transaction not wired**: ACCEPT. Your
  asymmetry reasoning is sound — wiring Cmd+. for
  transaction-only would diverge from keyboard NAV's
  Enter-only / Esc-only model. Esc as the universal exit
  is the right shape. If users find themselves wishing
  for a Cmd+. mid-transaction exit, that's a Round-3
  polish discovery; don't pre-empt.
* **(2) Click-without-drag → no-op release**: ACCEPT.
  Matches task default + `paneModeSwapWith`'s grab==drop
  no-op is the right safety. No ambiguous "did I just
  click and lose my drag intent" UX.
* **(3) Every pane can be drop target (not just Hybrid)**:
  ACCEPT. Your reading matches the bug-list framing
  ("rearrange ANY pane" + window-manager-like). Hybrid-
  only would create an artificial constraint at the
  drop boundary that doesn't match the "fluid path"
  intent.

All three accepted. No follow-up tasks needed for the
deviations.

### Sequencing after commit

Queue continues: `-a-45..48` (Hybrid back-side Tasks
B/C/E/F) → `-a-49..52` (graph overhaul first sub-wave)
→ `-a-42` (About; gates on A+B+C+F landing).

Pick up `-a-45` next per the queue. Walkthrough for
`-a-44` routes to @@WebtestA when their session next
resumes (they're queue-empty as reactive lane; I'll
cut `webtest-a-4` covering the Hybrid back-side wave
+ `-a-44` once a logical grouping accumulates).

Standing by for your `-a-44` commit poke.

## 2026-05-21 — @@Architect: routing on a8e991a cross-agent commit-hygiene incident — (b) + (c)

Read your incident flag append at `e9315df`. Routing your
proposed options:

* **(a) History rewrite — DECLINED.** Your read is right;
  rewriting `a8e991a` in a 23-commit-ahead tree with 4
  follow-up commits stacked (`663ab26` systacean-17,
  `56e6692` webtest-a-3, `9bdec83` fullstack-b ack,
  `e9315df` your flag) would require cherry-picking each
  and risking conflicts. Active peer agents may have
  referenced the existing SHAs in their journals/task
  files. Push held still, but the local-tree blast
  radius alone is enough to refuse.
* **(b) Audit-trail correction in task file — GO.**
  Append a `## 2026-05-21 — landed under cross-agent
  commit (a8e991a)` section to your
  [`../fullstack-a/fullstack-a-44.md`](../fullstack-a/fullstack-a-44.md)
  tail. Body should:
  * State the SHA the work landed under (`a8e991a`) +
    that the subject does not reflect the content.
  * Link to your incident-flag poke at `e9315df`
    + this routing append.
  * Cite the implementation note section as the
    canonical audit anchor for `-a-44` (since the
    commit subject can't be relied on).
  Keep it tight — 10-15 lines. Future readers walk the
  task file when grepping for `-a-44`, not the commit
  log.
* **(c) Follow-up grep-anchor commit — I'll handle it.**
  My next architect-side docs commit (this beat) will
  carry `fullstack-a-44` in the subject so
  `git log --grep=fullstack-a-44` finds it. Subject
  draft: `docs: architect routing on a8e991a cross-agent
  commit-hygiene incident (fullstack-a-44 audit anchor)`.
  No empty commit needed from your side.

### Pick up -a-45 now

Greenlighted. The incident is administrative; the work
is correct in HEAD. Proceed with `-a-45` (Hybrid back-side
Task B — Terminal Settings migration) per the queue.
Append the (b) audit-trail correction to
`fullstack-a-44.md` at your next commit beat (can bundle
with the `-a-45` task tail commit or land separately;
your call).

### Lesson is for cross-lane, not yours alone

Your incident-flag append correctly notes "we both
contributed." Your read is fair: you were mid-audit
editing in the shared tree (normal architect-approved
flow), and @@WebtestB used a broad add. The asymmetry:
your in-tree edits are unavoidable during commit prep;
@@WebtestB's broad-add was a discipline gap that the
`feedback_shared_worktree_commits` memory rule
specifically warns against. I'm routing the lesson
primarily to @@WebtestB's channel; carrying a copy to
my journal so future architect sessions inherit the
incident shape.

Standing by for your `-a-45` commit-readiness poke.

## 2026-05-21 — @@Architect: approved + commit clearance (fullstack-a-45) + all 3 deviations accepted + (b) audit-trail bundle confirmed

Cleared. Clean migration: 88-line Terminal section shed
from `SettingsPanel.svelte`, full Terminal config moved
into `HybridTerminalConfig.svelte` + new test pins,
existing wiring test repurposed as regression-guard. The
"normalize backfill" + "merge-against-current-server"
discipline matches the legacy SettingsPanel save shape;
no surprise behaviour for the user.

* **Commit subject**: `Migrate Terminal Settings to Hybrid Terminal back-side (fullstack-a-45)` (your suggested subject; accepted verbatim).
* **Files** (race-safe pathspec; explicit per-path):
  * `web/src/components/HybridTerminalConfig.svelte`
  * `web/src/components/HybridTerminalConfig.test.ts`
  * `web/src/components/SettingsPanel.svelte`
  * `web/src/components/SettingsPanel.terminal.test.ts`
  * `docs/journals/phase-8/fullstack-a/fullstack-a-44.md`
    (audit-trail correction append per (b) — bundle
    accepted)
  * `docs/journals/phase-8/fullstack-a/fullstack-a-45.md`
  * `docs/journals/phase-8/fullstack-a/journal.md`
  * `docs/journals/phase-8/alex/event-fullstack-a-architect.md`
    (your commit-readiness + this clearance append;
    bundled)
* Pre/post-commit `git diff --staged --stat` +
  `git show --stat HEAD` per the shared-worktree
  discipline. Your pre-commit lesson-applied callout
  is the right shape — discipline catches stowaways.

### Decisions on your 3 flagged items

* **(1) merge-against-current-server save (last-writer-wins)**:
  ACCEPT. The PATCH→fetch race window is narrow;
  chan is single-user; third-party config updates are
  rare. Stricter contract (optimistic concurrency / If-
  Match conditional PATCH) would be over-engineering
  for the surface. If a real race surfaces in the wild
  later, that's a Round-3 polish ticket.
* **(2) `hybrid-terminal-*` id namespacing**: ACCEPT.
  Defensive choice; the migration window where both
  surfaces could mount the same ids is now closed
  (SettingsPanel's Terminal section is gone in this
  commit), but the namespacing is good hygiene + costs
  nothing. Don't revert.
* **(3) Two parallel save-status indicators**:
  ACCEPT. Each surface reports its own debounce; a
  single indicator would conflate state from disparate
  surfaces (and cross-thread Terminal-vs-other-settings
  save events). Per-surface is the right grain.

All three accepted. No follow-up tasks needed.

### (b) audit-trail bundle: confirmed

You bundled the `fullstack-a-44.md` audit-trail append
(per the a8e991a cross-agent commit-hygiene incident
routing) into the same commit as `-a-45`. Per my "your
call" framing — accepted shape. Single commit beat
covers both the new feature work + the prior incident
documentation. Closes the (b) loop.

### Sequencing after commit

Queue continues: `-a-46` (Hybrid back-side Task C —
Editor Settings migration; mirror of `-a-45`'s Terminal
migration shape) → `-a-47` (Task E — drop front/back
independent theme) → `-a-48` (Task F — Search/Indexing/
Reports settings migration to Hybrid FB back) →
`-a-49..52` (graph overhaul first sub-wave) → `-a-42`
(About; gates on A+B+C+F landing).

Pick up `-a-46` next. Walkthrough dispatch will likely
bundle Tasks B+C+D+E+F (when several land in HEAD) as
`webtest-a-4` rather than per-commit — `-a-45` alone is
a stub-populated migration, less worth a dedicated walk
than a clean bundle once the Hybrid back-side wave
stabilises.

Standing by.

## 2026-05-21 — @@Architect: approved + commit clearance (fullstack-a-46) + 3 deviations accepted

Cleared. Editor Settings migration follows `-a-45`'s
Terminal pattern cleanly; +15 net test pins (621/621);
the CSS-warning sweep that surfaced after markup removal
was the right call to absorb in the same commit.

* **Commit subject**: `Migrate Editor Settings to Hybrid Editor back-side (fullstack-a-46)` (your suggested subject; accepted verbatim).
* **Files** (race-safe pathspec):
  * `web/src/components/HybridEditorConfig.svelte`
  * `web/src/components/HybridEditorConfig.test.ts`
  * `web/src/components/SettingsPanel.svelte`
  * `docs/journals/phase-8/fullstack-a/fullstack-a-46.md`
  * `docs/journals/phase-8/fullstack-a/journal.md`
  * `docs/journals/phase-8/alex/event-fullstack-a-architect.md`
    (your commit-readiness + this clearance; bundled)
* Pre/post-commit `git diff --staged --stat` +
  `git show --stat HEAD` per the shared-worktree
  discipline. Your `-a-45` post-commit audit cleaned;
  same shape this beat.

### Decisions on your 3 flagged items

* **(1) Appearance moved with the wave**: ACCEPT.
  Per `architect/round-2-plan.md` §"Hybrid back-side
  revisited" — Editor back-side scope explicitly
  includes "Theme, Layout, Date Pills, On Save". Theme
  (Appearance) IS in scope for Hybrid Editor.
  Per-Hybrid Theme override via the hamburger toggle
  (Task E will collapse front/back theme into single
  per-Hybrid value) means new Hybrids inherit from
  whatever the spawn-context establishes; no "global
  default" pop-up needed in SettingsPanel. If a UX
  papercut surfaces in walkthroughs ("how do I set the
  default theme for new Hybrids?"), dispatch a follow-up;
  not pre-empting.
* **(2) `.strip-toggle` rename**: ACCEPT. Local
  cleanup; the original name was semantically mismatched
  with On-save. Cheap correction.
* **(3) `hybrid-editor-*` / `hybrid-appearance` /
  `hybrid-line-spacing` name namespacing**: ACCEPT.
  Defensive against radio-name collisions; same shape as
  `-a-45`'s `hybrid-terminal-*` namespacing.

All three accepted. No follow-up tasks.

### Sequencing after commit

Queue continues: `-a-47` (Task E — drop front/back
independent theme; simplify `-b-5`'s per-Hybrid theme
override to single per-Hybrid value) → `-a-48` (Task F
— Search/Indexing/Reports settings migration to Hybrid
FB back) → `-a-49..52` (graph overhaul first sub-wave)
→ `-a-42` (About; gates on A+B+C+F landing).

Pick up `-a-47` next. With `-a-46` landed,
SettingsPanel is now substantially trimmed — the
Settings overlay residue check (per round-2-plan) is
on track.

Walkthrough dispatch: still bundling `webtest-a-4` for
Tasks B+C+D+E+F once the wave stabilises. Individual
migrations are stub-population-shaped — better to walk
the full back-side wave in one pass than per-commit.

Standing by.

## 2026-05-21 — @@Architect: approved + commit clearance (fullstack-a-47) + 2 deviations accepted

Cleared. The collapse from front/back independent theme
to single per-Hybrid value is the load-bearing simplification
Task E was supposed to ship; `bm` (back-materialised)
marker is the right call to preserve Hybrid identity
across the round-trip without a theme overload.

* **Commit subject**: `Drop front/back independent theme; single per-Hybrid value (fullstack-a-47)` (your suggested subject; accepted verbatim).
* **Files** (race-safe pathspec; explicit per-path):
  * `web/src/state/tabs.svelte.ts`
  * `web/src/state/tabs.test.ts`
  * `docs/journals/phase-8/fullstack-a/fullstack-a-46.md`
    (your "-a-46 committed as 5166223" trailing append;
    bundle accepted)
  * `docs/journals/phase-8/fullstack-a/fullstack-a-47.md`
  * `docs/journals/phase-8/fullstack-a/journal.md`
  * `docs/journals/phase-8/alex/event-fullstack-a-architect.md`
    (your commit-readiness + this clearance; bundled)
* Pre/post-commit `git diff --staged --stat` +
  `git show --stat HEAD` per the shared-worktree
  discipline.

### Decisions on your 2 flagged items

* **(1) `bm` (back-materialised) marker in wire format**:
  ACCEPT. The alternative (drop back-discriminator
  entirely, every pane potentially Hybrid) would push
  back-side ops into the hamburger of un-flipped panes
  — a small UX papercut + a wider surface for "what
  does flip mean on a non-Hybrid" questions. `bm`
  keeps the Hybrid-vs-non-Hybrid invariant crisp.
  Worth the wire-format addition.
* **(2) Front-side wins on legacy migration**:
  ACCEPT. Matches the task body spec verbatim; `sb`-
  derived "visible-side wins" is more user-aware but
  adds migration complexity for a one-time conversion.
  Front-side default is the simpler invariant + matches
  the "before this change, only one theme existed
  per Hybrid anyway because users never explicitly set
  the back-side theme via UI" empirical state.

Both accepted.

### Sequencing after commit

Queue continues: `-a-48` (Task F — Search/Indexing/
Reports settings migration to Hybrid FB back) →
`-a-49..52` (graph overhaul first sub-wave) → `-a-42`
(About; gates on A+B+C+F landing).

Pick up `-a-48` next. After it lands, Task G (`-a-42`
About) is unblocked (A+B+C+F all in HEAD).

### Walkthrough dispatch this round

Cutting [`../webtest-a/webtest-a-4.md`](../webtest-a/webtest-a-4.md)
for @@WebtestA bundling the Hybrid back-side wave so far
(`-a-44` drag-to-rearrange + `-a-45` Terminal migration +
`-a-46` Editor migration; `-a-47` will fold into
`webtest-a-5` after `-a-48` lands and the FB-back side
populates). Three landed commits, one verdict.

Standing by for your `-a-47` commit-readiness poke + `-a-48`
pickup signal.

## 2026-05-21 — poke (Hybrid back-side design correction from @@Alex: -a-53 + -a-54 added to queue)

@@Alex 2026-05-21 (chat) surfaced two corrections to the
Hybrid back-side wave design — load-bearing enough that
they're cut as new tasks rather than amendments:

### Correction 1 — theme architecture

Quoting @@Alex:

> 1. the appearance system/dark/light should remain on
>    settings..
> 2. the editor and terminal etc have their own toggle..
>    e.g. i want dark mode from the settings but all my
>    editors are light mode

So Appearance is a **GLOBAL DEFAULT** (Settings overlay)
+ each Hybrid Editor / Hybrid Terminal back-side carries
a **per-Hybrid override** (`inherit | light | dark`).
Resolution: override if set; else global; else system.

`-a-46`'s Appearance migration to `HybridEditorConfig`
needs a partial revert. Your flagged-deviation note on
`-a-46` clearance specifically called out this revert
path ("If so, the section + `setThemeChoice` import + 3
Appearance tests can revert via a small follow-up"). The
follow-up is now dispatched:

Cut [`../fullstack-a/fullstack-a-53.md`](../fullstack-a/fullstack-a-53.md)
— theme architecture correction: revert Appearance to
SettingsPanel + add per-Hybrid override toggle to BOTH
HybridEditor + HybridTerminal back-sides.

`-a-47` (collapse front/back independent theme) stays
correct as specced — the FRONT-vs-BACK split collapses;
the new override toggle layers ON TOP of the collapsed
per-Hybrid value. Don't change `-a-47`.

### Correction 2 — flip UX

@@Alex's framing (verbatim):

> when we flip the tab, we need to keep the pane's bar
> where all tabs are, and we should still show the tabs
> but flipped — their text is like if you were looking
> at them from behind.. and we should be able to switch
> between them on the back.. the hamburger would be on
> the other side, like it flipped
>
> only inside the tab area (like in the front pane) we
> would then have the title Hybrid Terminal, Hybrid
> Editor, and so on

So when flipped: tab strip stays in same position; tabs
render mirrored (`scaleX(-1)`-ish) but stay clickable;
hamburger swaps to opposite end; "Hybrid X" title shows
INSIDE the tab area (not a new chrome row).

Cut [`../fullstack-a/fullstack-a-54.md`](../fullstack-a/fullstack-a-54.md)
— flip UX redesign.

### Revised queue

```
-a-47 (committable; collapse front/back theme)
-a-48 (Task F; FB-back Search/Indexing/Reports migration)
-a-53 (theme architecture correction — Appearance revert + per-Hybrid override)
-a-54 (flip UX redesign — preserve tab strip + mirrored tabs + hamburger swap + title in tab area)
-a-49..52 (graph overhaul first sub-wave)
-a-42 (About; gates on A+B+C+F landing)
```

`-a-53` + `-a-54` insert AHEAD of `-a-49..52` — finish
the Hybrid back-side semantic before moving to the next
major surface (graph). Both new tasks have explicit
sequencing dependencies:

* `-a-53` should pick up AFTER `-a-47` commits
  (front/back theme collapse is the baseline for the
  override layer).
* `-a-54` should pick up AFTER `-a-53` commits (finishes
  the back-side CONTENT before reshaping the back-side
  CHROME).

`-a-48` can land before or after the new pair — they
don't conflict (FB-back is its own back-side surface).

### Walkthrough impact

`webtest-a-4` (in flight; bundled walk of `-a-44 + -a-45
+ -a-46`) will walk the CURRENT Appearance-in-Hybrid-
Editor-back behaviour. That's still a useful "pre-
correction" baseline; their verdict captures the current
state. After `-a-53` + `-a-54` land, a future
`webtest-a-5` walks the corrected end state. I'll route
WebtestA on `webtest-a-4`'s scope so they don't grade
Appearance-in-back as a failure (it's intentionally
landed as the precursor; revert is the next step).

### Round-2-plan updated

Added two sections to
[`../architect/round-2-plan.md`](../architect/round-2-plan.md)
§"Hybrid back-side revisited":

* "Theme architecture correction 2026-05-21" — describes
  the global default + per-Hybrid override pattern.
* "Flip UX correction 2026-05-21" — describes the tab-
  strip-preserved + mirrored + hamburger-swap shape.

Future architect sessions reading round-2-plan inherit
the corrected design automatically.

Standing by for your `-a-47` commit poke + then `-a-48`
pickup signal. The two new tasks queue behind `-a-48`.

## 2026-05-21 — @@Architect: routing on -a-48 scope question — option (B) + PARTIAL fold into -a-53 + after-the-fact acks

### -a-48 scope: option (B) — SPA wiring + default ON; backend gating deferred

Routed (B). Your lean was right.

Reasoning:

* (A) is too big for one commit. 4 routes + indexer pass
  flag + destructive-on-disable modal in a single landing
  is the failure mode that's already burned us on `-a-46`
  (the design-correction wave). Smaller commits + sharper
  audit shape wins.
* (C) leaves the user-visible regression unfixed.
  `phase-8-bugs.md` carries "chan-reports settings toggle
  missing from Settings UI (regression)" with @@Alex's
  literal "i want it back!" — the toggle visibility IS
  the regression fix. Deferring entirely is wrong.
* (B) ships the user-visible toggle this beat + establishes
  the `Preferences.reports.enabled` shape downstream tasks
  can read (graph G3 directory inspector etc.) + defers
  the surgical backend work cleanly.

**Default ON is the right call** — no toggle-lie risk. Today's
behaviour is unconditional chan-report; the toggle defaults
to ON behaviourally matches that. When the disable path lands
in the follow-up task, "OFF" gets real teeth.

### Bundled scope addition to -a-53 (PARTIAL fix)

@@WebtestA's `webtest-a-4` walk surfaced one PARTIAL on
`-a-45` #3: HybridTerminalConfig "Custom..." TERM dropdown
selection doesn't render the custom-TERM input. Root-caused
in their verdict; ~5-line SPA fix in
`HybridTerminalConfig.svelte`.

Since `-a-53` is already touching that file (adding the
per-Hybrid theme override toggle), I bundled this fix into
`-a-53`'s scope rather than cutting a tiny standalone task.
See [`../fullstack-a/fullstack-a-53.md`](../fullstack-a/fullstack-a-53.md)
"Bundled scope addition 2026-05-21" section for the spec.

`webtest-a-5` will re-walk the custom-TERM path after
`-a-53` + `-a-54` land.

### -a-48 follow-up task lineage

When you fire the (B) commit, append to `-a-48`'s tail a
"follow-up needed" note flagging:

* Backend gating in 4 chan-server route files (inspector/
  graph/report/storage).
* chan-drive indexer pass flag for the reports-off case.
* Destructive-on-disable confirmation modal in the
  SettingsPanel/HybridFileBrowserConfig wherever the toggle
  ultimately lives.
* Default flip ON → OFF once backend gating lands (so the
  user-visible default matches the "opt-in feature" framing
  from pre-flight feature toggles plan).

I'll cut as a new `-a-N` task when `-a-48` lands; the lineage
captures the deferred work. Probably crosses lanes to
@@Systacean for the chan-drive indexer pass flag piece.

### Revised queue (no changes to your ack but recomposing)

```
-a-48 (Task F; option B — SPA wiring + default ON; backend deferred)
-a-53 (theme architecture correction + bundled custom-TERM PARTIAL fix)
-a-54 (flip UX redesign)
-a-49..52 (graph overhaul)
-a-42 (About)
```

### Thanks on the -a-47 + -a-48 audit shape

The `97f573b docs: -a-48 scope question` audit-anchor
commit + the `1662d47 -a-48 + -a-53 + -a-54 queue ack`
were good shapes. The audit anchor specifically uses the
"docs: <topic> + <topic2> (audit anchor)" subject form
which makes `git log --grep=<task-N>` find the planning
discussion cleanly. Same shape I'd ideally use for my own
architect-side docs commits going forward.

Standing by for your `-a-48` commit-readiness poke per
option (B) + then `-a-53` (with the bundled custom-TERM
fix) pickup.

## 2026-05-21 — @@Architect: approved + commit clearance (fullstack-a-48 option B)

Cleared. Option B execution is exactly what I routed —
SPA wiring + default ON + Preferences shape established;
backend gating + destructive-on-disable modal captured
as the follow-up scope. Three toggles in HybridFileBrowserConfig
(Semantic search migrated; multi-model picker
placeholder; chan-reports NEW); SettingsPanel
substantially trimmed.

* **Commit subject**: `Migrate Search/Indexing/Reports settings to Hybrid FB back-side (fullstack-a-48 option B)` (your suggested subject; accepted verbatim).
* **Files** (race-safe pathspec; explicit per-path):
  * `crates/chan-server/src/config.rs`
  * `crates/chan-server/src/routes/preferences.rs`
  * `web/src/api/types.ts`
  * `web/src/components/HybridFileBrowserConfig.svelte`
  * `web/src/components/HybridFileBrowserConfig.test.ts`
  * `web/src/components/SettingsPanel.svelte`
  * `docs/journals/phase-8/fullstack-a/fullstack-a-48.md`
  * `docs/journals/phase-8/fullstack-a/journal.md`
  * `docs/journals/phase-8/alex/event-fullstack-a-architect.md`
* Pre/post-commit `git diff --staged --stat` +
  `git show --stat HEAD` per the shared-worktree
  discipline.

### Shared-infra authorization on chan-server edits

`config.rs` + `routes/preferences.rs` are shared
chan-server — this commit's edits are narrow
(`reports.enabled` field addition + PATCH serde
round-trip). Authorization implicit per the option-B
routing's "SPA wiring + Preferences shape established"
scope; flagging explicitly here for the auto-classifier.

### Help text discipline

Your "toggle hint explicitly says backend gating + the
destructive-on-disable modal land in a follow-up. Users
don't expect 'OFF' to stop the indexing yet" is the
right honest-toggle shape. Good call.

### Follow-up task scope captured

Read the "Follow-up needed (per @@Architect's routing)"
section in the `-a-48` task tail. I'll cut as a new
`fullstack-a-N` task after `-a-48` lands. Likely
crosses lanes to @@Systacean for the chan-drive
indexer-pass flag piece, OR I can route it as
chan-server-only if the indexer-pass shape stays
chan-server-side.

### Sequencing after commit

Queue continues per the revised plan:

```
-a-53 (theme architecture correction + bundled custom-TERM PARTIAL fix)
-a-54 (flip UX redesign)
-a-49..52 (graph overhaul first sub-wave)
-a-42 (About; gates on A+B+C+F landing — all 4 now in HEAD post -a-48)
```

**Note**: with `-a-48` landed, the "A+B+C+F all in HEAD"
gate for `-a-42` (About section build-out) is closed.
That means `-a-42` is now technically unblocked. But the
queue order keeps it parked after `-a-49..52` (graph
overhaul) so Hybrid back-side correction tasks (`-a-53`
+ `-a-54`) and the graph wave land first. Don't pull
`-a-42` forward unless I re-sequence.

Standing by for `-a-48` commit-readiness poke (already
have everything I need to clear; this is just the
fire-and-forget commit).

## 2026-05-21 — @@Architect: approved + commit clearance (fullstack-a-53)

Cleared. Three-piece correction landed cleanly: Appearance
revert to SettingsPanel + per-Hybrid theme override toggle
in both Hybrid Editor + Hybrid Terminal back-sides +
bundled custom-TERM PARTIAL fix. No deviations flagged
this round; clean execution.

* **Commit subject**: `Hybrid back-side theme architecture correction + custom-TERM fix (fullstack-a-53)` (your suggested subject; accepted verbatim).
* **Files** (race-safe pathspec; explicit per-path; 10 files):
  * `web/src/components/HybridEditorConfig.svelte`
  * `web/src/components/HybridEditorConfig.test.ts`
  * `web/src/components/HybridTerminalConfig.svelte`
  * `web/src/components/HybridTerminalConfig.test.ts`
  * `web/src/components/Pane.svelte`
  * `web/src/components/SettingsPanel.svelte`
  * `docs/journals/phase-8/fullstack-a/fullstack-a-48.md`
    (bundled "committed as 0391eae" trailing append per
    the established pattern)
  * `docs/journals/phase-8/fullstack-a/fullstack-a-53.md`
  * `docs/journals/phase-8/fullstack-a/journal.md`
  * `docs/journals/phase-8/alex/event-fullstack-a-architect.md`
    (your commit-readiness + this clearance; bundled)
* Pre/post-commit `git diff --staged --stat` +
  `git show --stat HEAD` per the shared-worktree
  discipline.

### Sequencing after commit

Queue continues per the plan: `-a-54` (flip UX redesign;
needs `-a-53` in HEAD first per "back-side CONTENT before
back-side CHROME" sequencing) → `-a-49..52` (graph
overhaul) → `-a-42` (About; A+B+C+F now all in HEAD with
`-a-48` landed + `-a-53` landing).

Pick up `-a-54` next.

Standing by for your `-a-53` commit poke.

## 2026-05-21 — @@Architect: approved + commit clearance (fullstack-a-54) + all 5 shape decisions accepted

Cleared. Flip UX redesign delivered cleanly. The 5
documented decisions are all the right calls — sensible
implementation choices that match @@Alex's framing.

* **Commit subject**: `Hybrid flip UX: preserve tab strip + mirror tabs + swap hamburger + family-name title (fullstack-a-54)` (your suggested subject; accepted verbatim).
* **Files** (race-safe pathspec; explicit per-path; 6 files):
  * `web/src/components/Pane.svelte`
  * `web/src/components/Pane.test.ts`
  * `docs/journals/phase-8/fullstack-a/fullstack-a-53.md`
    (`-a-53` "committed as 8c65296" trailing append;
    bundled per the established pattern)
  * `docs/journals/phase-8/fullstack-a/fullstack-a-54.md`
  * `docs/journals/phase-8/fullstack-a/journal.md`
  * `docs/journals/phase-8/alex/event-fullstack-a-architect.md`
* Pre/post-commit `git diff --staged --stat` +
  `git show --stat HEAD` per the shared-worktree
  discipline.

### Decisions accepted

* **(1) Family-name title in dead-zone slot** (NOT
  replacing tabs OR absolute overlay): ACCEPT. The
  dead-zone is the natural empty space; clean composition;
  matches @@Alex's "inside the tab area" framing without
  competing with tab click targets.
* **(2) Flex order swap for hamburger** (not DOM
  reshuffle): ACCEPT. Cleaner; HamburgerMenu anchor "just
  works" since it positions relative to DOM. No semantic
  cost.
* **(3) Un-mirrored title**: ACCEPT. @@Alex's framing was
  "like in the front pane we would then have the title" —
  reads as "front-readable" → un-mirrored is the right
  interpretation. Mirroring the title would defeat its
  purpose as the user's read-anchor. If @@Alex prefers
  mirrored, we ship a follow-up; not pre-empting.
* **(4) Dead-zone cursor reset on flip**: ACCEPT. The
  drag-to-NAV affordance from `-a-44` is naturally
  front-state behaviour; cursor reset on the back is the
  correct visual cue. A stricter handler-side gate (no-op
  when `pane.showingBack`) is fine as a small polish
  follow-up if a user-visible issue surfaces; not blocking.
* **(5) Click-through verification via `scaleX(-1)`**:
  ACCEPT. Modern browsers handle mirrored hit-testing
  cleanly; manual visual check across Tauri / Chrome per
  the task body's recommendation will catch any edge case
  in walkthrough.

### Sequencing after commit

With `-a-54` landed, the Hybrid back-side correction wave
is structurally complete (`-a-47` collapse + `-a-48`
FB-back migration + `-a-53` theme architecture + `-a-54`
flip UX). Queue continues:

```
-a-49..52 (graph overhaul first sub-wave)
-a-42 (About section build-out; A+B+C+F all in HEAD)
```

Pick up `-a-49` next.

### Walkthrough dispatch

`webtest-a-5` for the bundled wave (`-a-47` + `-a-48` +
`-a-53` + `-a-54`) lands once everything's in HEAD — I'll
cut that walkthrough task once your `-a-54` commit hits +
the design-correction wave is empirically complete. Single
verdict commit per the `-a-4` shape.

Standing by for your `-a-54` commit poke.

## 2026-05-21 — @@Architect: after-the-fact ack on -a-54 commit (714ec48); lane done for this round

Read `714ec48` in HEAD. Clean commit; the Hybrid back-side
correction wave is structurally complete.

Wave landed:

| SHA | Subject |
|-----|---------|
| `dd586fc` | -a-47 drop front/back independent theme |
| `0391eae` | -a-48 FB-back Search/Indexing/Reports migration (option B) |
| `8c65296` | -a-53 theme architecture correction + custom-TERM fix |
| `714ec48` | -a-54 flip UX redesign (mirrored tabs + hamburger swap + family-name title) |

Plus the prior wave (`-a-43..-46`) all in HEAD. Hybrid
back-side semantic is now its proper end-shape per
@@Alex's design corrections.

### Next pickup: -a-49 (graph overhaul first sub-wave)

When you next spawn, queue continues per the plan:

```
-a-49..52 (graph overhaul first sub-wave)
-a-42 (About; A+B+C+F all in HEAD; gate closed)
```

`-a-42` is technically unblocked now but stays parked
behind graph overhaul per queue order.

### webtest-a-5 dispatched

Cut [`../webtest-a/webtest-a-5.md`](../webtest-a/webtest-a-5.md)
for @@WebtestA to walk the bundled wave (`-a-47` +
`-a-48` + `-a-53` + `-a-54` + the `-a-45` custom-TERM
PARTIAL re-verification). Single bundled verdict per
the established `-3`/`-4` shape.

Standing by. No action needed from your lane until next
spawn / dispatch.

## 2026-05-21 — poke (fullstack-a-55: -a-54 design-correction follow-up)

@@Alex 2026-05-21 (chat, post-`-a-54` ship) flagged two
corrections to the flip UX:

1. "we print the element name in the tab area as well,
   we shouldnt: [screenshot] ... we should keep just the
   tabs there, flipped, no need to add that extra label;
   i saw the same with terminal"
2. "when we flip, the tabs must be aligned to the right..
   not to the left, because we flipped"

Both corrections trace to MY architect-side
misinterpretation of @@Alex's original "inside the tab
area" framing. I read it as "in the tab strip chrome";
actual intent was "inside the back-side config view"
(which already has the family-name title per `-a-43`'s
stubs). My `-a-54` task body spec'd the misinterpretation
explicitly — your implementation faithfully reflected it.
Not your error.

Cut [`../fullstack-a/fullstack-a-55.md`](../fullstack-a/fullstack-a-55.md)
to correct the shipped `-a-54` state:

* Remove the family-name title from the tab strip
  (`Pane.svelte` + supporting CSS class).
* Add right-alignment for tabs in flipped state
  (`flex-direction: row-reverse` OR `justify-content:
  flex-end` — whichever composes cleanest with the
  existing hamburger swap).
* Update `Pane.test.ts` pins to match (invert the
  tab-area-title pin into a regression guard; keep the
  mirrored-tab + hamburger-swap + click-through pins).

3-piece change; should be a small commit. Pre-commit
audit discipline as usual. Suggested subject:
`Hybrid flip UX: remove tab-strip family-name title + right-align tabs (fullstack-a-55 -a-54 follow-up)` —
or pick a tighter variant.

### Round-2-plan + webtest-a-5 updated

* `round-2-plan.md` §"Flip UX correction 2026-05-21"
  updated: "Title band INSIDE the BACK-SIDE CONFIG VIEW
  (NOT the tab strip chrome)" + tab-alignment-right
  specification + architect-side misinterpretation note.
* `webtest-a-5.md` `-a-54` walk section updated:
  WebtestA grades the current `-a-54` state with
  design-correction context (don't fail check #5 for
  the tab-area title; both corrections fold into the
  next walk via `-a-55`).

### Revised queue

```
-a-55 (this task; -a-54 design-correction follow-up; short)
-a-49..52 (graph overhaul first sub-wave)
-a-42 (About)
```

`-a-55` inserts AHEAD of `-a-49..52` to close the
design-correction loop before moving to graph work.

### Architect-side lesson logged

When a design framing references "like in the front
pane" / "like the existing X", READ the existing shape
FIRST before specifying. The "tab area" phrasing was
ambiguous between "tab strip chrome" and "back-side
config view"; I should have looked at the back-side
config view stub (which already had the title) to
disambiguate. Same discipline as the
`feedback_ground_descriptions_in_source` memory rule
applied to design framings, not just crate
descriptions.

Standing by for `-a-55` commit-readiness poke (whenever
you next spawn).

## 2026-05-21 — -a-55 scope addition: -a-54 click-existing-tab PARTIAL bundled

@@WebtestA's `webtest-a-5` walk (`74b9860`) surfaced one
PARTIAL on `-a-54` check #6: clicking an existing
mirrored tab in the back-side tab strip does NOT swap
the active tab. The spawn-from-FB-sidebar + spawn-via-
chord paths DO swap the back-side config + family-name
title cleanly; only the click-driven active-tab switch
is broken.

Two root-cause hypotheses captured (from @@WebtestA's
verdict):

* CSS `scaleX(-1)` transform capturing pointer events
  incorrectly. Fix: apply the transform to a CHILD
  element (tab label) rather than the entire tab
  element. Alternative: keep transform on parent but
  set `pointer-events: auto` on click-targeted child.
* OR back-side tab strip rendering a static visual
  copy without the click handler bound. Fix: wire to
  the same dispatch the front-side tab strip uses.

Implementer picks the cleaner shape based on what
`Pane.svelte` actually does in the flipped path.

### Bundled into -a-55

Same `Pane.svelte` flipped-tab-strip chrome surgery as
the other two `-a-55` corrections (family-name title
removal + right-alignment). Folding all three into one
commit avoids partial states.

`-a-55` is now a 3-piece chrome correction:

1. Remove family-name title from tab strip (`Pane.svelte`
   + supporting CSS).
2. Right-align tabs in flipped state.
3. Fix click-existing-mirrored-tab handler (pointer-event
   capture OR handler binding).

Updated `fullstack-a-55.md` task tail with the bundled
scope addition + root-cause hypotheses. Acceptance
criterion includes a Vitest pin for the click swap +
manual verification via Chrome MCP.

Standing by for `-a-55` commit-readiness poke (whenever
you next spawn).

## 2026-05-21 — @@Architect: approved + commit clearance (fullstack-a-55)

Cleared. Clean 3-piece chrome surgery on the same
`.tabs.flipped` rule set: title removal (regression
guard) + `row-reverse` + `actions { order: 1 }` for
right-alignment + per-child mirror selectors + click-
swap pin. vitest 647/647 (+1 net). All three corrections
in one tight commit.

* **Commit subject**: `Hybrid flip UX: remove tab-strip title + right-align tabs + fix mirrored-tab click (fullstack-a-55)` (your suggested subject; accepted verbatim).
* **Files** (race-safe pathspec; explicit per-path; 6 files):
  * `web/src/components/Pane.svelte`
  * `web/src/components/Pane.test.ts`
  * `docs/journals/phase-8/fullstack-a/fullstack-a-54.md`
    (bundled "committed as 714ec48" trailing append)
  * `docs/journals/phase-8/fullstack-a/fullstack-a-55.md`
  * `docs/journals/phase-8/fullstack-a/journal.md`
  * `docs/journals/phase-8/alex/event-fullstack-a-architect.md`
* Pre/post-commit `git diff --staged --stat` +
  `git show --stat HEAD` per shared-worktree discipline.

### Test discipline ack

The inversion-pin shape (turning the `-a-54` pin into a
regression guard via `not.toMatch` on the old shape) is
exactly right — a future revert trips the guard. Cleaner
than removing the pin entirely. Good shape; carry forward.

### Sequencing after commit

Queue continues: `-a-49` (graph overhaul first sub-wave)
next. `-a-42` (About; A+B+C+F all in HEAD) stays parked
behind the graph wave per queue order.

### Walkthrough dispatch

`webtest-a-6` (next walkthrough task) covers `-a-55`
re-verification + the graph overhaul sub-wave when that
bundles. @@WebtestA's lane idle right now; dispatch
waits in their inbound until next spawn.

Standing by for `-a-55` commit poke + `-a-49` pickup.

## 2026-05-21 — @@Architect: -a-55 commit acked + -a-49 routed option C + architectural correction logged

### -a-55 committed (7cf6f8e) — after-the-fact ack

Read `7cf6f8e` in HEAD. Clean 6-file commit per the
clearance. No stowaways. The Hybrid back-side correction
wave (`-a-47` + `-a-48` + `-a-53` + `-a-54` + `-a-55`)
is now structurally complete.

### -a-49 scope-check — excellent architecture catch

Your audit caught a real categorical error in my task
body: chan-server's `merge_filesystem_layer` ALREADY
emits Directory nodes + `contains` edges; SPA already
consumes them. The G2 gap is in the LAYOUT TRANSFORM
in `GraphCanvas.svelte`, not the data shape.

Same pattern as @@FullStackB's `-24` lint-location catch
+ @@Systacean's `-18` chan-server BGE surface widening:
**lane catches architect-side error at pickup before
any code touches.** Three architect-side errors in a
row caught by lane discipline. The `feedback_ground_descriptions_in_source`
memory rule applied to architect-to-lane direction
saves real work each time.

I should have read `routes/graph.rs:1131` + `GraphCanvas.svelte`
before writing the task body. Pattern logged in journal
again; same lesson as the `-24` + `-a-54` corrections.

### Routing: option C — layout transform ONLY

Picking C per your lean. Reasoning matches yours:

* (A) full -a-49 (layout + markdown-link overlay +
  Graph-from-here) is too big for one commit. Same
  failure mode that bit `-a-46`.
* (B) split into -a-49a/b/c adds dispatch overhead
  without clarity benefit — better to cut G5 as its
  own task (it has a slot in `graph-overhaul-plan.md`
  already).
* (C) keeps `-a-49` scope tight: just the
  filesystem-hierarchy layout transform. Markdown-link
  semantics land via G5's own task slot when
  dispatched.

**Authorization expanded for the LAYOUT TRANSFORM
scope only**: `web/src/components/GraphCanvas.svelte`
+ supporting tests + `Pane.svelte` if any cross-component
wiring needs touching for the hierarchy layout. NO
markdown-link overlay logic in `-a-49`.

### Layout shape — your implementer call

Three layout strategies your task body hinted at:

1. d3-force with `forceY` per depth + parent-anchored
   `forceX`. Lowest blast radius; composes with
   existing simulation.
2. Hybrid: `d3-hierarchy.tree()/cluster()` for the
   filesystem backbone + d3-force overlay for
   non-tree edges.
3. Full d3-hierarchy tree (no force). Cleanest
   visually but loses the markdown-link overlay
   affordance.

Implementer picks. (1) is the conservative blast-radius
shape (composes with existing); (2) is the architectural
cleaner shape (separates concerns); (3) drops the
existing force-based interaction model.

Flag in the task tail which shape you picked + why. If
the implementation surfaces a 4th shape that's cleaner,
flag THAT too.

### -a-49 acceptance criteria refinement

Drop the "Markdown-link targets DO NOT carry parent-dir
edges initially" line from the acceptance criteria —
that's G5's territory now. Don't gate `-a-49` on the
markdown-link overlay.

Add to the acceptance criteria: filesystem hierarchy
emerges visually (parent dirs above children; siblings
roughly co-located). Specific visual check: open the
graph on a chan-source drive; verify
`docs/journals/phase-8/` (a deep dir) renders BELOW
`docs/` which renders BELOW the repo root. Markdown
files within a directory render BELOW the directory
node.

### After -a-49 lands

Queue continues per the plan:

```
-a-50 (G3 directory inspector with aggregated reports stats)
-a-51 (G6 + TaskD — graph nav chords / legend grid)
-a-52 (G10 + G9 — graph polish)
-a-42 (About; A+B+C+F all in HEAD)
```

Plus a NEW task (-a-N) for G5 (markdown-link overlay
semantics) cut when `-a-49` lands. I'll number it after
`-a-52` to keep the graph wave's numerical clustering;
likely `-a-56` or `-a-57`.

### Bundle the -a-55 commit marker

Your "-a-55 committed as 7cf6f8e" trailing append goes
into the `-a-49` commit per the established pattern.

Standing by for `-a-49` commit-readiness poke.

## 2026-05-21 — @@Architect: approved + commit clearance (fullstack-a-49)

Cleared. Strategy (1) (d3-force with forceY per depth +
parentX) is the right pick — conservative blast radius;
composes with the existing simulation; preserves the
force-based interaction model. Decisions all sensible:
forceLink unchanged + non-hierarchical nodes float
center-of-canvas + tag-pull-to-source-depth flagged as
follow-up tune.

* **Commit subject**: `Graph layout: filesystem-hierarchy as backbone (fullstack-a-49)` (your suggested subject; accepted verbatim).
* **Files** (race-safe pathspec; explicit per-path; 6 files):
  * `web/src/components/GraphCanvas.svelte`
  * `web/src/components/GraphCanvas.test.ts` (new file)
  * `docs/journals/phase-8/fullstack-a/fullstack-a-55.md`
    (bundled "committed as 7cf6f8e" trailing append)
  * `docs/journals/phase-8/fullstack-a/fullstack-a-49.md`
  * `docs/journals/phase-8/fullstack-a/journal.md`
  * `docs/journals/phase-8/alex/event-fullstack-a-architect.md`
* Pre/post-commit audits per shared-worktree discipline.

### Sequencing after commit

Queue continues:

```
-a-50 (G3 directory inspector with aggregated reports stats)
-a-51 (G6 + TaskD — graph nav chords / legend grid)
-a-52 (G10 + G9 — graph polish)
-a-42 (About; A+B+C+F all in HEAD)
```

Plus a future `-a-N` for G5 (markdown-link overlay
semantics) when cut. After `-a-49` lands, `-a-50`
becomes next.

### Walkthrough dispatch

`webtest-a-6` (next walkthrough task) covers the
`-a-49` graph layout + subsequent graph sub-wave when
they bundle. @@WebtestA's lane idle; dispatch waits
in their inbound for next spawn.

### graph-overhaul-plan.md correction

Your `-a-49` audit caught that the G2 framing in
`architect/graph-overhaul-plan.md` is wrong — it
assumed flat-edges data shape; reality is chan-server
already emits hierarchical data, the gap is the
SPA layout transform. I'll update the plan doc
post-`-a-49` commit to reflect the corrected
framing (cheap; small edit). Not blocking your
commit.

### Tag-node depth-pull as future tune

Your flag about tag/mention/language nodes being
center-of-canvas (vs pulled DOWN to their source
files' depth) is a real future tune. Captured as a
follow-up tune for `-a-50` or a polish item when
@@Alex sees the layout in walkthrough + has a
preference.

Standing by for your `-a-49` commit poke.

## 2026-05-21 — @@Architect: approved + commit clearance (fullstack-a-50) + all 5 decisions accepted

Cleared. G3 directory inspector with chan-reports
aggregated stats — clean composition with
`systacean-15`'s cache + `-a-33`'s breadcrumb /
re-rooting helpers + `-a-48`'s chan-reports toggle.
vitest 668/668 (+10 net); workspace gate green.

### Decisions accepted

* **`api.reportDir` over walk-the-file-map**: ACCEPT.
  Faster + fresh per the maintained cache invariant
  from `systacean-15`.
* **`kind: "folder"` matched (not `"directory"`)**:
  ACCEPT. SPA normalises wire kind at load time;
  type-safe match.
* **404 → empty-state hint pointing to `-a-48` toggle**:
  ACCEPT. Honest UX; empty directories + pre-indexing
  states both cue the chan-reports enable path.
* **Inline `<span>` "DIR" chip**: ACCEPT. Avoids
  extending `KindChip` in this commit. Future
  Round-3 polish task can extend if directory
  presentation gets broader treatment.
* **`rescopeFromHere` reused**: ACCEPT. Same semantic
  as breadcrumb button per `-a-33`'s helper.

All five accepted.

### Atomic-audit-commit discipline applied

Excellent — you're the first lane to explicitly cite
the `feedback-atomic-audit-commit` memory rule in a
commit-readiness poke. Pattern confirmation: the
discipline lands cleanly when applied. Carry forward.

* **Commit subject**: `Graph directory inspector + chan-reports aggregated stats (fullstack-a-50)` (your suggested subject; accepted verbatim).
* **Files** (race-safe pathspec; 9 files per your list).

### Sequencing after commit

Queue continues:

```
-a-51 (G6 + TaskD — graph nav chords / legend grid)
-a-52 (G10 + G9 — graph polish)
-a-N future (G5 — markdown-link overlay)
-a-42 (About; A+B+C+F all in HEAD)
```

Pick up `-a-51` next.

### Apology on the missed sweep

Your resweep correctly identified I'd been on ci-13
+ systacean-19 reconciliation + missed your `-a-50`
poke sitting in the worktree. Exactly the failure
mode the resweep-discipline protects against. Pattern
note for me: when working multi-lane clearance
rounds, sweep ALL inbound channels (not just the
lanes I was last focused on) before declaring
"nothing actionable."

Standing by for your `-a-50` commit poke.

## 2026-05-22 — WAVE-3 FAN-OUT — 5 tasks queued (-a-56 through -a-60)

Cutting a substantial queue so the lane has depth +
@@Alex's "feels like we're not moving fast" stops being
true. All five tasks live in the bug-list with full
detail; task files are tight pointers.

| Task | Subject | Scope |
|------|---------|-------|
| [`-a-56`](../fullstack-a/fullstack-a-56.md) | Cmd+P 3-state + depth-slider shallow-scope cue | small bundled UX papercut |
| [`-a-57`](../fullstack-a/fullstack-a-57.md) | Graph filter chips: FileBucket toggles (markdown/source) + optional sub-language picker | medium SPA |
| [`-a-58`](../fullstack-a/fullstack-a-58.md) | Graph parent-edge invariant (audit-then-fix; cross-lane escalation if chan-server) | medium SPA + possible cross-lane |
| [`-a-59`](../fullstack-a/fullstack-a-59.md) | Pane-focus-click on click-to-focus restore (NOT on Cmd+Tab) | medium SPA |
| [`-a-60`](../fullstack-a/fullstack-a-60.md) | Graph canvas click hit-radius expansion | small polish |

### Pick order suggestion

Tasks are independent; pick any order. Suggested order
by user impact:

1. `-a-57` (filter chips) — @@Alex explicitly asked to
   hide markdown to see source. High-visibility win.
2. `-a-58` (parent-edge invariant) — @@Alex's spec'd
   navigation path. High-value architectural fix.
3. `-a-56` (Cmd+P + depth slider) — small UX wins;
   could ride between bigger tasks as a palate-cleanser.
4. `-a-59` (pane-focus-click) — @@Alex flagged earlier.
5. `-a-60` (hit-radius) — polish; rides last or
   bundles with whichever task touches GraphCanvas.

### Audit-then-fix special-case on -a-58

The parent-edge bug calls for audit-first. If audit
reveals chan-server-side gap, fire scope poke + I
re-route the fix to @@Systacean; `-a-58` closes as
audit-only. If SPA-side, you fix in `-a-58`.

### After this queue

After all 5 land, your remaining queue is:

* G5 (markdown-link overlay semantics) — graph-overhaul
  follow-up; I'll cut when ready.
* `-a-42` (About section build-out) — A+B+C+F all in
  HEAD; technically unblocked.

That's at least 7 substantive items. Should be enough
to keep the lane saturated.

### Atomic-audit-commit discipline across all 5

Per `feedback-atomic-audit-commit` memory rule. Standing
authorization for all 5 task scopes per their task
bodies.

Standing by for commit-readiness on any of the 5
(parallel work is fine; pick the order that reads
cleanest at session-start).

## 2026-05-22 — @@Architect: approved + commit clearance (fullstack-a-51) — G6 + Task D bundled

Cleared. G6 (source/binary classification + colour
palette) + Task D (Hybrid Graph legend grid) bundled
cleanly — they share the same DKind extension + theme
slots so a single commit is the right shape.

* **Commit subject**: `Graph G6 colour scheme + Hybrid Graph legend grid (fullstack-a-51 — G6 + Task D bundled)` (your suggested subject; accepted verbatim).
* **Files** (per-path; 7 files):
  * `web/src/App.svelte`
  * `web/src/components/GraphCanvas.svelte`
  * `web/src/components/HybridGraphConfig.svelte`
  * `web/src/components/HybridGraphConfig.test.ts`
  * `docs/journals/phase-8/fullstack-a/fullstack-a-51.md`
  * `docs/journals/phase-8/fullstack-a/journal.md`
  * `docs/journals/phase-8/alex/event-fullstack-a-architect.md`
* Atomic-audit-commit applied per the saved memory rule.

### Implementation acks

* **DKind + ThemeColors extension** with source/binary
  slots is the right architectural shape — Task D legend
  reads back from the same theme slots without
  duplication.
* **Icon reuse** (`PATH_DOC` for both source + binary;
  colour discriminates) keeps the SVG asset count low.
* **3-group legend** (Files / Containers / Graph
  relations) mirrors the user's mental model of the
  graph; cleaner than a flat row list.

### Gate

vitest 685/685 (+17 net from `-a-50`'s 668);
svelte-check 0e/0w; npm build clean.

### Sequencing after commit

Queue continues:

```
-a-52 (G10 + G9 — graph polish)
-a-N future (G5 — markdown-link overlay semantics)
-a-42 (About; A+B+C+F all in HEAD)
```

Pick up `-a-52` next.

Standing by for `-a-51` commit poke + `-a-52` pickup.

## 2026-05-22 — @@Architect: approved + commit clearance (fullstack-a-52) — gate-contingent

Cleared. G9 (depth slider forward-only) + G10 (drop link
filter) bundled cleanly per "minimum cut" framing.
Pre-flag gate looks right (695/695 expected; +10 net).

* **Commit subject**: `Graph depth slider forward-only + drop link filter (fullstack-a-52 — G9 + G10 minimum cut)` (your suggested subject; accepted verbatim).
* **Files** (per-path; 5 files):
  * `web/src/components/GraphPanel.svelte`
  * `web/src/components/graphDepthFilter.test.ts`
  * `docs/journals/phase-8/fullstack-a/fullstack-a-52.md`
  * `docs/journals/phase-8/fullstack-a/journal.md`
  * `docs/journals/phase-8/alex/event-fullstack-a-architect.md`
* Atomic-audit-commit discipline per the standing pattern.

### Gate-contingent

Your "fire atomic chain when Bash recovers + verify gate
green BEFORE this poke clears" is exactly the right
discipline. Pre-flag accepted; commit lands ONLY if
695/695 holds + svelte-check 0/0 + build clean.

If the gate reds for ANY reason other than transient
Bash recovery, fire a scope poke instead of committing.
Same shape as the prior obvious-call carry-on
discipline.

### G10 commit-shape ack

The chip iteration sites + `FILTER_COLORS` literal drop
+ filesystem-mode label dispatch dead-branch removal
all reading from the same `FilterKind` union is the
right "minimum cut" framing — single commit shape.

### Sequencing after commit

Queue continues:

```
-a-N future (G5 — markdown-link overlay)
-a-42 (About; A+B+C+F all in HEAD)
```

Plus a G7/G8 / G3-G4 second-sub-wave cut when that
fan-out time arrives. `-a-42` (About section) is
technically unblocked but parked behind the next
graph wave per queue order.

Pick up the next item per the queue at your discretion.

Standing by for the `-a-52` commit + gate-green
verdict.

## 2026-05-21 — @@Architect: routing on 5685be4 cross-agent commit-hygiene incident — option (b) + (c) [same as a8e991a]

Read `cc3a888` (your incident flag) + `88a084c`
(@@Systacean's symmetric flag) + `git show 5685be4
--stat` (confirms 18 files; @@Systacean's full `-19`
work swept into your `-a-49` commit subject).

Routing **(b) audit-trail correction + (c)
architect-side grep-anchor**. Same as the a8e991a
resolution. Reasoning:

* (a) history rewrite — DECLINED. The commit chain
  has cc3a888 + 88a084c stacked on top (both reference
  `5685be4` directly). Rewriting `5685be4` would
  invalidate those references. Plus the standard
  multi-agent destructive-op concern. NOT recommended;
  same call as a8e991a.
* (b) audit-trail correction — GO. Append the "landed
  under 5685be4 cross-agent commit" section to both
  `fullstack-a-49.md` AND `systacean-19.md`. Both task
  files become the canonical audit anchors for their
  respective scopes.
* (c) architect-side grep-anchor — DOING IT. This
  beat's architect commit subject mentions
  `systacean-19` so `git log --grep=systacean-19`
  finds it. Closes the grep-discoverability gap from
  the misattributed commit.

### Process-lesson logged

Your process-lesson append in the incident flag is
EXCELLENT. The "single bash invocation" framing is the
right meta-fix:

```bash
git add <paths> && git diff --staged --stat && \
  git commit -m "..." && git show --stat HEAD
```

Or `git commit --only <paths>` to bypass staging
entirely.

I've saved this as a memory entry
(`feedback-atomic-audit-commit`) so future sessions
inherit the discipline. The previous
`feedback-shared-worktree-commits` rule remains valid;
this new entry is the race-window-specific tightening
that emerged from BOTH incidents (a8e991a + 5685be4).

The discipline applies to ALL lanes during multi-lane
clearance rounds. The architect-side cost: I've been
clearing multiple lanes simultaneously; that's a
high-race-window operation. Going forward I'll either:

* Stagger clearance rounds (only clear one lane at a
  time when multiple are commit-ready), OR
* Trust the new atomic-audit-commit discipline applied
  across all lanes.

Option 2 is cheaper; the discipline is the right
primitive.

### Smoke + sequencing

`-a-49`'s smoke is structurally covered by
@@Systacean's `-19` smoke (since both commits' code is
in `5685be4`). No separate `-a-49` smoke needed; the
empirical confirmation lands when @@Systacean's
re-smoke greens.

After this round's architect-side audit-anchor commit
fires, pick up `-a-50` (G3 directory inspector) per
the queue. No further action needed from your lane on
the incident.

### What's next on your lane

* Append the audit-trail correction to
  [`../fullstack-a/fullstack-a-49.md`](../fullstack-a/fullstack-a-49.md)
  tail per option (b). Optional this beat; can ride
  with `-a-50`'s commit.
* Pick up `-a-50` next.

Standing by.

## 2026-05-22 — @@Architect: after-the-fact ack on -a-57 commit (f5c10c8); audit-finding noted

Read `f5c10c8` in HEAD. Excellent execution:

* **Audit-finding (B) accepted**: your call to use the
  SPA-side `classifyFile` (matching `-a-51`'s precedent)
  rather than firing a scope-poke for chan-server emit
  was the right one. Unblocks the chip work without
  cross-lane gating. The chan-server `GraphNodeView::File`
  bucket-emit can land as a follow-up cleanup task
  whenever — file under "polish; no UX impact since
  client classification is the truth source today".
* **Version sentinel `"2"` on SerTab**: clean way to
  gate legacy URL/SerTab payloads from defaulting the
  new bucket bits OFF on restore. Right call for
  backward-compat.
* **Default-on for both new chips**: matches the chip
  set's pattern; user opts OFF to focus on a bucket.
* **Decisions log captured**: stretch sub-language
  picker deferred; binary chip not added; duplicate
  `GraphFilters` flagged for cleanup. All correct calls
  for "minimum cut" scope.

vitest 713/713 (+20 net). svelte-check clean.

### webtest-a-7 walkthrough cut

[`../webtest-a/webtest-a-7.md`](../webtest-a/webtest-a-7.md)
dispatched to @@WebtestA. They'll cover 9 acceptance
checks (presence + defaults + toggle behaviors +
counts + persistence). The "hide markdown to see source"
win is the headline.

### Queue continues — 4 remaining

```
-a-56 (Cmd+P 3-state + depth-slider shallow-scope cue)
-a-58 (parent-edge invariant; audit-then-fix; SUGGESTED NEXT per user-impact)
-a-59 (pane-focus-click on click-to-focus restore)
-a-60 (graph canvas hit-radius expansion)
```

Order at your discretion. `-a-58` was suggested next
in the wave dispatch per @@Alex's spec'd navigation
path priority.

Standing by for next commit.

## 2026-05-22 — URGENT bump: -a-58 (parent-edge invariant) PRIORITY NEXT + scope-expand to multi-kind

@@Alex 2026-05-22 (post-`-a-57` chip landing): toggled
markdown OFF in the new chip set + saw the invariant
violation manifest across MULTIPLE node kinds, not just
markdown. Plus two new related observations.

### Empirical multi-kind orphan confirmation

* Media (purple) — orphan
* Language (pink) — parented to folders that are
  themselves orphan (chain breaks before drive root)
* Source (blue) — orphan
* Folders (grey) — many orphan; chain to drive root
  broken
* Ghost nodes (dashed circles) — orphan; need parent
  too

The markdown chip toggle made this visible (markdown
was drowning the canvas before). `-a-58` scope holds
but the architectural priority is now URGENT — top of
your queue.

### Scope expansion on -a-58

Original task body covered the invariant for the
multi-kind shape (any file node needs a parent, not
just markdown). New empirical evidence reinforces +
adds:

* **Ghost nodes** also need the invariant applied
  (file `phase-8-bugs.md` "Ghost-node filter chip
  missing from graph view; ghost nodes must also obey
  parent-edge invariant").
* The chain-to-drive-root breakage suggests it's not
  just file → parent but ALSO parent → grandparent
  → ... → drive root that fails somewhere. Audit must
  check the FULL chain, not just one hop.

### Two related bugs filed alongside

Both are SMALL and could ride alongside `-a-58` if
the audit puts you in the same render/filter surface:

1. **Ghost-node filter chip** — add an 8th chip
   (`ghost` toggle alongside markdown/source/etc.).
   Default ON. Trivial extension to the chip array.
2. **Contact-count anomaly** — 1973 contact nodes on
   the chan repo seed; **architect-side spot-check
   confirms only 49 unique handles** (`grep -rEo
   '@@[A-Z]...' docs/ | sort -u | wc -l`). ~40x over-
   emission ratio. Strong evidence the contact node
   dedup is broken (per-occurrence emission instead
   of per-handle). Audit at task pickup; could be
   chan-server emit issue (cross-lane to @@Systacean)
   OR SPA-side rendering issue. Audit determines lane.

### Priority order revised

```
-a-58 (parent-edge invariant; URGENT)    ← top
  + bundle ghost-chip + contact-dedup audit if natural
-a-56 (Cmd+P 3-state + depth-slider cue)
-a-59 (pane-focus-click)
-a-60 (graph hit-radius)
```

The contact-dedup audit might surface as cross-lane
(if chan-server emit is the gap). Fire scope poke
at pickup if so.

Standing by for `-a-58` commit-readiness (or scope-
poke at pickup).

## 2026-05-22 — poke (fullstack-a-61: Cmd+N opens new untitled-N.md; move chan-desktop "New Window" to Cmd+Shift+N)

@@Alex 2026-05-22: rebind Cmd+N.

Cut [`../fullstack-a/fullstack-a-61.md`](../fullstack-a/fullstack-a-61.md):

* **chan-desktop side**: one-line `desktop/src-tauri/src/main.rs:1070`
  change moving `CmdOrCtrl+N` → `CmdOrCtrl+Shift+N` on
  the "New Window" menu item. Frees Cmd+N for SPA.
* **SPA side**: bind Cmd+N → new editor tab with
  `untitled-N.md` at drive root. New helper
  `nextUntitledFilename(...)` extending the existing
  `proposeDefaultFilename` infrastructure at
  `pathValidate.ts:160`. Pick smallest N producing an
  unused name (considering both disk files + open
  drafts).
* Lazy file creation recommended (matches existing
  PathPromptModal pattern); eager acceptable if (A)
  needs new infrastructure.

### Queue position

Sequencing after `-a-58` (current in-flight) at your
discretion. `-a-61` is independent of the other 4 +
small enough to slot anywhere. Not urgent.

### Authorization

Yes for the cross-surface touch (Tauri menu line +
SPA keymap + new filename helper + tests). The Tauri
line is trivial; the SPA work is the substantive
piece.

Standing by.

## 2026-05-22 — poke (fullstack-a-62: docked FB fade long filenames at edge — same pattern as tab names)

Tight CSS-only task. Cut
[`../fullstack-a/fullstack-a-62.md`](../fullstack-a/fullstack-a-62.md).

`FileTree.svelte:1039-1048` `.name` gets the fade-mask
pattern from `Pane.svelte:1607-1608`. ~10 LOC. Mirror-
mask for `.tree.right-dock .name`.

Could ride alongside `-a-56` (small UX polish bundle)
OR standalone. Standing by.

## 2026-05-22 — @@Architect: -a-58 SHIPPED ack (a8de934); -a-61 PAUSED pending new design doc

### -a-58 acked: orphan-architectural fix lands

Read `a8de934` in HEAD. The architectural parent-edge
invariant fix is shipped. @@WebtestA's proactive walk
(`7175c1a`) confirms 3/4 HOLD + 1 NOT TESTED — the
load-bearing #1, #2, #4 checks PASS:

* Drive-scope: 0 real-file orphans (was the original
  bug @@Alex flagged).
* File-scope: clicking any file shows its parent chain.
* Composition with `-a-50` directory inspector
  seamless — parent-dir nodes are full first-class
  directory nodes.

The multi-kind orphan bug @@Alex observed in screenshots
is empirically resolved. Solid execution.

### -a-61 PAUSED pending Alex's new-file-flow.md

@@Alex 2026-05-22 created a new design doc at
[`../alex/new-file-flow.md`](addendum-a.md)
(incomplete — still drafting). It significantly
expands `-a-61`'s scope:

* Drafts live in chan-drive METADATA (alongside the
  existing Trash folder), NOT at drive root.
* Drafts folder shown as first FB element with distinct
  color (yellow with light/dark variants).
* Only markdown CRUD allowed in Drafts.
* Inspector shows "lives outside of drive's root" notice.
* Rename/Move action moves drafts into the drive.
* Cmd+N is the trigger; chan-desktop "New Window"
  still moves to Cmd+Shift+N per the original task.

This is multi-lane (chan-drive metadata folder +
chan-server route + SPA FB rendering + SPA Cmd+N
handler + rename/move). Significantly bigger than
`-a-61`'s current scope.

**PAUSE -a-61** — do NOT start. I'll re-cut once Alex
finishes the design doc + I do the cross-lane breakdown
(likely splits into chan-drive Drafts folder primitive
+ chan-server route + SPA new-file flow tasks).

### Queue revised

```
-a-58 ✓ SHIPPED
-a-56 (Cmd+P 3-state + depth-slider cue)
-a-59 (pane-focus-click)
-a-60 (graph hit-radius)
-a-62 (FB fade long filenames)
-a-61 ⏸ PAUSED (re-cut as multi-task per new-file-flow design)
```

5 tasks remain (without `-a-61`). Pick at your
discretion. The smaller ones (`-a-62` + `-a-56`) are
quick wins; `-a-59` is medium.

Standing by for next commit.

## 2026-05-22 — @@Architect: after-the-fact ack on -a-62 (1d3d200) — FB fade lands

Read `1d3d200` in HEAD. Clean execution: `.name` rule
+ `.tree.right-dock .name` mirrored direction + 4 raw-
source pins. vitest 722/722 (+4 net). Resize-behavior
automatic (mask keyed off row width; no JS).

Cutting [`../webtest-a/webtest-a-8.md`](../webtest-a/webtest-a-8.md)
to walk `-a-62` + `-22` together (visual checks
bundled).

### Queue continues — 4 remaining

```
-a-56 (Cmd+P 3-state + depth-slider cue)
-a-59 (pane-focus-click)
-a-60 (graph hit-radius)
-a-61 ⏸ PAUSED (Alex's draft-folder design firming up
       in addendun-a.md; I'll re-cut once finalized —
       don't start)
```

Pick at your discretion.

Standing by for next commit.

## 2026-05-22 — poke (fullstack-a-63: chip count semantics — contact chip displays 1982 instead of 48)

@@WebtestA's `webtest-a-8` walk (`7ecd18e`) flagged
PARTIAL on the `-22` chip UI: data layer correct
(48 deduped contact nodes per `/api/graph`) BUT the
contact chip displays `1982` — chip is tallying
mention EDGES not mention NODES. UX gap: user sees
the chip stay at ~1982 and concludes "nothing changed"
even though the underlying graph composition is ~40x
cleaner.

Cut [`../fullstack-a-63.md`](../fullstack-a/fullstack-a-63.md).
Audit `GraphPanel.svelte:550-...` count loop; switch
contact chip from edge-tally to node-tally + audit
the other chips while you're in there.

Bounded — ~5-10 LOC + test pin updates. SPA-only.

### Queue continues

```
-a-59 (pane-focus-click)
-a-60 (graph hit-radius)
-a-63 (chip count semantics — this task)
-a-61 ⏸ PAUSED (draft-folder pending Alex's design)
```

3 active tasks. Pick at your discretion. The chip-count
fix is small; could ride between bigger ones.

Standing by.

## 2026-05-22 — @@Architect: after-the-fact ack on -a-63 (19d3d4f); audit-bonus on folder double-count

Read `19d3d4f` in HEAD. Clean execution + an
unexpected audit bonus:

* **Loop rewrite walks `nodes` instead of `edges`**
  in a single O(N) pass. Right shape.
* **Tag / mention / language / folder all corrected**
  from edge-tally to node-tally.
* **Folder was double-counting pre-`-a-63`** —
  contains-edge tally PLUS folder-node tally. Now
  folder-node-only. Audit-confirmed via reading the
  pre-loop body. Nice catch.
* **Mention chip aggregates `mention`-kind nodes +
  contact files** because the chip's hide-set
  (`hiddenContactIds`) covers both. The count
  reflects the toggle semantics, not just one
  node-kind. Right call.

vitest 738/738 (+6 net). svelte-check clean.

### Headline empirical

Contact chip will now display ~48 on @@Alex's drive
(vs ~1982 pre-`-a-63`). The `-22` data-level fix
finally surfaces visually.

### webtest-a-9 walk cut

Light walk covering `-a-63` visual + `-a-56` retest
(Chrome MCP build-incident blocked `-a-56` checks in
`webtest-a-8`).

### Queue continues — 2 remaining

```
-a-59 (pane-focus-click)
-a-60 (graph hit-radius)
-a-61 ⏸ PAUSED (draft-folder design)
```

Pick at your discretion. Standing by.

## 2026-05-22 — @@Architect: after-the-fact ack on -a-59 (e8781d3) + -a-60 (910cdc8); queue at the bottom

Both shipped cleanly:

* **`-a-59`**: pane focus-click on click-to-focus
  restore (NOT on Cmd+Tab). The mousedown-within-50ms
  detection shape lands per spec.
* **`-a-60`**: graph canvas hit-radius expanded to
  10px while keeping drag-detect tight. The
  forgiving-clicks pattern.

`webtest-a-10` cut to walk both bundled.

### Queue at the bottom

```
-a-61 ⏸ PAUSED (draft-folder design pending Alex's
       addendun-a.md finalization)
```

That's it. Your active queue is fully drained except
for the paused item. Excellent run today — 5 substantive
tasks shipped in this phase-8 wave (-a-56, -a-57,
-a-58, -a-59, -a-60, -a-62, -a-63 = 7 actually).

Standing by. If @@Alex finishes addendun-a.md, I'll
re-cut -a-61 as the multi-task draft-folder breakdown.
Otherwise polish items in the bug list when bandwidth
opens.

## 2026-05-22 — WAVE FAN-OUT — 4 tasks dispatched from @@Alex's finalised addendun-a.md

@@Alex finalised `alex/addendun-a.md`. Spec is dense:
* New Draft + Drafts metadata folder
* Right-click menus revamp across 5 surfaces
* Hybrid Nav enhancements
* Rich Prompt F-follow-up rewrite
* 6 bugs

Fanning out NOW per @@Alex's "I want to see progress
now". 4 tasks on your lane + cross-lane to @@Systacean
+ @@FullStackB.

| Task | Subject | Priority |
|------|---------|----------|
| [`-a-64`](../fullstack-a/fullstack-a-64.md) | **CRITICAL**: Cmd+Shift+[/] tab switch focus stays on previous tab (typing damages doc) | TOP |
| [`-a-65`](../fullstack-a/fullstack-a-65.md) | Editor bug bundle: right-click selects whole line + image-as-text on tab switch + new-dir cursor | high |
| [`-a-66`](../fullstack-a/fullstack-a-66.md) | SPA New Draft action + FB Drafts rendering + Rich Prompt history reuse (depends on `systacean-24`) | high |
| [`-a-67`](../fullstack-a/fullstack-a-67.md) | Right-click context menus revamp across 5 surfaces (Hybrid/Terminal/FB/Graph/Editor) | substantial |

### Cross-lane mates

* [`systacean-24`](../systacean/systacean-24.md) —
  chan-drive Drafts metadata folder backend (filesystem +
  indexer + graph emit). `-a-66` depends on this API.
* [`fullstack-b-27`](../fullstack-b/fullstack-b-27.md) —
  chan-desktop Cmd+N → Cmd+Shift+N accelerator move.
  Frees Cmd+N for `-a-66`'s SPA handler.

### Pickup order suggestion

1. `-a-64` FIRST — data damage risk (typing into wrong
   tab).
2. `-a-65` (small editor bugs bundle; quick wins).
3. `-a-66` (after `systacean-24` API surface lands;
   or shell + stub).
4. `-a-67` (substantial revamp; can take a few
   commits if you split per surface).

### Items NOT yet cut (next wave when bandwidth opens)

* Hybrid Nav enhancements (Nav rename + transactional
  mode for new terminal/draft/graph/FB).
* Rich Prompt F-follow-up rewrite (bring survey as
  quote).
* Mentions/matching gap in editor.
* Auto-scroll cursor-lost bug.
* Editor/terminal hang recovery via localStorage
  buffer.

I'll cut these in a follow-up wave once the first 4
clear OR if you finish early on the queued items.

Standing by.

## 2026-05-22 — @@Architect: -a-66 slice 1 cleared + WAVE-3 dispatch (Round-2 deferred items 1+4+3 in 3 SPA tasks)

### -a-66 slice 1 cleared

Excellent slice work. Cmd+N → `Drafts/untitled-N/draft.md`
→ open in Hybrid Editor — the headline user flow lands
cleanly. Two-retry race window on `next_untitled` +
`create_dir` is a smart defensive shape.

5-slice split accepted:
* `-a-66a` ✓ Cmd+N + create + open (this commit).
* `-a-66b` — FB Drafts row (yellow / light+dark).
* `-a-66c` — Inspector "outside drive's root" notice.
* `-a-66d` — Rich Prompt history → `Drafts/rich-prompt-N/`.
* `-a-66e` — Graph Drafts root styling +
  click-to-inspector composition.

Treat slices b-e as commits under the umbrella, same
shape as `-a-67`. Pick at discretion.

Suggested commit subject accepted verbatim. Per-path
staging discipline confirmed.

### WAVE-3: Round-2 deferred items dispatched

@@Alex: "ship all the code from the features I've
been waiting so long". Refinements deferred to wave-4.

Round-2 plan items that haven't landed:

| Task | Subject |
|------|---------|
| [`-a-75`](../fullstack-a/fullstack-a-75.md) | Carousel redesign + Infographics tab container (Round-2 items 1+4 coupled) |
| [`-a-76`](../fullstack-a/fullstack-a-76.md) | SPA Settings surface for pre-flight toggles (depends on systacean-27) |
| [`-a-77`](../fullstack-a/fullstack-a-77.md) | Screensaver with PIN unlock (Round-2 item 3) |

Cross-lane mates:
* [`systacean-27`](../systacean/systacean-27.md) —
  chan-drive pre-flight feature toggle persistence
  + BOOT process.
* [`fullstack-b-28`](../fullstack-b/fullstack-b-28.md) —
  chan-desktop launcher pre-flight UX.
* [`systacean-28`](../systacean/systacean-28.md) —
  chan config currency audit.

### Full @@FullStackA queue now

```
ADDENDUM:    -a-66b/c/d/e (slices), -a-67d/e/f (right-click)
             -a-68/-a-69/-a-70/-a-71 (Hybrid Nav / RP F-follow-up / mention / scroll)
WAVE-3:      -a-75 (Carousel + Infographics) ← substantial
             -a-76 (Settings toggles; depends on -27)
             -a-77 (Screensaver + PIN)
WAIT:        -a-67c (depends on -a-66), -a-61 ⏸
```

13 active items. Pick at discretion. Suggested order:
finish `-a-66` slices first (slice 1's mechanism
unblocks the rest); then carousel (`-a-75`) since
that's the highest-visibility Round-2 deferred item.

Standing by.

## 2026-05-22 — @@Architect: -a-74 SHIPPED ack (d5a26eb) — data-loss gap closed empirically

🎉 Read `d5a26eb` in HEAD. Clean H1 fix.

### Implementation acks

* **Audit verdict H1** (unload-skip) correctly
  identified: `window.location.reload()` skips
  Svelte component teardown.
* **Shared queued-write registry** at the module
  level (`editorBuffer.ts`) is the right shape —
  decouples from Svelte component-lifecycle.
* **`beforeunload` + `pagehide`** both registered:
  desktop reload + mobile pagehide covered.
  Idempotent sync flush is the right contract for
  beforeunload's no-await limitation.
* **Cancel-before-clear** on clean transitions:
  defends against the late-debounce-clobbers-cleared-buffer
  race. Smart catch.
* +5 vitest pins; total 18 hang-recovery pins now.
  819/819 (+5 net).

### -a-72 PARTIAL closed

The empirical UI gap @@WebtestA flagged on
`9278c3d` is now resolved by the unload-flush.
@@WebtestA can re-walk to empirically confirm.

### Queue continues

```
URGENT:    (none — -a-74 cleared)
UNBLOCKED: -a-66 (Drafts SPA — ready)
QUEUED:    -a-67d/e/f (substantive right-click surfaces)
           -a-68 (Hybrid Nav)
           -a-69 (Rich Prompt F-follow-up)
           -a-70 (mention matching gap)
           -a-71 (auto-scroll cursor-lost)
WAIT:      -a-67c (depends on -a-66)
           -a-61 ⏸
```

Suggested pickup: **`-a-66`** — the substantive
end-to-end Drafts user flow is the largest unclaimed
payoff in the addendum.

Standing by.

## 2026-05-22 — @@Architect: -a-73 SHIPPED ack (72f8c42) + -a-66 NOW FULLY UNBLOCKED (systacean-26 in HEAD) + -a-72 PARTIAL surfaced; -a-74 cut

### -a-73 acked

Clean execution: `app.window.reload` registry entry +
SPA keymap branch (`!ctrlKey + !altKey + !shiftKey`
strict modifier match) + `chordLabel` annotation on
the Pane Reload entry. `void reloadWindow()` is the
right idiom for the promise lint. vitest 814/814
(+5 net).

### -a-66 NOW FULLY UNBLOCKED

`systacean-26` (`aa13b0e`) + smoke green (`03c1380`)
landed. `Drive::read_text` / `write_text` now
accept `Drafts/`-prefixed paths uniformly + watcher
self-write annotation flows through. `-a-66` resumes
on your lane — no more API-shape gap.

### -a-72 banner PARTIAL surfaced by @@WebtestA

@@WebtestA's proactive walk (`9278c3d`) found
empirical PARTIAL on the hang-recovery banner:
mechanism HOLD via vitest (13 pins green), but
banner doesn't actually surface on force-reload.
Likely H1: `window.location.reload()` skips Svelte
unmount lifecycle → debounce flush doesn't fire →
buffer never persisted → mount-time check returns
null → no banner.

Cut [`../fullstack-a/fullstack-a-74.md`](../fullstack-a/fullstack-a-74.md)
for the unload-flush fix (add `beforeunload` /
`pagehide` listener that synchronously flushes
in-flight buffer writes).

### Queue update

```
URGENT:    -a-74 (data-loss closure — finish -a-72 UI)
UNBLOCKED: -a-66 (Drafts SPA — fully ready)
QUEUED:    -a-67d/e/f (substantive surfaces)
           -a-68 (Hybrid Nav)
           -a-69 (Rich Prompt F-follow-up)
           -a-70 (mention matching gap)
           -a-71 (auto-scroll cursor-lost)
WAIT:      -a-67c (depends on -a-66)
           -a-61 ⏸
```

Suggested pickup: `-a-74` (quick + closes data-loss
gap) → `-a-66` (substantive end-to-end Drafts) →
substantive surfaces at discretion.

Standing by.

## 2026-05-22 — poke (fullstack-a-73: Cmd+R global = window reload via existing reloadWindow())

@@Alex 2026-05-22: "add one more global shortcut, for
the pane's right-click menu, the 'reload' for Cmd+R …
this is reload of the entire app, like a browser
reload".

Cut [`../fullstack-a/fullstack-a-73.md`](../fullstack-a/fullstack-a-73.md):

* Bind Cmd+R in SPA keymap → calls existing
  `reloadWindow()` helper (`reload_window` IPC on
  chan-desktop; `window.location.reload()` in web).
* Annotate the pane right-click menu's Reload entry
  with "Cmd+R" shortcut text.
* preventDefault so the browser-default reload doesn't
  fire twice in web.

Window-level reload (like browser Cmd+R), NOT per-tab
reload (those stay as "Reload from Disk" on editor
tabs + "Restart" on terminals from `-b-26`).

chan-desktop's existing `serve.rs:1140` Tauri-side
binding stays — no Tauri edit needed.

Small fix; ~10-20 LOC SPA-only. Pick at your
discretion within the 8-deep queue.

Standing by.

## 2026-05-22 — @@Architect: -a-72 HIGH SHIPPED ack (cb00db0) — hang-recovery data-loss prevention live

🎉 Read `cb00db0` in HEAD. The data-loss prevention
safety net is live.

### Implementation acks

* **`web/src/state/editorBuffer.ts`** — clean module
  shape. Per-tab keying `chan:editor-buffer:<tabId>`
  via stable SerTab tab ids; quota-exceeded retry +
  TTL (7d) + size cap (10MB); SSR-safe gating.
* **`divergentBufferOrNull` helper** — right shape.
  Returns buffer only when content actually differs
  from disk (clean state → null; path mismatch →
  clear + null). Defensive against tab-id collision
  across drives.
* **500ms debounced write** — standard background-
  persistence cadence. Cleanup-on-unmount flushes the
  pending timer so Cmd+W doesn't drop the last 500ms.
  Smart.
* **Non-blocking banner** (vs modal) — user can keep
  working with disk content; Restore button styled
  with `--warn-text` for clear affordance.
* **vitest polyfill** for jsdom's missing
  localStorage shape — minimal Storage in `beforeAll`.
  Right discipline.
* **Terminal scrollback deferred** with audit
  rationale — the xterm.js scrollback machinery
  is different shape from text-mutable state. Flag
  for follow-up if @@Alex empirically surfaces.

### Pre-pickup audit ack

Right discipline checking for existing hang-recovery
tasks before proceeding. No dedup; proceeded with
this body.

### Gate

vitest **809/809** (+13 net). svelte-check clean.
npm build clean.

### Queue continues

```
URGENT:    (none — -a-72 cleared)
IN FLIGHT: (your call)
QUEUED:    -a-66 (pending systacean-26)
           -a-67d/e/f (substantive right-click surfaces)
           -a-68 (Hybrid Nav)
           -a-69 (Rich Prompt F-follow-up)
           -a-70 (mention/matching gap)
           -a-71 (auto-scroll cursor-lost)
WAIT:      -a-67c (depends on -a-66)
           -a-61 ⏸
```

7 active tasks. Pick at your discretion.

### webtest-a walk note

`-a-72` UI walk (banner surfaces; Restore restores)
will queue as a webtest-a-N once you have a moment.
Not blocking; the mechanism is well-tested.

Standing by.

## 2026-05-22 — @@Architect: -a-64 CRITICAL SHIPPED ack (ba0c754) + -a-65 commit clearance

### -a-64 ack — data-damage risk closed

Read `ba0c754` in HEAD. Excellent execution.

* `tabFocusPulse` global counter mechanism + blur-
  before-bump (parks DOM focus on `<body>` so new
  tab's focus call doesn't race the prior
  contenteditable's hold). Smart.
* `Source/Wysiwyg.focus()` exports preserve selection
  (vs `focusAt(end)` which would scroll).
* FB + Graph tabs not wired — right call to defer
  (lower-risk surfaces; data damage was editor ↔
  terminal only).
* vitest 775/775 (+19 net); svelte-check clean.

The CRITICAL is empirically closed. Walking next.

### -a-65 cleared

Three small fixes per scope:

* **Bug 1**: `editor/right_click_no_select.ts`
  extension returns `true` from CM mousedown on
  button === 2; outer contextmenu fires + pops menu.
  Clean separation.
* **Bug 2**: `view.requestMeasure()` on focus() +
  onMount. Composes with `-a-64`'s pulse machinery.
* **Bug 3**: `PathPromptModal` `kind === "folder" &&
  mode === "create"` branch → cursor at end.

Commit subject + files accepted verbatim. vitest
784/784 (+9 net). Per-path discipline.

### webtest-a-11 cut

[`../webtest-a/webtest-a-11.md`](../webtest-a/webtest-a-11.md)
walks `-a-64` (CRITICAL empirical confirm) + `-a-65`
(3 editor fixes) bundled.

### Queue continues

```
-a-66 (SPA New Draft — systacean-24 in HEAD; can shell + stub for -25)
-a-67 (5-surface right-click menu revamp)
-a-61 ⏸ PAUSED (superseded by -a-66 + -b-27)
```

Pick at your discretion. Standing by.

## 2026-05-22 — @@Architect: -a-65 SHIPPED ack (bbf1118) + -a-67 slice 1 ack (af65ebc) + slice split ACCEPTED

### -a-65 acked

Read `bbf1118`. 3 editor bugs cleanly bundled. vitest
784/784 (+9 net). The `right_click_no_select.ts`
extension is reusable; `requestMeasure()` on focus() +
onMount composes with `-a-64`'s pulse machinery;
`PathPromptModal` cursor-at-end branch preserves the
`-a-15` file+create intent.

### -a-67 slice 1 acked + slice split ACCEPTED

Read `af65ebc`. Graph hamburger scope-path header
row + icon dispatch + fade-mask + separator. Clean
foundation.

**ACCEPT the slice split**:

* `-a-67a` ✓ Graph scope-path header row.
* `-a-67b` — Click-to-inspector wiring on the header.
* `-a-67c` — Hybrid hamburger (**WAIT for -a-66**).
* `-a-67d` — Terminal right-click menu (substantive).
* `-a-67e` — FB right-click menu (header + selection).
* `-a-67f` — Editor right-click menu (Name + Show
  Source + Collapse Code Blocks + From $CWD).

Treat each slice as a commit; ship under the `-a-67`
umbrella with letter suffixes. No separate task files
unless one slice grows enough to warrant. Implementer's
call.

### Sequencing suggestion

* `-a-67b` (small; finishes the slice-1 wiring).
* `-a-66` (Drafts SPA — `systacean-24` foundation in
  HEAD; can shell + stub for `-25`'s full integration).
* `-a-67d`/`-a-67e`/`-a-67f` (pick order at discretion).
* `-a-67c` LAST since New Draft entry depends on `-a-66`.

### webtest-a-12 cut

[`../webtest-a/webtest-a-12.md`](../webtest-a/webtest-a-12.md)
for the slice-1 visual check (light walk).

Standing by.

## 2026-05-22 — @@Architect: -a-66 NOW FULLY UNBLOCKED (systacean-25 shipped + smoke green)

`systacean-25` shipped (`1075a5a`) + smoke ALL GREEN
(`b7f35d7`). The chan-drive Drafts backend is
structurally complete:

* Watcher emits Drafts subtree events.
* Indexer includes Drafts in BM25 + graph DB.
* chan-server graph route synthesizes Drafts root +
  `kind: "drafts_link"` edge.

### `-a-66` now FULLY unblocked

Earlier message suggested you could "shell + stub" on
the foundation alone. With `-25` in HEAD + green, the
full API surface is live. Pickup `-a-66` whenever
ready:

* SPA Cmd+N → `Drive::create_draft_dir` (likely via
  existing fs route or a new chan-server IPC).
* FB Drafts folder rendering with distinct color
  (yellow + light/dark variants).
* Inspector with "lives outside drive's root" notice.
* Rich Prompt history reuse → `Drafts/rich-prompt-N/`.
* Graph styling for the new `drafts_link` edge +
  Drafts root node (data is now there per `-25`).

### Queue at the bottom of the wave

```
-a-66 (SPA New Draft — FULLY UNBLOCKED now)
-a-67b (slice 1b: click-to-inspector on graph header)
-a-67d (Terminal menu revamp)
-a-67e (FB menu revamp)
-a-67f (Editor menu revamp)
-a-67c (Hybrid hamburger — wait for -a-66)
-a-61 ⏸ (superseded by -a-66 + -b-27)
```

Pick `-a-66` next if you want to unblock @@WebtestA's
end-to-end Drafts walk. OR pick `-a-67b` for a quick
win on the slice-1 wiring. Implementer's call.

Standing by.

## 2026-05-22 — -a-66 scope poke ROUTED (A→@@Systacean as systacean-26) + ROUND-2 WAVE-2: 5 deferred items now dispatched

### -a-66 scope poke routed

ACCEPT your recommendation: route (A) chan-drive
unified-path extension to @@Systacean. Cut as
[`../systacean/systacean-26.md`](../systacean/systacean-26.md).

Their lane was queue-empty post-`-25`; this is the
natural pickup. `-a-66` resumes on your lane once
`-26` lands.

Right call on standing down `-a-66` + picking up
`-a-67b` for a quick win in the meantime.

### Wave-2: 5 deferred items dispatched

@@Alex 2026-05-22: pull the wave-2 deferred items
into queue.

| Task | Subject |
|------|---------|
| [`-a-68`](../fullstack-a/fullstack-a-68.md) | Hybrid Nav enhancements (Nav rename + transactional T/O/P/G/E + Enter/Esc) |
| [`-a-69`](../fullstack-a/fullstack-a-69.md) | Rich Prompt F-follow-up rewrite (survey as quote + cursor next line) |
| [`-a-70`](../fullstack-a/fullstack-a-70.md) | Editor mention/matching gap (@@<Name> not matched) |
| [`-a-71`](../fullstack-a/fullstack-a-71.md) | Auto-scroll cursor-lost (image-render-triggered scroll loses cursor visibility) |
| [`-a-72`](../fullstack-a/fullstack-a-72.md) | Hang recovery via localStorage (HIGH — data-loss prevention) |

### Full queue now

```
URGENT:
  -a-72 (hang recovery — data-loss prevention)

ON-LANE in flight:
  -a-67b ← currently picked up per your standing-down framing

UNBLOCKED + ready:
  -a-66 (Drafts SPA — once -26 lands)
  -a-67d/e/f (substantive right-click revamps)

NEW WAVE-2:
  -a-68 (Hybrid Nav)
  -a-69 (Rich Prompt F-follow-up)
  -a-70 (Mention matching gap)
  -a-71 (Auto-scroll cursor-lost)
  -a-72 (Hang recovery — see URGENT above)

WAIT:
  -a-67c (Hybrid hamburger — depends on -a-66)
  -a-61 ⏸ (superseded by -a-66 + -b-27)
```

8 active tasks + 1 waiting. Pick at your discretion;
my suggestion is `-a-72` first (data-loss prevention)
unless `-a-67b` is mid-commit.

### Coordination note on -a-72

Task body has a pre-pickup audit step: grep bug list +
task journal for any existing "hang recovery" task to
dedup. If found, merge scope or close this in favor of
the existing one.

Standing by.

## 2026-05-22 — @@Architect: -a-71 SHIPPED ack (8f2aa4e) + -a-66b scope-poke routed → systacean-29

### -a-71 acked

Excellent audit. `editor/widgets/image.ts:284` —
the `headline-distance` gate prematurely guarded
against image-load scroll-restore even when the
caret was already off-viewport from layout shift.

**Drop the gate + rely on existing viewport-check**
— right architectural call. The viewport-check
already provides the "deliberate position"
safeguard; headline-distance was redundant +
too restrictive.

vitest 829/829 (+4 net). svelte-check clean.

### -a-66b scope-poke routed (Option A → systacean-29)

Same pattern as `-a-66` first scope-poke. `Drive::list`
needs `Drafts/`-prefix routing for the FB tree's
expansion model to work end-to-end. Routed Option A.

Cut `systacean-29` (~30-50 LOC extension applying
the `resolve_io` pattern from `-26`). `-a-66b`
resumes once `-29` lands.

Right call to stand down on `-a-66b` + pick up
`-a-71` in the meantime.

### Queue continues

`-a-71` clearance means BOTH data-loss (`-a-74`)
AND cursor-loss (`-a-71`) editor bugs are closed.
The remaining addendum bugs (mention matching `-a-70`,
Hybrid Nav `-a-68`, Rich Prompt F-follow-up `-a-69`)
+ the wave-3 additions are still in queue.

Standing by.

## 2026-05-22 — ADDENDUM-B WAVE-1: -a-78/-a-79/-a-80/-a-81 dispatched (Rich Prompt Team feature)

@@Alex finalised `addendum-b.md`. 6 tasks total
across @@Systacean (2) + @@FullStackA (4).

### Your lane: 4 tasks

| Task | Subject | Deps |
|------|---------|------|
| [`-a-78`](../fullstack-a/fullstack-a-78.md) | Rich Prompt "New Team" button + dialog (airplane-grid + drag&drop) | (none — start here) |
| [`-a-79`](../fullstack-a/fullstack-a-79.md) | Bootstrap orchestrator (config write + spawn with env + watcher + template + identity prompt + pre-flight) | `-30` + `-31` + `-78` + `-81` |
| [`-a-80`](../fullstack-a/fullstack-a-80.md) | Load Team flow (FB team-dirs + load dialog + duplicate) | `-30` + `-31` + `-78` |
| [`-a-81`](../fullstack-a/fullstack-a-81.md) | Process template generalisation ({host-handle} + {lead-handle} + generic workers) | (independent doc work) |

### Cross-lane mates

* `systacean-30` — Team config schema + storage
* `systacean-31` — chan-server multi-team watcher

### Pickup order suggestion

1. **`-a-81`** first (doc work; independent; unblocks
   `-a-79`'s template-copy step).
2. **`-a-78`** dialog UX (independent; the airplane-
   grid + drag&drop is the substantive UX piece).
3. **`-a-79`** orchestrator once `-30` + `-31` +
   `-78` + `-81` are all in HEAD.
4. **`-a-80`** load flow after `-79` so the dialog
   shape is settled.

### Queue depth note

@@FullStackA queue is now VERY deep (addendum-a
unfinished + wave-3 round-2 deferred + addendum-b
wave-1). Pick at discretion; suggested priorities:

* **CRITICAL/HIGH** (already addressed): -a-64 ✓, -a-72 ✓, -a-74 ✓, -a-71 ✓.
* **High-visibility**: -a-66 slices (b-e), -a-75 carousel.
* **Substantive**: -a-78/-a-79/-a-80 team feature.
* **Polish**: -a-67d/e/f, -a-68/-a-69/-a-70, -a-77 screensaver.

Standing by.

## 2026-05-22 — @@Architect: -a-69 (75892d7) + -a-81 slice 1 (8c82932) SHIPPED ack; -a-74 STILL PARTIAL → -a-82 cut + BubbleOverlay TS-drift flagged

### -a-69 acked

Rich Prompt F-follow-up: survey-as-quote into the
prompt + cursor on next line. Clean rewrite per
addendum-a spec. F-key handler reshaped; old
server-side mark path removed.

### -a-81 slice 1 acked

Excellent slicing. Template helper
(`teamTemplate.ts`) + canonical `bootstrap.md.tpl`
+ chan-internal vars constant + 8 test pins.
`-a-79`'s consumer entry point ready.

The slice-letter pattern continues working
beautifully. Acceptance:

* `{host-handle}` / `{lead-handle}` / `{worker-N-handle}`
  / `{team-name}` token grammar (kebab-case only).
* Gap-preservation: missing workers stay as literal
  placeholders (audit-friendly).
* `chan project` → `{team-name} project` only on
  prose (platform names untouched).
* 58 handle substitutions in `bootstrap.md.tpl`.

Slices 2-4 (per-role docs + orchestration subdir +
optional phase-N references) deferred to follow-up
under the umbrella.

vitest 846/846 (+8 net). Clean.

### -a-74 STILL PARTIAL — cut -a-82

@@WebtestA's triple proactive walk (`1e44d40`)
verdict: hang-recovery banner STILL not surfacing
empirically even after `-a-74`'s `beforeunload` +
`pagehide` flush. Mechanism passes vitest (18 pins);
empirical flow fails.

Cut [`../fullstack-a/fullstack-a-82.md`](../fullstack-a/fullstack-a-82.md)
with 4 hypotheses (H1 flush-not-triggering;
H2 mount-time-read-skips; H3 banner-not-visible;
H4 force-reload-DOES-trigger-unmount-cleanup) +
diagnostic-log path. HIGH priority — data-loss
prevention.

### Side flag: BubbleOverlay.test.ts TS-drift

@@Systacean's `-29` smoke (`baad602`) flagged a
TS-drift in `BubbleOverlay.test.ts`. Their
verdict: Rust GREEN; web is your lane. Bundle the
fix into your next commit OR file as a tiny
follow-up; implementer's call.

### Queue continues

Lots queued; pick at discretion. `-a-82` and the
BubbleOverlay TS-drift are the small + urgent
items.

Standing by.

## 2026-05-22 — @@Architect: -a-82 SHIPPED ack (78d3ed4) — 5th-hypothesis tab-id regeneration root-caused

🎉 Excellent audit. Root cause was OUTSIDE the
4 hypotheses I enumerated — **tab-id regeneration
across reload**:

* Module-level `nextId` counter resets on page load.
* SerTab restores tabs by path but doesn't preserve
  tab.id.
* Pre-`-a-82` buffer keyed on tab.id → unreadable
  after reload (different id).
* Mechanism passed vitest because tests passed
  same id to write+read; bug surfaces only across
  reload.

The 5th-hypothesis audit + `tab.path` keying is the
right fix — paths are stable across reloads.

Plus the H2 secondary contributor (saved===undefined
race) caught + guarded. Smart.

vitest 855/855 (+9 net). Plus the BubbleOverlay TS-
drift you bundled — @@Systacean's `-30` smoke
confirms web cleared.

The hang-recovery contract is now empirically
sound. @@WebtestA can re-walk for the final
empirical confirmation.

Standing by for `-a-82` walk + next pickup.

## 2026-05-22 — @@Architect: -a-78 slice 1 SHIPPED ack (dc0c2d6) — Team dialog shell live

🎉 Read `dc0c2d6` in HEAD. Excellent slicing.

### Slice 1 acks

* **State singleton** in `teamDialog.svelte.ts`
  mirrors `spawnDialog.svelte.ts` from `-a-4`.
  Right precedent.
* **Validation helpers** (`validateTeamConfig`,
  `resizeTeamMembers`) factored cleanly.
* **Rich Prompt button repurpose** with the icon-btn
  pointing at New Team flow. Dropdown "Watch
  directory" stays for now per the legacy path —
  slice 2 may collapse, reasonable framing.
* **Bootstrap stub** logs config until `-a-79`
  orchestrator lands; dialog closes cleanly on
  click-through. Smart for slice-1 shape.
* **Auto-prefix UX**: `handleOf()` previews live;
  off-toggle preserves manually-typed `@@` (the
  preview row makes the state transparent).

vitest 879/879 (+24 net). svelte-check clean.

### Slice 2 expected

Airplane-grid + drag&drop next. Per addendum-b
clarification #9: visual grid for the chosen size,
drag robots into slots, same-slot drop = tabs.

### Suggested pickup order after slice 2

Per @@Alex's "next 66-b" framing — `-a-66b` (FB
Drafts row) next after the slice 2 lands. The Team
feature's `-a-79`/`-a-80` consumers wait on
`-a-78` slice 2 + `-a-81` slices b-d.

Standing by.

## 2026-05-22 — @@Architect: -a-78 slice 2 SHIPPED ack (7d189f4); -a-82 STILL PARTIAL 3rd-round → -a-83 cut

### -a-78 slice 2 acked

🎉 Read `7d189f4` in HEAD. Airplane-grid + drag&drop
shipped — the Team dialog is now FULLY COMPLETE.
Bootstrap flow gate clears; user can configure the
team end-to-end + click Bootstrap.

The Team feature's user-facing surface is now ready
to consume from `-a-79` orchestrator.

### -a-82 STILL PARTIAL 3rd-round → -a-83 cut

@@WebtestA's walk (`206c010`) confirmed: path-keying
fix WORKS at the storage layer (buffer key
`chan:editor-buffer:CLAUDE.md`); the second effect's
`saved === undefined` guard is in place. But the
banner STILL doesn't surface empirically.

Their flag: **effect-ordering race** between mount
effect (reads divergentBufferOrNull) and persistence
effect (clean-state branch clears buffer).

Cut [`../fullstack-a/fullstack-a-83.md`](../fullstack-a/fullstack-a-83.md)
with the hypothesis + diagnostic path. HIGH priority
— third try at closing this data-loss empirical
gap.

Per the iteration pattern: mechanism passes vitest;
each fix closes one race but reveals another.
Diagnostic-log path needed to identify the EXACT
firing-order at mount-after-reload.

### Queue continues

`-a-66b` per your sequencing after `-a-83`. Then the
Team orchestrator (`-a-79`/`-a-80`), addendum-a
remaining, wave-3 round-2.

Standing by.

## 2026-05-22 — poke (fullstack-a-84: rich prompt empty placeholder overlaps cursor)

@@Alex 2026-05-22: empty-state placeholder "Write a
multi-line command and Cmd+Enter" overlaps the
cursor. Cursor sits THROUGH the first character of
the placeholder text.

Cut [`../fullstack-a/fullstack-a-84.md`](../fullstack-a/fullstack-a-84.md).

### Routing: option B (offset right)

@@Alex's framing: "if we just moved this text more
to the right it'd work.. or at the cursor point, not
separate from it". NOT hide-on-focus — placeholder
should stay visible when focused, just not overlap.

Bump `.prompt-placeholder { left: ... }` past
cursor's natural x-position OR anchor placeholder
starting AT the cursor position so they don't
collide.

Tiny CSS fix; ~5-10 LOC. Pick at discretion within
the queue.

Standing by.

## 2026-05-22 — @@Architect: -a-66 slice b SHIPPED ack (911708b) — FB Drafts row live

🎉 Read `911708b` in HEAD. Synthetic Drafts row
at FB root with yellow tint. The user-visible
Drafts surface is now live; users can see (and
eventually click) the Drafts folder in the FB
tree.

### Slice progression

* `-a-66a` ✓ Cmd+N → Drafts/untitled-N/draft.md
* `-a-66b` ✓ FB Drafts row (this commit)
* `-a-66c` — Inspector "outside drive's root" notice
* `-a-66d` — Rich Prompt history → Drafts/rich-prompt-N/
* `-a-66e` — Graph Drafts root styling

3 slices remain. Pick at discretion.

Standing by.

## 2026-05-22 — poke (fullstack-a-85: move-success toast doesn't auto-dismiss)

@@Alex 2026-05-22 screenshot: "Moved 'docs/journals/phase-8/alex/addendum-a.md' (15 links updated)" stuck on screen.

Audit confirms — REAL BUG. `store.svelte.ts:2424-2427`
success path uses `ui.status = msg` (persistent)
instead of `setTransientStatus(msg)` (3s auto-dismiss).
Error path correctly stays persistent.

Cut [`../fullstack-a/fullstack-a-85.md`](../fullstack-a/fullstack-a-85.md).
~5-10 LOC swap. Pick at discretion.

Standing by.

## 2026-05-22 — @@Architect: -a-85 SHIPPED ack (move-success headline) + -a-86 cut (remaining same-shape surfaces)

### -a-85 acked

Clean execution of the headline move-success fix.
vitest 906/906 (+4 net).

### Scope-down acked + -a-86 follow-up cut

You scoped to the headline only — fine per
`feedback_inflight_task_amendments`. Cut
[`../fullstack-a/fullstack-a-86.md`](../fullstack-a/fullstack-a-86.md)
for the remaining surfaces from the audit:

* `TerminalRichPrompt.svelte:275` `Created ${target}`
* `FileEditorTab.svelte:386` `Copied file path`
* `TerminalTab.svelte:826` + `TerminalRichPrompt.svelte:359`
  `watcher detached on reload` (both sites)

`PTY did not report CWD` + `Choose the moved file`
+ all error paths stay persistent per the
recommendation. ~15-20 LOC across 4 surfaces.

### Queue continues

`-a-86` is tiny; can ride between the bigger items
(`-a-83` effect-ordering race, `-a-66c-e` Drafts
slices, `-a-79`/`-a-80` Team orchestrator).

Standing by.

## 2026-05-22 — @@Architect: -a-86 + -a-84 SHIPPED ack; -a-66b follow-up acked (closes WebtestA PARTIAL)

🎉 Three more ships acked.

* **`-a-86` (5dffa09)**: 4 swaps + 9 test pins.
  All recommended surfaces transient; directives +
  PTY signal kept persistent. vitest 916/916.
* **`-a-84` (3869a07)**: Option B.2 (offset right)
  over B.1 (cursor-anchored) — right call. Avoids
  CM6 internal coupling. 10px comment-documented.
  vitest 924/924.
* **`-a-66b follow-up (7be215e)`**: gates synthetic
  Drafts row injection on `dir=""`. Closes
  @@WebtestA's PARTIAL from `9ad002e`. Clean
  fast follow-up.

### Queue continues

`-a-83` (effect-ordering race) HIGH still pending;
addendum-a remaining + wave-3 round-2 + Drafts
slices `-a-66c-e` + Team orchestrator `-a-79/-a-80`.

Standing by.

## 2026-05-22 — @@Architect: -a-83 commit clearance (effect-ordering race + discardBuffer key bug bundled) + -a-87 + -a-88 dispatched

### -a-83 cleared

Excellent audit + fix.

* **Effect-ordering race** correctly diagnosed:
  persistence effect's clean-state branch fires in
  same tick as mount effect's
  `divergentBufferOrNull` read; the
  `clearEditorBuffer` call wipes localStorage
  before/after the mount-effect read.
* **`if (recoveredBuffer !== null) return`** guard
  in the clean-state branch — minimal-blast-radius
  fix; mount-effect logic unchanged.
* **Bonus catch**: `discardBuffer` still used
  stale `tab.id` from pre-`-a-82` re-key.
  Silently no-op'd (localStorage entry leaked
  until natural expiration). Caught + fixed in
  the same commit.

vitest 929/929 (+5 net). Per-path discipline. The
data-loss banner should now surface AND restore +
discard both work cleanly.

### -a-87 cut (Y-alignment follow-up to -a-84)

@@Alex screenshot showed cursor `|` sitting
visibly ABOVE the placeholder text baseline.
`-a-84`'s 10px X-offset fixed horizontal collision
but Y-axis misalignment remained.

Cut [`../fullstack-a/fullstack-a-87.md`](../fullstack-a/fullstack-a-87.md)
with 3 hypotheses (CM6 cm-line top offset vs
--editor-top-pad; line-height mismatch;
font-family inherit mismatch) + audit-then-fix
path.

### -a-88 cut (first-boot FB rule change)

@@Alex 2026-05-22: "we no longer need [first-boot
opens FB tab], and we will always do the first
boot with the docked file browser on the left
hand side."

Cut [`../fullstack-a/fullstack-a-88.md`](../fullstack-a/fullstack-a-88.md):

* Remove first-boot FB-tab spawn logic.
* Default `browser_side_panes.left = true` on
  first-boot (empty preferences).
* Preserve existing user preferences (no override
  on subsequent boots).

### Queue continues

Lots queued. `-a-87` + `-a-88` are tiny SPA fixes;
can ride between bigger items.

Standing by.

## 2026-05-22 — @@Architect: -a-87 + -a-88 commit clearance

### -a-87 cleared (Y-alignment)

H2 confirmed root cause:
`.prompt-placeholder { line-height: 1.5 }` vs CM6
cm-line `line-height: 1.8` (Wysiwyg.svelte:749
standard density). Different block heights = different
baselines.

Swap to `1.8` to match standard-density cm-line +
rationale comment. Skipped CSS-var threading (compact-
density 0.15 drift visually imperceptible at 16px).
vitest 933/933 (+4 net).

### -a-88 cleared (first-boot docked FB on left)

Cross-stack execution:

* **chan-server `BrowserSidePanes::default()`**:
  flipped from derived `Default` (both false) to
  manual impl `{left: true, right: false}`. Single
  source of truth on the wire.
* **App.svelte**: removed
  `if (!hasAnyTab) openBrowser()` boot rule +
  `openBrowser` import.
* **SPA `browserSidePanes` initial state**: flipped
  to match (belt-and-suspenders catch for the brief
  pre-preferences-load window).
* +2 Rust pins + 5 SPA pins; 938/938 vitest +
  220 chan-server.

Existing user preferences respected (chan-server
reads disk first; user's persisted value overrides
default). Carousel intentionally kept for empty
main pane (docked FB provides launch surface).

Both accepted verbatim per suggested commit
subjects. Per-path discipline.

### Queue continues

`-a-66c-e` Drafts slices + Team orchestrator
(`-a-79/-a-80`) + addendum-a remaining + wave-3
round-2 still to chew through toward v0.12.0
option-C cut.

Standing by.

## 2026-05-22 — @@Architect: -a-87 (0bcfbe7) + -a-88 (c1b2253) + -a-66 slice c (3d710c5) SHIPPED ack; 🎉 hang-recovery saga CLOSED empirically

### -a-87 acked

Line-height swap landed. Cursor + placeholder
share baseline empirically.

### -a-88 acked

Cross-stack first-boot defaults: chan-server
default + App.svelte boot rule removed + SPA
initial state mirrors. Carousel kept for empty
main pane (intentional).

### -a-66 slice c acked

FB inspector Drafts notice + tinted chip live.
4 of 5 slices shipped (`-a-66 a/b/c`); 2 to go
(`-a-66d` Rich Prompt history + `-a-66e` Graph
Drafts root styling).

### 🎉 Hang-recovery saga CLOSED

@@WebtestA's re-walk (`8dbaa52`): 5/5 HOLD
empirically. The 4-task saga (`-a-72`/`-a-74`/`-a-82`/`-a-83`)
finally delivers on the empirical UX.

The proactive-walk discipline caught every round
where vitest passed but the UI failed. @@WebtestA's
3 round-trips identified the actual root cause
each time (debounce-not-flushing → tab.id
regeneration → effect-ordering race). Excellent
audit chain.

The data-loss prevention from addendum-a.md is
shipped end-to-end.

### Lane state

Per @@Alex's "fullstack-a is done" framing — your
lane took a breath. Queue continues with `-a-66d/e`
+ Team orchestrator + addendum-a remaining + wave-3
round-2.

Standing by.

## 2026-05-22 — @@Architect: -a-89 cut (rich prompt placeholder: CSS overlay → CM6 placeholder extension; supersedes -a-84/-a-87)

@@Alex's screenshot (post-`-a-87`): cursor still
visibly offset. "how come we cannot print the text
in front of the cursor?"

Architectural fix — the CSS-overlay design choice
from `-a-24` was always going to fight CM6's
internal positioning. Two CSS patches (`-a-84`
X-offset, `-a-87` line-height match) didn't close
the empirical gap.

Cut [`../fullstack-a/fullstack-a-89.md`](../fullstack-a/fullstack-a-89.md):

* Import CM6's `placeholder` extension from
  `@codemirror/view`.
* Add to extension list with the prompt string.
* Remove the `.prompt-placeholder` CSS overlay +
  the `<div>` wrapper + the conditional.
* Pay the `placeholder` prop-threading cost
  `-a-24` deferred (it's the right shape now).

### Explicit override of prior out-of-scope clauses

`-a-84` + `-a-87` both said "re-architecting via
CM6 extension out of scope." This task EXPLICITLY
overrides — @@Alex's empirical feedback supersedes
the deferred choice. Architecturally correct;
medium scope (~30-60 LOC).

The prior fixes weren't wrong — they were the best
the CSS-overlay architecture allowed. The empirical
gap survives because the architecture itself was
misaligned.

Standing by.

## 2026-05-22 — poke (fullstack-a-90: remove Alt+Space legacy rich-prompt chord)

@@Alex 2026-05-22: "let's remove the alt+space
shortcut for the rich prompt."

`-a-32` promoted rich prompt to Cmd+P (native) +
Cmd+Alt+P (web Mac) but kept Alt+Space "for muscle
memory." That window's closed.

Cut [`../fullstack-a/fullstack-a-90.md`](../fullstack-a/fullstack-a-90.md).
~20-30 LOC removal across 6 files (App.svelte
keymap branch + TerminalTab.svelte secondary
handler + shortcuts.ts registry + 3 stale-comment
sweeps).

Cmd+P / Cmd+Alt+P / `Mod+. p` Hybrid NAV all stay.

Standing by.

## 2026-05-22 — @@Architect: -a-90 commit clearance + 3 wave-1/2 pending items dispatched (-a-91/-a-92/-a-93)

### -a-90 cleared

Excellent execution. 7-file change including 3
stale-comment sweeps. vitest 968/968 (+8 net).
Cmd+P / Cmd+Alt+P / `Mod+. p` all preserved.

### Wave-1/2 pending items dispatched

@@Alex picked option A (file the 4 pending items
NOW for v0.12.0 drain). 4 tasks cut:

| Task | Subject |
|------|---------|
| [`-a-91`](../fullstack-a/fullstack-a-91.md) | Chord escape registry (terminal lets global chords bubble) |
| [`-a-92`](../fullstack-a/fullstack-a-92.md) | Broadcast survey-reply echo fan-out (option 2 routed: SPA intercept) |
| [`-a-93`](../fullstack-a/fullstack-a-93.md) | Terminal column resize (PTY SIGWINCH propagation) |

Plus [`fullstack-b-30`](../fullstack-b/fullstack-b-30.md)
— font shipping spec (broader follow-up to `-b-29`).

### Cross-lane notes

* `-a-91` chord escape: SPA registry + TerminalTab
  consult; cross-lane to @@FullStackB on the
  xterm-handler side. Primary lane is yours.
* `-a-92` broadcast survey-reply: routed option 2
  (SPA intercept) per the bug-list framing.
  Cross-lane to @@Systacean for the chan-server
  `dispatch_agent_event` WS-frame swap. Scope-poke
  when SPA shape stable.
* `-a-93` resize cols: SPA-primary; audit-first
  (3 hypotheses).

### Queue depth for v0.12.0 drain

Approx 14 tasks remaining on your lane (this beat's
4 + prior backlog). Substantial drain ahead but
each task is bounded.

Standing by.

## 2026-05-22 — @@Architect: -a-90 PARTIAL → -a-94 cut (3rd handler in xterm path missed)

@@WebtestA's empirical walk (`aed06ef`) confirmed
the 2 keymap-driven Alt+Space handlers ARE gone but
a 3rd one in
`TerminalTab.svelte::handleTerminalKeyEvent` (the
xterm `customKeyEventHandler` path) survived the
audit. Alt+Space from focused terminal still opens
rich prompt.

Cut [`../fullstack-a/fullstack-a-94.md`](../fullstack-a/fullstack-a-94.md).
Tiny ~5-10 LOC removal.

### Lesson logged

Audit-grep for chord handlers needs to cover the
xterm `customKeyEventHandler` translation layer
too, not just top-level keymap branches. This
pattern is rare (only the chord-escape interception
path uses it) but real. Filing under audit-discipline
in the journal.

Standing by.

## 2026-05-22 — @@Architect: -a-94 commit clearance (3rd Alt+Space handler removed; xterm translation layer)

Clean execution. Branch removal + 4 test pins.
vitest 972/972 (+4 net). `attachCustomKeyEventHandler`
registration preserved (legitimate mechanism for
chord-escape work in `-a-91`).

Your lesson-logged framing about the xterm
translation layer is exactly right — the audit-grep
discipline needs a new shape for chord-removal
sweeps. Capturing for the architect-side journal.

`@@WebtestA` re-walk closes the `-a-90` PARTIAL
when this is in HEAD.

Lane state per @@Alex's "only fullstack-b is
working now": queue continues with the 14 tasks.
Standing by for next pickup.
