# @@FullStackB's phase-8 journal

Author: @@FullStackB
Date: 2026-05-19

Frontend + backend lane B. Same profile as @@FullStackA; operates
in parallel to clear the bug queue and (Round 2) feature queue.

Append-only. New entries go at the bottom under a dated heading.

## 2026-05-19 - Boot + queue scan

Fresh session boot. Read contact card, phase-8 process (inherits
phase-7 verbatim plus the @@CI lane delta and the bug-sweep round
shape), the phase request, the bug list, and my four task files:

| Task             | Scope                                                |
|------------------|------------------------------------------------------|
| fullstack-b-1    | Native window-config stack (chan-desktop Tauri)      |
| fullstack-b-2    | Cmd+T + scrollback + line-adjustment cluster         |
| fullstack-b-3    | Watcher dialog: out-of-root paths + create-dir UX    |
| fullstack-b-4    | Indexing-chart pan/zoom parity with Graph view       |

No inbound events in `alex/event-architect-fullstack-b.md` or
`alex/event-fullstack-b-alex.md` yet (files don't exist). Working
through tasks in numerical order.

## 2026-05-19 - fullstack-b-1 implemented

Landed the WindowConfig LRU stack inside chan-desktop's sidecar
config:

* `config::WindowConfig` plus `window_configs: Vec<WindowConfig>`
  in `Config` (cap = 20, newest first).
* Free-function helpers `push_window_config` / `pop_window_config`
  with dedupe-by-label semantics, plus
  `local_window_key` / `tunnel_window_key` to keep local and
  tunneled drives namespaced apart in the stack.
* `AppState::{push,pop}_window_config` wrap the lock + load +
  mutate + save pattern, best-effort (errors logged, not
  returned).
* `serve::spawn_local_drive_window` /
  `spawn_tunneled_drive_window` now pop a matching entry before
  generating a fresh label, reusing its `window_label` (so the
  `?w=<label>` lookup hits the same `session.json`) and stamping
  its saved URL hash into the new webview's fragment.
* `build_drive_window` installs a `WindowEvent::CloseRequested`
  handler that snapshots `WebviewWindow::url().fragment()` and
  pushes a fresh `WindowConfig` to the top of the stack.
* `pop_compatible_config` defends against stale entries: a popped
  label that no longer matches the drive's current hash prefix,
  or that's already live in this process, is dropped (the next
  close will produce a fresh entry anyway).

Why this shape: `session.json` is already keyed per-window in the
drive (`<drive>/.chan/sessions/<w>.json`), so reusing the same
`?w=` label across the close/open cycle is the smallest possible
delta that restores panes / tabs / selections (per `fullstack-15`
+ `fullstack-58`). The URL hash carries the overlay state
(`files=...`, `search=...`, `graph=...`) that chan deliberately
keeps out of `session.json` so shareable URLs stay shareable; we
mirror it onto the stack so close/reopen also round-trips those.

Tests: six new unit tests in `config::tests` covering insert,
dedupe, LRU eviction at MAX, and the local-vs-tunnel key
namespacing. `cargo test -p chan-desktop` is green (17 tests
total), `cargo clippy --workspace --all-targets -- -D warnings`
is clean, `cargo fmt --check` passes after one auto-rewrap.

Limits I'm aware of:

* Capturing the hash on close relies on `WKWebView.URL` (macOS)
  and `WebView2.Source` (Windows) reflecting hash changes pushed
  via `history.replaceState`. Both do today; if a future Tauri
  upgrade changes that, the captured hash falls back to empty
  (session.json restore still works via the reused `?w=` label).
* The "20 entries" cap is global, not per-drive. A user who opens
  twenty separate drive windows in quick succession could push
  older drive entries off the stack. Matches the bug ask
  ("keep up to 20 window configs"), and the more-recent drives
  are the ones the user is most likely to want restored anyway.
* No teardown of stale `session.json` files in the drive. The
  stack drops the label after 20 entries, but the per-window
  session file lives on in chan-drive. Not a correctness issue
  (the SPA loads only the label it's given), but it's some
  growing dust over time. Out of scope for this fix.

Followed CLAUDE.md: no em dashes in comments / docs, factual
prose, pinned toolchain unchanged. Atomic write contract carried
forward via the existing `ConfigStore::save` (temp + rename).
