# fullstack-a-77c — Screensaver no-PIN lockout: any-input unlocks when no PIN set (slice 3)

Owner: @@FullStackA
Cut: 2026-05-23 by @@Architect
Status: dispatched

## Goal

Close the no-PIN lockout @@WebtestA caught: if user
enables screensaver without setting a PIN, they get
locked out with no way to dismiss. The helper text
already says "Any input unlocks" — make the
mechanism match.

## Reference

@@WebtestA's walk (`bdfa657`) side observation:

* `PATCH /api/screensaver/state` with `{enabled:
  true, timeout_secs: 3}` + no PIN set.
* After 3s overlay fires.
* PIN form rendered + helper text says "Any input
  unlocks."
* But typing / clicking does NOT dismiss the
  overlay → user is locked out + Settings is
  unreachable.

## Routing decision: option 1 (any-input unlocks when no PIN)

@@WebtestA surfaced 3 options:

1. **Any-input unlocks when no PIN set** — matches
   helper text; lowest-friction default.
2. Refuse to enable screensaver without a PIN +
   update helper text.
3. "Disable lock" button in overlay when no PIN.

**Routed option 1** — matches the helper text's
implicit promise, no behavior surprise. Users who
want PIN protection set one explicitly; users who
just want a dim timeout get that without commitment.

## Fix shape

In `ScreensaverOverlay.svelte` (or the state
machine):

1. Read `pin_set` from `/api/screensaver/state` at
   overlay mount.
2. If `pin_set === false`:
   * Hide the PIN entry form.
   * Show "Press any key to unlock" message
     (already-present helper).
   * On ANY keypress / click anywhere on the
     overlay → dismiss the lock state machine.
3. If `pin_set === true`: existing PIN verify
   flow unchanged.

## Acceptance

1. **Screensaver enabled, no PIN set** → overlay
   appears, ANY keypress / click dismisses it.
2. **Screensaver enabled, PIN set** → existing PIN
   verify flow unchanged.
3. **No regression on `-a-77 slice 2`** mechanism.
4. **No regression on PBKDF2 verify path**.

### Tests

Vitest pin on the state machine's pin_set === false
branch (any-input → dismiss).

### Gate

`npm test` / `check` / `build` green.

## Coordination

* @@FullStackA. SPA-only.
* Tiny — state machine branch + render conditional.

## Authorization

Yes for `web/src/state/screensaver.svelte.ts` +
`web/src/components/ScreensaverOverlay.svelte` +
tests + task tail + outbound.

## Numbering

This is `-a-77 slice 3`. Tracked as `-77c` for
filename clarity.

## Out of scope

* Settings escape from overlay (Mod+, or similar)
  — defer to Round-3 if still needed after the
  any-input unlock lands.
* Helper text rewording (current text is correct
  once the mechanism matches).

---

## Implementation log

Date: 2026-05-23.

### Shape applied (option 1)

* New `unlockWithoutPin()` exported from
  `web/src/state/screensaver.svelte.ts`.
  Guards on `screensaver.pin_set` (no-op
  when a PIN is set, so the helper is
  safe to wire unconditionally), flips
  `screensaver.locked = false`, rearms
  the inactivity timer.
* `ScreensaverOverlay.svelte`:
  * Imported `unlockWithoutPin`.
  * Added `onBackdropKey(e)` +
    `onBackdropPointer()` — both gate on
    `pin_set` + dispatch
    `unlockWithoutPin()`.
  * Backdrop opener now carries
    `onkeydown` + `onclick` + `tabindex=-1`.
    Svelte's a11y warnings for non-
    interactive elements with handlers
    suppressed via `svelte-ignore`
    comments (the dialog role + aria
    attrs already telegraph the
    interactive intent).
  * Markup branches on
    `{#if screensaver.pin_set}`: PIN form
    (input + Unlock button + error pin)
    on the true arm; "Press any key or
    click to unlock" message on the
    false arm. No PIN entry rendered
    when there's no PIN.

### Tests

New file `web/src/state/screensaverNoPin.test.ts`
with 8 architectural pins:

* `unlockWithoutPin` exported.
* Guards on `pin_set` truthy.
* Flips locked + rearms timer.
* Overlay imports the helper.
* `onBackdropKey` + `onBackdropPointer`
  guard + dispatch.
* `onkeydown` + `onclick` wired on the
  backdrop.
* Markup branches on `pin_set` inside
  the locked block, with the no-PIN
  message present.
* PIN input lives inside the
  `pin_set === true` arm only (no
  PIN form rendered without a PIN).

Updated slice-2 pin in
`screensaverMachine.test.ts` to allow
the new backdrop attrs (the strict
literal opening-tag regex would have
failed otherwise).

### Acceptance walk (architectural)

| Case                         | Behavior        |
|------------------------------|-----------------|
| `pin_set=false`, keydown     | dismiss         |
| `pin_set=false`, click       | dismiss         |
| `pin_set=true`, backdrop key | PIN form owns input |
| `pin_set=true`, PIN entry    | unchanged       |

Behavioral walk handed back to
@@WebtestA (the original
finder).

### Gate

* `svelte-check` → 0/0.
* `vitest` → 110 files, **1124 passing**
  (+9 from the slice-3 baseline; 8 new
  no-PIN pins + 1 updated slice-2 pin).
* `npm run build` → clean.
* `cargo fmt --check` + `clippy
  --all-targets -- -D warnings` → clean
  (no Rust delta this slice; ran for
  pre-push hygiene).

### Suggested commit subject

```
Screensaver: any-input unlocks when no PIN set (fullstack-a-77c)
```

### Files (per-path)

* `web/src/state/screensaver.svelte.ts`
* `web/src/components/ScreensaverOverlay.svelte`
* `web/src/state/screensaverNoPin.test.ts` (new)
* `web/src/state/screensaverMachine.test.ts`
* `docs/journals/phase-8/fullstack-a/fullstack-a-77c.md`

Auth held. Standing by for cleared
push + next dispatched task.
