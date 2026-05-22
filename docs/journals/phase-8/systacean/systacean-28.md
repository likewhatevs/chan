# systacean-28 — chan config currency audit (Round-2 item 5)

Owner: @@Systacean
Cut: 2026-05-22 by @@Architect
Status: dispatched
Round: 2 wave-3

## Goal

Audit chan's config surface for currency: identify
stale / dead config fields, document the current
config schema, ensure all live fields are reachable
from the appropriate surfaces (CLI / Settings UI /
pre-flight). Clean up dead config.

## Reference

[`../architect/round-2-plan.md`](../architect/round-2-plan.md)
"Backlog item 5 — chan config currency audit"
(referenced at line 37: "Then 5 (config audit)").

## Scope

### Audit

1. Walk chan-drive's drive-config schema +
   chan-server's app-config + chan-desktop's
   per-window config.
2. For each field: is it CONSUMED? Reachable via
   CLI / Settings / pre-flight?
3. Identify dead fields (consumed nowhere).
4. Identify fields that aren't reachable via any
   user-facing surface.

### Cleanup

1. Remove dead fields (backward-compat: serde-skip-
   when-missing already protects pre-existing
   drives).
2. Surface unreachable-but-needed fields via
   appropriate UI / CLI.
3. Document the final config schema in a single
   reference doc.

## Acceptance

1. Audit verdict appended to task tail listing
   each config field + its consumers + its
   user-facing surface.
2. Dead fields removed (with backward-compat for
   old-shape config files).
3. Unreachable-but-needed fields surfaced.
4. Reference doc: `docs/config-reference.md` (or
   similar) listing the canonical schema.

### Tests

* Config round-trip tests for both removed and
  preserved fields.
* Backward-compat: old-shape configs load cleanly.

### Gate

`cargo fmt / clippy / test`; `RUSTFLAGS="-D warnings"
cargo build --no-default-features` green.

## Coordination

* @@Systacean lane (chan-drive config primary).
* May touch chan-server / chan-desktop config if
  field cleanup spans crates. Scope-poke if SPA
  config changes needed.
* Atomic-audit-commit.

## Authorization

Yes for chan-drive config schema + chan-server +
chan-desktop config paths + new reference doc +
tests + task tail + outbound.

## Numbering

This is `-28`.
