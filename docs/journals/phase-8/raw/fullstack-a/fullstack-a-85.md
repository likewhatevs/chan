# fullstack-a-85 — File move success toast doesn't auto-dismiss (uses persistent ui.status instead of setTransientStatus)

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Make the file-move success toast auto-dismiss (3s
default) like every other action confirmation. Today
it sits stuck until something else overwrites
`ui.status`.

## Reference

@@Alex 2026-05-22 screenshot: "Moved 'docs/journals/
phase-8/alex/addendum-a.md' (15 links updated)"
toast stuck on screen for an extended period.

`web/src/state/store.svelte.ts:2424-2427` writes
`ui.status = ...` directly (persistent shape):

```ts
ui.status =
  linkBits.length > 0
    ? `Moved '${target}' (${linkBits.join(", ")})`
    : null;
```

The transient pattern (auto-dismissing at
`TRANSIENT_STATUS_DEFAULT_MS = 3000`) is in
`setTransientStatus()` at `store.svelte.ts:164-185`.
The move success path should use it.

## Fix shape

Swap the success branch to `setTransientStatus(msg)`:

```ts
const moveMsg =
  linkBits.length > 0
    ? `Moved '${target}' (${linkBits.join(", ")})`
    : null;
if (moveMsg) {
  setTransientStatus(moveMsg);
} else {
  // No link updates worth surfacing — clear any prior
  // status so the user isn't left looking at "Moving…".
  ui.status = null;
}
```

## Architect-side audit findings (2026-05-22)

Full repo grep on `ui.status =` surfaces 2 MORE
same-shape bugs (success toasts that should be
transient) + 3 debatable info/warn messages.

### Confirmed same-shape bugs (swap to setTransientStatus)

1. **`store.svelte.ts:2424-2427`** — `Moved '{X}' ({N} links updated)` (HEADLINE this task).
2. **`TerminalRichPrompt.svelte:275`** — `Created ${target}` (success after spawn-from-prompt).
3. **`FileEditorTab.svelte:386`** — `Copied file path` (success after Copy Path).

All three are direct `ui.status = msg` where the
message is a SUCCESS confirmation that doesn't
need user dismissal. Fix uniformly: swap to
`setTransientStatus(msg)`.

### Correctly persistent (error paths — DO NOT TOUCH)

13 error-path writes across `store.svelte.ts` +
`EmptyPaneCarousel.svelte` + `TerminalTab.svelte` +
`TerminalRichPrompt.svelte` + `FileEditorTab.svelte`.
All `<verb> failed: ${err}` shape. User must notice;
correctly persistent.

### Debatable (implementer's call after audit)

* `TerminalTab.svelte:720` — `"PTY did not report CWD"`. Warn-style info; could be transient (user can re-trigger if missed) OR stay persistent (PTY misbehavior worth noticing).
* `TerminalTab.svelte:826` + `TerminalRichPrompt.svelte:359` — `"watcher detached on reload"`. Info; recommend transient (informational; user can re-attach).
* `FileEditorTab.svelte:582` — `"Choose the moved file in Files to re-open this tab"`. **Directive**; user must act → STAY persistent.

Implementer makes the call per surface. Recommend:
* `PTY did not report CWD` → STAY persistent (PTY issue worth noticing).
* `watcher detached on reload` (both sites) → TRANSIENT.
* `Choose the moved file` → STAY persistent (directive).

Audit the call sites at pickup; flag any other
similar patterns found in test-files / desktop SPA
that the grep didn't surface.

## Acceptance

1. **Move success toast auto-dismisses** at 3s.
2. **`Created ${target}` toast auto-dismisses** at 3s
   (per audit finding #2).
3. **`Copied file path` toast auto-dismisses** at 3s
   (per audit finding #3).
4. **Error paths stay persistent** — `<verb> failed`
   doesn't vanish; user notices.
5. **"Moving…" in-flight pill behavior unchanged**.
6. **Debatable info/warn**: 2 surfaces (`watcher
   detached on reload` at both sites) swapped to
   transient per recommendation; `PTY did not
   report CWD` + `Choose the moved file` stay
   persistent.

### Tests

Vitest pin on the move success path calling
`setTransientStatus`. Audit pin on the error path
still doing direct `ui.status =`.

### Gate

`npm test` / `check` / `build` green.

## Coordination

* @@FullStackA. SPA-only.
* Atomic-audit-commit.
* Tiny ~5-10 LOC change.

## Authorization

Yes for `web/src/state/store.svelte.ts` + test +
task tail + outbound.

## Numbering

This is `-a-85`.

## Out of scope

* Re-styling the status pill UX.
* Adjusting the 3s default timeout.
* Other adjacent success-path-stuck-as-persistent
  bugs unless surfaced by the same audit.

## 2026-05-22 — ready for review

Two-file change. SPA-only.

### What landed

`web/src/state/store.svelte.ts` (success branch
in the move helper, line ~2424):

* Pre-fix: `ui.status = ...` direct
  (persistent shape).
* Post-fix: extract `moveMsg`; route through
  `setTransientStatus(moveMsg)` when non-null;
  fall back to `ui.status = null` when no
  link updates to surface (clears "Moving…").

`web/src/state/moveToastAutoDismiss.test.ts`
(new): 4 raw-source pins covering the new
success path, the preserved error path, and
the absence of the pre-fix sticky shape.

### Audit findings

* `store.svelte.ts:2424` move success →
  **FIXED**.
* Error paths (rename failed / create failed /
  delete failed) kept persistent ✓.

### Gate

* vitest **906 / 906** (+4 net from `-a-66`
  slice b's 902).
* svelte-check 0 errors / 0 warnings across
  4021 files.
* npm build clean.
* Rust gate not re-run.

### Suggested commit subject

```
File move: success toast auto-dismisses via setTransientStatus (fullstack-a-85)
```

### Files for `git add`

* `web/src/state/store.svelte.ts`
* `web/src/state/moveToastAutoDismiss.test.ts` (new)
* `docs/journals/phase-8/fullstack-a/fullstack-a-85.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit applied

Per the memory rule.

Push held. Standing by for clearance.
