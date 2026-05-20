# @@Systacean's phase-8 journal

Author: @@Systacean
Date: 2026-05-19

Syseng + Rustacean. Owns CLI surface (`crates/chan`), build, deps,
the pinned toolchain, in-tree chan-drive code quality, and the
local pre-push gate. Coordinates with @@CI on signing-key rotation
and the release pipeline; CI workflows themselves are @@CI's lane.

Append-only. New entries go at the bottom under a dated heading.

## 2026-05-19 — systacean-1 landed

`chan list --json` + `chan remove --name <NAME>` shipped as
commit `51984c8` (subject: "chan list --json + chan remove
--name (systacean-1)"). @@Architect cleared the commit on the
task file; push waits for Round-1 close (systacean-3).

## 2026-05-19 — systacean-2 ready

Server-side fix for the graph "edges to files not in repo"
bug landed in the working tree (uncommitted). Repro caught
five real-but-marked-missing files (`LICENSE`, `desktop/LICENSE`,
`crates/chan-drive/src/library.rs`,
`crates/chan-drive/src/registry.rs`,
`docs/journals/phase-1/fake-codex-smoke.sh`) plus 282 link
edges marked `broken: true` whose target was actually on
disk. Root cause: `api_graph`'s resolver universe was
`graph.files() ∪ image_files`, which excluded every non-
markdown / non-image file even when the file existed.

Fix expands the resolver universe to all on-disk files via
new `drive_disk_files` + `image_subset` helpers in
`crates/chan-server/src/routes/graph.rs`; adds a
`referenced_disk_files` set that mirrors `referenced_images`
so non-graph existing files render as solid `File { missing:
false }` nodes. Three new unit tests pin the contract. End-
to-end verification: 0 mismatches after the fix.

Flagged a separate SPA-side follow-up (the screenshot's "not
in the current file listing" hint comes from
`GraphPanel.svelte`'s lazy-tree ghost check, not from the
server) for @@FullStack.

Test drive still mounted at `/tmp/chan-sys2-drv` (chan
registered as "sys2") in case @@Architect / webtest wants
to verify end-to-end. Teardown noted for the systacean-3
push-prep pass.

## 2026-05-19 — backlog: desktop/Makefile bundle-path drift

Side ask from @@Architect on
[`systacean-1`](systacean-1.md): `desktop/Makefile`'s
`app-signed` / `app-notarized` echo lines reference
`src-tauri/target/release/bundle/...` but the bundle actually
lands at the workspace `target/release/bundle/...` after the
Tauri build picks up the workspace `target/`. Not destructive,
just stale user output that points at a path that won't exist
post-merge. Pick up in a later slot (after the Round-1 bug
sweep + release cut; this is a doc/echo polish, not a bug
holding back the round).

## 2026-05-19 — desktop/Makefile bundle-path echoes fixed

Picked up as the between-waves fill-in @@Architect invited on
[`event-architect-systacean.md`](../alex/event-architect-systacean.md).
@@CI's [`ci-2.md`](../ci/ci-2.md#L165) had also routed the
same drift back to me as "@@Systacean lane" — no collision.

### Verification of the claim

`desktop/src-tauri/Cargo.toml`'s `name = "chan-desktop"` is a
workspace member (root `Cargo.toml:16`), so Tauri's `cargo`
invocation writes to the workspace `target/`. `ls`
confirms the live bundle directory at workspace
`target/release/bundle/{macos,dmg}/`; `desktop/src-tauri/target/`
does not exist.

### Changes

`desktop/Makefile`: four stale echo / `ls` lines under
`app-signed` and `app-notarized` switched from literal
`src-tauri/target/release/bundle/...` to
`$(CHAN_REPO)/target/release/bundle/...`, mirroring the
existing pattern in the `chan-bin` recipe
(`cp $(CHAN_REPO)/target/release/chan ...`). `CHAN_REPO ?= ..`
default keeps the rendered path workspace-relative for the
common `cd desktop && make app-signed` invocation; CI overrides
via `CHAN_REPO=/path` on the command line.

`make -n app-signed` / `make -n app-notarized` (with fake
`APPLE_*` env vars to clear the prereqs gate) confirm the
substituted output:

```
echo "signed .app: ../target/release/bundle/macos/Chan.app"
echo "verify: codesign -dv --verbose=2 ../target/release/bundle/macos/Chan.app"
ls -1 ../target/release/bundle/dmg/*.dmg 2>/dev/null || true
echo "          ../target/release/bundle/macos/Chan.app"
```

### Gate

Make-only, doc-shape change. No Rust, Svelte, or workflow
files touched, so `cargo fmt` / clippy / test / `npm run
check` / `npm run build` are unaffected. The pre-push gate
runs in full at `systacean-3` when Round-1 close lands all
these together.

### Commit readiness

Single-file diff against `desktop/Makefile`. Awaiting
@@Architect commit clearance.

Suggested subject:

```
desktop/Makefile: signed/notarized echo paths use workspace target (systacean fill-in)
```

## 2026-05-20 — systacean-2 committed

Per @@Architect's chase on
[`event-architect-systacean.md`](../alex/event-architect-systacean.md)
2026-05-20 — landed as commit `4a04917`:

```
Graph: link resolver universe includes all on-disk files, not just markdown + images (systacean-2)
```

Single-file commit (`crates/chan-server/src/routes/graph.rs`,
+183 / -23). Push parked for Round-1 close. @@WebtestA can
now re-verify bug 8 against the rebuilt binary; will pick
up the 3 directory-typed-as-file cases as separate scope
if they survive the rebuild. Task-file tail in
[`systacean-2.md`](systacean-2.md) carries the same.

## 2026-05-20 — Makefile fill-in committed

Landed as `6b10272`:

```
desktop/Makefile: signed/notarized echo paths use workspace target (systacean fill-in)
```

@@Architect cleared on
[`event-architect-systacean.md`](../alex/event-architect-systacean.md)
2026-05-20 along with the systacean-4 scope answer.
Single-file commit (`desktop/Makefile`, +4 / -4). Push held
for Round-1 close.

## 2026-05-20 — systacean-4 verified + committed

Bug still reproduced on the rebuilt binary post-systacean-2;
root cause was in `api_graph`'s ghost-emission path, NOT in
the indexer (the original task spec's "likely culprits"
guess pointed at an indexer walker). After @@Architect
approved option A (drop dir dsts from ghost + drop edges),
landed as `d35bbd7`:

```
Graph: drop directory link targets from ghost emission (systacean-4)
```

Single-file commit (`crates/chan-server/src/routes/graph.rs`,
+148 / -2). End-to-end against `/tmp/chan-sys2-drv`:
891 → 888 file-kind nodes, 3 → 0 directory-typed-as-file
leaks, 3775 → 3771 edges. Two new unit tests pin the
contract.

Audit-trail aside: first commit attempt accidentally rolled
in three files staged by concurrent agents (one
@@FullStackB task file + two web/ files); undone via
`git reset --soft HEAD~1` + `git restore --staged <files>`
+ clean re-commit. Pre-commit `git status` audit is
mandatory in concurrent-agent working trees; full writeup
in [`systacean-4.md`](systacean-4.md) under "Aside: commit
redo".

Push held for Round-1 close. Next up: systacean-5
(event_watcher "Is a directory" toast on empty watch root).

## 2026-05-20 — systacean-5 committed

Landed as `80a34ee`:

```
event_watcher: skip directory paths instead of treating them as failed event-file reads (systacean-5)
```

Single-file commit (`crates/chan-server/src/event_watcher.rs`,
+58 / -0). Root cause: macOS FSEvents synthetic Create-event
on first attach to a freshly-created dir delivers the watch
root path; `read_to_string` errors with EISDIR; counter ticks
up; rich-prompt toast surfaces via `/api/health`. Fix:
early-return in `ingest_once` when the path is a directory.
No log, no `dropped_events` bump — it's a non-event, not a
dropped event. New unit test
`ingest_once_skips_directory_paths_silently` direct-calls
the guarded function with watch-root + subdirectory paths
and asserts the counter stays 0 + no event dispatched.

Three commits ahead of `main`'s upstream now (Makefile,
systacean-4, systacean-5), plus the earlier systacean-1 and
systacean-2. Push held for Round-1 close. Back to waiting on
@@Architect for the commit-grouping plan.

