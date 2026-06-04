# Phase 15 - Dashboard carousel, cs CLI, Team Work + survey, indexing

Status: closed
Span: 2026-05-30 to 2026-06-01 (four rounds over ~three days; round dates
      from the journal headers, version cuts from the git tags below)
Versions: v0.20.0 (round 1), v0.21.0 (round 2), v0.22.0 (round 3),
          v0.23.0 (round 4)
Tags: #features #bugfixes #reliability #indexing #search #editor #cli
      #desktop #release #docs

## Roadmap (the asks)

A long, four-round phase. @@Alex drove it as a stream of enhancement
requests plus the bugs they surfaced, re-scoping each round from the
prior round's carryover. The asks group into a few areas that recur
across rounds.

Dashboard + carousel (round 1):
- True two-face CSS card flip for Cmd+, (the existing keyframe raced
  focus and only fired once focus left the pane).
- A per-tab right-click slot picker (checkboxes, at least one on,
  default all-on) so unchecked carousel slots skip auto-rotation, plus a
  Settings (Cmd+,) entry.
- Per-slot front+back surfaces: About (license on the version row,
  theme-reactive screensaver preview), Workspace (chan-reports moved in
  from File Browser settings), Search (conditional legend, the index
  widget + semantic settings moved in from File Browser settings).
- Dashboard Search-slot directory inspector: add Show Directory / Graph
  from here / New Terminal, drop Upload.

Search cleanup (round 1, then deepened round 3/4):
- Remove the SCOPE selector, the SEARCH STATUS button, and the search
  status overlay; search is workspace-wide.
- Make BM25 match @@mentions, paths, and file.md (round 3).
- Decide what to do about semantic vectors being built every reindex but
  never queried (round 4 product question -> gate hybrid behind the
  existing semantic_enabled opt-in).

cs CLI / shell integration (round 1 spine, extended every round):
- A `chan shell` subcommand, with `argv[0]=="cs"` dispatch so a `cs`
  symlink works directly (open / graph / term / term-write / dashboard).
- Terminal tab groups ($CHAN_TAB_GROUP) so Cmd+Shift+I broadcast is
  group-scoped (round 1).
- Rename `cs term` -> `cs terminal`, prefix-match subcommands,
  `cs terminal restart`, `cs search`, `cs dashboard --carousel-off`
  (round 2).
- A `chan-shell` crate so both `chan` and `chan-desktop` share the cs
  client; remove the in-terminal `chan open`; a per-agent submit-encoding
  map (round 3).
- `cs terminal team new|load --script` as the CLI equivalent of the
  Cmd+P team dialog, with server-side lead-first spawn (round 4).

Team Work + survey (round 3):
- Move the team config from a `/tmp` path into the workspace under a
  user-chosen `{team-name}/` directory.
- Rebuild the survey for real (overlay + reply round-trip + `[F]`
  followup file), replacing the static stub; expose it as a synchronous
  `cs terminal survey` control-socket call that blocks for the reply.

Indexing (IDX, the hardest thread, rounds 1-4):
- Stop a large-workspace synchronous embed pass from wedging boot;
  preflight should gate on BM25-ready and embed in the background.
- Cure the Cmd+R reload hang (round 3), fix the embed-chip clobber and
  in-flush freeze, and make the indexing spine pulse orange during the
  embed sweep (round 4).

Editor + terminal UX (rounds 2-4):
- Clickable terminal URLs; Shift+Enter newline fallback with an agent
  running; Cmd+R remap to Ctrl+Shift+R off macOS so bash reverse-search
  survives; conceal re-decorate on tab-switch.
- `[[` completion writes relative markdown links on disk (not wiki
  links); heading `#` / block `^` anchors; click-to-place-caret anywhere
  on a row; relative-link pills.

Desktop + release engineering (round 4):
- Build all Linux chan-desktop variants (ubuntu, fedora, arch) plus the
  gateway from a macOS host via sdme/lima; static musl `chan` CLI;
  multi-arch desktop CI matrix.
- Native macOS Export-to-PDF that paginates and honors `@pagebreak`; the
  desktop window-close crash; a unified orange enso favicon.

Docs hygiene (rounds 3-4):
- Collapse the raw `docs/journals/` tree into essence phase READMEs,
  drop the raw, fix graph ghost nodes, tag outcomes.

## Rounds and waves

Round 1 (v0.20.0). The opener: @@Alex's multi-request roadmap (Dashboard
carousel redesign, Search cleanup, the `chan shell`/`cs` surface,
terminal groups, plus three bugs). Dispatched across lanes A-D. The cut
shipped as v0.20.0, but an audit found several Lane A Dashboard items had
been dropped without being journaled (A3 slot menu, A4 inspector
actions, A6 license placement, A7 screensaver preview), which became the
opening work of round 2.

