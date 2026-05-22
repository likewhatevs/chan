# fullstack-b-28 — chan-desktop launcher pre-flight UX (surfaces BGE + chan-reports toggles)

Owner: @@FullStackB
Cut: 2026-05-22 by @@Architect
Status: dispatched
Round: 2 wave-3
Dependency: `systacean-27`

## Goal

Extend chan-desktop's drive launcher / pre-flight
screen to surface the BGE-small + chan-reports
feature toggles. Both off by default; user can
enable at pre-flight OR via Settings (separate
`-a-76` task).

## Reference

[`../architect/round-2-plan.md`](../architect/round-2-plan.md)
§"Pre-flight feature toggles" (line 193+) +
§"UI surface (item 2 pre-flight report + Settings)"
(line 222+).

`fullstack-b-13` (chan-desktop launcher pre-flight UX
foundation) is the surface to extend.

## Scope

* Add per-drive toggle UI for `bge` + `reports` to
  the pre-flight screen.
* Both default OFF.
* Persist the user's choice via the chan-drive
  config API (`systacean-27` provides).
* Tooltip / info-button explaining what each toggle
  enables (BGE = semantic search; reports = file
  classification + stats).

## Acceptance

1. **Pre-flight screen** shows BGE + reports toggle
   rows.
2. **Default OFF**: clean install opens drive lean.
3. **Toggle persistence**: user enables BGE →
   reflected in chan-drive config → BOOT picks up
   on next launch.
4. **Tooltip / info** describes each feature.

### Tests

Vitest pins + chan-desktop runtime test under
standing perm.

### Gate

`cargo` + `npm` gates green.

## Coordination

* @@FullStackB lane.
* Depends on `systacean-27` API. Can stub-shell the
  toggle persistence + wire when `-27` lands.

## Authorization

Yes for chan-desktop launcher + SPA pre-flight UI +
tests + task tail + outbound.

## Numbering

This is `-b-28`.
