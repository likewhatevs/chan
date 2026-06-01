# Phase 7 - project hygiene, Hybrid panes, agent-orchestration substrate

Status: closed (shipped v0.10.1, then v0.11.0)
Span: 2026-05-18 to 2026-05-19 (estimate; see Duration)

Tags: #features #editor #terminal #desktop #docs #release

## Initial asks

Source: `raw/request.md`, two rounds, plus the design
notes in `raw/ui-exploration.md`.

Round 1 (maintenance). Project hygiene first: "create `./docs/journals`
and move our `phase-*` directories" there, normalize agent references to
`@@{name}`, and "create `./docs/agents` and create their Contact.md
files" with skill guides, so the dev log itself can be graphed. Then an
enhancement and bugfix wave: docked GitHub-style file-browser side panes,
a unified style toolbar, terminal-menu parity, `chan open <path>` from a
chan-spawned terminal, terminal activity indicators, MCP auto-discovery
for external agents, a pane-menu reorganization with per-pane focus color
and next/prev navigation, and about twenty bug fixes.

Round 2 (features): a notification system over the rich prompt
(WhatsApp-style chat bubbles and a survey reply style, "so that I can
pick from 1, 2, 3"), and programmatic agent spawning ("allow agents (in
our process, this will be only the @@Architect) to create new terminal
tabs in the current pane, name them, restart to pick up on the name, and
execute a command in there to start an agent"). The back half also drove
two design docs: moving Graph and File Browser from overlays to
first-class tabs, and the Hybrid binary-tree pane model with a
transactional Cmd+K Pane Mode.

## Team, profiles, and coordination

Cards under `../../agents/`, mapped via
[../../agents/README.md](../../agents/README.md).

```
handle        role this phase                          card
------------  --------------------------------------   -----------------
@@Architect   plan, dispatch, decisions, journal       architect.md
@@FullStack   merged Backend + Frontend; split         fullstack.md
              mid-phase into A and B                    (-> A + B)
@@FullStackA  FullStack lane A (faster cluster)        fullstack-a.md
@@FullStackB  FullStack lane B (cross-stack)           fullstack-b.md
@@Systacean   code quality, build, the release runway  systacean.md
@@WebtestA    test server + Chrome-MCP walkthrough     webtest-a.md
@@WebtestB    test server + Chrome-MCP walkthrough     webtest-b.md
@@Alex        host: product direction, commit auth     (human owner)
```

Coordination scheme: this is the first phase on the per-author model,
specified in `raw/process.md`. It replaced the phase-6
flat-task-file model with three changes:

1. One directory per author, each holding a `journal.md` plus numbered
   task files.
2. Append-only everything: journals and task files are never rewritten;
   corrections are new dated appends with a back-link, so the audit trail
   is load-bearing.
3. A typed event channel (the poke bus): participants append to
   `alex/event-{from}-{to}.md` pair files with an event type (poke,
   agent-recycle, permission, capacity); the file is a pointer and the
   work lives in the linked task file.

Routing differed from earlier flat phases: the architect cuts tasks to
@@Alex (who only appends notes in place, never cutting a reply file);
working agents never task @@Alex directly; permission events go straight
to @@Alex; and agent-recycle is the only context-reset mechanism. The
Round-2 bubble overlay being built is itself the eventual automation of
this same event bus. Mid-phase (2026-05-19), @@FullStack split into A and
B for two-wide parallelism, sweeping about 150 references across the
journals.

## Duration

Estimate: 2026-05-18 to 2026-05-19, two days. Basis: in-file dated
headers fall on 2026-05-18 (62) or 2026-05-19 (88), and the architect
timeline runs 2026-05-18 11:29 to 2026-05-19 23:55 BST. The git tail of
2026-05-20/21 is later phase-8 closeouts touching phase-7 files, not work
time.

## Highlights and lowlights

Highlights:
- Two releases in two days (v0.10.1, then v0.11.0). The 2026-05-19 run
  alone was around 106 commits on top of v0.10.1.
- Standing topic-level commit clearance was the throughput unlock: once
  @@Alex said "make intelligent decisions," the architect authorized
  commits inline and the bulk flowed without per-commit ping-pong and no
  surprise rollbacks.
- The Hybrid pane model was easier than expected because the layout was
  already a persisted binary tree.
- Webtest lanes caught real defects (a markdown-table render crash,
  per-tab schema gaps, a graph scope-reset regression).

Lowlights:
- Append-only is fragile under mechanical edits: an early rename plus sed
  sweep nuked narrative references and needed hand repair.
- The docs migration was reported done but had never committed (staging
  dropped during a release closeout) and had to be recovered.
- A push-before-read near-miss: a chrome change hit origin/main before a
  queued visual-pass HOLD poke was read.
- The notification gap was self-evident all phase: agents do not
  auto-detect event-file appends, so pokes queued until @@Alex woke each
  terminal. The Round-2 feature is exactly what closes this loop, but it
  was not dogfooding itself yet.
- Chrome MCP cannot drive the macOS Tauri WKWebView, so several
  desktop-shell items could not be browser-verified.

## Constructive feedback

For the team:
- Webtest lanes were the canonical defect record; do not let a code
  lane's self-validation substitute for the walkthrough verdict.
- Atomic-write discipline (temp plus rename for every event file) is the
  load-bearing contract for the watcher substrate; preserve "reads once"
  and "no self-loop writes back into a watched dir".

For the architect:
- The append-only model needs care under mechanical edits; anchor
  narrative text and verify staging before bulk operations.
- Resist amending an already-cut task; in doubt, cut a new task.

For @@Alex:
- The "make intelligent decisions" plus standing topic clearance was the
  single biggest throughput lever; the one cost was the commit-vs-push
  ambiguity for chrome-class work, now codified.
- The two-lane split paid off but introduced redistribution and
  cross-lane-absorption hazards; pull redistributed work from the queue
  tail, not the head.

## What shipped, tried, and undone

Shipped (Round 1, v0.10.1): window-scoped `chan open`, docked
file-browser side panes, tab drag-reorder and reopen-closed-tabs, file
writes moved off the Tokio workers, Find empty-state ladders, external
links via the system browser plus a unified style toolbar.

Shipped (Round 2 plus the Hybrid work, v0.11.0): the fsnotify watcher
with event ingestion and PTY dispatch, the bubble overlay with a
watcher-set dialog and survey UI, an event-reply atomic-write endpoint,
the HTTP agent control channel (spawn/name/execute/restart), a terminal
activity indicator, MCP auto-discovery for claude/codex/gemini, Graph and
File Browser as first-class tabs, the Hybrid binary-tree panes with
drag-detach and the Cmd+K Pane Mode, broadcast reduced to binary
in/out membership (the MUTE concept dropped), and the orchestration
skill under `docs/agents/orchestration`.

Tried and undone:
- A storage-key namespacing change was reverted as a no-op once a
  `Vary: Host` plus cache-control fix proved sufficient; a regression
  test pins it.
- A GraphPanel scope-reset validator became pointless when the dropdown
  that drove it was removed; the task stayed as an audit trail, never
  implemented.
- A mid-flight task amendment was reverted in spirit and re-cut as a new
  task once @@Alex flagged that even a queued task counts as in-flight.
- The final comprehensive webtest re-walk was skipped at close per
  @@Alex (per-task green gates plus unit coverage deemed sufficient).

Deferred to phase 8 (nine items in
`raw/next-phase-backlog.md`), with the
headline exit criterion: ship a notarized macOS DMG (plus signed
Windows/Linux) installable without Gatekeeper or SmartScreen friction.

## Raw material

Raw working material (per-author journals, task/request/roadmap files,
coordination logs) is preserved in git history under this phase's `raw/`
tree; it was removed from the working tree in the phase-15 docs cleanup.

The request file originally embedded fifteen screenshots of the reported
bugs; per the journals-wide image removal each is now a short text note in
the source request.
