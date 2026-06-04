# coordination.md diff for @@Lead sign-off (guardrail 3)

Staged (uncommitted) in the working tree. See it live with:

    git diff docs/coordination.md

Holding the commit until you sign off. The other Wave-3-partial work is
already committed (74909e64 consolidation, 2e372a93 scrubs);
coordination.md is the only file left uncommitted on my side.

## What changed

Approved edits (task-Lead-LaneF-4):
- Edit 1 (intro): "the journals (`docs/journals/phase-N/`)" ->
  "the consolidated phase reports in [`phases/`](phases/)".
- Edit 2 ("What you'll see in the repo"): the 3 journal-path bullets ->
  docs/phases/phase-N.md + docs/agents/ (with playbook) + a bullet on the
  live per-round coordination bus distilled at close. Resolves the two
  relative links: `phases/` and `agents/playbook.md` (both exist).
- Edit 3 (one sentence in "How work flows"): the per-phase artifacts are
  consolidated + in git history, pointing to agents/playbook.md.
- Em dashes: the 5 numbered-list items (request/process/bug-list/task-
  files/event-channels) -> " - "; the parenthetical "(not SHAs - ...)"
  -> "(not SHAs; ...)" (semicolon, reads better than " - " in parens).

Additions BEYOND the enumerated edits (flagging for your review; same
stale-reference class, all visible in the diff):
- L89 was a `↔` arrow, not an em dash (you listed line 89): converted
  "architect ↔ desktect" -> "architect <-> desktect" for ASCII
  compliance. Tell me if you'd rather "between architect and desktect".
- 3 prose "the journals" references that go stale post-deletion and would
  contradict Edit 1: "the journals can look confusing" -> "it can look
  confusing"; "The journals cite commit subject lines" -> "The reports
  cite ..."; "so the journals make sense" -> "so the history makes
  sense". I left the 3 CONCEPTUAL "append-only journals" mentions (they
  describe the journaling discipline, still accurate).

No other reflow or rewording. 0 em dashes, 0 `↔`, 0 docs/journals paths
remain in the file.

## On your go

Say the word and I commit it as:
  docs(coordination): rewrite for the docs/phases layout + ASCII typography
via pathspec (`-- docs/coordination.md`), then it joins the other two
committed docs commits. If you want any of the "beyond-enumerated" items
reverted, I will before committing.
