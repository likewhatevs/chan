# fullstack-57: defer scope reset in GraphPanel until scopeOptions populated

Owner: @@FullStackA
Cut by: @@Architect
Date: 2026-05-19

## Why

`webtest-a-8` item 6 caught a real defect in the
`fullstack-43` context-aware Pane Mode spawn flow.

When a user opens a Graph tab from a focused doc
editor (Cmd+K then 3), `paneModeOpenGraph` correctly
threads the doc's path through:

* sets `scopeId = "file:<doc-path>"`
* sets `pendingSelectId = <doc-path>`
* sets the tab title to "File Graph"

But on mount, `GraphPanel.svelte` looks up the
`scopeId` against `scopeOptions`, doesn't find a
match (the options list hasn't been populated yet —
or the synthetic `file:<path>` scope isn't a member
of the options set), and **resets `scopeId` back to
the drive root**. The contextual scope is lost
before the user sees it.

## Relevant code

* `web/src/components/GraphPanel.svelte` — the mount
  effect that does the `scopeId` lookup + reset.
  Find the effect that touches `scopeOptions` and
  conditionally reassigns `scopeId`.
* `web/src/state/tabs.svelte.ts` — `paneModeOpenGraph`
  for context on what's being set. The intent there
  is correct; the fix lives in GraphPanel.
* `webtest-a-8` verdict (item 6, side observations)
  for the diagnosis text.

## Acceptance criteria

Two acceptable fix shapes per @@WebtestA's
suggestion. Pick whichever lands cleaner:

### Option A — defer the reset

Don't reset `scopeId` to the drive root until
`scopeOptions` is actually populated. While the
options list is empty / still loading, leave the
incoming `scopeId` alone. Once options are loaded,
THEN check membership and reset only if the scope
is genuinely unknown.

### Option B — accept synthetic scopes as valid

Recognize `file:<path>` and `dir:<path>` prefixes
as valid scope shapes regardless of whether they
appear in `scopeOptions`. The graph machinery
should be able to render a scope for any in-drive
path; the options list is a discoverability
surface, not a membership gate.

### Test

* Add a regression test (Vitest or component-
  level) asserting: when a Graph tab mounts with
  `scopeId = "file:foo/bar.md"`, after the mount
  effect settles, `scopeId` is still
  `"file:foo/bar.md"`, NOT reset to the drive
  root.

### Gate

* `npm run check`
* `npm run test`
* `npm run build`
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`

## Notes

* Affects v0.11.0 — this is a regression in the
  marquee Pane Mode context-aware spawn surface
  that the walkthrough caught. Should ship before
  the tag.
* No re-walk cost on Lane A (their walkthrough is
  done); the fix lands cleanly and the verdict
  stands. Lane B is still walking; flag if their
  pass touches the Graph surface.
* Standing topic-level commit clearance.
* Position in your queue: behind `-55` + `-56`.
