# task-Conductor-Editor-1 — items 4 + 1: tab-click focus, then editor keep-alive

From: @@Conductor. To: @@Editor. Cut: 2026-06-12.

## Scope

Items 4 and 1 from new-team-2/round-1-plan.md, in MANDATORY order:

1. Item 4 — clicking a terminal tab must focus the terminal
   (mouseup pulse fix).
2. Item 1 — editor tab keep-alive (raw-markdown flash + scroll-reset
   fix; the round's biggest web change).

Design (read fully before the first edit):
new-team-2/designs/item-1-4-editor-keepalive-and-tab-focus.md.
Line numbers are from main @ 3ebee587 — verify before editing.

## Sequencing / ownership

- You own web/src/components/Pane.svelte until your item-1
  restructure lands. @@PromptQueue's badge edit is gated behind it;
  nobody else touches the file.
- MILESTONE POKE (deliberate exception to one-poke-per-task): when
  the item-1 Pane.svelte restructure commit lands, poke me the sha
  (1 line) so I can release @@PromptQueue.

## Gate

- Own-gate after the FINAL edit (re-run if you edit again):
  `make web-check` (vitest) + svelte-check + build.
- Browser-smoke the reactivity changes in Chrome — static gates miss
  Svelte-5 runtime errors. Tear down any ad-hoc server + tabs after.
  Chrome is shared: verify location.href before asserting.
- WKWebView is the real verification gate for both items — route a
  desktop-build request through me when ready (@@Desktop owns builds).
- Commits pathspec-atomic: `git commit -F <msg-file> -- <paths>`
  (flags BEFORE `--`); `git diff --staged --stat` before,
  `git show --stat HEAD` after.
- Sweeps with `rg --text --no-ignore`, no type filters.

## Review pairing

- Your web commits get an adversarial second pass by @@TeamFlow
  (I route them; behavior preservation, not style).
- You review @@TeamFlow's web commits and @@Desktop's launcher JS
  when I route them to you.

## Stretch — B2 (NOT by default)

B2 (dispatch-to-matcher-loop shortcut refactor) is behavior-risk:
only after you cut me a short design note and I sign off. Do not
start it on your own.

## Completion

Cut new-team-2/tasks/task-Editor-Conductor-<n>.md with: commit shas,
gate results, verification evidence (explicitly: what is
Chrome-verified vs WKWebView-pending), follow-ups for round close.
ONE completion poke after the last part (plus the milestone poke
above). Journal in new-team-2/journals/journal-Editor.md, append-only.
