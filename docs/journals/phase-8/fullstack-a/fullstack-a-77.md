# fullstack-a-77 — Screensaver with PIN unlock (Round-2 item 3)

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched
Round: 2 wave-3

## Goal

Implement screensaver with PIN unlock per Round-2
item 3. Local-only screensaver protecting the drive
contents from over-the-shoulder viewing when the
user steps away.

## Reference

[`../architect/round-2-plan.md`](../architect/round-2-plan.md)
+ §"Backlog item 3 — screensaver" (referenced at
line 53 + 76).

## Scope

### Behavior

* After N minutes of inactivity (configurable per
  drive), screensaver overlay appears.
* Drive contents hidden behind a blank or themed
  overlay.
* User unlocks via PIN (numeric or any user-chosen
  string).
* PIN stored hashed in chan-drive metadata.
* PIN can be set / changed in Settings.
* On unlock, drive view restores.

### Storage

* PIN hash (e.g. argon2 or scrypt) stored in drive
  metadata via chan-drive config.
* No external crypto / no over-the-network — local
  hash only per round-2-plan: "isn't needed for a
  local-only screensaver PIN".

### Triggers

* Inactivity timeout (default 5 min; configurable).
* Manual "Lock now" affordance (chord OR menu
  entry).
* On window blur / tab background? Implementer's
  call — most conservative is inactivity-only.

## Acceptance

1. Settings shows screensaver enable/disable +
   timeout + PIN setup.
2. After timeout, screensaver overlay covers drive
   contents.
3. PIN entry unlocks.
4. Wrong PIN: shake + error feedback; no rate
   limit needed (local-only).
5. Manual "Lock now" works.

### Tests

Vitest pins for the timeout logic + overlay state
+ PIN verification.

### Gate

`npm test` / `check` / `build` green.

## Coordination

* @@FullStackA SPA primary.
* If chan-drive needs config schema additions for
  PIN hash + screensaver settings, scope-poke to
  @@Systacean OR bundle if minimal.
* Atomic-audit-commit.

## Authorization

Yes for SPA screensaver + Settings + chord
handlers + tests + task tail + outbound. If
chan-drive PIN storage needs new config field,
scope-poke first.

## Numbering

This is `-a-77`.

## Out of scope

* Network-based auth.
* Multi-user / per-user PINs.
* Drive encryption (separate concern).
