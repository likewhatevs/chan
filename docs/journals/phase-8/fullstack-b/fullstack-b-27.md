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
