# Phase 13 round 2 retrospective

Round close: 2026-05-29. Release: **v0.18.0** (tag pushed off main
`cf9c4e83`). Builds on the v0.17.0 round-1 cleanup.

Per `feedback_round_close_retrospective` + `feedback_curated_status_reports`:
curated round-close view (done / pending + highlights / lowlights /
contention + honest feedback for the agents, @@Alex, and the architect).
Detail lives in the per-lane journals + the coordination channel tails.

## Scope status

### Done

**Lane A (Team Work full-stack revamp)** - `55179ad9` + `25c81182`:
- Renamed Rich Prompt -> Team Work (UI + the new component/flow).
- Deleted the filesystem-watcher agent-event coordination backend
  (event watcher, event-reply / submit-mode endpoints, terminal-session
  dispatch, the rich-prompt workspace archival + spool).
- New lead-first Cmd+P flow: instantiate the Team Work lead terminal
  (embedded editor) first, then the redesigned Spawn-agents dialog (Neo
  default, New/Load TOML toggle, 1-9 dropdown, Lead radio, real-estate
  toggle); Cancel deletes the lead tab; Bootstrap is lead-first.
- Reduced bubbles to a frontend-only static stub.
- Follow-up dead-code cleanup: deleted the orphaned team name-registry
  API (client + backend routes + chan-workspace wrappers).

**Lane B (editor + desktop + shell + merge-gate)**:
- B1 list marker glyphs (`3eb7f4c4`): en-dash for `-`, filled circle
  (U+25CF) top / hollow circle (U+25EF) nested for `*`, ordered keeps
  source numbers; in-flow CSS so the glyph tracks text-indent at depth.
- B2 Bold/Italic chords + Dashboard off Cmd+I (`dc3a1230`): bound Mod-b
  / Mod-i in CM6; removed the hardcoded Cmd+I->Dashboard handler
  (App.svelte + desktop KEY_BRIDGE), repointed Dashboard to Hybrid Nav.
- B3 desktop Cmd+Shift+N -> current workspace (`b16e699d`).
- B4 hamburger split labels -> Cmd+/ , Cmd+? (`f2f78e52`).
- Cmd+, per-pane flip fix (`8c6f4a94`): strictly per-pane, >= 1 tab,
  no cross-pane coupling, persists across reload.
- Team Work label applied in the Lane-B-owned shared files (`ae06398b`).
- Merge gate: combined-tree re-gate + residual cleanup (`74ec13d7`) +
  full rich-prompt scrub (`c4a4adc6`) + merge to main; CHANGELOG
  v0.18.0 + v0.17.0 entries + orchestration-doc refresh; v0.18.0 cut.

### Pending / carryover

- chan-desktop (WKWebView) empirical walk by @@Alex on the combined
  build: B3 Cmd+Shift+N, the Cmd+I removal, the Cmd+P->teamWork
  KEY_BRIDGE. Gated + Chrome-smoked, but per
  `feedback_terminal_webgl_wkwebview` the desktop shell needs a human
  walk; flagged, not blocking the cut per
  `feedback_pre_release_merge_unverified`.
- Notification bubbles are a static stub; equivalent functionality
  returns in a later phase (documented in the orchestration skill +
  CHANGELOG).
- The orchestration skill (atomic-writes.md + the event contracts) still
  describes the removed watcher system as the blueprint for the
  returning implementation; a fuller rewrite lands when the replacement
  does.

## Highlights

- Clean auto-merge across both lanes despite three overlapping files
  (Pane.svelte, tabs.svelte.ts, App.svelte) - the file-disjoint lane
  split held; the only real overlap (the App.svelte Cmd+I branch +
  Pane.svelte dead-watcher-dot) was declared on the cross-lane channel
  before editing.
- Browser smoke earned its keep twice: it caught the B1 nested-glyph
  gutter detachment (absolute ::before ignores text-indent) and
  confirmed the Cmd+, per-pane invariants + the rich-prompt wire rename
  end to end. The static gate was green on both broken intermediates.
