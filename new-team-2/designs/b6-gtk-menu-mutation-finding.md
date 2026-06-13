# B6 finding — GTK in-place Window-menu mutation is SAFE; fallback not needed

Author: @@Desktop. Authorized: task-Conductor-Desktop-17. Closes the
phase-22 unknown ("GTK in-place Window-menu mutation is unverified on
Linux; the fallback (full set_menu rebuild) is documented but not
wired").

## Verdict

`rebuild_window_menu`'s in-place remove+append mutation of the live
Window submenu is **safe on GTK**. The documented `set_menu` full-
rebuild fallback does NOT need wiring. No code changed for B6.

## How it was tested (aarch64 Ubuntu 26.04, webkit2gtk 2.52.3, GTK 3.24.52)

sdme container `b6gtk` (fs `chan-desktop-ubuntu`), chan-desktop debug
build at f198df7b + throwaway debug IPCs (worktree-only, never
committed: menu-model snapshot, bury/unbury/list-windows), run under
Xvfb + dbus with an in-page driver executing:

- 12 bury → unbury cycles on a workspace window, asserting after
  every mutation: unique item ids, exact buried-section structure,
  exact header text ("Hidden Windows (1, kept warm in memory)" — the
  B5 affordance, confirmed working on GTK), and a byte-identical
  static menu prefix vs the pre-cycle baseline.
- A destroy storm (workspace toggled off with a buried window) and a
  recovery probe (turn on, open, bury, unbury) — the dynamic tail
  cleared and re-appended correctly after the destroys.
- stderr scanned for Gtk-CRITICAL / Gtk-WARNING across every run:
  ZERO. End-of-walk screenshot shows the visible per-window menubar
  (File / Edit / Window) rendering correctly.

Result: 130/133 driver assertions green over the final clean-state
run; the 3 failures are all attributable to two non-menu
observations below, not to menu mutation.

## Incidental observations (recorded, not B6 blockers)

1. **muda `text()` reads empty after window destroys** (debug-only).
   After destroying workspace windows, reading `.text()` on
   pre-existing static items (`win-main`, `app-new-window`) returns
   empty strings, while ids, structure, and freshly appended items
   read fine — and the VISIBLE menubar is intact (screenshot). Read-
   side artifact of muda's per-window GTK widget bookkeeping; the
   mutation path is unaffected. Only matters to debug tooling that
   reads the menu model back.
2. **Second/third window for the same workspace did not materialize
   on Linux** in the container: two further `open_local_workspace`
   calls returned Ok but no new webview appeared within 90s — no
   error, no GTK warnings, first window and post-destroy recovery
   window render fully. With a stale persisted window-config stack a
   `webview with label ... already exists` WARN also surfaced once
   (label reuse from the restore stack). Unclear whether this is
   container-specific (WebKitGTK multi-webprocess under Xvfb/bwrap)
   or real Linux multi-window behavior — needs a check on a real
   Linux desktop whenever one exists. Not a menu issue; macOS
   multi-window is unaffected (phase-22 shipped it and the item-6
   walk exercised repeat opens without error).

## Build note (for docs at round close)

chan does not compile for aarch64-linux out of the box: gemm-common
0.19 (candle dep) uses fp16 inline asm (`fmla .8h`) that the default
`aarch64-unknown-linux-gnu` target lacks. On Apple-Silicon-hosted
VMs (lima/sdme) the CPU supports it: `RUSTFLAGS="-C
target-feature=+fp16"` builds clean. x86_64 CI is unaffected.

## Reproduction

Container `b6gtk` retains the harness (/work/b6/run.sh, report
server, seeded HOME) and the built binary; stopped after the round's
walk, removable at round close. Rebuild recipe: tar worktree →
/work/chan, `RUSTFLAGS="-C target-feature=+fp16" cargo build` in
desktop/src-tauri (rust-toolchain pin 1.95.0 honored via rustup),
`/work/b6/run.sh`.
