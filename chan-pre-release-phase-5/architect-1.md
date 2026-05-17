# @@Architect task 1: coordination, decisions, wave-2 planning

Owner: @@Architect
Status: in progress

## Goal

Keep [journal](./journal.md) accurate as wave-1 finishes and dispatch
wave-2 (enhancements, bug fixes, hardening) from a clean baseline.

## Wave-1 coordination

* Mark [backend-1](./backend-1.md) and [frontend-1](./frontend-1.md) REVIEW
  in the dispatch table when their owners confirm completion.
* Hand the deeper chan-llm + chan-drive prune to
  [systacean-1](./systacean-1.md) with explicit acceptance criteria.
* Pick up the frontend residue in [frontend-2](./frontend-2.md).
* Stand up the smoke service in [webtest-1](./webtest-1.md) and the
  parallel scenarios in [webtest-2](./webtest-2.md).

## Wave-2 planning checklist

Capture as task files only once wave-1 is REVIEW/clean.

### Indexer + reports + search

* @@Systacean: prioritise graph + chan-report indexing ahead of full-text
  search. Define the gate and ordering inside `crates/chan-server/src/indexer.rs`
  and chan-drive's indexer.
* @@Systacean: expose a search-aggression knob (config + CLI flag),
  default conservative.
* @@Systacean: fs-change detection (git/hg checkouts), correctness tests,
  benchmarks, and resumption hardening.

### Embedded terminal

* @@Architect: verify @@Backend's terminal MCP env-var landing meets the
  request (`CLAUDE_MCP_SERVER_JSON` etc are picked up by claude / codex /
  gemini). Brief @@Webtest to validate end-to-end with a real CLI.
* @@Architect: design memo for tmux `-CC` integration with Alex. Decide
  fork vs. depend-on-tmux vs. Rust `tmux-cc` shim before implementation
  starts.

### Bug fixes

* @@Frontend: confirm-on-close for tabs with unsaved files or live
  terminals (reload exempt).
* @@Frontend: per-window pane/tab state for chan-desktop reloads.
* @@Frontend: editor scroll behaviour at top of screen-sized pages.
  Default proposal: only scroll-into-view when the cursor is in the
  bottom margin; suppress scroll when the cursor is near the top.

### Hardening + close-out

* End-to-end pass for every workflow above.
* Pre-push gate green on the final HEAD before push.
* Push the three terminal commits + the cleanup commits together at
  phase close.
* Produce [summary.md](./summary.md) with outcomes, highlights,
  lowlights, bugs, coverage, follow-ups, and agent rankings.

## Notes / decisions

* Workspace is the chan repo proper; chan-drive / chan-llm / chan-report /
  tunnel crates live here, not in a sibling. Updates to chan-drive +
  chan-llm happen in this checkout.
* Push timing: phase close, not piecemeal.
* CLAUDE.md "sibling chan-core" framing is stale and should be refreshed
  during the docs sweep.

## Progress

* Wrote [journal](./journal.md), reconciled wave-1 in-flight work, drafted
  wave-2 task slate.

## Completion notes

(populated as wave-2 lands)
