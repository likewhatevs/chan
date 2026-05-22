# fullstack-a-86 — Follow-up: same-shape toast auto-dismiss fixes (Created ${target} + Copied file path + watcher detached on reload)

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Apply the same `setTransientStatus()` swap pattern
from `-a-85` to the remaining same-shape bugs +
debatable surfaces the audit surfaced. `-a-85`
scoped to the move-success headline only.

## Reference

* `-a-85` task body §"Architect-side audit findings"
  for the per-surface list.
* `-a-85` commit (`<TBD>`) for the precedent fix
  shape on store.svelte.ts:2424.

## Scope

### Confirmed same-shape bugs (swap to setTransientStatus)

1. **`TerminalRichPrompt.svelte:275`** —
   `ui.status = \`Created ${target}\``. Same shape
   as the move success. Swap to
   `setTransientStatus(\`Created ${target}\`)`.
2. **`FileEditorTab.svelte:386`** —
   `ui.status = "Copied file path"`. Same shape.
   Swap.

### Debatable per audit (recommended action)

3. **`TerminalTab.svelte:826`** —
   `ui.status = "watcher detached on reload"`. Swap
   to transient.
4. **`TerminalRichPrompt.svelte:359`** —
   `ui.status = "watcher detached on reload"`
   (second site of the same string). Swap to
   transient.

### Stay persistent (DO NOT TOUCH)

* `TerminalTab.svelte:720` — `"PTY did not report
  CWD"` (PTY issue worth noticing; KEEP persistent).
* `FileEditorTab.svelte:582` — `"Choose the moved
  file in Files to re-open this tab"` (DIRECTIVE;
  KEEP persistent).
* All `<verb> failed: ${err}` error paths — KEEP.

## Acceptance

1. `Created ${target}` auto-dismisses at 3s.
2. `Copied file path` auto-dismisses at 3s.
3. Both `watcher detached on reload` sites
   auto-dismiss at 3s.
4. Error paths unchanged.
5. `PTY did not report CWD` + `Choose the moved
   file` unchanged (persistent).

### Tests

Vitest pin per swapped surface (4 swaps total).
Audit pin asserting the unchanged persistent sites.

### Gate

`npm test` / `check` / `build` green.

## Coordination

* @@FullStackA. SPA-only.
* Atomic-audit-commit.
* Tiny — ~15-20 LOC across 4 sites.

## Authorization

Yes for `TerminalRichPrompt.svelte` +
`FileEditorTab.svelte` + `TerminalTab.svelte` +
tests + task tail + outbound.

## Numbering

This is `-a-86`.

## Out of scope

* Re-styling the status pill UX.
* Adjusting the 3s default timeout.
* Re-architecting `setTransientStatus` /
  `ui.status` boundary.
