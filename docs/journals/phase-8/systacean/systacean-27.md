# systacean-27 — chan-drive pre-flight feature toggle persistence + BOOT process integration

Owner: @@Systacean
Cut: 2026-05-22 by @@Architect
Status: dispatched
Round: 2 wave-3

## Goal

Implement chan-drive's pre-flight feature toggle
persistence (BGE-small + chan-reports both
configurable per-drive) + the BOOT process that
kicks off optional indexing layers based on the
toggles.

## Reference

[`../architect/round-2-plan.md`](../architect/round-2-plan.md)
§"Pre-flight feature toggles" (line 193+) and §"BOOT
process" (line 222+).

## Scope (chan-drive backend)

### Per-drive config schema

* Extend drive config with `features: { bge: bool,
  reports: bool }` (or similar shape). Both default
  `false` (lean drive; BM25-only).
* Persist via existing config-write infrastructure
  (atomic write parity).

### BOOT process

* On drive open: read the config; if any toggle is
  ON, kick off the relevant indexing pass alongside
  the existing BM25 walk.
* Idempotent — boot doesn't re-index already-indexed
  content; just resumes where the last pass left off.

### Feature flag plumbing

* `Drive::feature_bge_enabled() -> bool`
* `Drive::feature_reports_enabled() -> bool`
* `Drive::set_feature_bge(enabled: bool)` /
  `set_feature_reports(enabled: bool)` — persists
  + triggers an incremental indexing pass when
  flipped ON.

### CLI surface

CLI subcommands to enable/disable per
[`../architect/round-2-plan.md`](../architect/round-2-plan.md)
"Enable later via Settings or CLI". e.g.
`chan features bge enable <drive>` /
`chan features reports enable <drive>`.

## Out of scope

* Pre-flight UI (chan-desktop) — separate task
  `fullstack-b-28`.
* Settings surface (SPA) — separate task
  `fullstack-a-76`.

## Acceptance

1. Config schema persists `features` field
   (backward-compat: missing field defaults to
   both off).
2. BOOT kicks off BGE indexing when `bge: true`.
3. BOOT kicks off reports indexing when `reports:
   true`.
4. Flipping ON later triggers incremental indexing.
5. Flipping OFF stops the indexing pass (graceful).
6. CLI subcommands enable/disable both flags per
   drive.

### Tests

* Config round-trip with features field.
* BOOT triggers each indexing pass.
* CLI enable/disable end-to-end.
* Backward-compat: pre-`-27` drives load with
  both features off.

### Gate

`cargo fmt / clippy / test`; `RUSTFLAGS="-D warnings"
cargo build --no-default-features` green.

## Coordination

* @@Systacean lane.
* `fullstack-b-28` (chan-desktop pre-flight UI) +
  `fullstack-a-76` (Settings surface) consume this
  API.
* Atomic-audit-commit.

## Authorization

Yes for chan-drive config + BOOT + chan CLI
subcommand surface + tests + task tail + outbound.

## Numbering

This is `-27`.
