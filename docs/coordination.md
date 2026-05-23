# How chan is developed

If you've landed here from the issue tracker, a PR, or
just browsing the repo, this doc explains the
multi-agent development pattern you'll see reflected
in the journals (`docs/journals/phase-N/`). It's not a
user-facing document; it's an explainer for outside
contributors and curious readers.

## TL;DR

chan is built by a small team of AI coding assistants
coordinated by the project owner. The assistants take
on persistent roles (architect, backend engineer,
frontend engineer, tester, etc.), communicate via
append-only event channels and task files in the
repo, and produce real commits on `main`. The owner is
the bridge between roles and the source of strategic
decisions.

This is unusual enough that the journals can look
confusing without context. Hence this doc.

## Roles

Two role types operate on the project:

* **Architect**: plans the phase, dispatches work,
  brokers decisions with the owner. Carries no
  implementation slot of its own. Writes task files
  for working roles.
* **Working roles**: backend, frontend, full-stack
  (a mix), tester, CI/infrastructure. Each picks up
  task files from the architect, implements them,
  fires status updates back through the event
  channels.

Roles are identified by handles starting with `@@`
(e.g., `@@Architect`, `@@FullStackA`, `@@Systacean`,
`@@CI`, `@@WebtestA`). Different sessions can occupy
the same role over time; the role's persistent
artifacts (its journal, its prior commits) carry
forward across sessions.

A second team operates on the chan-desktop side
(`@@Desktect`, `@@Desktacean`, `@@Desktest`) with the
same shape. The owner is the bridge between the two
team leads.

## How work flows

Phases organize the year-scale roadmap. A phase has:

1. **A request** (`request.md`) — the owner's
   high-level ask for this phase.
2. **A process** (`process.md`) — how the team
   coordinates this phase. Usually inherits from the
   previous phase with small deltas.
3. **A bug list** (`phase-N-bugs.md`) — durable
   audit-trail of every issue surfaced during the
   phase, with dispatch status and resolution.
4. **Task files** (`<role>/<role>-N.md`) — what each
   role is asked to do. Append-only journals: once
   started, new appends document progress, not
   rewrites of prior sections.
5. **Event channels** (`alex/event-<from>-<to>.md`) —
   the coordination protocol. When a role finishes a
   task or hits a blocker, they "poke" the architect
   via an event channel. The architect routes
   clearances + next work the same way.

Rounds within a phase break work into waves. A
typical phase has 2-4 rounds; each round closes with
a release tag.

## Why this pattern

A few design choices that show up everywhere:

* **Append-only journals**: nothing gets rewritten
  under another role. If a decision changes, a new
  dated section appends; the prior section stays as
  the audit trail.
* **Lane boundaries**: roles own specific code
  surfaces. Cross-lane work routes through the
  architect. This prevents two roles from editing the
  same file in parallel without coordination.
* **The owner is the bridge**: cross-team decisions
  (architect ↔ desktect; or any major scope call)
  flow through the owner explicitly. Direct
  team-to-team chatter is allowed for breadcrumbs and
  context-sharing, but decisional traffic is the
  owner's bridge.
* **Real commits, real CI**: every role's work lands
  in `main` with normal commit hygiene. The journals
  cite commit subject lines (not SHAs — multi-agent
  rebases would invalidate SHAs).

## What you'll see in the repo

* `docs/journals/phase-N/` — the active phase's
  journals. Read `process.md` and the architect's
  `architect/journal.md` to orient.
* `docs/journals/phase-N/alex/event-*.md` — the event
  channels. Naming convention is
  `event-<from>-<to>.md` (e.g., `event-architect-ci.md`
  is the architect's outbound to the CI role).
* `docs/journals/phase-N/<role>/<role>-N.md` — task
  files. Each is a self-contained brief plus the
  append-only progress journal.
* `docs/agents/` — role contact cards + skill files.
  Useful if you want to know what each role's
  responsibilities cover.

## What this doc is not

It's not a contributing guide (see
[`../CONTRIBUTING.md`](../CONTRIBUTING.md)) or a code
of conduct (see `CODE_OF_CONDUCT.md` once added).
It's just context, so the journals make sense.

## Getting involved

Contributions follow the standard GitHub PR flow as
documented in `CONTRIBUTING.md`. You don't need to
participate in or care about the multi-agent pattern
— it's an internal coordination protocol, not a
project requirement. PRs are reviewed the same way
regardless of whether they come from a human
contributor or a role acting on the project.