## 2026-05-20 — Round restructure + systacean-3 cancelled

@@Architect notified that @@Alex returned with a Round
restructure: no v0.11.1 cut for Round 1. Binary release waits
for Round 2 once signed+notarized DMG pipeline is exercised
with real Apple Developer ID keys. New round shape: Round 1 →
Round 2 → Round 3. `systacean-3` (version bump + tag + push)
cancelled for Round 1; a future task replaces it when Round
2 closes.

Detour: shrink the first release. Strip the BGE-small model
from the default binary (~89 MB → ~26 MB). New task
`systacean-6` cuts the build-side gating + runtime resolver;
`systacean-7` (incoming) handles the CLI / API for
download/enable/disable/status; `fullstack-a-21` (incoming)
adds the Settings page.

@@Architect's authorization on systacean-6 explicit-yes for
edits to chan-server/Cargo.toml, chan-drive/Cargo.toml,
workspace Cargo.toml, embed_seed.rs, fetch-models, and
the Makefiles.

## 2026-05-20 — systacean-6 committed

Landed as `8b35c03`:

```
Gate BGE-small model behind embed-model cargo feature + runtime resolver (systacean-6)
```

8 files (Makefile + 4 Cargo.toml-touched manifests/build +
embed_seed.rs + embeddings.rs + facade.rs + chan-server lib.rs),
+269 / -38. New `embed-model` cargo feature (default-off)
gates the bundle; runtime resolver
`chan_drive::index::embeddings::resolve_model(name) ->
Result<PathBuf, EmbedError>` surfaces `ModelNotDownloaded`
when the model isn't on disk + the binary wasn't built with
`--features embed-model`. Pre-commit audit (`git diff
--staged --stat`) clean per the systacean-4 lesson; no
stowaway files.

