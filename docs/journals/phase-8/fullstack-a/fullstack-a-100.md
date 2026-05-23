# fullstack-a-100 — Drafts FB chain bug: not browseable + Graph tabs broken + Cmd+N broken (v0.13.0 release blockers)

Owner: @@FullStackA
Phase: 8, Round 3
Date cut: 2026-05-23
Priority: **P0 — release blockers for v0.13.0** (3 bugs likely sharing one root cause)

## Goal

Root-cause + fix the three chained bugs @@Alex flagged 2026-05-23 (`docs/journals/phase-8/alex/round4.md`). My triage notes hypothesize a shared root cause: chan-drive's drafts metadata handle is broken / uninitialized on a pre-v0.12.0 registered drive. Confirm or falsify; fix.

## Background

@@Alex's three bugs (from `round4.md`):

1. **Drafts folder is not browseable from the File Browser** — cannot be expanded, can't see what's on it.
2. **Graph tabs aren't loading** — possibly because of the Drafts folder being "broken" or not loading.
3. **Cmd+N (New Draft) is also not working** — probably also chained off Drafts.

@@Alex is running v0.12.0 chan.app against a drive that was registered before v0.12.0 (pre-Drafts-metadata). My triage hypothesis: the drafts metadata directory may not have been bootstrapped on that drive, so `chan-drive`'s `drafts_dir_handle` errors when listed / written.

Full triage write-up at `docs/journals/phase-8/alex/round4.md` § "2026-05-23 — @@Architect triage notes".

## Scope (3 bugs, suspected one root cause)

### Code-read pathway (no repro yet)

* Bug 1: FB expand → `setExpanded("Drafts", true)` (`web/src/components/FileTree.svelte:354`) → `loadTreeDir("Drafts")` (`web/src/state/store.svelte.ts:545`) → `GET /api/files?dir=Drafts` → `list_dir_entries("Drafts")` → `Drive::list("Drafts")` → `drafts_dir_handle.read_dir(".")` (`crates/chan-drive/src/drive.rs:779`).
* Bug 4: Cmd+N → `app.draft.new` chord handler (`web/src/App.svelte:748` → `:895`) → `POST /api/drafts/new` → `routes/drafts.rs` → chan-drive draft-dir creation.
* Bug 3: Graph load — likely calls chan-drive graph endpoints; if any touch the Drafts subtree (or share a stale-config code path), they could fail.

## Acceptance criteria

1. **Reproduce all three bugs** against a drive shape that triggers them. Candidate repro shapes:
   * Fresh drive created with current main: confirms whether bugs are universal or drive-shape-specific.
   * Pre-v0.12.0-style drive: copy `/Users/fiorix/dev/github.com/fiorix/chan/` (or any drive @@Alex registered before v0.12.0) into a throwaway location; register it via `chan add`; observe.
2. **Identify root cause(s)**: confirm or falsify the chained-root-cause hypothesis. If they're independent, name each separately.
3. **Fix each**. Likely fix shapes (depending on root cause):
   * If drafts metadata handle isn't bootstrapped on existing drives → chan-drive needs a "create drafts metadata on first access" path, OR a one-time migration that runs when a drive is opened. @@Systacean lane if it requires chan-drive surgery; coordinate via me if so.
   * If the error surfaces but the SPA UI doesn't show it → fix the UI to surface `tree.dirErrors["Drafts"]` somewhere visible.
   * If POST `/api/drafts/new` returns an error that the SPA silently ignores → add error toast.
4. **Test pins**: regression tests for each (probably one chan-drive integration test + one or two SPA pins).
5. **Gate**: `cargo fmt + clippy + test + npm check + npm test + npm build` all green.

## How to start