Round 2 (v0.21.0). Roles fixed for the rest of the phase: @@LaneA =
@@Architect, @@LaneB = Dashboard/frontend, @@LaneC = search/indexing,
@@LaneD = terminal/cs/desktop/Team-Work. Part 1 finished the dropped
Lane A items; part 2 took new bugs and refinements. Shipped: the four
Dashboard fixes, terminal link clicks, the Shift+Enter LF fallback
(real-agent verified), the Ctrl+R reload remap, the cs rename +
`cs search` + `cs dashboard --carousel-off`, Team Work self-start +
groups + `cs terminal write --submit` (POKE-2.2), `chan open` OS
file-association, and IDX (preflight-on-BM25-ready + background embed +
a 2000-file cap + a 4097/4096 display clamp). IDX was the round's
hardest, highest-impact bug.

Round 3 (v0.22.0). Six backlog themes across four lanes in three waves
with a refresh handshake at each barrier. Wave 1 cured the
session-crashing RELOAD-HANG (an incremental `Reindexing` was re-locking
the boot overlay; the fix maps it to `Done` like `Idle`, only cold
`Building` locks). Other themes: Team Work moved into the workspace +
survey rebuilt for real; the `chan-shell` crate + desktop `cs` argv0
dispatch + `chan open` removal + per-agent submit map; relative-markdown
links + heading/block anchors; BM25 subtoken split for mentions/paths;
the embed-chip-clobber decoupled-signal fix; essence-only phase READMEs
with the raw dropped (phase 8 deferred because `docs/agents/` cited it).

Round 4 (v0.23.0). Four lanes, four waves; ships v0.23.0. The long pole
was @@LaneB building every Linux chan-desktop variant + the gateway from
macOS via sdme, plus a static musl `chan` and a multi-arch desktop CI
matrix. @@LaneC built `cs terminal team new|load --script` (script-first,
so the public cs surface had to express the whole bootstrap) + a
server-side spawn. @@LaneD wired semantic search behind `semantic_enabled`
and finished the phase-8 docs cleanup. Wave 3 added the desktop
window-close crash fix and native macOS Export-to-PDF (reworked to the
print pipeline so it paginates + honors `@pagebreak`). Wave 4 was a
3-fix indexing/graph cluster: the spine pulses orange during the embed
sweep, a tokei log-spam filter, and `.txt` is no longer a graph document.
The architect tab CRASHED mid-round during a fedora build; recovery was
empirical and complete because all durable state lived on disk and in the
VM.

## Team and coordination

@@LaneA through @@LaneD plus @@Host (@@Alex). See `../agents/README.md`
for the roster. From round 2 on, @@LaneA was also @@Architect.

The phase ran the architect-as-hub model that became the project's
standard: @@Architect is the only agent that talks to @@Host, sets the
wave, sequences merges to main (locally), arbitrates shared files, and
runs a refresh handshake at each wave barrier. Workers own disjoint file
sets and never edit another lane's file; cross-lane seams (C<->D survey
transport, A<->B search/graph) are arbitrated at the barrier. A wave
barrier is: each lane drives its items to gated-green + locally merged,
writes its journal, pokes @@Architect; @@Architect verifies all four and
sequences the merges; @@Host refreshes (restarts) every agent with a
one-line bootstrap pointer. A refresh is a clean-context reset, so ALL
durable state lives in the docs and on-disk task/journal/code, never in
an agent's head.

Two coordination mechanisms this phase used and improved:

