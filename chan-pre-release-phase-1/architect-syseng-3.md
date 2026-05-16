# architect-syseng-3: Phase 1 release-readiness audit

From: syseng. To: architect. Status: DONE.

Pre-seal audit pairing each `request.md` checkbox with its
deliverable plus one final clean-tree verification gate. Intended
as direct input for `summary.md`.

## Roadmap delivery map

### Fresh new to the world

| Roadmap item                                                | Deliverable                                                                                          | Status |
|-------------------------------------------------------------|------------------------------------------------------------------------------------------------------|--------|
| Clear migration code from internal iterative versions       | `rustacean-1` removed pre-v3 contact email backfill from `indexer.rs` + `cmd_status`. `chan-core-purge-1` removed the producer helper + tests in chan-drive.                                                | DONE   |
| First canonical version: no pre-Chan migration paths        | `rustacean-1` audit + architect-1 audit classified the residual `legacy`/`schema_version`/`v[0-9]+` hits as external contract names or in-progress editor compat. No internal-version migration code remains. | DONE   |
| Crystal clear comments + current-decision design doc        | `architect-1` wrote `design-snapshot.md` as a current-state contract. `rustacean-1` reworded `auth.rs` pre-release comment and renamed `pane_widths_legacy_file_*` test to snapshot tone.                  | DONE   |

### Search and graph

| Roadmap item                                                                | Deliverable                                                                                                                                                              | Status |
|-----------------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------|--------|
| Graph-like index for directories/files with symlinks, hardlinks, broken links | `rustacean-2` `GET /api/fs-graph` route; nodes `folder|file|symlink|ghost`, edges `contains|symlink|hardlink`. 11 unit tests + syseng-1 live probes (broken/escape/loop). | DONE   |
| File Browser right-click `Graph this`                                       | `webdev-2` row context menu + `webdev-5` wired through to `/api/fs-graph`. webtest-1 smoke confirmed on desktop and narrow viewports.                                    | DONE   |
| Graph overlay SCOPE: Folder with Parent Folder convenience                  | `webdev-2` `availableGraphScopes()` extension + folder/parent shortcuts.                                                                                                 | DONE   |
| File-scope graph includes its Folder in the dropdown                        | `webdev-2` folder shortcut for direct file scopes.                                                                                                                       | DONE   |
| Folder scope: graph files + subdirs from depth 1                            | `rustacean-2` default depth 1, `MAX_DEPTH=6`. Verified in syseng-1 probe (`folder_scope_depth_one_lists_direct_children`).                                                | DONE   |
| Search index dashboard overlay (sibling-button pattern from Assistant)      | `webdev-3` new `SearchStatusOverlay.svelte`, button beside Search scope.                                                                                                 | DONE   |
| Move DRIVE index info out of File Browser Inspector                         | `webdev-3` removed search-index section from `DriveInfoBody.svelte`.                                                                                                     | DONE   |
| Dashboard: reset index button + visible rebuild progress                    | `webdev-3` `Rebuild index` button, polls `/api/index/status` while open.                                                                                                 | DONE   |
| Dashboard: chan-report progress + SLOC-by-language                          | `webdev-3` loads `api.reportPrefix("")`, renders totals + per-language rollup.                                                                                           | DONE   |
| `language:<name>` search using chan-report data                             | `webdev-2` + `webdev-4` parsing + scan via `api.reportFile(path)`. `webtest-1` fixed lazy-tree hydration bug during smoke (root-only scan).                              | DONE   |
| Search arrow nav scrolls page, recalibrates on resize                       | `webdev-1` `ResizeObserver` on SearchPanel hits list + active-result scroll tracking. webtest-1 smoke confirmed.                                                         | DONE   |

### Assistant

| Roadmap item                                                                                              | Deliverable                                                                                                                                                                  | Status                                                                              |
|-----------------------------------------------------------------------------------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|-------------------------------------------------------------------------------------|
| Chat scroll-keep with bottom margin + resize recalibration                                                | `webdev-1` double-rAF bottom pinning, 28px margin, `ResizeObserver` on chat container. `webtest-1` later verified active-turn desktop and narrow smoke through the fake-Codex fixture. | DONE                                                                                |
| Chat bubble width stretches to chat-area max                                                              | `webdev-1` `max-width: 100%` on chat bubbles; active-turn smoke tightened the assistant body to fill the chat column on narrow screens.                                      | DONE                                                                                |
| Single orange-dot `thinking` badge (drop `thinking...` cycling)                                           | `webdev-1` removed the duplicate `thinking...` placeholder body when the status badge is visible.                                                                            | DONE                                                                                |
| Orange dot blinks during thinking — parity with file editor tab                                           | `webdev-1` added blinking animation to the stream status orange dot.                                                                                                         | DONE                                                                                |

Webtest follow-up: the original `/tmp/chan-dev` fixture keeps
`preferences.assistant.effective_enabled:false`, so assistant-active smoke
uses the isolated fake-Codex fixture recorded in `webtest-1.md`.

### Command line

| Roadmap item                              | Deliverable                                                                                                              | Status                                  |
|-------------------------------------------|--------------------------------------------------------------------------------------------------------------------------|-----------------------------------------|
| `chan config {get|set}` settings   | `rustacean-3` `chan config` for editor namespace via `EditorPrefs::save`. Server + assistant namespaces extended by architect. | DONE                                    |
| `chan graph` queries by scope             | `backend-1` (off-band) + `rustacean-3` integration. `--scope all` uses the content graph; `--scope file|folder` uses the filesystem graph builder. | DONE |
| `chan status` overall drive/index/graph/report | `backend-1` `chan status` returns drive root, index stats, graph counts, chan-report SLOC/language/COCOMO summary, with optional `--json`. | DONE                                    |

## Final verification gate (clean tree, syseng run 2026-05-16 12:36)

```
cargo build --release -p chan -p chan-server  # ok
cargo test --workspace                        # chan-server 92, chan 46
cargo clippy --all-targets -- -D warnings     # clean
cargo fmt --all -- --check                    # clean
otool -L target/release/chan                  # macOS system frameworks only
target/release/chan size: 92.7 MB             # +200 KB vs pre-fix
```

Workspace test totals reflect the fs-graph, indexer, CLI graph, and
CLI config coverage landed during the phase.

## Residuals at seal

None from the syseng/front-end verification matrix. Assistant active-turn
browser smoke is closed via `webtest-1.md`'s isolated fake-Codex fixture.

## Recommendation

Phase 1 is sealable from this verification matrix.

Suggested commit ordering follows `architect-rustacean-1.md`:

1. chan-core: `chan-core-purge-1` (producer-side backfill removed).
2. chan: `rustacean-1` (consumer-side backfill removed + comment /
   test rewording).
3. chan: `rustacean-2` (`/api/fs-graph` route).
4. chan: `rustacean-6` (`/api/fs-graph` mid-path symlink escape fix).
5. chan: `rustacean-3` (CLI parity for config / graph / status).
6. chan: `architect-syseng-2` fix (`apply_watch_change` helper +
   7 indexer tests). Could fold into rustacean-2 or its own
   commit; recommend its own commit so the regression is
   discoverable in `git log`.
7. chan: webdev-1, -2, -3, -4, -5 (frontend changes; can bundle
   given they all live under `web/`).
8. chan: `syseng-1.md` + `architect-syseng-{1,2,3}.md` + the
   `webtest-smoke.mjs` runner + the phase journal can land as a
   single phase-record commit so the engineering artifacts stay
   together.

syseng. Nothing else from me before seal.
