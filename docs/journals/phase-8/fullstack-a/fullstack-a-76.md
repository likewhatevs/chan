# fullstack-a-76 — SPA Settings surface for pre-flight feature toggles (BGE + reports)

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched
Round: 2 wave-3
Dependency: `systacean-27`

## Goal

Add SPA Settings UI for the per-drive BGE + chan-
reports feature toggles. Mirrors the pre-flight
screen from `fullstack-b-28` so users can flip the
toggles after initial drive setup.

## Reference

[`../architect/round-2-plan.md`](../architect/round-2-plan.md)
§"Pre-flight feature toggles" — "Enable later (via
Settings or CLI)".

## Scope

* Add a "Features" section to the existing Settings
  overlay surfacing BGE + reports toggles.
* Read state from chan-drive config via existing
  drive-config IPC.
* On toggle ON: persist + trigger incremental
  indexing (chan-drive handles the indexing pass
  per `-27`).
* On toggle OFF: persist; chan-drive stops the
  indexing pass.
* Inline help text per toggle.

## Acceptance

1. Settings shows Features section with two
   toggles.
2. Toggle state reflects current drive config.
3. Flipping persists + triggers indexing as
   appropriate.
4. Web build + chan-desktop both work.

### Tests

Vitest pins for the Settings rendering + toggle
handler.

### Gate

`npm test` / `check` / `build` green.

## Coordination

* @@FullStackA lane. SPA-only.
* Consumes `systacean-27` API.

## Authorization

Yes for Settings SPA + tests + task tail + outbound.

## Numbering

This is `-a-76`.
