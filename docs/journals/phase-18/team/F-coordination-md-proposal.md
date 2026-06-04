# Proposal: Wave-3 scrub of docs/coordination.md (for @@Lead pre-review)

Per task-Lead-LaneF-3.md guardrail 3, coordination.md (PUBLIC explainer)
needs a content edit, not a path swap, because Wave 3 deletes
docs/journals and the doc currently tells readers to go look there. This
is the proposed diff, staged for your eyeball BEFORE I land it at
round-close. Nothing is edited in coordination.md yet.

Kept factual + minimal: 1 mandatory intro fix, 1 mandatory section
rewrite (the stale path list), and 1 OPTIONAL light touch (your call).
New text follows the no-em-dash project rule even though the existing
doc uses em dashes throughout; I did NOT reflow the doc's other em
dashes (out of scope for "minimal").

## Edit 1 (mandatory) - intro, lines 3-8

BEFORE:
> If you've landed here from the issue tracker, a PR, or just browsing
> the repo, this doc explains the multi-agent development pattern you'll
> see reflected in the journals (`docs/journals/phase-N/`). It's not a
> user-facing document; it's an explainer for outside contributors and
> curious readers.

AFTER:
> If you've landed here from the issue tracker, a PR, or just browsing
> the repo, this doc explains the multi-agent development pattern behind
> the consolidated phase reports in [`phases/`](phases/). It's not a
> user-facing document; it's an explainer for outside contributors and
> curious readers.

## Edit 2 (mandatory) - "What you'll see in the repo", lines 99-113

BEFORE:
> * `docs/journals/phase-N/` - the active phase's journals. Read
>   `process.md` and the architect's `architect/journal.md` to orient.
> * `docs/journals/phase-N/alex/event-*.md` - the event channels. Naming
>   convention is `event-<from>-<to>.md` ...
> * `docs/journals/phase-N/<role>/<role>-N.md` - task files. Each is a
>   self-contained brief plus the append-only progress journal.
> * `docs/agents/` - role contact cards + skill files. Useful if you want
>   to know what each role's responsibilities cover.

AFTER:
> * `docs/phases/phase-N.md` - one consolidated report per phase: its
>   roadmap, rounds, waves, and retrospective. This is the front door to
>   the project's history.
> * `docs/agents/` - role contact cards plus `playbook.md`, the
>   cross-phase operational lessons (coordination, the gate, verification,
>   commit discipline, the pre-release norms).
> * While a round is active, the team runs an append-only coordination
>   bus (task files plus one-line pokes) under the phase's working
>   directory. It is distilled into the phase report when the round
>   closes; the raw per-phase working material is preserved in git
>   history, not the working tree.

## Edit 3 (OPTIONAL, your call) - "How work flows", lines 50-70

This section names the per-phase raw artifacts (request.md, process.md,
phase-N-bugs.md, `<role>/<role>-N.md`, `alex/event-*.md`). They no longer
live in the working tree post-deletion, but the section describes the
PATTERN, not a browse path, so it does not 404. Minimal option: leave it,
or append one sentence after line 74:

> These per-phase artifacts are consolidated into the phase report at
> close and preserved in git history; the section above describes the
> shape they took, which evolved over the project (see
> [`agents/playbook.md`](agents/playbook.md)).

Recommend: do Edit 3 (the one sentence) so the section is not subtly
misleading about what's in the tree. But it is optional and I default to
your preference; flag keep-or-drop.

## UPDATE (task-Lead-LaneF-4): all 3 edits APPROVED + em-dash scope add

@@Lead approved Edit 1, Edit 2, and Edit 3 (do the one-sentence hedge in
"How work flows"), and EXTENDED scope: convert the file's existing em
dashes to the project ASCII style (mechanical, meaning-preserving; flag
any that risk meaning). Below is the complete em-dash plan so the land is
mechanical.

### Em-dash inventory (11 total) and conversions

Eliminated by Edit 2's rewrite (no separate swap needed):
- L101, L104, L108, L111 (the "What you'll see in the repo" bullets are
  replaced wholesale; the AFTER text uses " - ").

Clean " - " swaps (mechanical, meaning unchanged):
- L54  `(`request.md`) — the owner's`          -> `(`request.md`) - the owner's`
- L56  `(`process.md`) — how the team`         -> `(`process.md`) - how the team`
- L59  `(`phase-N-bugs.md`) — durable`         -> `(`phase-N-bugs.md`) - durable`
- L62  `(`<role>/<role>-N.md`) — what each`     -> `(`<role>/<role>-N.md`) - what each`
- L66  `(`alex/event-<from>-<to>.md`) —`        -> `(`alex/event-<from>-<to>.md`) -`

Light restructure (where " - " reads wrong; flagged per your guardrail):
- L96  `(not SHAs — multi-agent rebases would invalidate SHAs)` ->
       `(not SHAs; multi-agent rebases would invalidate SHAs)`
       (em dash inside parentheses; semicolon preserves the meaning
       without an in-parens " - ").
- L127 `...the multi-agent pattern — it's an internal coordination
       protocol, not a project requirement.` ->
       `...the multi-agent pattern. It's an internal coordination
       protocol, not a project requirement.`
       (sentence split; cleaner than a mid-sentence " - ").

None of the 11 risk changing meaning. No other reflow or rewording.

## When I land it

At round-close, in the Wave-3 scrub pass, I re-read coordination.md
fresh, apply the 3 edits + the em-dash conversions above (anchored on the
verified BEFORE text), then poke you the actual
`git diff docs/coordination.md` for final sign-off before the commit,
exactly as you asked. The 2 light restructures (L96, L127) will be
visible in that diff for your call.
