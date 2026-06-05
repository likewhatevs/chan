---
name: gate
description: Run the chan pre-push gate (fmt, clippy, test, no-default-features
  build, gateway build, web checks, marketing checks) and the isolated/own-gate
  model for multi-agent rounds.
when_to_use: Before any push, when CI fails, or when you need to validate a
  change against the same checks CI runs.
---

# The pre-push gate

`scripts/pre-push` is the git hook; it `cd`s to the repo root and execs
`make pre-push`. Keeping the target list in the Makefile keeps the
local hook and CI from drifting. Install the hook with
`./scripts/install-hooks`.

## What `make pre-push` runs

The gate runs, in order:

1. `cargo fmt --check`
2. `cargo clippy --all-targets -- -D warnings` (with `RUSTFLAGS=-D warnings`)
3. `cargo test --all-targets` (with `RUSTFLAGS=-D warnings`)
4. `cargo build --no-default-features` (with `RUSTFLAGS=-D warnings`)
5. `make gateway-build` (the SEPARATE gateway Cargo workspace; builds
   its SPA then its release crates)
6. `make web-check` (svelte-check + vitest + production build)
7. `make web-marketing-check` (marketing site build + smokes)

The gateway is a separate Cargo workspace and is NOT a member of the
root workspace. A `crates/`-scoped check misses it, plus the
`chan-desktop` (`desktop/src-tauri`) construction sites. When a change
touches a cross-workspace struct, build the whole repo, not just the
default workspace.

## Discipline

- **Re-run after the last edit.** A check that ran before a later edit
  is stale. `cargo fmt --check` in particular must run AFTER the final
  edit, or an "own-gate-green" report is wrong.
- **Don't pipe the command you are verifying.** `cargo ... | tail`
  reports tail's exit 0 and hides cargo's failure. Run bare and check
  `$?`, or set `pipefail`.
- **The gate gates every push, including tags.** A backgrounded gated
  push can SIGPIPE (exit 141) and silently fail to update the remote.
  Push in the foreground, redirect to a file, and verify with
  `git ls-remote`.

## Isolated / own-gate model (multi-agent rounds)

In a multi-agent shared worktree, the full-tree `make pre-push` gates
the COMMITTED state and is run by a single owner (e.g. the round's
lead) from an isolated gate worktree, so it is immune to peers'
in-flight working-tree changes.

Worker lanes report a scoped OWN-gate-green plus the pathspec they
committed, and do not block on the main-tree pre-push: a concurrent
peer's WIP causes false reds there. The scoped own-gate for frontend
work must run `make web-check` (vitest included), not just svelte-check
plus build, or stale source-pins slip past the scoped check and the
integrated gate catches them later.
