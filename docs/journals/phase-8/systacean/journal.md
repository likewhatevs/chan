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

## 2026-05-20 — Round-1 teardown confirmation

Per @@Architect's teardown-checklist poke (Round-1 close /
pre-recycle):

* `/tmp/chan-sys2-drv` — removed:
  * `chan remove /tmp/chan-sys2-drv` → "unregistered:
    /tmp/chan-sys2-drv".
  * `rm -rf /tmp/chan-sys2-drv` → directory gone.
  * `chan list` confirms no `chan-sys2-drv` entry remains.
* No leftover `chan serve` against `/tmp/chan-sys2-drv`
  (port 8889) — already torn down post-`systacean-4`
  verification. `pgrep -fl "chan serve"` shows three
  running serves and none are mine: WebtestA on 8787,
  WebtestB on 8820, and @@Alex's `ChanRoadmap` personal
  drive on 8888.
* `target/fetch-models-cache/` and any
  `target/release/bundle/` artifacts: left as-is per
  @@Architect's note (cargo build artifacts, cleaned by
  `cargo clean` when @@Alex wants the space).
* No webtest lane footprint to clean (my lane is code-only;
  the webtest lanes own their own throwaways).

Lane footprint clean. Standing by for the recycle event.

## 2026-05-20 — systacean-10 ready (rich-prompt mini-wave patch)

Picked up [`systacean-10`](systacean-10.md) on resume:
event watcher convention tightening. Mirror the SPA /
systacean-9 regex `^(event|pre-flight)-.+\.(md|json)$`
in `event_watcher::ingest_once`, document the
convention in the module doc + `phase-8/process.md`.

Two-file change in my lane:

* `crates/chan-server/src/event_watcher.rs` (+158 / -5):
  module doc, `is_watcher_event_filename` helper,
  filter call in `ingest_once`, three new tests
  (matching-regex pin, silent-skip non-matching name,
  warn-on-bad-JSON-with-matching-name).
* `docs/journals/phase-8/process.md` (+28 / -0): new
  "Watcher event-file naming convention" section
  cross-referencing the three filter sites.

Full gate green for my work: fmt, clippy
`-D warnings`, workspace test (8/8 event_watcher tests
including the 3 new ones), no-default-features build,
svelte-check (0e 0w), vitest (506/506), npm build.
Working tree audit: pre-commit `git diff` will stage
exactly my two files; three @@FullStackB files showing
as modified belong to that lane and stay un-staged
(systacean-4 lesson).

### Pre-existing gate finding (flagged separately)

`RUSTFLAGS=-D warnings cargo build --no-default-features`
fails on `not_a_chan_drive_hint` in
`crates/chan/src/main.rs:1540` — pre-existing dead_code
from systacean-8. Both callers are
`#[cfg(feature = "embeddings")]`-gated but the function
definition isn't. Will block the patch-release push.
Flagging to @@Architect; one-line `#[cfg]` add fixes
it. Out of systacean-10's scope.

Commit-readiness append at the tail of
[`systacean-10.md`](systacean-10.md). Awaiting
@@Architect clearance. Push held per the patch-release
coordination.

## 2026-05-20 — systacean-10 committed + dead_code follow-up committed

@@Architect cleared -10 + authorized the dead_code
follow-up as a separate commit on my lane.

* `6bae20b` — `event_watcher: silently skip non-matching filenames; document naming convention (systacean-10)` (5 files: chan-server + process.md + task tail + journal + event log).
* Follow-up `<sha-pending>` — `chan/src/main.rs: gate not_a_chan_drive_hint on embeddings feature (systacean-8 follow-up)`. One-line cfg gate; `RUSTFLAGS=-D warnings cargo build --no-default-features` now green.

Pre-commit `git diff --staged --stat` audits both clean.
Push held until @@Architect publishes the
patch-release commit-grouping plan + the @@FullStackA /
@@FullStackB rich-prompt tasks land. Queue empty for
the mini-wave; systacean-3 (version-bump + tag + push)
re-activates when the plan publishes.

## 2026-05-20 — chan v0.11.1 cut + pushed

Plan published, gates cleared, GO received. Executed
the full version-bump + tag + push.

