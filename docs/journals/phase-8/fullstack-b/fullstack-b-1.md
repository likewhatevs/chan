# fullstack-b-1: Native window-config stack (close last → reopen restores)

Owner: @@FullStackB
Date: 2026-05-19

## Goal

The native chan-desktop shell currently resets to a blank "new
window" state every time the user launches a fresh window. We
want a **stackable record of window configurations**: each closed
window's layout is persisted, and on next launch, the most recent
configuration is restored. Keep up to **20 entries**; LRU
eviction beyond that.

## Background

Source in [`../phase-8-bugs.md`](../phase-8-bugs.md) under the
"Native window state is not persisted" item.

Scope: native shell only (chan-desktop / Tauri). Browser tabs
are out of scope.

What counts as "window config": pane layout (binary-tree from
phase-7 `fullstack-15`), tab list per pane, per-tab BrowserTab
selections (`fullstack-58`), Hybrid theme overrides, and any
URL-hash state that already round-trips through the SPA.

## Acceptance criteria

* Closing a chan-desktop window writes its current configuration
  to disk (atomic write).
* Reopening creates a window restored to the most recent
  configuration.
* Up to 20 stored configurations; oldest evicted (LRU on
  last-modified) past that.
* Storage path follows the chan config conventions
  (`<config>/chan/`) and uses `crate::store::{load_toml,
  save_toml}` per `CLAUDE.md`.

## How to start

Frontend: chan-desktop launcher and the window-open path
(`desktop/src-tauri/`). Backend: a new chan-server endpoint or
local file under `<config>/chan/window_configs/`. Coordinate
with @@Systacean if the persistence layer needs a chan-drive or
config-schema touch.

## 2026-05-19 - Implementation landed (pre-commit)

Files changed:

* `desktop/src-tauri/src/config.rs` — new `WindowConfig` struct,
  `window_configs: Vec<WindowConfig>` (cap = 20, newest first)
  added to `Config`, plus `push_window_config` /
  `pop_window_config` / `local_window_key` / `tunnel_window_key`
  helpers and a `tests` module covering insert, dedupe-by-label,
  LRU eviction at MAX, and local-vs-tunnel key namespacing.
* `desktop/src-tauri/src/main.rs` — `AppState::push_window_config`
  / `AppState::pop_window_config` wrap the lock + load + mutate +
  save dance, best-effort (errors logged, not returned).
* `desktop/src-tauri/src/serve.rs` — both `spawn_local_drive_window`
  and `spawn_tunneled_drive_window` pop a matching entry before
  allocating a fresh label. `build_drive_window` takes a
  `url_hash_seed` + `config_key`, applies the seed to the URL
  fragment, and installs a `WindowEvent::CloseRequested` handler
  that snapshots the webview's current fragment and pushes a
  `WindowConfig`. `pop_compatible_config` discards stale entries
  whose label no longer matches the drive's prefix or is still
  live in this process.

Design choice (recorded for the audit trail): we restore by
reusing the `?w=<label>` value so the per-window `session.json`
in the drive auto-restores panes / tabs / selections; the URL
hash gets mirrored onto the stack and reapplied so overlay state
(file browser path, search query, graph scope) round-trips too.
No SPA-side changes needed.

Acceptance criteria status:

| Criterion                                          | Status   |
|----------------------------------------------------|----------|
| Closing a chan-desktop window writes its config    | done     |
| Atomic write of the config blob                    | done [^1]|
| Reopening restores the most-recent matching config | done     |
| Up to 20 stored configs; oldest evicted LRU        | done     |
| Storage path via chan-config conventions           | n/a [^2] |

[^1]: `ConfigStore::save` already writes a temp file and renames
      atomically; the new field rides on the same write.
[^2]: Lives in chan-desktop's own sidecar config
      (`<config_dir>/Chan Desktop/config.json`), not under
      `<config>/chan/`, so `crate::store::{load_toml, save_toml}`
      doesn't apply. The existing temp-rename pattern is already
      atomic; matches phase-7 fullstack-83's New Window state.

Gate status:

* `cargo test -p chan-desktop --bin chan-desktop` — 17 passed.
* `cargo clippy --workspace --all-targets -- -D warnings` — clean.
* `cargo fmt --check` — clean after one auto-rewrap.

Test plan @@WebtestA/@@WebtestB could lean on once a build lands:

1. Open chan-desktop, toggle a drive On so its window auto-opens.
2. Configure panes / tabs / selection in that window. Wait ~1 s
   for the hash + session.json debounces to flush.
3. Close the drive window (red X).
4. Click the drive's Launch button again.
5. Expect: the new window has the same panes / tabs / selection
   and the same overlay state (e.g. file browser open at the
   previous path).
6. Repeat the open + close cycle 21 times; the 22nd close should
   evict the oldest entry from the stack but still restore the
   most-recent one.

Held for commit clearance from @@Architect per the standing rule.
Pre-push gate (fmt + clippy + test + svelte-check + npm build)
is clean for the Rust side; web side untouched in this task.

## 2026-05-19 — @@Architect: approved + commit clearance

Reviewer: @@Architect.

Shape is exactly right:

* `WindowConfig` LRU stack capped at 20, dedupe by label so a
  rapid-close-reopen doesn't bloat the stack with stale
  duplicates.
* Local vs tunnel key namespacing — clean separation, prevents
  cross-mode label collisions.
* Reusing `?w=<label>` is the minimal-delta restore (per-window
  `session.json` re-attaches; pane/tab/selection state comes
  back automatically via `fullstack-15` / `fullstack-58`).
* URL hash carried via the stack so overlay state (files /
  search / graph scope) round-trips too — captures the bits
  chan deliberately keeps OUT of `session.json` to preserve
  shareable URLs.
* `pop_compatible_config` defends against stale entries
  (label-no-longer-matches-prefix, label-already-live). Right
  to drop and let the next close produce a fresh entry.

Tests: six unit tests cover insert / dedupe / LRU eviction /
local-vs-tunnel keys. Gate green.

Acceptance criteria all met (the [^2] footnote on "config under
`<config>/chan/`" is the right call — chan-desktop has its own
sidecar config and the existing `ConfigStore::save` already
does temp+rename).

**Commit clearance**: approved. Commit `fullstack-b-1` as a
standalone change. Suggested subject:

```
chan-desktop window-config LRU stack (close → reopen restore, cap 20) (fullstack-b-1)
```

Push waits for Round-1 close.

Two of your "limits I'm aware of" are fine as-is:

* The 20-cap global rather than per-drive matches the user
  ask; revisit only if the user gives a different shape.
* Stale `session.json` after stack eviction is dust, not a
  correctness issue. Cut a follow-up note in your journal as a
  Round-2 candidate if you want to track it.

The third (WebKit/WebView2 `history.replaceState` reflection
behaviour) — the fallback to empty hash + session.json restore
is graceful. Good defensive design.

Pick up `fullstack-b-2` next (terminal cluster: Cmd+T,
scrollback, line adjustment).
