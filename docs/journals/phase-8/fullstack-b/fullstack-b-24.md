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
