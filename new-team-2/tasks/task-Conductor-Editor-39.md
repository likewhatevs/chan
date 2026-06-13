# task-Conductor-Editor-39 — graph tab keep-alive + Reload menu item (round-2 add-on)

From: @@Conductor. To: @@Editor. Cut: 2026-06-13.

## Scope (yours end-to-end)

@@Alex's pre-release add-on: stop the graph tab reloading on every
activation (keep-alive, your dadd5e64 pattern extended to a third tab
kind) + re-add the Reload right-click menu item between Depth and
"Copy link to graph". Web-only, three coupled files + tests.

FULL SPEC + verified anchors + ratified decisions:
new-team-2/designs/round-2-graph-keepalive.md — read it fully before
the first edit. It is self-contained (the approved harness plan,
transcribed into the bus). Line numbers are from main @ 00a585b3 —
verify before editing.

## Why you

This is literally your keep-alive work (dadd5e64) extended to graph
tabs — same Pane.svelte each-block family, same visibility contract.
You own all three files in one burst (Pane.svelte, GraphPanel.svelte,
GraphCanvas.svelte are a coupled prop contract; one owner avoids the
transient-compile-window problem).

## The two non-obvious cores (don't lose these)

1. **Hidden-state gating.** With keep-alive a hidden graph can't rely
   on remount-to-refresh. So: first load LAZY on first activation (not
   mount — else N graph tabs load at once on session restore); a hidden
   tab that misses an in-scope watcher edit sets a dirty flag and
   reloads ONCE on next activation; the watcher still reloads the
   VISIBLE graph (kept, per @@Alex). Latches are PLAIN locals, not
   $state (no state_unsafe_mutation). Spec § GraphPanel has the exact
   effect restructure.
2. **GraphCanvas `open` must LATCH, plus a new `paused` prop.**
   start() resets the pan/zoom transform (line 1323) and stop()
   discards the sim — so open={active} would kill pan/zoom on every
   switch. Latch open true once shown (open={canvasEverShown}); add
   paused={!active} to suspend the rAF loop so hidden graphs do zero
   background paint (the huge-workspace win). Resume effect resize()s +
   re-arms the loop, never start()s. Spec § GraphCanvas.

## Gate

- Own-gate `make web-check` (svelte-check + vitest + build) AFTER the
  final edit.
- Browser-smoke the Network panel recipe in the spec § Verification
  (tab switch = zero requests; Reload = exactly one; watcher via
  on-disk edit; hidden→dirty→one-on-reactivation; pan/zoom/selection
  survive; lazy restore; console clean for state_unsafe_mutation —
  the class static gates miss). Tear down server + tabs; Chrome is
  shared, verify location.href before asserting.
- WKWebView is the REAL gate (same surface as dadd5e64) — route a
  build request through me when ready; @@Desktop owns it.
- Commits pathspec-atomic: `git commit -F <msg> -- <paths>`;
  staged-stat before, show-stat after. One commit is fine for the
  coupled three-file change + tests, or split impl/tests as you see
  fit.

## Review

@@TeamFlow cross-reviews (they reviewed dadd5e64 — same surface,
adversarial behaviour-preservation). Route the sha to me on landing;
I'll route the review. Expect targets: the latch/dirty correctness,
the GraphCanvas latch-not-toggle, the .graph-tab CSS reconciliation
(drop flex:1), and the onClose-captures-t bug-avoidance.

## Completion

Cut new-team-2/tasks/task-Editor-Conductor-<n>.md: sha(s), gate
results, Chrome-smoke evidence (explicitly Chrome-verified vs
WKWebView-pending), any follow-ups. ONE completion poke. Journal in
journal-Editor.md, append-only.
