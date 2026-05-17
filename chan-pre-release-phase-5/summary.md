# Chan Pre-Release Phase 5 Summary

Status: **DRAFT** — populated while the final webtest re-smoke
runs. @@Architect publishes the non-draft version at Alex's signal
to close the phase.

Source request: [request.md](./request.md). Process:
[process.md](./process.md). Journal: [journal.md](./journal.md).

## Outcome and completion status

Phase 5 is in the close-out window. All wave-1 cleanup and wave-2
implementation lanes are REVIEW; one end-to-end smoke
([webtest-1](./webtest-1.md) PTY-reattach + multi-attach + idle
close) is running. Final close-out steps remaining at draft time:

* Webtest A's PTY-reattach + multi-attach + idle-close smoke.
* [architect-3](./architect-3.md) commit-message rewrite for the
  three chan-term commits (holding for Alex's sanity-check of the
  drafts).
* [architect-2](./architect-2.md) commit-groupings + the actual
  commits.
* Push at phase close.

## Request checklist outcome

### Cleanup
* `../chan-term` merge plan — finalised; the three terminal
  commits were already on local main, push owed at phase close.
* In-app Agent overlay + Agent history overlay removed end to end:
  frontend components, store, types, API client, settings, hash
  state; backend `/api/llm/*`, `/api/assistant/*`, `/api/answers`,
  `llm.toml`, LLM `/ws` events.
* chan-llm pared to MCP-only: dropped `LlmSession`, claude_cli /
  codex_cli / gemini_cli / mock / ndjson / subprocess_env backends,
  dead config / CLI / error / bench.
* chan-drive `*_assistant` blob API removed (helpers + tests +
  state-dir reservation).
* MCP server and external-agent drive access preserved.
* Documentation refresh: CLAUDE.md (layout, paths, MCP-only
  principle), design.md (workspace + routes + chan-llm), README.md
  spot-audit clean.

### Enhancements
* Embedded terminal exports MCP discovery env per session:
  `CHAN_MCP_SERVER_NAME`, `CHAN_MCP_SOCKET`, `CHAN_MCP_COMMAND`,
  `CHAN_MCP_COMMAND_JSON`, and `CHAN_MCP_SERVER_JSON`. Phase 5
  intentionally keeps this inside the `CHAN_` namespace; tools can
  translate the descriptor into CLI-specific MCP config.
* Indexer scheduling tightened: graph + chan-report run ahead of
  search on rebuild; watcher event-loss (provider-error /
  path-less) coalesces into a full rebuild; incremental gate now
  matches chan-drive's `is_indexable_text`; deletions apply
  before upserts in a due batch.
* `[search].aggression` config (conservative / balanced /
  aggressive) threaded through chan-server + chan-drive, with
  `chan serve --search-aggression` CLI flag. Default `balanced`.
* git/hg-aware indexing: drive-root VCS detection, `.git/HEAD`
  / `.git/index` / `.hg/dirstate` allowed through the watcher
  filter, control-file events + large bursts coalesced into one
  full rebuild. Staged-row resume guarded by `(mtime, size)`
  match.
* End-to-end correctness + manual benchmark coverage on git
  checkouts (80 files / 20 touched: initial 11078ms, checkout
  settle 3138ms, staged resume 235ms).
* Persistent terminal sessions: chan-native PTY session registry
  in chan-server, byte-offset ring replay, idle prune (default
  30 min), soft cap (default 32 per drive), drive/shutdown/
  explicit/idle/capped close reasons. Client persists
  `terminalSessionId` + `lastSeq` per tab in the per-window
  session blob, sends `session=<id>&since=<seq>` on reattach,
  surfaces missed-scrollback + close-reason UI.

### Bug fixes
* Confirm-on-close for dirty file tabs and live terminal tabs;
  reload remains uninterrupted.
* chan-desktop windows now key the editor session by
  `w=<window-label>` instead of sharing `default`. Pagehide
  keepalive uses the same key.
* Editor scroll: caret restore uses nearest-scrolling so the
  page no longer auto-scrolls when the cursor is near the top
  of a screen-sized page.

### Closing
* End-to-end hardening pass: in flight at draft time
  ([webtest-1](./webtest-1.md)).

## Highlights

* Wave-1 cleanup was already mostly in flight by the time
  capacity planning finished — the team self-dispatched
  efficiently and the journal had to be reconciled mid-stream.
  The cleanup landed clean with no rework.
* Most of wave-2 also overlapped. systacean-2 / 3 / 4 / 5 / 6
  shipped one after another inside a single session window;
  frontend-3 / 4 / 5 likewise.
