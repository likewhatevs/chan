# Chan Config Reference

Canonical schema for every persisted config in chan, produced as
the deliverable of `systacean-28` (phase-8 config currency audit).

This doc tracks **what gets persisted, where, and who consumes
it**. Per-field rows note serde defaults, the user-facing
surface (CLI subcommand / Settings field / launcher panel),
and any open findings flagged by the audit.

When adding a new persisted field: extend the relevant section
here in the same commit that lands the schema change so this
reference stays in lockstep with the code.

## chan-server

### `~/.chan/server.toml` — `ServerConfig`

Source: `crates/chan-server/src/config.rs`.

| Field | Type | Default | Reachability | Consumers |
|-------|------|---------|--------------|-----------|
| `attachments_dir` | `String` | `"attachments"` | `PATCH /api/server/config` | `/api/attachments` route + SPA upload UI |
| `search.aggression` | `SearchAggression` | `Balanced` | `PATCH /api/server/config` | search route default mode |
| `terminal.idle_timeout_secs` | `u64` | `1800` (30 min) | `PATCH /api/server/config` | terminal registry idle prune |
| `terminal.session_cap` | `usize` | `32` | `PATCH /api/server/config` | terminal registry create-gate |
| `terminal.ring_bytes` | `usize` | `1 << 20` (1 MB) | `PATCH /api/server/config` | terminal ring buffer alloc |
| `terminal.scrollback_mb` | `u32` | `50` (clamped `10..=500`) | `PATCH /api/server/config` | SPA xterm.js scrollback line cap |
| `terminal.default_term` | `String` | `"xterm-256color"` | `PATCH /api/server/config` | PTY spawn `TERM` env |
| `reports.enabled` | `bool` | `true` | `PATCH /api/server/config` | ⚠ **STALE** — round-trips only; per-drive `IndexConfig.reports_enabled` is the real source post-`systacean-27`. See systacean-28 finding 1. |

### `~/.chan/preferences.toml` — `EditorPrefs`

Source: `crates/chan-server/src/preferences.rs`.

| Field | Type | Reachability | Consumers |
|-------|------|--------------|-----------|
| `editor_theme` | `EditorTheme` | `PATCH /api/config` | Settings → Editor → theme selector |
| `theme` | `ThemeChoice` | `PATCH /api/config` | Settings → Appearance |
| `pane_widths.inspector` | `u32` | drag-resize | resize handle persistence |
| `pane_widths.graph` | `u32` | drag-resize | same |
| `pane_widths.browser` | `u32` | drag-resize | same |
| `pane_widths.search` | `u32` | drag-resize | same |
| `pane_widths.outline` | `u32` | drag-resize | same |
| `browser_side_panes.left` | `bool` | FB toggle | left side-pane visibility |
| `browser_side_panes.right` | `bool` | FB toggle | right side-pane visibility |
| `line_spacing` | `LineSpacing` | Settings | editor line-height |
| `date_format` | `String` | Settings | date rendering across SPA |
| `strip_trailing_whitespace_on_save` | `bool` | Settings | editor save hook |
| `bubble_overlay_mode` | `BubbleOverlayMode` | Bubble menu | overlay rendering |
| `empty_pane_carousel_cycling` | `bool` | Settings | empty-pane behavior |

## chan-drive

### `~/.chan/config.toml` — `Registry` (`KnownDrive[]`)

Source: `crates/chan-drive/src/registry.rs`.

Per-drive entry persisted at registration time:

| Field | Type | Default | Reachability | Consumers |
|-------|------|---------|--------------|-----------|
| `path` | `PathBuf` | required | `chan add <path>` | drive enumeration / open |
| `uuid` | `String` (16 hex) | minted on add | (internal identity) | per-drive sidecar keying |
| `name` | `Option<String>` | basename | `chan add --name` / `chan rename` | window title + UI |
| `last_opened` | `DateTime<Utc>` | now() | `chan list` | recency sort |
| `canonical_path` | transient (`#[serde(skip)]`) | n/a | (internal cache) | symlink-stable comparison |

### `<state_dir>/index/<uuid>/config.toml` — `IndexConfig`

Source: `crates/chan-drive/src/index/config.rs`.

| Field | Type | Default | Reachability | Consumers |
|-------|------|---------|--------------|-----------|
| `schema_version` | `u32` | `SCHEMA_VERSION` const | (internal) | version-mismatch wipe gate |
| `model` | `String` | `BAAI/bge-small-en-v1.5` | `chan index download-model --model` | embedder resolver |
| `chunking` | `Chunking` enum | `Headings` | (internal; no user surface yet) | indexer chunking strategy |
| `vectors_model` | `Option<String>` | `None` | (internal stamp) | mismatch-wipe trigger on `Index::open` |
| `vectors_dim` | `Option<u32>` | `None` | (internal stamp) | build-time defensive cross-check |
| `semantic_enabled` | `bool` | `false` | `chan index enable-semantic/disable-semantic --path <drive>` + Settings (`fullstack-a-21`) | `Drive::search` Hybrid default mode |
| `reports_enabled` | `bool` | `false` | `chan reports enable/disable --path <drive> [-y]` + `chan add --reports` | `Drive::report()` lazy init + `Drive::boot()` |

