# webdev-5

## Scope

Consume the frozen `/api/fs-graph` route from `rustacean-2.md` in the
frontend graph surface.

## Changes

- `web/src/api/types.ts`
  - Added typed filesystem graph response, node, edge, scope, and kind
    definitions.

- `web/src/api/client.ts`
  - Added `api.fsGraph({ scope, path, depth })` for
    `GET /api/fs-graph`.

- `web/src/components/FileTree.svelte`
  - `Graph this` now opens filesystem graph mode for files and
    directories from the file browser.

- `web/src/state/store.svelte.ts`
  - Added graph overlay mode: `semantic` or `filesystem`.
  - Added filesystem-specific file and directory open helpers.
  - Persisted graph mode in URL hash and session sidecars.
  - Added a narrow test seam for URL-hash overlay restore.

- `web/src/components/GraphPanel.svelte`
  - Loads `/api/fs-graph` when graph mode is filesystem and the current
    scope is `file:` or `dir:`.
  - Maps folder/file/symlink/ghost nodes and
    contains/symlink/hardlink edges onto the existing graph renderer.
  - Shows filesystem-specific filter labels, empty states, status, and
    inspector details including target/outside/broken metadata.

- `web/src/App.svelte`
  - Includes graph mode in hash persistence reactivity.

- `web/src/state/store.test.ts`
  - Added regression coverage for filesystem graph mode hash
    encode/restore and legacy semantic hash fallback.

- `phase-1/webtest-smoke.mjs`
  - Tightened the File Browser `Graph this` smoke to require the
    filesystem graph status/filters, not just any graph overlay.

## Verification

- `cd web && npm run check`
  - Passes with 0 errors and 0 warnings.
- `cd web && npm test -- --run`
  - Passes: 6 files / 97 tests.
- `node --check phase-1/webtest-smoke.mjs`
  - Passes.
- Current-build browser smoke:
  - Built `web/dist` with `npm run build`.
  - Rebuilt embedded assets with `cargo build --release -p chan`.
  - Served `/tmp/chan-webdev-smoke` on `http://127.0.0.1:8790/`.
  - `CHAN_WEB_URL=http://127.0.0.1:8790/ node phase-1/webtest-smoke.mjs`
    passed desktop and narrow checks:
    language search, Search Status overlay, File Browser `Graph this`
    opening filesystem graph mode, and assistant layout skip when disabled.

## Notes

- The canvas still uses the existing graph renderer shape/color vocabulary.
  Filesystem concepts are labeled in controls and inspector, but there are
  no dedicated canvas glyphs yet for symlink or hardlink edges.
