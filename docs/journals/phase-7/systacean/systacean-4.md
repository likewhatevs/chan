# systacean-4: chan open <dir> enters the dir

Owner: @@Systacean
Cut by: @@Architect
Date: 2026-05-18

## Goal

Small follow-up on `systacean-1`'s `chan open` CLI:
`chan open <dir>` currently opens the parent directory and
**highlights** the target dir inside the file browser, rather
than opening the browser **into** the dir's listing.
@@WebtestB found this during their Lane B pass.

Fix the behavior so `chan open ./images` lands the user
*inside* `./images/` with its contents listed.

## Relevant links

* @@WebtestB's note at
  [../webtest-b/webtest-b-1.md](../webtest-b/webtest-b-1.md)
  ("`chan open` variants" section).
* [./systacean-1.md](./systacean-1.md) for the original CLI
  contract.

## Acceptance criteria

* `chan open <dir>` (relative or absolute) opens the file
  browser surface with the contents of `<dir>` shown directly.
* If `<dir>` doesn't exist or isn't a directory, the CLI
  exits non-zero with a clear message (no change in that
  shape).
* Non-`.md` file opening (`chan open ./photo.png`) still
  opens the parent + selects the file — that's the right
  behavior for "I want to see this file in the browser
  context"; only the directory case changes.
* Adjust the open_path control-socket handler to
  differentiate the two cases in the response payload, and
  the frontend window_command handler to dispatch
  accordingly.
* Update the integration test or add a focused one covering
  the "directory enters its own listing" path.

## Hand-off

Standard, small task.

## 2026-05-18 14:55 BST - implementation ready

Implemented while `systacean-5` is blocked on the remaining
@@FullStack commits.

Changed:

* `crates/chan-server/src/control_socket.rs`
  * `open_path` now marks directory browser commands with
    `enter: true`.
  * Non-md file commands keep `select`, so they still open
    the parent directory and select the file.
  * Added `open_path_enters_existing_directory`.
* `web/src/state/store.svelte.ts`
  * `window_command` handling dispatches `enter: true` to a
    new `revealAndEnterDirectory` helper.
  * The helper expands the target directory itself so lazy
    loading reveals its contents.
* `web/src/state/store.test.ts`
  * Added coverage for directory-enter and existing non-md
    file-selection behavior.

Verification:

* `cargo fmt --check`
* `cargo test -p chan-server control_socket`
* `cd web && npm run test -- src/state/store.test.ts`

Not committed yet. Full pre-push gate still needed before
any `systacean-4` commit.

## 2026-05-18 16:00 BST — @@Architect review: APPROVED for commit (gated on @@Alex)

Clean small fix:

* Server side: `open_path` differentiates the dir-enter case with
  `enter: true` and keeps `select` semantics for non-md files.
* Frontend: `window_command` handler dispatches `enter: true` to a
  new `revealAndEnterDirectory` helper that expands the target dir
  itself so lazy loading reveals its contents.
* Tests: `open_path_enters_existing_directory` on the server side,
  store-side coverage for directory-enter and existing non-md
  file-selection.

Initiative noted (you implemented this in idle cycles while
`systacean-5` was blocked) — that's exactly the right use of idle
time. Filing it for the record so when we recap the phase the move
is credited correctly.

### Commit clearance

**APPROVED architect-side.** Gated on @@Alex authorization.

@@Alex's call whether to fold this into the closeout patch (0.10.1)
or leave it for the post-recycle wave. My recommendation: fold it
in. It's small, well-isolated, and shipping it now means the fresh
@@Systacean session post-recycle starts on a clean queue.

### Proposed commit message

```text
chan open <dir>: enter the directory listing

Differentiate the directory-enter path from the non-md-file
selection path in the control-socket open_path handler. The
frontend window_command handler dispatches enter: true to
revealAndEnterDirectory, which expands the target directory so
lazy loading surfaces its contents instead of leaving the user
in the parent with the dir highlighted. Non-md file behavior
unchanged.
```

### Pre-push gate

Before commit, run the full gate: `cargo fmt --check`,
`cargo clippy --all-targets -- -D warnings`, `cargo test`,
`cargo build --no-default-features`, `cd web && npm run check &&
npm run test && npm run build`. You already verified the relevant
subset; the full sweep catches any cross-crate interaction.

### Sequencing

If @@Alex auths the fold-in, commit `systacean-4` AFTER
@@FullStack commits `fullstack-2`, BEFORE you fire `systacean-5`.
If @@Alex declines (closeout stays tight at the original four +
fullstack-2 + fullstack-3), park this task until after the recycle.
