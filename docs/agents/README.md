# Agents

Contact cards and skill anchors for the agents that have worked
on chan. Used as the link target for `@@{name}` references in
the phase journals (`docs/journals/phase-*/`).

## Active roster (phase 7)

| Tag           | Contact                                  | Profile                              |
|---------------|------------------------------------------|--------------------------------------|
| @@Architect   | [architect/contact.md](architect/contact.md) | Plan, dispatch, decisions.       |
| @@FullStack   | [fullstack/contact.md](fullstack/contact.md) | Backend + Frontend merged.       |
| @@Systacean   | [systacean/contact.md](systacean/contact.md) | Syseng + Rustacean.              |
| @@WebtestA    | [webtest-a/contact.md](webtest-a/contact.md) | Web test lane A.                 |
| @@WebtestB    | [webtest-b/contact.md](webtest-b/contact.md) | Web test lane B.                 |

## Historical handles

Older phases used different slot names. Each historical
contact below is a redirect card pointing at its active
successor, so a `@@Backend` reference in a phase-2 journal
resolves to its contact card here AND to @@FullStack via
the predecessor list on the active card.

| Tag             | Active successor                | Contact                                                       |
|-----------------|---------------------------------|---------------------------------------------------------------|
| @@Backend       | @@FullStack                     | [backend/contact.md](backend/contact.md)                       |
| @@Frontend      | @@FullStack                     | [frontend/contact.md](frontend/contact.md)                     |
| @@Webdev        | @@FullStack                     | [webdev/contact.md](webdev/contact.md)                         |
| @@Syseng        | @@Systacean                     | [syseng/contact.md](syseng/contact.md)                         |
| @@Rustacean     | @@Systacean                     | [rustacean/contact.md](rustacean/contact.md)                   |
| @@Backsystacean | @@FullStack + @@Systacean       | [backsystacean/contact.md](backsystacean/contact.md)           |
| @@Webtest       | @@WebtestA + @@WebtestB         | [webtest/contact.md](webtest/contact.md)                       |

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