* Decision pivot on the terminal-persistence direction: Alex
  proposed Option 4 (chan-native PTY session registry) inside
  the [architect-tmux-1](./architect-tmux-1.md) memo, which is
  a fraction of the cost of any tmux-compatible approach and
  holds the "single binary, no runtime deps" line. Implementation
  closed cleanly in [systacean-5](./systacean-5.md) +
  [frontend-4](./frontend-4.md) inside the same window.
* fs-change correctness benchmark (80 files / 20 touched)
  produced concrete numbers (initial 11078ms, checkout settle
  3138ms, staged resume 235ms) that make future regressions
  measurable.
* Webtest A's re-smoke caught BUG-WT5-A, then later confirmed
  it disappeared after [systacean-4](./systacean-4.md), and
  Systacean added the regression test that pins the fix in
  place.

## Lowlights

* @@Architect orientation lagged the first wave. Journal had to
  be reconciled to in-flight work in round-1 housekeeping;
  noted in [journal.md](./journal.md) for next phase.
* Stale "sibling chan-core" framing in CLAUDE.md / design.md
  was load-bearing — agents would have reasoned wrongly about
  the workspace layout. Caught and rewritten in
  [architect-2](./architect-2.md), but pre-Phase-5 CLAUDE.md
  should be considered fragile in retrospect.
* Webtest re-smoke needed to wait for binary rebuilds; rebuild
  cycles (npm build → cargo build → restart) consumed a fair
  amount of wall time. Not unique to this phase.

## Bugs found and fixed

* **BUG-WT5-A** — incremental indexer missed `WatchKind::Created`
  events on indexable text files. Surfaced by [webtest-1](./webtest-1.md)
  round-3; resolved as fallout from
  [systacean-4](./systacean-4.md)'s classifier rewrite; pinned
  by [systacean-6](./systacean-6.md)'s new
  `create_event_admits_new_indexable_file_into_bm25` test.

## Observations (not regressions)

* **OBS-WT5-B** — `chan serve --search-aggression <level>` is
  applied at runtime but `/api/server/config` and `/api/config`
  return the persisted value. Phase 5 ships no UI for the knob,
  so no user-visible miss today. Deferred to the phase that
  introduces a Settings surface; proposed fix is an
  `effective_*` field on the response. Tracked in
  [architect-1](./architect-1.md).

## Test and hardening coverage

Across all lanes:

* `cargo fmt --check` green.
* `cargo clippy --all-targets -- -D warnings` green.
* `cargo build --no-default-features` green.
* `cargo test --workspace` 724 passed / 0 failed / 2 ignored
  on the round-4 baseline (was 703 at orientation; +21 from
  systacean-3/4 + watcher convergence + ring buffer +
  systacean-6 regression test).
* `npm --prefix web run check` clean (3919-3921 files).
* `npm --prefix web test -- --run` 16 files / 144 tests.
* `npm --prefix web run build` clean (existing Vite chunk-size
  warnings only).

Live coverage in [webtest-1](./webtest-1.md) and
[webtest-2](./webtest-2.md): all wave-1 + wave-2 acceptance
criteria PASS on the live service, including the MCP env
discovery transport (`initialize` + `tools/list` round-trip)
inside the PTY.

Manual benchmark coverage in [systacean-4](./systacean-4.md):
80-file / 20-touched git checkout profile, initial 11078ms,
checkout settle 3138ms, staged resume 235ms.

## Remaining follow-ups

* OBS-WT5-B (Settings UI effective vs persisted aggression).
* PATCH `/api/config` requires the full preferences body; either
  document or relax to true PATCH. Parked for Alex's call.
* Real `claude` / `codex` / `gemini` CLI end-to-end MCP
  validation needs a machine with those CLIs installed; not
  exercised on the dev host this phase.
* tmux `-CC` external compatibility — explicitly out of scope
  per Alex's Option 4 decision; archived in
  [architect-tmux-1](./architect-tmux-1.md).

## Agent rankings and feedback

(Populated by @@Architect at phase close, after the final smoke
and commit lands. Holding the section so the structure is
visible.)

* **@@Architect** (me) — TBD.
* **@@Backend** — TBD.
* **@@Frontend** — TBD.
* **@@Systacean** — TBD.
* **@@Webtest A** — TBD.
* **@@Webtest B** — TBD.

## Final delivery

* Local main is 3 commits ahead of `origin/main` at draft time
  (the chan-term terminal trio). Wave-1 + wave-2 cleanup +
  enhancement + bug-fix commits will land on top; the terminal
  trio's commit messages get rewritten through
  [architect-3](./architect-3.md). Push at phase close per the
  decision recorded in [journal.md](./journal.md).
