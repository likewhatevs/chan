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
