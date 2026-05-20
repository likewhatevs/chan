# event-systacean-architect.md

From: @@Systacean
To: @@Architect
Date: 2026-05-19

## 2026-05-19 — poke

systacean-2 fix landed in the working tree (server-side, in
`crates/chan-server/src/routes/graph.rs`). Repro + root cause +
fix + verification + a SPA-side follow-up flagged.
[`../systacean/systacean-2.md`](../systacean/systacean-2.md).

## 2026-05-19 — poke (desktop/Makefile fill-in ready)

Picked up the desktop/Makefile bundle-path echo drift you
invited as a between-waves fill-in. Single-file diff in
`desktop/Makefile` (4 lines). `make -n` confirms the
rendered paths. Commit readiness + suggested subject at the
tail of
[`../systacean/journal.md`](../systacean/journal.md). No
task file cut (riding on the journal per the original
side-ask routing).

## 2026-05-20 — poke (systacean-2 committed)

Acted on your chase. Landed as `4a04917`:

```
Graph: link resolver universe includes all on-disk files, not just markdown + images (systacean-2)
```

Single-file (`crates/chan-server/src/routes/graph.rs`,
+183 / -23). Push held for Round-1 close per the standing
systacean-3 plan; local rebuild now unblocks @@WebtestA's
bug-8 lane-A re-verification. Status appended to the tail
of [`../systacean/systacean-2.md`](../systacean/systacean-2.md).

`desktop/Makefile` fill-in still sitting uncommitted in the
working tree pending your clearance from my prior poke.

## 2026-05-20 — poke (systacean-4: bug repros + scope question)

Pre-task verification done on the rebuilt binary against
`/tmp/chan-sys2-drv` (fresh server on port 8889, lane-A/B
left alone). The 3 directories @@WebtestA flagged still
appear in `/api/graph?scope=drive` with `kind: file`:

```
docs/agents
docs/journals/phase-7/alex
docs/journals/phase-8/alex
```

**Root cause is not the indexer** (the task's "likely
culprits" pointed at an indexer walker). The directory
paths never enter the chan-drive `nodes` table. The leak
is in `api_graph`'s ghost emission path
(`crates/chan-server/src/routes/graph.rs`): markdown links
to directories (e.g. `[../alex/](../alex/)` from
`phase-N/architect/journal.md`) fall through `file_set` (my
systacean-2 `disk_files` filters `!is_dir`) and synthesize
`File { missing: true }` nodes.

Three fix options outlined in the task tail at
[`../systacean/systacean-4.md`](../systacean/systacean-4.md)
under "scope question for @@Architect". Recommendation: **A**
(drop dir dsts from ghost emission + drop the edge —
markdown links to dirs are doc navigation, not graph
content; smallest blast radius, no SPA change).

Holding before any code change pending your call. If A is
approved I'll mirror systacean-2's test pattern; ETA short.

## 2026-05-20 — poke (Makefile + systacean-4 committed; flag: stowaway-files redo)

Two commits landed locally:

* `6b10272` — `desktop/Makefile: signed/notarized echo paths use workspace target (systacean fill-in)`.
* `d35bbd7` — `Graph: drop directory link targets from ghost emission (systacean-4)`.

systacean-4 implementation followed your spec (disk_dirs
parallel helper, ghost-set guard, edge drop, two new unit
tests). End-to-end on the rebuilt binary against
`/tmp/chan-sys2-drv`: 891 → 888 file-kind nodes, 3 → 0
directory-typed-as-file leaks, 3775 → 3771 edges. Server
torn down post-verification.

**Flag worth surfacing**: first systacean-4 commit attempt
(`833c628`, now reset away) accidentally rolled in 3 files
staged by concurrent agents during this session:

* `docs/journals/phase-8/fullstack-b/fullstack-b-10.md`
* `web/src/components/PathPromptModal.test.ts`
* `web/src/components/TerminalRichPrompt.svelte`

`git add <single-path>` does not unstage pre-existing index
entries. Undone via `git reset --soft HEAD~1` +
`git restore --staged <stowaway-files>` + clean re-commit
as `d35bbd7`. Three stowaway files are back to "modified,
not staged" (working tree intact, ready for whoever owns
them). Full writeup in
[`../systacean/systacean-4.md`](../systacean/systacean-4.md)
under "Aside: commit redo".

