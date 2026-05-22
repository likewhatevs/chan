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

Remaining `docs/agents/*.md` files (architect/working-agent
process docs, file-browser / terminal / editor walkthroughs)
will be parameterised in follow-up slices of `-a-81` as
`-a-79`'s orchestrator surfaces the need; this slice ships the
helper + the first canonical template so orchestrator
integration can begin in parallel.

## Chan-internal usage

Chan's own agents (the project itself) operate as a special-
case team. `web/src/state/teamTemplate.ts::CHAN_INTERNAL_TEAM_VARS`
exports the chan substitution map (`@@Alex` / `@@Architect` /
`@@FullStackA` / etc.); `-a-79`'s orchestrator reuses it for
the chan-internal substitution path. New teams supply their
own vars at bootstrap.
