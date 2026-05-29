# systacean-1: CLI scriptability — `chan list --json`, `chan remove --name`

Owner: @@Systacean
Date: 2026-05-19

## Goal

Two CLI ergonomics fixes:

1. **`chan list --json`** — emit the drive registry as machine-
   readable JSON. Today's text output is not scriptable.
2. **`chan remove --name <name>`** — currently `chan remove`
   requires a path. If drive names are unique in the registry,
   accept `--name` as an alternative. **Audit uniqueness before
   implementing**; if names are not unique, surface a clear
   error and document the rule (or punt and propose a
   uniqueness migration as a follow-up).

## Background

Source in [`../phase-8-bugs.md`](../phase-8-bugs.md) under the
`cmdline:` item.

## Acceptance criteria

* `chan list --json` returns a stable JSON shape (array of
  drives, each with at least `name`, `path`, `id`, plus any
  fields the text variant already shows). Schema documented in
  the CLI help text.
* `chan list` text output unchanged for humans.
* Uniqueness audit completed and recorded in this task file.
* If unique: `chan remove --name <name>` removes by name.
* If not unique: `chan remove --name <name>` errors with the
  conflicting paths and asks the user to pick the path. Add a
  follow-up task suggestion for migration.
* Both flags reflected in `chan list --help` / `chan remove
  --help` output.

## How to start

`crates/chan/src/main.rs` for the clap subcommand definitions
and `cmd_list` / `cmd_remove` handlers. Registry uniqueness
lives in `crates/chan-drive/` — check `Drive::register` or the
registry add path.

## 2026-05-19 — uniqueness audit + implementation

### Uniqueness audit

Drive names in the registry are **not unique** and the code does
not try to make them so. Tracing the write path through
[`crates/chan-drive/src/registry.rs`](../../../../crates/chan-drive/src/registry.rs)
and
[`crates/chan-drive/src/library.rs`](../../../../crates/chan-drive/src/library.rs):

| API                                  | Uniqueness check on `name`? |
|--------------------------------------|-----------------------------|
| `Registry::touch(root, name)`        | No (only matches by path)   |
| `Registry::set_name(root, name)`     | No                          |
| `Library::register_drive(root, ...)` | Delegates to `touch`        |
| `Library::rename_drive(root, ...)`   | Delegates to `set_name`     |

`chan add /path/a --name notes` followed by `chan add /path/b
--name notes` produces two registry rows both named `notes`,
unique only by path / uuid. Smoke test below confirms the
behaviour against a freshly-built `./target/debug/chan`.

### Behaviour decision

`chan remove --name NAME` does the lookup itself and refuses to
guess when names collide:

* 0 matches → error pointing at `chan list` and the path form.
* 1 match  → unregister that path.
* 2+ matches → error listing the candidate paths, suggesting the
  path form. No follow-up uniqueness migration is proposed
  because the registry treats display names as user labels, not
  identifiers; the stable identity is `uuid` (and the user-typed
  identity is `path`).

### Changes

`crates/chan/src/main.rs`:

* File header refreshed to advertise `chan list [--json]` and
  the `<PATH> | --name N` shape on `chan remove`.
* `Command::List` now takes `--json` (help text describes the
  shape: `{"drives":[{name,path,uuid,last_opened},...]}`,
  `name` nullable, `last_opened` RFC3339 UTC).
* `Command::Remove` now uses a clap `ArgGroup` (`remove_target`)
  with `PATH` and `--name <NAME>` mutually exclusive and
  required-one-of.
* `cmd_list(json)` reuses the pattern from `cmd_status` /
  `cmd_graph`: when `--json`, serialize a dedicated
  `DriveListOutput`/`DriveListEntry` shape (always emits
  `{"drives": []}` for the empty case so scripts can pipe
  through `jq` unconditionally).
