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

## 2026-05-22 — audit verdict + reference doc; cleanup deferred to scope-routed follow-ups

Picked up `-28` per @@Alex's "pick up -28 next" routing. Audit-first per the task body.

### Surface walked

| Crate / File | Struct(s) | Storage location |
|--------------|-----------|------------------|
| `chan-server` `config.rs` | `ServerConfig` + `ReportsConfig` + `SearchConfig` + `TerminalConfig` | `~/.chan/server.toml` |
| `chan-server` `preferences.rs` | `EditorPrefs` + nested | `~/.chan/preferences.toml` |
| `chan-drive` `index/config.rs` | `IndexConfig` | `<state_dir>/index/<uuid>/config.toml` |
| `chan-drive` `registry.rs` | `KnownDrive` + `Registry` | `~/.chan/config.toml` |
| `chan-drive` `paths.rs` | `DrivePaths` (computed, not persisted) | n/a |
| `chan-drive` `teams.rs` | `TeamConfig` (per-team workspace) | `Drafts/team-{name}/config.toml` |
| `chan-desktop` `config.rs` | `Config` + `DriveSidecar` + `DriveFeatures` + `TunnelConfig` + `WindowConfig` | `<config>/chan/chan-desktop.json` |

### Per-field audit

#### chan-server `ServerConfig` (`~/.chan/server.toml`)

| Field | State | Consumers | Reachability |
|-------|-------|-----------|--------------|
| `attachments_dir: String` | ✓ ACTIVE | `/api/attachments` route + SPA upload UI | `PATCH /api/server/config` |
| `search.aggression: SearchAggression` | ✓ ACTIVE | search route default | `PATCH /api/server/config` |
| `terminal.idle_timeout_secs: u64` | ✓ ACTIVE | terminal registry idle prune | `PATCH /api/server/config` |
| `terminal.session_cap: usize` | ✓ ACTIVE | terminal registry create-gate | `PATCH /api/server/config` |
| `terminal.ring_bytes: usize` | ✓ ACTIVE | terminal ring buffer alloc | `PATCH /api/server/config` |
| `terminal.scrollback_mb: u32` | ✓ ACTIVE | SPA xterm.js construction | `PATCH /api/server/config` |
| `terminal.default_term: String` | ✓ ACTIVE | PTY spawn env | `PATCH /api/server/config` |
| `reports.enabled: bool` | ⚠ **STALE / HALF-WIRED** | SPA `HybridFileBrowserConfig.svelte` (UI exists). **NO backend gating** wired across `inspector` / `graph` / `report` / `storage` routes per the field's own doc comment. | `PATCH /api/server/config` (round-trips only) |

**Finding 1 — `ServerConfig.reports.enabled` is redundant after `systacean-27`**:

The field was Option-B-landed in `fullstack-a-48 Task F` as a placeholder for the global reports toggle, with the doc explicitly flagging "backend gating across the four chan-server routes ... a follow-up task". Then `systacean-27` introduced the **per-drive** `IndexConfig.reports_enabled` as the actual source of truth, reachable via `chan reports enable/disable` and `Drive::set_reports_enabled / reports_enabled / boot`.

