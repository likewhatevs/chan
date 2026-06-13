# task-Conductor-Editor-11 — cross-review: @@TeamFlow items 3+5 (3 commits, batched)

From: @@Conductor. To: @@Editor. Cut: 2026-06-12.

## PRIORITY ORDER — read first

Your item-1 restructure stays FIRST (it gates @@PromptQueue's badge
edit and two WKWebView verifications). Do this review at your next
natural break after the restructure lands. Batched deliberately so
you get one interrupt, not three.

## Scope

Adversarial second pass (behavior preservation + design conformance)
of @@TeamFlow's lane, all verified on main, each pathspec-atomic:

1. 0f146fcf — item 3, broadcast OFF:
   web/src/state/teamOrchestrator.svelte.ts +
   teamBootstrapOrchestrator.test.ts.
   Design: designs/item-3-broadcast-default-off.md. Targets: the
   clear-all sweep MUST survive (stale-group hygiene); only the
   lead-enable line + worker-target loop deleted; they also removed
   a dead workerTabs binding + orphaned import — confirm those were
   genuinely dead (no other reader); test re-pin asserts
   membership-EMPTY, not just lead-off; the pre-existing-groups-
   cleared test (~195-228) retained.
2. c9fbb909 — item 5 Part A, X dismiss:
   web/src/components/BubbleOverlay.svelte + survey.svelte.test.ts.
   Design: designs/item-5-survey-first-x-dismiss.md § Part A.
   Targets: x/X added alongside Escape with the same guard pattern
   (preventDefault + stopPropagation, focused-card scope); Escape
   still works; 1..9 and F/f untouched; label matches the [F]
   styling convention; comment updated.
3. 86a0dce9 — item 5 Part B, bootstrap template:
   crates/chan-server/src/routes/team_config.rs (Rust, but template
   strings + tests — in your range). Targets: survey-first wording
   matches the design's 3 requirements; ASCII-only assertion still
   present and still meaningful; interpolation style
   ({host_handle}/{lead_handle}/{team_dir}) consistent; extended
   test pins the NEW wording (not a tautology); no behavior change
   outside generate_bootstrap_md.

## Completion

Findings (or clean pass) →
new-team-2/tasks/task-Editor-Conductor-<n>.md + 1-line poke (may be
folded into your item-1 milestone/completion flow if that's your
next poke anyway). Findings become tasks routed by me — @@TeamFlow
fixes their own lane.
