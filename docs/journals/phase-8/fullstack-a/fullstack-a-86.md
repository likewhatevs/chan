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

## 2026-05-22 — ready for review

Four-file change. SPA-only.

### What landed

`web/src/components/TerminalRichPrompt.svelte`:
* Imports `setTransientStatus`.
* `Created ${target}` success branch → swapped.
* `watcher detached on reload` (line ~363) →
  swapped.
* Create-failed + submit-mode-flip-failed +
  bubble-mode-failed error paths preserved
  persistent.

`web/src/components/FileEditorTab.svelte`:
* Imports `setTransientStatus`.
* `Copied file path` → swapped.
* Copy-failed error preserved persistent.
* `Choose the moved file in Files to re-open
  this tab` left as `ui.status =` (DIRECTIVE
  per task body).

`web/src/components/TerminalTab.svelte`:
* Imports `setTransientStatus`.
* `watcher detached on reload`
  (`watcherDetached()`) → swapped.
* `PTY did not report CWD` left as
  `ui.status =` (PTY signal worth noticing).

`web/src/components/toastAutoDismissSweep.test.ts`
(new): 9 raw-source pins — 4 confirmed swaps
(Created / Copied / watcher detached × 2) +
4 error-path persistence assertions +
2 directive-persistent assertions.

### Acceptance

1. `Created ${target}` auto-dismisses at 3s ✓.
2. `Copied file path` auto-dismisses at 3s ✓.
3. Both `watcher detached on reload` sites
   auto-dismiss at 3s ✓.
4. Error paths unchanged ✓.
5. `PTY did not report CWD` + `Choose the
   moved file` unchanged ✓.

### Gate

* vitest **916 / 916** (+10 net from `-a-85`'s
  906).
* svelte-check 0 errors / 0 warnings across
  4022 files.
* npm build clean.
* Rust gate not re-run.

### Decisions

* **Inline comments at each swap site** —
  cross-reference `-a-85`'s precedent so a
  future audit knows why each surface
  switched. Mirrors the `feedback-ground-
  descriptions-in-source` discipline.
* **No new helper** — `setTransientStatus`
  already exists; -a-86 just calls it from
  more sites.

### Suggested commit subject

```
Toasts: same-shape auto-dismiss across 4 success / info surfaces (fullstack-a-86)
```

Single commit. 4 swaps + 9 test pins tightly
coupled around the same shape.

### Files for `git add`

* `web/src/components/TerminalRichPrompt.svelte`
* `web/src/components/FileEditorTab.svelte`
* `web/src/components/TerminalTab.svelte`
* `web/src/components/toastAutoDismissSweep.test.ts` (new)
* `docs/journals/phase-8/fullstack-a/fullstack-a-86.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging only.

Push held. Standing by for clearance.