1. **Reproduce on a fresh drive first** (~5 min):
   * `cargo build -p chan`.
   * `mkdir /tmp/chan-test-a-100-fresh && ./target/debug/chan add /tmp/chan-test-a-100-fresh`.
   * `./target/debug/chan serve /tmp/chan-test-a-100-fresh/`.
   * Open in browser; expand Drafts; try Cmd+N; open Graph tab.
   * If bugs reproduce on fresh drive → universal regression; bisect main against `chan-v0.12.0`.
   * If bugs DON'T reproduce on fresh drive → drive-shape-specific; reproduce against an older drive shape.
2. **Reproduce on a pre-v0.12.0-style drive** if step 1 didn't trigger:
   * Find any drive on disk that was registered before chan-v0.12.0 tag (mid-May 2026 boundary). E.g., the chan repo itself if it was a registered drive earlier.
   * Or: simulate by creating a drive then DELETING the `.chan/drafts/` subtree (if present) before serving — that mimics "drive existed before Drafts metadata was a thing".
   * Observe whether bugs trigger.
3. **Bisect / inspect**: once you've reproduced, the next move depends on what you see (chan-drive error vs SPA failure vs both).
4. Fix; pin; re-walk.

## Coordination

* Safety guardrail: do NOT touch @@Alex's running chan.app session. Throwaway drives only.
* If the root cause requires chan-drive changes (`crates/chan-drive/`), scope-poke @@Systacean or fold in (their lane is officially the drive crate but chan-server cross-edits have happened before for small bugfix scope).
* If the SPA UI needs error-surfacing, that's pure your lane.
* Coordinate with `-98` (menu gaps) + `-99` (screensaver themes) for sequencing — `-100` is the heaviest of the three because of the repro + bisect; consider tackling first or last per your bandwidth.

## Authorization

Yes for:

* SPA-side edits (`web/src/`).
* chan-server edits (`crates/chan-server/`).
* chan-drive edits (`crates/chan-drive/`) if root cause is there — scope-poke first to give @@Systacean visibility.
* Throwaway drives + dev builds.
* Test pins.

## Out of scope

* Wholesale Drafts feature rework — fix the specific bugs, don't redesign.
* chan-desktop side — bugs surface in SPA + chan-server.

## Reference

* `docs/journals/phase-8/alex/round4.md` — @@Alex's bug list + my triage notes.
* `crates/chan-drive/src/drive.rs:769-820` — Drive::list with Drafts routing.
* `crates/chan-server/src/routes/files.rs:125-160` — synthetic Drafts entry + dir listing.
* `crates/chan-server/src/routes/drafts.rs` — Cmd+N endpoint.
* `web/src/components/FileTree.svelte:891` — Drafts row rendering.
* `web/src/state/store.svelte.ts:545` — loadTreeDir.

---

## 2026-05-23 - candidate fix ready: Drafts stale-tree refresh after New Draft

Repro / falsification pass:

* Fresh throwaway drive under `/private/tmp/chan-a100-fresh`,
  isolated registry via `HOME=/private/tmp/chan-a100-home`,
  served on `127.0.0.1:8799` with `--no-token`.
* API probes on current HEAD:
  * `GET /api/files?dir=` returned synthetic `Drafts`.
  * `GET /api/files?dir=Drafts` returned `[]` on fresh drive.
  * `POST /api/drafts/new` returned
    `{"path":"Drafts/untitled/draft.md","name":"untitled"}`.
  * `GET /api/files?dir=Drafts` then returned
    `Drafts/untitled`.
  * `GET /api/files?dir=Drafts/untitled` returned
    `Drafts/untitled/draft.md`.
  * `GET /api/files/Drafts/untitled/draft.md` returned the
    empty editable file.
  * `GET /api/graph?scope=drive&depth=1` and filesystem graph
    endpoints returned valid payloads.

Root cause found in SPA state, not chan-drive bootstrap:

* `Drive::open` already eagerly creates/opens the per-drive
  drafts directory, and fresh-drive API paths work.
* The failing shape is stale client state: if the File Browser had
  loaded `Drafts` while empty, `tree.loadedDirs["Drafts"]` stayed
  true after `api.createDraft()`.
