# Phase 7 - project hygiene, Hybrid panes, agent-orchestration substrate

Status: closed
Span: 2026-05-18 to 2026-05-19 (basis: in-file dated headers + architect
      timeline 11:29 BST 2026-05-18 to 23:55 BST 2026-05-19; git commits
      after 2026-05-20 touching phase-7 files are phase-8 closeouts)
Versions: v0.10.1, v0.11.0
Tags: #features #editor #terminal #desktop #docs #release

## Roadmap (the asks)

Round 1 (maintenance and enhancements). @@Alex opened with project
hygiene: create `docs/journals/`, migrate the phase directories there,
normalize all agent references to `@@{name}`, and create `docs/agents/`
with per-agent Contact.md cards and skill guides so the development log
itself could be graphed. The enhancement and bugfix tranche that followed
covered: docked GitHub-style file-browser side panes, a unified style
toolbar, terminal-menu parity, `chan open <path>` from a chan-spawned
terminal, terminal activity indicators, MCP auto-discovery for external
agents, a pane-menu reorganization with per-pane focus color and next/prev
navigation, and roughly twenty bug fixes.

Round 2 (features and design). Two feature areas: a notification system
over the rich prompt (WhatsApp-style chat bubbles plus a survey-reply
style letting the host pick from numbered options), and programmatic agent
spawning (allow the @@Architect to create new terminal tabs in the current
pane, name them, restart to pick up on the name, and execute a command to
start an agent). The back half produced two design docs: moving Graph and
File Browser from overlays to first-class tabs, and the Hybrid binary-tree
pane model with a transactional Cmd+K Pane Mode.

## Rounds and waves

Round 1 ran as a single coordinated wave across @@FullStack (later split
into @@FullStackA and @@FullStackB), @@Systacean, and two webtest lanes.
The hygiene work (docs migration, agent cards) was a prerequisite wave;
the enhancement and bugfix tranche ran in parallel across the code and
webtest lanes once the hygiene work cleared. Round 1 closed with v0.10.1.

Round 2 followed directly. The notification overlay (bubble rendering,
survey UI, watcher-backed endpoint) and the agent control channel
(spawn/name/execute/restart over HTTP) were the two main workstreams.
Hybrid pane work (binary-tree layout, drag-detach, Cmd+K Pane Mode) ran
alongside. Round 2 closed with v0.11.0, which included approximately 106
commits on top of v0.10.1 in a single day (2026-05-19).

## Team and coordination

Agent roster: see ../agents/README.md. Handles active this phase:

  @@Architect      plan, dispatch, decisions, per-phase journal
  @@FullStack      merged backend+frontend lane; split into A and B
                   mid-phase for two-wide parallelism
  @@FullStackA     lane A (faster cluster of backend/frontend tasks)
  @@FullStackB     lane B (cross-stack work, slower but broader)
  @@Systacean      code quality, build system, release runway
  @@WebtestA       test server + Chrome-MCP walkthrough
  @@WebtestB       test server + Chrome-MCP walkthrough
  @@Alex           host: product direction, commit authorization

This is the first phase on the per-author coordination model, specified in
a process document written at phase start. It replaced the phase-6
flat-task-file model with three structural changes:

1. One directory per author, each holding a `journal.md` plus numbered
   task files. Journals and task files are append-only: corrections are
   new dated appends with a back-link, not in-place rewrites, so the audit
   trail is load-bearing.

2. A typed event channel (the poke bus): participants append to
   `alex/event-{from}-{to}.md` pair files with an event type (poke,
   agent-recycle, permission, capacity). The file is a pointer; the work
   lives in the linked task file.

3. Routing rules: the architect cuts tasks to @@Alex (who appends notes in
   place, never cutting a reply file); working agents never task @@Alex
   directly; permission events go straight to @@Alex; agent-recycle is the
   only context-reset mechanism.

The Round-2 notification overlay being built during this phase is the
eventual UI automation of this same event bus. It was not dogfooding
itself yet during phase 7: agents did not auto-detect event-file appends,
so pokes queued until @@Alex manually woke each terminal.

The @@FullStack split happened mid-phase on 2026-05-19 and required
sweeping roughly 150 cross-references across the journals to rename the
handle. That sweep exposed the fragility of append-only docs under
mechanical bulk edits.

## What shipped, tried, and undone

Shipped (Round 1, v0.10.1):
- Window-scoped `chan open` command from a chan-spawned terminal.
- Docked file-browser side panes (GitHub-style).
- Tab drag-reorder and reopen-closed-tabs.
- File writes moved off the Tokio workers onto a dedicated write path.
- Find panel empty-state improvements.
- External links via the system browser, plus a unified style toolbar.

