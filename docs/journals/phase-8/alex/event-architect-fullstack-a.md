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
