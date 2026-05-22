# fullstack-b-27 — chan-desktop Cmd+N accelerator move (Cmd+Shift+N for "New Window"; Cmd+N frees for SPA New Draft)

Owner: @@FullStackB
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

One-line Tauri accelerator change to move chan-desktop's
"New Window" menu item from `CmdOrCtrl+N` to
`CmdOrCtrl+Shift+N`. This frees Cmd+N for the SPA
New Draft handler being built in
`fullstack-a-66`.

Originally part of `-a-61` (now paused/superseded by
the multi-task draft-folder wave). This task carries
the chan-desktop side of that work.

## Reference

[`../alex/addendun-a.md`](../alex/addendun-a.md)
"## Flow for the 'New Draft' action" — Cmd+N is the
chord for New Draft.

`desktop/src-tauri/src/main.rs:1069-1071`:

```rust
let new_window = MenuItemBuilder::with_id("app-new-window", "New Window")
    .accelerator("CmdOrCtrl+N")
    .build(app)?;
```

## Fix shape

Change `.accelerator("CmdOrCtrl+N")` to
`.accelerator("CmdOrCtrl+Shift+N")`. Menu item label
stays "New Window"; only the accelerator moves.

## Acceptance

1. **Cmd+Shift+N opens new chan-desktop window** in
   chan-desktop builds.
2. **Cmd+N does NOT open a new window** — the
   accelerator no longer claims the chord.
3. **The SPA's Cmd+N handler (from
   `fullstack-a-66`)** can fire on this chord without
   menu interference. (This task ships first; `-a-66`
   binds Cmd+N second.)

### Tests

Structural test: menu accelerator string equals
`"CmdOrCtrl+Shift+N"`.

### Gate

* `cargo fmt --check`, `cargo clippy --all-targets --
  -D warnings`, `cargo test -p chan-desktop` green.
* `RUSTFLAGS="-D warnings" cargo build
  --no-default-features` green.

## Coordination

* @@FullStackB lane.
* Trivial one-line change + test.
* Ships independently of `-a-66`; no dependency.

## Authorization

Yes for `desktop/src-tauri/src/main.rs` + test +
task tail + outbound.

## Numbering

This is `-b-27`.

## 2026-05-22 — implementation note (ready for commit clearance)

Trivial one-line accelerator move per the task body.

### Changes

* **`desktop/src-tauri/src/main.rs:1069`** —
  `.accelerator("CmdOrCtrl+N")` →
  `.accelerator("CmdOrCtrl+Shift+N")` on the
  `app-new-window` MenuItem. Inline comment cites
  `-b-27` + `-a-66` for the chord-handoff motivation.
* **`desktop/src-tauri/src/serve.rs::tests`** — new
  `new_window_accelerator_uses_cmd_shift_n` pin: asserts
  main.rs contains the new chord AND does NOT contain
  plain `CmdOrCtrl+N` anywhere (the negative assertion
  prevents a future menu edit from silently re-binding
  Cmd+N to something else).

### Pre-push gate (local, macOS aarch64)

| Surface                                                  | State                         |
|----------------------------------------------------------|-------------------------------|
| `cargo fmt --check`                                      | Clean.                        |
| `cargo clippy --workspace --all-targets -- -D warnings`  | Clean.                        |
| `cargo test --workspace`                                 | All pass.                     |
| `cargo test -p chan-desktop --bin chan-desktop`          | 44 tests (was 43; +1 new).    |
| `cargo build --workspace --no-default-features`          | Clean.                        |

No SPA touch in `-b-27`; web gate unaffected.

### Files to stage

```
desktop/src-tauri/src/main.rs
desktop/src-tauri/src/serve.rs
docs/journals/phase-8/fullstack-b/fullstack-b-27.md
```

Atomic `git commit --only` per
`feedback_shared_worktree_commits`. Other agents' WIP
in tree (FullStackA on `-a-64`/`-a-65`, Systacean on
`-24`, CI on `ci-14`).

### Suggested commit subject

```
chan-desktop: move New Window accelerator to Cmd+Shift+N (frees Cmd+N for SPA New Draft) (fullstack-b-27)
```

### Coordination with -a-66

`-a-66` will bind Cmd+N to the SPA's New Draft handler.
`-b-27` ships first; once `-b-27` is in HEAD, `-a-66`
can claim plain Cmd+N without menu-accelerator collision.
