# Chan Pre-Release Phase 5 Summary

Status: **FINAL** — closed by @@Architect on Alex's "let's wrap!"
signal after round-10 acceptance reported all PASS.

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

Rankings are about phase-5 fit, not ability in absolute. Every
agent delivered.

* **@@Webtest A** — top of the phase. Caught BUG-WT5-A (create-
  event indexer regression), BUG-WT5-C (terminal-reload bootstrap
  race), and most importantly the **systacean-9 partial-redraw
  regression that Alex flagged visually** — Webtest A's own
  initial "PASS (MVP)" call was wrong; they corrected it under
  Alex's direct callout and produced the diagnosis + screenshot
  diff that drove [systacean-10](./systacean-10.md)'s fix. The
  screenshot-diff bar they established is now the canonical
  acceptance shape for any TUI reattach work. Constructive
  feedback: file the initial smoke result with a "preliminary"
  flag when the visual surface hasn't been compared against a
  fresh-launch baseline; the round-9 mis-call cost a round.

* **@@Systacean** — highest implementation throughput. Carried
  the chan-llm deep prune, the indexer scheduling tightening,
  the search-aggression knob, the VCS-aware fs-change path with
  benchmark numbers, the chan-native PTY session registry +
  byte-offset ring + idle prune + soft cap, the bootstrap-
  hydration race fix (systacean-8), and the alt-screen sniff +
  winsize wobble (systacean-10) — eight separate lanes, all
  landing with full pre-push gate green. Constructive
  feedback: the systacean-9 "MVP no-op resize" choice was a
  reasonable read of the brief but the failure mode (chrome-
  doesn't-redraw on no-op SIGWINCH) was knowable from htop's
  documented signal behavior; a one-paragraph "how does this
  actually trigger the structural repaint, not just the cell
  refresh?" check during design would have saved Webtest A and
  Alex a round.

* **@@Frontend** — solid through wave 1 + wave 2, fast on every
  task. Overlay removal, store/types residue, the bug-fix
  bundle, persistent-terminal client side, frontend-5 hash-key
  strip, frontend-6 hydrate-helper, frontend-7 per-tab session
  key, frontend-9 alt-key word motions, frontend-10 MCP env
  toggle + info + inject. Constructive feedback: frontend-6's
  hydration call landed too late and was caught by Webtest A's
  network trace; reading the lifecycle of `restoreLayout` +
  Svelte mount order before writing the patch would have caught
  the race up front. The systacean-8 fix (a Systacean lane Alex
  re-routed off Frontend's plate) was the right shape — fetch
  the blob first, pass to restoreLayout — and is worth carrying
  forward as Frontend's mental model for store-bootstrap fixes.

* **@@Backend** — narrow but disciplined. backend-1 (the agent
  HTTP surface removal + terminal MCP env exposure), backend-2
  (chan-desktop per-window session keys with focused client
  tests), and backend-3 (env namespace scope-down to CHAN_-only
  + `mcp_env` query param + doc sweep across six files). Every
  Backend commit shipped with verification trailers and a
  precise file list. Constructive feedback: backend-1 originally
  set CLAUDE_/CODEX_/GEMINI_ aliases without flagging the
  namespace-ownership question; better to ship the CHAN_-only
  shape first and add CLI aliases later if and only if a user
  needs them. Caught + fixed in backend-3 the same phase.

* **@@Webtest B** — clean parallel scenarios. backend-3 spot-
  check ahead of Webtest A's round-9 (so A could focus on
  frontend-10 + systacean-9), the post-frontend-2 hash-state
  probe, terminal MCP env probe, settings round-trip. Five
  filed follow-ups (3 already fixed by frontend-7 / frontend-5
  / backend-3, 2 parked). Constructive feedback: when the live
  service is shared with Webtest A, make the rebuild + relaunch
  ownership explicit in each progress note (A holds the
  service lifecycle per webtest-1.md, but B's reads can read
  the same PID; coordinate restarts).

* **@@Architect** (me) — orientation lag at the start. By the
  time I'd read the request, the process doc, and the
  workspace, @@Backend and @@Frontend had already self-
  dispatched and landed half of wave 1; I had to reconcile the
  journal mid-flight rather than dispatch from a clean
  baseline. Three full housekeeping rounds went into
  reconciling status drift. Constructive feedback for next
  phase: invest the first 5 minutes in skimming the process
  doc + an `ls -la <phase-dir>` before reading request.md, so
  the dispatch loop runs from minute one. The actual planning
  and the wave-2 architecture (Option 4 chan-native registry,
  the BUG-WT5-C diagnosis + fix shapes for systacean-8 and
  systacean-10, the commit groupings, the docs sweep) were
  fine. The tmux memo + Option 4 framing is reusable across
  future phases; carrying that pattern forward.

## Final delivery

Local `main` at phase close is **9 commits ahead of
`origin/main`**:

```
7da49f6 release: close phase 5 tasks
9ecb27d docs: refresh phase-5 boundary
790fd02 web: phase-5 frontend (overlay removal + persistent terminals + ux)
9e121d5 chan-server: prune agent surface + persistent terminal sessions
58fe80a chan-drive: drop assistant blobs + vcs-aware indexing
c748484 chan-llm: pare to MCP-only surface
02be09c web: add terminal tab controls
455c5df web: move terminal into workspace tabs
9c1ea91 web: add terminal overlay
```

The three terminal-trio commits (`9c1ea91`, `455c5df`,
`02be09c`) carry rewritten messages in the repo's canonical
style per [architect-3](./architect-3.md); their tree contents
are unchanged. The six new commits group phase-5 work by area
per [architect-2](./architect-2.md):

| Commit | Subject | Files | Lanes closed |
|--------|---------|-------|--------------|
| c748484 | chan-llm: pare to MCP-only surface | 17 | [systacean-1](./systacean-1.md) |
| 58fe80a | chan-drive: drop assistant blobs + vcs-aware indexing | 14 | [systacean-1](./systacean-1.md), [systacean-2](./systacean-2.md), [systacean-3](./systacean-3.md), [systacean-4](./systacean-4.md), [systacean-6](./systacean-6.md) |
| 9e121d5 | chan-server: prune agent surface + persistent terminal sessions | 21 | [backend-1](./backend-1.md), [backend-2](./backend-2.md), [backend-3](./backend-3.md), [systacean-2](./systacean-2.md), [systacean-3](./systacean-3.md), [systacean-5](./systacean-5.md), [systacean-9](./systacean-9.md), [systacean-10](./systacean-10.md) |
| 790fd02 | web: phase-5 frontend | ~40 | [frontend-1](./frontend-1.md)–[frontend-10](./frontend-10.md) (except frontend-6 superseded by [systacean-8](./systacean-8.md), inline), [systacean-8](./systacean-8.md) |
| 9ecb27d | docs: refresh phase-5 boundary | 3 | [architect-2](./architect-2.md) |
| 7da49f6 | release: close phase 5 tasks | 22 | this directory |

Pre-push gate on the final HEAD: `cargo fmt --check` ✓,
`cargo clippy --all-targets -- -D warnings` ✓,
`cargo build --no-default-features` ✓,
`cargo test --workspace` ✓, `npm --prefix web run check` ✓,
`npm --prefix web test -- --run` ✓ (16 files / 144 tests),
`npm --prefix web run build` ✓.

Working tree clean.

The `chan` release binary is staged under `target/release/`
from the parallel `make build-release` run that finished mid-
commit-sequence; `cd desktop && make build` is running to
produce `Chan.app` for the post-commit install lane
([systacean-7](./systacean-7.md)). The post-commit
click-around test service ([webtest-3](./webtest-3.md)) fires
on the same trigger.

Push to `origin/main` is the last action; @@Architect holds
on Alex's explicit go for that step.