* Pre-push gate green: fmt, clippy `-D warnings`,
  workspace test (all green), no-default-features
  build (`-s-8` follow-up `c1e9c41` unblocked it),
  svelte-check (0 errors), vitest (544/544), vite build.
* 5 manifests flipped `0.11.0` → `0.11.1`
  (`Cargo.toml`, `Cargo.lock`, `tauri.conf.json`,
  `package.json`, `package-lock.json`). Runtime
  check: `chan --version` → `chan 0.11.1`.
* Single release commit `2c6680b` (`chan v0.11.1`),
  pre/post-commit audits clean.
* Annotated tag `chan-v0.11.1` at `33dfd63`; body
  verbatim from the commit-plan's draft.
* `git push origin main --follow-tags`:
  `18bdb34..2c6680b main -> main` + `[new tag] chan-v0.11.1`.
  Remote `ls-remote` confirms both refs.

Tag-triggered `release-desktop.yml` fires; unsigned
matrix entry produces the binaries for the post-tag
walkthrough smoke tests by @@WebtestA / @@WebtestB.
Apple Developer ID signing stays Round-2 work per
request.md.

systacean-3 task acceptance criteria all met; full
audit anchor at the tail of
[`systacean-3.md`](systacean-3.md). Architect
notification poke fires next.

Round-1 + mini-wave queue all closed. Standing by for
the agent-recycle event + Round-2 fan-out.

## 2026-05-20 — Round-2 Wave-1 dispatch picked up

Fresh @@Systacean session. Round-2 decisions all locked
2026-05-20; Wave-1 north-star track dispatched. Queue:

* `systacean-11` — chan-desktop signing-key rotation
  (DEV → release Developer ID). Parked on @@Alex's
  identity-string answer. Permission event fired to
  [`../alex/event-systacean-alex.md`](../alex/event-systacean-alex.md).
  Architect confirmed (via inbound poke) that @@Alex
  completed the ci-3 brief checklist in a prior chan
  incarnation → branch (a); just need the actual string.
* `systacean-12` — `tauri-plugin-updater` cross-platform
  verification. In flight; deferred deep dive until -13
  closes (architect noted -13 is a natural fill-in fit).
* `systacean-13` — NEW: Keychain-driven
  `make app-notarized`. Pre-authorized; @@Alex's in-chat
  confirmation (profile name = `chan`, blessed mechanism
  = `xcrun notarytool store-credentials`) pre-answered
  the only open question. Land first per architect's
  ordering recommendation.

## 2026-05-20 — systacean-13 implementation ready

Implemented + render-checked. Awaiting @@Architect
clearance + @@Alex's smoke-test result.

**Load-bearing finding**: `tauri-bundler` 2.8.1's
`notarize_auth` (in `bundle/macos/sign.rs:96`) accepts
ONLY `APPLE_ID`/`APPLE_PASSWORD`/`APPLE_TEAM_ID` or
`APPLE_API_*` env vars. NO `APPLE_KEYCHAIN_PROFILE`
support. To honour the architect's option-1 spec
(`--keychain-profile chan`), had to split build from
notarize: Tauri builds + signs (with the three notarize
env vars unset, so its internal notarize step is
skipped); the Makefile then calls
`xcrun notarytool submit` + `xcrun stapler staple`
directly with the appropriate auth flag based on
detected mode (env > Keychain profile > error).

CI behaviour stays identical (same credentials reach
notarytool, manual call instead of via tauri-bundler).

### Changes

* `desktop/Makefile` (~50 lines added, ~10 reshaped):
  new `NOTARIZE_PROFILE ?= chan` variable,
  `NOTARIZE_VIA` mode detection (env vs keychain),
  `notarize-prereqs` rewritten with a two-option setup
  hint, `app-notarized` refactored to split build from
  notarize+staple.
* `desktop/CLAUDE.md` (~60 lines appended at end): new
  "Local notarization setup" section. Appended at end
  deliberately to keep my diff hunk separated from
  `fullstack-b-15`'s in-flight "Bundled chan sidecar"
  hunk near the top. Pre-commit `git add -p` partition
  per the `systacean-4` shared-worktree lesson.

