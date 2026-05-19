# Working on chan — Phase 7 process

Alex is the host. Agents come to Alex for product direction,
permission, or decisions that can't be made from the written
plan. Phase-wide coordination flows through @@Architect, who
brokers between Alex and the working agents.

This process replaces the phase-6 process in three main ways:

1. Each working agent (and Alex) owns a directory in this
   phase.
2. **@@Architect ↔ Alex is poke-driven.** @@Architect cuts
   tasks to Alex in `alex/{task}-{n}.md`; Alex does not cut
   tasks back. @@Architect pokes Alex to read, Alex appends
   notes to the same task file, then pokes @@Architect to carry
   on.
3. **Inter-agent pokes route through Alex.** When a working
   agent finishes a task that @@Architect should react to, the
   agent drops `alex/poke-{from}-{to}.md` that links to their
   latest update; Alex then pokes @@Architect.

## Roster

| Tag             | Profile                                       |
|-----------------|-----------------------------------------------|
| @@Architect     | Plan, dispatch, decisions, phase journal.     |
| @@FullStack     | Backend + Frontend merged. Owns HTTP, axum    |
|                 | routes, Svelte frontend, editor, terminal.    |
| @@Systacean     | Syseng + Rustacean. Owner of code quality,    |
|                 | build, CI, dependencies, toolchain.           |
| @@WebtestA      | Test-server + manual web walkthrough lane A.  |
| @@WebtestB      | Test-server + manual web walkthrough lane B.  |

Agent skill anchors and contacts live under
[`../../agents/`](../../agents/). Each agent has a top-
level `<name>.md` card (handle, profile, skill links) and,
for active rosters, a `<name>/skills/` subdirectory copied
from the contributor's skill library.

## Directory layout

```
phase-7/
├── process.md              (this file)
├── request.md              (Alex's source-of-truth ask)
├── summary.md              (@@Architect writes at phase close)
├── alex/
│   ├── journal.md
│   ├── {task}-{n}.md         (@@Architect-cut tasks for Alex)
│   └── event-{from}-{to}.md  (per-pair append-only event log)
├── architect/
│   ├── journal.md
│   └── architect-{task}.md
├── fullstack/
│   ├── journal.md
│   └── fullstack-{task}.md
├── systacean/
│   ├── journal.md
│   └── systacean-{task}.md
├── webtest-a/
│   ├── journal.md
│   └── webtest-a-{task}.md
└── webtest-b/
    ├── journal.md
    └── webtest-b-{task}.md
```

Names use lowercase with hyphens. Author directories drop the
`@@` prefix (e.g., `@@FullStack` → `fullstack/`). File suffixes
stay numeric (`{owner}-{n}.md`); references in prose use the
`@@{name}` form.

## Author journals

Each member maintains `{name}/journal.md`. The file starts with:

```markdown
# {Title}

Author: {name}
Date: 2026-MM-DD
```

A journal entry is a dated section under the author's title; the
journal grows **append-only**. Authors do not edit their own
prior entries; they add a new entry with the correction +
back-link if a previous entry was wrong.

`@@Architect`'s journal stays the canonical phase-wide journal:
plan summary, request checklist, capacity proposal, dispatch
table, decisions log, extended-requests trail. Other authors'
journals focus on their own work and observations.

## Task files

Task files live under each author's directory and follow the
`{name}/{name}-{task}.md` pattern. They are **append-only
journals**, not editable documents:

* Each new note appends a dated section at the bottom.
* Existing sections do not get rewritten. Corrections are new
  appended sections.
* Status is conveyed by the latest appended section, not by
  rewriting an old "Status:" header.

This makes the audit trail load-bearing: anyone reading a task
file sees the full progression of decisions and findings, not
just the latest state.

### Required fields in the first append

* Owner (@@mention).
* Goal.
* Relevant relative markdown links.
* Initial acceptance criteria.

### Subsequent appends

* Progress note + date.
* Review feedback.
* Hardening / specialist hand-off.
* Commit readiness, when the task is ready to land.

## Communication: @@Architect ↔ Alex

@@Architect requests things from Alex by creating
`alex/{task}-{n}.md` — a fresh, numbered task file owned by
Alex. The first append states:

* The decision or input @@Architect needs.
* Options under consideration with @@Architect's recommendation
  flagged.
* The downstream tasks gated on the answer.

@@Architect then files an **event** (see below) and waits.
Alex reads the task file, **appends** notes to the same
`alex/{task}-{n}.md`, then fires a return event to
@@Architect to carry on. Alex never cuts a separate file in
reply; everything lives in the original task file.

