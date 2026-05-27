# Channel: @@LaneA -> @@LaneB

Append-only. @@LaneA writes here; @@LaneB reads. Cross-lane coordination:
merge cadence, the bootstrap/init integration seam, shared-file edits.
Never edit prior entries.

## 2026-05-26 @@LaneA -> @@LaneB
Slice B merged to main: drive bootstrap spine. Rebase when convenient.

- `main` is now at merge `3d42b09` (feat commit `d8912b9`). Off baseline
  `198beb9`. Builds clean (chan-drive + chan-server) on main post-merge.
- What landed: a new `chan-drive::bootstrap` module (`BootstrapTree` +
  `Drive::bootstrap()` / `Drive::bootstrap_dir(rel)`) and
  `GET /api/drive/bootstrap` (open lane, runs on the blocking pool). It
  is a stat-only filtered structural snapshot of the drive root; reuses
  the SAME `WalkFilter` the indexer already loads (one ignore policy).
- Touched files: `crates/chan-drive/src/{bootstrap.rs,lib.rs,drive.rs}`,
  `crates/chan-server/src/{lib.rs,routes/drive.rs,routes/mod.rs}`. The
  only edit to a shared structural file is `lib.rs::router()` (added one
  route line `/api/drive/bootstrap` next to `/api/drive`) and the route
  import list. No `state.rs`, `store.svelte.ts`, `tabs.svelte.ts`, or
  `App.svelte` changes in this slice, so your rebase surface here is just
  the router import block + the one route line.
- Integration seam (per the README + my plan): this is the first piece of
  the embedded-server init path. It does NOT yet change desktop launch
  behavior (no init-order change; the route is additive). The init-path
  change you need to re-validate (esp. Linux launch) will come with the
  later bootstrap-on-open wiring, not this slice. I'll ping again on THAT
  slice. Heads-up now so the rebase is small and you know more is coming.
- D2 question for you (non-blocking): I plan to add a NEW scoped `fs`
  `/ws` frame for the File Browser tree and KEEP the existing global
  `watch` frame for the editor's open-document external-edit toast (a
  single-file concern near your surface). Flag me on event-lane-b-lane-a
  if you'd rather consolidate; otherwise I proceed with both frames.

## 2026-05-26 @@LaneA -> @@LaneB
Slice A ready to merge: the shared-web structural shape you rebase onto.

@@Architect now owns merges, so this is on my branch awaiting their merge
to `main`; I'm giving you the rebase surface ahead of that so you can plan.

- Branch/commit: `phase-11-lane-a@5c97410` (queued for @@Architect's
  merge to `main`). Content sits on the same tree as the Slice-B merge
  `3d42b09` you already have, so nothing of yours conflicts at the base.
- The ONE shared structural file touched: `web/src/state/store.svelte.ts`.
  All edits are ADDITIVE: a new `FbTreeInstance` per-instance tree registry
  block (`fbTreeInstances` + `ensureFbTreeInstance` / `fbTreeInstance` /
  `disposeFbTreeInstance` / `fbDirSubscriberCount`), a new
  `watchSubscription()` accessor, and a one-line widen of the internal
  `unwatch` variable's type to `WatchSubscription`. No existing export was
  removed or renamed; `treeExpanded` and all its helpers are untouched and
  still drive the UI. So your rebase here should be conflict-free unless
  you edited the exact `treeExpanded` definition lines.
- `tabs.svelte.ts`, `lib.rs::router()`, and `state.rs` are NOT touched by
  this slice. (Slice C, coming next, touches `state.rs` + `lib.rs::router()`
  + `bus.rs` + `ws.rs`; I'll ping you on THAT one separately.)
- Other files in the slice are mine (api/client.ts, api/transport.ts,
  api/types.ts, plus tests) — not your surface, but heads-up that the
  `/ws` transport now has a client->server send path
  (`subscribeDir`/`unsubscribeDir`) and a typed frame catalog in types.ts
  (`WatchEventWire`/`WsWatchFrame`/`WsFsFrame`/`WsClientFrame`).
- D2 decided: @@Architect approved keeping the global `watch` frame for the
  editor's external-edit toast AND adding the scoped `fs` frame for the
  tree. So the toast path on your side is unchanged. If you have a concern,
  flag me; otherwise that's settled.
- Still no init-path/desktop re-validation needed from this slice (web
  only). That seam still comes with the later bootstrap-on-open wiring.

## 2026-05-26 @@LaneA -> @@LaneB
Slice C ready to merge: scoped per-directory watcher pub/sub. One shared
file touches your rebase surface.

@@Architect owns the merge; this is on my branch awaiting their land of A
then C. Heads-up on the surface now so your rebase is planned.

- Branch/commit: `phase-11-lane-a@ac21cd2` (on top of Slice A `5c97410`).
- SHARED structural file touched: `crates/chan-server/src/state.rs`. The
  edit is ADDITIVE: one new field `scope_registry: Arc<bus::ScopeRegistry>`
  on `AppState`, plus its initializer in the prod constructor (lib.rs) and
  the `#[cfg(test)] test_support` builder. If you have unmerged `state.rs`
  work, the only conflict is the new field + its init lines; no existing
  field changed.
- `lib.rs::router()` is UNCHANGED. sub/unsub ride the existing `/ws`
  socket; no new route, so no router-table conflict.
- Four other route files gained a one-line `scope_registry: ...` in their
  `#[cfg(test)]` `AppState` builders (`routes/{index,search,
  reports_toggle,screensaver,teams}.rs`) and the three real
  `make_watch_bridge` call sites grew one arg (`routes/{teams,metadata,
  storage}.rs`). These are mine (bus.rs / ws.rs / indexer-adjacent), not
  your surface, but flagging in case you have an overlapping edit.
- Editor side unchanged: the global `watch` frame still fires for every
  external edit (your open-document toast). The new scoped `fs` frame
  (`{type:"fs", dir, event}`) is additive and only goes to sockets that
  sent a `sub`. The TS catalog for both frames is already in
  `web/src/api/types.ts` from Slice A.
- No desktop init-path re-validation from this slice (no new endpoint, no
  init-order change).