* `cmd_remove(path, name)` resolves the target through a new
  `pick_drive_by_name` helper that takes only the
  `(Option<&str>, &Path)` projection of the drive list (keeps
  the helper unit-testable without poking at `KnownDrive`'s
  `pub(crate)` `canonical_path` field).

Unit tests added next to the other CLI tests in `main.rs`:

* `pick_drive_by_name_finds_unique_match`
* `pick_drive_by_name_errors_when_no_match`
* `pick_drive_by_name_errors_on_duplicate_with_candidate_paths`
* `pick_drive_by_name_ignores_unnamed_drives`

### Smoke test

Built `./target/debug/chan`, then under an isolated
`XDG_CONFIG_HOME` / `HOME`:

```
chan add /tmp/a --name alpha
chan add /tmp/b --name beta
chan add /tmp/c --name beta

chan list           -> text, three rows
chan list --json    -> {"drives":[ ... three entries ... ]}
chan remove --name alpha   -> unregistered: /tmp/a
chan remove --name beta    -> error, lists /tmp/c and /tmp/b
chan remove --name ghost   -> error, points at chan list
chan remove /tmp/b --name alpha   -> clap rejects (exit 2)
chan remove                       -> clap rejects (exit 2)
chan remove /tmp/b                -> unregistered
chan list --json (empty)          -> {"drives": []}
```

All paths behave as designed.

### Gate

* `cargo fmt --check` — clean.
* `cargo clippy --all-targets -- -D warnings` — clean (after
  fixing a `clippy::filter_map_bool_then` finding by rewriting
  the candidate filter as `.filter(..).map(..)`).
* `cargo build` (default + `--no-default-features`) — clean.
* `cargo test` — all crates green, including the four new
  `pick_drive_by_name_*` tests.

`web/npm run check` + `web/npm run build` not in scope: the
change is Rust-only, the SPA does not shell out to `chan list`,
and a grep of `web/` / `desktop/` / `scripts/` finds no
consumers. Will be re-checked at the round-close push gate
(systacean-3) along with everything else that lands.

### Status

Ready to land. Awaiting @@Architect's commit-grouping plan
before pushing (per the standing "do not commit unless
@@Architect or @@Alex tells you to" rule).

## 2026-05-19 — @@Architect: approved + commit clearance

Reviewer: @@Architect.

Audit + behaviour decision both land cleanly:

* **Uniqueness audit**: confirms name is not unique; `path` is
  the user-typed identity, `uuid` the stable one. That matches
  what the system semantics already imply, and there's no
  reason to migrate names to a unique constraint just to power
  one CLI flag. Strong call.
* **`chan remove --name`** with the 0 / 1 / 2+ branches is the
  right shape — refuses to guess on collision, points the user
  at `chan list` and the path form. `ArgGroup` enforces the
  mutually-exclusive shape at the clap layer; no manual
  validation noise.
* **`chan list --json`** with the always-emit `{"drives":[]}`
  empty case is jq-friendly. RFC3339 UTC for `last_opened` is
  the right serialization choice.
* `pick_drive_by_name` extracted as a pure helper with four
  unit tests covering the meaningful branches. Test surface
  matches the behaviour spec.
* Gate green for the Rust half. `web/npm run check` +
  `npm run build` confirmed not in scope (no SPA consumer).
  Will re-run as part of `systacean-3`'s pre-push gate at
  Round-1 close.

**Commit clearance**: approved. Commit `systacean-1` as a
standalone change. Suggested subject:

```
chan list --json + chan remove --name (systacean-1)
```

Push waits for Round-1 close.

Pick up `systacean-2` next (graph showing links to files not
in the repo). `systacean-3` parks until I publish the
commit-grouping plan.

Side ask routed to you for a future slot: `desktop/Makefile`'s
`app-signed` / `app-notarized` echo lines reference
`src-tauri/target/release/bundle/...` but the workspace
target dir means the bundle lands at `target/release/bundle/...`
post-merge. Not destructive, but stale user output. Append a
follow-up note in your journal so the fix surfaces in a later
slot.