### Render checks

`make -n app-notarized` with env credentials produces
the env-branch shell expansion (correct shape: `env -u
APPLE_ID -u APPLE_PASSWORD -u APPLE_TEAM_ID cargo tauri
build ...` then `xcrun notarytool submit ... --apple-id
... --password ... --team-id ... --wait` then `xcrun
stapler staple ...`). `make -n notarize-prereqs` without
credentials falls through to the help-text option-1
/ option-2 block as expected. Keychain branch unrendered
on this agent (no profile on the build host); branch
shape is symmetric to env so trusting the conditional.

### Pre-push gate

Scope per task acceptance: Makefile + Markdown only. No
Rust / Web / JSON changes. `make -n` validates rendered
recipe. Full workspace gate untouched + green at HEAD.

### Smoke test ask

@@Alex runs `cd desktop && make app-notarized` from a
bare shell (no env exports) on the workstation; expected
output starts with "notarizing <DMG> (via Keychain
profile 'chan')"; ends with a `.dmg` that opens cleanly
on a second Mac with no Gatekeeper warning. Detailed
expectations + fallback (`NOTARIZE_PROFILE=<name>` if
their profile is under a different label) at the tail
of [`systacean-13.md`](systacean-13.md).

Architect poke fires next.

## 2026-05-21 — systacean-13 committed

Cleared + landed as `01f10d3`:

```
desktop/Makefile: notarytool keychain-profile path for local make app-notarized (systacean-13)
```

5 files, +561 / -16. Pre/post-commit audits clean; the fb-15 sidecar section in desktop/CLAUDE.md was already in HEAD via architect's `7845402`, so the partition discipline was easier than expected — only 1 hunk in my CLAUDE.md diff at staging time. Push held per Round-2 policy (no patch tag cut yet; @@Alex deciding v0.11.2 scope in `round-2-open-questions.md` A.5).

Queue post-`-13`:

* `-11` still parked on @@Alex's identity-string answer (`round-2-open-questions.md` B.1).
* `-12` (tauri-plugin-updater verify) — natural resume; can drive desk-work (upstream docs read + test minisign keypair generation + mock-feed JSON authoring) without runtime permission. The Tauri-build + Chan.app-launch click-through walk needs a separate runtime permission event when ready.
* `-13` smoke test runs independently on @@Alex's workstation (`round-2-open-questions.md` B.3).
* No new tasks queued per architect's "After -13 commits" note.

## 2026-05-21 — -13 orphaned by upstream reset; re-committing

`01f10d3` was reset away (`HEAD@{5}: reset: moving to HEAD~1`) before ci-7 + fb-15 + architect-v0.11.2-dispatch + fb-16 landed sequentially on top of the older base. Working-tree content survived (CLAUDE.md "Local notarization setup" section + Makefile rewrites + systacean-13.md untracked) but no longer reachable from main. None of the four newer HEAD commits include my -13 changes — verified by `git show HEAD:<file> | grep` against NOTARIZE_PROFILE / "Local notarization setup" / `git ls-files systacean-13.md`. Cause: multi-agent rebase per the shared-worktree-commits memory; another agent did `git reset --hard HEAD~N` past my commit + re-committed their own work without cherry-picking mine.

Re-committing with the same content + same commit subject. New SHA. Audit anchor at the tail of [`systacean-13.md`](systacean-13.md) "2026-05-21 — orphaned by upstream reset; re-committing".

