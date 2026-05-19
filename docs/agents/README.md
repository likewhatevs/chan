# Agents

Contact cards and skill anchors for the agents that have worked
on chan. Used as the link target for `@@{name}` references in
the phase journals (`docs/journals/phase-*/`).

## Active roster (phase 7)

| Tag           | Contact                                  | Profile                              |
|---------------|------------------------------------------|--------------------------------------|
| @@Architect   | [architect.md](architect.md) | Plan, dispatch, decisions.       |
| @@FullStack   | [fullstack.md](fullstack.md) | Backend + Frontend merged.       |
| @@Systacean   | [systacean.md](systacean.md) | Syseng + Rustacean.              |
| @@WebtestA    | [webtest-a.md](webtest-a.md) | Web test lane A.                 |
| @@WebtestB    | [webtest-b.md](webtest-b.md) | Web test lane B.                 |

## Historical handles

Older phases used different slot names. Each historical
contact below is a redirect card pointing at its active
successor, so a `@@Backend` reference in a phase-2 journal
resolves to its contact card here AND to @@FullStack via
the predecessor list on the active card.

| Tag             | Active successor                | Contact                                                       |
|-----------------|---------------------------------|---------------------------------------------------------------|
| @@Backend       | @@FullStack                     | [backend.md](backend.md)                       |
| @@Frontend      | @@FullStack                     | [frontend.md](frontend.md)                     |
| @@Webdev        | @@FullStack                     | [webdev.md](webdev.md)                         |
| @@Syseng        | @@Systacean                     | [syseng.md](syseng.md)                         |
| @@Rustacean     | @@Systacean                     | [rustacean.md](rustacean.md)                   |
| @@Backsystacean | @@FullStack + @@Systacean       | [backsystacean.md](backsystacean.md)           |
| @@Webtest       | @@WebtestA + @@WebtestB         | [webtest.md](webtest.md)                       |

## Skills

Each agent ships its own skill guides under
`docs/agents/{name}/skills/`. The guides are copies of the
shared skill library at
`~/dev/github.com/fiorix/dotfiles/ai/skills/` so the repo stays
self-contained.

## Why this directory exists

We want phase journals to be graphable: any `@@{name}`
reference resolves to a single canonical contact card and a
single skill anchor. Future tooling can crawl
`docs/journals/**` for `@@{name}` mentions and join against
this directory to render the dev-log graph.
