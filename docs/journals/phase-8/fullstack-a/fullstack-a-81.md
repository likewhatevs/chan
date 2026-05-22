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

## 2026-05-22 — slice 1 (helper + bootstrap.md.tpl) ready for review

Per the architect's slice-friendly framing, I'm
splitting `-a-81` into per-doc slices.
**Slice 1**: substitution helper + the first
canonical template (`bootstrap.md.tpl`).
Remaining `docs/agents/*.md` files
(architect.md, fullstack.md, systacean.md, etc.)
land in follow-up slices as `-a-79`'s
orchestrator surfaces the need.

Four-file change. Docs + SPA helper.

### What landed

`web/src/state/teamTemplate.ts` (new):

* Exports `substituteTeamTemplate(template, vars)`
  + `TeamTemplateVars` interface +
  `CHAN_INTERNAL_TEAM_VARS` constant.
* Token grammar: `{host-handle}` /
  `{lead-handle}` / `{worker-N-handle}` /
  `{team-name}`. Kebab-case only;
  CamelCase / snake_case variants left as-is
  so typos surface at audit time rather than
  silently rendering empty strings.
* Gap-friendly: `{worker-5-handle}` when only
  3 workers exist preserves the placeholder
  literally instead of inserting empty.
* `CHAN_INTERNAL_TEAM_VARS` exports chan's
  own substitution map (@@Alex / @@Architect
  / @@FullStackA..WebtestB / team-name="chan")
  so `-a-79` can reuse it for the chan-
  internal substitution path.

`docs/templates/team-process/bootstrap.md.tpl`
(new): canonical bootstrap prompt, parameterised
from `docs/agents/bootstrap.md`. Substitutions
applied via bulk regex:

* `@@FullStackA` → `{worker-1-handle}`
* `@@FullStackB` → `{worker-2-handle}`
* `@@Systacean` → `{worker-3-handle}`
* `@@CI` → `{worker-4-handle}`
* `@@WebtestA` → `{worker-5-handle}`
* `@@WebtestB` → `{worker-6-handle}`
* `@@Architect` → `{lead-handle}`
* `@@Alex` → `{host-handle}`
* `chan project` → `{team-name} project`
  (only the prose; platform-name references
  like `chan-server` / `chan-drive` left as-is
  since those reference the underlying chan
  platform that all teams use).

58 handle tokens substituted. 3 remaining `@@`
references are meta-placeholders showing the
substitution shape (`@@<AgentName>`,
`@@<Agent>`, `@@X`) — left as-is intentionally.

`docs/templates/team-process/README.md` (new):
documents the substitution tokens + the chan-
internal usage pattern. References the helper
+ the substitution map.

`web/src/state/teamTemplate.test.ts` (new): 8
test pins covering all four token types,
gap-preservation for missing workers,
team-name defaulting, unknown-token preservation
(audit-friendly), repeated tokens in a single
template, and the chan-internal vars
roundtrip.

### What's deferred to follow-up slices

* **Slice 2 (when `-a-79` needs it)**: parameterise
  `docs/agents/architect.md`,
  `docs/agents/fullstack.md`, etc. — the per-
  role process docs.
* **Slice 3**: parameterise the
  `docs/agents/orchestration/` subdir.
* **Slice 4**: optionally parameterise
  `phase-N` references if `-a-79`'s orchestrator
  wants new teams to start at a different
  phase label.

The parent `-a-81` umbrella stays open until
all docs are parameterised; architect's call on
whether to dispatch each slice as a separate
task or keep under the umbrella.

### Acceptance (slice 1)

1. `bootstrap.md.tpl` parameterised on
   `{host-handle}` + `{lead-handle}` +
   `{worker-N-handle}` ✓.
2. Chan's own agents still operate per the
   process — substitution path validated by
   `substituteTeamTemplate(bootstrap-tpl,
   CHAN_INTERNAL_TEAM_VARS)` producing
   chan-canonical handles ✓.
3. Template form exists for `-a-79`'s
   bootstrap orchestrator to copy + substitute
   ✓.
4. Substitution helper works ✓ (8 raw-source +
   behaviour pins).

### Gate

* vitest **846 / 846** (+8 net from `-a-69`'s
  838).
* svelte-check 0 errors / 0 warnings across
  4015 files.
* npm build clean.
* Rust gate not re-run (no Rust touched; this
  is docs + SPA helper).

(3 unrelated test flakes on first vitest run —
known EmptyPaneCarousel / Pane / TerminalTab
load-contention pattern; cleared on re-run.)

### Decisions

* **Per-slice split** matches `-a-67` /
  `-a-66`'s shape. Substantial doc work
  warrants slicing.
* **Helper in SPA (`web/src/state/teamTemplate.ts`)**
  vs chan-server-side Rust — the bootstrap
  orchestrator is SPA-side (`-a-79`); helper
  lives where its consumer does.
* **Kebab-case-only token grammar** —
  rigorous; typos / wrong-case land in the
  audit pin.
* **Preserve unknown tokens literally** —
  audit-friendly. The orchestrator can detect
  un-substituted tokens via a regex sweep
  before publishing the team's docs.
* **`{worker-N-handle}` gap preservation** —
  if a team has only 3 workers, references
  to worker 5 stay as `{worker-5-handle}`
  rather than rendering empty. The template
  author / orchestrator sees the gap.
* **`chan project` → `{team-name} project`**
  only on prose mentions; platform-name
  references (`chan-server`, `chan-drive`)
  left as-is. Those reference the underlying
  chan platform that all teams use.

### Suggested commit subject

```
Team-process templates: substitution helper + bootstrap.md.tpl (fullstack-a-81 slice 1)
```

Single commit. Helper + template + README +
tests tightly coupled.

### Files for `git add` (per-path discipline)

* `web/src/state/teamTemplate.ts` (new)
* `web/src/state/teamTemplate.test.ts` (new)
* `docs/templates/team-process/bootstrap.md.tpl` (new)
* `docs/templates/team-process/README.md` (new)
* `web/src/components/BubbleOverlay.test.ts` (type-fix follow-up to `-a-69`)
* `docs/journals/phase-8/fullstack-a/fullstack-a-81.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging only.

Push held. Standing by for clearance.
