# fullstack-a-7: Switch Hybrid NAV binding from Cmd+K to Cmd+.

Owner: @@FullStackA
Date: 2026-05-19

## Goal

Replace the Cmd+K keymap entry for Hybrid NAV with Cmd+. and
introduce Cmd+, as the binding for Settings (matching the macOS
system-wide convention for app preferences). Hard switch: drop
Cmd+K so we don't leave two active bindings.

## Background

Source in [`../phase-8-bugs.md`](../phase-8-bugs.md): "Switch
Hybrid NAV binding from Cmd+K to Cmd+.".

Phase-7 references for the existing Cmd+K binding:
`fullstack-15`, `fullstack-16` (pane mode substrate), and
`fullstack-62` (Pane Mode → Hybrid NAV rename).

## Acceptance criteria

* Pressing Cmd+. opens Hybrid NAV with the same behaviour Cmd+K
  has today.
* Pressing Cmd+, opens Settings. Confirm what "Settings" means
  for this UI (whether an overlay already exists or this
  bootstraps a settings-overlay surface — if the latter, append
  a scope question and wait for @@Architect rather than
  designing a Settings overlay in this task).
* Cmd+K no longer triggers Hybrid NAV.
* Status-bar label updated: replace `Hybrid ☯ Enter commit,
  Esc discard, H help` Cmd+K wording with the Cmd+. form
  everywhere it appears (status bar, PaneModeHelp cheatsheet,
  any inline copy or aria-labels).
* Web variant: Cmd+. on chan-desktop native; for the browser
  SPA, verify Cmd+. is not browser-reserved. If it is,
  introduce a web fallback (e.g. Cmd+Alt+.) per the phase-7
  pattern for Cmd+T / Cmd+]\Cmd+[.

## How to start

* Hybrid-NAV keymap dispatch: phase-7 `fullstack-15` /
  `fullstack-16` files for the keybinding registration site.
* Status-bar label: shared with `fullstack-a-3` (Cmd+K cluster
  fix). Coordinate so the wording lands in one place; if
  `fullstack-a-3` already ships the label change with Cmd+K
  wording, amend it via a follow-up entry in this task file.
* Settings overlay: if it does not exist, scope-question
  @@Architect before designing it. We can ship Cmd+. alone in
  this task and keep Cmd+, as a TODO if needed.

## 2026-05-19 — implementation note

Scope question answered itself: `app.settings.toggle` already
exists in `shortcuts.ts` with `web/native: "Mod+,"`, and
`openSettings` / `settingsOverlay` exist in
`state/store.svelte.ts`. So Cmd+, → Settings overlay is
already wired; no new overlay design needed.

Three edits:

1. **App.svelte window keydown handler** — the chord
   condition was `meta && !shift && !alt && e.code === "KeyK"`;
   changed `KeyK` to `Period`. `KeyboardEvent.code` (vs `.key`)
   is the safer match because Option/Alt-modified keys on macOS
   change `.key` but not `.code`.

2. **shortcuts.ts** — `app.pane.mode` now declares `web:
   "Mod+."` / `native: "Mod+."`; `app.pane.flip` updated to
   `"Mod+. Tab"` so the chord chain stays internally consistent.
   `app.settings.toggle` was already `Mod+,`.

3. **Stale chord references in inline doc comments** —
   `PaneModeHelp.svelte`'s header comment swapped `Cmd+K` for
   `Cmd+.`. Other in-code comments referencing `Cmd+K` are
   historical and stay (they explain phase-3 migration context
   and don't claim a current binding).

The Hybrid status-bar pill copy from `-3` already reads
`Hybrid ☯ Enter commit, Esc discard, H help` — no `Cmd+K` /
`Cmd+.` in the visible text, so no copy update needed there
(per the architect's coordination note).

**Browser-reserved check**: `Cmd+.` is not reserved by Safari
or Chrome on macOS for navigation / window actions, and JS can
preventDefault it. The chord handler calls `e.preventDefault()`
on the match. No `Cmd+Alt+.` fallback needed.

Files touched:

* `web/src/App.svelte` — `KeyK` → `Period`.
* `web/src/state/shortcuts.ts` — `app.pane.mode` /
  `app.pane.flip` chord descriptors.
* `web/src/components/PaneModeHelp.svelte` — header comment.

Pre-push gate (SPA portion): vitest 456/456 green;
`npm run check` 0 errors / 1 warning (`EmptyPaneCarousel.svelte`
non-reactive update — pre-existing, unrelated to this task);
`npm run build` clean.

## 2026-05-19 — @@Architect: approved + commit clearance

Reviewer: @@Architect.

Good catch on `Mod+,` → Settings already being wired — saved
yourself a scope question. `KeyboardEvent.code` over `.key` is
the right defensive choice for Option/Alt-modified macOS keys.
The browser-reserved check (Cmd+. is not Safari/Chrome-claimed
and `preventDefault` works) means we don't need the
`Cmd+Alt+.` fallback, simplifying the chord descriptors.

The pre-existing `EmptyPaneCarousel` warning is unrelated and
resolved by @@FullStackB's `fullstack-b-4` (which promoted
`panStart` to `$state`); both can land independently.

**Commit clearance**: approved. Suggested subject:

```
Hybrid NAV: Cmd+K → Cmd+. (Cmd+, already wired to Settings) (fullstack-a-7)
```

Push waits for Round-1 close. Pick up `fullstack-a-8` next
(CSS wobble restore).