### Binary size measurement

| Build       | Size  |
|-------------|-------|
| Default     | 25 MB |
| embed-model | 89 MB |

64 MB drop on default — matches the task target.

### Open questions flagged for @@Architect

1. **data_dir migration**: task spec mentions
   `<user-config>/chan/models/` paths matching `dirs::data_dir`,
   but `global_models_dir()` uses `dirs::cache_dir`. Kept
   cache_dir for backward compat with existing installs;
   migration would be a separate task.
2. **Desktop sidecar**: `desktop/Makefile::chan-bin` now
   inherits the lean default. Should desktop opt into
   `--features embed-model` (out-of-the-box Hybrid, +64 MB
   bundle), or lean on the Settings-driven download UX
   from `fullstack-a-21`?

Both flagged in [`systacean-6.md`](systacean-6.md) tail.
Picking up `systacean-7` next once the spec lands.

## 2026-05-20 — systacean-6 cleared + systacean-7 committed

@@Architect cleared systacean-6 with all three of my open
questions answered (cache_dir status-quo OK for now / defer
data_dir migration to Round 3 / leave desktop sidecar on
default features / ModelNotDownloaded shape stays as-is).

systacean-7 landed as `6bf44cd`:

```
chan index download-model | enable-semantic | disable-semantic | status + API (systacean-7)
```

7 files (`crates/chan/src/main.rs`, new
`crates/chan-server/src/routes/index.rs`, mod.rs,
`crates/chan-server/src/lib.rs`,
`crates/chan-drive/src/drive.rs`,
`crates/chan-drive/src/index/config.rs`, facade.rs),
+642 / -15.

### CLI breaking change

`chan index <path>` is now `chan index rebuild <path>`. The
flat positional shape collided with adding the
download-model / enable-semantic / disable-semantic / status
subcommands under `chan index`. The verb-first form matches
`chan config <action>` and forward-compats the Round-2
`chan reports enable/disable` parallel pair. Release-notes
should call this out.

### API contract locked for fullstack-a-21

Four endpoints under `/api/index/semantic/`:

* `GET /state` — open, returns SemanticState.
* `POST /enable` — settings-gated; 409 + structured
  `model_not_downloaded` body when the model isn't on disk.
* `POST /disable` — settings-gated; idempotent.
* `POST /download` — settings-gated; **synchronous** in v1
  (returns when done). Async/202 + progress streaming is
  flagged as a deferred follow-up.

### Gate

fmt + clippy clean across default / embed-model /
no-default-features; `cargo test --all` green. No endpoint
integration tests yet (mock-AppState wiring scope-creep);
flagged for a coverage pass.

### Open follow-ups

1. Async download + progress streaming (deferred from this
   commit).
2. Endpoint integration tests.
3. MCP tool schema update (optional; flagging for
   @@Architect).
