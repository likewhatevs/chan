# Round-2 @@LaneC — indexing-never-completes + cs search

## You are @@LaneC

Domain this round: search / indexing (round-1's Lane-B domain). Read
`bootstrap.md`, then this file, then the two index/preflight bugs at the top of
`round-2-part-2.md` and the `cs search` item under "The cs command line", then
`coordination.md`. Coordinate through **@@Architect (@@LaneA)**, not @@Host.
Confirm understanding, then start wave-1 (IDX has no cross-lane dependency).

You may spawn subagents within your scope.

## You own

`crates/chan-workspace/src/indexer.rs` (the indexer state machine),
`crates/chan-server/src/routes/preflight.rs`, `routes/search.rs`, the WS-bus
pieces that carry index state, `web/src/components/AppStatusBar.svelte`,
`web/src/components/PreflightOverlay.svelte`, and the **index/status state
region** of `web/src/state/store.svelte.ts` (NOT `handleWindowCommand` - that
is @@LaneD's region).

For `cs search` you append to `crates/chan/src/main.rs` (cs clap) and
`crates/chan-server/src/control_socket.rs` - both **owned by @@LaneD**. Treat
those as a coordinated append (new disjoint enum arms), gated on **CK-RENAME**
so you build on `cs terminal`, not the old `cs term`. See coordination below.

## Tasks

### Wave 1 — IDX: "indexing never reports complete" (start now)

This is the round's highest-impact bug and the two reported symptoms are **one
root cause** (per the @@Architect review notes in `round-2-part-2.md`):

- Symptom 1: status bar stuck on "reindexing Drafts/untitled/draft.md"
  (display-driven from `/api/index/status` while `state === "reindexing"`,
  never cleared because the poll never sees `Idle`). **Do NOT "fix" this with a
  dismiss timer** - that would hide a genuinely-stuck reindex. Wrong fix.
- Symptom 2: Cmd+R reload hangs chan-desktop on the preflight gate
  (`PreflightOverlay` stays `locked` until `IndexStatus == Idle`,
  `routes/preflight.rs`, polled 750ms). Same missing/stuck Idle transition;
  "workspace off/on" recovers only because restarting chan-server resets the
  indexer to Idle.

Direction (from the review):
1. **Answer Q1 empirically:** does the draft path actually reach `set_idle`
   server-side? Trace `indexer.rs` `apply_watch_change`: `Drafts/<sub>` ->
   `index_draft_file` -> `Indexed` -> `set_idle`. Find where the transition is
   lost / never fires for the draft path.
2. **Make Reindexing->Idle event-driven on the WS bus** instead of poll-only,
   so the SPA learns of completion immediately and the preflight gate unlocks
   reliably on reload.
3. Verify both surfaces clear: the status bar returns to idle and a Cmd+R
   reload unlocks preflight without a server restart.

**Reaches `CK-INDEX-IDLE`** when the Reindexing->Idle path is reliable -> tell
@@Architect; @@LaneD's Ctrl+R reload smoke and anyone reloading benefits.

Browser-smoke required (the bug is timing/state, invisible to static gates):
seed a drive with a shallow clone of this repo (1 editor + 1 terminal + 1 graph
+ 1 search-index dashboard, per @@Host's repro), open a draft, confirm the
status clears; then Cmd+R and confirm preflight unlocks. Ask @@Architect for the
seeded drive. If the bug only reproduces on chan-desktop, say so - but the
review says it reproduces in the browser too.

### Wave 2

- **SEARCH: `cs search`** (`round-2-part-2.md` item 3 + the @@Architect grounding
  block). Reuse `Workspace::search()` via the `routes/search.rs` path; add a new
  `ControlRequest::Search` that returns results on the connection like
  `term list`. **Output convention (resolves the TODO at item 4):** markdown by
  default, `--json` for compact machine output, `--json --pretty` for indented.
  Do NOT invent `--pretty-json`. Search is workspace-wide now (round-1 removed
  scope). Gated on **CK-RENAME** (build the subcommand under `cs search`, and
  add the clap arm / control-socket variant onto @@LaneD's already-renamed
  files as a coordinated disjoint append).
- **TOAST audit** (the smaller separate item under the reindex bug). Audit that
  real toasts (the timer-dismissed notifications, distinct from the
  display-driven index status) all route through the setter-with-timer. This is
  the "do real toasts auto-dismiss?" check the review split out - keep it
  separate from IDX. Small.

## Cross-lane coordination

- **CK-RENAME (inbound, from @@LaneD):** wait for `cs term` -> `cs terminal`
  before landing `cs search`, so the new subcommand and any prefix-match
  behavior are consistent. @@LaneD pokes you when the rename lands.
- **control_socket.rs / main.rs are @@LaneD's files.** Your `cs search`
  additions go in as new, disjoint enum arms. Coordinate the exact insertion
  region with @@LaneD at CK-RENAME; use the chained staged-diff commit
  discipline; tell @@Architect when you land in a shared file so @@LaneD
  rebases.

## Gate + smoke

Repo pre-push gate (Rust touched): `cargo fmt --check`,
`cargo clippy --all-targets -- -D warnings`, `cargo test`,
`cargo build --no-default-features`; `web/`: svelte-check + vitest + build.
Browser-smoke the IDX fix (required). Append progress to
`round-2-lane-c-journal.md` + `event-lane-c.md`; poke @@Architect on
completion/checkpoint.
