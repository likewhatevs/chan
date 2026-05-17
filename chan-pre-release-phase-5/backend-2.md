# @@Backend task 2: per-window desktop session keys

Owner: @@Backend
Status: REVIEW
Depends on: wave-1 cleanup reaching REVIEW
Coordinates with: [webtest-2](./webtest-2.md) for the two-window reload
regression check.

## Goal

Fix the chan-desktop pane/tab session collision where multiple windows for
the same drive all read and write `/api/session?w=default`.

## Acceptance criteria

* chan-desktop appends a stable, unique window session key to each drive
  webview URL.
* The web API client uses that key for `/api/session?w=<id>`.
* Plain browser use keeps the historical `default` session key.
* Pagehide keepalive session saves use the same key as normal saves.
* Add focused tests for key selection / encoding.

## Progress

* `desktop/src-tauri/src/serve.rs` now appends `w=<window-label>` when
  building each local or tunneled drive webview window.
* `web/src/api/client.ts` now derives the session key from the page URL and
  falls back to `default`.
* `web/src/state/store.svelte.ts` uses the same derived session path for
  pagehide keepalive saves.
* Added `web/src/api/client.test.ts` for default, desktop, and encoded
  window ids.
* Rebuilt `target/debug/chan` after the latest `npm --prefix web run build`
  so the Webtest lanes can relaunch with the current bundle.

## Verification

* `cargo fmt --check`
* `cargo check -p chan-desktop`
* `cargo build -p chan`
* `npm --prefix web run check`
* `npm --prefix web test -- --run src/api/client.test.ts src/state/store.test.ts`
* `npm --prefix web test`
* `npm --prefix web run build`

After [frontend-3](./frontend-3.md) reached REVIEW, reran the current
web gate and final binary rebuild:

* `npm --prefix web run check`
* `npm --prefix web test`
* `npm --prefix web run build`
* `cargo build -p chan`

## Completion notes

Ready for [webtest-2](./webtest-2.md) to rerun the two-window reload
scenario after the shared service is restarted on the rebuilt binary.
