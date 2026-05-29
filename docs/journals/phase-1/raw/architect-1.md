# architect-1: Release-surface audit and design snapshot

Owner: architect. Depends on: nothing. Unblocks: rustacean-1,
rustacean-2, rustacean-3, syseng-1.

## Goal

Produce a concise design snapshot for Phase 1 so implementation tasks
share the same boundaries and avoid changelog-style documentation.

## Scope

- Audit existing docs and code comments for stale migration / legacy
  release-history language that conflicts with "first canonical version".
- Identify which current decisions need snapshot docs:
  - drive boundary and path safety
  - search index lifecycle
  - graph index shape
  - report index use
  - assistant stream state
  - CLI/server/frontend ownership
- Record any public-contract decisions in `design.md`, local component
  design docs, or this phase directory. Prefer existing docs when they
  are the load-bearing reference.

## Non-goals

- Do not implement product changes.
- Do not rewrite historical comments that are accurate internal context
  unless they create release-facing confusion.

## Acceptance criteria

1. DONE: `phase-1/journal.md` names the docs that must be
   updated by implementation tasks.
2. DONE: `phase-1/design-snapshot.md` exists with the
   current intended Phase 1 architecture and open risks.
3. DONE: stale migration cleanup found during the audit is assigned to
   `rustacean-1`.

## Verification

- Read-only audit plus `rg` evidence in the task update.

Command run:

```
rg -n "migrat|schema_version|legacy|old version|compat|backward|deprecated|v[0-9]+|pre-release|canonical" README.md design.md CLAUDE.md crates web/src -g '!*.lock'
```

Findings:

- `crates/chan-server/src/indexer.rs`: pre-v3 contact email backfill is a
  real internal-version migration path. `rustacean-1` owns cleanup.
- `crates/chan-server/src/preferences.rs`: partial preference defaults and
  pane-width tests are current config resilience, not release migration.
- `web/src/state/store.svelte.ts`: legacy layout/session parsing is local
  browser state compatibility. Defer unless Alex wants a hard first-run
  reset of UI sessions.
- Editor `legacy` hits mostly describe compatibility with current source
  editor/component contracts during the WYSIWYG rewrite.
- `@codemirror/legacy-modes` is an upstream package name.
- `chan-error-v2` is a wrapper contract-version note, not a migration.

## Done means

Update this file with findings, mark status REVIEW in `journal.md`, and
create follow-up tasks only for concrete implementation work.

Status: DONE.