Once a decision lands, @@Architect mirrors it into the journal
under the "Decisions log" section so future readers can find it
without scanning every alex/ task file.

## Communication: working agents ↔ Alex (via @@Architect)

Working agents (@@FullStack, @@Systacean, @@WebtestA,
@@WebtestB) do not cut tasks directly to Alex. When they finish
a piece of work that @@Architect needs to react to, they:

1. Update their own `{name}/{name}-{task}.md` with the
   completion / blocker note.
2. File an event `alex/event-{from}-{architect}.md` (type
   `poke`) with a one-liner + a link to the task file update.
3. Stop and wait.

Alex reads the event when convenient and forwards a poke to
@@Architect. @@Architect picks the work up from the linked task
file.

A small set of asks may go **direct to Alex** instead of via
@@Architect — see "Event types" below.

## Events (the poke channel)

Events live at `alex/event-{from}-{to}.md`. Each event file is
an **append-only** log of pings between two participants. The
file is a pure pointer + reason; the underlying work / decision
content lives in the linked task file.

Why "events" and not "pokes": pokes are one type of event.
Future tooling (fsnotify on this directory) will watch event
files and trigger automation; the type tag is what drives
dispatch.

### First append (file header)

```markdown
# event-{from}-{to}.md

From: @@{From}
To: @@{To}
Date: 2026-MM-DD
```

Then one section per event, dated, in append order.

### Per-event entry

```markdown
## 2026-MM-DD HH:MM — {type}

(one or two lines + link to the relevant artifact)
```

### Event types

| Type            | From → To             | What it means                                                                 |
|-----------------|-----------------------|-------------------------------------------------------------------------------|
| `poke`          | anyone → anyone       | "Have a look at X." Pure pointer, no content beyond the link.                |
| `agent-recycle` | @@Architect → @@Alex  | "Agent @@X is done; recycle the context, hand over via journal." Names the   |
|                 |                       | target agent and links to the handover journal.                              |
| `permission`    | any agent → @@Alex    | "I need a permission from you" — terminal exec, browser launch, etc.        |
|                 |                       | Direct to @@Alex (skips @@Architect) since these are interactive.            |
| `capacity`      | (future)              | Reserved for future capacity-request automation.                             |

`poke` is the default; the others are explicit type tags.

### Routing rule

* Most events from working agents flow to @@Architect via
  `alex/event-{from}-architect.md` (with @@Alex as the human
  doing the forwarding poke).
* From @@Architect, events flow to @@Alex via
  `alex/event-architect-alex.md`.
* Direct-to-@@Alex events are allowed when the ask is
  inherently interactive (terminal / browser permission). Tag
  as `type: permission`.

### Approving a `permission` event

A permission event is "approved in writing" when an append on
the same event file matches one of:

1. @@Alex appends a section beginning with the word
   `approved` (any heading shape; the audit cue is the word).
2. @@Architect appends a section titled
   `approved (transcribed by @@Architect)` with a chat
   timestamp, e.g.
   `## 2026-05-18 12:15 BST - approved (transcribed by @@Architect)`,
   then the body notes "@@Alex approved this verbally in
   chat" plus the scope covered. Use this when @@Alex
   approved in chat and the agent is waiting on a written
   record.

The agent watches *its own* permission event file and may
proceed as soon as either form is appended. The scope of the
approval is whatever the original permission request
described; anything beyond requires a fresh `permission`
event.

### Why one file per from/to pair

Per-pair files keep the event log linear and easy for an
fsnotify watcher to follow. Old entries accumulate at the top;
the latest event is always the last one appended.

## Agent-recycle protocol

@@Architect signals an agent recycle when:

* The agent's task envelope (round / phase scope) is done.
* The agent's context window is heavy enough that a fresh
  session will execute better than the current one.
* A profile change is happening (e.g., closing @@FullStack on a
  feature and re-opening for a different feature).

Procedure:

1. The outgoing agent writes a handover entry to its own
   journal: what it did, what it left unfinished, which task
   files are open, which decisions are pending.
2. @@Architect files
   `alex/event-architect-alex.md` with type
   `agent-recycle`, naming the agent and linking to the
   handover journal entry.
3. @@Alex closes the current agent session and opens a fresh
   one with the same profile; the fresh agent reads its
   journal first, then resumes.

The recycle event is the only mechanism for context resets.
Agents do not unilaterally request recycles; they ask
@@Architect.

## Capacity planning

Same shape as phase 6: @@Architect drafts a proposal in their
journal, Alex validates the actual resource assignment by
appending to `alex/capacity-{n}.md`. Profiles, slots, expected
handoffs, and known capability assumptions all recorded there.

