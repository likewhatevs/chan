# fullstack-a-81 — Process template generalisation ({host-handle} + {lead-handle} + generic workers)

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched
Round: addendum-b wave-1

## Goal

Generalise the chan-internal agent process docs
(`docs/agents/*.md` + related) so they parameterise
on `{host-handle}` + `{lead-handle}` + generic
worker handles, rather than hardcoding `@@Alex` /
`@@Architect` / `@@FullStackA` / `@@Systacean` etc.

The generalised template will be COPIED into each
team's `Drafts/team-{name}/docs/` at bootstrap time
(per `-a-79`'s scope) with the team's actual
handles substituted.

## Reference

[`../alex/addendum-b.md`](../alex/addendum-b.md)
clarification #6:

> Process is actually between Host↔Lead and
> Lead↔team-of-generic-workers — chan-specific
> worker handles shouldn't be in the template.

@@Alex's framing: "the process is really between
the host and the lead, and the lead and the rest
of the team — we will update/generalise our
template to fit this model."

## Scope

### Audit current docs/agents/

Walk `docs/agents/` + related process docs.
Identify every mention of chan-specific handles
(`@@Alex`, `@@Architect`, `@@FullStackA`,
`@@FullStackB`, `@@Systacean`, `@@CI`, `@@WebtestA`,
`@@WebtestB`, etc.) that pertain to PROCESS
(coordination rules, journal patterns, task
dispatch, event channels) vs CHAN-SPECIFIC
(actual phase-8 work history).

### Parameterise process docs

Replace process-pertinent handles with template
variables:

* `@@Alex` (host) → `{host-handle}`
* `@@Architect` (lead) → `{lead-handle}`
* `@@FullStackA` / `@@Systacean` / etc. (workers)
  → `{worker-handles}` OR generic enumeration
  (`{worker-1-handle}`, etc.). Implementer's call
  on shape.

### Two versions

* **Chan-internal version** (lives in
  `docs/agents/` and used for the chan project
  itself): keeps the substituted-out templated
  form. Chan's own agents (us right now) are
  treated as a TEAM with `{host-handle} =
  @@Alex`, `{lead-handle} = @@Architect`, etc.
* **Template version** (lives in a new
  `docs/templates/team-process/` or similar):
  raw parameterised version that bootstrap
  copies + substitutes per-team.

Or: same source-of-truth with a substitution step.
Either works; implementer picks the shape that
reads cleanest.

### What NOT to parameterise

Chan-specific work history (phase-8 journals,
task files for our work, etc.) stays as-is.
Those are records of OUR project, not the
template.

## Acceptance

1. `docs/agents/bootstrap.md` (and related)
   parameterised on `{host-handle}` + `{lead-handle}`
   + worker handles.
2. Chan's own agents (us) still operate per the
   process — substitution path validated by
   reading the substituted form.
3. Template form exists for `-a-79`'s bootstrap
   orchestrator to copy + substitute.
4. Substitution function works (write the helper
   if not already present).

### Tests

Unit test on the substitution helper. Source-level
audit pin that the parameterised form has no
chan-specific handles in process docs (only in
phase journals).

### Gate

If any web/ tests touch the docs paths,
`npm test` / `check` / `build` green. Likely
docs-only commit; minimal gate.

## Coordination

* @@FullStackA (or @@Architect — this is doc work
  that touches no source code typically). Assigning
  to @@FullStackA since the substitution helper
  may need SPA-side wiring for `-a-79`'s template
  copy.
* If audit reveals the doc work is bigger than
  expected (e.g. many cross-references to chan-
  specific handles in process flow), flag + I'll
  re-route.

## Authorization

Yes for `docs/agents/*.md` + new
`docs/templates/team-process/` (if used) +
substitution helper + tests + task tail + outbound.

## Numbering

This is `-a-81`.

## Out of scope

* Phase-8 journal rewrites (those are project
  history, not process template).
* Bootstrap orchestrator (`-a-79`).
* Team config schema (`systacean-30`).
