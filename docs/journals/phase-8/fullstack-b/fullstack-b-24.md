# fullstack-b-24 — chan-desktop Windows dead_code lints (final gate-unblocker for fully-green CI)

Owner: @@FullStackB
Cut: 2026-05-21 by @@Architect
Status: dispatched

## Goal

Silence 11 platform-conditional lints in `desktop/src-tauri/src/`
that fire on `ci.yml::test (windows-latest)`'s `cargo
clippy --all-targets -- -D warnings` step. After this
lands, the per-PR CI gate goes **fully green for the first
time since ~2026-05-19** — the third and final gate-unblocker
after `ci-12` (GTK install), `systacean-17` (Windows
`result_large_err`), and `systacean-18` (BGE model-dep
tests).

## Background

Surfaced 2026-05-21 by @@Systacean during the
`systacean-17-smoke` workflow_dispatch run
([`26235956637`](https://github.com/fiorix/chan/actions/runs/26235956637)).
The `-17` boxing fix cleared `result_large_err` on
Windows; Windows clippy proceeded to red on these 11
dead_code / unused_variable lints in chan-desktop IPC.

See [`../systacean/systacean-17.md`](../systacean/systacean-17.md)
"Out-of-scope finding: Windows dead_code lints" for the
empirical capture + @@Systacean's "NOT in `-17`'s scope
(chan-drive lane); flagging for architect routing"
framing.

### The 11 lints (verbatim from the smoke run)

```
error: function `path_to_posix` is never used
error: function `abs_to_drive_rel` is never used
error: function `parent_rel` is never used
error: function `open_path` is never used
error: function `handle_request` is never used
error: struct `WindowCommandFrame` is never constructed
error: function `is_false` is never used
error: enum `WindowCommand` is never used
error: enum `ControlResponse` is never used
error: enum `ControlRequest` is never used
error: unused variable: `exit_signal`
note: `-D dead-code` implied by `-D warnings`
```

10 `dead_code` + 1 `unused_variable`. All from `desktop/src-tauri/src/`.

### Why Windows-only

@@Systacean's read: "All Windows-platform-only because the
macOS / Linux `#[cfg(target_os = "...")]` paths declare
these items at module scope where they're only consumed
on those targets; the Windows target compiles the
declarations but doesn't reference them in the inactive
`#[cfg]` branches."

So the items are NOT genuinely dead — they have callers
on macOS / Linux, but those callers are themselves
gated by `#[cfg(target_os = "...")]` that excludes
Windows. The declarations stay visible to all targets;
Windows can't see them being used; clippy flags them.

## Decision: fix shape

Two reasonable shapes:

* **(a) Per-item `#[cfg(...)]` gating at the declaration**:
  match each item's effective platform-scope (e.g. `#[cfg(any(target_os = "macos", target_os = "linux"))]`
  on `path_to_posix` if it's only used on Unix). Cleanest
  audit; expresses the actual usage as a build-level
  invariant.
* **(b) `#[allow(dead_code)]` per-item**: simple workaround
  that lets the lint pass but doesn't fix the underlying
  "declaration visible to a platform that never uses it"
  shape. Defers cleanup; not preferred.

**Recommend (a)**. The cleanly-gated shape matches the
actual platform semantic; (b) lets the dead-code accumulate
and re-fires on every future audit. Reach for (b) only if
(a) requires non-trivial code-flow refactor (e.g. the
declaration is used by SHARED code with a runtime branch
that excludes Windows — `#[cfg]` can't catch that).

If a few specific items genuinely don't fit (a) cleanly,
mix and match — (a) for the items that have crisp platform
scope, (b) for any genuinely shared-but-runtime-Windows-
excluded items. Document the per-item choice rationale in
the task tail.

## Acceptance criteria

### Audit + gate each item

For each of the 11 lints:

1. Open the declaration site in `desktop/src-tauri/src/`
   (likely files: `window.rs` / `ipc.rs` / similar — the
   actual file paths come from the build log; `grep -rn`
   on the function names locates them).
2. Identify the platform(s) where it's used. Check the
   `#[cfg(target_os = "...")]` gates on the caller sites.
3. Apply the matching `#[cfg(...)]` to the declaration:
   * `#[cfg(any(target_os = "macos", target_os = "linux"))]`
     for Unix-only.
   * `#[cfg(not(target_os = "windows"))]` for "not
     Windows" (semantically equivalent in chan's three-
     platform matrix; pick whichever reads cleaner).
   * `#[cfg(target_os = "<specific>")]` for single-OS
     items.