4. Release-note line for the `chan index <path>` → `chan
   index rebuild <path>` rename.

### Round-1 systacean queue

* `-1` ✓ committed (push held)
* `-2` ✓ committed (push held)
* Makefile fill-in ✓ committed (push held)
* `-4` ✓ committed (push held)
* `-5` ✓ committed (push held)
* `-3` CANCELLED for Round 1
* `-6` ✓ committed (push held)
* `-7` ✓ committed (push held)

That's the queue. Idle until @@Architect cuts the next
task or the round-recycle event fires.

## 2026-05-20 — systacean-8 committed + systacean-9 scope question

@@WebtestB ran a proactive CLI walk on `6bf44cd` and
surfaced two clusters. @@Architect cut systacean-8 +
systacean-9 — both Round-1 polish, both authorized.

### systacean-8 ✓ committed as `693b161`

```
chan index ergonomics: lock-free status + no auto-register + --path on rebuild (systacean-8)
```

2 files (`crates/chan-drive/src/index/config.rs`,
`crates/chan/src/main.rs`), +98 / -14.

Three fixes in one pass:
1. `chan index status` is now lock-free + registry-only:
   no `Drive::open`, no writer-lock acquire. Live-served
   drives stop blocking the CLI. Implementation uses
   `Library::drive_paths_for(&root)` + direct
   `config::load(&paths.index)`.
2. `chan index status` + `enable-semantic` /
   `disable-semantic` no longer auto-register the path.
   Missing-from-registry → clean
   "not a chan drive at <path>; run `chan add <path>`
   first" via the new `not_a_chan_drive_hint` helper.
3. `chan index rebuild` accepts BOTH the backwards-
   compat positional `<PATH>` AND a uniform `--path
   <PATH>` flag.

New unit test `load_works_while_drive_lock_is_held` in
`chan-drive/src/index/config.rs` pins the lock-free
invariant.

### systacean-9 scope question (no code yet)

Traced the read path. The SPA's `readWatcherEvents` calls
`api.list(dir)` + `api.read(path)` → both route through
`/api/files`, which enforces the drive sandbox →
ENOENT on absolute outside-drive paths.

The task body says "no SPA work needed". But the cleanest
fix shape needs either:

* **Option A**: dedicated `GET /api/terminal/:session/watcher/events`
  endpoint + a small (~5-line) SPA-side patch in
  `web/src/state/watcherEvents.ts` to call it. Tiny
  deviation from "no SPA work".
* **Option B**: bypass the drive sandbox in `/api/files`
  for absolute paths matching active watcher state.
  Strictly no SPA change, but more cross-cutting + the
  axum `*path` wildcard on `/api/files//tmp/foo`
  (double slash) needs experimental verification.

Recommendation A flagged in
[`systacean-9.md`](systacean-9.md) tail under "scope
question for @@Architect". Holding before any code
change.

### Round-1 queue update

* `-8` ✓ committed (push held)
* `-9` blocked on scope decision (option A / B)

Idle until @@Architect responds on `-9`.

## 2026-05-20 — systacean-9 committed (Option A landed)

@@Architect approved Option A with explicit cross-lane
SPA authorization (one-time crossing for a logically-
coupled small change). Landed as `c69e2fc`:

```
Watcher events: dedicated /api/terminal/:session/watcher/events endpoint (systacean-9)
```

7 files (4 Rust + 3 web), +232 / -44. Server endpoint
reads from `Registry::watcher_dir` directly via
`std::fs::read_dir` + `read_to_string` — bypasses the
drive sandbox that was the source of ENOENT. SPA's
`readWatcherEvents(sessionId)` calls the new endpoint
in one shot instead of composing `api.list + api.read`
per file.

Two new server tests (`is_watcher_event_filename_matches_spa_regex`,
`list_watcher_events_reads_outside_drive_dir`) plus an
updated SPA-side test (`reads pre-flight event files via
the watcher-events endpoint`). All gates green: fmt,
clippy, cargo test, svelte-check, vitest, vite build.

@@WebtestB to re-verify on lane-B
(`/tmp/chan-watch-wave3-outside/`) once the rebuilt
binary + web bundle are in place.

### Round-1 queue update

* `-8` ✓ committed (push held)
* `-9` ✓ committed (push held)

Both Round-1 polish tasks done. Back to idle until next
task or recycle.
