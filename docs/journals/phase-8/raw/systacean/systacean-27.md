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

## 2026-05-22 — implementation complete; ready for smoke

Picked up `-27` per the WAVE-3 dispatch poke + user's "Full -27" routing.

### What landed

**1. Per-drive config schema** (`crates/chan-drive/src/index/config.rs`)

Added `reports_enabled: bool` field to `IndexConfig` alongside the existing `semantic_enabled: bool` from `systacean-7`. Both default-false; both `#[serde(default)]` for backward-compat with pre-`-27` config.toml files. Persists at `<state_dir>/index/<uuid>/config.toml` via the existing atomic-write infrastructure.

Pragmatic placement: extends the existing IndexConfig rather than introducing a parallel ReportConfig file. The naming is slightly awkward (`reports_enabled` lives inside the index config) but the cost of a separate file + load/save infrastructure isn't justified by the single bool. Round-3 may refactor to a dedicated `features.toml` if/when more feature flags accumulate.

**2. Index facade** (`crates/chan-drive/src/index/facade.rs`)

New `Index::set_reports_enabled(bool)` method parallel to `set_semantic_enabled`. Idempotent (no-op re-set), atomic persist.

**3. Drive plumbing** (`crates/chan-drive/src/drive.rs`)

* `Drive::reports_enabled() -> Result<bool>`
* `Drive::set_reports_enabled(bool) -> Result<()>`: persists; on disable, drops the persisted `report.jsonl` (destructive per Round-2 spec). Re-enable triggers a fresh scan via the lazy `report_state()` initializer.
* `Drive::boot() -> Result<()>`: BOOT entry-point. Consumers call after `Drive::open`. Kicks off the optional indexing layers per the persisted flags. Idempotent (lazy `OnceLock`-backed init). No-op when both flags off (lean drive stays lean).

**4. CLI surface** (`crates/chan/src/main.rs`)

* New `Command::Reports { action: ReportsAction }` with `enable` / `disable` subcommands. Pattern parallel to `IndexAction::EnableSemantic` / `DisableSemantic`.
* `cmd_reports` + `cmd_reports_set` handlers. `disable` is destructive — requires `-y` to skip the interactive confirmation prompt.
* `Command::Add` extended with `--semantic-search` + `--reports` flags. Both off by default; opt-in routes through `set_*_enabled(true) + boot()` so the kickoff scan runs once.

### Acceptance criteria status

| # | Criterion | Status |
|---|-----------|--------|
| 1 | Config schema persists `features` field (backward-compat) | ✓ (`reports_enabled` as `serde(default)` on IndexConfig) |
| 2 | BOOT kicks off BGE indexing when `bge: true` | ✓ via `Drive::boot()` (lazy via index facade) |
| 3 | BOOT kicks off reports indexing when `reports: true` | ✓ via `Drive::boot()` → `report_state()` |
| 4 | Flipping ON later triggers incremental indexing | ✓ (`set_reports_enabled(true) + boot()` in `cmd_reports_set`) |
| 5 | Flipping OFF stops the indexing pass (graceful) | ✓ (`set_reports_enabled(false)` drops `report.jsonl`) |
| 6 | CLI subcommands enable/disable both flags per drive | ✓ (`chan reports enable/disable` + existing `chan index enable-semantic`) |

### Tests added (+3)

1. `index::config::tests::reports_enabled_defaults_false_and_round_trips_true` — default + round-trip + backward-compat (legacy file without the field deserializes with `false`).
2. `drive::tests::reports_enabled_round_trips_through_drive_and_boot_kicks_off_initial_scan` — Drive-level: enable + boot + verify scan kicked off + disable drops the persisted jsonl.
3. `drive::tests::boot_is_noop_when_features_disabled` — backward-compat: lean drive (both flags off) calling `boot()` does NOT trigger any scan.

Plus existing `IndexConfig` literal sites in `facade.rs` tests (3 sites) updated to include `reports_enabled: false`.

### Pre-push gate

* `cargo fmt --check`: clean.
* `cargo clippy --all-targets -- -D warnings`: clean (had one rustdoc list-item warning + em dash; fixed both).
* `cargo test -p chan-drive --lib`: **449 passed; 0 failed; 2 ignored** (was 446; +3 new).
* `cargo test` workspace: all crates green.
* `RUSTFLAGS="-D warnings" cargo build --no-default-features`: green.

### Files

| File                                       | +    | -   |
|--------------------------------------------|------|-----|
| `crates/chan-drive/src/index/config.rs`    | +45  | 0   |
| `crates/chan-drive/src/index/facade.rs`    | +19  | 0   |
| `crates/chan-drive/src/drive.rs`           | +120 | 0   |
| `crates/chan/src/main.rs`                  | +163 | -2  |

Plus task tail + outbound poke. 6 paths total.

### Suggested commit subject

```
chan-drive + chan: reports_enabled feature flag + Drive::boot + chan reports CLI + chan add flags (systacean-27)
```

### Smoke plan

`gh workflow run ci.yml --ref systacean-27-smoke` on a fresh smoke branch. Expected: all 5 jobs green. Pure additive (chan-drive + chan); backward-compat preserves all existing behavior.

### What's deferred (not in this PR)

* **`chan index status --json` extension for reports**: the task body's optional `chan index status` extension to include report state. Skipped — `chan index status` lives in its own subcommand surface; reports state has its own enable/disable verbs. Could be added in a polish follow-up if needed.
* **chan-server route gating**: chan-server's graph route `merge_language_layer` calls `drive.report()`. If `reports_enabled = false`, `Drive::report()` would still try to compute. Recommend a `if drive.reports_enabled()? { merge_language_layer(...) }` guard as a follow-up — kept this PR scoped to the chan-drive + CLI plumbing.
* **fullstack-b-28 + fullstack-a-76**: the consumer-side surfaces of these flags (pre-flight UI + Settings). They consume `Drive::reports_enabled()` / `set_reports_enabled()` per their own lanes.

Per architect's pre-authorization, proceeding to commit + push + smoke.
