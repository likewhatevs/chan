# v0.59.0-rc1

Aggregated release-candidate notes for v0.59.0. Each lane appends its own
section below; the final notes are compiled from these at release time.

## Index and dashboard

Lane: `index-dashboard` (branch `index-dashboard`, based on `main`). Covers the
`## Index and dashboard` item in `dev/v0.59.0/request.md`.

### Theme

Make the indexing status something you can act on, and make the Dashboard's
Indexing graph tell the truth about progress. Clicking the indexing notification
now takes you straight to the live per-directory graph; that graph shows one
directory pulsing at a time instead of the whole tree flashing in lockstep; and
it stays put across tab switches instead of reloading.

### What landed

**Clickable indexing notification → Dashboard Indexing slide, paused**

- The top-right indexing status pill (`AppStatusBar.svelte`) is now a button. A
  click opens a Dashboard tab focused on the Indexing (Search) carousel slide
  with auto-rotation off, so a user watching the index build lands directly on
  the live graph and it does not rotate away.
- New shared helper `openIndexingDashboard()` (+ `DASHBOARD_SEARCH_SLIDE = 1`
  and an `OpenDashboardOptions` overrides type) in `tabs.svelte.ts`. The server
  `cs dashboard` (`open_dashboard`) handler was refactored to reuse the same
  `openDashboardInActivePane({ slide, autoRotate })` path instead of mutating the
  freshly-created tab after the fact.

**Per-path indexing pulse (fixes "all nodes flash orange together")**

- Root cause was backend, not frontend. During the background embedding sweep
  the indexer reaches `Idle { embedding: Some(..) }` with no per-file label, so
  `build_indexing_state` marked *every* directory with indexable files as
  `Indexing` at once. On a small workspace (near-instant BM25) that meant the
  only visible phase was the embed sweep: all-orange → all-green in lockstep.
- The embed forward-pass already knows which file it is draining; that label was
  being discarded. `EmbedProgress` now carries `file: Option<String>`, populated
  from the live `IndexFile` label. `current_index_file` surfaces it during the
  embed sweep, and the sweep-broadening condition was narrowed to
  `(embedding_sweep || current_file.is_some()) && !current_file_matched_entry`.
- Net effect: whenever a real file label is known (foreground build OR embed
  drain) only that one directory pulses `Indexing`; the rest resolve to
  `Indexed`/`Pending`. The broad pulse is now only a fallback for the gaps with
  no per-file signal (the initial empty-label Building window and between embed
  batch flushes), so the spine still never looks idle.

**Dashboard tab keep-alive (fixes reload/re-layout on tab switch)**

- `DashboardTab` moved from the active-tab if-chain into the keep-alive
  each-loop in `Pane.svelte`, mirroring graph/file/terminal tabs. It now stays
  mounted and hides via the `visibility: hidden; pointer-events: none` contract
  (never `display:none`) with an `active` gate
  (`!paneMode.active && !pane.showingBack && t.id === pane.activeTabId`).
- The Indexing carousel's `GraphCanvas` force layout and 3s poll survive tab
  switches instead of tearing down and rebuilding, so the graph no longer
  reloads/re-lays-out on every switch. The `active` gate also pauses the carousel
  and stops the indexer poll while the tab is hidden or flipped, so a kept-alive
  background dashboard does no work. Reload is now an explicit user action:
  Cmd+R or the existing right-click **Reload** row.

### Highlights

- The flash-together fix is backend-only and additive; the frontend graph
  already rendered per-node state faithfully, so no `GraphCanvas` changes were
  needed (keeps this lane clear of the `graph-tuning` lane's `GraphCanvas`
  rewrite).
- Reused existing seams throughout: the carousel's `active` prop already paused
  rotation + poll, the keep-alive visibility contract already existed for
  graph/file/terminal tabs, and the `open_dashboard` server path already applied
  `carouselSlide`/`autoRotate`.
- Fixed a stale doc comment on `DashboardTab.carouselSlide` that claimed the
  Indexing graph was slide 2; it is slide 1 (confirmed by the `SLOTS` order and
  the `slideIndex === 1` poll gate).

### Lowlights / follow-ups

- The Indexing graph polls every 3s, so the orange pulse advances in 3s steps
  rather than smoothly; fine for "watch it progress", but not frame-accurate.
- Between embed batch flushes `current_file` can briefly be `None`, so the spine
  falls back to the broad pulse for those gaps (by design). On a large workspace
  with long flush intervals this can read as a brief all-orange blip.
- The right-click **Reload** row still does a full `reloadWindow()`. It satisfies
  "user-requested reload", but a lighter graph-only refresh could be added later.
- Empirically verified via a seeded local server (see Validation); desktop
  (WKWebView) not separately verified.

### Validation

- `cargo test -p chan-server` (new `indexing_state_embedding_sweep_with_current_file_pulses_one_dir`
  plus updated `EmbedProgress`/`set_idle` tests).
- `cd web && npm run build && npm run check -w @chan/workspace-app`.
- `cd web && npm run test -w @chan/workspace-app` (2100 passed, incl. new
  `paneDashboardTabKeepAlive.test.ts` and updated `dashboardTabAndCarousel.test.ts`).
- Full `make pre-push` gate.
- Seeded local `chan serve --standalone` over a throwaway `CHAN_HOME` +
  workspace: watched the pill build, clicked it to land on the paused Indexing
  slide, confirmed `/api/indexing/state` reported one `indexing` directory at a
  time, and confirmed the graph did not reload on tab switch.

### Repro Fixed

Before this change, on a small workspace the Dashboard Indexing graph only ever
showed every indexable directory turn orange simultaneously and then green
simultaneously, because the whole visible pass was the background embedding sweep
and the backend painted the entire spine `Indexing` for its duration. It now
pulses the single directory whose file is being embedded, so directories move
through pending → indexing → indexed independently. Separately, switching tabs
away from and back to the Dashboard rebuilt the graph from scratch (force layout
reset, poll re-fired); the tab is now kept alive so the graph persists and only a
Cmd+R / right-click Reload forces a refresh.

### Raw data (files touched)

- `crates/chan-server/src/indexer.rs` — `EmbedProgress.file`; populate from the
  live IndexFile label in the embed branch; `EmbedPhaseState.file`; drop `Copy`.
- `crates/chan-server/src/routes/search.rs` — `current_index_file` returns the
  embed file; narrowed `broad_sweep`; new + updated `build_indexing_state` tests.
- `web/packages/workspace-app/src/api/types.ts` — optional `embedding.file`.
- `web/packages/workspace-app/src/state/tabs.svelte.ts` — `OpenDashboardOptions`,
  `openDashboardInPane(opts)`, `openDashboardInActivePane(opts)`,
  `DASHBOARD_SEARCH_SLIDE`, `openIndexingDashboard()`; fixed slide doc comment.
- `web/packages/workspace-app/src/state/store.svelte.ts` — `open_dashboard`
  reuses the opts path.
- `web/packages/workspace-app/src/components/AppStatusBar.svelte` — clickable
  indexing pill.
- `web/packages/workspace-app/src/components/Pane.svelte` — dashboard keep-alive
  each-loop (front-face arm removed; back-face DashboardSlotBack dispatch kept).
- `web/packages/workspace-app/src/components/DashboardTab.svelte` — `active`
  gate + keep-alive visibility contract; carousel threading.
- `web/packages/workspace-app/src/components/paneDashboardTabKeepAlive.test.ts`
  (new) and `dashboardTabAndCarousel.test.ts` (updated).
