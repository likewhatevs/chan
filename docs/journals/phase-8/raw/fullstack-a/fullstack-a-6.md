# fullstack-a-6: Cmd+K F focuses the search overlay input

Owner: @@FullStackA
Date: 2026-05-19

## Goal

Opening the search overlay via Cmd+K F currently does not place
the caret in the search field. The user has to click the input
before typing. Fix it so the caret lands in the search input
automatically on overlay open, mirroring the cursor-focus rule
in `fullstack-a-4` (rich prompt cluster): open an overlay → caret
in the primary input.

## Background

Source in [`../phase-8-bugs.md`](../phase-8-bugs.md): "Cmd+K F
(enter search overlay) does not focus the cursor in the search
input".

Cmd+K F is the Hybrid-NAV binding that landed in phase-7
`fullstack-74` (moved Search from `s` to `f`).

## Acceptance criteria

* Pressing Cmd+K F places the caret in the search input
  immediately.
* Typing right after Cmd+K F starts the search.
* Esc still closes the overlay (no regression).

## How to start

* Find the Search overlay component in `web/src/components/`
  (likely `SearchOverlay.svelte` or similar; phase-7 reference
  is `fullstack-74`).
* Add a `tick()`-then-`focus()` on mount or on the `open`
  prop transition, whichever the existing pattern uses for the
  other overlays (rich prompt, find buffer).
* Confirm consistency with how the find buffer focuses on
  Cmd+F open.

## 2026-05-19 — implementation note

The overlay is `web/src/components/SearchPanel.svelte`. It
already had a focus path in its open-transition `$effect`:

```
queueMicrotask(() => {
  inputEl?.focus();
  if (seed || restored) inputEl?.select();
});
```

But `queueMicrotask` ran before Svelte flushed the mount
work for the OverlayShell child block, so `inputEl` was
still `undefined` and the focus call was a silent no-op.
Swapped `queueMicrotask` for `tick().then(...)` so Svelte's
component-update tick completes (mounting the input +
binding `inputEl`) before the focus runs. Matches the
pattern used by `TerminalRichPrompt`'s
`fullstack-79`-era focus effect.

Acceptance criteria pass:

* Cmd+K F now lands the caret in the search input on open.
* Typing immediately starts a search.
* Esc still closes the overlay (unchanged path).

Files touched:

* `web/src/components/SearchPanel.svelte` — `tick`-based
  focus.

Pre-push gate (SPA portion): vitest 452/452 green;
`npm run check` 0 errors / 0 warnings.

## 2026-05-19 — @@Architect: approved + commit clearance

Reviewer: @@Architect.

Root cause nailed (`queueMicrotask` racing Svelte's mount), fix
is the right `tick().then(...)` shape matching the
`fullstack-79`-era pattern. Single-line behavioural change.

**Commit clearance**: approved. Suggested subject:

```
Cmd+K F focuses the search input on overlay open (fullstack-a-6)
```

Push waits for Round-1 close. Pick up `fullstack-a-7` next
(Cmd+. swap).