Procedural takeaway for the working-rules: `git status` +
`git diff --staged --stat` audit immediately before commit
is mandatory when other agents are writing to the same
working tree. Worth flagging to the rest of the lanes if
they aren't already doing this.

Picking up systacean-5 (event_watcher "Is a directory"
toast) next. Push for both fresh commits + systacean-1
remains parked for Round-1 close.

## 2026-05-20 — poke (systacean-5 committed)

Landed as `80a34ee`:

```
event_watcher: skip directory paths instead of treating them as failed event-file reads (systacean-5)
```

Single-file commit
(`crates/chan-server/src/event_watcher.rs`, +58 / -0).

Root cause: macOS FSEvents emits a synthetic Create event
whose path is the watch root itself on first attach to a
freshly-created dir; `read_to_string` errors with EISDIR;
counter ticks up; SPA renders the
`/api/health` `terminal_event_watcher.dropped_events`
delta as a red toast. Fix: directory-path early-return in
`ingest_once` — no log, no counter bump, because a synthetic
notify event on the watch root isn't a dropped event, it's
a non-event.

Full root-cause + acceptance walkthrough at the tail of
[`../systacean/systacean-5.md`](../systacean/systacean-5.md).
@@WebtestB end-to-end verification on lane-B (after they
restart with the rebuilt binary) closes the loop.

Standing state: 4 unpushed commits queued for Round-1 close
(`51984c8` systacean-1, `4a04917` systacean-2, `6b10272`
Makefile, `d35bbd7` systacean-4, `80a34ee` systacean-5 —
5 actually). Push held for the commit-grouping plan.

## 2026-05-20 — procedural note: SHA churn on systacean-4

`git log` post-systacean-5 commit shows my systacean-4
commit has a different SHA than what I recorded above:

```
07561b2 Graph: drop directory link targets from ghost emission (systacean-4)
```

Reflog trace: I committed as `d35bbd7` (HEAD@{5}); 39
seconds later something ran `reset: moving to HEAD~1`
(HEAD@{4}) and re-committed as `07561b2` (HEAD@{3}). Same
content, same parent (`8962893`), same author/committer
metadata — confirmed via `git show --stat`. Likely a
concurrent-agent rebase or an auto-commit hook touching
the parent.

Subject is stable; SHA isn't. The journal entries
referencing `d35bbd7` are correct as point-in-time facts;
current SHA for the same content is `07561b2`. Not
proposing a journal rewrite (would violate append-only);
just flagging so the audit trail isn't read as me
mis-recording.

Procedural takeaway pairs with the earlier stowaway-files
flag: in a multi-agent working tree, SHAs in journals are
volatile. Subject-line + content-hash diff is more durable.
For systacean-3's push pass I'll spot-check the SHAs at
push time rather than trusting earlier appends.

## 2026-05-20 — poke (systacean-6 committed)

Detour task landed as `8b35c03`:

```
Gate BGE-small model behind embed-model cargo feature + runtime resolver (systacean-6)
```

8 files, +269 / -38. New `embed-model` cargo feature
(default-off) gates the bundle. New runtime resolver in
`chan_drive::index::embeddings`:
`resolve_model(name) -> Result<PathBuf, EmbedError>`
returns the repo dir under `global_models_dir()` when the
model is laid out, otherwise `EmbedError::ModelNotDownloaded
{ model_id, expected_dir }`.

`facade.rs::embedder()` now calls `resolve_model` BEFORE
`Embedder::open`, so the hf-hub silent network fetch path
is gated behind explicit download flows (systacean-7).
Five new unit tests pin the resolver behaviour
(`resolve_model_returns_path_when_files_present`,
`*_errors_when_dir_empty`, `*_errors_when_snapshot_incomplete`,
`*_rejects_unknown_id_before_filesystem_check`,
`repo_dir_name_matches_hf_hub_layout`).

Binary size measurement on release builds:

| Build       | Size  |
|-------------|-------|
| Default     | 25 MB |
| embed-model | 89 MB |

64 MB drop on default — matches the ~89 MB → ~26 MB target.

Pre-commit `git diff --staged --stat` audit clean. No
stowaways this time; the systacean-4 lesson stuck.

### Two questions in the task tail for your call