## Review and hardening

Specialists review work that touches their area:

* @@FullStack owns frontend UX, state, interaction, plus
  backend HTTP and filesystem semantics.
* @@Systacean owns Rust code quality, build, CI, dependencies,
  toolchain.
* @@WebtestA / @@WebtestB validate the running web experience.

When a specialist sign-off is owed, the requesting author
appends a "Specialist review requested" note to their task file
and the specialist appends their findings.

## Test server URL hand-off

At the end of each round, @@WebtestA and @@WebtestB pick one
running test server URL for Alex to click around. They send
this through:

1. A task append at `{name}/{name}-{task}.md` describing the
   server (drive path, URL with bearer token, what to look at).
2. An event at `alex/event-{webtest}-architect.md` (type
   `poke`).
3. @@Architect then files
   `alex/event-architect-alex.md` (type `poke`) with the URL
   forwarded.

Webtest may roll the hand-off into a later round if Alex isn't
replying; the URL is never a blocker for downstream progress.

## Teardown

Before phase close, @@Architect appends teardown checklists for
each active author. Teardown notes (services stopped, temp
files removed, branches cleaned) live in each author's own
task file or journal.

## Commit coordination

Same as phase 6: commits are coordinated through task files.
Before committing, the owner appends a "Commit readiness" note
with files changed, tests run, review/hardening performed,
known risks, and proposed commit message. @@Architect appends
the final commit-grouping plan to
`architect/architect-{commit-plan}.md`.

## What carries over from phase 6

* Drive boundary contract: filesystem work routes through
  `chan_drive::Drive`.
* Single binary, no runtime deps.
* Local-first by default, opt-in tunnel.
* MCP server only, no in-app agent.
* Writing rules: no em dashes, ASCII tables, factual prose,
  comments explain WHY not WHAT.
* Pinned Rust toolchain in `rust-toolchain.toml`.

## What changes from phase 6

* `@@Backsystacean` → `@@Systacean` (Syseng + Rustacean only;
  backend moves into @@FullStack).
* `@@Backend` and `@@Frontend` merged into `@@FullStack`.
* Alex no longer cuts tasks back; @@Architect cuts and Alex
  appends in place (event-driven).
* Working agents fire events via `alex/event-{from}-{to}.md`
  instead of poking @@Architect directly. Event types: `poke`,
  `agent-recycle`, `permission`, `capacity` (reserved).
* Skills + contacts pulled into `docs/agents/{name}/`.
* Phase journals live under `docs/journals/` (no longer at the
  repo root).

This format makes the audit trail clearer when revisiting
phases later, makes it explicit who owns what artifact,
removes the "two-way task" ambiguity from phase 6, and gives
us a typed event log that a future fsnotify-driven dispatcher
can act on.

## The rich prompt + watcher + protocol are one feature

Three things look separate but evolve as a unit:

1. **`process.md` (this file)** — the protocol. What event
   types exist, how agents communicate, what an atomic
   event write looks like, recycle and permission
   semantics, survey shape constraints. Chan-specific for
   now; generalises over time.
2. **The fsnotify watcher** (chan-server side) — the
   engine that reads event files written into a watched
   dir, dispatches `poke\n` to the matching agent's PTY,
   and surfaces events to the bubble overlay. The
   watcher's behavior must match the protocol exactly,
   or things fail silently.
3. **The rich prompt + bubble overlay** — the
   human-facing surface. Inbox-like: surveys arrive,
   user replies via numbered keystrokes, deferred items
   stay around. The whole thing hides when the prompt
   hides.

When we change one, we think about the other two:

* New event type in this file → watcher must parse +
  dispatch it → overlay must render or ignore it
  consistently.
* New overlay shape (e.g. follow-up state) → reply
  schema gains a field → watcher (+ producer agents)
  must handle the new field gracefully.
* New protocol rule (e.g. atomic write) → watcher
  enforces it → external writer docs (orchestration
  SKILL) must instruct it.

Treat the trio as one feature when planning, reviewing,
or hardening. Don't ship a watcher change without
checking the overlay, and don't ship an overlay change
without checking the protocol it serves.

### Survey shape constraints

* **Single-topic surveys**: 1-3 numbered options.
* **Multi-topic surveys**: 2-4 topics, each with 1-3
  numbered options.
* If a decision has more options or topics, the
  producer (most often @@Architect) is asked to split
  it into multiple surveys or fold scope down before
  emitting. The TUI density of the overlay only works
  at these bounds.
