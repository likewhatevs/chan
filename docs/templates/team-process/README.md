# Team-process templates

Parameterised process docs that `fullstack-a-79`'s bootstrap
orchestrator copies into each new team's `Drafts/team-<name>/
docs/` at create-team time, substituting the team's actual
handles via `web/src/state/teamTemplate.ts::substituteTeamTemplate`.

## Substitution tokens

| Token                  | Substituted to                              |
|------------------------|---------------------------------------------|
| `{host-handle}`        | The team's host handle (e.g. `@@Alex`)      |
| `{lead-handle}`        | The team's lead handle (e.g. `@@Architect`) |
| `{worker-N-handle}`    | Nth worker (1-indexed); gaps preserved      |
| `{team-name}`          | Team name slug (e.g. `team-alpha`)          |

Tokens are kebab-case; CamelCase / snake_case variants are NOT
recognised so typos surface at audit time rather than silently
rendering an empty string.

## Files

* `bootstrap.md.tpl` — agent bootstrap prompt. Parameterised
  from `docs/agents/bootstrap.md` (the chan-internal canonical
  version) so a new team's lead can read it and dispatch
  workers with the same process discipline chan uses.
* `architect.md.tpl` — lead-role descriptor. Parameterised
  from `docs/agents/architect.md`. The lead reads this on
  bootstrap to learn the dispatch / journal / coordination
  conventions.
* `fullstack.md.tpl` — full-stack worker role descriptor.
  Parameterised from `docs/agents/fullstack.md`. Each
  full-stack worker on a team consumes this on bootstrap.
* `systacean.md.tpl` — systems / Rust-quality worker role
  descriptor. Parameterised from `docs/agents/systacean.md`.
* `webtest.md.tpl` — UI-walkthrough / proactive-walk worker
  role descriptor. Parameterised from `docs/agents/webtest.md`.

### Note on chan-specific phase history

The role docs reference predecessor handles (`@@Backend`,
`@@Frontend`, `@@Syseng`, etc.) + phase numbers (1-8) that
record chan's own evolution. These references stay verbatim
in the template; the `fullstack-a-79` orchestrator can
optionally strip them at publish time if a new team starts
fresh without inheriting chan's phase history.

### Orchestration subdir (`orchestration/`)

Mirrors `docs/agents/orchestration/`. Cross-cutting protocol
docs that every agent in a team reads:

* `orchestration/README.md.tpl` — subdir index.
* `orchestration/atomic-writes.md.tpl` — atomic-write
  discipline; 1 handle substitution.
* `orchestration/mcp-discovery.md.tpl` — MCP discovery flow;
  no handle substitutions.
* `orchestration/spawn-protocol.md.tpl` — spawn-agent IPC
  contract; 2 handle substitutions. Remaining `@@<Name>`
  references are placeholder examples (the spawn protocol
  documents the shape, not specific handles).

### Deferred to follow-up slices

* Slice 4 (deferred): per-agent contact cards
  (fullstack-a.md / fullstack-b.md / webtest-a.md /
  webtest-b.md / ci.md). Per-agent cards encode individual
  identity (slot history, predecessors) that doesn't map
  cleanly to the template variables; requires a different
  shape (per-worker metadata file generated from team
  config at bootstrap time).
* Slice 5 (deferred): optionally parameterise `phase-N`
  references if `-a-79`'s orchestrator wants new teams to
  start at a different phase label.

## Chan-internal usage

Chan's own agents (the project itself) operate as a special-
case team. `web/src/state/teamTemplate.ts::CHAN_INTERNAL_TEAM_VARS`
exports the chan substitution map (`@@Alex` / `@@Architect` /
`@@FullStackA` / etc.); `-a-79`'s orchestrator reuses it for
the chan-internal substitution path. New teams supply their
own vars at bootstrap.