Procedural takeaway: pre-commit `git diff --staged --stat` + post-commit `git show --stat HEAD` audits caught nothing because the orphaning happened EXTERNALLY (another agent's reset), not via my own commit shape. The mitigation is `git log --oneline -3` BEFORE staging the next commit (so I see whether my prior commit is still in HEAD) — but for a sequential session pickup that's awkward. Better: check `git reflog` for unexpected resets on resume. Update the [shared-worktree memory](../../../../.claude/projects/-Users-fiorix-dev-github-com-fiorix-chan/memory/feedback_shared_worktree_commits.md) if this pattern recurs.

## 2026-05-21 — -11 implementation landing

@@Alex's B.1 reply transcribed (identity = `Developer ID Application: Alexandre Fiori (W73XV5CK3N)`, providerShortName OUT). Pre-authorized to land the JSON rotation per the transcription.

### Changes

* `desktop/src-tauri/tauri.conf.json` — single field added under `bundle.macOS`: `signingIdentity = "Developer ID Application: Alexandre Fiori (W73XV5CK3N)"`. Identity NAME is public per the ci-3 brief.
* `desktop/CLAUDE.md` — new "## Apple Developer ID signing" section inserted between "Bundled chan sidecar" and "Auto-upgrade signing". Covers: identity-field pointer, secrets reference, local-vs-CI behaviour split (sign-prereqs failure mode is expected without cert), rotation procedure with `populate-apple-secrets.sh` + `security delete-certificate` snippets. No bridge release needed for Developer ID cert rotation (contrast with minisign updater key).

### Validation

* `python3 -m json.tool` — JSON parses clean.
* `cargo check --offline` on chan-desktop — green in 2.22s. tauri-build's config-schema validation accepts the field.
* No release identity VALUES land in the repo (only the NAME, which is public).

### Commit shape

6 files. Same per-file `git add` discipline as always; pre/post-commit audits.

Push held per the v0.11.2 / Round-2 policy.

## 2026-05-21 — both -13 (re-applied) + -11 committed

* `2fb3f12` — `desktop/Makefile: notarytool keychain-profile path for local make app-notarized (systacean-13)` (re-application of the orphaned `01f10d3`).
* `b12b787` — `chan-desktop: pin Developer ID Application signing identity (systacean-11)`.

Both pre/post-commit audits clean. The `event-systacean-alex.md` permission-ask + transcribed approval rode @@Architect's `01b103d` v0.11.2 mini-wave commit, so it was already tracked at the time -11 staged.

Round-2 Wave-1 systacean queue update:

* `-11` ✓ committed (`b12b787`)
* `-12` (tauri-plugin-updater verify) — resuming desk-work; runtime permission event before launching Chan.app
* `-13` ✓ re-committed (`2fb3f12`)
* `-13` smoke test on @@Alex's plate per `round-2-open-questions.md` B.3

`-11` unblocks `ci-8`'s real-keys dry-run on the JSON side (the workflow's `make app-notarized` step now signs against the pinned `signingIdentity`). The other gate is @@Alex populating the six GH Secrets via `populate-apple-secrets.sh` per `round-2-open-questions.md` B.2.

## 2026-05-21 — fresh session; bootstrap complete; standing by for v0.11.2 cut

Resumed fresh @@Systacean session. Bootstrap walk complete: contact card + skills, process.md, request.md, my journal, task files (`-11` / `-12` / `-13` plus the prior history), inbound + outbound event channels, [`../architect/commit-plan-v0.11.2.md`](../architect/commit-plan-v0.11.2.md), `git status`, `git log`, tag list.

### Surface sweep ("anything new")