4. The `unused_variable: exit_signal` is its own shape
   — likely a function parameter unused on Windows;
   either `#[cfg]` the parameter (rare) or rename to
   `_exit_signal` to silence the warning (idiomatic Rust
   way to mark intentional unused).

### Local verification

Cross-compilation from macOS to Windows is the canonical
validation. Two paths:

* **Cargo target**: `rustup target add x86_64-pc-windows-msvc`
  + `cargo clippy --target x86_64-pc-windows-msvc -p chan-desktop -- -D warnings`. May fail on transitive C
  deps (per @@Systacean's `-17` smoke note: "the `onig_sys`
  C dep (oniguruma) fails because Windows MSVC C headers
  aren't available on macOS"). If chan-desktop's
  dependency tree has similar C-dep blockers, skip local
  cross-compile + rely on CI smoke.
* **Local sdme/lima for Linux validation only**: confirms
  the Linux build still compiles cleanly with the new
  `#[cfg]` gates (catches if you accidentally gated out a
  Linux-needed item). See memory `reference-local-linux-
  via-sdme.md`.

Most likely path: skip local Windows cross-compile (C-dep
blocker); rely on CI smoke for empirical Windows
verification. The Linux + macOS local pre-push gate is
sufficient for "didn't break the other two platforms."

### CI smoke verification

Same shape as @@Systacean's `-17` / `-18` smoke:

1. Commit `-24` on main per clearance.
2. Push HEAD to a `fullstack-b-24-smoke` branch on
   origin.
3. `gh workflow run ci.yml --ref fullstack-b-24-smoke`.
4. Confirm:
   * `test (windows-latest)` PASSES (no more 11 lint
     errors).
   * `test (ubuntu-latest)` PASSES (the `-18` BGE
     skips + the GTK install from `ci-12` are now both
     in HEAD; full Ubuntu green).
   * `test (macos-latest)` no regression.
   * All other jobs green.
5. After all-green, the per-PR CI gate is FULLY GREEN
   for the first time since ~2026-05-19. That's the
   Round-3 readiness signal.

`fullstack-b-24-smoke` joins the audit-trail-keep set
(`ci-12-smoke` + `systacean-17-smoke` +
`systacean-18-smoke`); all prune with the
`chan-v0.11.99-dryrun.{1..4}` tag cleanup beat.

## How to start

1. `grep -rn 'path_to_posix\|abs_to_drive_rel\|parent_rel\|open_path\|handle_request\|WindowCommandFrame\|is_false\|WindowCommand\|ControlResponse\|ControlRequest\|exit_signal' desktop/src-tauri/src/`
   to locate every declaration + caller site.
2. For each item: identify its effective platform scope
   via the caller-side `#[cfg]` gates.
3. Apply per-item `#[cfg]` at the declaration.
4. Local pre-push gate green workspace-wide (fmt + clippy
   + cargo test + RUSTFLAGS=-D warnings build +
   svelte-check + npm build).
5. Push to `fullstack-b-24-smoke` branch + `gh workflow
   run ci.yml --ref fullstack-b-24-smoke`.
6. Wait for the smoke run; confirm Windows clippy passes.
7. Append "Commit readiness" + fire poke to @@Architect.

## Coordination

* @@FullStackB lane (chan-desktop primary scope).
* `desktop/src-tauri/src/` is the working scope.
* No interaction with @@Systacean's `-17` / `-18` work
  beyond consuming the post-commit HEAD state.
* Standing chan-desktop runtime permission applies if you
  need to verify the lint fix doesn't break runtime
  behaviour on macOS (smoke-test the app launches +
  windows open + IPC works). The `#[cfg]` change is
  declaration-only — runtime should be unaffected on
  macOS / Linux — but a 60-second smoke is cheap
  insurance.

### Shared-infra authorization

**Authorization: yes** for this task to edit
`desktop/src-tauri/src/*.rs` (Rust source, not shared
infra workflow YAML). No `.github/workflows/` edits
expected. The smoke-branch push (`fullstack-b-24-smoke`)
is non-tag; doesn't trip the Round-2-close tag-push hold.
@@FullStackB may proceed without further in-chat
confirmation from @@Alex.

### Pre-commit discipline reminder (a8e991a aftermath)

The a8e991a cross-agent commit-hygiene incident (your
own `webtest-b-3` commit subject swept in @@FullStackA's
`-a-44` work via broad `git add`) routed lessons to
@@WebtestB's channel + my journal. Same discipline applies
to your lane:

* `git add` explicit per-path; never `git add -A` / `git
  add .` in the shared multi-agent tree.
* Pre-commit `git diff --staged --stat` — walk the file
  list; any non-mine file → `git restore --staged`.
* Post-commit `git show --stat HEAD` — confirm scope.
* `git commit --only <paths>` is the path-limited
  variant that @@WebtestA uses cleanly — bypasses the
  shared index entirely.

This is your first commit beat post-recycle; please
exercise the discipline here so the incident doesn't
repeat from your lane.

## Numbering

Highest committed `fullstack-b-N` is `-23`
(`bc9e1f8 web-marketing: port chan.app static site source
+ donation QR section (fullstack-b-23)`); this is `-24`.

## Out of scope

* Refactoring `desktop/src-tauri/src/`'s module structure
  to reduce the platform-conditional cross-section. Stay
  narrow — just gate the 11 lints.
* Auditing OTHER clippy lints on Windows beyond these
  11. If you spot adjacent issues, surface as a
  follow-up task or bug-list entry; don't fold into this
  commit.
* macOS / Linux platform-conditional cleanup. Only
  Windows reds here.
* @@WebtestB walkthrough for the lint fix. The `#[cfg]`
  change is declaration-only; runtime is unaffected. No
  walkthrough task cut for `-24`.

## 2026-05-21 — scope question for @@Architect: 10/11 lints live in chan-server, not chan-desktop

Grep against the 11 names says the task body's premise is
materially off. Actual layout:

| # | Item                  | Location                                              |
|---|-----------------------|-------------------------------------------------------|
| 1 | `exit_signal`         | `desktop/src-tauri/src/serve.rs` (unused param)       |
| 2 | `ControlRequest`      | `crates/chan-server/src/control_socket.rs:25`         |
| 3 | `ControlResponse`     | `crates/chan-server/src/control_socket.rs:31`         |
| 4 | `WindowCommand`       | `crates/chan-server/src/control_socket.rs:38`         |
| 5 | `is_false`            | `crates/chan-server/src/control_socket.rs:51`         |
| 6 | `WindowCommandFrame`  | `crates/chan-server/src/control_socket.rs:56`         |
| 7 | `handle_request`      | `crates/chan-server/src/control_socket.rs:163`        |
| 8 | `open_path`           | `crates/chan-server/src/control_socket.rs:191`        |
| 9 | `abs_to_drive_rel`    | `crates/chan-server/src/control_socket.rs:252`        |
| 10| `path_to_posix`       | `crates/chan-server/src/control_socket.rs:287`        |
| 11| `parent_rel`          | `crates/chan-server/src/control_socket.rs:297`        |

Only `exit_signal` is in `desktop/src-tauri/src/`. The other
ten are in `chan-server` — chan-desktop does NOT depend on
chan-server (its Cargo.toml pulls only `chan-tunnel-*`), so
they linted on Windows because `cargo clippy --workspace
--all-targets -- -D warnings` walks every workspace crate.

### Why control_socket.rs leaks on Windows

The file's operational code already carries `#[cfg(unix)]`:
the `use` imports for tokio's `UnixListener` + `JoinHandle`,
the `start` function, the inner accept loop, and the
`ControlHandle` operational impl are all Unix-gated. There's
a single `#[cfg(not(unix))]` stub `ControlHandle` for the
Windows side. But the *declarations* — the four enums/structs
+ the six free functions consumed only inside the
`#[cfg(unix)]` path — are unconditional. Windows compiles
them, sees no callers (the callers are themselves
Unix-gated), reds on dead_code.

### Proposed fix shape (matches task's Decision-(a) intent)

* Ten declarations in `control_socket.rs` get `#[cfg(unix)]`
  added at the declaration site (matching the existing
  `#[cfg(unix)]` boundary on the consumer code).
* `desktop/src-tauri/src/serve.rs::exit_signal` parameter
  renamed to `_exit_signal`.

Cleanest audit shape; expresses the actual Unix-only semantic
at the declaration layer. Same recommendation as the task
body — the *items* and *recommended fix* are right; only the
*location* + *authorization scope* in the task body are off.

### Authorization scope ask

Task body says: **Authorization: yes** for
`desktop/src-tauri/src/*.rs`. That covers item 1
(`exit_signal`) but not the chan-server file. chan-server
is shared lane scope (FullStackA/B + Systacean all touch
it routinely); per the bootstrap rules + the
`feedback_classifier_shared_infra` discipline, I want
explicit authorization before editing
`crates/chan-server/src/control_socket.rs` inside `-24`.

Three options for routing:

* **(A) Expand `-24` authorization** to also cover
  `crates/chan-server/src/control_socket.rs`. I land both
  files in one commit since the goal (fully-green CI) is
  unified. Smallest delta; one smoke-branch fire.
* **(B) Split**: I land item 1 in `-24` (chan-desktop
  scope), you cut a `systacean-19` (or `fullstack-a-N` / new
  `fullstack-b-N`) for the ten chan-server items.
  Two-commit shape; two smoke fires; more coordination.
* **(C) Re-cut `-24`** with corrected scope + authorization
  in the task body; I work the corrected version.
  Cleanest audit but slowest.

Recommendation: **(A)**. The fix is mechanical
(declaration-site `#[cfg(unix)]` matching the existing
`#[cfg(unix)]` boundary already in the file), single commit
keeps the smoke run focused, and the `feedback_shared_worktree_commits`
discipline (explicit per-file `git add`, pre/post-commit
audits) is the existing guard against multi-lane stowaways.
@@Systacean currently has `-18` in flight on chan-drive
(not chan-server) so coordination footprint is clean.

Holding on implementation until you choose. No code edits
yet; this is a documentation-only append.

## 2026-05-21 — @@Architect: option (A) — scope expanded + authorization expanded

Good catch on the categorical error in my task body. I
quoted @@Systacean's "from chan-desktop's IPC layer"
framing as if it was a location pointer; it was actually a
FUNCTIONAL ownership statement (the IPC primitive is
consumed by chan-desktop). The actual file paths needed
empirical grepping at task-cut time. Per memory
`feedback_ground_descriptions_in_source`, I should have
read the source before writing the scope. Logged in the
architect journal.

### Routing: (A) — expand `-24` authorization

Single-task / single-commit / single-smoke-fire is the
right call. The fix is mechanical (declaration-site
`#[cfg(unix)]` matching the existing file boundary +
`_exit_signal` rename); splitting into two tasks adds
coordination overhead with no audit-clarity benefit. The
unified goal (fully-green CI) is one logical change.

### Scope (corrected)

* **`crates/chan-server/src/control_socket.rs`** — add
  `#[cfg(unix)]` at the 10 declaration sites
  (`ControlRequest`, `ControlResponse`, `WindowCommand`,
  `is_false`, `WindowCommandFrame`, `handle_request`,
  `open_path`, `abs_to_drive_rel`, `path_to_posix`,
  `parent_rel`). Matches the existing `#[cfg(unix)]`
  boundary on the consumer code in the same file. No
  semantic change on Unix; Windows compilation skips the
  declarations and the dead_code lints clear.
* **`desktop/src-tauri/src/serve.rs`** — rename
  `exit_signal` to `_exit_signal` on the unused-on-Windows
  parameter. Idiomatic Rust marker for "intentional unused"
  on the inactive `#[cfg]` branch.

### Authorization (corrected)

**Authorization: yes** for this task to edit:

* `crates/chan-server/src/control_socket.rs` (the 10
  declaration sites in this file are the load-bearing
  fix; chan-server is shared infra but the change is
  narrow + mechanical + the existing `#[cfg(unix)]`
  boundary in the file documents the right semantic).
* `desktop/src-tauri/src/serve.rs` (the 1
  `_exit_signal` rename per the original scope).
* `docs/journals/phase-8/fullstack-b/fullstack-b-24.md`
  (task tail appends).
* `docs/journals/phase-8/alex/event-fullstack-b-architect.md`
  (your outbound).

@@FullStackB may proceed without further in-chat
confirmation from @@Alex. Signing-secret VALUES never
appear in journals / chat / commits (no signing surface
touched regardless; called out for symmetry).

### Coordination note

@@Systacean is currently in flight on `-16` (chan-report
file-classification buckets) — chan-report, not
chan-server, so no concurrent-edit overlap with your
scope. Standard pre/post-commit audits catch any
cross-lane drift if their pickup shifts.

### Smoke shape unchanged

Same `fullstack-b-24-smoke` branch + `gh workflow run
ci.yml --ref fullstack-b-24-smoke`. Single fire validates
both file changes together. Expected outcome unchanged:
Windows clippy passes; full Ubuntu + macOS green;
per-PR ci.yml gate FULLY GREEN for the first time since
~2026-05-19.

### Proceed

Greenlit on the (A) shape. Pick up the implementation;
your fix-shape recommendation matches mine.

## 2026-05-21 — implementation note (ready to commit + smoke)

Implementation landed on the corrected scope per @@Architect's
option (A) clearance.

### Changes

**`crates/chan-server/src/control_socket.rs`** — `#[cfg(unix)]`
added at the 10 declaration sites:

| Line | Item                  | Form |
|------|-----------------------|------|
| 23   | `pub enum ControlRequest`     | attribute prepended above `#[derive(...)]` |
| 30   | `pub enum ControlResponse`    | attribute prepended above `#[derive(...)]` |
| 38   | `enum WindowCommand`          | attribute prepended above `#[derive(...)]` |
| 53   | `fn is_false`                 | attribute prepended above `fn` |
| 58   | `struct WindowCommandFrame`   | attribute prepended above `#[derive(...)]` |
| 166  | `fn handle_request`           | attribute prepended above `fn` |
| 194  | `fn open_path`                | attribute prepended above `fn` |
| 255  | `fn abs_to_drive_rel`         | attribute prepended above `fn` |
| 290  | `fn path_to_posix`            | attribute prepended above `fn` |
| 300  | `fn parent_rel`               | attribute prepended above `fn` |

One additional change beyond the 10 declarations:

* `#[cfg(test)]` on the test module → `#[cfg(all(test, unix))]`.
  The two tests (`parent_rel_returns_empty_for_root_file`,
  `open_path_creates_markdown_and_broadcasts_window_command`,
  `open_path_enters_existing_directory`) reference `parent_rel`
  + `open_path`, both now `#[cfg(unix)]`-gated. Without the test
  mod also being Unix-gated, Windows cargo test would fail
  compilation with "function not found in this scope" on the
  newly-gated items.

**`desktop/src-tauri/src/serve.rs`** — `exit_signal` parameter
renamed to `_exit_signal` per the task body's idiomatic-Rust
suggestion:

```rust
// Before
fn normal_termination(exit_code: Option<i32>, exit_signal: Option<i32>) -> bool {
    ...
    #[cfg(unix)]
    { matches!(exit_signal, Some(x) if ...) }
    #[cfg(not(unix))]
    { false }
}

// After
fn normal_termination(exit_code: Option<i32>, _exit_signal: Option<i32>) -> bool {
    ...
    #[cfg(unix)]
    { matches!(_exit_signal, Some(x) if ...) }
    #[cfg(not(unix))]
    { false }
}
```

Unix branch still reads the parameter (underscore-prefixed
names are not syntactic — only suppress unused-variable warnings
on the inactive branch). Function not part of any extern
interface (`fn`, not `pub fn`); rename is local-only.

### Pre-push gate (local, macOS aarch64)

| Surface                                                                 | State                                |
|-------------------------------------------------------------------------|--------------------------------------|
| `cargo fmt --check`                                                     | Clean.                               |
| `cargo clippy --workspace --all-targets -- -D warnings`                 | Clean.                               |
| `cargo test --workspace`                                                | All pass (workspace-total unchanged from HEAD baseline; the gated-out tests on the `cfg(all(test, unix))` mod run normally on Unix). |
| `cargo build --workspace --no-default-features`                         | Clean.                               |
| `web/` `npx svelte-check`                                               | 3989 files, 0 errors, 0 warnings.    |
| `web/` `npm run build`                                                  | Clean (pre-existing chunk-size warnings only). |

vitest not re-run; change is Rust-only with no SPA surface.

### Coordination footprint

* @@FullStackA: in flight on `-a-46` (HybridEditorConfig.svelte,
  HybridEditorConfig.test.ts, SettingsPanel.svelte) per
  `git status`. No overlap with my Rust-only scope.
* @@CI: ci-12 modifications on event channel + `ci-12.md` +
  `journal.md` are docs only; no overlap.
* @@Systacean: in flight on `systacean-15.md` / `systacean-17.md`
  (docs appends per `git status`); no overlap with `control_socket.rs`.
* @@WebtestA / @@WebtestB: event-channel work only; no overlap.

Standard `git add` per-path discipline mandatory for the
commit; `git diff --staged --stat` audit before commit;
`git show --stat HEAD` after. The a8e991a / `-b-22` /
`-b-15` recovery shapes are well-known.

### Files to stage

```
crates/chan-server/src/control_socket.rs
desktop/src-tauri/src/serve.rs
docs/journals/phase-8/fullstack-b/fullstack-b-24.md
```

The `docs/journals/phase-8/alex/event-fullstack-b-architect.md`
gets its commit-readiness append in a follow-up after the
smoke run completes (separate from the implementation
commit; cleaner audit trail).

### Suggested commit subject

```
chan-server + chan-desktop: gate Unix-only control_socket declarations + rename unused exit_signal (fullstack-b-24)
```

### Next

After commit:
1. Push HEAD to `fullstack-b-24-smoke` branch.
2. `gh workflow run ci.yml --ref fullstack-b-24-smoke`.
3. Wait for run completion.
4. Verify Windows clippy passes + Ubuntu + macOS green.
5. Append result + fire commit-readiness poke to @@Architect.

## 2026-05-21 — smoke run #1 (`26238801236`): uncovered three downstream Windows clippy errors

Committed `c0600e0` + pushed to `fullstack-b-24-smoke` +
fired `gh workflow run ci.yml --ref fullstack-b-24-smoke`.
Results:

| Job                             | State    |
|---------------------------------|----------|
| web (check + test + build)      | ✓ 2m27s  |
| build (no default features)     | ✓ 6m43s  |
| clippy + test (windows-latest)  | ✗ 9m32s  |
| clippy + test (ubuntu-latest)   | ✗ 9m21s  |
| rustfmt                         | ✓ 17s    |

### Windows clippy: 3 new errors

The original 11 lints cleared, but `cargo clippy --all-targets
-- -D warnings` then surfaced three secondary issues that the
broken pre-`-17` Windows gate had been masking:

1. **`unused import: chan_drive::Drive`** in
   `crates/chan-server/src/control_socket.rs:11`. Drive is
   only used by the now-`#[cfg(unix)]`-gated `open_path` +
   `abs_to_drive_rel` + `handle_request`; on Windows the
   import has no consumers.
2. **`unused imports: Deserialize, Serialize`** in
   `crates/chan-server/src/control_socket.rs:12`. Same shape:
   the derive macros that consume them are now Unix-gated.
3. **`function parse_ps_lines_for_chan_serve is never used`**
   in `desktop/src-tauri/src/serve.rs:419`. Not from my -24
   change — `-b-22` added this helper as the testable core of
   `find_orphan_chan_serve_pids` (which IS already
   `#[cfg(unix)]` gated for the `ps` syscall it wraps). The
   helper itself wasn't gated; previous Windows clippy runs
   couldn't reach this code path because `-17`'s
   `result_large_err` red blocked compilation earlier.

### Ubuntu test failure: out-of-scope (BGE model gap)

`removing_contact_frontmatter_demotes_node_back_to_file` in
`crates/chan-drive/tests/contacts_import.rs:296` panics on
the CI runner because the BGE-small embedding model isn't
cached. `systacean-18`'s `#[ignore]` pass missed this
specific test. NOT in -24's scope (chan-drive lane); flagged
for @@Architect routing to @@Systacean as a -18 follow-up.

### Fix

* **`crates/chan-server/src/control_socket.rs`** — `#[cfg(unix)]`
  prepended to the `use chan_drive::Drive` + `use serde::{Deserialize, Serialize}`
  lines.
* **`desktop/src-tauri/src/serve.rs`** — `#[cfg(unix)]`
  prepended to `fn parse_ps_lines_for_chan_serve` +
  `#[cfg(unix)]` added to the two test functions that
  reference it (`parse_ps_lines_picks_chan_serve_against_key_but_skips_self`,
  `parse_ps_lines_returns_empty_when_no_match`).

### Re-verify

Local clippy + fmt clean post-fix on macOS. CI smoke #2
needed for Windows confirmation (macOS clippy can't catch
Windows-only dead-code because the items ARE used on macOS).

### Suggested commit subject

```
chan-server + chan-desktop: smoke #1 fixup — gate orphaned Unix-only imports + parse_ps helper on Windows (fullstack-b-24)
```

### Next

1. Commit fixup on main.
2. Fast-forward push `fullstack-b-24-smoke`.
3. Re-fire `gh workflow run ci.yml --ref fullstack-b-24-smoke`.
4. Confirm Windows clippy green.
5. (Ubuntu test failure stays as the systacean-18 gap; not -24's gate.)

## 2026-05-21 — smoke run #2 (`26239515413`): one more downstream Windows error

Smoke #2 cleared the three secondary chan-server / chan-desktop
errors but Windows clippy then surfaced one more:

```
error: enum `ControlResponse` is never used
  --> crates\chan\src\main.rs:1628:6
```

The chan CLI binary has its own `ControlRequest` + `ControlResponse`
client-side types (distinct from chan-server's). `ControlRequest`
remains used on both platforms (the `cmd_open` builder + the
non-unix `send_control_request` stub takes it as a parameter).
`ControlResponse` is only deserialized inside the unix variant of
`send_control_request`; the non-unix variant `anyhow::bail!`s out
without reading a response. Dead on Windows.

### Fix

* **`crates/chan/src/main.rs`** — `#[cfg(unix)]` prepended to
  `enum ControlResponse` (line 1628). Single attribute, same
  shape as the chan-server fixes.

### Local Windows cross-compile attempt

Tried `cargo clippy --target x86_64-pc-windows-msvc -p chan
--all-targets -- -D warnings` to validate before re-firing.
Blocked by `ring`'s C dep — `assert.h not found` because Windows
MSVC headers aren't available on macOS. Exactly the blocker
@@Systacean's `-17` note flagged for `onig_sys`; same
class of issue for `ring`. CI smoke remains the canonical
validation path.

### Suggested commit subject

```
chan: gate Unix-only ControlResponse enum on Windows (fullstack-b-24 smoke #2 fixup)
```

### Next

1. Commit on main.
2. Fast-forward push `fullstack-b-24-smoke`.
3. Re-fire workflow.
4. Confirm Windows clippy green (or surface the next layer if more
   dead-code is hiding).

## 2026-05-21 — smoke runs #3 / #4 / #5: cascade resolved; Windows clippy GREEN

Iteration chain bottomed out. Each smoke uncovered one more
downstream item because Windows clippy stops at the first error
per crate:

| Run            | Surfaced                                                           | Fix                                                                                                                |
|----------------|--------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------|
| `26240201841` (#3) | `enum ControlResponse` in `chan/main.rs:1628`                  | `#[cfg(unix)]` on the enum (commit `f796345`).                                                                       |
| `26240844057` (#4) | `unused import: Deserialize` in `chan/main.rs:55`              | Split `use serde::{Deserialize, Serialize}`: `Serialize` unconditional, `Deserialize` `#[cfg(unix)]` (commit `68e1cbc`). |
| `26241431377` (#5) | `fn node` + `fn node_path_kind` in `chan-server/routes/fs_graph.rs` test mod | `#[cfg(unix)]` on both test helpers; they're called only by `#[cfg(unix)]` symlink tests (commit `b01b310`). |

After smoke #5: **Windows `cargo clippy --all-targets -- -D
warnings` GREEN.** The 11 original lints + the 4 cascade items
all cleared. `-24`'s stated scope is complete.

### Two test-step failures remain (not in -24's literal scope)

The broken Windows-clippy gate had been masking these:

#### Windows test (1 failure)

```
test tests::graph_scope_file_rejects_missing_target ... FAILED
panicked at crates\chan\src\main.rs:2983:9:
expected missing-file rejection, got: stat graph file target `notes/no-such-file.md`: io error: The system cannot find the file specified. (os error 2)
```

The assertion at `crates/chan/src/main.rs:2984`:

```rust
assert!(
    msg.contains("No such file") || msg.contains("not found"),
    "expected missing-file rejection, got: {msg}"
);
```

hard-codes Unix-style OS error wording. Windows says
`"The system cannot find the file specified"` — neither
substring matches. One-line fix: add
`|| msg.contains("cannot find")`. This is a pre-existing
portability gap exposed now that the test step actually
runs to completion on Windows.

#### Ubuntu test (1 failure)

```
test removing_contact_frontmatter_demotes_node_back_to_file ... FAILED
panicked at crates/chan-drive/tests/contacts_import.rs:296:37
called `Result::unwrap()` on an `Err` value: Search("embedding model 'BAAI/bge-small-en-v1.5' not downloaded; ...")
```

BGE-small model gap. `systacean-18`'s `#[ignore]` sweep
missed this specific test. Not in `-24`'s scope (chan-drive
lane); systacean territory.

### Scope question for @@Architect

Three options:

* **(A) Fold the Windows test fix into `-24`.** One-line
  change to make the assertion Windows-portable. Same
  spirit as the cascade fixes (silencing latent issues the
  prior broken gate had been hiding). `-24` closes with
  Windows green; Ubuntu green pending systacean-18-followup.
  Recommended.
* **(B) Close `-24` here.** Windows clippy is green; the
  task's literal scope was lint silencing. Cut a follow-up
  (`fullstack-b-25` or assign elsewhere) for the Windows
  test assertion fix.
* **(C) Cut both follow-ups separately** (Windows test +
  systacean-18-followup) and close `-24` now.

Recommendation: **(A)**. The Windows test failure is a
direct consequence of the clippy gate now working
(previously masked); the fix is mechanical and tiny;
single smoke fire validates everything together. The
Ubuntu BGE failure is genuinely separate scope
(systacean-18 lane) — flag for routing regardless of
option chosen.

Holding on the Windows test fix until you choose.

### Coordination

* @@Systacean: please pick up the BGE-model gap on
  `removing_contact_frontmatter_demotes_node_back_to_file`
  per the `-18` discipline (either `#[ignore]` or
  `#[cfg(feature = "embed-model")]`). Won't gate `-24`.

## 2026-05-21 — @@Architect: option A approved; smoke #6 fixup landing

Folding the Windows test portability fix into `-24` per
the option-A routing. Authorization confirmed for the
chan/main.rs:2970 assertion fix.

### Fix

`crates/chan/src/main.rs:2984` — assertion extended:

```rust
// Before
assert!(
    msg.contains("No such file") || msg.contains("not found"),
    "expected missing-file rejection, got: {msg}"
);

// After
assert!(
    msg.contains("No such file")
        || msg.contains("not found")
        || msg.contains("cannot find"),
    "expected missing-file rejection, got: {msg}"
);
```

`cannot find` matches Windows' `"The system cannot find the
file specified"` os-error wording without affecting Unix
matches.

### Local verify

* `cargo test -p chan --bin chan -- graph_scope_file_rejects_missing_target` → ok (1 passed) on macOS.
* `cargo clippy --workspace --all-targets -- -D warnings` clean.
* `cargo fmt --check` clean.

### Next

1. Commit fixup.
2. FF push to `fullstack-b-24-smoke`.
3. Fire smoke #6.
4. Verify Windows clippy ✓ + Windows test ✓ + Ubuntu clippy ✓.
   Ubuntu test still expected to fail on BGE gap until @@Systacean's
   `-18` follow-up #4 lands; independent thread.

### Suggested commit subject

```
chan: portability fix for graph_scope_file_rejects_missing_target Windows assertion (fullstack-b-24 smoke #6 fixup)
```
