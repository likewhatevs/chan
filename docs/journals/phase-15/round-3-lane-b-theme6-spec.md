# Theme-6 docs/journals cleanup - spec (@@LaneB, Wave 3)

Authorization: @@Host confirmed DELETE-RAW + SUMMARIZE (round-3-status.md
"@@Host decisions"). Runs after B's Wave-1 relative-link rule (landed
b273e0b5). Safeguard (status doc): defer deleting any artifact still cited by a
live reference until that reference is updated, so the audit trail does not
break.

## End state

Each closed phase keeps ONE essence doc (`README.md`) plus a hashtag line; the
bulky `raw/` provenance is dropped from the working tree (preserved in git
history). Images are already transcribed-and-removed for phases 1-13 (top-level
README records this), so the image step is a no-op for the in-scope phases.

## Per-phase disposition (grounded in the survey)

```
phase   raw/ cited by            action this wave
-----   --------------------     -------------------------------------------
1-7     own README only          delete raw/; add Tags; neutralize raw-links
9-13    own README only          delete raw/; add Tags; neutralize raw-links
8       docs/agents/{desktect,   DEFER raw/ deletion (cited externally, even
        bootstrap}.md (ALREADY-  if those links are already broken). Add Tags
        broken pre-raw paths)    to its README only. See Escalation below.
14      flat raw files + no      SYNTHESIZE essence README (phases 1-13 style)
        README                   + Tags; then delete the flat raw files.
15      ACTIVE round bus         EXCLUDE. phase-15 docs are the live
                                 coordination bus; @@Architect commits the
                                 phase-15 tree at round close. Its essence
                                 README + raw cleanup happen at/after that.
pub-    untracked, not a phase   EXCLUDE. Not committed, not a phase journal;
site-                            @@Host working material, out of Theme-6 scope.
release
```

## README edit rules (phases 1-7, 9-14)

1. Keep the README narrative essence verbatim (it is already the synthesized
   report; do not re-summarize or shorten the analysis).
2. Replace the trailing `## Raw material` section (the list of `raw/...` links)
   with a single line:
   "Raw working material (per-author journals, task/request/roadmap files,
   coordination logs) is preserved in git history under this phase's `raw/`
   tree; it was removed from the working tree in the phase-15 docs cleanup."
   Drop the now-dangling `[raw/...]` links. Keep any prose that section carried
   that is not just a link (e.g. the image-removal note) as plain text.
3. Add a `Tags:` line immediately under the Status/Span header.
4. No em dashes; ASCII; factual; WHY-not-WHAT. Relative-markdown links only
   (Wave-1 rule) for any cross-phase link that remains.

## Hashtag taxonomy (pick the relevant subset per phase from its README)

Core outcome tags:
  #features #bugfixes #reliability #performance #refactor #docs #release
Area tags (add where a phase centred on one):
  #editor #search #graph #terminal #desktop #cli #mcp #ci #security #tunnel
  #indexing

Rule: tag what the phase ACTUALLY shipped/fixed per its own README, not
aspiration. 3-6 tags per phase is the target.

## phase-14 README synthesis

Phase 14 (Gateway monorepo migration, then a frontend review + pristine
cleanup, per the top-level journals README) has flat raw (lane-{a,b,c}-*,
roadmap-round-{1,2,3}, addendum-1, lane-c-c2-spec, coordination/) but no
README. Synthesize a README matching the phases 1-13 front-door shape: Status
(closed) + Span, Tags, Initial asks, Team/coordination, Duration (from git
author dates), Highlights/lowlights, Constructive feedback, What shipped/tried/
undone. Then the flat raw is deleted (git history preserves it).

## Top-level docs/journals/README.md update

- Note the layout change: phases keep only `README.md`; the `raw/` provenance
  now lives in git history (removed in the phase-15 cleanup). Phase-8 raw is
  retained for now pending the docs/agents citation fix.
- Flip phase-14 from "in progress" to closed with its one-line summary.
- Keep the conventions section.

## Escalation to @@Architect (cross-lane / safeguard)

1. phase-8 raw DEFERRED: `docs/agents/desktect.md` + `docs/agents/bootstrap.md`
   cite phase-8 content (`phase-8/architect/...`, `phase-8/alex/...`,
   `phase-8/process.md`). Those links are ALREADY broken (they predate the
   raw/ reorg; the files now sit under `phase-8/raw/...`). Deleting raw/ would
   remove even the content a future fix would point at. docs/agents/ is OUT of
   @@LaneB's lane. ASK: assign the 5-citation fix (repoint to `raw/...` or drop)
   and then I delete phase-8 raw, OR keep phase-8 raw indefinitely.
2. Confirm phase-15 + pub-site-release exclusion (my reading: yes - active bus
   + untracked non-phase).

These do not block the phases 1-7,9-14 cleanup, which proceeds now.