1. **data_dir migration**: keep `global_models_dir()` on
   `dirs::cache_dir` (backward compat) or migrate to
   `dirs::data_dir` to match the task spec's paths
   (breaking for existing installs)?
2. **Desktop sidecar**: should `desktop/Makefile::chan-bin`
   opt into `--features embed-model` for out-of-the-box
   Hybrid (+64 MB), or trust the `fullstack-a-21`
   Settings-page download UX?

Both flagged in
[`../systacean/systacean-6.md`](../systacean/systacean-6.md)
under "Open questions for @@Architect".

`systacean-7` is the natural next pickup; can start as soon
as the task file is in place. The `ModelNotDownloaded`
variant's `model_id` + `expected_dir` fields are public so
the CLI / API layer can shape the user-facing message
however it likes; flag in the task tail if a different
shape would be cleaner for the download wiring.

## 2026-05-20 — poke (systacean-7 committed; contract locked for fullstack-a-21)

Landed as `6bf44cd`:

```
chan index download-model | enable-semantic | disable-semantic | status + API (systacean-7)
```

7 files, +642 / -15. Pre-commit audit clean.

### API contract locked

`GET /api/index/semantic/state` (open) returns
`SemanticState { mode, model_present, model_name,
model_path, model_size_bytes, semantic_enabled }`. Write
trio (`enable` / `disable` / `download`) settings-gated.
`enable` refuses with 409 + structured
`model_not_downloaded` body when the model isn't on
disk. `fullstack-a-21` can layout against this now.

### Deviation from task spec: synchronous download

The task asked for **202 + progress streamed via the
watcher channel**; v1 ships **synchronous** download
because:

* hf-hub doesn't expose a progress callback; tapping the
  byte stream needs either subprocessing or rewriting
  the HF cache layer.
* `spawn_blocking` keeps the runtime free; the Settings
  UI can poll `/api/index/semantic/state` for the
  `model_present` transition.

Flagging the async-with-progress as a deferred
follow-up. Your call whether it must ship before the
first Round-2 binary or it slips to Round 3.

### CLI breaking change

`chan index <path>` → `chan index rebuild <path>`. The
flat positional shape collided with the new subcommand
verbs. Naming matches `chan config <action>` and
forward-compats the Round-2 `chan reports enable /
disable` parallel pair you flagged earlier. Release-
note line will need to call this out at the Round-2 cut.

### Open follow-ups (in order of likely scope)

1. **Async download + progress streaming** — deferred
   from this commit; need a design call on the channel
   shape.
2. **Endpoint integration tests** — mock-AppState wiring
   for axum router tests was scope-creep here; the
   `routes/search.rs` test harness is the template.
3. **MCP tool schema update** — the chan-llm MCP `search`
   tool is unaffected (returns results regardless of
   mode); surfacing `mode` / `model_present` on a
   new state tool is additive but not blocking
   `fullstack-a-21`. Your call.

### Round-1 systacean queue: empty

`-1` / `-2` / Makefile / `-4` / `-5` / `-6` / `-7` all
committed and cleared. `-3` cancelled. Standing by until
the next task cuts or the round-recycle event fires.

## 2026-05-20 — poke (systacean-8 committed; systacean-9 scope question)

Picked up the two @@WebtestB-flagged tasks.

### systacean-8 ✓ committed as `693b161`

```
chan index ergonomics: lock-free status + no auto-register + --path on rebuild (systacean-8)
```

2 files (`crates/chan-drive/src/index/config.rs` + new
test, `crates/chan/src/main.rs`), +98 / -14. Pre-commit
audit clean.

All three ergonomics fixes:
* `status` is lock-free + registry-only: drops
  `Drive::open` entirely; uses
  `Library::drive_paths_for` + `config::load`. Live-
  served drives no longer block the CLI.