* `/api/drafts/new` calls `self_writes.note(path)`, so the SPA does
  not get a normal watcher echo to refresh the tree.
* The scoped watcher refresh only refreshed the immediate parent
  of a changed path. For `Drafts/untitled/draft.md`, that parent is
  `Drafts/untitled`, which usually is not loaded yet, so the
  already-loaded `Drafts` listing stayed stale.
* Graph tabs were not failing at the server layer in the fresh
  repro; the visible graph issue is the same missing post-create
  invalidation/reload path when graph tabs are open.

Fix:

* `refreshTreeForPath(path)` now climbs to the nearest loaded
  ancestor instead of no-oping when the immediate parent is not
  loaded. `Drafts/untitled/draft.md` refreshes `Drafts` when that
  subtree is the loaded ancestor.
* Added `noteDraftCreated(path)` to mirror the watcher-side
  invalidation after same-SPA draft creation: refresh nearest tree
  ancestor, schedule drive refresh, invalidate graph data, and bump
  `graphReloadSignal` when graph tabs/overlay are present.
* `createDraftAndOpen()` and staged Hybrid Nav draft materialization
  now call `noteDraftCreated(path)` before opening the file.
* Draft-create failures now surface a transient status message
  instead of only logging to console.

Verification:

* `npm test -- --run src/state/watcherScope.test.ts src/components/newDraftCmdN.test.ts`
  - 2 files passed, 18 tests passed.
* `npm run check`
  - svelte-check 0 errors / 0 warnings.
* `npm test -- --run`
  - 127 files passed, 1 skipped; 1341 tests passed, 11 skipped.
* `npm run build`
  - passed; existing chunk-size / ineffective dynamic import
    warnings only.

No Rust edits in this slice; chan-drive/server API repro on the
throwaway drive passed, so no cargo gate was run for the SPA-only
fix.

## 2026-05-23 — @@Architect: approved + commit clearance (shipped: e364517)

Excellent triage. My hypothesis (chan-drive drafts handle uninitialized on pre-v0.12.0 drives) was wrong — your falsification pass on a fresh throwaway drive confirmed the API path is fine end-to-end. Real root cause was the SPA's `tree.loadedDirs["Drafts"]` staying stale post-`api.createDraft()` because `/api/drafts/new` calls `self_writes.note(path)` (suppressing the watcher echo) AND the existing watcher refresh only re-fetched the immediate parent of the new file (`Drafts/untitled`) not the loaded ancestor (`Drafts`).

Fix shape is right:

* `refreshTreeForPath(path)` climbs to nearest loaded ancestor — generalises the post-write invalidation for any depth-N self-write. Solid primitive.
* `noteDraftCreated(path)` mirrors the watcher-side invalidation for same-SPA writes — the cross-tab pattern for SPA-initiated writes that bypass the watcher.
* Draft-create failures now surface to the user — silent-failure mode (which would have made bug 4 invisible from the UI) closed.
* Tests: 18/18 on the new pin coverage + 1341/1341 full-suite green.

Code shipped at `e364517` under your pre-authorization. This append is documentation-only.

### v0.13.0 blocker status post-`-100`

| Task | Status |
|---|---|
| `-97` (terminal glyph) | ✓ shipped + HOLD |
| `-98` (menu gaps) | ✓ shipped + HOLD (`dd459bb`) |
| `-100` (Drafts chain P0) | ✓ shipped (this); awaits @@WebtestA walk |
| `-101` (tab focus) | ✓ shipped + HOLD (`dd459bb`) |
| `-96` sub-passes 1/2/3 | cleared, non-blocking |
| `-99` (themes + bounds) | open |
| `-102` (menu nits) | open |
| `systacean-45` (chan-server sync-call audit) | in flight |

Thank you for the disciplined repro-first approach — it killed my wrong hypothesis cheaply.

## 2026-05-23 - teardown-complete

No FullStackA-owned server, build, dev-server, or throwaway drive
state remains for this task. Phase-8 stand-down acknowledged.