- The lean poke bus: pokes are one-line pointers ("poke from X: read
  <path>"), not fat context; the context lives in on-disk task/plan/
  journal files. Every poke ends with the Meta+Enter submit chord
  `\x1b[27;9;13~` (a bare `\r`/`\n` parks the poke unsubmitted). Round 2
  validated the chord (CK-SUBMIT) and then productized it as
  `cs terminal write --submit` (POKE-2.2), so the round closed its own
  coordination gap.
- The synchronous `cs terminal survey` control-socket call: an agent's
  CLI blocks until @@Host picks an option, then prints the option (or a
  new followup file path) to stdout. The coordination docs stayed
  untracked during each round; @@Architect committed the whole
  `docs/journals/phase-15/` tree as one `docs(phase-15)` commit at close.

Commit hygiene in the shared worktree: every shared-file commit was
guarded-atomic (chained `git add <paths>` + `git diff --staged --stat`
audit + `git commit`, verified with `git show --stat HEAD` after), moving
to pathspec-only commits after the round-3 Wave-1 collision (one lane's
blanket stage swept another's uncommitted crate).

## What shipped, tried, and undone

Shipped (durable, by area):
- Dashboard: true CSS card flip; per-tab slot picker + Settings entry;
  per-slot About/Workspace/Search backs; the Search-slot inspector
  actions (v0.20.0/v0.21.0).
- Search: SCOPE selector / status button / status overlay removed; BM25
  subtoken split for @@mentions/paths/file.md (v0.22.0); semantic hybrid
  gated behind `semantic_enabled` (v0.23.0).
- cs CLI: `chan shell` / `cs` with argv0 dispatch; terminal tab groups +
  group-scoped broadcast; the rename to `cs terminal`; `cs search`;
  `cs dashboard --carousel-off`; the `chan-shell` crate shared by
  `chan` + `chan-desktop`; `cs terminal write --submit`;
  `cs terminal team new|load --script` + server-side spawn.
- Team Work: config moved into the workspace; survey rebuilt for real
  (overlay + reply round-trip + `[F]` followup); team self-start;
  per-agent submit map (v0.21.0/v0.22.0).
- Indexing: preflight gates on BM25-ready + background embed + file cap;
  the RELOAD-HANG cure; chip-clobber + in-flush-freeze fixes; the
  orange indexing-spine pulse during the embed sweep; `.txt` is no
  longer a graph document.
- Editor/terminal: clickable terminal URLs; Shift+Enter LF fallback;
  Ctrl+R -> Ctrl+Shift+R off macOS; conceal re-decorate on tab-switch;
  relative-markdown `[[` links + heading/block anchors; relative-link
  pills.
- Desktop/release: Linux chan-desktop ubuntu/fedora/arch builds + gateway
  via sdme; static musl `chan`; multi-arch desktop CI matrix; the
  window-close crash fix; native macOS print-pipeline Export-to-PDF; the
  unified orange enso favicon (v0.23.0).
- Docs: essence phase READMEs, raw dropped; graph ghost-node + EDGES-PK
  fixes.

Tried then corrected (course changes):
- IDX: several confabulated source-read theories were overturned at once
  by a live `sample` of the wedged process (777/777 in candle BERT
  matmul); the synchronous-embed root cause was empirical, not a
  lock/loop. The fix gated boot on BM25-ready rather than chasing the
  symptom.
- TEAM-SELFSTART: the blessed command+env-override fix was never the
  real bug; only the real-agent smoke exposed the reattach gap.
- macOS PDF: the first version used WKWebView `createPDF` (a screen
  capture that clips long notes and ignores `@pagebreak`); reworked to
  the print pipeline. The wrong API shipped one commit before the right
  one.
- EDGES-PK: dropped a careful migration for a clean v1-schema change once
  @@Host confirmed a fresh `~/.chan` is fine (the no-back-compat norm).

Deferred / not done (recorded risks):
- Content "magic" file-type detection + a "pending indexing" state
  (round 5): a hand-rolled UTF-8/NUL sniff that touches the editable-text
  correctness gate.
- arch chan-desktop AppImage could not be fully validated (linuxdeploy
  fails even with NO_STRIP=1; deb+rpm validated).
- cs-on-AppImage argv0 dispatch; the SPA-visible CLI team spawn; the
  Metal/GPU re-enable (candle Metal hangs in `waitUntilCompleted`, so
  embeddings stay CPU); the `[[` stuck-Indexing-bubble smoke.
- A hard-dated CI risk: the v0.23.0 release run flagged GitHub Actions
  Node 20 deprecation; `checkout@v4` / `setup-node@v4` /
  `upload-artifact@v4` run on Node 20, forced to Node 24 on 2026-06-16.

## Retrospective

The learning payload. The round-2/3/4 retrospectives recorded these in
detail; the durable lessons follow.

Highlights:
- Smoke-first repeatedly caught the WRONG fix. TEAM-SELFSTART, the IDX
  wedge, and the round-4 spine pulse all looked done under static gates
  but were only validated (or corrected) by a real-agent / live-server
  smoke. Live-browser-smoking the spine pulse (watching 4 dirs go orange
  during a real 359/360 embed sweep) is the gold standard for
  runtime-reactive verification that static gates miss.
- Ground-truth discipline under tooling corruption. @@LaneC's reads and
  bash stdout fabricated content mid-round 2; anchoring on git/sha/
  compiler-error ground truth and refusing to blind-commit caught it
  every time, and the build error itself reconciled a lane-vs-lane
  contradiction. The same discipline caught a confabulated favicon
  deletion and a misattributed "LaneB completed" later in the phase.
- The architect-as-hub + wave model held cleanly: disjoint file
  ownership, refresh handshakes at each barrier, and pathspec-only
  commits kept the shared worktree uncontaminated across ~16 commits
  from four lanes plus the architect.
- The round literally built its own coordination tool: directed-wake via
  the submit chord (CK-SUBMIT) was validated then shipped as
  `cs terminal write --submit`.
- Empirical crash recovery worked: the round-4 architect tab died
  mid-build, and because all durable state was on disk + the VM, recovery
  was complete. The doc-as-source-of-truth model earned its keep.
- De-risk-first on the riskiest unknowns: @@LaneB proved chan-desktop via
  sdme and musl static linking (`ldd` "not a dynamic executable", runs on
  the VM) locally before touching CI.

Lowlights / contention:
- Tooling flakiness was the dominant drag in round 2: output truncation +
  confabulation caused two @@LaneC incidents, a tab recycle, and several
  stalls that had nothing to do with the work.
- Poke-delivery stacking burdened @@Host before CK-SUBMIT: pre-`\r`-recipe
  pokes parked un-submitted in the architect's terminal and @@Host
  hand-Entered them, causing the "waiting on you" stalls.
- The shared Chrome MCP tab group is a single-driver resource exactly
  like the worktree, learned mid-round 3 when two lanes collided on it.
  Worse, browser `navigate` was denied to multiple lanes, so two editor
  smokes shipped source-tested + gated-green but empirically-unverified.
- The round-3 Wave-1 commit collision (a blanket stage swept another
  lane's uncommitted crate) cost a recovery cycle before pathspec-only
  commits were ratified.
- Two false positives that ground-truth caught: a truncated `make
  pre-push` reported a false exit-0 (the gate never ran; caught by
  reading the log, not the exit code), and a stale local `web/dist`
  produced a phantom `--carousel-off` "bug" (CLAUDE.md already mandates
  rebuilding web before cargo).
- Multi-agent core contention: 4+ agents plus concurrent cargo builds on
  limited cores produced the in-flush chip freeze and slow/locked builds.

Constructive feedback / lessons that generalize:
- Stale-binary and stale-bundle false positives are a recurring class.
  Rebuild `web/dist` BEFORE `cargo build` and grep the SERVED bundle
  before calling a frontend-touching CLI flag broken; for a re-walk of a
  failed empirical test, explicitly rebuild and verify provenance.
- Never pipe the command whose exit code you are verifying (`cargo ... |
  tail` reports tail's 0 and hides cargo's failure); run bare and capture
  `$?`, or set pipefail.
- Gate-blind wire surfaces need runtime smokes, not just a green build:
  cross-crate clap derive + serde tags must stay byte-identical or every
  cs command breaks at runtime. @@LaneC's correct REFUSAL to half-wire
  the SPA `--command/--env` seam avoided exactly this trap; verifying
  desktop `cs` against the REAL desktop control socket was the right
  rigor.
- The pre-release no-back-compat norm is a genuine simplifier: drop old
  shapes outright, never escalate a back-compat question. The EDGES-PK
  clean v1-schema change is the model.
- Completion claims should travel as evidence (a journal entry, a
  commit), not hearsay; the misattribution relay made the case.
- Scope: the rounds (especially round 2, where IDX alone was arguably a
  round's worth) ran very large, which is where the flakiness compounds;
  a tighter per-round scope and an earlier "here is everything queued"
  checkpoint would reduce architect context-thrash. The architect's own
  recurring miss was deep-diving code before surfacing a depth/risk
  decision to @@Host, and not always applying the ground-truth-verify
  habit to its OWN alarms before raising them. A standing
  browser-access decision at round start (who may navigate, on which
  ports) would have avoided the late verification scramble.

## Notes

Terminology drift a new reader needs:
- "drive" means the chan workspace directory (later renamed to
  chan-workspace); these journals predate that rename, so "drive" and
  "workspace" are used interchangeably. Note "drive" also has unrelated
  meanings elsewhere (cloud products, tunnel domain); here it is the
  on-disk directory only.
- "Rich prompt" / "Rich Prompt" was the deleted Team Work compose widget;
  its survey surface was rebuilt this phase as Team Work + the survey
  overlay.
- "carrousel" in @@Alex's round-1 roadmap is the carousel; "term" /
  "term-write" were renamed to `cs terminal` / `cs terminal write` in
  round 2.
- Outcome hashtags carried from the source: #features #bugfixes
  #reliability #indexing #search #editor #cli #desktop #release #docs.

The raw working material (per-round plans, per-lane task/journal files,
event channels, status files, the survey/search/group interface
contracts, and the load-bearing screenshots) lived in git history under
`docs/journals/phase-15/`; that tree was removed during the phase docs
cleanup. The bug screenshots there were text-described inline in the
round-2 plan: [a stuck "reindexing Drafts/untitled/draft.md" status bar
that never cleared] and [the preflight overlay frozen at "Build search
index / working..." after a Cmd+R reload on a busy seeded clone of this
repo].
