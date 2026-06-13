# task-Conductor-Editor-30 — cross-review: item-2 badge (7c976a68) + b82a0a27 rider

From: @@Conductor. To: @@Editor. Cut: 2026-06-13.

## Why this exists (and why you)

@@TeamFlow's report-data matrix caught that the badge commit has no
review row — the web-half review (task-19) explicitly excluded it
(unlanded at the time) and I never routed it after release. Second
gap the data audit has caught. You're the right reviewer: you own
Pane.svelte, the pill renders inside your restructured strip, and
the commit's flipped-pane claim is your keep-alive domain. Small:
36 insertions total.

## Scope

1. 7c976a68 — feat(web): terminal tab-strip queue-depth pill.
   Pane.svelte +22, richPromptTerminalWiring.test.ts +14. Verified
   on main. Design: designs/item-2-prompt-queue-visibility.md
   § Pane badge ("next to the activity dot, same affordance
   family"). Targets:
   - Placement + gating: terminal tabs only,
     `(t.queueDepth ?? 0) > 0`, title "queued terminal messages" —
     per design § Pane badge.
   - YOUR restructure integration: the pill sits in the strip you
     restructured — confirm it doesn't disturb your item-4 mouseup
     path, the drag affordance, or the close-button hit area.
   - The flipped counter-mirror claim: they added .queue-pill to
     the `.tabs.flipped` selector list so the digit isn't mirrored
     — verify the selector list edit is complete (your flip-face
     analysis from the item-1 review is the lens; is there any
     OTHER transform context the pill needs entering?).
   - Test pin quality: badge markup + flipped-selector pinned, not
     tautological.
2. RIDER (30s): b82a0a27 — 4-line docs-only comment on
   enqueue_write (the N1 lock nuance). Confirm comment-only (no
   code motion) and that the comment states the nuance accurately
   (cf. task-CtxPass-Conductor-14 N1).

## Completion

Findings (or clean pass) →
new-team-2/tasks/task-Editor-Conductor-<n>.md + 1-line poke. This
is the round's genuinely-last review; the WKWebView walk follows
@@Desktop's B6.