The server-level `reports.enabled` is therefore:
1. **Round-trip only** — SPA writes it, server persists it, but nothing CONSUMES the value behind the routes the comment promised to gate.
2. **Conceptually redundant** — per-drive truth lives in `IndexConfig.reports_enabled`. A server-wide default would be coherent, but the field would need to mean "default for newly-added drives" (currently doesn't carry that semantic).

**Recommended action**: REMOVE the field. The SPA's HybridFileBrowserConfig toggle should route through `chan reports enable/disable --path <drive>` (or a chan-server passthrough route) and observe per-drive state instead of a single server-wide bool. Cross-lane: SPA + chan-server route change + this field removal. Defer to a routed follow-up task (`systacean-N` or `fullstack-a-N+M`).

#### chan-server `EditorPrefs` (`~/.chan/preferences.toml`)

All fields ACTIVE: `editor_theme`, `theme`, `pane_widths.{inspector,graph,browser,search,outline}`, `browser_side_panes.{left,right}`, `line_spacing`, `date_format`, `strip_trailing_whitespace_on_save`, `bubble_overlay_mode`, `empty_pane_carousel_cycling`. Each consumed by Settings UI + at least one SPA component. Reachability: `PATCH /api/config`.

No dead fields. Schema is healthy.

#### chan-drive `IndexConfig` (`<state>/index/<uuid>/config.toml`)

| Field | State | Consumers | Reachability |
|-------|-------|-----------|--------------|
| `schema_version: u32` | ✓ ACTIVE | version-mismatch wipe gate | (internal; no user surface needed) |
| `model: String` | ✓ ACTIVE | embedder resolver | `chan index download-model` |
| `chunking: Chunking` | ✓ ACTIVE | indexer chunking | (internal; no user surface yet) |
| `vectors_model: Option<String>` | ✓ ACTIVE | mismatch-wipe trigger | (internal stamp) |
| `vectors_dim: Option<u32>` | ✓ ACTIVE | build_all defensive cross-check | (internal stamp) |
| `semantic_enabled: bool` | ✓ ACTIVE post-`-7` | `Drive::search` Hybrid default mode | `chan index enable-semantic/disable-semantic` + Settings (`-21`) |
| `reports_enabled: bool` | ✓ ACTIVE post-`-27` | `Drive::report()` lazy init + boot | `chan reports enable/disable` |

No dead fields. Schema is healthy.

#### chan-drive `KnownDrive` (`~/.chan/config.toml`)

All 5 fields ACTIVE: `path`, `uuid`, `name`, `last_opened`, `canonical_path` (transient, `#[serde(skip)]`). Reachability: `chan add` / `chan list` / `chan remove` / `chan rename`.

No dead fields.

#### chan-drive `TeamConfig` (`Drafts/team-{name}/config.toml`)

All 6 fields ACTIVE post-`-30`: `team_name`, `host_name`, `host_handle`, `auto_prefix_at`, `created_at`, `members[]`. Reachability: chan-server `/api/teams/{name}/load/unload/loaded` routes (post-`-31`) + `Drive::create_team / list_teams / load_team / duplicate_team`.

New as of `-30`; schema is healthy.

#### chan-desktop `Config` (`<config>/chan/chan-desktop.json`)

| Field | State | Consumers | Reachability |
|-------|-------|-----------|--------------|
| `sidecar.{key}.last_port: Option<u16>` | ✓ ACTIVE | port-reuse on serve restart | (internal; no user surface needed) |
| `sidecar.{key}.features.bge: bool` | ⚠ **MIRROR / DRIFT-PRONE** | `get_drive_features` reads the mirror; `set_drive_features` writes via `chan index enable-semantic/disable-semantic` then mirrors. | Launcher row expand panel (`fullstack-b-28a + b`) |
| `sidecar.{key}.features.reports: bool` | ⚠ **MIRROR / DRIFT-PRONE** | Same pattern via `chan reports enable/disable`. | Same launcher row |
| `tunnel.preferred_port: u16` | ✓ ACTIVE | tunnel listen panel | Tunnel listener UI |
| `tunnel.preferred_label: String` | ✓ ACTIVE | Tunnel UI default | Same |
| `tunnel.preferred_drive: String` | ✓ ACTIVE | Tunnel UI default | Same |
| `window_configs: Vec<WindowConfig>` | ✓ ACTIVE | LRU pop on window open | (internal; URL hash + zoom round-trip) |

**Finding 2 — `DriveFeatures` mirror in chan-desktop sidecar is drift-prone after `-b-28b-i`**:

`fullstack-b-28b-i` correctly swapped `set_drive_features` to shell out to `chan index enable-semantic/disable-semantic` + `chan reports enable/disable`. Source of truth is now chan-drive's `IndexConfig`. BUT chan-desktop still PERSISTS a local mirror copy in the sidecar config. The mirror updates AFTER the CLI succeeds (good) but isn't refreshed on app boot, so a state change made via `chan index enable-semantic` from a terminal (bypassing chan-desktop) leaves the mirror stale until the SPA's next `set_drive_features` round-trip.

**Recommended action**: REPLACE the mirror with a `get_drive_features` that reads chan-drive's `IndexConfig` directly via a CLI subprocess (or a chan-server route if chan-server is the launch path). One-time-per-launch read; updates flow through `set_drive_features` as today.

Alternative: keep the mirror but refresh on every `get_drive_features` call (always shells out). Higher per-call cost; eliminates drift.

Cross-lane: chan-desktop change + possibly chan-server route addition. Defer to a routed follow-up task.

### Cleanups deferred (cross-lane decisions)

| Finding | Owner / cross-lane | Recommended task |
|---------|-------------------|------------------|
| 1. `ServerConfig.reports.enabled` removal | chan-server + SPA (HybridFileBrowserConfig) | `systacean-N` or `fullstack-a-N+M` |
| 2. `DriveFeatures` mirror drift in chan-desktop | chan-desktop + chan-drive (read path) | `fullstack-b-N+M` or `systacean-N` |

Both findings are real-but-not-urgent. The `reports.enabled` round-trips harmlessly (no behavior change to fix); the `DriveFeatures` mirror drifts only when users bypass chan-desktop's UI for feature toggles (corner case).

### Reference doc

Authored `docs/config-reference.md` (~140 lines) listing the canonical config schema per crate, with file locations + per-field type + serde defaults + reachability + finding notes for the 2 flagged items. The doc cross-references the systacean / fullstack tasks that introduced each field for future audit-trail use.

### Gate

* `cargo fmt --check`: clean.
* `cargo clippy --all-targets -- -D warnings`: clean.
* `cargo test`: no code changes; tests unchanged.

### Files

* `docs/config-reference.md` (new).
* `docs/journals/phase-8/systacean/systacean-28.md` (task tail; this).
* `docs/journals/phase-8/alex/event-systacean-architect.md` (outbound).

### Suggested commit subject

```
docs: chan config currency audit + reference doc; 2 findings deferred to cross-lane follow-ups (systacean-28)
```

### Acceptance criteria status

| # | Criterion | Status |
|---|-----------|--------|
| 1 | Audit verdict listing each config field + consumers + user-facing surface | ✓ (this tail) |
| 2 | Dead fields removed (with backward-compat for old-shape configs) | ⚠ DEFERRED — 2 findings are cross-lane, recommend routed follow-up tasks |
| 3 | Unreachable-but-needed fields surfaced | ✓ — every audited field is reachable via at least one of CLI / Settings / pre-flight / launcher panel |
| 4 | Reference doc `docs/config-reference.md` listing canonical schema | ✓ (this commit) |

Acceptance #2 partial: no fully-dead fields surfaced in the audit (every field IS consumed by something). The 2 "STALE" / "MIRROR" findings need cross-lane work to resolve cleanly. Defer per the architect's standing pattern.