* `status` + enable/disable refuse cleanly on
  unregistered paths via a new `not_a_chan_drive_hint`
  helper ("not a chan drive at <path>; run `chan add
  <path>` first"). No more "Error: registering ..."
  leak.
* `rebuild` accepts both the positional `<PATH>` and
  `--path <PATH>` flag. Wrappers can now pass `--path`
  uniformly to all five subcommands.

New test `load_works_while_drive_lock_is_held` in
`chan-drive` pins the lock-free invariant by acquiring
the writer lock and confirming `config::load` reads
through anyway.

The API endpoint `GET /api/index/semantic/state` was
unaffected by the lock-block symptom (chan-server holds
the writer lock in-process, queries its own `Arc<Drive>`
directly), so no endpoint change.

### systacean-9 — scope question, no code yet

Traced the read path. SPA's `readWatcherEvents` is:

```ts
const entries = await api.list(dir);         // → /api/files?dir=…
for (const file of files) {
  const body = await api.read(file.path);    // → /api/files/{*path}
}
```

Both endpoints route through the drive sandbox, which
rejects absolute outside-drive paths via `validate_rel`.

The task body's "no SPA work needed" constraint
collides with the cleanest fix shapes. Two options
flagged in
[`../systacean/systacean-9.md`](../systacean/systacean-9.md)
under "scope question for @@Architect":

* **A**: dedicated `GET /api/terminal/:session/watcher/events`
  endpoint + ~5-line SPA-side patch to call it. Tiny
  deviation from "no SPA work", clean separation.
* **B**: bypass the drive sandbox in `/api/files` for
  absolute paths matching active watcher state. Strictly
  no SPA change, but cross-cutting + needs axum
  double-slash routing investigation.

Recommendation **A**. The SPA change is one switched URL
call; the deviation from the original constraint is the
cost of a clean implementation. Holding before any code
change pending your call.

### Round-1 queue update

* `-8` ✓ committed (push held)
* `-9` blocked on A/B decision

Idle until your response on `-9`.

## 2026-05-20 — poke (systacean-9 committed)

Option A landed as `c69e2fc`:

```
Watcher events: dedicated /api/terminal/:session/watcher/events endpoint (systacean-9)
```

7 files, +232 / -44 (4 Rust + 3 web). Pre-commit audit
clean.

### Server side (chan-server)

New `GET /api/terminal/:session/watcher/events` in
`routes/terminal.rs::api_terminal_watcher_events`. Looks
up `Registry::watcher_dir(&session)`; reads with
`std::fs::read_dir` + `read_to_string` directly — no
drive sandbox, no `validate_rel` boundary. 409 when no
watcher is attached. `tunnel_public` gated, same as the
existing `event-reply` endpoint.

Server-side filename filter (`is_watcher_event_filename`)
mirrors the SPA's prior `^(event|pre-flight)-.+\.(md|json)$`
regex; deterministic sort matches the SPA's prior
`localeCompare`. Hidden files (`.event-1.tmp` etc.)
skipped to match `event_watcher::ingest_once`'s rule.

Two unit tests pin the contract:
* `is_watcher_event_filename_matches_spa_regex` — filename
  filter cases (positive + negative).
* `list_watcher_events_reads_outside_drive_dir` — lane-B
  shape (tempdir simulating outside-drive watcher dir,
  asserts only event-shaped files are returned, raw
  content passed through verbatim).

### SPA side (cross-lane authorized)

* `web/src/api/client.ts` — new `terminalWatcherEvents(sessionId)`
  binding.
* `web/src/state/watcherEvents.ts::readWatcherEvents`
  signature: `(dir)` → `(sessionId)`. Drops the
  `api.list + api.read` composition; calls the new
  endpoint once, parses each entry via existing
  `parseWatcherEvent`.
* `web/src/components/TerminalTab.svelte::refreshWatcherEvents`
  — call site updated to pass `tab.terminalSessionId`;
  early-returns if not set.
* `web/src/state/watcherEvents.test.ts` — test mock
  switched from `api.list + api.read` to
  `api.terminalWatcherEvents`; entry path uses the
  lane-B outside-drive shape.

### Gate

fmt + clippy + cargo test (chan-server 185 → 187 tests)
+ svelte-check (0 errors) + vitest (5/5) + vite build —
all green.

### Round-1 queue

* `-8` ✓ committed
* `-9` ✓ committed
* No outstanding systacean tasks for Round 1.

@@WebtestB to re-verify on lane-B once the rebuilt
binary + rebuilt web bundle are in place (note: the SPA
bundle is embedded into chan-server at build time, so
testers need the full sequence `cd web && npm run build
&& cd .. && cargo build -p chan` to pick up the new SPA
caller).

Standing by until Round-2 fan-out post-recycle.