### `Drafts/team-{name}/config.toml` — `TeamConfig`

Source: `crates/chan-drive/src/teams.rs` (post-`systacean-30`).

| Field | Type | Default | Reachability | Consumers |
|-------|------|---------|--------------|-----------|
| `team_name` | `String` | required | `chan-server /api/teams/{name}/load + .../unload + GET .../loaded` (`systacean-31`) | team identification |
| `host_name` | `String` | required | (set at create time) | UI rendering |
| `host_handle` | `String` | required | (set at create time) | @@-prefix policy |
| `auto_prefix_at` | `bool` | `true` | (set at create time; future Settings) | bubble overlay @@-auto-prefix |
| `created_at` | `String` (ISO 8601) | required | (set at create time) | sort + display |
| `members[]` | `Vec<Member>` | empty | (future Settings) | team roster + position grid |

`Member`: `handle: String`, `command: String`, `env: BTreeMap<String, String>`, `is_lead: bool`, `position: Option<Position>`.

`Position`: `row: u32`, `col: u32` (airplane-grid coordinate).

## chan-desktop

### `<config>/chan/chan-desktop.json` — `Config`

Source: `desktop/src-tauri/src/config.rs`.

| Field | Type | Default | Reachability | Consumers |
|-------|------|---------|--------------|-----------|
| `sidecar` | `HashMap<String, DriveSidecar>` | empty | (auto-populated on drive interactions) | per-drive UI state |
| `sidecar.{key}.last_port` | `Option<u16>` | `None` | (auto on serve) | port-reuse on serve restart |
| `sidecar.{key}.features.bge` | `bool` | `false` | Launcher row expand panel (`-b-28a + b`) | ⚠ **MIRROR / DRIFT-PRONE** — chan-drive's `IndexConfig.semantic_enabled` is the real source. See systacean-28 finding 2. |
| `sidecar.{key}.features.reports` | `bool` | `false` | Launcher row expand panel | Same. Per-drive `IndexConfig.reports_enabled` is the real source. |
| `tunnel.preferred_port` | `u16` | `0` (OS-assigned) | Tunnel listener UI | tunnel listen-bind hint |
| `tunnel.preferred_label` | `String` | `""` | Tunnel listener UI | bearer/label default |
| `tunnel.preferred_drive` | `String` | `""` | Tunnel listener UI | drive name default |
| `window_configs[]` | `Vec<WindowConfig>` | empty | (auto on window close) | LRU pop on window open; preserves panes/tabs + URL hash + zoom level |

`WindowConfig`: `key: String`, `window_label: String`, `url_hash: String`, `zoom_level: f64`, `saved_at: u64`.

## Open findings (systacean-28)

| # | Finding | Recommended action | Owner | Priority |
|---|---------|---------------------|-------|----------|
| 1 | `ServerConfig.reports.enabled` is round-trip only — UI exists, no backend gating; per-drive `IndexConfig.reports_enabled` (post-`systacean-27`) is the real source. | Remove field; route SPA toggle through `chan reports enable/disable --path <drive>` or chan-server passthrough; observe per-drive state. | chan-server + SPA | Low (harmless round-trip; not data-damage) |
| 2 | `DriveFeatures` mirror in chan-desktop sidecar drifts when users bypass chan-desktop's UI for feature toggles (e.g. `chan index enable-semantic` from terminal). | Replace mirror with on-read pass-through to chan-drive's `IndexConfig`, OR refresh-on-read. | chan-desktop + chan-drive | Low (corner case for power users) |

Both findings need cross-lane coordination. Recommend filing as routed follow-up tasks when the architect compiles the Round-3 polish backlog.

## Layout pointers

* Per-user config dir: `~/.chan/` (desktop) or `state_dir/` (iOS / Android).
* Per-user state dir: `XDG_DATA_HOME/chan/` (Linux) / `~/Library/Application Support/chan/` (macOS).
* Per-user cache dir: `XDG_CACHE_HOME/chan/` / `~/Library/Caches/chan/`.

Per-drive subpaths key by `KnownDrive.uuid` (16 hex chars), assigned at registration:

* `state_dir/sessions/<uuid>/` — session blobs (window/pane layout).
* `state_dir/graph/<uuid>/` — graph DB + sidecar markers.
* `state_dir/locks/<uuid>/` — per-drive index-writer lockfile.
* `state_dir/tokens/<uuid>/` — chan-server bearer token (mode 0600).
* `state_dir/trash/<uuid>/` — soft-deleted files (lazy GC).
* `state_dir/report/<uuid>/report.jsonl` — chan-report state (lazy on opt-in post-`systacean-27`).
* `state_dir/drafts/<uuid>/` — Drafts metadata (post-`systacean-24`). Contains regular drafts (`untitled-N/`) + team workspaces (`team-{name}/`).
* `cache_dir/index/<uuid>/` — tantivy search-index segments + `config.toml`.

See `crates/chan-drive/src/paths.rs::DrivePaths` for the canonical computation.