- The Cmd+, fix root-caused TWO coupling sources (splitPane copying the
  flip onto an empty pane + the round-1 B2c setActivePane band-aid that
  was itself the regression) rather than patching the symptom.
- The 160-ref rich-prompt scrub landed with 0 svelte-check collisions +
  a green browser smoke of the renamed chord/CSS/field.

## Lowlights

- B1's first attempt (absolute-positioned ::before) was wrong and only
  the browser smoke caught it - I knew the list-line CSS used a negative
  text-indent and should have predicted that an out-of-flow box ignores
  it. One avoidable round-trip.
- The blanket scrub flipped a TerminalTab absence-guard
  (`.not include "Rich Prompt"` -> `.not include "Team Work"`,
  contradicting the "Show Team Work" toggle the same test requires). I
  anticipated absence-guard hazards but still shipped one into the gate;
  caught by vitest, fixed manually.
- The "rich prompt" cleanup arrived in three escalating asks (fix the 2
  residuals -> no rich prompt code -> refresh the docs). Each was clear,
  but I'd committed the narrow cleanup before the broad directive, so
  the scrub superseded my own commit.

## Contention

- The chord-id rename (`app.terminal.richPrompt` ->
  `app.terminal.teamWork`) overrode Lane A's deliberate "keep the chord
  id stable" cross-lane decision. Resolved by @@Alex's explicit "do not
  leave any rich prompt code behind"; the merge gate is the right place
  to reconcile a whole-tree rename once both lanes have landed.

## Feedback

**For @@LaneA**: the rename was thorough on the feature surface but left
user-facing residuals (`broken_rich_prompt` warning label, the Drafts
inspector `rich-prompt-N` notice) and kept internal identifiers. A
`grep -i "rich.?prompt"` sweep before signalling merge-ready would have
surfaced both, and flagging "I'm intentionally keeping X internal ids"
on the cross-lane channel would have set the merge-gate's expectation.

**For @@LaneB (me)**: predict the static-gate-blind failure modes BEFORE
the smoke, not after - the text-indent interaction (B1) and the
absence-guard flip (scrub) were both foreseeable. When running a blanket
identifier rename, pre-scan for assertions that check the OLD string's
ABSENCE and exclude/fix them deliberately.

**For @@Alex**: the incremental directives were decisive and correct,
but "rename Rich Prompt -> Team Work" implies "completely" - stating the
"no legacy identifiers, pre-release so no back-compat" expectation in the
round-2 roadmap up front would have let Lane A do the full scrub in its
own commit (cleaner provenance) instead of the merge gate retrofitting
it. Same for "keep the changelog history + add a rename entry": a
one-line changelog convention in the bootstrap avoids the round-trip.

**For the architect (round-2 request files)**: `lane-b-request-round-2.md`
asserted B-slice 2 needed "no App.svelte change", but the real
Cmd+I->Dashboard binding was a hardcoded `e.code === "KeyI"` branch in
App.svelte (+ the desktop KEY_BRIDGE), not the shortcuts.ts registry the
anchor pointed at. Grounding the file/line anchors in the actual dispatch
site (per `feedback_ground_descriptions_in_source`) would have avoided a
mid-round cross-lane overlap declaration. Separately, the cross-lane plan
"keep the chord id stable" conflicted with the eventual whole-tree
rename; deciding rename completeness up front (it's a pre-release rename,
so "complete" is the obvious default) would have removed the contention.

## Process notes

- The merge gate caught what the lanes' own gates could not: cross-lane
  residuals + whole-tree rename completeness. Re-gating the COMBINED
  tree (not trusting per-lane green) + a residual grep are the load-
  bearing steps.
- Reused the round-1 lesson: gated Lane A's NEW head (`25c81182`) after
  their cleanup, not the stale `55179ad9` - read the channel tail +
  branch HEAD, never a noted status.
- Docs committed in-flight per @@Alex's explicit ask (deviating from
  `feedback_coordination_docs_commit_timing`); the round-close commit
  still captures the final tree.