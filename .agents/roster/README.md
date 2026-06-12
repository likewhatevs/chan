# Agents

Contact cards and skill anchors for the agent handles that appear in
the phase reports (`docs/phases/`). This directory is a historical
index: teams are now provisioned per-round by the `cs terminal team`
tooling with fresh handles, so nothing here is an "active" roster.
The cards exist so any `@@{name}` reference in a report resolves to
who that was.

For the cross-phase operational lessons (coordination, the gate,
verification, commit discipline, the pre-release norms) see the
[playbook](../playbook.md). It is the "how we work and what went wrong when
we did not" companion to these contact cards.

## Workspace team

| Tag           | Contact                                  | Profile                              |
|---------------|------------------------------------------|--------------------------------------|
| @@Architect   | [architect.md](architect.md)   | Plan, dispatch, decisions.       |
| @@FullStackA  | [fullstack-a.md](fullstack-a.md) | FullStack lane A (smaller/faster).|
| @@FullStackB  | [fullstack-b.md](fullstack-b.md) | FullStack lane B (bigger/cross-stack).|
| @@Systacean   | [systacean.md](systacean.md)   | Syseng + Rustacean. Release runway. |
| @@WebtestA    | [webtest-a.md](webtest-a.md)   | Web test lane A.                 |
| @@WebtestB    | [webtest-b.md](webtest-b.md)   | Web test lane B.                 |

## Desktop + CI lanes

A parallel chan-desktop team plus a dedicated CI lane.

| Tag          | Contact                        | Profile                       |
|--------------|--------------------------------|-------------------------------|
| @@Desktect   | [desktect.md](desktect.md)     | chan-desktop architect lead.  |
| @@Desktacean | [desktacean.md](desktacean.md) | Tauri/Rust desktop impl.      |
| @@Desktest   | [desktest.md](desktest.md)     | chan-desktop tester.          |
| @@CI         | [ci.md](ci.md)                 | CI/release infrastructure.    |

## Historical handles

Earlier phases used different slot names. Each maps to its successor
below, so a `@@Backend` reference in an early report resolves to the
role that absorbed it. The standalone redirect cards were removed;
this map is the single resolution point.

| Tag             | Active successor             |
|-----------------|------------------------------|
| @@Backend       | @@FullStackA + @@FullStackB  |
| @@Frontend      | @@FullStackA + @@FullStackB  |
| @@Webdev        | @@FullStackA + @@FullStackB  |
| @@FullStack     | @@FullStackA + @@FullStackB  |
| @@Syseng        | @@Systacean                  |
| @@Rustacean     | @@Systacean                  |
| @@Backsystacean | @@FullStack + @@Systacean    |
| @@Webtest       | @@WebtestA + @@WebtestB      |

## Skills

The stable general skill profiles (architect, rustacean, syseng,
webdev, pythonic) are now vendored in-repo under
[`../skills/`](../skills/), one copy each, alongside the chan-specific
workflows (test-server, release, gate). The upstream source stays
`~/dev/github.com/fiorix/dotfiles/ai/skills/`; the vendored copies are
snapshots and may drift, re-synced on demand. The cards reference
skills by name.

## Why this directory exists

We want the phase reports to be graphable: any `@@{name}` reference
resolves to a single canonical contact card. Future tooling can crawl
`docs/phases/**` for `@@{name}` mentions and join against this directory
to render the dev-log graph.
