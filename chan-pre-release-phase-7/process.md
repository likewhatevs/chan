# Working on chan — Phase 7 process

Alex is the host. Agents come to Alex for product direction,
permission, or decisions that can't be made from the written
plan. Otherwise coordination happens through per-author task
files in this directory tree.

This process amends the phase-6 process in two main ways:

1. Each team member (and Alex) owns a directory in this phase.
2. Communication runs **both ways**: @@Architect can request
   things from Alex through task files, not just receive
   direction from Alex.

## Directory layout

```
chan-pre-release-phase-7/
├── process.md              (this file)
├── request.md              (Alex's source-of-truth ask, when filed)
├── summary.md              (@@Architect writes at phase close)
├── alex/
│   ├── journal.md
│   └── alex-{task}.md      (open asks / decisions Alex is tracking)
├── architect/
│   ├── journal.md
│   └── architect-{task}.md
├── frontend/
│   ├── journal.md
│   └── frontend-{task}.md
├── backsystacean/
│   ├── journal.md
│   └── backsystacean-{task}.md
├── webtest-a/
│   ├── journal.md
│   └── webtest-a-{task}.md
└── webtest-b/
    ├── journal.md
    └── webtest-b-{task}.md
```

Names use lowercase with hyphens. The author directory matches
the lowercase team-member name without the `@@` prefix.

## Author journals

Each member maintains `{name}/journal.md`. The file starts with:

```markdown
# {Title}

Author: {name}
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

## Communication: Architect ↔ Alex

@@Architect can request things from Alex by appending to
`alex/alex-{topic}.md`. The append acts as a request channel.
Each request should include:

* The decision or input @@Architect needs.
* The options under consideration with @@Architect's
  recommendation flagged.
* The downstream tasks gated on the answer.

Alex responds by appending to the same file. Once decided, the
decision is mirrored into @@Architect's journal under the
"Decisions" log so future readers can find it without scanning
every alex/ task file.

Other agents can also append asks for Alex into
`alex/alex-{topic}.md`, but should prefer routing through
@@Architect for coordination unless the ask is genuinely
single-owner.

## Capacity planning

Same shape as phase 6: @@Architect drafts a proposal in their
journal, Alex validates the actual resource assignment by
appending to `alex/alex-capacity.md`. Profiles, slots,
expected handoffs, known capability assumptions all recorded
there.

## Review and hardening

Specialists review work that touches their area, same as phase
6:

* @@Frontend reviews frontend UX, state, interaction.
* @@Backsystacean reviews backend HTTP, filesystem semantics,
  Rust quality (combined profile).
* @@Webtest validates the running web experience.

When a specialist sign-off is owed, the requesting author
appends a "Specialist review requested" note to their task file
and the specialist appends their findings.

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

* Single shared directory → per-author directories.
* Single shared journal → per-author journals plus the
  @@Architect canonical journal for phase-wide state.
* Task files are append-only.
* Explicit channel for @@Architect → Alex requests via
  `alex/alex-{topic}.md`.

This format makes the audit trail clearer when revisiting
phases later and makes it explicit who owns what artifact.