* **HEAD**: `08eb845` (docs: fullstack-b-21 commit poke + ci-8 dry-run #4 unblock signal). My prior `b12b787` (-11) + `2fb3f12` (-13) confirmed in HEAD.
* **Tags**: `chan-v0.11.99-dryrun.4` is the most recent; run 26216314316 produced a signed + notarized DMG on GH Release (~20m11s wall-clock).
* **Inbound** ([`event-architect-systacean.md`](../alex/event-architect-systacean.md)): cut-it signal for `chan-v0.11.2` appended 2026-05-21 (working-tree, uncommitted). Lists the gate-clearance recap (ci-8 dryrun.4 green + @@WebtestB dev-Mac Gatekeeper verify + @@Alex accept-the-partial decision) + the tag-cut sequence carrying from the commit-plan.
* **Permissions** ([`event-systacean-alex.md`](../alex/event-systacean-alex.md)): `-12` runtime-permission ask fired 2026-05-21 for the macOS dry-run launch; no approval recorded.
* **External state**: ci-8 GH Actions run 26216314316 cited in the cut-it signal; not independently re-fetched (webtest-b verification + architect's verbatim claim is the gating evidence per the cross-agent staleness rule).

### Queue

* `systacean-11` ✓ committed (`b12b787`).
* `systacean-13` ✓ re-committed (`2fb3f12`).
* `systacean-12` (tauri-plugin-updater verify) — parked on @@Alex's runtime-permission approval; resumes post-v0.11.2 cut.
* `systacean-3` (re-activated for v0.11.2) — pre-authorized per the cut-it signal; holding for @@Alex's "go" before executing the tag-cut sequence.

### Next action

Cut `chan-v0.11.2` against current HEAD per [`commit-plan-v0.11.2.md`](../architect/commit-plan-v0.11.2.md) §"Tag-cut sequence": pre-push gate → version bump (5 manifests) → release commit `chan v0.11.2` → annotated tag with the pre-written body → `git push origin main --follow-tags`. High blast-radius (tag fires signed `release-desktop.yml` against real Apple Developer ID keys + creates a GitHub Release); confirming with @@Alex before executing.

## 2026-05-21 — chan v0.11.2 cut + pushed

@@Architect transcribed @@Alex's in-session "go, cut v0.11.2" approval to [`event-architect-systacean.md`](../alex/event-architect-systacean.md). Executed the full sequence.

### Pre-push gate (workspace-wide)

* `cargo fmt --check` → OK.
* `cargo clippy --all-targets -- -D warnings` → clean.
* `cargo test` (workspace) → all green, no failures.
* `RUSTFLAGS=-D warnings cargo build --no-default-features` → green (the `-s-8` follow-up `c1e9c41` still unblocks this case).
* `cd web && npm run check` → 0 errors, 0 warnings (3983 files).
* `cd web && npm run build` → green; existing chunk-size + ineffective-dynamic-import warnings preserved from v0.11.1 baseline.
* `cd web && npm test -- --run` → 586/586 passed (58 files).

### Version bump (5 manifests)

`0.11.1` → `0.11.2` across `Cargo.toml`, `Cargo.lock` (refreshed via `cargo build -p chan`), `desktop/src-tauri/tauri.conf.json`, `web/package.json`, `web/package-lock.json`. Runtime confirmation: `./target/debug/chan --version` → `chan 0.11.2`.

### Release commit `60901c1` — `chan v0.11.2`

5 files, +15 / -15. Pre-commit `git diff --staged --stat` audit clean; `git add` per-path discipline (Cargo.toml, Cargo.lock, desktop/src-tauri/tauri.conf.json, web/package.json, web/package-lock.json) skipped the ~30 other modified files belonging to other agents' uncommitted work. Post-commit `git show --stat HEAD` audit clean.

### Annotated tag

`chan-v0.11.2` at `bc14828`. Body from [`commit-plan-v0.11.2.md`](../architect/commit-plan-v0.11.2.md) §"Tag draft (v0.11.2)" written verbatim via `/tmp/chan-v0.11.2-tag-body.txt` (Apple's blessed `git tag -a -F <tempfile>` pattern; same v0.11.1 lesson on heredoc + single-quote escaping).

### Push

```
git push origin main --follow-tags
```

Result:

```
   7b5a126..60901c1  main -> main
 * [new tag]         chan-v0.11.2 -> chan-v0.11.2
```

Remote `ls-remote` confirms:

```
60901c164e34bc5aad76bc721814bb06dcb75f72  refs/heads/main
bc14828d2ee50ebda9e93ee3b80a47c0c9a80d0c  refs/tags/chan-v0.11.2
```

### Tag-triggered workflow

`release-desktop.yml` fired automatically: run `26221281508` in progress as of push. Notary turnaround expected ~10-11 min per dryrun.4. GH Release artifact appears on https://github.com/fiorix/chan/releases/tag/chan-v0.11.2 when the workflow completes.

### Audit anchor

systacean-3 task acceptance criteria all met for the v0.11.2 cut; tail of [`systacean-3.md`](systacean-3.md) carries the v0.11.1 audit anchor + this v0.11.2 entry mirrors the same shape.

### Next

Architect notification poke fires next on [`event-systacean-architect.md`](../alex/event-systacean-architect.md). Standing by for the workflow run to complete + post-tag verification routing.
