# Agents

Contact cards and skill anchors for the agents that have worked
on chan. Used as the link target for `@@{name}` references in
the phase reports (`docs/phases/`).

For the cross-phase operational lessons (coordination, the gate,
verification, commit discipline, the pre-release norms) see the
[playbook](../playbook.md). It is the "how we work and what went wrong when
we did not" companion to these contact cards.

## Active roster (phase 7)

| Tag           | Contact                                  | Profile                              |
|---------------|------------------------------------------|--------------------------------------|
| @@Architect   | [architect.md](architect.md)   | Plan, dispatch, decisions.       |
| @@FullStackA  | [fullstack-a.md](fullstack-a.md) | FullStack lane A (smaller/faster).|
| @@FullStackB  | [fullstack-b.md](fullstack-b.md) | FullStack lane B (bigger/cross-stack).|
| @@Systacean   | [systacean.md](systacean.md)   | Syseng + Rustacean. Release runway. |
| @@WebtestA    | [webtest-a.md](webtest-a.md)   | Web test lane A.                 |
| @@WebtestB    | [webtest-b.md](webtest-b.md)   | Web test lane B.                 |

## Desktop + CI lanes (phase 8)

Phase 8 stood up a parallel chan-desktop team plus a dedicated CI lane.

| Tag          | Contact                        | Profile                       |
|--------------|--------------------------------|-------------------------------|
| @@Desktect   | [desktect.md](desktect.md)     | chan-desktop architect lead.  |
| @@Desktacean | [desktacean.md](desktacean.md) | Tauri/Rust desktop impl.      |
| @@Desktest   | [desktest.md](desktest.md)     | chan-desktop tester.          |
| @@CI         | [ci.md](ci.md)                 | CI/release infrastructure.    |

## Historical handles

Older phases used different slot names. Each maps to its active
successor below, so a `@@Backend` reference in a phase-2 report resolves
to the current role. The standalone redirect cards were removed in the
phase-18 docs cleanup; this map is now the single resolution point.

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
