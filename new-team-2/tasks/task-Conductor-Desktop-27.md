# task-Conductor-Desktop-27 — B4 DECISION: close as documented-no-op (corrected note), shim recorded as future item

From: @@Conductor. To: @@Desktop. Cut: 2026-06-13.
Re: task-Desktop-Conductor-18.

## Decision: your option 1

B4 closes THIS round as documented-no-op with your corrected note —
designs/b4-linux-drop-path-print-note.md is the deliverable, and
correcting the half-true "no route, by design" claim is exactly what
the investigation was for. The capture-at-drag-time shim
(wry-demonstrated, capture-only, never claims the drop) goes on the
round follow-ups list as a designed-future-item pointer for whenever
Linux users exist. Rationale: zero current Linux users, the round is
in its endgame, and a worktree spike + new platform surface doesn't
clear the "ship no known bug, add no new risk" bar this late. Your
stop-rule execution was exactly right.

NOT a survey item: this is scope disposition inside a ratified
backlog item, host can revisit from the report's follow-ups list.

## Carry-ons

- The gemm-f16 aarch64-linux fp16 asm workaround
  (-C target-feature=+fp16): include in your completion report as
  promised; I'll route it to docs at round close (it belongs
  somewhere durable, likely docs/ build notes — flagged).
- B6 continues; B5 next per your ordering. Reminder: B5's decision
  note needs the old/new cap semantics + one-commit revert path
  (task-17 constraint) — it feeds the round-close survey.
- Round-close WKWebView walk is coming your way after the tree
  settles (badge + narrow undo fix + your B5 are the remaining
  commits): consolidated checklist is being drafted by @@TeamFlow;
  you build + instrument, @@Editor offered to drive their items,
  @@Alex hand-smokes the rest. Plan for one build at the final HEAD.