Shipped (Round 2 + Hybrid work, v0.11.0):
- fsnotify watcher with event ingestion and PTY dispatch.
- Bubble overlay with watcher-set dialog and survey UI (numbered reply).
- Event-reply atomic-write endpoint.
- HTTP agent control channel (spawn/name/execute/restart).
- Terminal activity indicator.
- MCP auto-discovery for claude/codex/gemini agents.
- Graph and File Browser promoted to first-class tabs.
- Hybrid binary-tree panes with drag-detach and Cmd+K Pane Mode.
- Broadcast membership simplified to binary in/out (the MUTE concept
  dropped).
- Orchestration skill written and committed under docs/agents/.
- Per-agent Contact.md cards and skill guides in docs/agents/.

Tried and undone:
- A storage-key namespacing change was reverted as a no-op after a
  `Vary: Host` plus cache-control fix proved sufficient; a regression
  test pins the fix.
- A GraphPanel scope-reset validator was never implemented: the dropdown
  that drove the validation requirement was removed before the task ran.
  The task was kept as an audit trail entry.
- A mid-flight task amendment (new requirement appended to an in-progress
  task) was reverted in spirit and re-cut as a new task after @@Alex
  flagged that queued tasks count as in-flight.
- The final comprehensive webtest re-walk was skipped at close at @@Alex's
  direction; per-task green gates plus unit coverage were deemed
  sufficient.

Deferred to phase 8: nine items from the backlog, headlined by the
requirement to ship a notarized macOS DMG (plus signed Windows and Linux
installers) installable without Gatekeeper or SmartScreen friction.

## Retrospective

Highlights:
- Two releases in two days (v0.10.1, v0.11.0) with roughly 106 commits
  on the second day alone. The throughput came from standing topic-level
  commit clearance: once @@Alex granted "make intelligent decisions," the
  architect authorized commits inline and the bulk flowed without
  per-commit ping-pong. No surprise rollbacks resulted.
- The Hybrid pane model shipped faster than expected because the layout
  was already a persisted binary tree; the structural work was mostly
  already in place.
- Webtest lanes caught real defects that code-lane self-validation missed:
  a markdown-table render crash, per-tab schema gaps, and a graph
  scope-reset regression.
- The per-author directory plus append-only event-channel model gave the
  phase a coherent audit trail and became the baseline for all later
  phases.

Lowlights and contention:
- Append-only discipline is fragile under mechanical bulk edits. An early
  rename-plus-sed sweep nuked narrative references and required hand
  repair. The lesson: anchor narrative text and verify staging before bulk
  operations.
- The docs migration was reported done but had never been committed.
  Staging was dropped during a release closeout and the work had to be
  recovered. Lesson: verify the staged state separately from the work
  state.
- A push-before-read near-miss: a Chrome-class change hit origin/main
  before a queued visual-pass HOLD poke was read. This codified the
  commit-versus-push distinction for browser-visible work; checking event
  channels before pushing chrome-class work is now a standing rule.
- The notification gap was visible all phase: because agents do not
  auto-detect event-file appends, pokes queued silently until @@Alex woke
  each terminal. The Round-2 feature was designed to close this loop but
  was not yet operating on its own bus.
- Chrome MCP cannot drive the macOS Tauri WKWebView, so several
  desktop-shell items were never browser-verified. This boundary was
  discovered empirically during phase 7 and documented for future rounds.
- The @@FullStack split mid-phase introduced redistribution and
  cross-lane-absorption risk. The specific hazard: pulling redistributed
  work from the queue head (where the lane is already working) rather than
  the tail (safe). This was caught and codified.

Lessons for future agents:
- Webtest lanes are the canonical defect record; a code lane's
  self-validation does not substitute for the walkthrough verdict.
- The append-only model is the load-bearing contract for the event bus.
  Do not rewrite event files in place; corrections are new dated appends
  with a back-link.
- Atomic-write discipline (temp-plus-rename) for event files preserves
  "reads once" and "no self-loop writes back into a watched directory."
- When redistributing tasks across lanes, take from the queue tail, not
  the head; the current-head item may already be in progress.
- Standing topic-level commit clearance is a significant throughput lever.
  The cost is that commit authorization must not be read as push
  authorization for chrome-class work; those are distinct gates.

## Notes

Terminology: "rich prompt" in phase-7 sources refers to what later phases
call "Team Work." The bubble overlay UI is the same feature; the rename
happened after phase 7. "chan-drive" (pre-phase-12) and "chan-folder"
appear in some phase-7 task files where "chan-workspace" is the current
name.

Raw working material (per-author journals, task files, request files,
roadmap and coordination logs) is preserved in git history under
`docs/journals/phase-7/`; that tree was removed from the working tree in
the phase-15 docs cleanup.
