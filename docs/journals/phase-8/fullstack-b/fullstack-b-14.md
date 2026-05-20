# fullstack-b-14: chan-desktop window title is the drive path, not "chan drive: <name>"

Owner: @@FullStackB
Date: 2026-05-20

## Goal

@@Alex 2026-05-20: "the tauri title: 'chan drive: <name>'
should be <path> instead". Replace today's `chan drive:
<drive-name>` window title format with the full drive path
(e.g. `/Users/fiorix/dev/github.com/fiorix/chan`).

Read of the ask: title is the path — no `chan drive:` prefix.
If you have a strong reason to keep the prefix (menubar
discoverability, OS-level window switcher hint), surface a
scope question; otherwise default to path-only.

## Background

Bug entry: [`../phase-8-bugs.md`](../phase-8-bugs.md)
"chan-desktop Tauri window title: shows 'chan drive: <name>',
should be '<path>' instead".

Per CLAUDE.md, chan-desktop's per-window state is keyed by
`w=<window-label>` URL parameter, with `serve.rs` driving
window creation. The title swap happens at window-build-
time alongside the existing label-derivation logic; this is
likely a one- or two-line change in
`desktop/src-tauri/src/serve.rs` (or wherever
`WebviewWindowBuilder::title(...)` gets called).

`fullstack-b-1`'s window-config LRU work is the most recent
chan-desktop change touching the same window-creation path;
read its commit (`fullstack-b-1` task tail) to ground the
current shape before editing.

## Acceptance criteria

* New chan-desktop windows open with the title set to the
  full drive path. No `chan drive:` prefix.
* Title updates appropriately if a window is recycled
  against a different drive (LRU restore path from -b-1).
* Existing menubar / OS window-switcher behaviour
  unchanged (the title is just text; no other affordance
  depends on the `chan drive: ` prefix verbatim).
* `cargo test -p chan-desktop --bin chan-desktop` green.
* Pre-push gate clean.

## How to start

1. Grep `desktop/src-tauri/src/` for the current title
   format — likely `format!("chan drive: {}", ...)` or
   similar near a `title(...)` call.
2. Replace with the drive's path string directly.
3. Verify the LRU restore path from `fullstack-b-1` still
   produces the correct title on a reopened window.
4. Pre-push gate.

## Coordination

* Tiny scope; rides the patch release alongside the
  rich-prompt mini-wave.
* Independent of [`fullstack-b-13`](fullstack-b-13.md);
  different file (`serve.rs` vs `terminal_sessions.rs` /
  `routes/terminal.rs`). Land in any order.
* @@WebtestB verifies on lane-B once landed (Tauri-launch
  permission already in effect from the prior session
  per `event-webtest-b-alex.md`).
* Push held for the patch-release commit-grouping cut.

## 2026-05-20 — implemented (@@FullStackB)

Two-line + one-test change.

* `drive_title(key)` in `desktop/src-tauri/src/serve.rs:362`
  now returns `key.to_string()` directly. The earlier
  `Path::new(key).file_name() ... "chan drive: {base}"`
  shape is gone. `Path` import stays (still used as a
  function parameter type at line 105).
* `spawn_tunneled_drive_window` (same file, line 432)
  swapped `"chan drive: {tenant_label} \u{00b7} {drive}"`
  for `"{tenant_label} \u{00b7} {drive}"` — same shape as
  the local-drive title (no prefix). Tunneled drives have
  no local filesystem path, so the closest analog is the
  existing tenant·drive label.
* New unit test `drive_title_is_the_path_verbatim`
  pins three cases: typical absolute path, trailing slash,
  empty string. Catches an accidental revert of the prefix.

### LRU restore path verification

`spawn_local_drive_window` always calls `drive_title(key)`
regardless of whether `pop_compatible_config` returns a
restored entry; the title is derived from the live `key`
each open, not stored in `WindowConfig`. With my change the
restored window's title is the same path as a fresh window.
LRU restore behaviour unchanged.

### Pre-push gate

* `cargo fmt --check` — clean.
* `cargo test -p chan-desktop --bin chan-desktop` — 20/20
  green (19 baseline + 1 new).
* `cargo clippy -p chan-desktop --all-targets -- -D warnings`
  — clean.

Workspace-wide gate (svelte-check, npm build, vitest, full
clippy, full test) holds until I'm ready to commit; the
isolated chan-desktop slice is green.

### Suggested commit subject

```
chan-desktop: window title = drive path verbatim (fullstack-b-14)
```

### Coordination footprint

* `desktop/src-tauri/src/serve.rs` is currently unmodified
  by other lanes in `git status`; my edit is the only
  uncommitted change to this file.
* Independent of -b-13 (different file). Land in any order.

### Status

Commit-ready. Holding for @@Architect commit clearance.
@@WebtestB verifies on lane-B per the task body.
