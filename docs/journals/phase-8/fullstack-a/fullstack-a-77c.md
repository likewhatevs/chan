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
